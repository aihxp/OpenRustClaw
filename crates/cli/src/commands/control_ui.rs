use axum::response::Html;

const CONTROL_UI_HTML: &str = include_str!("control_ui.html");

pub fn dashboard() -> Html<&'static str> {
    Html(CONTROL_UI_HTML)
}

#[cfg(test)]
mod tests {
    use super::CONTROL_UI_HTML;

    #[test]
    fn dashboard_includes_session_continuity_panel() {
        assert!(CONTROL_UI_HTML.contains("id=\"session-continuity\""));
        assert!(CONTROL_UI_HTML.contains("Assistant Continuity"));
        assert!(CONTROL_UI_HTML.contains("function renderSessionContinuity"));
    }
}
