use std::env;

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub port: u16,
    pub base_url: String,
    pub google_client_id: Option<String>,
    pub google_client_secret: Option<String>,
    pub allowed_emails: Vec<String>,
    pub admin_emails: Vec<String>,
}

impl Config {
    pub fn from_env() -> Self {
        let allowed = env::var("ALLOWED_EMAILS")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        let admins = env::var("ADMIN_EMAILS")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();

        Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:///data/mybooks.db".to_string()),
            port: env::var("PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3000),
            base_url: env::var("APP_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
            google_client_id: env::var("GOOGLE_CLIENT_ID").ok(),
            google_client_secret: env::var("GOOGLE_CLIENT_SECRET").ok(),
            allowed_emails: allowed,
            admin_emails: admins,
        }
    }
}
