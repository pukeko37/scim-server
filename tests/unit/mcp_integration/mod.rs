//! Unit tests for the MCP integration module.

#[cfg(feature = "mcp")]
mod tests {
    use scim_server::ScimServer;
    use scim_server::mcp_integration::ScimMcpServer;
    use scim_server::providers::InMemoryProvider;
    use scim_server::resource_handlers::create_user_resource_handler;
    use serde_json::json;

    #[tokio::test]
    async fn test_mcp_server_creation() {
        let provider = InMemoryProvider::new();
        let mut scim_server = ScimServer::new(provider).unwrap();

        let user_schema = scim_server
            .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
            .unwrap()
            .clone();

        let user_handler = create_user_resource_handler(user_schema);
        scim_server
            .register_resource_type(
                "User",
                user_handler,
                vec![scim_server::multi_tenant::ScimOperation::Create],
            )
            .unwrap();

        let mcp_server = ScimMcpServer::new(scim_server);
        assert_eq!(mcp_server.server_info.name, "SCIM Server");
    }

    #[tokio::test]
    async fn test_mcp_tools_list() {
        let provider = InMemoryProvider::new();
        let scim_server = ScimServer::new(provider).unwrap();
        let mcp_server = ScimMcpServer::new(scim_server);

        let tools = mcp_server.get_tools();
        assert!(!tools.is_empty());

        let tool_names: Vec<_> = tools
            .iter()
            .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
            .collect();
        assert!(tool_names.contains(&"scim_create_user"));
        assert!(tool_names.contains(&"scim_get_user"));
        assert!(tool_names.contains(&"scim_list_users"));
    }

    #[tokio::test]
    async fn test_mcp_tool_execution() {
        let provider = InMemoryProvider::new();
        let mut scim_server = ScimServer::new(provider).unwrap();

        let user_schema = scim_server
            .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
            .unwrap()
            .clone();

        let user_handler = create_user_resource_handler(user_schema);
        scim_server
            .register_resource_type(
                "User",
                user_handler,
                vec![
                    scim_server::multi_tenant::ScimOperation::Create,
                    scim_server::multi_tenant::ScimOperation::List,
                ],
            )
            .unwrap();

        let mcp_server = ScimMcpServer::new(scim_server);

        // Test get schemas tool
        let result = mcp_server.execute_tool("scim_get_schemas", json!({})).await;
        assert!(result.success);

        // Test list users tool
        let result = mcp_server.execute_tool("scim_list_users", json!({})).await;
        assert!(result.success);
    }
}
