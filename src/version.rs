//! SDK version information and API compatibility checking.

use crate::Error;
use tracing::warn;

/// Current SDK version.
pub const SDK_VERSION: &str = "0.0.0";

/// Minimum API version this SDK supports.
pub const MIN_API_VERSION: &str = "0.0.0";

/// Maximum API version this SDK was built against.
pub const MAX_KNOWN_API_VERSION: &str = "0.0.0";

/// Parse a semver version string into components.
///
/// Returns (major, minor, patch, prerelease).
pub fn parse_version(version: &str) -> (u32, u32, u32, Option<&str>) {
    let parts: Vec<&str> = version.split('-').collect();
    let prerelease = parts.get(1).copied();

    let nums: Vec<u32> = parts[0].split('.').filter_map(|s| s.parse().ok()).collect();

    (
        nums.first().copied().unwrap_or(0),
        nums.get(1).copied().unwrap_or(0),
        nums.get(2).copied().unwrap_or(0),
        prerelease,
    )
}

/// Compare two semver versions.
///
/// Returns:
/// - `-1` if a < b
/// - `0` if a == b
/// - `1` if a > b
pub fn compare_versions(a: &str, b: &str) -> i8 {
    let (a_major, a_minor, a_patch, _) = parse_version(a);
    let (b_major, b_minor, b_patch, _) = parse_version(b);

    if a_major != b_major {
        return if a_major < b_major { -1 } else { 1 };
    }
    if a_minor != b_minor {
        return if a_minor < b_minor { -1 } else { 1 };
    }
    if a_patch != b_patch {
        return if a_patch < b_patch { -1 } else { 1 };
    }

    0
}

/// Check if an API version is compatible with this SDK.
///
/// Returns an error if the API version is too old.
/// Logs a warning if the API version is newer than expected.
pub fn check_api_version_compatibility(api_version: &str) -> Result<(), Error> {
    // If API version is lower than minimum supported, return error
    if compare_versions(api_version, MIN_API_VERSION) < 0 {
        return Err(Error::UnsupportedApiVersion {
            api_version: api_version.to_string(),
            min_version: MIN_API_VERSION.to_string(),
            max_known_version: MAX_KNOWN_API_VERSION.to_string(),
        });
    }

    // If API major version is higher than known, warn
    let (api_major, _, _, _) = parse_version(api_version);
    let (max_major, _, _, _) = parse_version(MAX_KNOWN_API_VERSION);

    if api_major > max_major {
        warn!(
            api_version = api_version,
            sdk_version = SDK_VERSION,
            max_known_version = MAX_KNOWN_API_VERSION,
            "API version {} is newer than this SDK was built for ({}). \
             There may be breaking changes. Consider upgrading the SDK.",
            api_version,
            MAX_KNOWN_API_VERSION
        );
    }

    Ok(())
}

/// Build the User-Agent string for SDK requests.
pub fn build_user_agent(suffix: Option<&str>) -> String {
    let mut ua = format!(
        "Refyne-SDK-Rust/{} ({}; {})",
        SDK_VERSION,
        std::env::consts::OS,
        std::env::consts::ARCH
    );

    if let Some(s) = suffix {
        ua.push(' ');
        ua.push_str(s);
    }

    ua
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version() {
        assert_eq!(parse_version("1.2.3"), (1, 2, 3, None));
        assert_eq!(parse_version("0.0.0"), (0, 0, 0, None));
        assert_eq!(parse_version("1.2.3-beta"), (1, 2, 3, Some("beta")));
    }

    #[test]
    fn test_compare_versions() {
        assert_eq!(compare_versions("1.2.3", "1.2.3"), 0);
        assert_eq!(compare_versions("2.0.0", "1.0.0"), 1);
        assert_eq!(compare_versions("1.0.0", "2.0.0"), -1);
        assert_eq!(compare_versions("1.2.0", "1.1.0"), 1);
        assert_eq!(compare_versions("1.1.2", "1.1.1"), 1);
    }

    #[test]
    fn test_version_constants() {
        // Min should be <= Max
        assert!(compare_versions(MIN_API_VERSION, MAX_KNOWN_API_VERSION) <= 0);
    }

    #[test]
    fn test_build_user_agent() {
        let ua = build_user_agent(None);
        assert!(ua.contains("Refyne-SDK-Rust"));
        assert!(ua.contains(SDK_VERSION));

        let ua_with_suffix = build_user_agent(Some("MyApp/1.0"));
        assert!(ua_with_suffix.contains("MyApp/1.0"));
    }
}
