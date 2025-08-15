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
scim-server = "=0.3.2"
tokio = { version = "1.0", features = ["full"] }
serde_json = "1.0"
```

> **⚠️ Version Pinning**: Use exact version pinning (`=0.3.2`) during active development to avoid breaking changes. See [Version Strategy](../reference/versioning.md) for details.

### Option 2: Using Cargo Add Command

```bash
cargo add scim-server@=0.3.2
cargo add tokio --features full
cargo add serde_json
```

## Feature Flags

SCIM Server provides several optional features to reduce compile time and binary size:

```toml
[dependencies]
scim-server = { version = "=0.3.2", features = ["mcp", "auth", "logging"] }
```

Available features:

| Feature | Description | Default |
|---------|-------------|---------|
| `mcp` | Model Context Protocol for AI integration | ❌ |
| `auth` | Compile-time authentication system | ❌ |
| `logging` | Enhanced logging capabilities | ❌ |
| `serde` | JSON serialization support | ✅ |
| `async` | Async runtime support | ✅ |

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

```rust
use scim_server::{
    providers::StandardResourceProvider,
    storage::InMemoryStorage,
    RequestContext,
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

## IDE Setup

### Visual Studio Code

For the best development experience with VS Code:

1. Install the [rust-analyzer extension](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
2. Install the [Better TOML extension](https://marketplace.visualstudio.com/items?itemName=bungcip.better-toml)

### IntelliJ IDEA / CLion

Install the [Rust plugin](https://plugins.jetbrains.com/plugin/8182-rust) for full Rust support.

## Next Steps

Now that you have SCIM Server installed, you're ready to:

1. **[Create Your First Server](./first-server.md)** - Build a basic SCIM server
2. **[Learn Basic Operations](./basic-operations.md)** - Understand CRUD operations
3. **[Explore Examples](../../examples/)** - See working code samples

## Troubleshooting

### Common Installation Issues

**Rust version too old**:
```bash
rustup update stable
```

**Compilation errors**:
- Ensure you're using exact version pinning (`=0.3.2`)
- Check that all required features are enabled
- Verify tokio features include `"full"` or at minimum `"rt-multi-thread", "macros"`

**Performance issues during compilation**:
- Consider disabling unused features
- Use `cargo build --release` for optimized builds
- Increase available RAM for compilation

### Getting Help

If you encounter issues:

1. Check the [Troubleshooting Guide](../how-to/troubleshooting.md)
2. Search existing [GitHub Issues](https://github.com/pukeko37/scim-server/issues)
3. Create a new issue with your system details and error messages

## Platform-Specific Notes

### Windows

No special requirements. SCIM Server works on all Windows versions supported by Rust.

### macOS

No special requirements. Works on both Intel and Apple Silicon Macs.

### Linux

Works on all major Linux distributions. If using system packages instead of rustup:

**Ubuntu/Debian**:
```bash
sudo apt update
sudo apt install build-essential
```

**CentOS/RHEL/Fedora**:
```bash
sudo yum groupinstall "Development Tools"
# or for newer versions:
sudo dnf groupinstall "Development Tools"
```

You're now ready to build with SCIM Server! 🚀