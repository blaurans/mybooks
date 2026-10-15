use reqwest::Client;
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
    pub status: String,
    pub progress_pct: i64,
    pub current_page: Option<i64>,
    pub rating: i64,
    pub review: Option<String>,
    pub review_visibility: String,
    pub tags: Vec<String>,
}

pub async fn rebuild_fts(pool: &sqlx::SqlitePool) {
    let _ = sqlx::query("INSERT INTO books_fts(books_fts) VALUES('rebuild')")
        .execute(pool)
        .await;
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
        Ok(_) => {
            rebuild_fts(pool).await;
            Ok(id)
        }
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
    tag: &str,
) -> Result<Vec<BookCard>, String> {
    // Try FTS5 first; fall back to LIKE on invalid query or empty results.
    match list_with_fts(pool, user_id, q, status, tag).await {
        Ok(rows) => Ok(rows),
        Err(_) => list_with_like(pool, user_id, q, status, tag).await,
    }
}

async fn list_with_like(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    q: &str,
    status: &str,
    tag: &str,
) -> Result<Vec<BookCard>, String> {
    let like = format!("%{}%", q);
    let taglike = format!("%{}%", tag);
    let rows = sqlx::query(
        "SELECT b.id, b.title, b.authors, b.cover_url, ub.status, ub.rating, ub.progress_pct
         FROM user_books ub JOIN books b ON b.id = ub.book_id
         WHERE ub.user_id = ?1
           AND (?2 = '' OR b.title LIKE ?3 OR b.authors LIKE ?3 OR COALESCE(ub.review,'') LIKE ?3)
           AND (?4 = '' OR ub.status = ?4)
           AND (?5 = '' OR EXISTS (
                SELECT 1 FROM user_book_tags t
                WHERE t.user_id = ub.user_id AND t.book_id = ub.book_id AND t.tag LIKE ?6))
         ORDER BY ub.updated_at DESC",
    )
    .bind(user_id)
    .bind(q)
    .bind(&like)
    .bind(status)
    .bind(tag)
    .bind(&taglike)
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

async fn list_with_fts(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    q: &str,
    status: &str,
    tag: &str,
) -> Result<Vec<BookCard>, String> {
    let taglike = format!("%{}%", tag);
    if q.trim().is_empty() {
        return list_with_like(pool, user_id, q, status, tag).await;
    }
    let rows = sqlx::query(
        "SELECT b.id, b.title, b.authors, b.cover_url, ub.status, ub.rating, ub.progress_pct
         FROM user_books ub JOIN books b ON b.id = ub.book_id
         WHERE ub.user_id = ?1
           AND b.rowid IN (SELECT rowid FROM books_fts WHERE books_fts MATCH ?2)
           AND (?3 = '' OR ub.status = ?3)
           AND (?4 = '' OR EXISTS (
                SELECT 1 FROM user_book_tags t
                WHERE t.user_id = ub.user_id AND t.book_id = ub.book_id AND t.tag LIKE ?5))
         ORDER BY ub.updated_at DESC",
    )
    .bind(user_id)
    .bind(q)
    .bind(status)
    .bind(tag)
    .bind(&taglike)
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

pub fn current_year() -> i32 {
    chrono::Utc::now().format("%Y").to_string().parse().unwrap_or(2026)
}

pub async fn get_goal(pool: &sqlx::SqlitePool, user_id: &str, year: i32) -> Result<i64, String> {
    let row = sqlx::query("SELECT target_books FROM reading_goals WHERE user_id=?1 AND year=?2")
        .bind(user_id)
        .bind(year)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(row.map(|r| r.get::<i64, _>("target_books")).unwrap_or(0))
}

pub async fn set_goal(pool: &sqlx::SqlitePool, user_id: &str, year: i32, target: i64) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO reading_goals (user_id, year, target_books) VALUES (?1, ?2, ?3)
         ON CONFLICT(user_id, year) DO UPDATE SET target_books = excluded.target_books",
    )
    .bind(user_id)
    .bind(year)
    .bind(target)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn count_read_this_year(pool: &sqlx::SqlitePool, user_id: &str, year: i32) -> Result<i64, String> {
    let y = year.to_string();
    let prefix = format!("{}%", y);
    let row = sqlx::query(
        "SELECT COUNT(*) AS c FROM user_books
         WHERE user_id = ?1 AND status = 'read'
           AND COALESCE(finished_at, created_at) LIKE ?2",
    )
    .bind(user_id)
    .bind(&prefix)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(row.get::<i64, _>("c"))
}

pub struct AdminUserRow {
    pub id: String,
    pub email: String,
    pub name: String,
    pub role: String,
    pub book_count: i64,
}

pub async fn list_users_for_admin(pool: &sqlx::SqlitePool) -> Result<Vec<AdminUserRow>, String> {
    let rows = sqlx::query(
        "SELECT u.id, u.email, u.name, u.role,
                (SELECT COUNT(*) FROM user_books ub WHERE ub.user_id = u.id) AS book_count
         FROM users u
         ORDER BY u.created_at DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        out.push(AdminUserRow {
            id: r.get("id"),
            email: r.get("email"),
            name: r.get("name"),
            role: r.get("role"),
            book_count: r.get("book_count"),
        });
    }
    Ok(out)
}

pub async fn set_user_role(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    role: &str,
) -> Result<bool, String> {
    if role != "admin" && role != "member" {
        return Err("rôle invalide".to_string());
    }
    if role == "member" {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role = 'admin'")
            .fetch_one(pool)
            .await
            .map_err(|e| e.to_string())?;
        if count <= 1 {
            return Ok(false);
        }
    }
    let res = sqlx::query("UPDATE users SET role = ?1, updated_at = datetime('now') WHERE id = ?2")
        .bind(role)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(res.rows_affected() > 0)
}

pub async fn delete_user(pool: &sqlx::SqlitePool, user_id: &str) -> Result<bool, String> {
    // Prevent removing the last admin (avoid lockout).
    let target_role: String = sqlx::query_scalar("SELECT role FROM users WHERE id = ?1")
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    if target_role.is_empty() {
        return Ok(false);
    }
    if target_role == "admin" {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role = 'admin'")
            .fetch_one(pool)
            .await
            .map_err(|e| e.to_string())?;
        if count <= 1 {
            return Ok(false);
        }
    }
    sqlx::query("DELETE FROM users WHERE id = ?1")
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(true)
}
