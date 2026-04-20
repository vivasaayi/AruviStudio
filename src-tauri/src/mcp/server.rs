use crate::mcp::protocol;
use crate::mcp::tools;
use crate::state::AppState;
use serde_json::{json, Value};
use std::io::{self, BufReader};
use tokio::runtime::Runtime;

const DEFAULT_PROTOCOL_VERSION: &str = "2024-11-05";

pub struct McpServer {
    state: AppState,
}

impl McpServer {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    pub fn serve(&mut self, runtime: &Runtime) -> Result<(), Box<dyn std::error::Error>> {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut reader = BufReader::new(stdin.lock());
        let mut writer = stdout.lock();

        while let Some(body) = protocol::read_message(&mut reader)? {
            if let Some(response) = runtime.block_on(self.handle_body(&body)) {
                protocol::write_message(&mut writer, &response)?;
            }
        }

        Ok(())
    }

    async fn handle_body(&self, body: &str) -> Option<Value> {
        let parsed = match serde_json::from_str::<Value>(body) {
            Ok(value) => value,
            Err(error) => {
                return Some(error_response(
                    Value::Null,
                    -32700,
                    "Parse error",
                    Some(json!({ "details": error.to_string() })),
                ))
            }
        };

        handle_json_rpc_value(&self.state, parsed).await
    }
}

pub async fn handle_json_rpc_value(state: &AppState, parsed: Value) -> Option<Value> {
    match parsed {
        Value::Array(batch) => handle_batch(state, batch).await,
        Value::Object(object) => handle_object(state, object).await,
        _ => Some(error_response(
            Value::Null,
            -32600,
            "Invalid Request",
            Some(json!({ "details": "Request must be a JSON object or batch array" })),
        )),
    }
}

async fn handle_batch(state: &AppState, batch: Vec<Value>) -> Option<Value> {
    if batch.is_empty() {
        return Some(error_response(
            Value::Null,
            -32600,
            "Invalid Request",
            Some(json!({ "details": "Batch requests cannot be empty" })),
        ));
    }

    let mut responses = Vec::new();
    for item in batch {
        match item {
            Value::Object(object) => {
                if let Some(response) = handle_object(state, object).await {
                    responses.push(response);
                }
            }
            _ => responses.push(error_response(
                Value::Null,
                -32600,
                "Invalid Request",
                Some(json!({ "details": "Batch items must be JSON objects" })),
            )),
        }
    }

    if responses.is_empty() {
        None
    } else {
        Some(Value::Array(responses))
    }
}

async fn handle_object(state: &AppState, object: serde_json::Map<String, Value>) -> Option<Value> {
    let id = object.get("id").cloned();
    let method = object.get("method").and_then(Value::as_str);
    let params = object.get("params").cloned();

    let Some(method) = method else {
        if object.contains_key("result") || object.contains_key("error") {
            return None;
        }

        return Some(error_response(
            id.unwrap_or(Value::Null),
            -32600,
            "Invalid Request",
            Some(json!({ "details": "Missing method" })),
        ));
    };

    if id.is_none() {
        handle_notification(method, params).await;
        return None;
    }

    let id = id.unwrap_or(Value::Null);
    let result = match method {
        "initialize" => Ok(handle_initialize(params)),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools::definitions() })),
        "tools/call" => handle_tool_call(state, params).await,
        "resources/list" => Ok(json!({ "resources": [] })),
        "resources/templates/list" => Ok(json!({ "resourceTemplates": [] })),
        "prompts/list" => Ok(json!({ "prompts": [] })),
        _ => Err(error_response(
            id.clone(),
            -32601,
            "Method not found",
            Some(json!({ "method": method })),
        )),
    };

    Some(match result {
        Ok(result) => success_response(id, result),
        Err(error) => error,
    })
}

async fn handle_notification(_method: &str, _params: Option<Value>) {}

fn handle_initialize(params: Option<Value>) -> Value {
    let negotiated_protocol = params
        .as_ref()
        .and_then(|value| value.get("protocolVersion"))
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_PROTOCOL_VERSION);

    json!({
        "protocolVersion": negotiated_protocol,
        "capabilities": {
            "tools": {
                "listChanged": false
            },
            "resources": {
                "listChanged": false,
                "subscribe": false
            },
            "prompts": {
                "listChanged": false
            }
        },
        "serverInfo": {
            "name": "Aruvi Studio MCP",
            "version": env!("CARGO_PKG_VERSION")
        }
    })
}

async fn handle_tool_call(state: &AppState, params: Option<Value>) -> Result<Value, Value> {
    let params = params.ok_or_else(|| {
        error_response(
            Value::Null,
            -32602,
            "Invalid params",
            Some(json!({ "details": "Missing tool call params" })),
        )
    })?;
    let params_object = params.as_object().ok_or_else(|| {
        error_response(
            Value::Null,
            -32602,
            "Invalid params",
            Some(json!({ "details": "Tool call params must be an object" })),
        )
    })?;

    let tool_name = params_object
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            error_response(
                Value::Null,
                -32602,
                "Invalid params",
                Some(json!({ "details": "Tool call params are missing name" })),
            )
        })?;
    let arguments = params_object
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let payload = match tools::dispatch_tool(state, tool_name, arguments).await {
        Ok(value) => success_tool_result(value),
        Err(error) => error_tool_result(tool_name, &error.to_string()),
    };

    Ok(payload)
}

fn success_response(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

fn error_response(id: Value, code: i64, message: &str, data: Option<Value>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message,
            "data": data
        }
    })
}

fn success_tool_result(payload: Value) -> Value {
    let text = serde_json::to_string_pretty(&payload).unwrap_or_else(|_| payload.to_string());
    json!({
        "content": [
            {
                "type": "text",
                "text": text
            }
        ],
        "structuredContent": payload
    })
}

fn error_tool_result(tool_name: &str, message: &str) -> Value {
    json!({
        "content": [
            {
                "type": "text",
                "text": format!("{tool_name} failed: {message}")
            }
        ],
        "isError": true
    })
}
