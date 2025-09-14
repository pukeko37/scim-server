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
scim-server = "=0.5.2"
tokio = { version = "1.0", features = ["full"] }
serde_json = "1.0"
```

> **Note**: The library is under active development. Pin to exact versions for stability. Breaking changes are signaled by minor version increments until v1.0.

## Verification

Create a simple test to verify the installation works:

```rust
use scim_server::{
    ScimServer,                          // Core SCIM server - see API docs
    providers::StandardResourceProvider, // Standard resource provider implementation
    storage::InMemoryStorage,            // In-memory storage for development
    RequestContext                       // Request context for operations
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // See: https://docs.rs/scim-server/latest/scim_server/storage/struct.InMemoryStorage.html
    let storage = InMemoryStorage::new();
    // See: https://docs.rs/scim-server/latest/scim_server/providers/struct.StandardResourceProvider.html
    let provider = StandardResourceProvider::new(storage);
    // See: https://docs.rs/scim-server/latest/scim_server/struct.ScimServer.html
    let server = ScimServer::new(provider)?;
    // See: https://docs.rs/scim-server/latest/scim_server/struct.RequestContext.html
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
- [Configuration Guide](./configuration.md) - Learn about storage backends and advanced setup
- [API Reference](https://docs.rs/scim-server/latest/scim_server/) - Complete API documentation on docs.rs

For production deployments, see the [Configuration Guide](./configuration.md) for information about storage backends, multi-tenant setup, and scaling considerations.