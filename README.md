<!-- How to run and explain the Community Scam Map. Owner: A and B. -->
# Community Scam Map

People warn each other about scam phone numbers, UPI IDs, links and SMS senders, and check one before they
trust it. Anyone can report in under a minute, with no login. A lookup answers in milliseconds with one of
**Known scam**, **Suspicious**, **No reports** or **Verified business**, worded as "Reported by N people near
you". A map shows the last 7 days by area, and alerts fire only when something is unusual.

The hard part is fake reports. The trust rules below stop all four tricks in the brief (40 friends, 22 accounts
on one Wi-Fi, one person six times, junk) without missing real scams, and a judge simulator proves it on data
the rules were never tuned on.

| | |
| --- | --- |
| API | `http://<laptop address>:8000` on the same Wi-Fi; `scripts/run_local.sh` prints the address (http://127.0.0.1:8000 on the laptop itself) |
| Mobile website | `http://<laptop address>:8501` (Streamlit, English and Hindi, works with icons alone) |
| Brief | [docs/PRD.pdf](docs/PRD.pdf), our plan in [docs/SPEC.md](docs/SPEC.md), design in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) |

## For the judges: five minutes

1. Start the app on the host laptop with `scripts/run_local.sh`. It prints the Wi-Fi address, for example `http://192.168.60.86:8000`.
2. From any device on the same Wi-Fi: `curl http://<address>:8000/health` returns `{"ok":true}`.
3. Every route in PRD Appendix A is there with the exact fields. Reset with `POST /admin/reset`, then load reports
   with `POST /reports` as you like.
4. The licence demo (PRD row 5) is a four-step script: [docs/DEMO.md](docs/DEMO.md).
5. Our own evidence, in plain numbers: [docs/RESULTS.md](docs/RESULTS.md). The PRD scoring table, row by row, is in
   [docs/DEMO.md](docs/DEMO.md).

What to submit, from the brief, and where each item is:

| Item | Where |
| --- | --- |
| Code in a Git repository | this repository |
| Server address, working during judging | `http://<laptop address>:8000` on the judges' Wi-Fi (see Known limits: `http`, not `https`) |
| App | Mobile website on port 8501 (no APK) |
| README: run it, trust rules, licence | this file: [Run it](#run-it), [How trust rules work](#how-trust-rules-work), [How the licence works](#how-the-licence-works) |
| Demo video, up to 3 minutes | link given by the team at submission; the script is in [docs/DEMO.md](docs/DEMO.md) |
| Two licence files | `licence-long.json` is kept ready; the 5-minute file is signed just before the demo ([docs/DEMO.md](docs/DEMO.md)) |

## Run it

You need [uv](https://docs.astral.sh/uv/) (Python 3.12 is installed by uv if missing) and Git. Full setup,
including Windows notes, is in [docs/SETUP.md](docs/SETUP.md).

**Quickest: everything on this laptop in one command** (Git Bash on Windows). It starts the API on :8000 and the
website on :8501, loads a licence signed with your key, fills in demo data and prints numbers to try. Ctrl+C stops it.

```bash
scripts/run_local.sh
```

Next time it keeps everything you entered; `--fresh` clears it all and reloads demo data; `--empty` clears it and loads nothing (use it for judging). The steps below do the
same by hand.

```bash
git clone <repo-url> scam-map && cd scam-map
```

**1. Start the API** on port 8000. It needs an HMAC key (any long random string locally). These commands
are for Git Bash or macOS/Linux; PowerShell sets the variable with `$env:SCAM_MAP_HMAC_KEY = "..."` first:

```bash
cd api && uv sync --frozen && SCAM_MAP_HMAC_KEY=change-me-to-a-long-random-string uv run uvicorn app.main:app
```

`GET http://127.0.0.1:8000/health` returns `{"ok": true}`. Every other route answers
`403 {"error": "LICENSE_EXPIRED"}` until a licence is loaded.

**2. Load a licence.** Make a key pair once (the private key goes to `~/.scam-map/license.key`, never the repo),
sign a licence and upload it:

```bash
uv run --project api python scripts/license_keygen.py
```

```bash
uv run --project api python scripts/license_sign.py --licensee "Demo Bank" --expires-in 30d --out licence.json
```

```bash
curl -X POST -H "content-type: application/json" --data-binary @licence.json http://127.0.0.1:8000/license
```

**3. Start the website** on port 8501, in another terminal:

```bash
cd web && uv sync --frozen && SCAM_MAP_API_URL=http://127.0.0.1:8000 uv run streamlit run app.py
```

Open http://127.0.0.1:8501 on a phone-sized window. Try the PRD's example message on the Check screen:
`Your KYC has expired. Update at http://www.sbi-kyc.in/pay?utm=sms or call +91 98765 43210`.

**4. Run the judge simulator** against a server it starts itself:

```bash
cd judge && uv sync --frozen && uv run python runner.py --seeds holdout --target local
```

**5. Load the evaluation data with plain curl**, the way the judges' tool will. This **erases** the running server's
data, then sends every report of the five datasets (902 reports, 4 disputes) as its own `curl -X POST .../reports`,
checks each reply's status, and scores the outcomes per PRD row:

```bash
scripts/curl_load.sh http://127.0.0.1:8000
```

The generated scripts, one literal curl command per line, are kept in `.local/curl/` so every request can be read.
When it finishes, all five datasets are loaded in the app to browse.

Hosting options (one command, or the two Docker containers) are in [infra/README.md](infra/README.md).

## How trust rules work

Every report about the same identifier (after normalising, so `+91 98765 43210`, `09876543210`, `98765-43210`
and `919876543210` are one number) forms a case. Each time a report arrives, the rules re-read the whole case.
All numbers are in [api/app/core/trust_config.py](api/app/core/trust_config.py).

1. **One person is one person.** Reports with the same `reporter_id` count once, however far apart.
2. **One connection is one person.** Reports from the same `ip_prefix` within 24 hours merge into one. That
   stops 22 accounts on one Wi-Fi.
3. **Copy-paste is one voice.** Notes that are 80% or more alike (character shingles, so English, Hindi and
   Hinglish all work) merge across different people.
4. **Too many too fast is slowed down.** More than 3 reports from one reporter within an hour get `429`.
5. **Coordinated bursts are capped.** When 5 or more brand-new reporters hit one identifier within an hour,
   and either most of their notes are empty or copied, or all their incidents happened within 15 minutes, the
   whole group counts as **one** reporter and adds at most **1.5** to the score. That stops the 40 friends.
   A real sudden wave, where people were called at different times and write their own words, is not capped.
6. **Own words count more.** Each independent group weighs 1.0 with a note of at least 15 characters, 0.6
   without.
7. **Verdict.** On the verified list (PRD Appendix C): always **Verified business**. Otherwise **Known scam**
   needs at least **3 independent groups** and a score of at least **3.0**, and is capped at **Suspicious** while
   a business disputes the number. Any accepted report short of that is **Suspicious**; none is **No reports**.
8. **Fakes never alert.** Each report is marked `counted`, `merged` or `suspect`. The map and alerts use
   only `counted` reports, so fakes the rules ignored never move a map point or set off an alert.

`distinct_reporters` in a lookup is the number of independent groups, so the 40 friends show as 1 and the
bakery's number stays masked.

**Why a bombed bakery is not labelled a scam:** whichever way the 40 friends send their reports (one Wi-Fi or 40,
copied, lightly edited or empty notes), they arrive as a burst of new reporters telling one story about one
moment. They count as one reporter, weighing at most 1.5, and stay below both bars for Known scam.

**What these rules cannot do:** tell 40 friends who each write their own words, on different days, from
different networks, apart from a real campaign. That attack costs real effort for every report, and the bakery
can dispute the number, which caps it at Suspicious while under review.

## How the licence works

A licence is a small signed file:

```json
{"licensee": "Demo Bank", "expires_at": "2026-11-15T10:05:00+05:30", "signature": "..."}
```

- **Signed offline.** The signature is Ed25519 over the canonical JSON of `licensee` and `expires_at` (sorted
  keys, no spaces). The private key stays on our laptop (`~/.scam-map/license.key`, blocked by `.gitignore`);
  the server holds only the public key, built into the image. Editing the date breaks the signature.
- **Checked on every request.** The verified licence sits in memory, so the check is one time comparison.
  When it has expired, every route except `/health` and `/license` answers exactly
  `403 {"error": "LICENSE_EXPIRED"}`, and the app shows "Licence expired. Contact your provider."
- **Renewed without a restart.** `POST /license` verifies the new file, swaps it in memory and saves it on the
  volume, so it survives restarts. A refused file leaves the current licence in force.
- **Expired locks, never deletes.** Data is untouched; renewing brings it all back. Reset keeps the licence too.
- **Setting the clock back does not help.** The server remembers the latest time it has seen (saved to disk).
  A clock more than a minute behind it counts as expired, and expiry is judged against the later of the two.

The PRD's four-step demo, end to end against any server (waits for the short licence to expire):

```bash
DEMO_NUMBER=<a number with reports> scripts/license_demo.sh http://127.0.0.1:8000 licence-5min.json licence-long.json
```

A licence's `expires_at` is an absolute time, so **sign the 5-minute file just before the demo**; an
already-expired file is refused. Run this from the repo root (the private key must exist: `scripts/license_keygen.py`
makes it once):

```bash
uv run --project api python scripts/license_sign.py --licensee "Demo Bank" --expires-in 5m --out .local/demo/licence-5min.json
```

The long licence is signed once:

```bash
uv run --project api python scripts/license_sign.py --licensee "Demo Bank" --expires-in 365d --out .local/demo/licence-long.json
```

## Privacy

| PRD rule | How |
| --- | --- |
| Reporters anonymous | No login, name or phone number. `reporter_id`, `ip_prefix` and identifiers are stored only as keyed HMACs; notes only as keyed MinHash fingerprints that cannot be turned back into text. Area only, never a location. The website sends a random id per browser session. |
| Hidden until 3 people | `display_value` is masked (`98XXXXX210`, `r***@okaxis`, `sbi***.in/...`, `HD****`) until 3 independent reporters. |
| "Reported", not "scammer" | "Reported by N people near you" in both languages; a scanner fails the build on banned words. |
| No scraping | The API makes no outbound calls at all. |
| No personal data in logs | Access logs are off; a scanner reads every log line of a full run. |

The privacy scanners (`cd judge && uv run python -m scanners.all --target local`) run the simulator and then
check every reply, every server log line and the raw database bytes for anything a reporter sent.

## Tests

| Layer | What | Command | Result (1 Nov) |
| --- | --- | --- | --- |
| Unit and property | Normaliser, masking, verdicts, trust rules, licence; Hypothesis with 1,000 examples per property | `cd api && uv run pytest` | 432 tests, 99% coverage |
| Contract | Every PRD route, status and reply schema | `cd judge && uv run python contract.py --target local` | 33/33 |
| Judge simulator | 15 scenario families, tuned on seeds 1 to 10, reported on held-out seeds 101 to 150 | `uv run python runner.py --seeds holdout --target local` | 750/750 scenarios, all thresholds met; fresh seeds at 75, 300 and 900 scenarios all score 82.0 of 82 |
| Metamorphic | Same scenario shuffled, re-spelled, other time offsets, junk mixed in | `uv run python metamorphic.py --seeds holdout --target local` | 1,600/1,600 identical |
| Privacy scanners | Replies, logs, database, wording, hardcoding | `uv run python -m scanners.all --target local` | 0 findings |
| Evaluation datasets | 5 fixed PRD-format datasets (tricks, real scams, verified and disputes, map and alerts, a judging-day mix), scored per PRD row | `uv run python score_dataset.py --target local` | 121/121 checks |
| The same datasets, loaded with plain `curl` | One curl per report (902) plus 4 disputes, statuses checked, then scored | `scripts/curl_load.sh http://127.0.0.1:8000` | 116/116 checks, 82.0 of 82 ([docs/RESULTS.md](docs/RESULTS.md)) |
| Robustness | Fuzzed bodies and queries, 50 concurrent reports | in the API tests | no 5xx, exact counts |
| Performance | 100k reports, 2,000 lookups, 500 reports | `uv run python perf.py --target <url>` | see [docs/RESULTS.md](docs/RESULTS.md) |
| UI | Every screen in English, Hindi and icon-only mode | `cd web && uv run pytest` | 197 tests |

`scripts/check.sh` runs all local layers; `scripts/verify_deployed.sh https://<api> --yes` runs them against the
deployed server (it resets it first).

## Layout

```text
api/      FastAPI server: routes, core rules, licence, migrations, tests      (Person B)
web/      Streamlit mobile website: six screens, English and Hindi, icons      (Person A)
judge/    Contract tests, judge simulator, metamorphic tests, scanners, perf   (Person B)
shared/   The contract: JSON Schemas, PRD data (areas, verified, scam types), test vectors
infra/    Dockerfiles and the local hosting guide                              (Person A)
scripts/  check.sh, verify_deployed.sh, licence key, sign and demo scripts
docs/     PRD, spec, architecture, decisions, results, setup
```

## Known limits

- **"Verified business" means "on the organisers' list", nothing more.** It is an exact match against the 19 values
  in PRD Appendix C (`shared/data/verified.json`). The app does not check anyone's identity: nobody can become
  verified through it, and a dispute never adds a number to the list. Callers can fake any number, which the result
  screen says. Real verification would need a registry or proof per type (see DECISIONS D61).
- **One dispute per number.** A second dispute on the same number gets `429`, so it cannot be spammed.
- **Nothing older than 30 days is shown as an alert or activity.** Such a report is still accepted and counted
  (the map can show it with a wider window); it just never appears in the live panel.
- **40 friends writing their own words over days** cannot be told apart from a real campaign (see above).
- **Device id per session.** The website's anonymous id lasts one browser session (DECISIONS D57); a returning
  visitor on another network counts as a new reporter.
- **One API machine.** SQLite on one volume means one machine; fine for this load (see performance), but scaling
  out would mean moving to Postgres.
- **Hosted on a laptop, reached over the Wi-Fi** (DECISIONS D59, D62). The judges curl `http://<laptop address>:8000`
  from the same network; the address is plain `http`, and the PRD's example is HTTPS. Windows Firewall needs the
  two ports opened ([infra/README.md](infra/README.md)).
- **Rows 6 and 7 are judged by people** and our tools cannot score them: the usability stopwatch test and the demo
  video are done by hand.

## Future scope

- **Weigh reports by city.** A report from outside a number's home city would count for less until reporters from
  that city have also reported it, so a distant attacker cannot brand a local business. Reports already carry an
  area, so this needs a city per area and one extra weight in `trust_config.py`. It is a proposal, not built.
- Public HTTPS hosting and an Android app.
- A review queue for disputes and an updatable, signed verified-business list.
- Postgres and caching for many cities; more languages; alerts pushed to phones.

## Documents

| File | What it is |
| --- | --- |
| [docs/PRD.pdf](docs/PRD.pdf) | The organisers' brief (source of truth) |
| [docs/SPEC.md](docs/SPEC.md) | Build plan and scoring map |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | System design and request paths |
| [docs/DECISIONS.md](docs/DECISIONS.md) | Every design decision and why |
| [docs/DEMO.md](docs/DEMO.md) | Demo script, licence steps, PRD scoring table |
| [docs/RESULTS.md](docs/RESULTS.md) | Test runs and measurements |
| [docs/SETUP.md](docs/SETUP.md) | Developer setup |
| [docs/Hackathon-Brief.pdf](docs/Hackathon-Brief.pdf) | Three-page summary: checklist, design, likely questions |
| [infra/README.md](infra/README.md) | Hosting on a laptop and on the judges' Wi-Fi |
| [docs/diagrams/](docs/diagrams) | Architecture, request flow, licence and schema diagrams |
