#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

ensure_node20() {
  # Load nvm in non-interactive shells if available.
  export NVM_DIR="${NVM_DIR:-$HOME/.nvm}"
  if [[ -s "$NVM_DIR/nvm.sh" ]]; then
    # shellcheck source=/dev/null
    . "$NVM_DIR/nvm.sh"
    nvm use 20 >/dev/null || true
  fi

  if ! command -v node >/dev/null 2>&1; then
    echo "Error: node is not installed. Install Node.js 20.19+."
    exit 1
  fi

  local major
  major="$(node -p "process.versions.node.split('.')[0]")"
  if [[ "$major" -lt 20 ]]; then
    echo "Error: Node.js $(node -v) detected. Vite requires Node.js 20.19+."
    echo "Run: nvm install 20 && nvm use 20"
    exit 1
  fi
}

cd "$ROOT_DIR"

echo "[1/4] Starting PostgreSQL (docker compose)..."
docker compose up -d postgres

ensure_node20
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
