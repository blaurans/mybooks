-- Keep books_fts in sync with books.
CREATE TRIGGER IF NOT EXISTS books_fts_ai AFTER INSERT ON books BEGIN
    INSERT INTO books_fts(rowid, title, authors, description)
    VALUES (new.rowid, new.title, new.authors, new.description);
END;

CREATE TRIGGER IF NOT EXISTS books_fts_ad AFTER DELETE ON books BEGIN
    DELETE FROM books_fts WHERE rowid = old.rowid;
END;

CREATE TRIGGER IF NOT EXISTS books_fts_au AFTER UPDATE ON books BEGIN
    DELETE FROM books_fts WHERE rowid = old.rowid;
    INSERT INTO books_fts(rowid, title, authors, description)
    VALUES (new.rowid, new.title, new.authors, new.description);
END;

-- Backfill any rows missing from the index (idempotent).
INSERT INTO books_fts (rowid, title, authors, description)
SELECT b.rowid, b.title, b.authors, b.description
FROM books AS b
WHERE b.rowid NOT IN (SELECT rowid FROM books_fts);
