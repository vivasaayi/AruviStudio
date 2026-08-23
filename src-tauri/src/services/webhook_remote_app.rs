pub const REMOTE_APP_HTML: &str = include_str!("remote_app.html");

#[cfg(test)]
mod tests {
    use super::REMOTE_APP_HTML;

    #[test]
    fn product_overview_uses_summary_endpoint_by_default() {
        assert!(REMOTE_APP_HTML
            .contains("/api/mobile/products/\" + encodeURIComponent(productId) + \"/summary"));
        assert!(!REMOTE_APP_HTML
            .contains("/api/mobile/products/\" + encodeURIComponent(productId) + \"/tree"));
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
