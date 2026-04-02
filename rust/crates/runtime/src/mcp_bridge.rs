//! MCP bridge — connects configured MCP servers to the tool registry.
//!
//! Spawns MCP server processes (Stdio transport), queries their available tools
//! via the JSON-RPC `tools/list` method, and registers them as prefixed tools.
//! Tool execution dispatches `tools/call` to the running server process.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};

use crate::mcp::mcp_tool_name;
use crate::mcp_client::{McpClientBootstrap, McpClientTransport, McpStdioTransport};

/// An active MCP server connection.
pub struct McpConnection {
    pub server_name: String,
    pub tool_prefix: String,
    pub tools: Vec<McpToolDef>,
    child: Option<Child>,
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
                "version": "1.0.1"
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
    })
}

/// Call an MCP tool on a running connection.
pub fn call_tool(
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

    // Extract content from result
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
            // Other transports need async HTTP/WebSocket — not yet implemented
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
    // Read Content-Length header
    let mut header = String::new();
    loop {
        header.clear();
        if reader.read_line(&mut header).ok()? == 0 {
            return None;
        }
        let trimmed = header.trim();
        if trimmed.is_empty() {
            break; // End of headers
        }
        if trimmed.starts_with("Content-Length:") {
            // Parse length but we'll just read until we get valid JSON
        }
    }

    // Read body line
    let mut body = String::new();
    reader.read_line(&mut body).ok()?;
    serde_json::from_str(body.trim()).ok()
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

impl Drop for McpConnection {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
        }
    }
}
