# Installation

This guide will get you up and running with the SCIM server library in under 5 minutes.

## Prerequisites

- **Rust 1.75 or later** - [Install Rust](https://rustup.rs/)

To verify your installation:
```bash
rustc --version
cargo --version
```

## Adding the Dependency

Add to your `Cargo.toml`:

```toml
[dependencies]
scim-server = "=0.5.1"
tokio = { version = "1.0", features = ["full"] }
serde_json = "1.0"
```

> **Note**: The library is under active development. Pin to exact versions for stability. Breaking changes are signaled by minor version increments until v1.0.

## Verification

Create a simple test to verify the installation works:

```rust
use scim_server::{
    ScimServer, providers::StandardResourceProvider, storage::InMemoryStorage,
    RequestContext
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let server = ScimServer::new(provider)?;
    let context = RequestContext::new("test".to_string());

    let user_data = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "userName": "john.doe",
        "active": true
    });

    let user = server.create_resource("User", user_data, &context).await?;
    let retrieved = server.get_resource("User", user.get_id().unwrap(), &context).await?;

    assert_eq!(retrieved.get_attribute("active").unwrap(), &json!(true));

    Ok(())
}
```

Run with:
```bash
cargo run
```

If this runs without errors, your installation is working correctly!


## Next Steps

Once installation is complete, proceed to:

- [Your First SCIM Server](./first-server.md) - Build a complete working implementation
- [Configuration Guide](../configuration/basic-config.md) - Learn about storage backends and advanced setup
- [API Reference](../api/overview.md) - Explore all available operations

For production deployments, see the [Production Setup Guide](../deployment/production.md) for information about system requirements, databases, and scaling considerations.