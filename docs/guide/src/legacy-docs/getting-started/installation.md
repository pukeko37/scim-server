# Installation

This guide covers installing and setting up the SCIM Server library in your Rust project.

## Prerequisites

Before you begin, ensure you have:

- **Rust 1.75 or later** - [Install Rust](https://rustup.rs/)
- **Cargo** - Comes with Rust installation
- Basic familiarity with Rust and async programming

You can verify your Rust installation:

```bash
rustc --version
cargo --version
```

## Adding SCIM Server to Your Project

### Option 1: Using Cargo (Recommended)

Add SCIM Server to your `Cargo.toml` dependencies:

```toml
[dependencies]
scim-server = "0.3.7"
tokio = { version = "1.0", features = ["full"] }
serde_json = "1.0"
```

> **⚠️ Version Pinning**: Use flexible versioning (`0.3.7`) to get patch fixes automatically. For exact version control, use `=0.3.7`. See [Version Strategy](../reference/versioning.md) for details.

### Option 2: Using Cargo Add Command

```bash
cargo add scim-server@0.3.7
cargo add tokio --features full
cargo add serde_json
```

## Feature Flags

SCIM Server provides several optional features to reduce compile time and binary size:

```toml
[dependencies]
scim-server = { version = "0.3.7", features = ["mcp"] }
```

Available features:

| Feature | Description | Default |
|---------|-------------|---------|
| `mcp` | Model Context Protocol for AI integration (includes async-trait and rust-mcp-sdk) | ❌ |
| `async-trait` | Async trait support (included with mcp) | ❌ |
| `rust-mcp-sdk` | MCP SDK dependency (included with mcp) | ❌ |

Note: Only the `mcp` feature is currently available. This enables AI integration capabilities through the Model Context Protocol.

## Development Dependencies

For development and testing, you may want additional dependencies:

```toml
[dev-dependencies]
tokio-test = "0.4"
serde_json = "1.0"
uuid = { version = "1.0", features = ["v4"] }
```

## Verification

Create a simple test to verify your installation:

```rust,no_run
use scim_server::{
    StandardResourceProvider,
    InMemoryStorage,
    RequestContext,
    ResourceProvider,  // Required trait for create_resource method
};
use serde_json::json;

#[tokio::test]
async fn test_installation() {
    let storage = InMemoryStorage::new();
    let provider = StandardResourceProvider::new(storage);
    let context = RequestContext::new("test".to_string());
    
    // Test creating a simple resource
    let user_data = json!({
        "userName": "test.user",
        "emails": [{"value": "test@example.com", "primary": true}]
    });
    
    let user = provider.create_resource("User", user_data, &context).await.unwrap();
    
    // If this compiles and runs, installation is successful!
    println!("SCIM Server installed successfully!");
    println!("Created test user: {}", user.get_username().unwrap_or("unknown"));
}
```

Run the test:

```bash
cargo test test_installation
```

## Next Steps

Now that you have SCIM Server installed, you're ready to:

1. **[Create Your First Server](./first-server.md)** - Build a basic SCIM server
2. **[Learn Basic Operations](./basic-operations.md)** - Understand CRUD operations