//! Bounded renderer for a caller-supplied, non-session Planner snapshot.
use crate::error::AppError;
use crate::services::planner_review_packet::DesignReviewPacket;
#[cfg(test)]
use rquickjs::{Context, Function, Object, Runtime};
use serde_json::Value;
use std::io::{self, Write};
use std::sync::{Arc, OnceLock};
#[cfg(test)]
use std::time::{Duration, Instant};

#[cfg(test)]
const RENDERER: &str = include_str!("../../resources/planner_packet_renderer.js");
const MAX_INPUT: usize = 8 * 1024 * 1024;
static SLOTS: OnceLock<Arc<tokio::sync::Semaphore>> = OnceLock::new();

struct BoundedWriter(usize);
impl Write for BoundedWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_INPUT.saturating_sub(self.0) {
            return Err(io::Error::other("snapshot exceeds input limit"));
        }
        self.0 += bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn render(snapshot: &Value) -> Result<DesignReviewPacket, AppError> {
    let object = snapshot
        .as_object()
        .ok_or_else(|| AppError::Validation("Planner packet snapshot must be an object".into()))?;
    if object
        .get("snapshotKind")
        .is_some_and(|value| !value.is_null() && value.as_str() != Some("proposal"))
    {
        return Err(AppError::Validation(
            "Unsupported Planner packet snapshot kind".into(),
        ));
    }
    for field in ["title", "generatedAt"] {
        if !object.get(field).is_some_and(Value::is_string) {
            return Err(AppError::Validation(format!(
                "Planner packet {field} must be a string"
            )));
        }
    }
    for field in [
        "currentProducts",
        "currentProductTrees",
        "currentWorkItems",
        "draftTreeNodes",
    ] {
        if !object.get(field).is_some_and(Value::is_array) {
            return Err(AppError::Validation(format!(
                "Planner packet {field} must be an array"
            )));
        }
    }
    if !object
        .get("currentWorkItemsHasMore")
        .is_some_and(Value::is_boolean)
        || !object.get("validation").is_some_and(Value::is_object)
    {
        return Err(AppError::Validation(
            "Planner packet snapshot is incomplete".into(),
        ));
    }
    let mut writer = BoundedWriter(0);
    serde_json::to_writer(&mut writer, snapshot)
        .map_err(|_| AppError::Validation("Planner packet snapshot exceeds input limit".into()))?;
    crate::services::planner_packet_native::render(snapshot)
}

#[cfg(test)]
fn render_legacy(snapshot: &Value) -> Result<DesignReviewPacket, AppError> {
    let request = serde_json::to_string(snapshot).map_err(|_| {
        AppError::Validation("Planner packet snapshot could not be rendered".into())
    })?;
    let runtime = Runtime::new()
        .map_err(|e| AppError::Internal(format!("Planner packet runtime unavailable: {e}")))?;
    runtime.set_memory_limit(128 * 1024 * 1024);
    runtime.set_max_stack_size(1024 * 1024);
    let deadline = Instant::now() + Duration::from_secs(10);
    runtime.set_interrupt_handler(Some(Box::new(move || Instant::now() >= deadline)));
    let context = Context::full(&runtime)
        .map_err(|e| AppError::Internal(format!("Planner packet context unavailable: {e}")))?;
    let output: String = context
        .with(|ctx| {
            ctx.eval::<(), _>(RENDERER)?;
            let api: Object = ctx.globals().get("AruviPlannerPacket")?;
            let function: Function = api.get("renderJson")?;
            function.call((request,))
        })
        .map_err(|_| {
            AppError::Validation("Planner packet snapshot could not be rendered".into())
        })?;
    serde_json::from_str(&output)
        .map_err(|_| AppError::Internal("Planner packet renderer returned invalid output".into()))
}

pub async fn render_snapshot(snapshot: Value) -> Result<DesignReviewPacket, AppError> {
    let permit = SLOTS
        .get_or_init(|| Arc::new(tokio::sync::Semaphore::new(2)))
        .clone()
        .acquire_owned()
        .await
        .map_err(|_| AppError::Internal("Planner packet renderer unavailable".into()))?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        render(&snapshot)
    })
    .await
    .map_err(|e| AppError::Internal(format!("Planner packet renderer task failed: {e}")))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn snapshot() -> Value {
        json!({"title":"<Inventory>","generatedAt":"fixture","activeProductName":null,
            "currentProducts":[],"currentProductTrees":[],"currentWorkItems":[],"currentWorkItemsHasMore":false,
            "draftTreeNodes":[],"plan":null,"validation":{"score":0,"issues":[]},"selectedNode":null,"latestAssistantText":null})
    }
    #[test]
    fn native_action_summaries_match_the_legacy_renderer() {
        let actions = vec![
            json!({"type":"create_product_area","name":"Area <&>","target":{"productName":"Shop"}}),
            json!({"type":"create_capability","target":{"capabilityName":"Stock"}}),
            json!({"type":"create_capability","name":"", "target":{"productName":"Shop","productAreaName":"Stock"}}),
            json!({"type":"create_capability", "target":{"productName":42,"productAreaName":"Stock"}}),
            json!({"type":"report_status", "target":{"productName":0,"workItemTitle":7}}),
            json!({"type":"create_work_item","title":"Count","description":"Verify & count"}),
            json!({"type":"apply_capability_template","description":"Outline"}),
            json!({"type":"convert_capability_kind"}),
            json!({"type":"convert_capability_kind", "nodeKind":1.0,"childStrategy":-0.0}),
            json!({"type":"update_product","target":{"productName":"Shop"},"fields":{"vision":"Great <world>"}}),
            json!({"type":"update_product_area","fields":{"x":2}}),
            json!({"type":"update_capability"}),
            json!({"type":"update_work_item"}),
            json!({"type":"report_tree"}),
            json!({"type":"report_status","target":{"workItemTitle":"Count"}}),
            json!({"type":"start_workflow","target":{"workItemTitle":"Count"}}),
            json!({"type":"unexpected_action","value":"<fixture>"}),
            json!(null),
        ];
        for action in actions {
            let mut input = snapshot();
            input["plan"] = json!({"actions":[action],"assistant_response":"Review"});
            let old = render_legacy(&input).unwrap().html;
            let new = render(&input).unwrap().html;
            let extract = |html: &str| {
                html.split("<div class=\"diff\">")
                    .nth(1)
                    .unwrap()
                    .split("</section>")
                    .next()
                    .unwrap()
                    .to_string()
            };
            assert_eq!(
                extract(&new),
                extract(&old),
                "action {}",
                input["plan"]["actions"][0]
            );
            let extract_json = |html: &str| {
                html.split("<pre>")
                    .nth(1)
                    .unwrap()
                    .split("</pre>")
                    .next()
                    .unwrap()
                    .to_string()
            };
            assert_eq!(
                extract_json(&new),
                extract_json(&old),
                "structured change set"
            );
        }
    }
    #[tokio::test]
    async fn renders_explicit_snapshot_and_rejects_invalid_or_large_input() {
        let packet = render_snapshot(snapshot()).await.unwrap();
        assert_eq!(packet.filename, "inventory-design-review-packet.html");
        assert!(packet.html.contains("&lt;Inventory&gt;"));
        assert!(!packet.html.contains("<Inventory>"));
        assert!(packet.html.contains("caller-supplied snapshot"));
        let mut detailed = snapshot();
        detailed["draftTreeNodes"] =
            json!([{"id":"area","label":"Stock <&>","node_type":"product_area","children":[]}]);
        detailed["plan"] = json!({"actions":[{"type":"create_work_item","title":"Count stock"}]});
        detailed["validation"] = json!({"score":72,"issues":[{"tone":"warn","title":"Review risk","detail":"Missing owner"}]});
        let html = render_snapshot(detailed).await.unwrap().html;
        for text in [
            "Stock &lt;&amp;&gt;",
            "Count stock",
            "Review risk",
            "Missing owner",
            "Complete staged design hierarchy",
        ] {
            assert!(html.contains(text), "missing {text}");
        }
        assert!(!html.contains("<script"));
        assert!(!html.contains("cdn.jsdelivr.net"));
        let mut malicious = snapshot();
        malicious["validation"]["score"] = json!("</div><img src=x onerror=alert(1)>");
        let html = render_snapshot(malicious).await.unwrap().html;
        assert!(html.contains("&lt;/div&gt;&lt;img src=x onerror=alert(1)&gt;"));
        assert!(!html.contains("<img src=x onerror=alert(1)>"));
        assert!(matches!(
            render_snapshot(json!({})).await,
            Err(AppError::Validation(_))
        ));
        let mut invalid_kind = snapshot();
        invalid_kind["snapshotKind"] = json!("unknown");
        assert!(matches!(
            render_snapshot(invalid_kind).await,
            Err(AppError::Validation(_))
        ));
        let mut large = snapshot();
        large["title"] = Value::String("x".repeat(MAX_INPUT + 1));
        assert!(matches!(
            render_snapshot(large).await,
            Err(AppError::Validation(_))
        ));
    }
}
