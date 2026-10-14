pub mod books;
pub mod health;

use crate::config::Config;
use crate::db::SqlitePool;
use crate::middleware::auth_middleware;
use axum::{
    extract::State,
    middleware::from_fn_with_state,
    routing::{get, post},
    Router,
};

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
        .route("/library", get(crate::handlers::books::library))
        .route("/books/new", get(crate::handlers::books::new_book))
        .route("/books/from-isbn", post(crate::handlers::books::create_from_isbn))
        .route("/books/manual", post(crate::handlers::books::create_manual))
        .route("/books/:id", get(crate::handlers::books::detail))
        .route("/books/:id", post(crate::handlers::books::update))
        .route("/books/:id/delete", post(crate::handlers::books::delete))
        .route("/scan", get(crate::handlers::books::scan))
        .route("/api/isbn/:isbn", get(crate::handlers::books::lookup_api))
        .route_layer(from_fn_with_state(state.clone(), auth_middleware));

    Router::new()
        .route("/health", get(crate::handlers::health::health))
        .route("/auth/login", get(crate::auth::login))
        .route("/auth/callback", get(crate::auth::callback))
        .route("/auth/logout", get(crate::auth::logout))
        .merge(protected)
        .with_state(state.clone())
}
