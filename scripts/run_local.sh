#!/usr/bin/env bash
# Runs the whole app on this laptop: API on :8000, website on :8501, a licence signed with your
# key, and demo data. Ctrl+C stops both. Data lives in .local/ (gitignored) and survives restarts.
#   scripts/run_local.sh            start with everything you entered before; demo data only the
#                                   first time, when the database is new
#   scripts/run_local.sh --fresh    DELETE every report and challenge, then load fresh demo data
#   scripts/run_local.sh --empty    DELETE every report and challenge and load NO demo data (for judging)
# Your own reports are never deleted unless you pass --fresh.
# Other machines on the same Wi-Fi can reach both servers: the banner at the end prints the address
# to curl. SCAM_MAP_HOST=127.0.0.1 scripts/run_local.sh keeps them on this laptop only.
# Owner: A. Filled in by: A6.
set -euo pipefail
cd "$(dirname "$0")/.."

API_PORT=8000
WEB_PORT=8501
LOCAL=.local
KEY="$HOME/.scam-map/license.key"
mkdir -p "$LOCAL"

if [ ! -f "$KEY" ]; then
  echo "No licence key at $KEY. Make one first:" >&2
  echo "  uv run --project api python scripts/license_keygen.py" >&2
  exit 1
fi

venv_python() {
  if [ -x "$1/.venv/Scripts/python.exe" ]; then echo "$1/.venv/Scripts/python.exe"; else echo "$1/.venv/bin/python"; fi
}

HOST=${SCAM_MAP_HOST:-0.0.0.0}  # 0.0.0.0: reachable from the Wi-Fi; 127.0.0.1: this laptop only

# The address other machines on the Wi-Fi use: the one this laptop would send from.
lan_ip() {
  uv run --project api python -c "import socket; s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM); s.connect(('10.255.255.255', 1)); print(s.getsockname()[0])" 2>/dev/null || echo 127.0.0.1
}

echo "== Installing locked dependencies"
(cd api && uv sync --frozen --quiet)
(cd web && uv sync --frozen --quiet)
(cd judge && uv sync --frozen --quiet)

# One HMAC key for this laptop, kept so stored reports stay reachable between runs.
if [ ! -f "$LOCAL/hmac.key" ]; then
  uv run --project api python -c "import secrets; print(secrets.token_hex(32))" > "$LOCAL/hmac.key"
fi

case "${1:-}" in
  "") SEED=no ;;
  --fresh) SEED=yes ;;
  --empty) SEED=empty ;;
  *) echo "usage: $0 [--fresh | --empty]" >&2; exit 2 ;;
esac
# A brand-new database gets demo data so there is something to look at, unless --empty was asked.
[ -f "$LOCAL/scam_map.sqlite" ] || [ "$SEED" = empty ] || SEED=yes

API_PID=""
cleanup() {
  if [ -n "$API_PID" ]; then kill "$API_PID" 2>/dev/null || true; wait "$API_PID" 2>/dev/null || true; fi
  echo "stopped"
}
trap cleanup EXIT INT TERM

echo "== Starting the API on port $API_PORT (listening on $HOST)"
(cd api && SCAM_MAP_DB_PATH="../$LOCAL/scam_map.sqlite" SCAM_MAP_HMAC_KEY="$(cat "../$LOCAL/hmac.key")" \
  exec "$(venv_python .)" -m uvicorn app.main:app --host "$HOST" --port "$API_PORT" \
  --no-access-log --log-level warning) &
API_PID=$!
for _ in $(seq 1 60); do
  curl -sf "http://127.0.0.1:$API_PORT/health" >/dev/null && break
  sleep 0.5
done
curl -sf "http://127.0.0.1:$API_PORT/health" >/dev/null || { echo "API did not start" >&2; exit 1; }

echo "== Loading a 30-day licence signed with your key"
uv run --project api python scripts/license_sign.py --licensee "Local demo" --expires-in 30d \
  --out "$LOCAL/licence-local.json" >/dev/null
curl -s -X POST -H 'content-type: application/json' --data-binary "@$LOCAL/licence-local.json" \
  "http://127.0.0.1:$API_PORT/license"
echo

if [ "$SEED" = empty ]; then
  echo "== Clearing the database; no demo data (the licence is kept)"
  curl -s -X POST "http://127.0.0.1:$API_PORT/admin/reset"
  echo
elif [ "$SEED" = yes ]; then
  echo "== Loading demo data (this clears the local database)"
  (cd judge && uv run python seed_demo.py --target "http://127.0.0.1:$API_PORT")
else
  echo "== Keeping your existing data (--fresh clears it and loads demo data; --empty clears it and loads nothing)"
fi

echo
if [ "$HOST" = "0.0.0.0" ]; then
  LAN=$(lan_ip)
  echo "== On this Wi-Fi (judges, phones): API http://$LAN:$API_PORT   website http://$LAN:$WEB_PORT"
  echo "   try from another machine:  curl http://$LAN:$API_PORT/health"
  echo "   if it cannot connect, allow ports $API_PORT and $WEB_PORT in Windows Firewall (infra/README.md)"
fi
echo "== On this laptop: http://127.0.0.1:$WEB_PORT   (Ctrl+C stops everything)"
cd web
SCAM_MAP_API_URL="http://127.0.0.1:$API_PORT" "$(venv_python .)" -m streamlit run app.py \
  --server.address "$HOST" --server.port "$WEB_PORT" --server.headless true
