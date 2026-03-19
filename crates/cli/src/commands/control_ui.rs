use axum::response::Html;

const CONTROL_UI_HTML: &str = include_str!("control_ui.html");

pub fn dashboard() -> Html<&'static str> {
    Html(CONTROL_UI_HTML)
}
