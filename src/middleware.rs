use crate::auth::CurrentUser;
use crate::handlers::AppState;
use axum::{
    extract::State,
    http::{header, Request},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};

fn is_public(path: &str) -> bool {
    path == "/health"
        || path.starts_with("/auth/")
        || path.starts_with("/static/")
}

fn get_cookie(headers: &axum::http::HeaderMap, name: &str) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    for part in raw.split(';') {
        let part = part.trim();
        if let Some((k, v)) = part.split_once('=') {
            if k == name {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn sha256_hex(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(input.as_bytes());
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let path = req.uri().path().to_string();
    if is_public(&path) {
        return next.run(req).await;
    }

    let user = if let Some(raw) = get_cookie(req.headers(), "sid") {
        let hash = sha256_hex(&raw);
        let row = sqlx::query(
            "SELECT u.id, u.email, u.name, u.role
             FROM sessions s JOIN users u ON u.id = s.user_id
             WHERE s.token_hash = ?1 AND s.expires_at > datetime('now')",
        )
        .bind(&hash)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten();
        row.map(|r| {
            use sqlx::Row;
            CurrentUser {
                id: r.get("id"),
                email: r.get("email"),
                name: r.get("name"),
                role: r.get("role"),
            }
        })
    } else {
        None
    };

    match user {
        Some(u) => {
            req.extensions_mut().insert(u);
            next.run(req).await
        }
        None => Redirect::to("/auth/login").into_response(),
    }
}
