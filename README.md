# MyBooks

Gestion de bibliothèque personnelle et familiale (Rust + Axum + SQLite), déployée en Docker behind Caddy.

## Stack
- Rust (Axum, sqlx/SQLite, Askama)
- Docker Compose (app + Caddy reverse proxy / TLS)
- Hébergement : ociub via SSH, domaine mybooks.laurans.org

## Développement local
```sh
cp .env.example .env
cargo run
# http://localhost:3000
```

## Production (ociub)
```sh
ssh ociub
cd ~/mybooks
git pull
docker compose up -d --build
```

## Roadmap MVP
- [ ] Auth Google (allowlist emails famille)
- [ ] CRUD livres + lookup ISBN (OpenLibrary)
- [ ] Scan code-barres (caméra navigateur)
- [ ] Statuts/progression/notes (rating quart d'étoile)
- [ ] Objectif annuel de lecture
- [ ] Recherche FTS5
