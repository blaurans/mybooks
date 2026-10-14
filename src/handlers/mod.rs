pub mod books;
pub mod health;

use crate::auth::CurrentUser;
use crate::config::Config;
use crate::db::SqlitePool;
use crate::middleware::auth_middleware;
use axum::{
    extract::{Request, State},
    middleware::from_fn_with_state,
    routing::get,
    Router,
};
use std::sync::Arc;
use tower_http::services::ServeDir;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub cfg: Config,
    pub http: reqwest::Client,
}

pub fn router(pool: SqlitePool, cfg: Config) -> Router {
    let state = AppState {
        pool,
        cfg: cfg.clone(),
        http: reqwest::Client::new(),
    };

    let protected = Router::new()
        .route("/", get(crate::handlers::books::index))
        .route_layer(from_fn_with_state(state.clone(), auth_middleware));

    Router::new()
        .route("/health", get(crate::handlers::health::health))
        .route("/auth/login", get(crate::auth::login))
        .route("/auth/callback", get(crate::auth::callback))
        .route("/auth/logout", get(crate::auth::logout))
        .merge(protected)
        .with_state(state.clone())
}
