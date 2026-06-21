use serde_json::{json, Value};

pub(crate) fn string_field(action: &Value, key: &str) -> Option<String> {
    action
        .get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

pub(crate) fn string_array_field(action: &Value, key: &str) -> Option<Vec<String>> {
    action.get(key).and_then(Value::as_array).map(|values| {
        values
            .iter()
            .filter_map(Value::as_str)
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    })
}

pub(crate) fn target_field<'a>(action: &'a Value, key: &str) -> Option<&'a str> {
    action
        .get("target")
        .and_then(|target| match target {
            Value::Object(map) => map.get(key).and_then(Value::as_str),
            _ => None,
        })
        .or_else(|| action.get(key).and_then(Value::as_str))
}

pub(crate) fn fields_field<'a>(action: &'a Value, key: &str) -> Option<&'a Value> {
    action.get("fields")?.get(key)
}

pub(crate) fn fields_string(action: &Value, key: &str) -> Option<String> {
    fields_field(action, key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

pub(crate) fn fields_string_array(action: &Value, key: &str) -> Option<Vec<String>> {
    fields_field(action, key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
}

pub(crate) fn set_optional_string_value(target: &mut Value, key: &str, value: Option<String>) {
    if let Some(value) = value {
        set_string_value(target, key, &value);
    }
}

pub(crate) fn copy_analysis(target: &mut Value, source: &Value) {
    if let Some(analysis) = source.get("analysis").cloned() {
        if !target.is_object() {
            *target = json!({});
        }
        if let Value::Object(map) = target {
            map.insert("analysis".to_string(), analysis);
        }
    }
}

fn analysis_field<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value.get("analysis")?.get(key)
}

pub(crate) fn analysis_string(value: &Value, key: &str) -> Option<String> {
    analysis_field(value, key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

pub(crate) fn analysis_string_array(value: &Value, key: &str) -> Vec<String> {
    analysis_field(value, key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

pub(crate) fn format_joined(values: Option<Vec<String>>) -> String {
    values.unwrap_or_default().join(", ")
}

pub(crate) fn set_string_value(target: &mut Value, key: &str, value: &str) {
    if !target.is_object() {
        *target = json!({});
    }
    if let Value::Object(map) = target {
        map.insert(key.to_string(), Value::String(value.to_string()));
    }
}

pub(crate) fn set_string_array_value(target: &mut Value, key: &str, values: &[String]) {
    if !target.is_object() {
        *target = json!({});
    }
    if let Value::Object(map) = target {
        map.insert(
            key.to_string(),
            Value::Array(values.iter().cloned().map(Value::String).collect()),
        );
    }
}

pub(crate) fn set_target_string_value(target: &mut Value, key: &str, value: &str) {
    if !target.is_object() {
        *target = json!({});
    }
    if let Value::Object(map) = target {
        let target_entry = map.entry("target".to_string()).or_insert_with(|| json!({}));
        if !target_entry.is_object() {
            *target_entry = json!({});
        }
        if let Value::Object(target_map) = target_entry {
            target_map.insert(key.to_string(), Value::String(value.to_string()));
        }
    }
}

pub(crate) fn remove_target_value(target: &mut Value, key: &str) {
    if let Some(Value::Object(target_map)) = target.get_mut("target") {
        target_map.remove(key);
    }
}
