# AI Integration with MCP

This tutorial shows you how to integrate your SCIM Server with AI assistants using the Model Context Protocol (MCP). You'll learn to expose SCIM operations as MCP tools, enabling AI assistants to manage identity resources through natural language.

## What is MCP?

The Model Context Protocol (MCP) is a standardized way for AI applications to connect to external data sources and tools. It enables AI assistants like Claude, ChatGPT, and custom bots to:

- **Execute operations** through defined tools
- **Access real-time data** from external systems  
- **Maintain context** across conversations
- **Provide structured responses** based on live data

For SCIM servers, MCP integration means AI assistants can:
- Create, read, update, and delete users and groups
- Query identity data with natural language
- Automate complex provisioning workflows
- Generate reports and insights from identity data

## Quick Start Example

Here's a simple MCP server that exposes SCIM operations:

```rust
use scim_server::{ScimServer, InMemoryProvider, ScimUser, ScimGroup};
use mcp_server::{McpServer, Tool, ToolResult, McpError};
use serde_json::{json, Value};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create SCIM server
    let provider = InMemoryProvider::new();
    let scim_server = ScimServer::builder()
        .provider(provider)
        .build();
    
    // Create MCP server with SCIM tools
    let mcp_server = McpServer::builder()
        .name("SCIM Identity Manager")
        .version("1.0.0")
        .tool(create_user_tool(scim_server.clone()))
        .tool(get_user_tool(scim_server.clone()))
        .tool(list_users_tool(scim_server.clone()))
        .tool(create_group_tool(scim_server.clone()))
        .build();
    
    // Start MCP server
    let listener = TcpListener::bind("127.0.0.1:3001").await?;
    println!("MCP Server running on localhost:3001");
    mcp_server.serve(listener).await?;
    
    Ok(())
}
```

With this setup, an AI assistant can perform operations like:
- "Create a new user named Alice Johnson with email alice@company.com"
- "Show me all users in the Engineering department"
- "Add Alice to the Administrators group"

## Step 1: Define MCP Tools

MCP tools are functions that AI assistants can call. Let's define tools for common SCIM operations:

### Create User Tool

```rust
use mcp_server::{Tool, ToolInput, ToolResult};
use serde_json::{json, Value};

fn create_user_tool(scim_server: ScimServer) -> Tool {
    Tool::builder()
        .name("create_user")
        .description("Create a new user in the SCIM system")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "tenant_id": {
                    "type": "string",
                    "description": "Tenant identifier"
                },
                "username": {
                    "type": "string",
                    "description": "User's username/email"
                },
                "given_name": {
                    "type": "string",
                    "description": "User's first name"
                },
                "family_name": {
                    "type": "string",
                    "description": "User's last name"
                },
                "email": {
                    "type": "string",
                    "description": "User's email address"
                },
                "active": {
                    "type": "boolean",
                    "description": "Whether the user is active",
                    "default": true
                },
                "department": {
                    "type": "string",
                    "description": "User's department"
                }
            },
            "required": ["tenant_id", "username", "given_name", "family_name", "email"]
        }))
        .handler(move |input: ToolInput| {
            let scim_server = scim_server.clone();
            async move {
                let tenant_id = input.get_string("tenant_id")?;
                let username = input.get_string("username")?;
                let given_name = input.get_string("given_name")?;
                let family_name = input.get_string("family_name")?;
                let email = input.get_string("email")?;
                let active = input.get_bool("active").unwrap_or(true);
                let department = input.get_optional_string("department");
                
                // Build the user
                let mut user_builder = ScimUser::builder()
                    .username(&username)
                    .given_name(&given_name)
                    .family_name(&family_name)
                    .email(&email)
                    .active(active);
                
                if let Some(dept) = department {
                    user_builder = user_builder.department(&dept);
                }
                
                let user = user_builder.build()?;
                
                // Create the user
                let created_user = scim_server.create_user(&tenant_id, user).await?;
                
                ToolResult::success(json!({
                    "message": format!("Successfully created user {} ({})", created_user.username(), created_user.id()),
                    "user": {
                        "id": created_user.id(),
                        "username": created_user.username(),
                        "name": {
                            "givenName": created_user.given_name(),
                            "familyName": created_user.family_name()
                        },
                        "email": created_user.primary_email(),
                        "active": created_user.active(),
                        "department": created_user.department()
                    }
                }))
            }
        })
        .build()
}
```

### Get User Tool

```rust
fn get_user_tool(scim_server: ScimServer) -> Tool {
    Tool::builder()
        .name("get_user")
        .description("Retrieve a user by ID or username")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "tenant_id": {
                    "type": "string",
                    "description": "Tenant identifier"
                },
                "identifier": {
                    "type": "string",
                    "description": "User ID or username"
                }
            },
            "required": ["tenant_id", "identifier"]
        }))
        .handler(move |input: ToolInput| {
            let scim_server = scim_server.clone();
            async move {
                let tenant_id = input.get_string("tenant_id")?;
                let identifier = input.get_string("identifier")?;
                
                // Try to get user by ID first, then by username
                let user = if identifier.contains('@') {
                    scim_server.find_user_by_username(&tenant_id, &identifier).await?
                } else {
                    scim_server.get_user(&tenant_id, &identifier).await?
                };
                
                match user {
                    Some(user) => ToolResult::success(json!({
                        "user": {
                            "id": user.id(),
                            "username": user.username(),
                            "name": {
                                "formatted": user.formatted_name(),
                                "givenName": user.given_name(),
                                "familyName": user.family_name()
                            },
                            "emails": user.emails(),
                            "active": user.active(),
                            "department": user.department(),
                            "title": user.title(),
                            "manager": user.manager().map(|m| m.display_name()),
                            "meta": {
                                "created": user.meta().created,
                                "lastModified": user.meta().last_modified,
                                "version": user.meta().version
                            }
                        }
                    })),
                    None => ToolResult::error(format!("User '{}' not found", identifier))
                }
            }
        })
        .build()
}
```

### List Users Tool

```rust
fn list_users_tool(scim_server: ScimServer) -> Tool {
    Tool::builder()
        .name("list_users")
        .description("List users with optional filtering")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "tenant_id": {
                    "type": "string",
                    "description": "Tenant identifier"
                },
                "filter": {
                    "type": "string",
                    "description": "SCIM filter expression (e.g., 'department eq \"Engineering\"')"
                },
                "count": {
                    "type": "integer",
                    "description": "Maximum number of results",
                    "default": 50,
                    "maximum": 200
                },
                "sort_by": {
                    "type": "string",
                    "description": "Attribute to sort by",
                    "default": "meta.lastModified"
                }
            },
            "required": ["tenant_id"]
        }))
        .handler(move |input: ToolInput| {
            let scim_server = scim_server.clone();
            async move {
                let tenant_id = input.get_string("tenant_id")?;
                let filter = input.get_optional_string("filter");
                let count = input.get_optional_int("count").unwrap_or(50).min(200);
                let sort_by = input.get_optional_string("sort_by").unwrap_or_else(|| "meta.lastModified".to_string());
                
                let mut options = ListOptions::builder()
                    .count(count as usize)
                    .sort_by(&sort_by);
                
                // Note: Filter expressions are not yet implemented
                // For now, we'll load all users and filter in memory if needed
                
                let response = scim_server.list_users(&tenant_id, &options.build()).await?;
                
                let users: Vec<Value> = response.resources.into_iter().map(|user| {
                    json!({
                        "id": user.id(),
                        "username": user.username(),
                        "name": {
                            "formatted": user.formatted_name(),
                            "givenName": user.given_name(),
                            "familyName": user.family_name()
                        },
                        "email": user.primary_email(),
                        "active": user.active(),
                        "department": user.department(),
                        "title": user.title(),
                        "lastModified": user.meta().last_modified
                    })
                }).collect();
                
                ToolResult::success(json!({
                    "totalResults": response.total_results,
                    "startIndex": response.start_index,
                    "itemsPerPage": response.items_per_page,
                    "users": users
                }))
            }
        })
        .build()
}
```

## Step 2: Group Management Tools

Add tools for group operations:

### Create Group Tool

```rust
fn create_group_tool(scim_server: ScimServer) -> Tool {
    Tool::builder()
        .name("create_group")
        .description("Create a new group")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "tenant_id": {
                    "type": "string",
                    "description": "Tenant identifier"
                },
                "display_name": {
                    "type": "string",
                    "description": "Group display name"
                },
                "description": {
                    "type": "string",
                    "description": "Group description"
                },
                "members": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    },
                    "description": "Array of user IDs or usernames to add as members"
                }
            },
            "required": ["tenant_id", "display_name"]
        }))
        .handler(move |input: ToolInput| {
            let scim_server = scim_server.clone();
            async move {
                let tenant_id = input.get_string("tenant_id")?;
                let display_name = input.get_string("display_name")?;
                let description = input.get_optional_string("description");
                let member_identifiers = input.get_optional_array("members").unwrap_or_default();
                
                // Resolve member identifiers to user IDs
                let mut members = Vec::new();
                for identifier_value in member_identifiers {
                    if let Some(identifier) = identifier_value.as_str() {
                        let user = if identifier.contains('@') {
                            scim_server.find_user_by_username(&tenant_id, identifier).await?
                        } else {
                            scim_server.get_user(&tenant_id, identifier).await?
                        };
                        
                        if let Some(user) = user {
                            members.push(GroupMember {
                                value: user.id().to_string(),
                                ref_: Some(format!("../Users/{}", user.id())),
                                type_: Some("User".to_string()),
                                display: user.formatted_name(),
                            });
                        } else {
                            return ToolResult::error(format!("User '{}' not found", identifier));
                        }
                    }
                }
                
                // Build the group
                let mut group_builder = ScimGroup::builder()
                    .display_name(&display_name)
                    .members(members);
                
                if let Some(desc) = description {
                    group_builder = group_builder.description(&desc);
                }
                
                let group = group_builder.build()?;
                
                // Create the group
                let created_group = scim_server.create_group(&tenant_id, group).await?;
                
                ToolResult::success(json!({
                    "message": format!("Successfully created group '{}' with {} members", 
                                     created_group.display_name(), 
                                     created_group.members().len()),
                    "group": {
                        "id": created_group.id(),
                        "displayName": created_group.display_name(),
                        "description": created_group.description(),
                        "memberCount": created_group.members().len(),
                        "members": created_group.members().iter().map(|m| json!({
                            "id": m.value,
                            "display": m.display
                        })).collect::<Vec<_>>()
                    }
                }))
            }
        })
        .build()
}
```

### Add User to Group Tool

```rust
fn add_user_to_group_tool(scim_server: ScimServer) -> Tool {
    Tool::builder()
        .name("add_user_to_group")
        .description("Add a user to a group")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "tenant_id": {
                    "type": "string",
                    "description": "Tenant identifier"
                },
                "group_identifier": {
                    "type": "string", 
                    "description": "Group ID or display name"
                },
                "user_identifier": {
                    "type": "string",
                    "description": "User ID or username"
                }
            },
            "required": ["tenant_id", "group_identifier", "user_identifier"]
        }))
        .handler(move |input: ToolInput| {
            let scim_server = scim_server.clone();
            async move {
                let tenant_id = input.get_string("tenant_id")?;
                let group_identifier = input.get_string("group_identifier")?;
                let user_identifier = input.get_string("user_identifier")?;
                
                // Find the group
                let group = scim_server.find_group(&tenant_id, &group_identifier).await?
                    .ok_or_else(|| format!("Group '{}' not found", group_identifier))?;
                
                // Find the user
                let user = if user_identifier.contains('@') {
                    scim_server.find_user_by_username(&tenant_id, &user_identifier).await?
                } else {
                    scim_server.get_user(&tenant_id, &user_identifier).await?
                }.ok_or_else(|| format!("User '{}' not found", user_identifier))?;
                
                // Add user to group using PATCH operation
                let patch_op = PatchOperation {
                    op: PatchOp::Add,
                    path: Some("members".to_string()),
                    value: Some(json!([{
                        "value": user.id(),
                        "$ref": format!("../Users/{}", user.id()),
                        "type": "User",
                        "display": user.formatted_name()
                    }])),
                };
                
                let updated_group = scim_server.patch_group(&tenant_id, group.id(), vec![patch_op]).await?;
                
                ToolResult::success(json!({
                    "message": format!("Successfully added {} to group '{}'", 
                                     user.formatted_name(), 
                                     updated_group.display_name()),
                    "group": {
                        "id": updated_group.id(),
                        "displayName": updated_group.display_name(),
                        "memberCount": updated_group.members().len()
                    },
                    "user": {
                        "id": user.id(),
                        "username": user.username(),
                        "name": user.formatted_name()
                    }
                }))
            }
        })
        .build()
}
```

## Step 3: Advanced Query Tools

Create intelligent tools that can understand natural language queries:

### Search Tool

```rust
fn search_tool(scim_server: ScimServer) -> Tool {
    Tool::builder()
        .name("search")
        .description("Search for users or groups using natural language")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "tenant_id": {
                    "type": "string",
                    "description": "Tenant identifier"
                },
                "query": {
                    "type": "string",
                    "description": "Natural language search query"
                },
                "resource_type": {
                    "type": "string",
                    "enum": ["users", "groups", "both"],
                    "default": "both",
                    "description": "Type of resources to search"
                }
            },
            "required": ["tenant_id", "query"]
        }))
        .handler(move |input: ToolInput| {
            let scim_server = scim_server.clone();
            async move {
                let tenant_id = input.get_string("tenant_id")?;
                let query = input.get_string("query")?;
                let resource_type = input.get_optional_string("resource_type").unwrap_or_else(|| "both".to_string());
                
                let mut results = json!({
                    "query": query,
                    "results": {}
                });
                
                // Load users and apply in-memory filtering based on query
                let options = ListOptions::builder()
                    .count(200)  // Load more to filter in memory
                    .build();
                
                // Search users
                if resource_type == "users" || resource_type == "both" {
                    let user_response = scim_server.list_users(&tenant_id, &options).await?;
                    
                    // Filter users in memory based on query
                    let filtered_users: Vec<_> = user_response.resources.into_iter()
                        .filter(|user| matches_query(user, &query))
                        .collect();
                    
                    let users: Vec<Value> = filtered_users.into_iter().map(|user| {
                        json!({
                            "id": user.id(),
                            "username": user.username(),
                            "name": user.formatted_name(),
                            "email": user.primary_email(),
                            "department": user.department(),
                            "active": user.active()
                        })
                    }).collect();
                    
                    results["results"]["users"] = json!({
                        "count": users.len(),
                        "items": users
                    });
                }
                
                // Search groups
                if resource_type == "groups" || resource_type == "both" {
                    let group_response = scim_server.list_groups(&tenant_id, &options).await?;
                    let groups: Vec<Value> = group_response.resources.into_iter().map(|group| {
                        json!({
                            "id": group.id(),
                            "displayName": group.display_name(),
                            "description": group.description(),
                            "memberCount": group.members().len()
                        })
                    }).collect();
                    
                    results["results"]["groups"] = json!({
                        "count": groups.len(),
                        "items": groups
                    });
                }
                
                ToolResult::success(results)
            }
        })
        .build()
}

// Helper function to match users against natural language queries
fn matches_query(user: &ScimUser, query: &str) -> bool {
    let query_lower = query.to_lowercase();
    
    // Check various user fields for matches
    if let Some(username) = user.username() {
        if username.to_lowercase().contains(&query_lower) {
            return true;
        }
    }
    
    if let Some(email) = user.primary_email() {
        if email.to_lowercase().contains(&query_lower) {
            return true;
        }
    }
    
    if let Some(name) = user.formatted_name() {
        if name.to_lowercase().contains(&query_lower) {
            return true;
        }
    }
    
    if let Some(department) = user.department() {
        if department.to_lowercase().contains(&query_lower) {
            return true;
        }
    }
    
    // Specific keyword matching
    if query_lower.contains("engineer") || query_lower.contains("engineering") {
        return user.department().map_or(false, |d| d.to_lowercase().contains("engineer"));
    }
    
    if query_lower.contains("active") {
        return user.active();
    }
    
    if query_lower.contains("inactive") || query_lower.contains("disabled") {
        return !user.active();
    }
    
    false
}

fn extract_email(query: &str) -> Option<String> {
    // Simple email extraction
    let email_regex = regex::Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").ok()?;
    email_regex.find(query).map(|m| m.as_str().to_string())
}

fn extract_name(query: &str) -> Option<String> {
    // Extract quoted names or capitalize first word
    if let Some(start) = query.find('"') {
        if let Some(end) = query[start + 1..].find('"') {
            return Some(query[start + 1..start + 1 + end].to_string());
        }
    }
    
    // Take first word as potential name
    query.split_whitespace().next().map(|s| s.to_string())
}
```

## Step 4: Analytics and Reporting Tools

Add tools for generating insights:

### User Analytics Tool

```rust
fn user_analytics_tool(scim_server: ScimServer) -> Tool {
    Tool::builder()
        .name("user_analytics")
        .description("Generate analytics and insights about users")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "tenant_id": {
                    "type": "string",
                    "description": "Tenant identifier"
                },
                "report_type": {
                    "type": "string",
                    "enum": ["summary", "department_breakdown", "activity_report", "growth_trends"],
                    "default": "summary",
                    "description": "Type of analytics report to generate"
                },
                "date_range": {
                    "type": "string",
                    "description": "Date range for the report (e.g., 'last_30_days', 'last_quarter')",
                    "default": "last_30_days"
                }
            },
            "required": ["tenant_id"]
        }))
        .handler(move |input: ToolInput| {
            let scim_server = scim_server.clone();
            async move {
                let tenant_id = input.get_string("tenant_id")?;
                let report_type = input.get_optional_string("report_type").unwrap_or_else(|| "summary".to_string());
                let date_range = input.get_optional_string("date_range").unwrap_or_else(|| "last_30_days".to_string());
                
                match report_type.as_str() {
                    "summary" => generate_user_summary(&scim_server, &tenant_id).await,
                    "department_breakdown" => generate_department_breakdown(&scim_server, &tenant_id).await,
                    "activity_report" => generate_activity_report(&scim_server, &tenant_id, &date_range).await,
                    "growth_trends" => generate_growth_trends(&scim_server, &tenant_id, &date_range).await,
                    _ => ToolResult::error("Unknown report type".to_string())
                }
            }
        })
        .build()
}

async fn generate_user_summary(scim_server: &ScimServer, tenant_id: &str) -> ToolResult {
    let all_users = scim_server.list_users(tenant_id, &ListOptions::default()).await?;
    
    let total_users = all_users.total_results;
    let active_users = all_users.resources.iter().filter(|u| u.active()).count();
    let inactive_users = total_users - active_users;
    
    // Department breakdown
    let mut departments = std::collections::HashMap::new();
    for user in &all_users.resources {
        if let Some(dept) = user.department() {
            *departments.entry(dept.to_string()).or_insert(0) += 1;
        } else {
            *departments.entry("Unassigned".to_string()).or_insert(0) += 1;
        }
    }
    
    // Recent activity (last 7 days)
    let week_ago = chrono::Utc::now() - chrono::Duration::days(7);
    let recent_users = all_users.resources.iter()
        .filter(|u| u.meta().created > week_ago)
        .count();
    
    ToolResult::success(json!({
        "report": "User Summary",
        "generated_at": chrono::Utc::now(),
        "total_users": total_users,
        "active_users": active_users,
        "inactive_users": inactive_users,
        "activity_rate": format!("{:.1}%", (active_users as f64 / total_users as f64) * 100.0),
        "new_users_last_7_days": recent_users,
        "department_breakdown": departments,
        "top_departments": {
            let mut dept_vec: Vec<_> = departments.iter().collect();
            dept_vec.sort_by(|a, b| b.1.cmp(a.1));
            dept_vec.into_iter().take(5).map(|(k, v)| json!({"department": k, "count": v})).collect::<Vec<_>>()
        }
    }))
}

async fn generate_department_breakdown(scim_server: &ScimServer, tenant_id: &str) -> ToolResult {
    let all_users = scim_server.list_users(tenant_id, &ListOptions::default()).await?;
    
    let mut department_stats = std::collections::HashMap::new();
    
    for user in &all_users.resources {
        let dept = user.department().unwrap_or("Unassigned");
        let entry = department_stats.entry(dept.to_string()).or_insert_with(|| json!({
            "name": dept,
            "total_users": 0,
            "active_users": 0,
            "managers": 0,
            "recent_additions": 0
        }));
        
        entry["total_users"] = json!(entry["total_users"].as_u64().unwrap() + 1);
        
        if user.active() {
            entry["active_users"] = json!(entry["active_users"].as_u64().unwrap() + 1);
        }
        
        if user.title().map_or(false, |t| t.to_lowercase().contains("manager")) {
            entry["managers"] = json!(entry["managers"].as_u64().unwrap() + 1);
        }
        
        let week_ago = chrono::Utc::now() - chrono::Duration::days(7);
        if user.meta().created > week_ago {
            entry["recent_additions"] = json!(entry["recent_additions"].as_u64().unwrap() + 1);
        }
    }
    
    let departments: Vec<_> = department_stats.into_values().collect();
    
    ToolResult::success(json!({
        "report": "Department Breakdown",
        "generated_at": chrono::Utc::now(),
        "total_departments": departments.len(),
        "departments": departments
    }))
}
```

## Step 5: Complete MCP Server Setup

Put it all together in a complete MCP server:

```rust
use scim_server::{ScimServer, InMemoryProvider, DatabaseProvider};
use mcp_server::{McpServer, ServerInfo};
use serde_json::json;
use std::sync::Arc;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::init();
    
    // Create SCIM server with your choice of provider
    let provider = if std::env::var("DATABASE_URL").is_ok() {
        let db_url = std::env::var("DATABASE_URL")?;
        Box::new(DatabaseProvider::new(&db_url).await?) as Box<dyn Provider>
    } else {
        Box::new(InMemoryProvider::new()) as Box<dyn Provider>
    };
    
    let scim_server = Arc::new(ScimServer::builder()
        .provider(provider)
        .build());
    
    // Create MCP server with comprehensive tool set
    let mcp_server = McpServer::builder()
        .server_info(ServerInfo {
            name: "SCIM Identity Manager".to_string(),
            version: "1.0.0".to_string(),
            description: Some("AI-powered identity management through SCIM protocol".to_string()),
            author: Some("Your Organization".to_string()),
            license: Some("MIT".to_string()),
        })
        // User management tools
        .tool(create_user_tool(scim_server.clone()))
        .tool(get_user_tool(scim_server.clone()))
        .tool(list_users_tool(scim_server.clone()))
        .tool(update_user_tool(scim_server.clone()))
        .tool(delete_user_tool(scim_server.clone()))
        // Group management tools
        .tool(create_group_tool(scim_server.clone()))
        .tool(get_group_tool(scim_server.