use serde::{Deserialize, Serialize};

/// A host-declared MCP server endpoint. Transport startup is intentionally out of scope.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct McpManifest {
    pub name: String,
    pub command: String,
    pub arguments: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: serde_json::Value,
}

impl JsonRpcRequest {
    pub fn initialize(id: u64) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            method: "initialize".into(),
            params: serde_json::json!({}),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: u64,
    pub result: serde_json::Value,
}

pub trait McpTransport {
    fn request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse, McpError>;
}

#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("MCP protocol version must be 2.0")]
    InvalidVersion,
    #[error("MCP transport error: {0}")]
    Transport(String),
}

pub fn initialize(transport: &dyn McpTransport) -> Result<JsonRpcResponse, McpError> {
    let request = JsonRpcRequest::initialize(1);
    let response = transport.request(request)?;
    if response.jsonrpc != "2.0" {
        return Err(McpError::InvalidVersion);
    }
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct LocalDouble;
    impl McpTransport for LocalDouble {
        fn request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse, McpError> {
            Ok(JsonRpcResponse {
                jsonrpc: "2.0".into(),
                id: request.id,
                result: serde_json::json!({"serverInfo":{"name":"local"}}),
            })
        }
    }

    #[test]
    fn sends_standard_initialize_to_a_local_double() {
        let response = initialize(&LocalDouble).unwrap();
        assert_eq!(response.id, 1);
        assert_eq!(response.result["serverInfo"]["name"], "local");
    }

    #[test]
    fn rejects_non_json_rpc_response() {
        struct Bad;
        impl McpTransport for Bad {
            fn request(&self, _: JsonRpcRequest) -> Result<JsonRpcResponse, McpError> {
                Ok(JsonRpcResponse {
                    jsonrpc: "1.0".into(),
                    id: 1,
                    result: serde_json::json!({}),
                })
            }
        }
        assert!(matches!(initialize(&Bad), Err(McpError::InvalidVersion)));
    }
}
