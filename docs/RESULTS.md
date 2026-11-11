<!-- Test run log. Owner: A and B. -->
# Results

Test runs, newest at the bottom of each section. The server is now a laptop on the judges' Wi-Fi (DECISIONS D59), so the Fly.io run below is history.

## Simulator at different sizes, local, 1 Nov

Fresh seeds never used for tuning, in-process, all 15 families:

| Seeds per family | Scenarios | Time | Measured score | Thresholds |
| --- | --- | --- | --- | --- |
| 5 | 75 | 13 s | 82.0 / 82 | all met |
| 20 | 300 | 49 s | 82.0 / 82 | all met |
| 60 | 900 | 139 s | 82.0 / 82 | all met |

The held-out run of 750 scenarios (seeds 101 to 150) also passed. Rows 6 and 7 are judged by people and are not measured here.

## B8 performance, local, 31 Oct

`judge/perf.py --target local --reports 100000` on a Windows laptop (one uvicorn worker, SQLite WAL):

| What | n | p50 | p95 | max |
| --- | --- | --- | --- | --- |
| Load | 100,000 reports in 264 s (379/s), 0 refused | | | |
| Lookup | 2,000 | 1.9 ms | 2.6 ms | 5.2 ms |
| Report | 500 | 3.5 ms | 5.2 ms | 38.3 ms |
| Map (5 km, 7 days) | 100 | 10.9 ms | 31.3 ms | 40.6 ms |

Over a network, round-trip time adds to these; `perf.py --target <url>` measures any address.

## scripts/check.sh, 2026-10-31

Passed end to end: API 406 tests (99% coverage of core and license), judge contract 32/32, held-out simulator all thresholds met, privacy scanners 0 findings, web 180 tests, pip-audit 0 findings in all three projects.

Re-run on 1 Nov after the latest changes: `scripts/check.sh` passed end to end (exit 0): lint, types, API tests, judge contract, held-out simulator 82.0/82 with all thresholds met, privacy scanners 0 findings, web 197 tests, pip-audit 0 findings.

## verify_deployed.sh, 2026-10-31T17:07Z, https://scam-map-api-shubhang.fly.dev (history: the Fly.io apps were deleted, D59)

- PASS  health
- PASS  licence valid
- PASS  lookup under 500 ms
- PASS  contract
- PASS  simulator (held-out seeds)
- FAIL  metamorphic (tuning seeds)

Notes on that run against https://scam-map-api-shubhang.fly.dev (Fly.io, Singapore):

- Simulator held-out: every family met its threshold. R2 was 49/50: one scenario (R2:120) failed only because a
  request got no reply within 30 s (client `ReadTimeout`), not on any rule.
- Metamorphic: the run stopped on a second 30 s network timeout. `metamorphic.py` now records a network error as one
  failed check instead of stopping (fixed after this run).
- Fly restarted the machine once during the run (`requested_stop=true`, not out of memory). A restart measured on
  purpose afterwards took about 4 s from stop to the first `200`, so the 30 s stalls were the network path from
  Bengaluru to Singapore, not the server.
- After redeploying: contract 32/32 against the live address, licence intact.

## Evaluation datasets, local, 31 Oct

`judge/score_dataset.py` loaded the five datasets in `judge/datasets/` (902 PRD-format reports, 4 disputes) into
the local server and scored them: **119/119 checks passed** (121/121 after the live panel added two), an estimated 82.0 of the 82 points the datasets can
measure (rows 6 and 7 are judged by people).

| Dataset | Reports | Checks |
| --- | --- | --- |
| 01 tricks (40 friends x3, 22 accounts, spam x2, 19 kinds of junk) | 173 | 28/28 |
| 02 real scams (all four types, wave, 1-3 reporters, empty and Hindi notes, offsets) | 55 | 25/25 |
| 03 verified and disputes (all 19 Appendix C values, 4 disputes) | 158 | 32/32 |
| 04 map and alerts (windows, radius edge, other cities, every alert rule, fakes) | 199 | 18/18 |
| 05 judging-day mix (all 25 areas) | 317 | 16/16 |

The first scoring run found 4 failures, all in the dataset builder (verified short numbers spelled as mobiles; a
test area inside the radius check). They were fixed in the builder; the server was unchanged.

## Evaluation datasets loaded with plain curl, 1 Nov

`scripts/curl_load.sh http://127.0.0.1:8000` erased the local server with `POST /admin/reset`, then sent every
report and dispute of the five datasets as its own `curl -X POST` (906 requests: 902 reports and 4 disputes), with no
Python client in between. The generated scripts, one literal curl command per line, are in `.local/curl/`.

| What | Result |
| --- | --- |
| Unexpected statuses | 0 of 906. Reports: 872 accepted (201), 21 junk refused (400), 9 spam refused (429). Disputes: 3 accepted, 1 refused (400, an invalid number), as the datasets expect. |
| Database empty after each reset | yes, all 6 resets |
| Checks, scored per PRD row | 116/116, an estimated 82.0 of the 82 points the datasets can measure |
| Stored data against the dataset files | 0 differences in note length, area, scam type, screenshot flag or amount for all 872 stored reports, including the Devanagari notes |
| Time | 141 s for both phases: 6 resets, about 1,800 curl calls, the checks. A curl per report is about 13 requests/s on Windows (the Python client does about 380/s), because each curl is its own process. |

Phase 1 loads and scores each dataset on its own (the checks assume one at a time). Phase 2 loads all five together
for browsing and is not scored.

## Rust API parity (plan R5), local, 11 Nov

The Rust server in `api-rs/` run through the same judge tools that scored the Python one. The tools now build and
start the Rust release binary (`judge/contract.py` `local_server()`).

| Check | Rust | Python (earlier runs) |
| --- | --- | --- |
| Contract (`contract.py`) | 33/33 | 33/33 |
| Simulator, held-out seeds 101 to 150 | 750/750, all 15 families 100%, 82.0/82 | 750/750, 82.0/82 |
| Metamorphic, held-out seeds | 1,600/1,600 identical | 1,600/1,600 |
| Privacy scanners (hardcoding now reads the Rust source) | 0 findings in all five | 0 findings |
| Five datasets, Python client (`score_dataset.py`) | 121/121 | 121/121 |
| Five datasets, plain curl (`scripts/curl_load.sh`) | 116/116, 0 unexpected statuses | 116/116 |

Speed, both servers on this laptop with the same 20,000-report load (`perf.py --reports 20000`):

| | Python | Rust |
| --- | --- | --- |
| Load rate | 173/s | 315/s |
| Report p50 / p95 | 10.0 / 12.4 ms | 7.2 / 10.0 ms |
| Lookup p50 / p95 | 2.2 / 3.9 ms | 0.7 / 1.3 ms |
| Map p50 / p95 | 4.8 / 9.6 ms | 3.4 / 7.6 ms |

At 100,000 reports the Rust server loads at 58/s (report p50 32 ms; lookup p95 1.7 ms, map p95 32.8 ms). Writes slow
down as an area's history grows because every accepted report re-reads the area's last 31 days of reports to check for
new alerts for the live panel. Both servers do this; the 379/s in the B8 run above predates the live panel.
