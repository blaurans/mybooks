pub mod auth;
pub mod books;
pub mod health;

use crate::config::Config;
use crate::db::SqlitePool;
use axum::{routing::get, Router};

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub cfg: Config,
}

pub fn router(pool: SqlitePool, cfg: Config) -> Router {
    let state = AppState { pool, cfg };
    Router::new()
        .route("/", get(crate::handlers::books::index))
        .route("/health", get(crate::handlers::health::health))
        .route("/auth/login", get(crate::handlers::auth::login))
        .route("/auth/callback", get(crate::handlers::auth::callback))
        .route("/auth/logout", get(crate::handlers::auth::logout))
        .with_state(state)
}
