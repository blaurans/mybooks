# Handoff — MyBooks (Rust / Axum / SQLite)

Date: 2026-10-14
Repo: https://github.com/blaurans/mybooks
Branch: main
État: Jalons 1–6 terminés (MVP fonctionnel + durcissement)

## Décisions prises (à conserver)
- Expérience: app web Rust Axum + sqlx/SQLite + Askama/HTMX.
- Auth: Google OAuth, allow-all si ALLOWED_EMAILS vide (mode ouvert, à restreindre avant prod publique).
- Bibliothèque par utilisateur, statuts, progression, note/avis 0,25/5, tags libres.
- Edition par ISBN, métadonnées OpenLibrary.
- Visibilité avis: private/family.
- DB SQLite volume Docker, WAL/FK, migrations sqlx.
- Recherche: FTS5 + triggers + rebuild; fallback LIKE.
- Déploiement: build sur ociub, Caddy TLS, domaine mybooks.laurans.org.
- Backup: conteneur sidecar quotidien + script manuel, rétention 14 j.

## Fichiers clés à rouvrir
- src/main.rs, src/auth.rs, src/middleware.rs, src/handlers/books.rs, src/handlers/mod.rs, src/books_service.rs.
- src/config.rs, src/db.rs.
- templates/*.html.
- migrations/0001_init.sql, 0002_sessions.sql, 0003_fts.sql.
- docker-compose.yml, Dockerfile, Caddyfile, scripts/backup.sh, handoff.md.

## Jalons
1. Auth Google — fait, vérifié.
2. CRUD livres + ISBN — fait, vérifié.
3. Scan code-barres — fait, vérifié (page).
4. Recherche & filtres FTS5 — fait, vérifié (Fox, tag).
5. Objectif annuel + page goals/dashboard — fait, vérifié (lecture comptabilisée).
6. Durcissement — fait: build sans warnings, backup sidecar/script, .env example.

## Vérifié
- cargo build sans warnings
- Ajout manuel/ISBN, fiche, mise à jour, scan page, recherche, objectif annuel
- Push GitHub main effectué à chaque jalon

## Points ouverts
- Allowlist famille à configurer en prod (ALLOWED_EMAILS)
- Secrets Google fournis dans la conversation : ne jamais committer; les mettre uniquement dans .env serveur.
- Tests caméra réels en conditions HTTPS mobile non effectués.
- Reste possible: page scan mobile, import/export CSV, stats avancées, PWA/offline.

## Prochaine action
- Déployer sur ociub selon README, configurer DNS A mybooks.laurans.org et les secrets Google Cloud Console.
- Restreindre ALLOWED_EMAILS (famille) puis rebuild.
Marque: sessions, backups réels et secrets ne figurent pas dans ce dépôt.
