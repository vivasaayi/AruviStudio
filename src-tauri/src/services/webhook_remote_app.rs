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
}
