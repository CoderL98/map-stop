#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export $(grep -v '^#' .env 2>/dev/null | xargs -r) || true
mkdir -p data
cd services/api
exec cargo run
