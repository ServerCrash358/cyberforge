#!/usr/bin/env bash
# Shows, in files you can open, that nothing a reporter sends ends up in logs or the database.
#   scripts/privacy_check.sh [dataset.json]     default: judge/datasets/05_judging_day.json
# Starts a separate API on :8100 writing everything it prints to .local/privacy/server.log,
# loads the dataset (every report has reporter_id, ip_prefix, note and a phone number or UPI),
# then searches the log and the raw database bytes for every value that was sent.
# Owner: B. Filled in by: B7.
set -euo pipefail
cd "$(dirname "$0")/.."

DATASET=${1:-judge/datasets/05_judging_day.json}
OUT=.local/privacy
PORT=8100
rm -rf "$OUT" && mkdir -p "$OUT"

export PATH="$HOME/.cargo/bin:$PATH"
(cd api-rs && cargo build --release --locked --quiet)
KEY=$(uv run --project judge python -c "import secrets; print(secrets.token_hex(32))")
(cd api-rs && SCAM_MAP_DB_PATH="../$OUT/scam_map.sqlite" SCAM_MAP_HMAC_KEY="$KEY" \
  SCAM_MAP_PORT="$PORT" exec ./target/release/scam-map-api > "../$OUT/server.log" 2>&1) &
API_PID=$!
trap 'kill $API_PID 2>/dev/null || true; wait $API_PID 2>/dev/null || true' EXIT
for _ in $(seq 1 60); do curl -sf "http://127.0.0.1:$PORT/health" >/dev/null && break; sleep 0.5; done

uv run --project judge python scripts/license_sign.py --licensee "Privacy check" --expires-in 1d \
  --out "$OUT/licence.json" >/dev/null
curl -s -o /dev/null -X POST -H 'content-type: application/json' --data-binary "@$OUT/licence.json" \
  "http://127.0.0.1:$PORT/license"

echo "== Loading $DATASET into the API on :$PORT"
(cd judge && uv run python score_dataset.py "../$DATASET" --target "http://127.0.0.1:$PORT" \
  --keep-data > "../$OUT/score.txt" 2>&1) || true
tail -1 "$OUT/score.txt"

kill $API_PID 2>/dev/null || true; wait $API_PID 2>/dev/null || true; trap - EXIT

echo "== Searching the log and the database for every value that was sent"
(cd judge && uv run python - "../$DATASET" "../$OUT" <<'PY')
import json, sys
from pathlib import Path

dataset, out = Path(sys.argv[1]), Path(sys.argv[2])
reports = json.loads(dataset.read_text(encoding="utf-8"))["reports"]
sent = {
    "reporter_id": {r["reporter_id"] for r in reports},
    "ip_prefix": {r["ip_prefix"] for r in reports if len(r["ip_prefix"]) >= 8},
    "note": {r["note"] for r in reports if len(r["note"].strip()) >= 15},
    "phone number": {"".join(c for c in r["identifier_value"] if c.isdigit())[-10:]
                     for r in reports if r["identifier_type"] == "phone"
                     and sum(c.isdigit() for c in r["identifier_value"]) >= 10},
    "UPI ID": {r["identifier_value"].strip().lower() for r in reports if r["identifier_type"] == "upi"},
}
log = (out / "server.log").read_bytes()
db = b"".join(p.read_bytes() for p in out.glob("scam_map.sqlite*"))
lines = log.decode("utf-8", "replace").splitlines()
print(f"server.log: {len(lines)} lines; database: {len(db):,} bytes; {len(reports)} reports sent")
bad = 0
for what, values in sent.items():
    in_log = sum(v.encode() in log for v in values)
    in_db = sum(v.encode() in db for v in values)
    bad += in_log + in_db
    print(f"  {len(values):>4} {what:<13} found in log: {in_log}   found in database: {in_db}")
print("PASS: no reporter field, note, number or UPI ID in the log or database" if bad == 0
      else f"FAIL: {bad} values found")
PY
echo
echo "Open these to see for yourself:"
echo "  $OUT/server.log      everything the server logged"
echo "  $OUT/scam_map.sqlite  the database (open with any SQLite viewer, e.g. DB Browser for SQLite)"
echo "  $DATASET             what was sent"
