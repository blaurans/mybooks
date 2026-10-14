use crate::handlers::AppState;
use reqwest::Client;
use serde::Deserialize;
use sqlx::Row;

pub struct BookMeta {
    pub isbn: Option<String>,
    pub title: String,
    pub authors: String,
    pub publisher: Option<String>,
    pub published_date: Option<String>,
    pub pages: Option<i64>,
    pub cover_url: Option<String>,
}

#[derive(Clone)]
pub struct BookCard {
    pub id: String,
    pub title: String,
    pub authors: String,
    pub cover_url: Option<String>,
    pub status: String,
    pub rating: i64,
    pub progress_pct: i64,
}

#[derive(Clone)]
pub struct UserBookDetail {
    pub book_id: String,
    pub isbn: Option<String>,
    pub title: String,
    pub authors: String,
    pub publisher: Option<String>,
    pub published_date: Option<String>,
    pub pages: Option<i64>,
    pub cover_url: Option<String>,
    pub description: Option<String>,
    pub status: String,
    pub progress_pct: i64,
    pub current_page: Option<i64>,
    pub rating: i64,
    pub review: Option<String>,
    pub review_visibility: String,
    pub tags: Vec<String>,
}

pub fn normalize_isbn(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_uppercase()
}

pub async fn fetch_openlibrary(client: &Client, isbn: &str) -> Result<BookMeta, String> {
    let isbn = normalize_isbn(isbn);
    if isbn.is_empty() {
        return Err("ISBN vide".into());
    }
    let url = format!(
        "https://openlibrary.org/api/books?bibkeys=ISBN:{}&format=json&jscmd=data",
        isbn
    );
    let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("openlibrary http {}", resp.status()));
    }
    let value: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let key = format!("ISBN:{}", isbn);
    let data = value.get(&key).cloned().ok_or_else(|| "livre introuvable".to_string())?;
    if !data.is_object() {
        return Err("livre introuvable".into());
    }
    let title = data
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Sans titre")
        .to_string();
    let authors = data
        .get("authors")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr
                .iter()
                .filter_map(|a| a.get("name").and_then(|n| n.as_str()))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Auteur inconnu".to_string());
    let publishers = data
        .get("publishers")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .map(String::from);
    let published_date = data
        .get("publish_date")
        .and_then(|v| v.as_str())
        .map(String::from);
    let pages = data
        .get("number_of_pages")
        .and_then(|v| v.as_i64());
    let cover_url = data
        .get("cover")
        .and_then(|c| c.get("large").or_else(|| c.get("medium")).or_else(|| c.get("small")))
        .and_then(|u| u.as_str())
        .map(String::from);

    Ok(BookMeta {
        isbn: Some(isbn),
        title,
        authors,
        publisher: publishers,
        published_date,
        pages,
        cover_url,
    })
}

// ---------- DB helpers ----------

pub async fn find_book_by_isbn(pool: &sqlx::SqlitePool, isbn: &str) -> Result<Option<String>, String> {
    let row = sqlx::query("SELECT id FROM books WHERE isbn = ?1")
        .bind(isbn)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(row.map(|r| r.get::<String, _>("id")))
}

pub async fn create_book(pool: &sqlx::SqlitePool, m: &BookMeta) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let authors = m.authors.clone();
    let isbn = m.isbn.clone();
    let res = sqlx::query(
        "INSERT INTO books (id, isbn, title, authors, cover_url, publisher, published_date, pages)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    )
    .bind(&id)
    .bind(&isbn)
    .bind(&m.title)
    .bind(&authors)
    .bind(&m.cover_url)
    .bind(&m.publisher)
    .bind(&m.published_date)
    .bind(&m.pages)
    .execute(pool)
    .await;
    match res {
        Ok(_) => Ok(id),
        Err(e) => {
            // possibly unique constraint on isbn -> fetch existing
            if let Some(isbn) = isbn {
                if let Some(existing) = find_book_by_isbn(pool, &isbn).await? {
                    return Ok(existing);
                }
            }
            Err(e.to_string())
        }
    }
}

pub async fn link_user_book(pool: &sqlx::SqlitePool, user_id: &str, book_id: &str) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO user_books (user_id, book_id, status, progress_pct, rating, review_visibility)
         VALUES (?1, ?2, 'to_read', 0, 0, 'private')
         ON CONFLICT(user_id, book_id) DO NOTHING",
    )
    .bind(user_id)
    .bind(book_id)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn remove_user_book(pool: &sqlx::SqlitePool, user_id: &str, book_id: &str) -> Result<(), String> {
    sqlx::query("DELETE FROM user_books WHERE user_id=?1 AND book_id=?2")
        .bind(user_id)
        .bind(book_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    let _ = sqlx::query("DELETE FROM user_book_tags WHERE user_id=?1 AND book_id=?2")
        .bind(user_id)
        .bind(book_id)
        .execute(pool)
        .await;
    Ok(())
}

pub async fn list_user_books(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    q: &str,
    status: &str,
) -> Result<Vec<BookCard>, String> {
    let like = format!("%{}%", q);
    let rows = sqlx::query(
        "SELECT b.id, b.title, b.authors, b.cover_url, ub.status, ub.rating, ub.progress_pct
         FROM user_books ub JOIN books b ON b.id = ub.book_id
         WHERE ub.user_id = ?1
           AND (?2 = '' OR b.title LIKE ?3 OR b.authors LIKE ?3)
           AND (?4 = '' OR ub.status = ?4)
         ORDER BY ub.updated_at DESC",
    )
    .bind(user_id)
    .bind(q)
    .bind(&like)
    .bind(status)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        out.push(BookCard {
            id: r.get("id"),
            title: r.get("title"),
            authors: r.get("authors"),
            cover_url: r.get("cover_url"),
            status: r.get("status"),
            rating: r.get("rating"),
            progress_pct: r.get("progress_pct"),
        });
    }
    Ok(out)
}

pub async fn get_user_book(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    book_id: &str,
) -> Result<Option<UserBookDetail>, String> {
    let row = sqlx::query(
        "SELECT b.id AS bid, b.isbn, b.title, b.authors, b.publisher, b.published_date, b.pages, b.cover_url,
                ub.status, ub.progress_pct, ub.current_page, ub.rating, ub.review, ub.review_visibility
         FROM books b JOIN user_books ub ON ub.book_id = b.id
         WHERE b.id = ?1 AND ub.user_id = ?2",
    )
    .bind(book_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;
    let row = match row {
        Some(r) => r,
        None => return Ok(None),
    };
    let tags_rows = sqlx::query(
        "SELECT tag FROM user_book_tags WHERE user_id=?1 AND book_id=?2 ORDER BY tag",
    )
    .bind(user_id)
    .bind(book_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    let tags: Vec<String> = tags_rows.iter().map(|r| r.get::<String, _>("tag")).collect();
    Ok(Some(UserBookDetail {
        book_id: row.get("bid"),
        isbn: row.get("isbn"),
        title: row.get("title"),
        authors: row.get("authors"),
        publisher: row.get("publisher"),
        published_date: row.get("published_date"),
        pages: row.get("pages"),
        cover_url: row.get("cover_url"),
        // description not stored; could fetch from books table later if column added
        description: None,
        status: row.get("status"),
        progress_pct: row.get("progress_pct"),
        current_page: row.get("current_page"),
        rating: row.get("rating"),
        review: row.get("review"),
        review_visibility: row.get("review_visibility"),
        tags,
    }))
}

pub async fn update_user_book(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    book_id: &str,
    status: &str,
    progress_pct: i64,
    current_page: Option<i64>,
    rating: i64,
    review: Option<&str>,
    review_visibility: &str,
    tags: Vec<String>,
) -> Result<(), String> {
    let started_sql = if status == "reading" {
        "CASE WHEN started_at IS NULL THEN datetime('now') ELSE started_at END"
    } else {
        "started_at"
    };
    let finished_sql = if status == "read" {
        "COALESCE(finished_at, datetime('now'))"
    } else {
        "finished_at"
    };
    let q = format!(
        "UPDATE user_books SET status=?1, progress_pct=?2, current_page=?3, rating=?4, review=?5, review_visibility=?6,
         started_at={}, finished_at={}, updated_at=datetime('now')
         WHERE user_id=?7 AND book_id=?8",
        started_sql, finished_sql
    );
    sqlx::query(&q)
        .bind(status)
        .bind(progress_pct.clamp(0, 100))
        .bind(current_page)
        .bind(rating.clamp(0, 20))
        .bind(review)
        .bind(review_visibility)
        .bind(user_id)
        .bind(book_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    // replace tags
    sqlx::query("DELETE FROM user_book_tags WHERE user_id=?1 AND book_id=?2")
        .bind(user_id)
        .bind(book_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    for tag in tags {
        let tag = tag.trim().to_lowercase();
        if tag.is_empty() {
            continue;
        }
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO user_book_tags (user_id, book_id, tag) VALUES (?1, ?2, ?3)",
        )
        .bind(user_id)
        .bind(book_id)
        .bind(&tag)
        .execute(pool)
        .await;
    }
    Ok(())
}
