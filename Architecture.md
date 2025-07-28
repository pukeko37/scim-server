# SCIM Server Architecture

## Overview

This crate implements a comprehensive System for Cross-domain Identity Management (SCIM) server library in Rust, following RFC 7643 (Core Schema) and RFC 7644 (Protocol) specifications. SCIM is designed to make identity management in cloud-based applications easier by providing a common user schema and extension model for exchanging identity data via HTTP.

## Purpose and Value

The primary goal of this library is to enable developers to implement SCIM-compliant identity providers with minimal effort while maintaining type safety, performance, and extensibility. Key benefits include:

- **Standards Compliance**: Full RFC 7643/7644 compliance for core User schema
- **Type Safety**: Compile-time guarantees preventing invalid operations
- **Flexibility**: Trait-based architecture supporting any storage backend
- **Performance**: Async-first design with functional programming patterns
- **Extensibility**: Dynamic schema registration and custom resource types

## Core Architectural Principles

### 1. Separation of Concerns

The crate is organized into distinct modules, each with a single responsibility:

- **`schema`**: Schema definitions, validation, and registry management
- **`resource`**: Resource types, handlers, and provider abstractions
- **`scim_server`**: Full-featured dynamic SCIM server implementation
- **`schema_discovery`**: Lightweight schema discovery component
- **`error`**: Comprehensive error handling with detailed context
- **`resource_handlers`**: Factory functions for standard resource types

### 2. Dual Component Architecture

The library provides two complementary components:

#### ScimServer (Full-Featured)
```rust
ScimServer<P: ResourceProvider>
```
- Dynamic resource type registration at runtime
- Full CRUD operations (Create, Read, Update, Delete, List, Search)
- Schema-driven validation and operations
- Supports custom resource types and operations
- Production-ready for complete SCIM endpoints

#### SchemaDiscovery (Lightweight)
```rust
SchemaDiscovery<State = Ready>
```
- Schema discovery and introspection only
- Service provider configuration access
- Type-safe state machine with phantom types
- Minimal overhead for schema-only scenarios

This separation allows users to choose the appropriate level of functionality for their use case.

### 3. Type-Safe State Machine

The `SchemaDiscovery` component uses phantom types to encode configuration state at compile time:

```rust
pub struct SchemaDiscovery<State = Ready> {
    inner: Option<DiscoveryInner>,
    _state: PhantomData<State>,
}

// State markers
pub struct Uninitialized;
pub struct Ready;
```

This design prevents:
- Operations on uninitialized components
- Invalid state transitions
- Runtime configuration errors

### 4. Resource Provider Pattern

The `ResourceProvider` trait abstracts data access, enabling support for any storage backend:

```rust
pub trait ResourceProvider {
    type Error: std::error::Error + Send + Sync + 'static;
    
    fn create_resource(&self, resource_type: &str, data: Value, context: &RequestContext) 
        -> impl Future<Output = Result<Resource, Self::Error>> + Send;
    // ... other CRUD operations
}
```

This pattern follows dependency inversion, allowing the library to remain agnostic about:
- Database technologies (SQL, NoSQL, in-memory)
- Authentication mechanisms
- Business logic implementations

### 5. Dynamic Schema-Driven Operations

Rather than hard-coding resource types, the architecture uses:

#### SchemaRegistry
Central registry for all schemas with validation capabilities:
```rust
pub struct SchemaRegistry {
    core_user_schema: Schema,
    schemas: HashMap<String, Schema>,
}
```

#### ResourceHandler
Defines operations and behavior for specific resource types:
```rust
pub struct ResourceHandler {
    pub schema: Schema,
    pub handlers: HashMap<String, AttributeHandler>,
    pub mappers: Vec<Box<dyn SchemaMapper>>,
    pub custom_methods: HashMap<String, Box<dyn Fn(&mut Value) -> Result<Value, String>>>,
}
```

#### SchemaResourceBuilder
Fluent API for constructing resource handlers dynamically:
```rust
SchemaResourceBuilder::new(user_schema)
    .with_getter("id", |data| data.get("id")?.as_str().map(Value::String))
    .with_setter("userName", |data, value| { /* implementation */ })
    .with_database_mapping("users", vec![("userName", "username")])
    .build()
```

This approach enables:
- Runtime resource type registration
- Schema evolution without code changes
- Custom attribute handlers and transformations
- Database mapping abstractions

### 6. Validation Architecture

Multi-layered validation ensures schema compliance:

#### Attribute-Level Validation
- Type checking (string, boolean, integer, etc.)
- Cardinality validation (single vs. multi-valued)
- Required attribute enforcement
- Canonical value constraints

#### Resource-Level Validation
- Schema conformance checking
- Unknown attribute detection
- Complex attribute validation
- Cross-attribute consistency

#### Custom Validation
- Extensible validation rules
- Business logic integration
- Provider-specific constraints

### 7. Error Handling Strategy

Comprehensive error types provide detailed context:

```rust
pub enum ScimError {
    Validation(ValidationError),
    Provider(Box<dyn Error + Send + Sync>),
    ResourceNotFound { resource_type: String, id: String },
    SchemaNotFound { schema_id: String },
    // ... other variants
}
```

Error handling follows Rust best practices:
- No panics in library code
- Detailed error context
- Proper error chaining
- Type-safe error propagation

### 8. Functional Programming Patterns

The codebase emphasizes functional programming:

#### Immutable Data
- Prefer immutable data structures
- Copy-on-write semantics where needed
- Functional transformations over mutations

#### Iterator Combinators
```rust
// Preferred
schemas.iter()
    .filter(|s| s.active)
    .map(|s| &s.name)
    .collect()

// Avoided
let mut result = Vec::new();
for schema in schemas {
    if schema.active {
        result.push(&schema.name);
    }
}
```

#### Pure Functions
- Side-effect free where possible
- Predictable behavior
- Easy testing and reasoning

## Extension Points

The architecture provides several extension mechanisms:

### 1. Custom Resource Types
Register new resource types with custom schemas:
```rust
let custom_handler = SchemaResourceBuilder::new(custom_schema)
    .with_custom_method("validate_business_rules", |data| { /* validation */ })
    .build();

server.register_resource_type("CustomResource", custom_handler, operations)?;
```

### 2. Database Mapping
Abstract database schemas from SCIM schemas:
```rust
let mapper = DatabaseMapper::new("users", vec![
    ("userName", "username"),
    ("emails.value", "email_address"),
]);
```

### 3. Custom Validation
Implement domain-specific validation rules:
```rust
builder.with_transformer("userName", |value| {
    // Custom business logic
    validate_username_policy(value)
})
```

### 4. Authentication Integration
Implement authentication through the `RequestContext`:
```rust
pub struct RequestContext {
    pub request_id: String,
    // Add authentication info here
}
```

## Performance Considerations

### Async-First Design
- All I/O operations are async
- Efficient resource utilization
- High concurrency support

### Memory Efficiency
- Schema registry shared across operations
- Lazy loading of schemas where possible
- Efficient JSON processing with serde

### Compile-Time Optimizations
- Phantom types eliminate runtime overhead
- Generic specialization where beneficial
- Zero-cost abstractions

## Testing Strategy

The architecture supports comprehensive testing:

### Unit Tests
- Individual component testing
- Mock implementations of traits
- Property-based testing for validation

### Integration Tests
- End-to-end SCIM operations
- Schema compliance verification
- Error handling scenarios

### Example Usage
- Complete working examples in `examples/`
- Documentation tests ensuring API correctness

## Future Extensibility

The architecture is designed for future enhancements:

### Protocol Support
- Easy addition of new SCIM operations
- Protocol version evolution
- Custom endpoint support

### Schema Evolution
- Runtime schema updates
- Backward compatibility
- Migration support

### Performance Optimizations
- Caching strategies
- Bulk operations
- Streaming support

## Conclusion

This SCIM server architecture balances several competing concerns:
- **Type Safety vs. Flexibility**: Uses Rust's type system for compile-time guarantees while maintaining runtime flexibility
- **Performance vs. Abstraction**: Provides high-level abstractions without sacrificing performance
- **Standards Compliance vs. Customization**: Ensures RFC compliance while allowing extensive customization

The result is a robust, extensible foundation for building SCIM-compliant identity management systems that can evolve with changing requirements while maintaining safety and performance.