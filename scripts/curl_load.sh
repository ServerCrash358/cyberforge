#!/usr/bin/env bash
# Erases a server's data, then loads the five evaluation datasets with plain curl, one request per
# report, the way the judges' tool will: no Python client in between.
#   scripts/curl_load.sh [BASE_URL]          default http://127.0.0.1:8000
#
# Phase 1, for each dataset: POST /admin/reset, curl every report and dispute (the generated script is
#   kept in .local/curl/NAME.sh so every command can be read), then check the dataset's expected
#   outcomes (verdicts, masking, map, alerts, disputes) and print the PRD scoring table.
# Phase 2: reset again and curl all five datasets into the server together, left loaded to browse.
#   Not scored: the checks assume one dataset at a time (for example "this quiet area has no alerts").
# Never run it against the server the judges use: it erases it.
# Owner: B. Filled in by: B9.
set -uo pipefail
cd "$(dirname "$0")/.."

BASE=${1:-http://127.0.0.1:8000}
BASE=${BASE%/}
OUT=.local/curl
ROOT=$(pwd -W 2>/dev/null || pwd)   # a path Python on Windows understands, for the files it reads
mkdir -p "$OUT"

if ! curl -sf "$BASE/health" >/dev/null; then
  echo "nothing answers at $BASE/health: start the server first (scripts/run_local.sh)" >&2
  exit 1
fi
licence=$(curl -s "$BASE/license")
case "$licence" in
  *'"valid":true'*) ;;
  *) echo "the server's licence is not valid: $licence" >&2; exit 1 ;;
esac

# A public server started with SCAM_MAP_ADMIN_TOKEN wants the same token on reset.
reset() {
  curl -s -o /dev/null -w '%{http_code}' -X POST ${SCAM_MAP_ADMIN_TOKEN:+-H "x-admin-token: $SCAM_MAP_ADMIN_TOKEN"} \
    "$BASE/admin/reset"
}
empty_now() {  # nothing on the map and no activity: what a reset must leave behind
  [ "$(curl -s "$BASE/map?pincode=560102&radius_km=5000&days=3650")" = '{"clusters":[]}' ] \
    && curl -s "$BASE/app/activity" | grep -q '"events":\[\]'
}
in_judge() { (cd judge && uv run python "$@"); }
make_script() { in_judge curl_dataset.py "datasets/$1.json" > "$OUT/$1.sh"; }
count_requests() { grep -c '^code=\$(curl' "$OUT/$1.sh"; }

failed=0
names=$(cd judge/datasets && ls ./*.json | sed 's#^\./##; s#\.json$##')

echo "== Phase 1: each dataset on its own, erased first, loaded with curl, then checked"
for name in $names; do
  echo
  code=$(reset)
  if empty_now; then empty=yes; else empty=NO; failed=1; fi
  echo "-- $name: POST /admin/reset answered $code; database empty afterwards: $empty"
  make_script "$name"
  echo "   $(count_requests "$name") curl requests; every command is in $OUT/$name.sh"
  : > "$OUT/$name.status.tsv"
  STATUS_LOG="$OUT/$name.status.tsv" bash "$OUT/$name.sh" "$BASE" || failed=1
  in_judge score_dataset.py "datasets/$name.json" --target "$BASE" --no-load \
    --statuses "$ROOT/$OUT/$name.status.tsv" \
    --json "$ROOT/$OUT/$name.results.json" > "$OUT/$name.score.txt" 2>&1 || failed=1
  grep -E '^===|FAIL' "$OUT/$name.score.txt"
done

echo
echo "== PRD scoring table, all datasets together (each was loaded with curl, then checked)"
in_judge score_dataset.py --summary "$ROOT/$OUT"/*.results.json || failed=1

echo
code=$(reset)
if empty_now; then empty=yes; else empty=NO; failed=1; fi
echo "== Phase 2: erase again (POST /admin/reset answered $code; database empty afterwards: $empty),"
echo "   then curl all datasets together"
for name in $names; do
  make_script "$name"
  echo "-- $name ($(count_requests "$name") requests)"
  bash "$OUT/$name.sh" "$BASE" || failed=1
done

counted=$(curl -s "$BASE/map?pincode=560102&radius_km=5000&days=60" | grep -o '"count":[0-9]*' | cut -d: -f2 | awk '{s += $1} END {print s + 0}')
echo
echo "== Done in ${SECONDS}s. The server now holds all five datasets: $counted counted reports on the map."
if [ "$failed" -ne 0 ]; then echo "SOMETHING FAILED: read the lines above" >&2; fi
exit "$failed"
