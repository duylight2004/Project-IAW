#!/bin/sh
set -e

# Mỗi container = 1 người chơi/team. DB nằm trên tmpfs (/tmp/db), tạo mới mỗi lần start.
DB_DIR=/tmp/db
mkdir -p "$DB_DIR"

# init schema + seed do binary tự lo (idempotent). Xoá DB cũ để state sạch mỗi lần start.
rm -f "$DB_DIR/antiqua.db" "$DB_DIR/antiqua.db-wal" "$DB_DIR/antiqua.db-shm" 2>/dev/null || true

exec /app/antiqua-library
