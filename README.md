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

## Production (ociub, arm64)
`ociub` est en `aarch64` et son builder Docker ne peut pas joindre crates.io. Utiliser la compose serveur prévue pour cela (`docker-compose.server.yml`) et une image arm64 construite localement avec buildx :

```sh
# sur la machine de build (avec QEMU/buildx)
docker buildx build --platform linux/arm64 --load -t mybooks:arm64 .
docker save mybooks:arm64 | gzip -1 | ssh ociub 'gunzip | docker load'

# sur ociub
cd /app/mybooks
git pull
cp .env.example .env  # puis remplir les valeurs
docker compose -f docker-compose.server.yml up -d
```

Le Caddy existant de `ociub` est partagé (`/app/guacamole/Caddyfile`). Y ajouter le bloc de `deploy/caddy-mybooks-block.conf` puis recharger :

```sh
# exemple idempotent
grep -q mybooks.laurans.org /app/guacamole/Caddyfile || \
  cat deploy/caddy-mybooks-block.conf >> /app/guacamole/Caddyfile
docker exec guacamole-caddy-1 caddy validate --config /etc/caddy/Caddyfile
docker exec guacamole-caddy-1 caddy reload --config /etc/caddy/Caddyfile
```

## Roadmap MVP
- [x] Auth Google (allowlist emails famille)
- [x] CRUD livres + lookup ISBN (OpenLibrary)
- [x] Scan code-barres (caméra navigateur)
- [x] Statuts/progression/notes (rating quart d'étoile)
- [x] Objectif annuel de lecture
- [x] Recherche FTS5
- [x] Backup SQLite automatisé
