# Tutorial: Custom Resources

This tutorial guides you through implementing custom SCIM resources beyond the standard User and Group types. You'll learn how to define custom schemas, implement resource providers, and integrate them with the SCIM server.

## Table of Contents

- [Overview](#overview)
- [Prerequisites](#prerequisites)
- [Step 1: Define Custom Schema](#step-1-define-custom-schema)
- [Step 2: Implement Resource Provider](#step-2-implement-resource-provider)
- [Step 3: Register Custom Resource](#step-3-register-custom-resource)
- [Step 4: Add Validation Logic](#step-4-add-validation-logic)
- [Step 5: Test Your Implementation](#step-5-test-your-implementation)
- [Advanced Customizations](#advanced-customizations)
- [Best Practices](#best-practices)

## Overview

Custom resources allow you to extend SCIM beyond standard User and Group resources to support organization-specific entities like Projects, Teams, Applications, or any other business objects that need SCIM-style management.

In this tutorial, we'll create a custom "Project" resource that represents software development projects with attributes like name, description, status, and team members.

## Prerequisites

- Basic understanding of SCIM concepts
- Familiarity with the SCIM server crate
- Rust development environment set up
- Completed the [First Server Tutorial](tutorial-first-server.md)

## Step 1: Define Custom Schema

First, create a schema definition for your custom resource. Create a new file `schemas/Project.json`:

```json
{
  "id": "urn:example:scim:schemas:core:2.0:Project",
  "name": "Project",
  "description": "Software Development Project",
  "attributes": [
    {
      "name": "id",
      "type": "string",
      "multiValued": false,
      "required": false,
      "caseExact": true,
      "mutability": "readOnly",
      "returned": "always",
      "uniqueness": "server",
      "description": "Unique identifier for the project"
    },
    {
      "name": "name",
      "type": "string",
      "multiValued": false,
      "required": true,
      "caseExact": false,
      "mutability": "readWrite",
      "returned": "always",
      "uniqueness": "server",
      "description": "Project name"
    },
    {
      "name": "description",
      "type": "string",
      "multiValued": false,
      "required": false,
      "caseExact": false,
      "mutability": "readWrite",
      "returned": "default",
      "uniqueness": "none",
      "description": "Project description"
    },
    {
      "name": "status",
      "type": "string",
      "multiValued": false,
      "required": true,
      "caseExact": false,
      "mutability": "readWrite",
      "returned": "always",
      "uniqueness": "none",
      "description": "Project status",
      "canonicalValues": ["active", "inactive", "archived", "planning"]
    },
    {
      "name": "repository",
      "type": "complex",
      "multiValued": false,
      "required": false,
      "mutability": "readWrite",
      "returned": "default",
      "uniqueness": "none",
      "description": "Repository information",
      "subAttributes": [
        {
          "name": "url",
          "type": "reference",
          "multiValued": false,
          "required": true,
          "caseExact": true,
          "mutability": "readWrite",
          "returned": "default",
          "uniqueness": "none",
          "description": "Repository URL"
        },
        {
          "name": "type",
          "type": "string",
          "multiValued": false,
          "required": false,
          "caseExact": false,
          "mutability": "readWrite",
          "returned": "default",
          "uniqueness": "none",
          "description": "Repository type",
          "canonicalValues": ["git", "svn", "mercurial"]
        }
      ]
    },
    {
      "name": "members",
      "type": "complex",
      "multiValued": true,
      "required": false,
      "mutability": "readWrite",
      "returned": "default",
      "uniqueness": "none",
      "description": "Project team members",
      "subAttributes": [
        {
          "name": "value",
          "type": "string",
          "multiValued": false,
          "required": true,
          "caseExact": true,
          "mutability": "readWrite",
          "returned": "default",
          "uniqueness": "none",
          "description": "User ID of the member"
        },
        {
          "name": "$ref",
          "type": "reference",
          "multiValued": false,
          "required": false,
          "caseExact": true,
          "mutability": "readWrite",
          "returned": "default",
          "uniqueness": "none",
          "description": "URI reference to the user resource"
        },
        {
          "name": "role",
          "type": "string",
          "multiValued": false,
          "required": false,
          "caseExact": false,
          "mutability": "readWrite",
          "returned": "default",
          "uniqueness": "none",
          "description": "Member's role in the project",
          "canonicalValues": ["owner", "maintainer", "developer", "viewer"]
        },
        {
          "name": "display",
          "type": "string",
          "multiValued": false,
          "required": false,
          "caseExact": false,
          "mutability": "readWrite",
          "returned": "default",
          "uniqueness": "none",
          "description": "Display name of the member"
        }
      ]
    },
    {
      "name": "created",
      "type": "dateTime",
      "multiValued": false,
      "required": false,
      "mutability": "readOnly",
      "returned": "default",
      "uniqueness": "none",
      "description": "Project creation date"
    },
    {
      "name": "lastActivity",
      "type": "dateTime",
      "multiValued": false,
      "required": false,
      "mutability": "readWrite",
      "returned": "default",
      "uniqueness": "none",
      "description": "Last activity timestamp"
    },
    {
      "name": "tags",
      "type": "string",
      "multiValued": true,
      "required": false,
      "caseExact": false,
      "mutability": "readWrite",
      "returned": "default",
      "uniqueness": "none",
      "description": "Project tags for categorization"
    }
  ]
}
```

## Step 2: Implement Resource Provider

Create a resource provider for your custom Project resource:

```rust
// src/providers/project_provider.rs

use scim_server::providers::ResourceProvider;
use scim_server::resource::{Resource, ResourceId};
use scim_server::error::ScimError;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Project {
    pub id: Option<ResourceId>,
    pub name: String,
    pub description: Option<String>,
    pub status: ProjectStatus,
    pub repository: Option<Repository>,
    pub members: Vec<ProjectMember>,
    pub created: Option<DateTime<Utc>>,
    pub last_activity: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectStatus {
    Active,
    Inactive,
    Archived,
    Planning,
}

impl ProjectStatus {
    fn from_str(s: &str) -> Result<Self, ScimError> {
        match s.to_lowercase().as_str() {
            "active" => Ok(ProjectStatus::Active),
            "inactive" => Ok(ProjectStatus::Inactive),
            "archived" => Ok(ProjectStatus::Archived),
            "planning" => Ok(ProjectStatus::Planning),
            _ => Err(ScimError::invalid_value(&format!("Invalid project status: {}", s))),
        }
    }
    
    fn as_str(&self) -> &'static str {
        match self {
            ProjectStatus::Active => "active",
            ProjectStatus::Inactive => "inactive",
            ProjectStatus::Archived => "archived",
            ProjectStatus::Planning => "planning",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Repository {
    pub url: String,
    pub repo_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProjectMember {
    pub user_id: String,
    pub user_ref: Option<String>,
    pub role: Option<String>,
    pub display_name: Option<String>,
}

pub struct ProjectProvider {
    projects: Arc<RwLock<HashMap<ResourceId, Project>>>,
}

impl ProjectProvider {
    pub fn new() -> Self {
        Self {
            projects: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    fn project_to_resource(&self, project: &Project) -> Resource {
        let mut attributes = HashMap::new();
        
        if let Some(id) = &project.id {
            attributes.insert("id".to_string(), Value::String(id.to_string()));
        }
        
        attributes.insert("name".to_string(), Value::String(project.name.clone()));
        
        if let Some(description) = &project.description {
            attributes.insert("description".to_string(), Value::String(description.clone()));
        }
        
        attributes.insert("status".to_string(), Value::String(project.status.as_str().to_string()));
        
        if let Some(repo) = &project.repository {
            let mut repo_obj = HashMap::new();
            repo_obj.insert("url".to_string(), Value::String(repo.url.clone()));
            if let Some(repo_type) = &repo.repo_type {
                repo_obj.insert("type".to_string(), Value::String(repo_type.clone()));
            }
            attributes.insert("repository".to_string(), Value::Object(repo_obj));
        }
        
        if !project.members.is_empty() {
            let members: Vec<Value> = project.members.iter().map(|member| {
                let mut member_obj = HashMap::new();
                member_obj.insert("value".to_string(), Value::String(member.user_id.clone()));
                
                if let Some(user_ref) = &member.user_ref {
                    member_obj.insert("$ref".to_string(), Value::String(user_ref.clone()));
                }
                
                if let Some(role) = &member.role {
                    member_obj.insert("role".to_string(), Value::String(role.clone()));
                }
                
                if let Some(display) = &member.display_name {
                    member_obj.insert("display".to_string(), Value::String(display.clone()));
                }
                
                Value::Object(member_obj)
            }).collect();
            
            attributes.insert("members".to_string(), Value::Array(members));
        }
        
        if let Some(created) = project.created {
            attributes.insert("created".to_string(), Value::String(created.to_rfc3339()));
        }
        
        if let Some(last_activity) = project.last_activity {
            attributes.insert("lastActivity".to_string(), Value::String(last_activity.to_rfc3339()));
        }
        
        if !project.tags.is_empty() {
            let tags: Vec<Value> = project.tags.iter()
                .map(|tag| Value::String(tag.clone()))
                .collect();
            attributes.insert("tags".to_string(), Value::Array(tags));
        }
        
        Resource {
            id: project.id.clone(),
            schemas: vec!["urn:example:scim:schemas:core:2.0:Project".to_string()],
            attributes,
            meta: None,
        }
    }
    
    fn resource_to_project(&self, resource: &Resource) -> Result<Project, ScimError> {
        let name = resource.attributes.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ScimError::invalid_value("Project name is required"))?
            .to_string();
        
        let description = resource.attributes.get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        let status = resource.attributes.get("status")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ScimError::invalid_value("Project status is required"))?;
        let status = ProjectStatus::from_str(status)?;
        
        let repository = if let Some(repo_value) = resource.attributes.get("repository") {
            let repo_obj = repo_value.as_object()
                .ok_or_else(|| ScimError::invalid_value("Repository must be an object"))?;
            
            let url = repo_obj.get("url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ScimError::invalid_value("Repository URL is required"))?
                .to_string();
            
            let repo_type = repo_obj.get("type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            
            Some(Repository { url, repo_type })
        } else {
            None
        };
        
        let members = if let Some(Value::Array(members_array)) = resource.attributes.get("members") {
            members_array.iter().map(|member| {
                let member_obj = member.as_object()
                    .ok_or_else(|| ScimError::invalid_value("Member must be an object"))?;
                
                let user_id = member_obj.get("value")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ScimError::invalid_value("Member value is required"))?
                    .to_string();
                
                let user_ref = member_obj.get("$ref")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                
                let role = member_obj.get("role")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                
                let display_name = member_obj.get("display")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                
                Ok(ProjectMember {
                    user_id,
                    user_ref,
                    role,
                    display_name,
                })
            }).collect::<Result<Vec<_>, ScimError>>()?
        } else {
            Vec::new()
        };
        
        let created = resource.attributes.get("created")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));
        
        let last_activity = resource.attributes.get("lastActivity")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));
        
        let tags = if let Some(Value::Array(tags_array)) = resource.attributes.get("tags") {
            tags_array.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect()
        } else {
            Vec::new()
        };
        
        Ok(Project {
            id: resource.id.clone(),
            name,
            description,
            status,
            repository,
            members,
            created,
            last_activity,
            tags,
        })
    }
}

#[async_trait]
impl ResourceProvider for ProjectProvider {
    async fn create(&self, resource: Resource) -> Result<Resource, ScimError> {
        let mut project = self.resource_to_project(&resource)?;
        
        // Generate ID and set creation timestamp
        let id = ResourceId::new();
        project.id = Some(id.clone());
        project.created = Some(Utc::now());
        project.last_activity = Some(Utc::now());
        
        // Store the project
        {
            let mut projects = self.projects.write().await;
            projects.insert(id, project.clone());
        }
        
        Ok(self.project_to_resource(&project))
    }
    
    async fn get_by_id(&self, id: &ResourceId) -> Result<Option<Resource>, ScimError> {
        let projects = self.projects.read().await;
        
        match projects.get(id) {
            Some(project) => Ok(Some(self.project_to_resource(project))),
            None => Ok(None),
        }
    }
    
    async fn update(&self, id: &ResourceId, resource: Resource) -> Result<Resource, ScimError> {
        let mut updated_project = self.resource_to_project(&resource)?;
        updated_project.id = Some(id.clone());
        updated_project.last_activity = Some(Utc::now());
        
        let mut projects = self.projects.write().await;
        
        // Check if project exists
        if !projects.contains_key(id) {
            return Err(ScimError::not_found(&format!("Project {} not found", id)));
        }
        
        // Preserve creation date from existing project
        if let Some(existing) = projects.get(id) {
            updated_project.created = existing.created;
        }
        
        projects.insert(id.clone(), updated_project.clone());
        
        Ok(self.project_to_resource(&updated_project))
    }
    
    async fn delete(&self, id: &ResourceId) -> Result<(), ScimError> {
        let mut projects = self.projects.write().await;
        
        match projects.remove(id) {
            Some(_) => Ok(()),
            None => Err(ScimError::not_found(&format!("Project {} not found", id))),
        }
    }
    
    async fn list(&self, filter: Option<&str>, start_index: usize, count: usize) 
        -> Result<(Vec<Resource>, usize), ScimError> {
        
        let projects = self.projects.read().await;
        let mut project_list: Vec<&Project> = projects.values().collect();
        
        // Apply filtering
        if let Some(filter_str) = filter {
            project_list = self.apply_filter(project_list, filter_str)?;
        }
        
        // Apply pagination
        let total_results = project_list.len();
        let start_idx = start_index.saturating_sub(1);
        let end_idx = (start_idx + count).min(total_results);
        
        let paginated_projects = project_list[start_idx..end_idx].to_vec();
        let resources: Vec<Resource> = paginated_projects
            .iter()
            .map(|project| self.project_to_resource(project))
            .collect();
        
        Ok((resources, total_results))
    }
}

impl ProjectProvider {
    fn apply_filter(&self, projects: Vec<&Project>, filter: &str) -> Result<Vec<&Project>, ScimError> {
        // Simple filter implementation for demonstration
        // In production, you'd want a more sophisticated filter parser
        
        if filter.contains("status eq ") {
            let status_str = filter.split("status eq ")
                .nth(1)
                .and_then(|s| s.trim_matches('"').split_whitespace().next())
                .ok_or_else(|| ScimError::invalid_filter("Invalid status filter"))?;
            
            let target_status = ProjectStatus::from_str(status_str)?;
            
            Ok(projects.into_iter()
                .filter(|project| project.status == target_status)
                .collect())
        } else if filter.contains("name co ") {
            let name_part = filter.split("name co ")
                .nth(1)
                .and_then(|s| s.trim_matches('"').split_whitespace().next())
                .ok_or_else(|| ScimError::invalid_filter("Invalid name filter"))?;
            
            Ok(projects.into_iter()
                .filter(|project| project.name.to_lowercase().contains(&name_part.to_lowercase()))
                .collect())
        } else {
            // No filter or unsupported filter - return all
            Ok(projects)
        }
    }
}
```

## Step 3: Register Custom Resource

Integrate your custom resource with the SCIM server:

```rust
// src/bin/custom_server.rs

use scim_server::{ScimServer, ScimServerBuilder};
use scim_server::schema::SchemaRegistry;
use scim_server::providers::ResourceProvider;
use std::sync::Arc;

mod providers;
use providers::project_provider::ProjectProvider;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load schemas including our custom Project schema
    let schema_registry = SchemaRegistry::from_schema_dir("schemas/")?;
    
    // Create providers
    let user_provider = Arc::new(scim_server::providers::InMemoryProvider::new());
    let group_provider = Arc::new(scim_server::providers::InMemoryProvider::new());
    let project_provider = Arc::new(ProjectProvider::new());
    
    // Build the server with custom resource
    let server = ScimServerBuilder::new(schema_registry)
        .add_resource_provider("User", user_provider)
        .add_resource_provider("Group", group_provider)
        .add_resource_provider("Project", project_provider)
        .build()?;
    
    // Setup HTTP routes
    let app = axum::Router::new()
        .route("/Users", axum::routing::get(list_users).post(create_user))
        .route("/Users/:id", axum::routing::get(get_user).put(update_user).delete(delete_user))
        .route("/Groups", axum::routing::get(list_groups).post(create_group))
        .route("/Groups/:id", axum::routing::get(get_group).put(update_group).delete(delete_group))
        .route("/Projects", axum::routing::get(list_projects).post(create_project))
        .route("/Projects/:id", axum::routing::get(get_project).put(update_project).delete(delete_project))
        .with_state(Arc::new(server));
    
    println!("SCIM server with custom Project resources running on http://0.0.0.0:3000");
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

// HTTP handlers for Project resource
async fn list_projects(
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
    axum::extract::State(server): axum::extract::State<Arc<ScimServer>>,
) -> Result<axum::Json<serde_json::Value>, ScimError> {
    let filter = params.get("filter").map(|s| s.as_str());
    let start_index = params.get("startIndex")
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let count = params.get("count")
        .and_then(|s| s.parse().ok())
        .unwrap_or(20);
    
    let (resources, total) = server.list_resources("Project", filter, start_index, count).await?;
    
    let response = json!({
        "schemas": ["urn:ietf:params:scim:api:messages:2.0:ListResponse"],
        "totalResults": total,
        "startIndex": start_index,
        "itemsPerPage": resources.len(),
        "Resources": resources
    });
    
    Ok(axum::Json(response))
}

async fn create_project(
    axum::extract::State(server): axum::extract::State<Arc<ScimServer>>,
    axum::Json(payload): axum::Json<serde_json::Value>,
) -> Result<(axum::http::StatusCode, axum::Json<serde_json::Value>), ScimError> {
    let resource = server.create_resource("Project", payload).await?;
    Ok((axum::http::StatusCode::CREATED, axum::Json(serde_json::to_value(resource)?)))
}

async fn get_project(
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::State(server): axum::extract::State<Arc<ScimServer>>,
) -> Result<axum::Json<serde_json::Value>, ScimError> {
    let resource_id = scim_server::resource::ResourceId::new(&id)
        .map_err(|e| ScimError::invalid_value(&e.to_string()))?;
    
    match server.get_resource("Project", &resource_id).await? {
        Some(resource) => Ok(axum::Json(serde_json::to_value(resource)?)),
        None => Err(ScimError::not_found(&format!("Project {} not found", id))),
    }
}

async fn update_project(
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::State(server): axum::extract::State<Arc<ScimServer>>,
    axum::Json(payload): axum::Json<serde_json::Value>,
) -> Result<axum::Json<serde_json::Value>, ScimError> {
    let resource_id = scim_server::resource::ResourceId::new(&id)
        .map_err(|e| ScimError::invalid_value(&e.to_string()))?;
    
    let resource = server.update_resource("Project", &resource_id, payload).await?;
    Ok(axum::Json(serde_json::to_value(resource)?))
}

async fn delete_project(
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::State(server): axum::extract::State<Arc<ScimServer>>,
) -> Result<axum::http::StatusCode, ScimError> {
    let resource_id = scim_server::resource::ResourceId::new(&id)
        .map_err(|e| ScimError::invalid_value(&e.to_string()))?;
    
    server.delete_resource("Project", &resource_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

// Additional handlers for User and Group resources would be similar...
```

## Step 4: Add Validation Logic

Create custom validation logic for your Project resource:

```rust
// src/validation/project_validator.rs

use scim_server::resource::Resource;
use scim_server::error::ScimError;
use serde_json::Value;
use std::collections::HashSet;

pub struct ProjectValidator {
    valid_roles: HashSet<String>,
    valid_repo_types: HashSet<String>,
}

impl ProjectValidator {
    pub fn new() -> Self {
        let mut valid_roles = HashSet::new();
        valid_roles.insert("owner".to_string());
        valid_roles.insert("maintainer".to_string());
        valid_roles.insert("developer".to_string());
        valid_roles.insert("viewer".to_string());
        
        let mut valid_repo_types = HashSet::new();
        valid_repo_types.insert("git".to_string());
        valid_repo_types.insert("svn".to_string());
        valid_repo_types.insert("mercurial".to_string());
        
        Self {
            valid_roles,
            valid_repo_types,
        }
    }
    
    pub fn validate_project(&self, resource: &Resource) -> Result<(), ScimError> {
        self.validate_project_name(resource)?;
        self.validate_project_status(resource)?;
        self.validate_repository(resource)?;
        self.validate_members(resource)?;
        self.validate_tags(resource)?;
        
        Ok(())
    }
    
    fn validate_project_name(&self, resource: &Resource) -> Result<(), ScimError> {
        if let Some(Value::String(name)) = resource.attributes.get("name") {
            if name.is_empty() {
                return Err(ScimError::invalid_value("Project name cannot be empty"));
            }
            
            if name.len() > 100 {
                return Err(ScimError::invalid_value("Project name cannot exceed 100 characters"));
            }
            
            // Business rule: no special characters in project names
            if name.chars().any(|c| !c.is_alphanumeric() && !c.is_whitespace() && c != '-' && c != '_') {
                return Err(ScimError::invalid_value(
                    "Project name can only contain letters, numbers, spaces, hyphens, and underscores"
                ));
            }
        }
        
        Ok(())
    }
    
    fn validate_project_status(&self, resource: &Resource) -> Result<(), ScimError> {
        if let Some(Value::String(status)) = resource.attributes.get("status") {
            let valid_statuses = ["active", "inactive", "archived", "planning"];
            if !valid_statuses.contains(&status.as_str()) {
                return Err(ScimError::invalid_value(&format!(
                    "Invalid project status: {}. Must be one of: {}",
                    status,
                    valid_statuses.join(", ")
                )));
            }
        }
        
        Ok(())
    }
    
    fn validate_repository(&self, resource: &Resource) -> Result<(), ScimError> {
        if let Some(repo_value) = resource.attributes.get("repository") {
            let repo_obj = repo_value.as_object()
                .ok_or_else(|| ScimError::invalid_value("Repository must be an object"))?;
            
            // Validate URL format
            if let Some(Value::String(url)) = repo_obj.get("url") {
                if !url.starts_with("http://") && !url.starts_with("https://") && !url.starts_with("git://") {
                    return Err(ScimError::invalid_value("Repository URL must be a valid URL"));
                }
            }
            
            // Validate repository type
            if let Some(Value::String(repo_type)) = repo_obj.get("type") {
                if !self.valid_repo_types.contains(repo_type) {
                    return Err(ScimError::invalid_value(&format!(
                        "Invalid repository type: {}. Must be one of: {}",
                        repo_type,
                        self.valid_repo_types.iter().cloned().collect::<Vec<_>>().join(", ")
                    )));
                }
            }
        }
        
        Ok(())
    }
    
    fn validate_members(&self, resource: &Resource) -> Result<(), ScimError> {
        if let Some(Value::Array(members)) = resource.attributes.get("members") {
            for (i, member) in members.iter().enumerate() {
                let member_obj = member.as_object()
                    .ok_or_else(|| ScimError::invalid_value(&format!("Member[{}] must be an object", i)))?;
                
                // Validate user ID is present
                if !member_obj.contains_key("value") {
                    return Err(ScimError::invalid_value(&format!("Member[{}] missing required 'value' field", i)));
                }
                
                // Validate role if present
                if let Some(Value::String(role)) = member_obj.get("role") {
                    if !self.valid_roles.contains(role) {
                        return Err(ScimError::invalid_value(&format!(
                            "Invalid member role: {}. Must be one of: {}",
                            role,
                            self.valid_roles.iter().cloned().collect::<Vec<_>>().join(", ")
                        )));
                    }
                }
            }
        }
        
        Ok(())
    }
    
    fn validate_tags(&self, resource: &Resource) -> Result<(), ScimError> {
        if let Some(Value::Array(tags)) = resource.attributes.get("tags") {
            for (i, tag) in tags.iter().enumerate() {
                if let Value::String(tag_str) = tag {
                    if tag_str.is_empty() {
                        return Err(ScimError::invalid_value(&format!("Tag[{}] cannot be empty", i)));
                    }
                    
                    if tag_str.len() > 50 {
                        return Err(ScimError::invalid_value(&format!("Tag[{}] cannot exceed 50 characters", i)));
                    }
                } else {
                    return Err(ScimError::invalid_value(&format!("Tag[{}] must be a string", i)));
                }
            }
        }
        
        Ok(())
    }
}
```

## Step 5: Test Your Implementation

Create comprehensive tests for your custom resource:

```rust
// tests/project_resource_tests.rs

use scim_server::{ScimServer, ScimServerBuilder};
use scim_server::schema::SchemaRegistry;
use scim_server::resource::ResourceId;
use serde_json::json;
use std::sync::Arc;

mod common;
use common::providers::ProjectProvider;

#[tokio::test]
async fn test_create_project() -> Result<(), Box<dyn std::error::Error>> {
    let schema_registry = SchemaRegistry::from_schema_dir("schemas/")?;
    let project_provider = Arc::new(ProjectProvider::new());
    
    let server = ScimServerBuilder::new(schema_registry)
        .add_resource_provider("Project", project_provider)
        .build()?;
    
    let project_data = json!({
        "schemas": ["urn:example:scim:schemas:core:2.0:Project"],
        "name": "My Test Project",
        "description": "A project for testing",
        "status": "active",
        "repository": {
            "url": "https://github.com/example/test-project",
            "type": "git"
        },
        "tags": ["rust", "scim", "test"]
    });
    
    let created = server.create_resource("Project", project_data).await?;
    
    assert!(created.id.is_some());
    assert_eq!(created.attributes.get("name").unwrap().as_str().unwrap(), "My Test Project");
    assert_eq!(created.attributes.get("status").unwrap().as_str().unwrap(), "active");
    
    Ok(())
}

#[tokio::test]
async fn test_project_with_members() -> Result<(), Box<dyn std::error::Error>> {
    let schema_registry = SchemaRegistry::from_schema_dir("schemas/")?;
    let project_provider = Arc::new(ProjectProvider::new());
    
    let server = ScimServerBuilder::new(schema_registry)
        .add_resource_provider("Project", project_provider)
        .build()?;
    
    let project_data = json!({
        "schemas": ["urn:example:scim:schemas:core:2.0:Project"],
        "name": "Team Project",
        "status": "active",
        "members": [
            {
                "value": "user123",
                "$ref": "/Users/user123",
                "role": "owner",
                "display": "John Doe"
            },
            {
                "value": "user456",
                "$ref": "/Users/user456",
                "role": "developer",
                "display": "Jane Smith"
            }
        ]
    });
    
    let created = server.create_resource("Project", project_data).await?;
    
    let members = created.attributes.get("members").unwrap().as_array().unwrap();
    assert_eq!(members.len(), 2);
    assert_eq!(members[0].get("role").unwrap().as_str().unwrap(), "owner");
    assert_eq!(members[1].get("role").unwrap().as_str().unwrap(), "developer");
    
    Ok(())
}

#[tokio::test]
async fn test_project_validation() -> Result<(), Box<dyn std::error::Error>> {
    let schema_registry = SchemaRegistry::from_schema_dir("schemas/")?;
    let project_provider = Arc::new(ProjectProvider::new());
    
    let server = ScimServerBuilder::new(schema_registry)
        .add_resource_provider("Project", project_provider)
        .build()?;
    
    // Test missing required field
    let invalid_project = json!({
        "schemas": ["urn:example:scim:schemas:core:2.0:Project"],
        "description": "Missing name"
    });
    
    let result = server.create_resource("Project", invalid_project).await;
    assert!(result.is_err());
    
    // Test invalid status
    let invalid_status_project = json!({
        "schemas": ["urn:example:scim:schemas:core:2.0:Project"],
        "name": "Test Project",
        "status": "invalid_status"
    });
    
    let result = server.create_resource("Project", invalid_status_project).await;
    assert!(result.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_project_filtering() -> Result<(), Box<dyn std::error::Error>> {
    let schema_registry = SchemaRegistry::from_schema_dir("schemas/")?;
    let project_provider = Arc::new(ProjectProvider::new());
    
    let server = ScimServerBuilder::new(schema_registry)
        .add_resource_provider("Project", project_provider)
        .build()?;
    
    // Create test projects
    let active_project = json!({
        "schemas": ["urn:example:scim:schemas:core:2.0:Project"],
        "name": "Active Project",
        "status": "active"
    });
    
    let archived_project = json!({
        "schemas": ["urn:example:scim:schemas:core:2.0:Project"],
        "name": "Archived Project",
        "status": "archived"
    });
    
    server.create_resource("Project", active_project).await?;
    server.create_resource("Project", archived_project).await?;
    
    // Test filtering by status
    let (active_projects, _) = server.list_resources(
        "Project", 
        Some("status eq \"active\""), 
        1, 
        10
    ).await?;
    
    assert_eq!(active_projects.len(), 1);
    assert_eq!(active_projects[0].attributes.get("name").unwrap().as_str().unwrap(), "Active Project");
    
    Ok(())
}
```

## Advanced Customizations

### Custom Attribute Types

You can create custom attribute types with specific validation:

```rust
// src/resource/value_objects/project_status.rs

use scim_server::error::ScimError;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProjectStatus {
    Active,
    Inactive,
    Archived,
    Planning,
}

impl ProjectStatus {
    pub fn from_str(s: &str) -> Result<Self, ScimError> {
        match s.to_lowercase().as_str() {
            "active" => Ok(ProjectStatus::Active),
            "inactive" => Ok(ProjectStatus::Inactive),
            "archived" => Ok(ProjectStatus::Archived),
            "planning" => Ok(ProjectStatus::Planning),
            _ => Err(ScimError::invalid_value(&format!(
                "Invalid project status: {}. Must be one of: active, inactive, archived, planning", 
                s
            ))),
        }
    }
    
    pub fn is_modifiable(&self) -> bool {
        !matches!(self, ProjectStatus::Archived)
    }
    
    pub fn can_transition_to(&self, new_status: &ProjectStatus) -> bool {
        match (self, new_status) {
            // Can always transition to same status
            (a, b) if a == b => true,
            
            // Planning can go to active or inactive
            (ProjectStatus::Planning, ProjectStatus::Active) => true,
            (ProjectStatus::Planning, ProjectStatus::Inactive) => true,
            
            // Active can go to inactive or archived
            (ProjectStatus::Active, ProjectStatus::Inactive) => true,
            (ProjectStatus::Active, ProjectStatus::Archived) => true,
            
            // Inactive can go to active or archived
            (ProjectStatus::Inactive, ProjectStatus::Active) => true,
            (ProjectStatus::Inactive, ProjectStatus::Archived) => true,
            
            // Archived cannot transition to anything else
            (ProjectStatus::Archived, _) => false,
            
            // All other transitions are invalid
            _ => false,
        }
    }
}

impl fmt::Display for ProjectStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            ProjectStatus::Active => "active",
            ProjectStatus::Inactive => "inactive",
            ProjectStatus::Archived => "archived",
            ProjectStatus::Planning => "planning",
        })
    }
}
```

### Resource Relationships

Implement relationships between custom resources and standard SCIM resources:

```rust
// src/relationships/project_relationships.rs

use scim_server::resource::{Resource, ResourceId};
use scim_server::providers::ResourceProvider;
use scim_server::error::ScimError;
use serde_json::Value;
use std::sync::Arc;

pub struct ProjectRelationshipManager {
    user_provider: Arc<dyn ResourceProvider>,
    project_provider: Arc<dyn ResourceProvider>,
}

impl ProjectRelationshipManager {
    pub fn new(
        user_provider: Arc<dyn ResourceProvider>,
        project_provider: Arc<dyn ResourceProvider>,
    ) -> Self {
        Self {
            user_provider,
            project_provider,
        }
    }
    
    pub async fn add_member_to_project(
        &self,
        project_id: &ResourceId,
        user_id: &ResourceId,
        role: &str,
    ) -> Result<Resource, ScimError> {
        // Verify user exists
        let user = self.user_provider.get_by_id(user_id).await?
            .ok_or_else(|| ScimError::not_found(&format!("User {} not found", user_id)))?;
        
        // Get current project
        let mut project = self.project_provider.get_by_id(project_id).await?
            .ok_or_else(|| ScimError::not_found(&format!("Project {} not found", project_id)))?;
        
        // Add member to project
        let display_name = user.attributes.get("displayName")
            .or_else(|| user.attributes.get("userName"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        let new_member = json!({
            "value": user_id.to_string(),
            "$ref": format!("/Users/{}", user_id),
            "role": role,
            "display": display_name
        });
        
        // Update members array
        let members = project.attributes.entry("members".to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
        
        if let Value::Array(members_array) = members {
            // Check if user is already a member
            let already_member = members_array.iter().any(|member| {
                member.get("value")
                    .and_then(|v| v.as_str())
                    .map_or(false, |id| id == user_id.as_str())
            });
            
            if already_member {
                return Err(ScimError::invalid_value("User is already a project member"));
            }
            
            members_array.push(new_member);
        }
        
        // Update the project
        self.project_provider.update(project_id, project).await
    }
    
    pub async fn remove_member_from_project(
        &self,
        project_id: &ResourceId,
        user_id: &ResourceId,
    ) -> Result<Resource, ScimError> {
        let mut project = self.project_provider.get_by_id(project_id).await?
            .ok_or_else(|| ScimError::not_found(&format!("Project {} not found", project_id)))?;
        
        if let Some(Value::Array(members)) = project.attributes.get_mut("members") {
            let original_len = members.len();
            members.retain(|member| {
                member.get("value")
                    .and_then(|v| v.as_str())
                    .map_or(true, |id| id != user_id.as_str())
            });
            
            if members.len() == original_len {
                return Err(ScimError::not_found("User is not a member of this project"));
            }
        }
        
        self.project_provider.update(project_id, project).await
    }
    
    pub async fn get_user_projects(&self, user_id: &ResourceId) -> Result<Vec<Resource>, ScimError> {
        let (all_projects, _) = self.project_provider.list(None, 1, 1000).await?;
        
        let user_projects = all_projects.into_iter()
            .filter(|project| {
                if let Some(Value::Array(members)) = project.attributes.get("members") {
                    members.iter().any(|member| {
                        member.get("value")
                            .and_then(|v| v.as_str())
                            .map_or(false, |id| id == user_id.as_str())
                    })
                } else {
                    false
                }
            })
            .collect();
        
        Ok(user_projects)
    }
}
```

## Best Practices

### Schema Design

1. **Use Meaningful URIs**: Choose descriptive schema IDs that reflect your organization
2. **Follow SCIM Conventions**: Use similar attribute patterns to standard SCIM resources
3. **Plan for Evolution**: Design schemas that can be extended without breaking changes
4. **Validate Thoroughly**: Include comprehensive validation rules in your schema

### Provider Implementation

1. **Error Handling**: Provide clear, actionable error messages
2. **Performance**: Implement efficient filtering and pagination
3. **Consistency**: Maintain data consistency across operations
4. **Logging**: Add logging for debugging and monitoring

### Testing Strategy

1. **Unit Tests**: Test individual components in isolation
2. **Integration Tests**: Test complete workflows
3. **Validation Tests**: Test all validation rules
4. **Performance Tests**: Ensure acceptable performance under load

### Security Considerations

1. **Authorization**: Implement proper access controls for custom resources
2. **Input Validation**: Validate all inputs thoroughly
3. **Audit Logging**: Log all operations on custom resources
4. **Data Sanitization**: Ensure sensitive data is handled appropriately

## Example: Complete Project Server

Here's a complete example bringing everything together:

```rust
// src/bin/project_server.rs

use scim_server::{ScimServer, ScimServerBuilder};
use scim_server::schema::SchemaRegistry;
use scim_server::providers::ResourceProvider;
use scim_server::error::ScimError;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;

mod providers;
mod validation;
mod relationships;

use providers::ProjectProvider;
use validation::ProjectValidator;
use relationships::ProjectRelationshipManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();
    
    // Load schemas
    let schema_registry = SchemaRegistry::from_schema_dir("schemas/")?;
    
    // Create providers
    let user_provider = Arc::new(scim_server::providers::InMemoryProvider::new());
    let group_provider = Arc::new(scim_server::providers::InMemoryProvider::new());
    let project_provider = Arc::new(ProjectProvider::new());
    
    // Create relationship manager
    let relationship_manager = Arc::new(ProjectRelationshipManager::new(
        user_provider.clone(),
        project_provider.clone(),
    ));
    
    // Build the server
    let server = ScimServerBuilder::new(schema_registry)
        .add_resource_provider("User", user_provider)
        .add_resource_provider("Group", group_provider)
        .add_resource_provider("Project", project_provider)
        .build()?;
    
    let server_state = AppState {
        server: Arc::new(server),
        relationship_manager,
        validator: Arc::new(ProjectValidator::new()),
    };
    
    // Build router
    let app = Router::new()
        // Standard SCIM endpoints
        .route("/Users", get(list_users).post(create_user))
        .route("/Users/:id", get(get_user).put(update_user).delete(delete_user))
        .route("/Groups", get(list_groups).post(create_group))
        .route("/Groups/:id", get(get_group).put(update_group).delete(delete_group))
        
        // Custom Project endpoints
        .route("/Projects", get(list_projects).post(create_project))
        .route("/Projects/:id", get(get_project).put(update_project).delete(delete_project))
        
        // Custom relationship endpoints
        .route("/Projects/:id/members", post(add_project_member))
        .route("/Projects/:id/members/:user_id", delete(remove_project_member))
        .route("/Users/:id/projects", get(get_user_projects))
        
        // Service provider configuration
        .route("/ServiceProviderConfig", get(get_service_provider_config))
        
        .layer(
            ServiceBuilder::new()
                .layer(CorsLayer::permissive())
        )
        .with_state(server_state);
    
    println!("SCIM server with custom Project resources running on http://0.0.0.0:3000");
    println!("Available endpoints:");
    println!("  GET    /Users");
    println!("  POST   /Users");
    println!("  GET    /Users/:id");
    println!("  PUT    /Users/:id");
    println!("  DELETE /Users/:id");
    println!("  GET    /Groups");
    println!("  POST   /Groups");
    println!("  GET    /Groups/:id");
    println!("  PUT    /Groups/:id");
    println!("  DELETE /Groups/:id");
    println!("  GET    /Projects");
    println!("  POST   /Projects");
    println!("  GET    /Projects/:id");
    println!("  PUT    /Projects/:id");
    println!("  DELETE /Projects/:id");
    println!("  POST   /Projects/:id/members");
    println!("  DELETE /Projects/:id/members/:user_id");
    println!("  GET    /Users/:id/projects");
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

#[derive(Clone)]
struct AppState {
    server: Arc<ScimServer>,
    relationship_manager: Arc<ProjectRelationshipManager>,
    validator: Arc<ProjectValidator>,
}

// Project-specific handlers
async fn list_projects(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Result<Json<Value>, ScimError> {
    let filter = params.get("filter").map(|s| s.as_str());
    let start_index = params.get("startIndex").and_then(|s| s.parse().ok()).unwrap_or(1);
    let count = params.get("count").and_then(|s| s.parse().ok()).unwrap_or(20);
    
    let (resources, total) = state.server.list_resources("Project", filter, start_index, count).await?;
    
    Ok(Json(json!({
        "schemas": ["urn:ietf:params:scim:api:messages:2.0:ListResponse"],
        "totalResults": total,
        "startIndex": start_index,
        "itemsPerPage": resources.len(),
        "Resources": resources
    })))
+}

+async fn create_project(
+    State(state): State<AppState>,
+    Json(payload): Json<Value>,
+) -> Result<(StatusCode, Json<Value>), ScimError> {
+    // Create resource through server (includes schema validation)
+    let resource = state.server.create_resource("Project", payload).await?;
+    
+    // Apply additional business validation
+    state.validator.validate_project(&resource)?;
+    
+    Ok((StatusCode::CREATED, Json(serde_json::to_value(resource)?)))
+}
+
+async fn get_project(
+    Path(id): Path<String>,
+    State(state): State<AppState>,
+) -> Result<Json<Value>, ScimError> {
+    let resource_id = scim_server::resource::ResourceId::new(&id)
+        .map_err(|e| ScimError::invalid_value(&e.to_string()))?;
+    
+    match state.server.get_resource("Project", &resource_id).await? {
+        Some(resource) => Ok(Json(serde_json::to_value(resource)?)),
+        None => Err(ScimError::not_found(&format!("Project {} not found", id))),
+    }
+}
+
+async fn update_project(
+    Path(id): Path<String>,
+    State(state): State<AppState>,
+    Json(payload): Json<Value>,
+) -> Result<Json<Value>, ScimError> {
+    let resource_id = scim_server::resource::ResourceId::new(&id)
+        .map_err(|e| ScimError::invalid_value(&e.to_string()))?;
+    
+    let resource = state.server.update_resource("Project", &resource_id, payload).await?;
+    state.validator.validate_project(&resource)?;
+    
+    Ok(Json(serde_json::to_value(resource)?))
+}
+
+async fn delete_project(
+    Path(id): Path<String>,
+    State(state): State<AppState>,
+) -> Result<StatusCode, ScimError> {
+    let resource_id = scim_server::resource::ResourceId::new(&id)
+        .map_err(|e| ScimError::invalid_value(&e.to_string()))?;
+    
+    state.server.delete_resource("Project", &resource_id).await?;
+    Ok(StatusCode::NO_CONTENT)
+}
+
+// Relationship handlers
+async fn add_project_member(
+    Path(id): Path<String>,
+    State(state): State<AppState>,
+    Json(payload): Json<Value>,
+) -> Result<Json<Value>, ScimError> {
+    let project_id = scim_server::resource::ResourceId::new(&id)
+        .map_err(|e| ScimError::invalid_value(&e.to_string()))?;
+    
+    let user_id_str = payload.get("userId")
+        .and_then(|v| v.as_str())
+        .ok_or_else(|| ScimError::invalid_value("userId is required"))?;
+    
+    let user_id = scim_server::resource::ResourceId::new(user_id_str)
+        .map_err(|e| ScimError::invalid_value(&e.to_string()))?;
+    
+    let role = payload.get("role")
+        .and_then(|v| v.as_str())
+        .unwrap_or("developer");
+    
+    let updated_project = state.relationship_manager
+        .add_member_to_project(&project_id, &user_id, role)
+        .await?;
+    
+    Ok(Json(serde_json::to_value(updated_project)?))
+}
+
+async fn remove_project_member(
+    Path((project_id, user_id)): Path<(String, String)>,
+    State(state): State<AppState>,
+) -> Result<StatusCode, ScimError> {
+    let project_id = scim_server::resource::ResourceId::new(&project_id)
+        .map_err(|e| ScimError::invalid_value(&e.to_string()))?;
+    
+    let user_id = scim_server::resource::ResourceId::new(&user_id)
+        .map_err(|e| ScimError::invalid_value(&e.to_string()))?;
+    
+    state.relationship_manager
+        .remove_member_from_project(&project_id, &user_id)
+        .await?;
+    
+    Ok(StatusCode::NO_CONTENT)
+}
+
+async fn get_user_projects(
+    Path(id): Path<String>,
+    State(state): State<AppState>,
+) -> Result<Json<Value>, ScimError> {
+    let user_id = scim_server::resource::ResourceId::new(&id)
+        .map_err(|e| ScimError::invalid_value(&e.to_string()))?;
+    
+    let projects = state.relationship_manager.get_user_projects(&user_id).await?;
+    
+    Ok(Json(json!({
+        "schemas": ["urn:ietf:params:scim:api:messages:2.0:ListResponse"],
+        "totalResults": projects.len(),
+        "startIndex": 1,
+        "itemsPerPage": projects.len(),
+        "Resources": projects
+    })))
+}
+
+// Standard SCIM handlers (simplified for brevity)
+async fn list_users(
+    Query(params): Query<HashMap<String, String>>,
+    State(state): State<AppState>,
+) -> Result<Json<Value>, ScimError> {
+    let filter = params.get("filter").map(|s| s.as_str());
+    let start_index = params.get("startIndex").and_then(|s| s.parse().ok()).unwrap_or(1);
+    let count = params.get("count").and_then(|s| s.parse().ok()).unwrap_or(20);
+    
+    let (resources, total) = state.server.list_resources("User", filter, start_index, count).await?;
+    
+    Ok(Json(json!({
+        "schemas": ["urn:ietf:params:scim:api:messages:2.0:ListResponse"],
+        "totalResults": total,
+        "startIndex": start_index,
+        "itemsPerPage": resources.len(),
+        "Resources": resources
+    })))
+}
+
+async fn create_user(
+    State(state): State<AppState>,
+    Json(payload): Json<Value>,
+) -> Result<(StatusCode, Json<Value>), ScimError> {
+    let resource = state.server.create_resource("User", payload).await?;
+    Ok((StatusCode::CREATED, Json(serde_json::to_value(resource)?)))
+}
+
+// Additional handlers would follow similar patterns...
+
+async fn get_service_provider_config(
+    State(state): State<AppState>,
+) -> Result<Json<Value>, ScimError> {
+    Ok(Json(json!({
+        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:ServiceProviderConfig"],
+        "patch": {
+            "supported": true
+        },
+        "bulk": {
+            "supported": true,
+            "maxOperations": 1000,
+            "maxPayloadSize": 1048576
+        },
+        "filter": {
+            "supported": true,
+            "maxResults": 200
+        },
+        "changePassword": {
+            "supported": false
+        },
+        "sort": {
+            "supported": true
+        },
+        "etag": {
+            "supported": false
+        },
+        "authenticationSchemes": [
+            {
+                "type": "httpbasic",
+                "name": "HTTP Basic",
+                "description": "HTTP Basic Authentication",
+                "specUri": "http://www.rfc-editor.org/info/rfc2617",
+                "documentationUri": "http://example.com/help/httpBasic.html"
+            }
+        ]
+    })))
+}
+```
+
+## Running Your Custom Server
+
+1. **Validate Your Schema**:
+   ```bash
+   cargo run --bin schema-validator schemas/Project.json
+   ```
+
+2. **Run Tests**:
+   ```bash
+   cargo test project_resource_tests
+   ```
+
+3. **Start the Server**:
+   ```bash
+   cargo run --bin project_server
+   ```
+
+4. **Test with curl**:
+   ```bash
+   # Create a project
+   curl -X POST http://localhost:3000/Projects \
+     -H "Content-Type: application/scim+json" \
+     -d '{
+       "schemas": ["urn:example:scim:schemas:core:2.0:Project"],
+       "name": "My New Project",
+       "status": "active",
+       "description": "A test project"
+     }'
+   
+   # List projects
+   curl http://localhost:3000/Projects
+   
+   # Filter projects by status
+   curl "http://localhost:3000/Projects?filter=status eq \"active\""
+   ```
+
+## Next Steps
+
+Now that you have a working custom resource implementation, consider:
+
+1. **Add More Custom Resources**: Implement additional business-specific resources
+2. **Enhance Relationships**: Build more sophisticated relationships between resources
3. **Add Database Persistence**: Replace in-memory storage with database backends
4. **Implement Authorization**: Add role-based access control for custom resources
5. **Add Webhooks**: Implement event notifications for resource changes
6. **Performance Optimization**: Add caching and indexing for large datasets
7. **API Versioning**: Plan for future schema evolution and versioning

## Troubleshooting

### Common Issues

**Schema Validation Errors**
- Ensure your schema file follows the correct JSON structure
- Validate required fields are properly marked
- Check that canonical values match the attribute type

**Provider Implementation Issues**
- Verify all ResourceProvider methods are implemented correctly
- Check error handling and return appropriate SCIM errors
- Ensure resource ID generation is unique and consistent

**Integration Problems**
- Confirm schema registry loads your custom schemas
- Verify resource providers are registered with correct names
- Check HTTP route mappings match your resource names

### Debugging Tips

1. **Enable Logging**: Add comprehensive logging to track request flow
2. **Use Schema Validator**: Run schema validation during development
3. **Test Incrementally**: Test each component in isolation before integration
4. **Check Examples**: Refer to existing providers for implementation patterns

## Related Documentation

- [Basic Usage Guide](basic-usage.md) - Understanding SCIM fundamentals
- [Configuration Guide](configuration.md) - Server configuration options
- [Advanced Features Examples](../examples/advanced-features.md) - Complex implementation patterns
- [API Reference - Providers](../api/providers.md) - ResourceProvider trait details
- [Schema Reference](../reference/schemas.md) - Complete schema documentation

## Conclusion

You've successfully implemented a custom SCIM resource from schema definition through complete server integration. This foundation allows you to extend SCIM to manage any business resource while maintaining SCIM protocol compliance and leveraging the robust validation and error handling provided by the SCIM server crate.

The Project resource example demonstrates key concepts that apply to any custom resource:
- Structured schema definition with proper attribute types
- Comprehensive resource provider implementation
- Business logic validation
- Resource relationships and lifecycle management
- Integration testing and API endpoints

These patterns can be adapted for any custom resource type your organization needs to manage through SCIM.
