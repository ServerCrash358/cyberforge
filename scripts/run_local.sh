#!/usr/bin/env bash
# Runs the whole app on this laptop: the Rust API and the website together on :8000, a licence
# signed with your key, and demo data. Ctrl+C stops it. Data lives in .local/ (gitignored) and
# survives restarts.
#   scripts/run_local.sh            start with everything you entered before; demo data only the
#                                   first time, when the database is new
#   scripts/run_local.sh --fresh    DELETE every report and challenge, then load fresh demo data
#   scripts/run_local.sh --empty    DELETE every report and challenge and load NO demo data (for judging)
# Your own reports are never deleted unless you pass --fresh.
# Other machines on the same Wi-Fi can reach it: the banner at the end prints the address to open
# and to curl. SCAM_MAP_HOST=127.0.0.1 scripts/run_local.sh keeps it on this laptop only.
# Owner: A. Filled in by: A6, U1 (Rust API serving the React website).
set -euo pipefail
cd "$(dirname "$0")/.."

PORT=8000
LOCAL=.local
KEY="$HOME/.scam-map/license.key"
mkdir -p "$LOCAL"

if [ ! -f "$KEY" ]; then
  echo "No licence key at $KEY. Make one first:" >&2
  echo "  uv run --project judge python scripts/license_keygen.py" >&2
  exit 1
fi

# cargo installs to ~/.cargo/bin; a shell opened before Rust was installed may not have it yet.
export PATH="$HOME/.cargo/bin:$PATH"

HOST=${SCAM_MAP_HOST:-0.0.0.0}  # 0.0.0.0: reachable from the Wi-Fi; 127.0.0.1: this laptop only

# The address other machines on the Wi-Fi use: the one this laptop would send from.
lan_ip() {
  uv run --project judge python -c "import socket; s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM); s.connect(('10.255.255.255', 1)); print(s.getsockname()[0])" 2>/dev/null || echo 127.0.0.1
}

echo "== Building the website and the API"
(cd ui && npm ci --no-fund --no-audit --loglevel=error && npm run build --silent)
(cd api-rs && cargo build --release --locked --quiet)
(cd judge && uv sync --frozen --quiet)

# One HMAC key for this laptop, kept so stored reports stay reachable between runs.
if [ ! -f "$LOCAL/hmac.key" ]; then
  uv run --project judge python -c "import secrets; print(secrets.token_hex(32))" > "$LOCAL/hmac.key"
fi

case "${1:-}" in
  "") SEED=no ;;
  --fresh) SEED=yes ;;
  --empty) SEED=empty ;;
  *) echo "usage: $0 [--fresh | --empty]" >&2; exit 2 ;;
esac
# A brand-new database gets demo data so there is something to look at, unless --empty was asked.
[ -f "$LOCAL/scam_map.sqlite" ] || [ "$SEED" = empty ] || SEED=yes

echo "== Starting the API and website on port $PORT (listening on $HOST)"
(cd api-rs && SCAM_MAP_DB_PATH="../$LOCAL/scam_map.sqlite" SCAM_MAP_HMAC_KEY="$(cat "../$LOCAL/hmac.key")" \
  SCAM_MAP_HOST="$HOST" SCAM_MAP_PORT="$PORT" SCAM_MAP_WEB_DIR=../ui/dist \
  exec ./target/release/scam-map-api) &
API_PID=$!
trap 'kill $API_PID 2>/dev/null || true; wait $API_PID 2>/dev/null || true; echo stopped' EXIT INT TERM
for _ in $(seq 1 60); do
  curl -sf "http://127.0.0.1:$PORT/health" >/dev/null && break
  sleep 0.5
done
curl -sf "http://127.0.0.1:$PORT/health" >/dev/null || { echo "API did not start" >&2; exit 1; }

echo "== Loading a 30-day licence signed with your key"
uv run --project judge python scripts/license_sign.py --licensee "Local demo" --expires-in 30d \
  --out "$LOCAL/licence-local.json" >/dev/null
curl -s -X POST -H 'content-type: application/json' --data-binary "@$LOCAL/licence-local.json" \
  "http://127.0.0.1:$PORT/license"
echo

if [ "$SEED" = empty ]; then
  echo "== Clearing the database; no demo data (the licence is kept)"
  curl -s -X POST "http://127.0.0.1:$PORT/admin/reset"
  echo
elif [ "$SEED" = yes ]; then
  echo "== Loading demo data (this clears the local database)"
  (cd judge && uv run python seed_demo.py --target "http://127.0.0.1:$PORT")
else
  echo "== Keeping your existing data (--fresh clears it and loads demo data; --empty clears it and loads nothing)"
fi

echo
if [ "$HOST" = "0.0.0.0" ]; then
  LAN=$(lan_ip)
  echo "== On this Wi-Fi (judges, phones): website and API http://$LAN:$PORT"
  echo "   try from another machine:  curl http://$LAN:$PORT/health"
  echo "   if it cannot connect, allow port $PORT in Windows Firewall (infra/README.md)"
fi
echo "== On this laptop: http://127.0.0.1:$PORT   (Ctrl+C stops everything)"
wait "$API_PID"
