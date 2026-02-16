#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$ROOT_DIR"

echo "[1/4] Starting PostgreSQL (docker compose)..."
docker compose up -d postgres

echo "[2/4] Installing frontend dependencies..."
cd "$ROOT_DIR/frontend"
npm install

echo "[3/4] Building frontend..."
npm run build

cd "$ROOT_DIR/backend"
if [[ ! -f .env ]]; then
  cp .env.example .env
fi

echo "[4/4] Running backend on http://0.0.0.0:8181 ..."
REPO_ROOT="$ROOT_DIR" cargo run
