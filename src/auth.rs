use crate::handlers::AppState;
use axum::{
    extract::{Query as AxumQuery, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use base64::Engine as _;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::collections::HashMap;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct CurrentUser {
    pub id: String,
    pub email: String,
    pub name: String,
    pub role: String,
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

fn is_secure(cfg: &crate::config::Config) -> bool {
    cfg.base_url.starts_with("https://")
}

fn get_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
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

fn set_cookie(name: &str, value: &str, max_age: i64, secure: bool, path: &str) -> String {
    let mut s = format!(
        "{}={}; HttpOnly; Path={}; Max-Age={}; SameSite=Lax",
        name, value, path, max_age
    );
    if secure {
        s.push_str("; Secure");
    }
    s
}

fn sha256_hex(input: &str) -> String {
    let hash = Sha256::digest(input.as_bytes());
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}

fn make_pkce() -> (String, String) {
    // verifier: 32 random bytes -> base64url
    let mut buf = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut buf);
    let verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(buf);
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    (verifier, challenge)
}

pub async fn login(State(state): State<AppState>) -> Response {
    let cfg = &state.cfg;
    let client_id = match &cfg.google_client_id {
        Some(v) if !v.is_empty() => v.clone(),
        _ => return (StatusCode::INTERNAL_SERVER_ERROR, "GOOGLE_CLIENT_ID manquant").into_response(),
    };
    if cfg.google_client_secret.is_none() {
        return (StatusCode::INTERNAL_SERVER_ERROR, "GOOGLE_CLIENT_SECRET manquant").into_response();
    }

    let (verifier, challenge) = make_pkce();
    let state_str = uuid::Uuid::new_v4().to_string();
    let secure = is_secure(cfg);
    let redirect_uri = format!("{}/auth/callback", cfg.base_url.trim_end_matches('/'));

    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    serializer.append_pair("client_id", &client_id);
    serializer.append_pair("redirect_uri", &redirect_uri);
    serializer.append_pair("response_type", "code");
    serializer.append_pair("scope", "openid email profile");
    serializer.append_pair("state", &state_str);
    serializer.append_pair("code_challenge", &challenge);
    serializer.append_pair("code_challenge_method", "S256");
    let url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?{}",
        serializer.finish()
    );

    let oauth_cookie = set_cookie(
        "oauth_state",
        &format!("{}.{}", state_str, verifier),
        600,
        secure,
        "/",
    );

    let mut resp = Redirect::temporary(&url).into_response();
    if let Ok(v) = oauth_cookie.parse() {
        resp.headers_mut().insert(header::SET_COOKIE, v);
    }
    resp
}

pub async fn callback(
    State(state): State<AppState>,
    AxumQuery(q): AxumQuery<CallbackQuery>,
    headers: HeaderMap,
) -> Response {
    let cfg = &state.cfg;
    let secure = is_secure(cfg);

    // state from cookie
    let cookie_val = match get_cookie(&headers, "oauth_state") {
        Some(v) => v,
        None => return (StatusCode::BAD_REQUEST, "state cookie manquant").into_response(),
    };
    let mut parts = cookie_val.splitn(2, '.');
    let c_state = parts.next().unwrap_or("");
    let verifier = parts.next().unwrap_or("");
    let q_state = q.state.clone().unwrap_or_default();
    if c_state.is_empty() || verifier.is_empty() || q_state != c_state {
        return (StatusCode::BAD_REQUEST, "state invalide").into_response();
    }
    if let Some(err) = &q.error {
        return (StatusCode::BAD_REQUEST, format!("erreur oauth: {}", err)).into_response();
    }
    let code = match &q.code {
        Some(c) if !c.is_empty() => c.clone(),
        _ => return (StatusCode::BAD_REQUEST, "code manquant").into_response(),
    };

    let client_id = match &cfg.google_client_id {
        Some(v) if !v.is_empty() => v.clone(),
        _ => return (StatusCode::INTERNAL_SERVER_ERROR, "client id manquant").into_response(),
    };
    let client_secret = match &cfg.google_client_secret {
        Some(v) if !v.is_empty() => v.clone(),
        _ => return (StatusCode::INTERNAL_SERVER_ERROR, "client secret manquant").into_response(),
    };
    let redirect_uri = format!("{}/auth/callback", cfg.base_url.trim_end_matches('/'));

    // exchange
    let form = HashMap::from([
        ("grant_type", "authorization_code"),
        ("code", &code),
        ("redirect_uri", &redirect_uri),
        ("client_id", &client_id),
        ("client_secret", &client_secret),
        ("code_verifier", verifier),
    ]);
    let token_resp = state
        .http
        .post("https://oauth2.googleapis.com/token")
        .form(&form)
        .send()
        .await;
    let token_resp = match token_resp {
        Ok(r) => r,
        Err(e) => return (StatusCode::BAD_GATEWAY, format!("token request: {}", e)).into_response(),
    };
    if !token_resp.status().is_success() {
        let txt = token_resp.text().await.unwrap_or_default();
        return (StatusCode::BAD_GATEWAY, format!("token error: {}", txt)).into_response();
    }
    #[derive(Deserialize)]
    struct TokenJson {
        access_token: String,
    }
    let token_json: TokenJson = match token_resp.json().await {
        Ok(t) => t,
        Err(e) => return (StatusCode::BAD_GATEWAY, format!("token parse: {}", e)).into_response(),
    };

    // userinfo
    let ui_resp = state
        .http
        .get("https://openidconnect.googleapis.com/v1/userinfo")
        .bearer_auth(&token_json.access_token)
        .send()
        .await;
    let ui_resp = match ui_resp {
        Ok(r) => r,
        Err(e) => return (StatusCode::BAD_GATEWAY, format!("userinfo request: {}", e)).into_response(),
    };
    if !ui_resp.status().is_success() {
        let txt = ui_resp.text().await.unwrap_or_default();
        return (StatusCode::BAD_GATEWAY, format!("userinfo error: {}", txt)).into_response();
    }
    #[derive(Deserialize)]
    struct UserInfo {
        sub: String,
        email: String,
        name: Option<String>,
        picture: Option<String>,
    }
    let info: UserInfo = match ui_resp.json().await {
        Ok(i) => i,
        Err(e) => return (StatusCode::BAD_GATEWAY, format!("userinfo parse: {}", e)).into_response(),
    };

    // allowlist: empty list means "allow all emails" (convenience mode)
    if !cfg.allowed_emails.is_empty()
        && !cfg
            .allowed_emails
            .iter()
            .any(|a| a.eq_ignore_ascii_case(&info.email))
    {
        return (StatusCode::FORBIDDEN, "email non autorisé").into_response();
    }
    if cfg.allowed_emails.is_empty() {
        tracing::warn!("ALLOWED_EMAILS vide : inscription ouverte à tous les emails");
    }

    let mut role = if cfg
        .admin_emails
        .iter()
        .any(|a| a.eq_ignore_ascii_case(&info.email))
    {
        "admin"
    } else {
        "member"
    };
    // Bootstrap: the first user in an empty DB becomes admin.
    if role != "admin" {
        let admin_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role = 'admin'")
            .fetch_one(&state.pool)
            .await
            .unwrap_or(0);
        if admin_count == 0 {
            role = "admin";
        }
    }
    let name = info.name.clone().unwrap_or_else(|| info.email.clone());

    // upsert user
    let user_row = sqlx::query(
        "INSERT INTO users (id, google_sub, email, name, avatar_url, role, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'), datetime('now'))
         ON CONFLICT(google_sub) DO UPDATE SET
           email=excluded.email,
           name=excluded.name,
           avatar_url=excluded.avatar_url,
           role=excluded.role,
           updated_at=datetime('now')
         RETURNING id, email, name, role",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(&info.sub)
    .bind(&info.email)
    .bind(&name)
    .bind(&info.picture)
    .bind(role)
    .fetch_one(&state.pool)
    .await;
    let user_row = match user_row {
        Ok(r) => r,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("db user: {}", e)).into_response(),
    };
    let user_id: String = user_row.get("id");
    let user_email: String = user_row.get("email");
    let user_name: String = user_row.get("name");
    let user_role: String = user_row.get("role");

    // create session
    let raw_token = uuid::Uuid::new_v4().to_string();
    let token_hash = sha256_hex(&raw_token);
    if let Err(e) = sqlx::query(
        "INSERT INTO sessions (token_hash, user_id, expires_at) VALUES (?1, ?2, datetime('now', '+30 days'))",
    )
    .bind(&token_hash)
    .bind(&user_id)
    .execute(&state.pool)
    .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("db session: {}", e)).into_response();
    }

    let sid_cookie = set_cookie("sid", &raw_token, 30 * 24 * 3600, secure, "/");
    let clear_oauth = set_cookie("oauth_state", "", 0, secure, "/");

    let mut resp = Redirect::to("/").into_response();
    if let Ok(v) = sid_cookie.parse() {
        resp.headers_mut().append(header::SET_COOKIE, v);
    }
    if let Ok(v) = clear_oauth.parse() {
        resp.headers_mut().append(header::SET_COOKIE, v);
    }
    let _ = (user_email, user_name, user_role);
    resp
}

pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let secure = is_secure(&state.cfg);
    if let Some(raw) = get_cookie(&headers, "sid") {
        let hash = sha256_hex(&raw);
        let _ = sqlx::query("DELETE FROM sessions WHERE token_hash = ?1")
            .bind(hash)
            .execute(&state.pool)
            .await;
    }
    let clear = set_cookie("sid", "", 0, secure, "/");
    let mut resp = Redirect::to("/auth/login").into_response();
    if let Ok(v) = clear.parse() {
        resp.headers_mut().insert(header::SET_COOKIE, v);
    }
    resp
}
