use crate::auth::CurrentUser;
use crate::books_service::{
    delete_user, list_user_books, list_users_for_admin, set_user_role, AdminUserRow, BookCard,
};
use crate::handlers::AppState;
use askama::Template;
use askama_axum::IntoResponse as TemplateResponse;
use axum::{
    extract::{Extension, Form, Path, Query, State},
    http::StatusCode,
    response::{Redirect, Response},
};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Template)]
#[template(path = "admin.html")]
pub struct AdminIndexTemplate {
    pub user_name: String,
    pub users: Vec<AdminUserRow>,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "admin_user.html")]
pub struct AdminUserTemplate {
    pub user_name: String,
    pub target_email: String,
    pub target_name: String,
    pub book_count: i64,
    pub books: Vec<BookCard>,
}

#[derive(Deserialize)]
pub struct RoleForm {
    pub role: String,
}

fn deny() -> Response {
    (StatusCode::FORBIDDEN, "403 — accès réservé aux admins").into_response()
}

fn is_admin(user: &CurrentUser) -> bool {
    user.role == "admin"
}

pub async fn index(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Query(q): Query<HashMap<String, String>>,
) -> Response {
    if !is_admin(&user) {
        return deny();
    }
    let users = list_users_for_admin(&state.pool).await.unwrap_or_default();
    TemplateResponse::into_response(AdminIndexTemplate {
        user_name: user.name,
        users,
        message: q.get("msg").cloned(),
        error: q.get("error").cloned(),
    })
}

pub async fn user_detail(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<String>,
) -> Response {
    if !is_admin(&user) {
        return deny();
    }
    let users = match list_users_for_admin(&state.pool).await {
        Ok(u) => u,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    };
    let Some(target) = users.into_iter().find(|u| u.id == id) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let books = list_user_books(&state.pool, &target.id, "", "", "")
        .await
        .unwrap_or_default();
    TemplateResponse::into_response(AdminUserTemplate {
        user_name: user.name,
        target_email: target.email,
        target_name: target.name,
        book_count: books.len() as i64,
        books,
    })
}

pub async fn update_role(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<String>,
    Form(f): Form<RoleForm>,
) -> Response {
    if !is_admin(&user) {
        return deny();
    }
    if id == user.id {
        return Redirect::to("/admin?error=Tu+ne+peux+pas+changer+ton+propre+rôle").into_response();
    }
    let role = if f.role == "admin" || f.role == "member" {
        f.role.clone()
    } else {
        "member".to_string()
    };
    match set_user_role(&state.pool, &id, &role).await {
        Ok(true) => Redirect::to("/admin?msg=Rôle+mis+à+jour").into_response(),
        Ok(false) => {
            Redirect::to("/admin?error=Opération+refusée (dernier+admin)").into_response()
        }
        Err(e) => Redirect::to(&format!("/admin?error={}", e)).into_response(),
    }
}

pub async fn delete_user_handler(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<String>,
) -> Response {
    if !is_admin(&user) {
        return deny();
    }
    if id == user.id {
        return Redirect::to("/admin?error=Tu+ne+peux+pas+te+supprimer+toi-même").into_response();
    }
    match delete_user(&state.pool, &id).await {
        Ok(true) => Redirect::to("/admin?msg=Utilisateur+supprimé").into_response(),
        Ok(false) => {
            Redirect::to("/admin?error=Suppression+refusée (dernier+admin)").into_response()
        }
        Err(e) => Redirect::to(&format!("/admin?error={}", e)).into_response(),
    }
}
