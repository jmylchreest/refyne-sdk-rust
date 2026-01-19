#!/usr/bin/env python3
"""
Generate Rust types from the Refyne API OpenAPI specification.

This script fetches the OpenAPI spec from a configurable location and
generates Rust structs with serde derives. The output is written to src/types.rs.

Usage:
  python scripts/generate.py [options]

Options:
  --url <url>      Fetch spec from URL (default: http://localhost:8080/openapi.json)
  --file <path>    Read spec from local file
  --output <path>  Output file path (default: src/types.rs)
  --help           Show this help message

Environment Variables:
  OPENAPI_SPEC_URL   Override the default URL
  OPENAPI_SPEC_FILE  Use a local file instead of fetching
"""

import argparse
import json
import os
import sys
import urllib.request
from pathlib import Path
from typing import Any, Optional

DEFAULT_SPEC_URL = "http://localhost:8080/openapi.json"
DEFAULT_OUTPUT = "src/types.rs"

# Rust reserved keywords that need to be renamed
RUST_KEYWORDS = {
    "type", "fn", "let", "const", "static", "mut", "ref", "self", "super",
    "crate", "mod", "pub", "use", "struct", "enum", "trait", "impl", "for",
    "where", "loop", "while", "if", "else", "match", "return", "break",
    "continue", "move", "box", "async", "await", "dyn", "abstract", "become",
    "do", "final", "macro", "override", "priv", "typeof", "unsized", "virtual",
    "yield", "try", "union", "in", "as"
}

# Collected inline enums during processing
inline_enums: dict[str, list[str]] = {}


def parse_args() -> argparse.Namespace:
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(
        description="Generate Rust types from the Refyne API OpenAPI specification.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Fetch from local development server
  python scripts/generate.py

  # Fetch from production API
  python scripts/generate.py --url https://api.refyne.uk/openapi.json

  # Documentation: https://refyne.uk/docs

  # Use a local file
  python scripts/generate.py --file ./openapi.json

  # Using environment variables
  OPENAPI_SPEC_URL=http://localhost:8080/openapi.json python scripts/generate.py
"""
    )
    parser.add_argument(
        "--url",
        help=f"Fetch spec from URL (default: {DEFAULT_SPEC_URL})"
    )
    parser.add_argument(
        "--file",
        help="Read spec from local file"
    )
    parser.add_argument(
        "--output",
        default=DEFAULT_OUTPUT,
        help=f"Output file path (default: {DEFAULT_OUTPUT})"
    )

    args = parser.parse_args()

    # Check environment variables if not set via CLI
    if args.file is None and args.url is None:
        if os.environ.get("OPENAPI_SPEC_FILE"):
            args.file = os.environ["OPENAPI_SPEC_FILE"]
        elif os.environ.get("OPENAPI_SPEC_URL"):
            args.url = os.environ["OPENAPI_SPEC_URL"]
        else:
            args.url = DEFAULT_SPEC_URL

    return args


def fetch_spec(url: str) -> dict:
    """Fetch OpenAPI spec from URL."""
    print(f"Fetching OpenAPI spec from: {url}")
    try:
        with urllib.request.urlopen(url, timeout=30) as response:
            return json.loads(response.read().decode("utf-8"))
    except urllib.error.URLError as e:
        raise RuntimeError(f"Failed to fetch spec: {e}")


def load_spec_from_file(file_path: str) -> dict:
    """Load OpenAPI spec from a local file."""
    print(f"Loading OpenAPI spec from file: {file_path}")
    path = Path(file_path).resolve()
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


def to_snake_case(name: str) -> str:
    """Convert camelCase or PascalCase to snake_case."""
    result = []
    for i, char in enumerate(name):
        if char.isupper():
            if i > 0 and (name[i-1].islower() or (i + 1 < len(name) and name[i+1].islower())):
                result.append("_")
            result.append(char.lower())
        else:
            result.append(char)
    return "".join(result)


def to_pascal_case(name: str) -> str:
    """Convert snake_case or kebab-case to PascalCase."""
    return "".join(word.capitalize() for word in name.replace("-", "_").split("_"))


def escape_rust_keyword(name: str) -> str:
    """Escape Rust keywords by prefixing with r#."""
    if name in RUST_KEYWORDS:
        return f"r#{name}"
    return name


def resolve_ref(ref: str, spec: dict) -> dict:
    """Resolve a JSON reference to its schema."""
    ref_path = ref.replace("#/", "").split("/")
    current = spec
    for part in ref_path:
        current = current[part]
    return current


def make_enum_name(parent_type: str, field_name: str) -> str:
    """Generate an enum name from parent type and field name."""
    return f"{parent_type}{to_pascal_case(field_name)}"


def openapi_type_to_rust(
    schema: dict,
    spec: dict,
    is_required: bool = True,
    parent_type: str = "",
    field_name: str = ""
) -> str:
    """Convert an OpenAPI schema type to a Rust type string."""
    global inline_enums

    if "$ref" in schema:
        ref_name = schema["$ref"].split("/")[-1]
        rust_type = ref_name
        return rust_type if is_required else f"Option<{rust_type}>"

    if "allOf" in schema:
        # For allOf, take the first ref or combine
        types = [openapi_type_to_rust(s, spec, True) for s in schema["allOf"]]
        # Usually allOf is used for composition, use first type
        rust_type = types[0] if types else "serde_json::Value"
        return rust_type if is_required else f"Option<{rust_type}>"

    if "oneOf" in schema or "anyOf" in schema:
        # For oneOf/anyOf, use serde_json::Value as a catch-all
        return "serde_json::Value" if is_required else "Option<serde_json::Value>"

    if "enum" in schema:
        # Generate a proper enum for inline enums
        if parent_type and field_name:
            enum_name = make_enum_name(parent_type, field_name)
            inline_enums[enum_name] = schema["enum"]
            return enum_name if is_required else f"Option<{enum_name}>"
        # Fallback to String for enums without context
        return "String" if is_required else "Option<String>"

    schema_type = schema.get("type", "object")
    schema_format = schema.get("format")

    if schema_type == "string":
        rust_type = "String"
    elif schema_type == "integer":
        if schema_format == "int64":
            rust_type = "i64"
        elif schema_format == "int32":
            rust_type = "i32"
        else:
            rust_type = "i64"  # Default to i64 for integers
    elif schema_type == "number":
        if schema_format == "float":
            rust_type = "f32"
        else:
            rust_type = "f64"  # Default to f64 for numbers
    elif schema_type == "boolean":
        rust_type = "bool"
    elif schema_type == "array":
        items = schema.get("items", {})
        item_type = openapi_type_to_rust(items, spec, True, parent_type, field_name)
        rust_type = f"Vec<{item_type}>"
    elif schema_type == "object":
        additional_props = schema.get("additionalProperties")
        if additional_props is True:
            rust_type = "serde_json::Value"
        elif isinstance(additional_props, dict):
            value_type = openapi_type_to_rust(additional_props, spec, True, parent_type, field_name)
            rust_type = f"std::collections::HashMap<String, {value_type}>"
        elif schema.get("properties"):
            # Inline object - use Value
            rust_type = "serde_json::Value"
        else:
            rust_type = "serde_json::Value"
    else:
        rust_type = "serde_json::Value"

    return rust_type if is_required else f"Option<{rust_type}>"


def is_request_type(name: str) -> bool:
    """Check if a type name represents a request type."""
    # InputBody is a request type, but OutputBody/ResponseBody are response types
    if name.endswith("OutputBody") or name.endswith("ResponseBody"):
        return False
    return name.endswith("Request") or name.endswith("Input") or name.endswith("InputBody")


def is_response_type(name: str) -> bool:
    """Check if a type name represents a response type."""
    return (
        name.endswith("Response") or
        name.endswith("Output") or
        name.endswith("Result") or
        name.endswith("OutputBody") or
        name.endswith("ResponseBody")
    )


def has_required_enum_fields(schema: dict) -> bool:
    """Check if schema has required fields that are enums."""
    required = set(schema.get("required", []))
    properties = schema.get("properties", {})

    for prop_name, prop_schema in properties.items():
        if prop_name in required:
            # Check if field is an enum (either inline or ref)
            if "enum" in prop_schema:
                return True
            # For refs, we can't easily check, so assume they might be enums
            # Actually, most refs will be to complex types, not enums
    return False


def get_serde_attributes(name: str, schema: dict) -> list[str]:
    """Get serde derive attributes for a struct."""
    attrs = ["Debug", "Clone"]

    # Request types need Serialize, response types need Deserialize
    # Some types need both (for user customization)
    if is_request_type(name):
        attrs.append("Serialize")
        # Only add Default if there are no required enum fields
        if not has_required_enum_fields(schema):
            attrs.append("Default")
    elif is_response_type(name):
        attrs.append("Deserialize")
    else:
        # Other types might need both
        attrs.extend(["Serialize", "Deserialize"])

    return attrs


def generate_enum(name: str, values: list[str], description: str = "") -> list[str]:
    """Generate a Rust enum from enum values."""
    lines = []

    # Doc comment
    if description:
        lines.append(f"/// {description}")

    # Derive attributes
    lines.append("#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]")

    # Determine rename strategy
    all_lowercase = all(v == v.lower() for v in values)

    if all_lowercase:
        lines.append('#[serde(rename_all = "lowercase")]')

    lines.append(f"pub enum {name} {{")

    for value in values:
        # Convert to PascalCase for Rust variant
        variant = to_pascal_case(value)

        # Add serde rename if needed (for non-lowercase variants)
        if not all_lowercase:
            lines.append(f'    #[serde(rename = "{value}")]')

        lines.append(f"    /// {value}")
        lines.append(f"    {variant},")

    lines.append("}")
    return lines


def generate_struct(name: str, schema: dict, spec: dict) -> list[str]:
    """Generate a Rust struct from an OpenAPI schema."""
    lines = []

    # Doc comment
    description = schema.get("description", "")
    if description:
        lines.append(f"/// {description}")
    if schema.get("deprecated"):
        msg = schema.get("x-deprecated-message", "This type is deprecated.")
        lines.append(f"#[deprecated(note = \"{msg}\")]")

    # Derive attributes
    derives = get_serde_attributes(name, schema)
    lines.append(f"#[derive({', '.join(derives)})]")

    # Check if any fields use snake_case (have underscores)
    properties = schema.get("properties", {})
    has_snake_case = any("_" in k for k in properties.keys())

    # Only add rename_all if fields don't have underscores (are camelCase)
    if not has_snake_case and properties:
        lines.append('#[serde(rename_all = "camelCase")]')

    lines.append(f"pub struct {name} {{")

    required_fields = set(schema.get("required", []))

    for prop_name, prop_schema in properties.items():
        # Skip JSON Schema metadata fields (like $schema)
        if prop_name.startswith("$"):
            continue
        is_required = prop_name in required_fields
        rust_field_name = to_snake_case(prop_name)
        rust_field_name = escape_rust_keyword(rust_field_name)
        rust_type = openapi_type_to_rust(prop_schema, spec, is_required, name, prop_name)

        # Doc comment for field
        prop_description = prop_schema.get("description", "")
        if prop_description:
            lines.append(f"    /// {prop_description}")
        if prop_schema.get("deprecated"):
            msg = prop_schema.get("x-deprecated-message", "This field is deprecated.")
            lines.append(f"    #[deprecated(note = \"{msg}\")]")

        # Add serde rename for fields that need it
        actual_field_name = rust_field_name.replace("r#", "")
        needs_rename = False

        # Need rename if field is a Rust keyword
        if actual_field_name != rust_field_name.replace("r#", ""):
            needs_rename = True
        # Need rename if original has underscores but we converted to snake_case differently
        elif "_" not in prop_name and has_snake_case:
            needs_rename = True

        if needs_rename:
            lines.append(f'    #[serde(rename = "{prop_name}")]')

        # Skip serializing None for optional fields in request types
        if not is_required and is_request_type(name):
            lines.append("    #[serde(skip_serializing_if = \"Option::is_none\")]")

        lines.append(f"    pub {rust_field_name}: {rust_type},")

    lines.append("}")
    return lines


def generate_type_alias(name: str, schema: dict, spec: dict) -> list[str]:
    """Generate a Rust type alias from an OpenAPI schema."""
    lines = []

    description = schema.get("description", "")
    if description:
        lines.append(f"/// {description}")

    rust_type = openapi_type_to_rust(schema, spec, True)
    lines.append(f"pub type {name} = {rust_type};")

    return lines


def generate_types(spec: dict) -> str:
    """Generate all Rust types from an OpenAPI spec."""
    global inline_enums
    inline_enums = {}  # Reset for each run

    api_version = spec.get("info", {}).get("version", "unknown")

    lines = [
        "//! API types for the Refyne SDK.",
        "//!",
        "//! These types are generated from the OpenAPI specification.",
        "//! Do not edit this file manually - run `make generate` to regenerate.",
        "//!",
        f"//! Generated from API version: {api_version}",
        "",
        "#![allow(dead_code)]",
        "",
        "use serde::{Deserialize, Serialize};",
        "",
    ]

    schemas = spec.get("components", {}).get("schemas", {})

    if not schemas:
        lines.append("// No schemas found in OpenAPI specification")
        return "\n".join(lines)

    # First pass: collect all inline enums
    for name, schema in schemas.items():
        if schema.get("type") == "object" or "properties" in schema:
            for prop_name, prop_schema in schema.get("properties", {}).items():
                if "enum" in prop_schema and prop_schema.get("type") == "string":
                    enum_name = make_enum_name(name, prop_name)
                    inline_enums[enum_name] = prop_schema["enum"]

    # Group schemas by category
    request_schemas = []
    response_schemas = []
    enum_schemas = []
    other_schemas = []

    for name, schema in schemas.items():
        if "enum" in schema and "properties" not in schema:
            enum_schemas.append((name, schema))
        elif is_request_type(name):
            request_schemas.append((name, schema))
        elif is_response_type(name):
            response_schemas.append((name, schema))
        else:
            other_schemas.append((name, schema))

    # Generate top-level enums first
    if enum_schemas or inline_enums:
        lines.append("// " + "=" * 76)
        lines.append("// Enums")
        lines.append("// " + "=" * 76)
        lines.append("")

        # Top-level enums from schema
        for name, schema in enum_schemas:
            lines.extend(generate_enum(name, schema.get("enum", []), schema.get("description", "")))
            lines.append("")

        # Inline enums collected during processing
        for enum_name, values in sorted(inline_enums.items()):
            # Skip if already generated as top-level
            if any(name == enum_name for name, _ in enum_schemas):
                continue
            lines.extend(generate_enum(enum_name, values))
            lines.append("")

    # Generate request types
    if request_schemas:
        lines.append("// " + "=" * 76)
        lines.append("// Request Types")
        lines.append("// " + "=" * 76)
        lines.append("")
        for name, schema in request_schemas:
            if schema.get("type") == "object" or "properties" in schema:
                lines.extend(generate_struct(name, schema, spec))
            else:
                lines.extend(generate_type_alias(name, schema, spec))
            lines.append("")

    # Generate response types
    if response_schemas:
        lines.append("// " + "=" * 76)
        lines.append("// Response Types")
        lines.append("// " + "=" * 76)
        lines.append("")
        for name, schema in response_schemas:
            if schema.get("type") == "object" or "properties" in schema:
                lines.extend(generate_struct(name, schema, spec))
            else:
                lines.extend(generate_type_alias(name, schema, spec))
            lines.append("")

    # Generate other types
    if other_schemas:
        lines.append("// " + "=" * 76)
        lines.append("// Other Types")
        lines.append("// " + "=" * 76)
        lines.append("")
        for name, schema in other_schemas:
            if schema.get("type") == "object" or "properties" in schema:
                lines.extend(generate_struct(name, schema, spec))
            elif "allOf" in schema:
                lines.extend(generate_struct(name, schema, spec))
            else:
                lines.extend(generate_type_alias(name, schema, spec))
            lines.append("")

    # Add missing types that the SDK depends on but aren't in the OpenAPI spec
    lines.append("// " + "=" * 76)
    lines.append("// Additional Types (not in OpenAPI spec but required by SDK)")
    lines.append("// " + "=" * 76)
    lines.append("")

    # ProvidersResponse - used by list_providers()
    lines.extend([
        "/// Response containing available LLM providers.",
        "#[derive(Debug, Clone, Deserialize)]",
        "pub struct ProvidersResponse {",
        "    /// List of available provider names.",
        "    pub providers: Vec<String>,",
        "}",
        "",
    ])

    # Model type for ModelList items (if not already defined)
    if not any(name == "Model" for name, _ in other_schemas):
        lines.extend([
            "/// Available LLM model.",
            "#[derive(Debug, Clone, Deserialize)]",
            "pub struct Model {",
            "    /// Model identifier.",
            "    pub id: String,",
            "    /// Display name.",
            "    pub name: String,",
            "}",
            "",
        ])

    # Add type aliases for client.rs compatibility
    # Only add aliases for types that don't already exist in the schema
    schema_names = set(schemas.keys())

    lines.extend([
        "// ==========================================================================",
        "// Type Aliases for Client Compatibility",
        "// ==========================================================================",
        "",
    ])

    # Define all aliases - only add if alias name doesn't exist in schema
    type_aliases = [
        # Job types
        ("Job", "JobResponse", "Single job response."),
        ("JobList", "ListJobsOutputBody", "Job list response."),
        ("JobResults", "serde_json::Value", "Job extraction results (dynamic JSON)."),

        # Schema types
        ("Schema", "SchemaOutput", "Schema response."),
        ("SchemaList", "ListSchemasOutputBody", "Schema list response."),
        ("CreateSchemaRequest", "CreateSchemaInputBody", "Schema creation request."),

        # Site types
        ("Site", "SavedSiteOutput", "Saved site response."),
        ("SiteList", "ListSavedSitesOutputBody", "Saved site list response."),
        ("CreateSiteRequest", "CreateSavedSiteInputBody", "Site creation request."),

        # API key types
        ("ApiKeyList", "ListKeysOutputBody", "API key list response."),
        ("ApiKeyCreated", "CreateKeyOutputBody", "API key creation response."),

        # LLM key types
        ("LlmKey", "UserServiceKeyResponse", "User LLM service key response."),
        ("LlmKeyList", "ListUserServiceKeysOutputBody", "LLM service key list response."),
        ("UpsertLlmKeyRequest", "UserServiceKeyInput", "LLM key upsert request."),

        # LLM chain types
        ("LlmChain", "GetUserFallbackChainOutputBody", "LLM fallback chain."),
        ("LlmChainEntry", "UserFallbackChainEntryInput", "LLM fallback chain entry."),

        # Model types
        ("ModelList", "UserListModelsOutputBody", "Model list response."),

        # Extract types
        ("ExtractRequest", "ExtractInputBody", "Extract request."),
        ("ExtractResponse", "ExtractOutputBody", "Extract response."),

        # Crawl types
        ("CrawlRequest", "CreateCrawlJobInputBody", "Crawl request."),
        ("CrawlJobCreated", "CrawlJobResponseBody", "Crawl job created response."),

        # Analyze types
        ("AnalyzeRequest", "AnalyzeInputBody", "Analyze request."),
        ("AnalyzeResponse", "AnalyzeResponseBody", "Analyze response."),
    ]

    for alias_name, target_type, doc in type_aliases:
        # Skip if alias name already exists in the schema
        if alias_name in schema_names:
            continue
        lines.append(f"/// {doc}")
        lines.append(f"pub type {alias_name} = {target_type};")
        lines.append("")

    return "\n".join(lines)


def main() -> int:
    """Main entry point."""
    args = parse_args()

    try:
        # Load spec
        if args.file:
            spec = load_spec_from_file(args.file)
        else:
            spec = fetch_spec(args.url)

        print(f"OpenAPI version: {spec.get('openapi', 'unknown')}")
        print(f"API title: {spec.get('info', {}).get('title', 'unknown')}")
        print(f"API version: {spec.get('info', {}).get('version', 'unknown')}")

        # Generate types
        types_code = generate_types(spec)

        # Write output
        output_path = Path(args.output).resolve()
        output_path.parent.mkdir(parents=True, exist_ok=True)

        with open(output_path, "w", encoding="utf-8") as f:
            f.write(types_code)

        print(f"Types written to: {output_path}")

        # Count generated types
        schema_count = len(spec.get("components", {}).get("schemas", {}))
        enum_count = len(inline_enums)
        print(f"Generated {schema_count} types + {enum_count} inline enums")

        return 0

    except Exception as e:
        print(f"Error generating types: {e}", file=sys.stderr)
        import traceback
        traceback.print_exc()
        return 1


if __name__ == "__main__":
    sys.exit(main())
