//! # SCIM Schema Validator
//!
//! A command-line utility for validating SCIM schema files to ensure they conform to the
//! expected format and can be loaded by the SCIM server library.
//!
//! ## Overview
//!
//! This utility performs comprehensive validation of SCIM schema files, including:
//! - JSON syntax validation
//! - Required field presence checking
//! - Schema ID URI format validation
//! - Attribute structure validation
//! - Complex attribute sub-attribute validation
//! - Canonical values format verification
//! - Schema registry loading tests
//!
//! ## Usage
//!
//! ### Validate a Single Schema File
//!
//! ```bash
//! cargo run --bin schema-validator schemas/User.json
//! ```
//!
//! ### Validate All Schemas in a Directory
//!
//! ```bash
//! cargo run --bin schema-validator ./schemas/
//! ```
//!
//! ## Output Examples
//!
//! ### Successful Validation
//!
//! ```text
//! Validating schema file: schemas/User.json
//! ✓ Schema is valid!
//!
//! Schema Summary:
//!   ID: urn:ietf:params:scim:schemas:core:2.0:User
//!   Name: User
//!   Description: User Account
//!   Attributes: 15
//!   Required attributes: 2
//!   Multi-valued attributes: 4
//!   Attribute types:
//!     - String: 8
//!     - Boolean: 2
//!     - Complex: 4
//!     - DateTime: 1
//!   Required attribute names: id, userName
//! ```
//!
//! ### Directory Validation
//!
//! ```text
//! Validating schemas in directory: ./schemas/
//!
//! Validating: User.json
//!   ✓ Valid - User (urn:ietf:params:scim:schemas:core:2.0:User)
//!
//! Validating: Group.json
//!   ✓ Valid - Group (urn:ietf:params:scim:schemas:core:2.0:Group)
//!
//! Validation Summary:
//!   Valid schemas: 2
//!   Invalid schemas: 0
//!
//! Testing schema registry loading...
//! ✓ Schema registry loaded successfully
//!   Total schemas loaded: 2
//!     - User (urn:ietf:params:scim:schemas:core:2.0:User)
//!     - Group (urn:ietf:params:scim:schemas:core:2.0:Group)
//! ```
//!
//! ### Error Output
//!
//! ```text
//! Validating schema file: invalid-schema.json
//! ❌ Schema validation failed: Schema missing required 'id' field
//! ```
//!
//! ## Validation Rules
//!
//! The validator enforces these rules:
//!
//! ### Schema Structure
//! - Must be valid JSON
//! - Must have required fields: `id`, `name`, `attributes`
//! - Schema ID must be a valid URI (starts with `urn:` or `http`)
//! - Schema name cannot be empty
//! - Must have at least one attribute
//!
//! ### Attribute Validation
//! - Attribute name cannot be empty
//! - Canonical values only allowed for string attributes
//! - Complex attributes must have sub-attributes
//! - Non-complex attributes cannot have sub-attributes
//! - Sub-attributes are recursively validated
//!
//! ## Exit Codes
//!
//! - `0`: All schemas are valid
//! - `1`: One or more schemas are invalid or validation error occurred
//!
//! ## Integration with SCIM Server
//!
//! This utility uses the same validation logic as the SCIM server library,
//! ensuring that schemas validated here will work correctly when loaded
//! into a production SCIM server instance.

use scim_server::schema::{Schema, SchemaRegistry};
use std::env;
use std::fs;
use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <schema-file-or-directory>", args[0]);
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  {} schemas/User.json", args[0]);
        eprintln!("  {} ./schemas/", args[0]);
        process::exit(1);
    }

    let path = &args[1];
    let path = Path::new(path);

    if path.is_file() {
        validate_single_file(path);
    } else if path.is_dir() {
        validate_directory(path);
    } else {
        eprintln!(
            "Error: '{}' is not a valid file or directory",
            path.display()
        );
        process::exit(1);
    }
}

fn validate_single_file(file_path: &Path) {
    println!("Validating schema file: {}", file_path.display());

    match load_and_validate_schema(file_path) {
        Ok(schema) => {
            println!("✓ Schema is valid!");
            print_schema_summary(&schema);
        }
        Err(e) => {
            eprintln!("❌ Schema validation failed: {}", e);
            process::exit(1);
        }
    }
}

fn validate_directory(dir_path: &Path) {
    println!("Validating schemas in directory: {}", dir_path.display());

    let mut valid_count = 0;
    let mut error_count = 0;

    // Look for JSON files in the directory
    match fs::read_dir(dir_path) {
        Ok(entries) => {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("json") {
                        println!(
                            "\nValidating: {}",
                            path.file_name().unwrap().to_string_lossy()
                        );

                        match load_and_validate_schema(&path) {
                            Ok(schema) => {
                                println!("  ✓ Valid - {} ({})", schema.name, schema.id);
                                valid_count += 1;
                            }
                            Err(e) => {
                                eprintln!("  ❌ Invalid - {}", e);
                                error_count += 1;
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Error reading directory: {}", e);
            process::exit(1);
        }
    }

    println!("\nValidation Summary:");
    println!("  Valid schemas: {}", valid_count);
    println!("  Invalid schemas: {}", error_count);

    if error_count > 0 {
        process::exit(1);
    }

    // Try to load all schemas together
    println!("\nTesting schema registry loading...");
    match SchemaRegistry::from_schema_dir(dir_path) {
        Ok(registry) => {
            println!("✓ Schema registry loaded successfully");
            let schemas = registry.get_schemas();
            println!("  Total schemas loaded: {}", schemas.len());
            for schema in schemas {
                println!("    - {} ({})", schema.name, schema.id);
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to load schema registry: {}", e);
            process::exit(1);
        }
    }
}

fn load_and_validate_schema(file_path: &Path) -> Result<Schema, Box<dyn std::error::Error>> {
    // Read the file
    let content = fs::read_to_string(file_path)?;

    // Parse as JSON first
    let json_value: serde_json::Value = serde_json::from_str(&content)?;

    // Validate it has required top-level fields
    let obj = json_value
        .as_object()
        .ok_or("Schema must be a JSON object")?;

    if !obj.contains_key("id") {
        return Err("Schema missing required 'id' field".into());
    }

    if !obj.contains_key("name") {
        return Err("Schema missing required 'name' field".into());
    }

    if !obj.contains_key("attributes") {
        return Err("Schema missing required 'attributes' field".into());
    }

    // Try to deserialize as Schema
    let schema: Schema = serde_json::from_str(&content)?;

    // Validate the schema structure
    validate_schema_structure(&schema)?;

    Ok(schema)
}

fn validate_schema_structure(schema: &Schema) -> Result<(), Box<dyn std::error::Error>> {
    // Validate schema ID format (should be a URI)
    if schema.id.is_empty() {
        return Err("Schema ID cannot be empty".into());
    }

    if !schema.id.starts_with("urn:") && !schema.id.starts_with("http") {
        return Err("Schema ID should be a URI (starting with 'urn:' or 'http')".into());
    }

    // Validate schema name
    if schema.name.is_empty() {
        return Err("Schema name cannot be empty".into());
    }

    // Validate attributes
    if schema.attributes.is_empty() {
        return Err("Schema must have at least one attribute".into());
    }

    // Validate each attribute
    for (i, attr) in schema.attributes.iter().enumerate() {
        validate_attribute(attr, &format!("attribute[{}]", i))?;
    }

    Ok(())
}

fn validate_attribute(
    attr: &scim_server::schema::AttributeDefinition,
    context: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Validate attribute name
    if attr.name.is_empty() {
        return Err(format!("{}: Attribute name cannot be empty", context).into());
    }

    // Validate canonical values if present
    if !attr.canonical_values.is_empty() {
        if !matches!(attr.data_type, scim_server::schema::AttributeType::String) {
            return Err(format!(
                "{}: Canonical values only allowed for string attributes",
                context
            )
            .into());
        }
    }

    // Validate sub-attributes for complex types
    if matches!(attr.data_type, scim_server::schema::AttributeType::Complex) {
        if attr.sub_attributes.is_empty() {
            return Err(format!("{}: Complex attributes must have sub-attributes", context).into());
        }

        // Recursively validate sub-attributes
        for (i, sub_attr) in attr.sub_attributes.iter().enumerate() {
            let sub_context = format!("{}.subAttributes[{}]", context, i);
            validate_attribute(sub_attr, &sub_context)?;
        }
    } else if !attr.sub_attributes.is_empty() {
        return Err(format!(
            "{}: Non-complex attributes cannot have sub-attributes",
            context
        )
        .into());
    }

    Ok(())
}

fn print_schema_summary(schema: &Schema) {
    println!();
    println!("Schema Summary:");
    println!("  ID: {}", schema.id);
    println!("  Name: {}", schema.name);
    println!("  Description: {}", schema.description);
    println!("  Attributes: {}", schema.attributes.len());

    // Count different attribute types
    let mut type_counts = std::collections::HashMap::new();
    let mut required_count = 0;
    let mut multi_valued_count = 0;

    for attr in &schema.attributes {
        *type_counts
            .entry(format!("{:?}", attr.data_type))
            .or_insert(0) += 1;
        if attr.required {
            required_count += 1;
        }
        if attr.multi_valued {
            multi_valued_count += 1;
        }
    }

    println!("  Required attributes: {}", required_count);
    println!("  Multi-valued attributes: {}", multi_valued_count);
    println!("  Attribute types:");
    for (attr_type, count) in type_counts {
        println!("    - {}: {}", attr_type, count);
    }

    // List required attributes
    let required_attrs: Vec<&str> = schema
        .attributes
        .iter()
        .filter(|attr| attr.required)
        .map(|attr| attr.name.as_str())
        .collect();

    if !required_attrs.is_empty() {
        println!("  Required attribute names: {}", required_attrs.join(", "));
    }
}
