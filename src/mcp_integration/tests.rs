//! Tests for MCP integration functionality
//!
//! This module contains comprehensive tests for the MCP protocol implementation,
//! including stdio communication, tool discovery, and tool execution.

#[cfg(feature = "mcp")]
mod mcp_tests {
    use super::super::core::{McpServerInfo, ScimMcpServer};
    use crate::{
        multi_tenant::ScimOperation, providers::StandardResourceProvider,
        resource_handlers::create_user_resource_handler, scim_server::ScimServer,
        storage::InMemoryStorage,
    };
    use serde_json::{Value, json};

    /// Test helper to create a test MCP server
    async fn create_test_mcp_server() -> ScimMcpServer<StandardResourceProvider<InMemoryStorage>> {
        let storage = InMemoryStorage::new();
        let provider = StandardResourceProvider::new(storage);
        let mut scim_server = ScimServer::new(provider).expect("Failed to create SCIM server");

        // Register User resource type
        let user_schema = scim_server
            .get_schema_by_id("urn:ietf:params:scim:schemas:core:2.0:User")
            .expect("Failed to get user schema")
            .clone();
        let user_handler = create_user_resource_handler(user_schema);

        scim_server
            .register_resource_type(
                "User",
                user_handler,
                vec![
                    ScimOperation::Create,
                    ScimOperation::Read,
                    ScimOperation::Update,
                    ScimOperation::Delete,
                    ScimOperation::Search,
                ],
            )
            .expect("Failed to register User resource type");

        ScimMcpServer::new(scim_server)
    }

    #[tokio::test]
    async fn test_tool_discovery() {
        let mcp_server = create_test_mcp_server().await;
        let tools = mcp_server.get_tools();

        assert_eq!(tools.len(), 9, "Should have 9 tools available");

        // Verify expected tool names are present
        let tool_names: Vec<&str> = tools
            .iter()
            .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
            .collect();

        let expected_tools = vec![
            "scim_create_user",
            "scim_get_user",
            "scim_update_user",
            "scim_delete_user",
            "scim_list_users",
            "scim_search_users",
            "scim_user_exists",
            "scim_get_schemas",
            "scim_server_info",
        ];

        for expected_tool in expected_tools {
            assert!(
                tool_names.contains(&expected_tool),
                "Should contain tool: {}",
                expected_tool
            );
        }
    }

    #[tokio::test]
    async fn test_tool_execution_create_user() {
        let mcp_server = create_test_mcp_server().await;

        let arguments = json!({
            "user_data": {
                "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                "userName": "test.user@example.com",
                "active": true,
                "name": {
                    "givenName": "Test",
                    "familyName": "User"
                }
            }
        });

        let result = mcp_server.execute_tool("scim_create_user", arguments).await;

        // Debug the result if it fails
        if !result.success {
            println!("Tool execution failed!");
            println!(
                "Content: {}",
                serde_json::to_string_pretty(&result.content).unwrap()
            );
            println!("Metadata: {:?}", result.metadata);
        }

        assert!(
            result.success,
            "Tool execution should succeed. Content: {}",
            result.content
        );
        assert!(result.content.get("id").is_some(), "Should return user ID");
        if let Some(user_name) = result.content.get("userName") {
            assert_eq!(user_name.as_str().unwrap(), "test.user@example.com");
        }
    }

    #[tokio::test]
    async fn test_tool_execution_unknown_tool() {
        let mcp_server = create_test_mcp_server().await;

        let result = mcp_server.execute_tool("unknown_tool", json!({})).await;

        assert!(!result.success, "Unknown tool should fail");
        assert!(
            result.content.get("error").is_some(),
            "Should return error message"
        );
        assert_eq!(
            result.content.get("tool_name").unwrap().as_str().unwrap(),
            "unknown_tool"
        );
    }

    /// Test MCP JSON-RPC message parsing
    #[test]
    fn test_mcp_request_parsing() {
        let initialize_request = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0.0"}}}"#;

        let parsed: Value =
            serde_json::from_str(initialize_request).expect("Should parse initialize request");

        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["method"], "initialize");
        assert_eq!(parsed["id"], 1);
    }

    /// Test MCP response formatting
    #[test]
    fn test_mcp_response_formatting() {
        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "SCIM Server",
                    "version": "2.0"
                }
            }
        });

        let serialized = serde_json::to_string(&response).expect("Should serialize response");

        assert!(serialized.contains("jsonrpc"));
        assert!(serialized.contains("protocolVersion"));
        assert!(serialized.contains("SCIM Server"));
    }

    /// Test stdio communication flow
    #[tokio::test]
    async fn test_stdio_communication_flow() {
        let mcp_server = create_test_mcp_server().await;

        // Test messages that would come from stdin
        let test_messages = vec![
            // Initialize request
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "test-client",
                        "version": "1.0.0"
                    }
                }
            }),
            // List tools request
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/list",
                "params": {}
            }),
            // Call tool request
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {
                    "name": "scim_server_info",
                    "arguments": {}
                }
            }),
        ];

        // Simulate processing each message
        for (_i, message) in test_messages.iter().enumerate() {
            let method = message["method"].as_str().unwrap();
            let id = message["id"].clone();

            match method {
                "initialize" => {
                    // Should return initialize response
                    let response = create_initialize_response(id);
                    assert!(response["result"]["serverInfo"]["name"].is_string());
                }
                "tools/list" => {
                    // Should return tools list
                    let tools = mcp_server.get_tools();
                    assert_eq!(tools.len(), 9);
                }
                "tools/call" => {
                    // Should execute tool
                    let tool_name = message["params"]["name"].as_str().unwrap();
                    let arguments = message["params"]["arguments"].clone();
                    let result = mcp_server.execute_tool(tool_name, arguments).await;
                    assert!(result.success);
                }
                _ => panic!("Unexpected method: {}", method),
            }
        }
    }

    /// Test error handling in MCP protocol
    #[tokio::test]
    async fn test_mcp_error_handling() {
        let mcp_server = create_test_mcp_server().await;

        // Test invalid JSON parsing
        let _invalid_json = "invalid json";
        let error_response = create_parse_error_response();
        assert_eq!(error_response["error"]["code"], -32700);
        assert_eq!(error_response["error"]["message"], "Parse error");

        // Test method not found
        let unknown_method_response = create_method_not_found_response(json!(1));
        assert_eq!(unknown_method_response["error"]["code"], -32601);
        assert_eq!(
            unknown_method_response["error"]["message"],
            "Method not found"
        );

        // Test tool execution failure
        let result = mcp_server
            .execute_tool("scim_create_user", json!({"invalid": "data"}))
            .await;
        assert!(!result.success);
    }

    // Helper functions for creating MCP responses
    fn create_initialize_response(id: Value) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "SCIM Server",
                    "version": "2.0"
                }
            }
        })
    }

    fn create_parse_error_response() -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": null,
            "error": {
                "code": -32700,
                "message": "Parse error"
            }
        })
    }

    fn create_method_not_found_response(id: Value) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32601,
                "message": "Method not found"
            }
        })
    }

    /// Test concurrent tool execution
    #[tokio::test]
    async fn test_concurrent_tool_execution() {
        let mcp_server = std::sync::Arc::new(create_test_mcp_server().await);

        let mut handles = vec![];

        // Execute multiple tools concurrently
        for i in 0..5 {
            let server = mcp_server.clone();
            let handle = tokio::spawn(async move {
                let result = server.execute_tool("scim_server_info", json!({})).await;
                (i, result.success)
            });
            handles.push(handle);
        }

        // Wait for all executions to complete
        for handle in handles {
            let (id, success) = handle.await.expect("Task should complete");
            assert!(success, "Concurrent execution {} should succeed", id);
        }
    }

    /// Test server info functionality
    #[test]
    fn test_server_info() {
        let server_info = McpServerInfo::default();

        assert_eq!(server_info.name, "SCIM Server");
        assert_eq!(server_info.version, "2.0");
        assert!(
            server_info
                .supported_resource_types
                .contains(&"User".to_string())
        );
        assert!(
            server_info
                .supported_resource_types
                .contains(&"Group".to_string())
        );
    }

    /// Test custom server info
    #[test]
    fn test_custom_server_info() {
        let custom_info = McpServerInfo {
            name: "Custom SCIM Server".to_string(),
            version: "1.5.0".to_string(),
            description: "Custom description".to_string(),
            supported_resource_types: vec!["User".to_string()],
        };

        assert_eq!(custom_info.name, "Custom SCIM Server");
        assert_eq!(custom_info.version, "1.5.0");
        assert_eq!(custom_info.description, "Custom description");
        assert_eq!(custom_info.supported_resource_types.len(), 1);
    }

    /// Integration test that simulates a complete MCP client-server interaction
    #[tokio::test]
    async fn test_complete_mcp_stdio_integration() {
        let mcp_server = create_test_mcp_server().await;

        // Test complete MCP workflow: initialize -> list tools -> call tool

        // 1. Initialize
        let initialize_response = mcp_server.handle_mcp_request(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0.0"}}}"#
        ).await;

        assert!(initialize_response.is_some());
        let init_resp = initialize_response.unwrap();
        assert!(init_resp.result.is_some());
        assert!(init_resp.error.is_none());

        let init_result = init_resp.result.unwrap();
        assert_eq!(init_result["protocolVersion"], "2024-11-05");
        assert!(init_result["capabilities"]["tools"].is_object());
        assert_eq!(init_result["serverInfo"]["name"], "SCIM Server");

        // 2. List tools
        let tools_response = mcp_server
            .handle_mcp_request(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#)
            .await;

        assert!(tools_response.is_some());
        let tools_resp = tools_response.unwrap();
        assert!(tools_resp.result.is_some());
        assert!(tools_resp.error.is_none());

        let tools_result = tools_resp.result.unwrap();
        let tools_array = tools_result["tools"].as_array().unwrap();
        assert_eq!(tools_array.len(), 9);

        // Verify expected tools are present
        let tool_names: Vec<String> = tools_array
            .iter()
            .filter_map(|tool| tool.get("name"))
            .filter_map(|name| name.as_str())
            .map(|s| s.to_string())
            .collect();

        assert!(tool_names.contains(&"scim_create_user".to_string()));
        assert!(tool_names.contains(&"scim_get_user".to_string()));
        assert!(tool_names.contains(&"scim_server_info".to_string()));

        // 3. Call a tool - create user
        let create_user_response = mcp_server.handle_mcp_request(
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"scim_create_user","arguments":{"user_data":{"schemas":["urn:ietf:params:scim:schemas:core:2.0:User"],"userName":"integration.test@example.com","active":true,"name":{"givenName":"Integration","familyName":"Test"}}}}}"#
        ).await;

        assert!(create_user_response.is_some());
        let create_resp = create_user_response.unwrap();
        assert!(create_resp.result.is_some());
        assert!(create_resp.error.is_none());

        let create_result = create_resp.result.unwrap();
        assert!(create_result["content"].is_array());
        let content_array = create_result["content"].as_array().unwrap();
        assert!(!content_array.is_empty());

        let content_text = content_array[0]["text"].as_str().unwrap();
        let user_data: Value = serde_json::from_str(content_text).unwrap();
        assert!(user_data.get("id").is_some());
        assert_eq!(user_data["userName"], "integration.test@example.com");

        // 4. Call server info tool
        let server_info_response = mcp_server.handle_mcp_request(
            r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"scim_server_info","arguments":{}}}"#
        ).await;

        assert!(server_info_response.is_some());
        let info_resp = server_info_response.unwrap();
        assert!(info_resp.result.is_some());
        assert!(info_resp.error.is_none());

        // 5. Test error handling with invalid tool
        let error_response = mcp_server.handle_mcp_request(
            r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"nonexistent_tool","arguments":{}}}"#
        ).await;

        assert!(error_response.is_some());
        let err_resp = error_response.unwrap();
        assert!(err_resp.error.is_some());
        assert!(err_resp.result.is_none());

        let error_obj = err_resp.error.unwrap();
        assert_eq!(error_obj["code"], -32000);

        // 6. Test ping
        let ping_response = mcp_server
            .handle_mcp_request(r#"{"jsonrpc":"2.0","id":6,"method":"ping","params":{}}"#)
            .await;

        assert!(ping_response.is_some());
        let ping_resp = ping_response.unwrap();
        assert!(ping_resp.result.is_some());
        assert!(ping_resp.error.is_none());

        // 7. Test invalid JSON
        let invalid_response = mcp_server.handle_mcp_request("invalid json").await;
        assert!(invalid_response.is_some());
        let invalid_resp = invalid_response.unwrap();
        assert!(invalid_resp.error.is_some());
        assert_eq!(invalid_resp.error.unwrap()["code"], -32700);

        println!("✅ Complete MCP stdio integration test passed!");
    }

    /// Test that demonstrates user lifecycle through MCP tools
    #[tokio::test]
    async fn test_user_lifecycle_via_mcp() {
        let mcp_server = create_test_mcp_server().await;

        // 1. Create user
        let create_result = mcp_server
            .execute_tool(
                "scim_create_user",
                json!({
                    "user_data": {
                        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                        "userName": "lifecycle.test@example.com",
                        "active": true,
                        "name": {
                            "givenName": "Lifecycle",
                            "familyName": "Test"
                        }
                    }
                }),
            )
            .await;

        assert!(create_result.success, "User creation should succeed");
        let user_id = create_result.content.get("id").unwrap().as_str().unwrap();
        println!("✅ Created user with ID: {}", user_id);

        // 2. Get user
        let get_result = mcp_server
            .execute_tool(
                "scim_get_user",
                json!({
                    "user_id": user_id
                }),
            )
            .await;

        assert!(get_result.success, "User retrieval should succeed");
        assert_eq!(get_result.content["userName"], "lifecycle.test@example.com");
        println!("✅ Retrieved user successfully");

        // 3. Update user
        let update_result = mcp_server
            .execute_tool(
                "scim_update_user",
                json!({
                    "user_id": user_id,
                    "user_data": {
                        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                        "userName": "lifecycle.updated@example.com",
                        "active": false,
                        "name": {
                            "givenName": "Updated",
                            "familyName": "User"
                        }
                    }
                }),
            )
            .await;

        assert!(update_result.success, "User update should succeed");
        assert_eq!(
            update_result.content["userName"],
            "lifecycle.updated@example.com"
        );
        println!("✅ Updated user successfully");

        // 4. Verify update by getting user again
        let verify_result = mcp_server
            .execute_tool(
                "scim_get_user",
                json!({
                    "user_id": user_id
                }),
            )
            .await;

        assert!(verify_result.success, "User verification should succeed");
        assert_eq!(
            verify_result.content["userName"],
            "lifecycle.updated@example.com"
        );
        assert_eq!(verify_result.content["active"], false);
        println!("✅ Verified user update");

        // 5. Delete user
        let delete_result = mcp_server
            .execute_tool(
                "scim_delete_user",
                json!({
                    "user_id": user_id
                }),
            )
            .await;

        assert!(delete_result.success, "User deletion should succeed");
        println!("✅ Deleted user successfully");

        // 6. Verify user is gone
        let verify_gone_result = mcp_server
            .execute_tool(
                "scim_get_user",
                json!({
                    "user_id": user_id
                }),
            )
            .await;

        assert!(
            !verify_gone_result.success,
            "Getting deleted user should fail"
        );
        println!("✅ Confirmed user deletion");

        println!("✅ Complete user lifecycle test passed!");
    }
}
