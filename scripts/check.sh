#!/usr/bin/env bash
# Local zero-warning gate.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "== Repo scans"
if git ls-files | grep -E '\.key$|\.env$' ; then echo "secret file tracked"; exit 1; fi
if git grep -n "BEGIN PRIVATE KEY" -- . ':!docs' ':!scripts/check.sh' ':!scripts/license_keygen.py' ; then echo "private key in repo"; exit 1; fi

echo "== API (Rust)"
export PATH="$HOME/.cargo/bin:$PATH"
(cd api-rs
  cargo fmt --check
  cargo clippy --locked --all-targets -- -D warnings
  cargo test --locked --quiet
  cargo audit --deny warnings)

echo "== Judge (in-process, held-out seeds)"
(cd judge
  uv sync --frozen
  uv run ruff check .
  uv run mypy --strict .
  uv run pytest -q -W error
  uv run python contract.py --target local
  uv run python runner.py --seeds holdout --target local --fail-under-thresholds
  uv run python -m scanners.all --target local
  uv run pip-audit)

echo "== Web (Streamlit)"
(cd web
  uv sync --frozen
  uv run ruff check .
  uv run ruff format --check .
  uv run mypy --strict .
  uv run pytest -q -W error
  uv run pip-audit)

echo "check passed on $(date -u +%F)"
