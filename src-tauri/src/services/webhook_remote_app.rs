pub const REMOTE_APP_HTML: &str = include_str!("remote_app.html");
const WORKFLOW_TOPOLOGY: &str = include_str!("../../resources/workflow-topology.json");

pub fn render_remote_app() -> &'static str {
    static RENDERED: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    RENDERED.get_or_init(|| insert_topology(REMOTE_APP_HTML, WORKFLOW_TOPOLOGY)).as_str()
}

fn insert_topology(html: &str, topology: &str) -> String {
    // Escape '<' so a future label cannot terminate the JSON script element.
    html.replace("__ARUVI_WORKFLOW_TOPOLOGY__", &topology.replace('<', "\\u003c"))
}

#[cfg(test)]
mod tests {
    use super::{insert_topology, render_remote_app, REMOTE_APP_HTML, WORKFLOW_TOPOLOGY};

    #[test]
    fn injects_valid_shared_topology_as_json() {
        let topology: serde_json::Value = serde_json::from_str(WORKFLOW_TOPOLOGY).unwrap();
        assert_eq!(topology["nodes"].as_array().unwrap().len(), 20);
        let rendered = render_remote_app();
        assert!(!rendered.contains("__ARUVI_WORKFLOW_TOPOLOGY__"));
        assert!(rendered.contains("id=\"workflowTopology\""));
        assert!(rendered.contains("\"planning_split\""));
        let embedded = rendered.split("id=\"workflowTopology\">").nth(1).unwrap().split("</script>").next().unwrap();
        assert_eq!(serde_json::from_str::<serde_json::Value>(embedded).unwrap(), topology);
        let hostile = insert_topology("<script>__ARUVI_WORKFLOW_TOPOLOGY__</script>", r#"{"label":"</script>"}"#);
        assert_eq!(hostile.matches("</script>").count(), 1);
        assert!(hostile.contains(r"\u003c/script>"));
    }

    #[test]
    fn product_overview_uses_summary_endpoint_by_default() {
        assert!(REMOTE_APP_HTML
            .contains("/api/mobile/products/\" + encodeURIComponent(productId) + \"/summary"));
        assert!(REMOTE_APP_HTML.contains("Load product tree"));
    }

    #[test]
    fn remote_app_exposes_operational_workflow_controls() {
        for expected in [
            "Submit Work",
            "Approve &amp; Start",
            "/api/mobile/work-items",
            "/approve",
            "/workflow/start",
            "/delivery",
            "/action",
        ] {
            assert!(
                REMOTE_APP_HTML.contains(expected),
                "remote app is missing {expected}"
            );
        }
    }
}
