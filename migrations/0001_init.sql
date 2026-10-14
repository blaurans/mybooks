CREATE TABLE IF NOT EXISTS users (
  id TEXT PRIMARY KEY,
  google_sub TEXT NOT NULL UNIQUE,
  email TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  avatar_url TEXT,
  role TEXT NOT NULL DEFAULT 'member' CHECK(role IN ('admin','member')),
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS books (
  id TEXT PRIMARY KEY,
  isbn TEXT UNIQUE,
  title TEXT NOT NULL,
  authors TEXT NOT NULL,
  cover_url TEXT,
  publisher TEXT,
  published_date TEXT,
  pages INTEGER,
  language TEXT,
  description TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS user_books (
  user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  book_id TEXT NOT NULL REFERENCES books(id) ON DELETE CASCADE,
  status TEXT NOT NULL DEFAULT 'to_read' CHECK(status IN ('to_read','reading','read','dropped')),
  progress_pct INTEGER NOT NULL DEFAULT 0,
  current_page INTEGER,
  rating INTEGER NOT NULL DEFAULT 0,
  review TEXT,
  review_visibility TEXT NOT NULL DEFAULT 'private' CHECK(review_visibility IN ('private','family')),
  started_at TEXT,
  finished_at TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  PRIMARY KEY(user_id, book_id)
);

CREATE TABLE IF NOT EXISTS reading_goals (
  user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  year INTEGER NOT NULL,
  target_books INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY(user_id, year)
);

CREATE TABLE IF NOT EXISTS user_book_tags (
  user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  book_id TEXT NOT NULL REFERENCES books(id) ON DELETE CASCADE,
  tag TEXT NOT NULL,
  PRIMARY KEY(user_id, book_id, tag)
);

CREATE VIRTUAL TABLE IF NOT EXISTS books_fts USING fts5(title, authors, description, content='books', content_rowid='rowid');
