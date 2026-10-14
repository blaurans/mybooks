# Handoff — MyBooks (Rust / Axum / SQLite)

Date: 2026-10-14
Repo: https://github.com/blaurans/mybooks
Branch: main
State: Jalon 4 terminé (recherche & filtres)

## Décisions prises (ne pas perdre)
- Expérience: app web, Rust Axum + sqlx/SQLite + Askama/HTMX.
- Auth: Google OAuth désormais, allow-all si `ALLOWED_EMAILS` vide (mode ouvert).
- Bibliothèque par utilisateur (multi-user), statuts, progression, note/avis 0,25/5, tags libres.
- Une ligne user_books par utilisateur/livre; editions par ISBN.
- Couvertures via URL externe ; scans via caméra navigateur.
- Visibilité avis: private/family.
- DB: SQLite volume Docker, WAL, FK, migrations sqlx.
- Recherche: FTS5 sur title/authors/... avec triggers + rebuild; fallback LIKE.
- Déploiement: build sur ociub, Caddy TLS, domaine mybooks.laurans.org.
- Pas d'backup automatique en v1 (décision reportée, à faire au jalon 6).
- Scan code-barres côté client (BarcodeDetector + fallback ZXing).

## Fichiers à rouvrir en priorité
- src/main.rs: bootstrap, migrations, rebuild FTS.
- src/auth.rs: Google OAuth, sessions, allowlist.
- src/middleware.rs: auth middleware.
- src/handlers/books.rs: routes CRUD, scan, search/filter.
- src/books_service.rs: metadata (OpenLibrary), DB helpers, FTS rebuild.
- src/handlers/mod.rs: routing.
- templates/library.html, book_detail.html, book_new.html, scan.html.
- migrations/0001_init.sql, 0002_sessions.sql, 0003_fts.sql.
- docker-compose.yml, Caddyfile, Dockerfile.
- handoff.md (ce fichier).

## Jalon 1 — Auth Google (fait, vérifié)
- OAuth2 PKCE, allowlist, sessions HTTP-only 30j, middleware.
- Vérifié: redirection / -> /auth/login, /auth/login -> Google, /health ok.
- Ouvert: client ID/secret réels à configurer en prod (voir .env.example); mode allow-all à restreindre si instance publique.

## Jalon 2 — CRUD livres + ISBN (fait, vérifié)
- Ajout ISBN via OpenLibrary + ajout manuel, fiche livre, statut/progression/notes/tags.
- Vérifié: ajout manuel, ajout ISBN (Fantastic Mr. Fox), lecture fiche, update (status, note, tags).

## Jalon 3 — Scan code-barres (fait, vérifié)
- Page /scan, caméra, BarcodeDetector + ZXing fallback, auto-ajout.
- Vérifié: page 200, build OK. Tests caméra réels non effectués (pas d'UI automatisée).

## Jalon 4 — Recherche & filtres (fait, vérifié)
- FTS5 migration with triggers/backfill/rebuild, recherche q, filtres statut/tag.
- Vérifié: recherche "Fox" retrouve Fantastic Mr. Fox; filtre tag sci-fi retrouve Test Book.

## Problèmes ouverts / non récupérables
- Avertissements de compilation (imports/dead code) non bloquants; à nettoyer au jalon 6.
- ALLOWED_EMAILS vide = inscription ouverte; à restreindre pour prod publique.
- Tokens Google envoyés en clair dans l'historique chat: NON stockés dans le repo (ne pas committer de .env).
- Backup DB automatisé non prévu (risque de perte).
- Objectif de lecture annuel + dashboard: non implémenté (jalon 5).
- Durcissement (jalon 6) non fait.

## Prochaine action
- Jalon 5: objectif annuel + dashboard (table reading_goals, endpoints, template goals.html, affichage progression).
- Jalon 6: durcissement (warnings, allowlist docs, backup SQLite cron/rsync), puis push.
Marque: les secrets et états de sessions sont volatils et ne figurent pas dans ce dépôt.
