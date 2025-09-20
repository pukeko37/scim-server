# How to Use This Guide

The guide is organized into progressive sections:

1. **Getting Started**: Quick setup and basic usage
2. **Core Concepts**: Understanding the fundamental ideas
3. **Tutorials**: Step-by-step guides for common scenarios
4. **How-To Guides**: Solutions for specific problems
5. **Advanced Topics**: Deep dives into complex scenarios
6. **Reference**: Technical specifications and details

## Learning Path

**New to SCIM?** Start with the [Architecture Overview](./architecture.md) to understand the standard.

**Ready to code?** Jump to [Your First SCIM Server](./getting-started/first-server.md) for hands-on experience.

**Building production systems?** Read through [Installation](./getting-started/installation.md) and the [Configuration Guide](./getting-started/configuration.md).

## What You'll Learn

By the end of this guide, you'll understand how to:

- Compose SCIM Server components for your specific requirements
- Implement the [`ResourceProvider` trait](https://docs.rs/scim-server/latest/scim_server/trait.ResourceProvider.html) for your application's data model
- Create custom [schema extensions](./concepts/schema-mechanisms.md) and value objects
- Build [multi-tenant systems](./concepts/multi-tenant-architecture.md) using the provided context components
- Integrate SCIM components with web frameworks and [AI tools](./concepts/mcp-integration.md)
- Deploy production systems using the [concurrency control](./concepts/concurrency.md) and observability components

## Getting Help

- **Examples**: Check the [examples directory](https://github.com/pukeko37/scim-server/tree/main/examples) for working code
- **API Documentation**: See [docs.rs/scim-server](https://docs.rs/scim-server/latest/scim_server/) for detailed API reference
- **Issues**: Report bugs or ask questions on [GitHub Issues](https://github.com/pukeko37/scim-server/issues)

Let's get started! 🚀