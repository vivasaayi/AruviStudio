use serde_json::{json, Value};

const DEFAULT_PROTOCOL_VERSION: &str = "2025-11-25";
const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &[DEFAULT_PROTOCOL_VERSION];

pub(super) fn handle_initialize(params: Option<Value>) -> Value {
    let requested_protocol = params
        .as_ref()
        .and_then(|value| value.get("protocolVersion"))
        .and_then(Value::as_str);
    let negotiated_protocol = negotiate_protocol_version(requested_protocol);

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

fn negotiate_protocol_version(requested_protocol: Option<&str>) -> &'static str {
    match requested_protocol {
        Some(requested) => SUPPORTED_PROTOCOL_VERSIONS
            .iter()
            .copied()
            .find(|supported| *supported == requested)
            .unwrap_or(DEFAULT_PROTOCOL_VERSION),
        _ => DEFAULT_PROTOCOL_VERSION,
    }
}

pub(super) fn success_response(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

pub(super) fn error_response(id: Value, code: i64, message: &str, data: Option<Value>) -> Value {
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

pub(super) fn success_tool_result(payload: Value) -> Value {
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

pub(super) fn error_tool_result(tool_name: &str, message: &str) -> Value {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_defaults_to_latest_supported_protocol_version() {
        let response = handle_initialize(None);
        assert_eq!(
            response
                .get("protocolVersion")
                .and_then(Value::as_str)
                .expect("protocolVersion"),
            DEFAULT_PROTOCOL_VERSION
        );
    }

    #[test]
    fn initialize_echoes_requested_protocol_version() {
        let response = handle_initialize(Some(json!({
            "protocolVersion": "2025-11-25"
        })));
        assert_eq!(
            response
                .get("protocolVersion")
                .and_then(Value::as_str)
                .expect("protocolVersion"),
            "2025-11-25"
        );
    }

    #[test]
    fn initialize_negotiates_to_supported_protocol_version_when_request_is_unsupported() {
        let response = handle_initialize(Some(json!({
            "protocolVersion": "2099-01-01"
        })));
        assert_eq!(
            response
                .get("protocolVersion")
                .and_then(Value::as_str)
                .expect("protocolVersion"),
            DEFAULT_PROTOCOL_VERSION
        );
    }
}
