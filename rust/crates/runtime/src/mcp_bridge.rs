//! MCP bridge — connects configured MCP servers to the tool registry.
//!
//! Spawns MCP server processes (Stdio transport), queries their available tools
//! via the JSON-RPC `tools/list` method, and registers them as prefixed tools.
//! Tool execution dispatches `tools/call` to the running server process.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};

use crate::mcp::mcp_tool_name;
use crate::mcp_client::{McpClientBootstrap, McpClientTransport, McpRemoteTransport, McpStdioTransport};

/// An active MCP server connection (stdio or HTTP).
pub struct McpConnection {
    pub server_name: String,
    pub tool_prefix: String,
    pub tools: Vec<McpToolDef>,
    child: Option<Child>,
    /// HTTP endpoint for remote MCP servers.
    http_url: Option<String>,
    /// HTTP headers for remote MCP servers.
    http_headers: BTreeMap<String, String>,
}

/// A tool definition from an MCP server.
#[derive(Debug, Clone)]
pub struct McpToolDef {
    /// Full prefixed name: mcp__server__tool_name
    pub full_name: String,
    /// Original tool name from the server.
    pub original_name: String,
    /// Description from the server.
    pub description: String,
    /// Input JSON schema from the server.
    pub input_schema: serde_json::Value,
}

/// Start an MCP stdio server and query its tools.
///
/// Returns the connection with tool definitions, or None if the server can't start.
pub fn connect_stdio(bootstrap: &McpClientBootstrap) -> Option<McpConnection> {
    let McpClientTransport::Stdio(ref stdio) = bootstrap.transport else {
        return None;
    };

    // Spawn the MCP server process
    let mut child = spawn_stdio_process(stdio)?;

    // Initialize the JSON-RPC connection
    let stdin = child.stdin.as_mut()?;
    let stdout = child.stdout.as_mut()?;
    let mut reader = BufReader::new(stdout);

    // Send initialize request
    let init_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "openanalyst-cli",
                "version": "1.0.89"
            }
        }
    });

    if send_jsonrpc(stdin, &init_request).is_err() {
        return None;
    }

    let _init_response = read_jsonrpc_response(&mut reader)?;

    // Send initialized notification
    let initialized = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    let _ = send_jsonrpc(stdin, &initialized);

    // Query tools list
    let tools_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });

    if send_jsonrpc(stdin, &tools_request).is_err() {
        return None;
    }

    let tools_response = read_jsonrpc_response(&mut reader)?;
    let tools = parse_tools_list(&bootstrap.server_name, &tools_response);

    Some(McpConnection {
        server_name: bootstrap.server_name.clone(),
        tool_prefix: bootstrap.tool_prefix.clone(),
        tools,
        child: Some(child),
        http_url: None,
        http_headers: BTreeMap::new(),
    })
}

/// Call an MCP tool on a running connection (routes to stdio or HTTP).
pub fn call_tool(
    connection: &mut McpConnection,
    tool_name: &str,
    arguments: serde_json::Value,
) -> Result<String, String> {
    if connection.http_url.is_some() {
        call_tool_http(connection, tool_name, arguments)
    } else {
        call_tool_stdio(connection, tool_name, arguments)
    }
}

/// Call a tool via stdio transport.
fn call_tool_stdio(
    connection: &mut McpConnection,
    tool_name: &str,
    arguments: serde_json::Value,
) -> Result<String, String> {
    let child = connection.child.as_mut().ok_or("MCP server not running")?;
    let stdin = child.stdin.as_mut().ok_or("No stdin")?;
    let stdout = child.stdout.as_mut().ok_or("No stdout")?;
    let mut reader = BufReader::new(stdout);

    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": tool_name,
            "arguments": arguments,
        }
    });

    send_jsonrpc(stdin, &request).map_err(|e| e.to_string())?;
    let response = read_jsonrpc_response(&mut reader).ok_or("No response from MCP server")?;
    extract_mcp_result(&response)
}

/// Call a tool via HTTP transport (JSON-RPC over POST).
fn call_tool_http(
    connection: &McpConnection,
    tool_name: &str,
    arguments: serde_json::Value,
) -> Result<String, String> {
    let url = connection.http_url.as_deref().ok_or("No HTTP URL configured")?;
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": tool_name,
            "arguments": arguments,
        }
    });

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let mut req = client.post(url).json(&request);
    for (key, value) in &connection.http_headers {
        req = req.header(key.as_str(), value.as_str());
    }

    let resp = req.send().map_err(|e| format!("MCP HTTP request failed: {e}"))?;
    let status = resp.status();
    let body = resp.text().map_err(|e| format!("MCP HTTP read failed: {e}"))?;

    if !status.is_success() {
        return Err(format!("MCP HTTP {status}: {body}"));
    }

    let response: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("MCP HTTP JSON parse failed: {e}"))?;
    extract_mcp_result(&response)
}

/// Extract text content from an MCP JSON-RPC response.
fn extract_mcp_result(response: &serde_json::Value) -> Result<String, String> {
    if let Some(result) = response.get("result") {
        if let Some(content) = result.get("content") {
            if let Some(arr) = content.as_array() {
                let texts: Vec<&str> = arr
                    .iter()
                    .filter_map(|c| c.get("text").and_then(|t| t.as_str()))
                    .collect();
                return Ok(texts.join("\n"));
            }
        }
        Ok(result.to_string())
    } else if let Some(error) = response.get("error") {
        Err(error.to_string())
    } else {
        Ok(String::new())
    }
}

/// Bootstrap all configured MCP servers from a config collection.
pub fn bootstrap_mcp_servers(
    servers: &BTreeMap<String, crate::config::ScopedMcpServerConfig>,
) -> Vec<McpConnection> {
    let mut connections = Vec::new();
    for (name, config) in servers {
        let bootstrap = McpClientBootstrap::from_scoped_config(name, config);
        match &bootstrap.transport {
            McpClientTransport::Stdio(_) => {
                if let Some(conn) = connect_stdio(&bootstrap) {
                    connections.push(conn);
                }
            }
            McpClientTransport::Http(remote) | McpClientTransport::Sse(remote) => {
                if let Some(conn) = connect_http(&bootstrap.server_name, remote) {
                    connections.push(conn);
                }
            }
            // WebSocket, SDK, ManagedProxy — not yet implemented
            _ => {}
        }
    }
    connections
}

// ── Internal helpers ──

fn spawn_stdio_process(stdio: &McpStdioTransport) -> Option<Child> {
    let mut cmd = Command::new(&stdio.command);
    cmd.args(&stdio.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    for (key, val) in &stdio.env {
        cmd.env(key, val);
    }

    cmd.spawn().ok()
}

fn send_jsonrpc(stdin: &mut impl Write, request: &serde_json::Value) -> std::io::Result<()> {
    let body = serde_json::to_string(request)?;
    write!(stdin, "Content-Length: {}\r\n\r\n{}", body.len(), body)?;
    stdin.flush()
}

fn read_jsonrpc_response(reader: &mut impl BufRead) -> Option<serde_json::Value> {
    // Read headers to find Content-Length
    let mut content_length: Option<usize> = None;
    let mut header = String::new();
    loop {
        header.clear();
        if reader.read_line(&mut header).ok()? == 0 {
            return None;
        }
        let trimmed = header.trim();
        if trimmed.is_empty() {
            break; // End of headers (\r\n\r\n)
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            content_length = value.trim().parse().ok();
        }
    }

    // Read exactly Content-Length bytes (handles multi-line JSON correctly)
    let length = content_length?;
    let mut body = vec![0u8; length];
    reader.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok()
}

fn parse_tools_list(server_name: &str, response: &serde_json::Value) -> Vec<McpToolDef> {
    let Some(result) = response.get("result") else {
        return Vec::new();
    };
    let Some(tools) = result.get("tools").and_then(|t| t.as_array()) else {
        return Vec::new();
    };

    tools
        .iter()
        .filter_map(|tool| {
            let name = tool.get("name")?.as_str()?;
            let description = tool
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string();
            let input_schema = tool
                .get("inputSchema")
                .cloned()
                .unwrap_or(serde_json::json!({}));

            Some(McpToolDef {
                full_name: mcp_tool_name(server_name, name),
                original_name: name.to_string(),
                description,
                input_schema,
            })
        })
        .collect()
}

/// Connect to an MCP server via HTTP transport.
/// Sends JSON-RPC requests via HTTP POST, receives responses in the body.
fn connect_http(server_name: &str, remote: &McpRemoteTransport) -> Option<McpConnection> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .ok()?;

    let mut headers = remote.headers.clone();
    headers
        .entry("Content-Type".to_string())
        .or_insert_with(|| "application/json".to_string());

    // Initialize
    let init_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "openanalyst-cli",
                "version": "1.0.89"
            }
        }
    });

    let mut req = client.post(&remote.url).json(&init_request);
    for (key, value) in &headers {
        req = req.header(key.as_str(), value.as_str());
    }
    let resp = req.send().ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let _init_response: serde_json::Value = resp.json().ok()?;

    // Send initialized notification
    let initialized = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    let mut req = client.post(&remote.url).json(&initialized);
    for (key, value) in &headers {
        req = req.header(key.as_str(), value.as_str());
    }
    let _ = req.send();

    // Query tools
    let tools_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });
    let mut req = client.post(&remote.url).json(&tools_request);
    for (key, value) in &headers {
        req = req.header(key.as_str(), value.as_str());
    }
    let resp = req.send().ok()?;
    let tools_response: serde_json::Value = resp.json().ok()?;
    let tools = parse_tools_list(server_name, &tools_response);

    Some(McpConnection {
        server_name: server_name.to_string(),
        tool_prefix: crate::mcp::mcp_tool_prefix(server_name),
        tools,
        child: None,
        http_url: Some(remote.url.clone()),
        http_headers: headers,
    })
}

impl Drop for McpConnection {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn read_single_line_json() {
        let body = r#"{"jsonrpc":"2.0","id":1,"result":{"ok":true}}"#;
        let frame = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
        let mut reader = BufReader::new(Cursor::new(frame.as_bytes().to_vec()));
        let result = read_jsonrpc_response(&mut reader).expect("should parse");
        assert_eq!(result["id"], 1);
        assert_eq!(result["result"]["ok"], true);
    }

    #[test]
    fn read_multi_line_json() {
        let body = "{\n  \"jsonrpc\": \"2.0\",\n  \"id\": 2,\n  \"result\": {\n    \"tools\": []\n  }\n}";
        let frame = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
        let mut reader = BufReader::new(Cursor::new(frame.as_bytes().to_vec()));
        let result = read_jsonrpc_response(&mut reader).expect("should parse multi-line");
        assert_eq!(result["id"], 2);
        assert!(result["result"]["tools"].is_array());
    }

    #[test]
    fn read_missing_content_length_returns_none() {
        let frame = "X-Custom: something\r\n\r\n{}";
        let mut reader = BufReader::new(Cursor::new(frame.as_bytes().to_vec()));
        assert!(read_jsonrpc_response(&mut reader).is_none());
    }

    #[test]
    fn read_empty_stream_returns_none() {
        let mut reader = BufReader::new(Cursor::new(Vec::<u8>::new()));
        assert!(read_jsonrpc_response(&mut reader).is_none());
    }

    #[test]
    fn send_jsonrpc_formats_correctly() {
        let mut buf = Vec::new();
        let request = serde_json::json!({"jsonrpc":"2.0","id":1,"method":"test"});
        send_jsonrpc(&mut buf, &request).expect("should write");
        let output = String::from_utf8(buf).unwrap();
        assert!(output.starts_with("Content-Length: "));
        assert!(output.contains("\r\n\r\n"));
        // Verify the body after headers is valid JSON
        let body_start = output.find("\r\n\r\n").unwrap() + 4;
        let body = &output[body_start..];
        let parsed: serde_json::Value = serde_json::from_str(body).unwrap();
        assert_eq!(parsed["method"], "test");
    }

    #[test]
    fn parse_tools_list_extracts_tools() {
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "result": {
                "tools": [
                    {
                        "name": "read_file",
                        "description": "Read a file",
                        "inputSchema": { "type": "object" }
                    },
                    {
                        "name": "write_file",
                        "description": "Write a file",
                        "inputSchema": { "type": "object" }
                    }
                ]
            }
        });
        let tools = parse_tools_list("myserver", &response);
        assert_eq!(tools.len(), 2);
        assert!(tools[0].full_name.contains("myserver"));
        assert_eq!(tools[0].original_name, "read_file");
        assert_eq!(tools[1].original_name, "write_file");
    }

    #[test]
    fn parse_tools_list_handles_empty_result() {
        let response = serde_json::json!({"jsonrpc":"2.0","id":2,"result":{"tools":[]}});
        let tools = parse_tools_list("srv", &response);
        assert!(tools.is_empty());
    }

    #[test]
    fn parse_tools_list_handles_missing_result() {
        let response = serde_json::json!({"jsonrpc":"2.0","id":2,"error":{"code":-1,"message":"fail"}});
        let tools = parse_tools_list("srv", &response);
        assert!(tools.is_empty());
    }

    #[test]
    fn sequential_reads_from_same_stream() {
        let body1 = r#"{"jsonrpc":"2.0","id":1,"result":"init"}"#;
        let body2 = r#"{"jsonrpc":"2.0","id":2,"result":{"tools":[]}}"#;
        let frame = format!(
            "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
            body1.len(), body1, body2.len(), body2
        );
        let mut reader = BufReader::new(Cursor::new(frame.as_bytes().to_vec()));
        let r1 = read_jsonrpc_response(&mut reader).expect("first read");
        let r2 = read_jsonrpc_response(&mut reader).expect("second read");
        assert_eq!(r1["id"], 1);
        assert_eq!(r2["id"], 2);
    }

    // ── Edge case / fuzz-like tests ──

    #[test]
    fn read_truncated_body_returns_none() {
        // Content-Length says 100 but body is only 10 bytes
        let frame = "Content-Length: 100\r\n\r\n{\"short\":1}";
        let mut reader = BufReader::new(Cursor::new(frame.as_bytes().to_vec()));
        assert!(read_jsonrpc_response(&mut reader).is_none());
    }

    #[test]
    fn read_zero_content_length_returns_none() {
        let frame = "Content-Length: 0\r\n\r\n";
        let mut reader = BufReader::new(Cursor::new(frame.as_bytes().to_vec()));
        // Empty body is not valid JSON
        assert!(read_jsonrpc_response(&mut reader).is_none());
    }

    #[test]
    fn read_malformed_json_returns_none() {
        let body = "not json at all {{{";
        let frame = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
        let mut reader = BufReader::new(Cursor::new(frame.as_bytes().to_vec()));
        assert!(read_jsonrpc_response(&mut reader).is_none());
    }

    #[test]
    fn read_extra_headers_ignored() {
        let body = r#"{"jsonrpc":"2.0","id":1,"result":"ok"}"#;
        let frame = format!(
            "X-Custom: ignored\r\nContent-Length: {}\r\nX-Another: also-ignored\r\n\r\n{}",
            body.len(),
            body
        );
        let mut reader = BufReader::new(Cursor::new(frame.as_bytes().to_vec()));
        let result = read_jsonrpc_response(&mut reader).expect("should parse with extra headers");
        assert_eq!(result["id"], 1);
    }

    #[test]
    fn read_unicode_json_body() {
        let body = r#"{"jsonrpc":"2.0","id":1,"result":"Héllo wörld 日本語"}"#;
        let frame = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
        let mut reader = BufReader::new(Cursor::new(frame.as_bytes().to_vec()));
        let result = read_jsonrpc_response(&mut reader).expect("should parse unicode");
        assert!(result["result"].as_str().unwrap().contains("日本語"));
    }

    #[test]
    fn read_large_json_body() {
        // Simulate a large tools/list response
        let mut tools = Vec::new();
        for i in 0..100 {
            tools.push(serde_json::json!({
                "name": format!("tool_{i}"),
                "description": format!("Description for tool number {i} with some extra text to increase size"),
                "inputSchema": {"type": "object", "properties": {"arg": {"type": "string"}}}
            }));
        }
        let body = serde_json::json!({"jsonrpc":"2.0","id":2,"result":{"tools": tools}}).to_string();
        let frame = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
        let mut reader = BufReader::new(Cursor::new(frame.as_bytes().to_vec()));
        let result = read_jsonrpc_response(&mut reader).expect("should parse large body");
        let parsed_tools = parse_tools_list("big", &result);
        assert_eq!(parsed_tools.len(), 100);
    }

    #[test]
    fn extract_mcp_result_handles_error_response() {
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "error": {"code": -32600, "message": "Invalid Request"}
        });
        let result = extract_mcp_result(&response);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid Request"));
    }

    #[test]
    fn extract_mcp_result_handles_empty_content() {
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "result": {"content": []}
        });
        let result = extract_mcp_result(&response).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn extract_mcp_result_handles_mixed_content_types() {
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "result": {
                "content": [
                    {"type": "text", "text": "Hello"},
                    {"type": "image", "data": "base64..."},
                    {"type": "text", "text": "World"}
                ]
            }
        });
        let result = extract_mcp_result(&response).unwrap();
        assert_eq!(result, "Hello\nWorld");
    }
}
