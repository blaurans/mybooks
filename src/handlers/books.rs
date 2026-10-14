use crate::auth::CurrentUser;
use crate::handlers::AppState;
use askama::Template;
use askama_axum::IntoResponse;
use axum::{
    extract::{Extension, State},
};
use sqlx::Row;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    books_count: i64,
    user_name: String,
}

pub async fn index(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
) -> impl IntoResponse {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM books")
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);
    IndexTemplate {
        books_count: count,
        user_name: user.name,
    }
}
