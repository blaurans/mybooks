mod auth;
mod books_service;
mod config;
mod db;
mod error;
mod handlers;
mod middleware;
mod models;
use std::net::SocketAddr;
use tower_http::services::ServeDir;
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::db::connect;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cfg = Config::from_env();
    let pool = connect(&cfg.database_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    // Rebuild FTS index for external-content table (safe, fast for small DBs).
    let _ = sqlx::query("INSERT INTO books_fts(books_fts) VALUES('rebuild')")
        .execute(&pool)
        .await;

    let app = handlers::router(pool.clone(), cfg.clone())
        .nest_service("/static", ServeDir::new("static"));

    let addr = SocketAddr::from(([0, 0, 0, 0], cfg.port));
    tracing::info!("mybooks listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
