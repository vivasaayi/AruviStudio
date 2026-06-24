use crate::mcp::protocol;
use crate::mcp::resources;
use crate::mcp::server_responses::{
    error_response, error_tool_result, handle_initialize, success_response, success_tool_result,
};
use crate::mcp::tools;
use crate::state::AppState;
use serde_json::{json, Value};
use std::io::{self, BufReader};
use tokio::runtime::Runtime;

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
        "resources/list" => handle_resources_list(id.clone(), params),
        "resources/read" => handle_resources_read(id.clone(), params),
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

fn handle_resources_list(id: Value, params: Option<Value>) -> Result<Value, Value> {
    if let Some(value) = params {
        if !value.is_object() {
            return Err(error_response(
                id,
                -32602,
                "Invalid params",
                Some(json!({ "details": "resources/list params must be an object when provided" })),
            ));
        }
    }

    Ok(json!({
        "resources": resources::definitions()
    }))
}

fn handle_resources_read(id: Value, params: Option<Value>) -> Result<Value, Value> {
    let params = params.ok_or_else(|| {
        error_response(
            id.clone(),
            -32602,
            "Invalid params",
            Some(json!({ "details": "Missing resources/read params" })),
        )
    })?;
    let params_object = params.as_object().ok_or_else(|| {
        error_response(
            id.clone(),
            -32602,
            "Invalid params",
            Some(json!({ "details": "resources/read params must be an object" })),
        )
    })?;
    let uri = params_object
        .get("uri")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            error_response(
                id.clone(),
                -32602,
                "Invalid params",
                Some(json!({ "details": "resources/read params are missing uri" })),
            )
        })?;

    match resources::read(uri) {
        Some(content) => Ok(json!({
            "contents": [content]
        })),
        None => Err(error_response(
            id,
            -32002,
            "Resource not found",
            Some(json!({ "uri": uri })),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resources_list_returns_doctrine_resources() {
        let response = handle_resources_list(Value::from(1), None).expect("resources/list");
        let resources = response
            .get("resources")
            .and_then(Value::as_array)
            .expect("resources array");
        assert!(resources.iter().any(|resource| {
            resource.get("uri").and_then(Value::as_str) == Some("aruvi://guides/product-philosophy")
        }));
    }

    #[test]
    fn resources_read_returns_markdown_contents() {
        let response = handle_resources_read(
            Value::from(1),
            Some(json!({
                "uri": "aruvi://guides/product-philosophy"
            })),
        )
        .expect("resources/read");
        let contents = response
            .get("contents")
            .and_then(Value::as_array)
            .expect("contents array");
        assert!(contents.iter().any(|entry| {
            entry
                .get("text")
                .and_then(Value::as_str)
                .map(|text| text.contains("# Aruvi Product Philosophy"))
                .unwrap_or(false)
        }));
    }

    #[test]
    fn resources_read_returns_not_found_for_unknown_uri() {
        let error = handle_resources_read(
            Value::from(1),
            Some(json!({
                "uri": "aruvi://guides/not-real"
            })),
        )
        .expect_err("resources/read should fail");
        assert_eq!(
            error
                .get("error")
                .and_then(|value| value.get("code"))
                .and_then(Value::as_i64),
            Some(-32002)
        );
    }
}
