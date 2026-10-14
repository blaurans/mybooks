use axum::response::IntoResponse;

pub async fn health() -> &'static str {
    "ok"
}
