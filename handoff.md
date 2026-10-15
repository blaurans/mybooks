# Handoff — MyBooks (Rust / Axum / SQLite)

Date: 2026-10-15
Repo: https://github.com/blaurans/mybooks
Branch: main
État: App en ligne sur https://mybooks.laurans.org (working: / health, /auth/login)

## Décisions
- App web Rust Axum + sqlx/SQLite + Askama/HTMX.
- Auth Google OAuth (PKCE), sessions cookie 30j, allowlist si `ALLOWED_EMAILS` non vide (prod actuellement vide = allow-all).
- Bibliothèque par utilisateur, statuts, progression, note/avis 0,25/5, tags, visibilité private/family.
- Découpage utilisateur + tables books globaux; dédup ISBN.
- Recherche FTS5 + fallback LIKE.
- Objectif annuel par utilisateur.
- Mode sombre/clair + persistance localStorage et préférence système.
- Backup SQLite quotidien rétention 14j (conteneur keinos/sqlite3).
- Hébergement ociub arm64, Caddy partagé guacamole.

## Fichiers clés
- src/main.rs, src/auth.rs, src/middleware.rs, src/books_service.rs, src/handlers/*
- templates/*.html
- migrations/0001_init.sql, 0002_sessions.sql, 0003_fts.sql
- docker-compose.server.yml (compose ociub)
- Dockerfile (build multi-arch), Dockerfile.binary (image runtime-only)
- deploy/caddy-mybooks-block.conf
- scripts/backup.sh
- README.md, handoff.md

## Problèmes / contraintes trouvées
- Safari/iPad ne fournit pas toujours `BarcodeDetector` : fallback ZXing maintenant servi localement (`static/vendor/zxing-browser.js`).
- L’entrée `/scan` était peu visible : lien ajouté depuis bibliothèque et page ajout.
- Serveur ociub est `aarch64`; compiler localement en x86_64 ne marche pas.
- Builder Docker du serveur n’a pas accès à `crates.io`/debian lors des builds.
- Solution actuelle : build local `docker buildx --platform linux/arm64`, puis `docker save | ssh ociub docker load`.
- Image locale doît toujours être rebuild/load quand le code change (pas de registry).
- Runtime Debian a besoin de `libssl3` (ajouté au Dockerfile).
- Caddy est partagé : bloc Caddy ajouté dans `/app/guacamole/Caddyfile`, backup créé.
- Compose command string doit être passée en argv pour éviter split (`docker-compose.server.yml`).
- Backup volume nécessite `user: "0:0"`.

## Fait / vérifié
- App up sur ociub : mybooks-app-1 (mybooks:arm64), mybooks-backup-1 (keinos/sqlite3).
- Caddy valide et rechargé ; test local et public:
  - https://mybooks.laurans.org/health -> "ok" HTTP 200
  - https://mybooks.laurans.org/ -> 303 vers /auth/login
  - /auth/login -> 307 vers Google
- OpenLibrary accessible depuis le réseau container (HTTP 200).
- Backup initial créé dans `mybooks_mybooks_backups`.
- Mode sombre : page rendue avec CSS variables + toggle (test local HTML, image ARM64 déployée).
- Scan fallback iPad : vendor ZXing servi en local (HTTP 200, 441 Ko).

## Secrets / ne pas committer
- `.env` sur ociub contient GOOGLE_CLIENT_ID/SECRET et Allowed/Admin (actuellement vide).
- Ne jamais les mettre dans le repo Git. Le handoff ne contient aucun secret.

## Points ouverts
- Restreindre `ALLOWED_EMAILS` et `ADMIN_EMAILS` avant exposition familiale large.
- Scanner en conditions mobile HTTPS non testé (page ok).
- Cache image/registry non configuré : changement code = rebuild arm64 + load.
- Logs Caddy du bloc dans `/data/mybooks-access.log` (dans volume caddy partagé).

## Prochaine action
- Après changement Rust : rebuild + load image, `git pull`, `docker compose -f docker-compose.server.yml up -d`.
- Tester OAuth réel depuis le navigateur avec un compte autorisé, puis compléter ajout livre ISBN/manuel/page objectif.
- Si public : remplir ALLOWED_EMAILS avec les comptes famille.
