#!/usr/bin/env bash
set -euo pipefail

DB="${DB:-/data/mybooks.db}"
BACKUP_DIR="${BACKUP_DIR:-/backups}"
RETENTION_DAYS="${RETENTION_DAYS:-14}"
TS="$(date +%Y%m%d-%H%M%S)"

mkdir -p "$BACKUP_DIR"
sqlite3 "$DB" ".backup '$BACKUP_DIR/mybooks-$TS.db'"
find "$BACKUP_DIR" -name 'mybooks-*.db' -mtime +"$RETENTION_DAYS" -delete
echo "backup created: $BACKUP_DIR/mybooks-$TS.db"
