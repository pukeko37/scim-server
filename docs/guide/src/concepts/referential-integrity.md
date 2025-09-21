# Referential Integrity

Referential integrity in SCIM systems involves managing data dependencies between Users and Groups while maintaining consistency across multiple identity providers and clients. This page outlines the SCIM server's principled approach to handling these challenges.

## Value Proposition

The SCIM server's referential integrity stance provides:

- **Protocol Compliance**: Strict adherence to SCIM 2.0 specifications for group membership management
- **Client Authority**: Clear delineation of responsibilities between server and client systems
- **Scalable Architecture**: Design that works across single and multi-client environments
- **Operational Clarity**: Well-defined boundaries for what the server does and doesn't manage
- **Integration Flexibility**: Support for diverse IdP ecosystems without imposing rigid constraints

## The Referential Integrity Challenge

### Single Source of Truth Complexity

In traditional identity management, a single Identity Provider (IdP) like Active Directory serves as the authoritative source for user and group data. However, modern SCIM deployments often involve:

- **Multiple SCIM Clients**: Different systems provisioning to the same SCIM server
- **Federated Identity Sources**: Various HR systems, directories, and applications
- **Hybrid Environments**: Mix of cloud and on-premises identity sources
- **Temporal Inconsistencies**: Different provisioning schedules and update cycles

### Data Dependency Scenarios

```text
Scenario 1: User Deletion
┌─────────────────┐    ┌─────────────────┐
│ IdP-A deletes   │    │ Group still     │
│ User "john.doe" │───▶│ references      │
│                 │    │ deleted user    │
└─────────────────┘    └─────────────────┘

Scenario 2: Group Membership Changes  
┌─────────────────┐    ┌─────────────────┐
│ IdP-A removes   │    │ IdP-B adds same │
│ user from Group │◄──▶│ user to Group   │
│ simultaneously  │    │ simultaneously  │
└─────────────────┘    └─────────────────┘

Scenario 3: Nested Group Dependencies
┌─────────────────┐    ┌─────────────────┐
│ Parent Group    │    │ Child Group     │
│ references      │───▶│ gets deleted    │
│ Child Group     │    │ by different    │
│                 │    │ client          │
└─────────────────┘    └─────────────────┘
```

## SCIM 2.0 Protocol Foundation

### Group-Centric Membership Model

The SCIM 2.0 specification establishes clear principles for referential integrity:

**Groups Resource as Authority**
- Group membership is managed through the `Group` resource's `members` attribute
- The `members` attribute contains authoritative member references
- All membership changes must flow through Group resource operations

**User Groups as Read-Only**
- The User resource's `groups` attribute is **read-only**
- User groups are derived from Group memberships, not directly managed
- Clients cannot modify user group memberships via User resource operations

**Example: Proper Group Membership Management**
```json
{
  "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
  "id": "engineering-team",
  "displayName": "Engineering Team", 
  "members": [
    {
      "value": "user-123",
      "$ref": "https://example.com/v2/Users/user-123",
      "type": "User",
      "display": "John Doe"
    },
    {
      "value": "user-456", 
      "$ref": "https://example.com/v2/Users/user-456",
      "type": "User",
      "display": "Jane Smith"
    }
  ]
}
```

### Multi-Valued Attribute Semantics

SCIM's multi-valued attribute design supports referential relationships:

- **Resource References**: `$ref` fields provide URI-based resource linking
- **Type Indicators**: `type` field distinguishes User vs Group members
- **Display Names**: Human-readable references for operational clarity
- **Value Fields**: Canonical resource identifiers

## SCIM Server's Principled Stance

### Core Philosophy: Client Authority

The SCIM server adopts a **client authority model** where:

1. **SCIM Clients (IdPs) maintain authoritative control** over referential integrity
2. **The server validates protocol compliance**, not business logic consistency  
3. **Cross-resource relationship enforcement is the client's responsibility**
4. **The server provides mechanisms, clients provide policies**

### What the Server Does

**Protocol Validation**
- Validates that member references use proper ResourceId format
- Ensures Group.members follows SCIM multi-valued attribute structure
- Verifies that member types ("User", "Group") are recognized values
- Maintains consistent JSON schema compliance

**Resource Operation Support**
- Processes Group resource CREATE/UPDATE/DELETE operations
- Handles member addition/removal through Group resource modifications
- Supports PATCH operations for incremental membership changes
- Provides proper HTTP status codes and error responses

**Version Management**
- Tracks resource versions for optimistic concurrency control
- Enables conditional operations to prevent lost updates
- Maintains version consistency across resource modifications

### What the Server Does NOT Do

**Cross-Resource Validation**
- Does not verify that referenced User resources exist
- Does not prevent creation of groups with non-existent members
- Does not validate that User.groups matches Group.members

**Cascading Operations**
- Does not automatically remove users from groups when users are deleted
- Does not cascade group deletions to remove nested group references
- Does not synchronize User.groups when Group.members changes

**Multi-Client Coordination**
- Does not resolve conflicts between competing SCIM clients
- Does not enforce "last writer wins" or other conflict resolution policies
- Does not maintain client priority or authorization hierarchies

## Client Responsibilities

### Single-Client Environments

When one SCIM client manages all resources:

**Consistency Maintenance**
- Ensure User deletions trigger Group membership cleanup
- Maintain User.groups derivation from Group.members relationships  
- Handle nested group dependencies appropriately

**Error Recovery**
- Implement retry logic for failed referential operations
- Provide reconciliation processes for inconsistent states
- Monitor for orphaned references and clean them up

### Multi-Client Environments

When multiple SCIM clients operate against the same server:

**External Coordination Required**
- Implement client-side coordination mechanisms outside SCIM
- Use external message queues, databases, or orchestration systems
- Establish client priority and conflict resolution policies

**Common Coordination Patterns**
```text
Pattern 1: Master-Slave Hierarchy
┌─────────────────┐    ┌─────────────────┐
│ Primary IdP     │    │ Secondary IdPs  │
│ (HR System)     │───▶│ defer to        │
│ authoritative   │    │ primary for     │
│ for users       │    │ conflicts       │
└─────────────────┘    └─────────────────┘

Pattern 2: Functional Separation  
┌─────────────────┐    ┌─────────────────┐
│ IdP-A manages   │    │ IdP-B manages   │
│ User lifecycle  │    │ Group           │
│ (create/delete) │    │ memberships     │
└─────────────────┘    └─────────────────┘

Pattern 3: External Orchestrator
┌─────────────────┐    ┌─────────────────┐
│ Identity        │    │ Multiple IdPs   │
│ Orchestrator    │───▶│ coordinate      │
│ coordinates     │    │ through         │
│ all changes     │    │ orchestrator    │
└─────────────────┘    └─────────────────┘
```

## Implementation in SCIM Server

### Group Members Architecture

The server's `GroupMembers` value object properly models SCIM group membership:

```rust
// Type-safe group member representation
pub struct GroupMember {
    value: ResourceId,           // Required: member resource ID
    display: Option<String>,     // Optional: human-readable name
    member_type: Option<String>, // Optional: "User" or "Group"
}

// Collection type with validation
pub type GroupMembers = MultiValuedAttribute<GroupMember>;
```

**Design Benefits**
- Compile-time validation of member reference format
- Support for both User and Group members (nested groups)
- SCIM-compliant JSON serialization with `$ref` fields
- Integration with the server's value object architecture

### Resource Provider Integration

ResourceProvider implementations handle group operations:

```rust
// Group creation with members
async fn create_resource(
    &self,
    resource_type: "Group",
    data: group_with_members_json,
    context: &RequestContext,
) -> Result<VersionedResource, Self::Error>

// Group membership updates
async fn update_resource(
    &self,
    resource_type: "Group", 
    id: "engineering-team",
    data: updated_members_json,
    expected_version: Some(version),
    context: &RequestContext,
) -> Result<VersionedResource, Self::Error>
```

**Implementation Boundaries**
- Validates JSON structure and member format
- Does not verify referenced users exist in storage
- Processes membership changes as requested by client
- Returns appropriate errors for malformed requests only

## Industry IdP Behavior Patterns

### Okta
- **Single Source Model**: Okta maintains authoritative user/group data
- **Push-Based Sync**: Pushes complete group memberships to SCIM servers
- **Consistency Expectation**: Expects SCIM server to store what's pushed
- **Conflict Handling**: Limited multi-client coordination capabilities

### Microsoft Entra (Azure AD)
- **Directory Authority**: Azure AD as single source of truth
- **Group-Centric Operations**: Manages membership through Group resources
- **Telemetry Support**: Provides detailed error reporting for reconciliation
- **Enterprise Features**: Advanced conflict detection and reporting

### Ping Identity
- **Flexible Architecture**: Supports complex multi-source scenarios  
- **Directory Integration**: Can act as both SCIM client and server
- **Custom Reconciliation**: Allows scripted consistency checks
- **Enterprise Coordination**: Advanced tools for multi-client environments

### Common Pattern: Server Delegation
All major IdPs expect the SCIM server to:
- Store the data as provided by the client
- Validate protocol compliance, not business logic
- Provide mechanisms for the IdP to maintain consistency
- Report errors for malformed requests, not referential issues

## Operational Considerations

### Monitoring and Observability  

**Recommended Metrics**
- Group membership operation rates
- Member reference validation errors  
- Resource not found errors (indicating potential orphans)
- Multi-client operation overlaps

**Alerting Strategies**
- High rates of referential errors may indicate client coordination issues
- Sudden spikes in group operations may indicate bulk synchronization
- Monitor for patterns indicating multiple clients modifying same resources

### Diagnostic Capabilities

**Optional Enhancement: Consistency Reports**
While not enforcing referential integrity, the server could provide optional diagnostic endpoints:

```http
GET /v2/Diagnostics/ReferentialConsistency?tenantId=acme-corp

{
  "orphanedGroupMembers": [
    {
      "groupId": "engineering-team",
      "memberId": "deleted-user-123", 
      "issue": "Member references non-existent User resource"
    }
  ],
  "inconsistentUserGroups": [
    {
      "userId": "john.doe",
      "issue": "User.groups does not match Group.members references"
    }
  ]
}
```

**Benefits**
- Helps clients identify and resolve consistency issues
- Provides operational visibility without enforcing constraints
- Supports client-side reconciliation processes

## Best Practices for SCIM Server Operators

### 1. Document Client Responsibilities Clearly

Provide explicit documentation to IdP integration teams about:
- Who is responsible for referential integrity (they are)
- What the server validates (protocol format) vs. doesn't (business consistency)
- Recommended patterns for multi-client coordination
- Error codes that indicate client-side issues to resolve

### 2. Design for Protocol Compliance

Focus server implementation on being an excellent SCIM protocol implementation:
- Strict adherence to SCIM 2.0 specifications
- Comprehensive support for Group resource operations
- Robust error handling with appropriate HTTP status codes
- Clear API documentation and examples

### 3. Support Client Success

Provide tools and capabilities that help clients maintain consistency:
- Version-based concurrency control to prevent lost updates
- Detailed error messages for malformed requests
- Optional diagnostic capabilities for troubleshooting
- Clear examples of proper Group membership management

### 4. Avoid Over-Engineering

Resist the temptation to solve identity management problems beyond SCIM's scope:
- Don't build custom conflict resolution algorithms
- Don't impose business logic constraints on clients
- Don't attempt to coordinate between multiple clients
- Focus on being the best possible SCIM server, not an identity management system

## Comparison with Alternative Approaches

| Approach | Protocol Compliance | Operational Complexity | Client Flexibility | Scalability |
|----------|-------------------|----------------------|-------------------|-------------|
| **Client Authority** (Recommended) | ✅ High | ✅ Low | ✅ High | ✅ High |
| Server-Enforced Integrity | ⚠️ Medium | ❌ High | ❌ Low | ⚠️ Medium |  
| Hybrid Validation | ⚠️ Medium | ⚠️ Medium | ⚠️ Medium | ⚠️ Medium |
| No Validation | ❌ Low | ✅ Low | ✅ High | ✅ High |

The **Client Authority** model provides the best balance of SCIM compliance, operational simplicity, and real-world scalability.

## Future Considerations

### SCIM Protocol Evolution

The SCIM working group continues to evolve the specification. Future versions may include:
- Enhanced referential integrity guidance
- Standardized multi-client coordination patterns  
- Improved error reporting for consistency issues
- Optional server-side validation capabilities

### Integration Ecosystem Maturity

As the SCIM ecosystem matures:
- IdPs are developing better coordination mechanisms
- Industry patterns for multi-client scenarios are emerging
- Standardized orchestration tools are becoming available
- Best practices for complex identity topologies are evolving

## Conclusion

The SCIM server's referential integrity stance prioritizes **protocol compliance over data policing**. By clearly delineating responsibilities—with clients maintaining authoritative control over data consistency and the server providing robust protocol implementation—this approach scales across diverse deployment scenarios while remaining true to SCIM's fundamental design principles.

This client authority model acknowledges that referential integrity is fundamentally an identity management challenge, not a protocol challenge. The server's role is to be an excellent SCIM protocol implementation that enables clients to build sophisticated identity management solutions, rather than attempting to solve those challenges directly.

For organizations deploying SCIM servers, this approach provides a clear operational model: invest in robust client-side identity management processes and use the SCIM server as a reliable, compliant protocol gateway that faithfully executes the identity operations you design.