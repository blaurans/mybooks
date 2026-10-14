use crate::handlers::AppState;
use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::State;
use sqlx::Row;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    books_count: i64,
}

pub async fn index(State(state): State<AppState>) -> impl IntoResponse {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM books")
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);
    IndexTemplate { books_count: count }
}
