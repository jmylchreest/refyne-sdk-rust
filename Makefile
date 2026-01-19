# Refyne Rust SDK Makefile

.PHONY: generate build test clean help

# Default OpenAPI spec URL (can be overridden with OPENAPI_SPEC_URL env var)
OPENAPI_SPEC_URL ?= http://localhost:8080/openapi.json

help: ## Show this help message
	@echo "Refyne Rust SDK"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  %-15s %s\n", $$1, $$2}'
	@echo ""
	@echo "Environment variables:"
	@echo "  OPENAPI_SPEC_URL   URL to fetch OpenAPI spec (default: $(OPENAPI_SPEC_URL))"
	@echo "  OPENAPI_SPEC_FILE  Path to local OpenAPI spec file"

generate: ## Generate Rust types from OpenAPI spec
	@echo "Generating types from OpenAPI spec..."
	python3 scripts/generate.py --url "$(OPENAPI_SPEC_URL)"
	@echo "Done. Running cargo check..."
	cargo check

generate-file: ## Generate types from a local OpenAPI spec file
	@if [ -z "$(OPENAPI_SPEC_FILE)" ]; then \
		echo "Error: OPENAPI_SPEC_FILE is not set"; \
		echo "Usage: make generate-file OPENAPI_SPEC_FILE=path/to/openapi.json"; \
		exit 1; \
	fi
	@echo "Generating types from $(OPENAPI_SPEC_FILE)..."
	python3 scripts/generate.py --file "$(OPENAPI_SPEC_FILE)"
	@echo "Done. Running cargo check..."
	cargo check

generate-prod: ## Generate types from the production API
	@echo "Generating types from production API..."
	python3 scripts/generate.py --url "https://api.refyne.uk/openapi.json"
	@echo "Done. Running cargo check..."
	cargo check

build: ## Build the crate
	cargo build

test: ## Run tests
	cargo test

check: ## Run cargo check
	cargo check

clippy: ## Run clippy lints
	cargo clippy -- -D warnings

fmt: ## Format code
	cargo fmt

fmt-check: ## Check code formatting
	cargo fmt -- --check

clean: ## Clean build artifacts
	cargo clean

doc: ## Generate documentation
	cargo doc --no-deps --open

all: fmt clippy test build ## Run all checks and build
