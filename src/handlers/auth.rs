use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
};
use crate::handlers::AppState;

pub async fn login(State(_state): State<AppState>) -> impl IntoResponse {
    // TODO: Google OAuth redirect (mise en place plus tard mais branche prévu ici).
    Redirect::temporary("/?auth=todo")
}

pub async fn callback(State(_state): State<AppState>) -> &'static str {
    "callback (todo)"
}

pub async fn logout() -> &'static str {
    "logout (todo)"
}
