//! Cache implementation that respects Cache-Control headers.

use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// Trait for cache implementations.
pub trait Cache: Send + Sync {
    /// Get a cached entry by key.
    fn get(&self, key: &str) -> Option<CacheEntry>;

    /// Store an entry in the cache.
    fn set(&self, key: &str, entry: CacheEntry);

    /// Delete an entry from the cache.
    fn delete(&self, key: &str);
}

/// A cached entry.
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// The cached value.
    pub value: Value,
    /// Unix timestamp (seconds) when the entry expires.
    pub expires_at: u64,
    /// Parsed Cache-Control directives.
    pub cache_control: CacheControlDirectives,
}

/// Parsed Cache-Control header directives.
#[derive(Debug, Clone, Default)]
pub struct CacheControlDirectives {
    /// Don't cache at all.
    pub no_store: bool,
    /// Revalidate before serving.
    pub no_cache: bool,
    /// Only cache for the authenticated user.
    pub private: bool,
    /// Maximum age in seconds.
    pub max_age: Option<u64>,
    /// Serve stale while revalidating.
    pub stale_while_revalidate: Option<u64>,
}

/// Parse a Cache-Control header into directives.
pub fn parse_cache_control(header: Option<&str>) -> CacheControlDirectives {
    let mut directives = CacheControlDirectives::default();

    let header = match header {
        Some(h) => h,
        None => return directives,
    };

    for part in header.split(',') {
        let part = part.trim().to_lowercase();

        if part == "no-store" {
            directives.no_store = true;
        } else if part == "no-cache" {
            directives.no_cache = true;
        } else if part == "private" {
            directives.private = true;
        } else if let Some(value) = part.strip_prefix("max-age=") {
            if let Ok(v) = value.parse() {
                directives.max_age = Some(v);
            }
        } else if let Some(value) = part.strip_prefix("stale-while-revalidate=") {
            if let Ok(v) = value.parse() {
                directives.stale_while_revalidate = Some(v);
            }
        }
    }

    directives
}

/// Create a cache entry from a response.
///
/// Returns `None` if the response should not be cached.
pub fn create_cache_entry(value: Value, cache_control_header: Option<&str>) -> Option<CacheEntry> {
    let cache_control = parse_cache_control(cache_control_header);

    // Don't cache if no-store
    if cache_control.no_store {
        return None;
    }

    // Need max-age to cache
    let max_age = cache_control.max_age?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    Some(CacheEntry {
        value,
        expires_at: now + max_age,
        cache_control,
    })
}

/// Generate a cache key from request details.
pub fn generate_cache_key(method: &str, url: &str, auth_hash: Option<&str>) -> String {
    let mut key = format!("{}:{}", method.to_uppercase(), url);
    if let Some(hash) = auth_hash {
        key.push(':');
        key.push_str(hash);
    }
    key
}

/// Hash a string using SHA-256 (truncated to 16 chars for cache keys).
pub fn hash_string(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    let result = hasher.finalize();
    // Return first 16 hex chars (64 bits of entropy)
    hex::encode(&result[..8])
}

/// In-memory cache implementation with O(1) eviction.
pub struct MemoryCache {
    store: Arc<RwLock<HashMap<String, CacheEntry>>>,
    order: Arc<RwLock<VecDeque<String>>>,
    max_entries: usize,
}

impl MemoryCache {
    /// Create a new memory cache with the given maximum entries.
    pub fn new(max_entries: usize) -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::with_capacity(max_entries))),
            order: Arc::new(RwLock::new(VecDeque::with_capacity(max_entries))),
            max_entries,
        }
    }

    /// Get the current number of entries.
    pub fn size(&self) -> usize {
        self.store.read().unwrap().len()
    }

    /// Clear all entries.
    pub fn clear(&self) {
        let mut store = self.store.write().unwrap();
        let mut order = self.order.write().unwrap();
        store.clear();
        order.clear();
    }
}

impl Cache for MemoryCache {
    fn get(&self, key: &str) -> Option<CacheEntry> {
        let store = self.store.read().unwrap();
        let entry = store.get(key)?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Check if expired
        if entry.expires_at < now {
            // Check stale-while-revalidate
            if let Some(swr) = entry.cache_control.stale_while_revalidate {
                let stale_deadline = entry.expires_at + swr;
                if now < stale_deadline {
                    return Some(entry.clone());
                }
            }

            // Fully expired - caller should call delete
            return None;
        }

        Some(entry.clone())
    }

    fn set(&self, key: &str, entry: CacheEntry) {
        if entry.cache_control.no_store {
            return;
        }

        let mut store = self.store.write().unwrap();
        let mut order = self.order.write().unwrap();

        // Evict oldest if at capacity (O(1) with VecDeque)
        while store.len() >= self.max_entries {
            if let Some(oldest) = order.pop_front() {
                store.remove(&oldest);
            } else {
                break;
            }
        }

        // Check if key exists - if so, it's already in order
        if !store.contains_key(key) {
            order.push_back(key.to_string());
        }

        store.insert(key.to_string(), entry);
    }

    fn delete(&self, key: &str) {
        let mut store = self.store.write().unwrap();
        let mut order = self.order.write().unwrap();

        store.remove(key);
        // Note: This is still O(n), but delete is infrequent
        // For true O(1) delete, we'd need a linked hash map
        order.retain(|k| k != key);
    }
}

impl Default for MemoryCache {
    fn default() -> Self {
        Self::new(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_cache_control() {
        let d = parse_cache_control(None);
        assert!(!d.no_store);
        assert!(d.max_age.is_none());

        let d = parse_cache_control(Some("no-store"));
        assert!(d.no_store);

        let d = parse_cache_control(Some("max-age=3600"));
        assert_eq!(d.max_age, Some(3600));

        let d = parse_cache_control(Some("private, max-age=300, stale-while-revalidate=60"));
        assert!(d.private);
        assert_eq!(d.max_age, Some(300));
        assert_eq!(d.stale_while_revalidate, Some(60));
    }

    #[test]
    fn test_create_cache_entry() {
        assert!(create_cache_entry(json!({}), Some("no-store")).is_none());
        assert!(create_cache_entry(json!({}), Some("private")).is_none());

        let entry = create_cache_entry(json!({"test": true}), Some("max-age=3600"));
        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert_eq!(entry.value, json!({"test": true}));
    }

    #[test]
    fn test_memory_cache() {
        let cache = MemoryCache::new(2);

        let entry = create_cache_entry(json!("v1"), Some("max-age=3600")).unwrap();
        cache.set("k1", entry);

        assert!(cache.get("k1").is_some());
        assert!(cache.get("k2").is_none());

        cache.delete("k1");
        assert!(cache.get("k1").is_none());
    }

    #[test]
    fn test_hash_string() {
        let h1 = hash_string("test");
        let h2 = hash_string("test");
        assert_eq!(h1, h2);

        let h3 = hash_string("other");
        assert_ne!(h1, h3);
    }
}
