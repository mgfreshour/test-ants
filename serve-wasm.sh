#!/usr/bin/env bash
set -euo pipefail

PORT="${1:-8080}"

if [ ! -f web/out/colony.js ]; then
    echo "Error: web/out/colony.js not found. Run ./build-wasm.sh first."
    exit 1
fi

echo "Serving at http://localhost:${PORT}"
uv run python -m http.server "${PORT}" --directory web
