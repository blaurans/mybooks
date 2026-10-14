use crate::auth::CurrentUser;
use crate::books_service::{
    count_read_this_year, create_book, current_year, fetch_openlibrary, find_book_by_isbn, get_goal,
    get_user_book, link_user_book, list_user_books, normalize_isbn, remove_user_book, set_goal,
    update_user_book, BookCard, BookMeta, UserBookDetail,
};
use crate::handlers::AppState;
use askama::Template;
use askama_axum::IntoResponse as TemplateResponse;
use axum::{
    extract::{Extension, Form, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Template)]
#[template(path = "library.html")]
struct LibraryTemplate {
    user_name: String,
    books: Vec<BookCard>,
    q: String,
    status: String,
    tag: String,
    goal_year: i32,
    goal_target: i64,
    goal_read: i64,
}

#[derive(Template)]
#[template(path = "goals.html")]
pub struct GoalTemplate {
    pub user_name: String,
    pub year: i32,
    pub target: i64,
    pub read: i64,
    pub message: Option<String>,
}

#[derive(Template)]
#[template(path = "book_new.html")]
pub struct BookNewTemplate {
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "scan.html")]
pub struct ScanTemplate {
    pub user_name: String,
}
#[derive(Template)]
#[template(path = "book_detail.html")]
pub struct BookDetailTemplate {
    pub book: UserBookDetail,
    pub rating_display: f64,
}

#[derive(Deserialize, Default)]
pub struct LibraryQuery {
    pub q: Option<String>,
    pub status: Option<String>,
    pub tag: Option<String>,
}

#[derive(Deserialize)]
pub struct IsbnForm {
    pub isbn: String,
}

#[derive(Deserialize)]
pub struct ManualForm {
    pub title: String,
    pub authors: Option<String>,
    pub isbn: Option<String>,
    pub pages: Option<String>,
    pub publisher: Option<String>,
    pub published_date: Option<String>,
    pub cover_url: Option<String>,
}

#[derive(Deserialize)]
pub struct GoalForm {
    pub target_books: String,
}

#[derive(Deserialize)]
pub struct UpdateForm {
    pub status: String,
    pub progress_pct: Option<String>,
    pub current_page: Option<String>,
    pub rating: Option<String>,
    pub review: Option<String>,
    pub review_visibility: String,
    pub tags: Option<String>,
}

pub async fn index(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Query(q): Query<LibraryQuery>,
) -> impl IntoResponse {
    library_with(state, user, q).await
}

pub async fn library(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Query(q): Query<LibraryQuery>,
) -> impl IntoResponse {
    library_with(state, user, q).await
}

async fn library_with(state: AppState, user: CurrentUser, q: LibraryQuery) -> Response {
    let qstr = q.q.clone().unwrap_or_default();
    let status = q.status.clone().unwrap_or_default();
    let tag = q.tag.clone().unwrap_or_default();
    let year = current_year();
    let goal_target = get_goal(&state.pool, &user.id, year).await.unwrap_or(0);
    let goal_read = count_read_this_year(&state.pool, &user.id, year).await.unwrap_or(0);
    let books = list_user_books(&state.pool, &user.id, &qstr, &status, &tag)
        .await
        .unwrap_or_default();
    TemplateResponse::into_response(LibraryTemplate {
        user_name: user.name,
        books,
        q: qstr,
        status,
        tag,
        goal_year: year,
        goal_target,
        goal_read,
    })
}

pub async fn new_book(
    Query(q): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    TemplateResponse::into_response(BookNewTemplate {
        error: q.get("error").cloned(),
    })
}

pub async fn scan(Extension(user): Extension<CurrentUser>) -> impl IntoResponse {
    TemplateResponse::into_response(ScanTemplate {
        user_name: user.name,
    })
}

pub async fn create_from_isbn(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Form(f): Form<IsbnForm>,
) -> Response {
    let isbn = normalize_isbn(&f.isbn);
    if isbn.is_empty() {
        return Redirect::to("/books/new?error=ISBN+vide").into_response();
    }
    let book_id = match find_book_by_isbn(&state.pool, &isbn).await {
        Ok(Some(id)) => id,
        Ok(None) => match fetch_openlibrary(&state.http, &isbn).await {
            Ok(meta) => match create_book(&state.pool, &meta).await {
                Ok(id) => id,
                Err(e) => {
                    return Redirect::to(&format!("/books/new?error={}&isbn={}", urlencoding(&e), isbn))
                        .into_response()
                }
            },
            Err(e) => {
                return Redirect::to(&format!("/books/new?error={}&isbn={}", urlencoding(&e), isbn))
                    .into_response()
            }
        },
        Err(e) => {
            return Redirect::to(&format!("/books/new?error={}&isbn={}", urlencoding(&e), isbn))
                .into_response()
        }
    };

    if let Err(e) = link_user_book(&state.pool, &user.id, &book_id).await {
        return Redirect::to(&format!("/books/new?error={}", urlencoding(&e))).into_response();
    }
    Redirect::to(&format!("/books/{}", book_id)).into_response()
}

pub async fn create_manual(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Form(f): Form<ManualForm>,
) -> Response {
    let title = f.title.trim().to_string();
    if title.is_empty() {
        return Redirect::to("/books/new?error=Titre+requis").into_response();
    }
    let isbn = f.isbn.clone().map(|s| normalize_isbn(&s)).filter(|s| !s.is_empty());
    let meta = BookMeta {
        isbn: isbn.clone(),
        title,
        authors: f
            .authors
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "Auteur inconnu".to_string()),
        publisher: f.publisher.clone().filter(|s| !s.trim().is_empty()),
        published_date: f.published_date.clone().filter(|s| !s.trim().is_empty()),
        pages: f.pages.as_deref().and_then(|s| s.trim().parse().ok()),
        cover_url: f.cover_url.clone().filter(|s| !s.trim().is_empty()),
    };

    let book_id = if let Some(ref is) = meta.isbn {
        if let Ok(Some(existing)) = find_book_by_isbn(&state.pool, is).await {
            existing
        } else {
            match create_book(&state.pool, &meta).await {
                Ok(id) => id,
                Err(e) => return Redirect::to(&format!("/books/new?error={}", urlencoding(&e))).into_response(),
            }
        }
    } else {
        match create_book(&state.pool, &meta).await {
            Ok(id) => id,
            Err(e) => return Redirect::to(&format!("/books/new?error={}", urlencoding(&e))).into_response(),
        }
    };

    if let Err(e) = link_user_book(&state.pool, &user.id, &book_id).await {
        return Redirect::to(&format!("/books/new?error={}", urlencoding(&e))).into_response();
    }
    Redirect::to(&format!("/books/{}", book_id)).into_response()
}

pub async fn detail(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<String>,
) -> Response {
    match get_user_book(&state.pool, &user.id, &id).await {
        Ok(Some(book)) => {
            let rating_display = book.rating as f64 / 4.0;
            TemplateResponse::into_response(BookDetailTemplate {
                book,
                rating_display,
            })
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),

        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

pub async fn update(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<String>,
    Form(f): Form<UpdateForm>,
) -> Response {
    let status = if ["to_read", "reading", "read", "dropped"].contains(&f.status.as_str()) {
        f.status.clone()
    } else {
        "to_read".to_string()
    };
    let progress_pct: i64 = f
        .progress_pct
        .as_deref()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    let current_page: Option<i64> = f
        .current_page
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse().ok());
    let rating = quarters_from_input(f.rating.as_deref().unwrap_or("0"));
    let review = f.review.clone().filter(|s| !s.trim().is_empty());
    let visibility = if f.review_visibility == "family" { "family" } else { "private" };
    let tags: Vec<String> = f
        .tags
        .clone()
        .unwrap_or_default()
        .split(',')
        .map(|s| s.to_string())
        .collect();

    match update_user_book(
        &state.pool,
        &user.id,
        &id,
        &status,
        progress_pct,
        current_page,
        rating,
        review.as_deref(),
        visibility,
        tags,
    )
    .await
    {
        Ok(_) => Redirect::to(&format!("/books/{}", id)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

pub async fn delete(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<String>,
) -> Response {
    let _ = remove_user_book(&state.pool, &user.id, &id).await;
    Redirect::to("/").into_response()
}

pub async fn lookup_api(
    State(state): State<AppState>,
    Path(isbn): Path<String>,
) -> Response {
    let isbn = normalize_isbn(&isbn);
    match fetch_openlibrary(&state.http, &isbn).await {
        Ok(m) => {
            let v = serde_json::json!({
                "isbn": m.isbn,
                "title": m.title,
                "authors": m.authors,
                "publisher": m.publisher,
                "published_date": m.published_date,
                "pages": m.pages,
                "cover_url": m.cover_url,
            });
            (StatusCode::OK, axum::Json(v)).into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, e).into_response(),
    }
}

fn quarters_from_input(s: &str) -> i64 {
    let f = s.trim().parse::<f64>().unwrap_or(0.0).clamp(0.0, 5.0);
    (f * 4.0).round() as i64
}

fn urlencoding(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

pub async fn goals(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Query(q): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let year = current_year();
    let target = get_goal(&state.pool, &user.id, year).await.unwrap_or(0);
    let read = count_read_this_year(&state.pool, &user.id, year).await.unwrap_or(0);
    TemplateResponse::into_response(GoalTemplate {
        user_name: user.name,
        year,
        target,
        read,
        message: q.get("msg").cloned(),
    })
}

pub async fn set_goal_handler(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Form(f): Form<GoalForm>,
) -> Response {
    let year = current_year();
    let target: i64 = f.target_books.trim().parse().unwrap_or(0).max(0);
    let _ = set_goal(&state.pool, &user.id, year, target).await;
    Redirect::to("/goals?msg=Objectif+mis+%C3%A0+jour").into_response()
}
