<!-- How the app is hosted: on a laptop (DECISIONS D59). Owner: A. Filled in by: A0. -->
# infra: hosting locally

The app runs on a laptop (DECISIONS D59): the API and the Streamlit website as two separate services (D55).
Pick one of the two ways below. Both keep the database between runs.

## Option 1: one command (recommended)

From the repo root, in Git Bash on Windows:

```bash
scripts/run_local.sh
```

- API on http://127.0.0.1:8000, website on http://127.0.0.1:8501.
- Loads a 30-day licence signed with your key (`~/.scam-map/license.key`). Demo data is loaded only the first time;
  after that your own reports are kept. `--fresh` clears everything and reloads demo data; `--empty` clears everything and loads none.
- Data, the local HMAC key and the licence file live in `.local/` (gitignored). Ctrl+C stops everything.

## Option 2: the two containers

The same images a server would run. Build them from the repo root:

```bash
docker build -f infra/Dockerfile -t scam-map-api .
```

```bash
docker build -f infra/Dockerfile.web -t scam-map-web .
```

Make an HMAC key once and keep it: changing it makes every stored report unreachable.

```bash
mkdir -p .local && uv run --project api python -c "import secrets; print(secrets.token_hex(32))" > .local/hmac.key
```

Run both on one Docker network; the database lives on the `scam-map-data` volume:

```bash
docker network create scam-map
```

```bash
docker run -d --name scam-map-api --network scam-map -p 8000:8000 -v scam-map-data:/data -e SCAM_MAP_HMAC_KEY="$(cat .local/hmac.key)" scam-map-api
```

```bash
docker run -d --name scam-map-web --network scam-map -p 8501:8501 -e SCAM_MAP_API_URL=http://scam-map-api:8000 scam-map-web
```

Then load a licence (every route except `/health` and `/license` answers `403 LICENSE_EXPIRED` until you do):

```bash
uv run --project api python scripts/license_sign.py --licensee "Demo Bank" --expires-in 30d --out .local/licence.json
```

```bash
curl -X POST -H "content-type: application/json" --data-binary @.local/licence.json http://127.0.0.1:8000/license
```

Stop with `docker stop scam-map-api scam-map-web`; start again with `docker start scam-map-api scam-map-web`. The
API image starts as root only to take ownership of the `/data` volume, then runs as an unprivileged user.

## Checking it

These all work against the local address (they reset the server first):

Load the five evaluation datasets with plain curl, one request per report, and score them per PRD row. The
generated scripts are kept in `.local/curl/` so every command can be read:

```bash
scripts/curl_load.sh http://127.0.0.1:8000
```

The other checks:

```bash
scripts/verify_deployed.sh http://127.0.0.1:8000 --yes
```

```bash
scripts/license_demo.sh http://127.0.0.1:8000 licence-5min.json licence-long.json
```

Logs go to the terminal (Option 1) or `docker logs scam-map-api` (Option 2). By design there is no per-request log:
no URLs, query strings, identifiers or reporter fields are ever logged.

## Judging: other machines on the same Wi-Fi

`scripts/run_local.sh` listens on all network interfaces and prints the address at startup, for example:

```text
== On this Wi-Fi (judges, phones): API http://192.168.60.86:8000   website http://192.168.60.86:8501
   try from another machine:  curl http://192.168.60.86:8000/health
```

Judges reach the API at `http://<that address>:8000`, with the routes from PRD Appendix A, and the website at
`http://<that address>:8501`. Check from a second device on the same Wi-Fi first:

```bash
curl http://192.168.60.86:8000/health
```

**1. Allow the ports in Windows Firewall.** If the Wi-Fi is classed as Public (see it with `Get-NetConnectionProfile`),
Windows blocks incoming connections. Open PowerShell **as administrator** and allow just these two ports, only from
machines on your own subnet:

```powershell
New-NetFirewallRule -DisplayName "Scam Map (judges)" -Direction Inbound -Protocol TCP -LocalPort 8000,8501 -Action Allow -RemoteAddress LocalSubnet -Profile Any
```

Remove the rule when judging is over:

```powershell
Remove-NetFirewallRule -DisplayName "Scam Map (judges)"
```

**2. Know the limits.**
- The address changes if the laptop gets a new one from the router: read it again from the startup banner.
- Some Wi-Fi networks block devices from talking to each other (client isolation). If a second device cannot connect
  even with the rule, ask the organisers, or use a phone hotspot that both devices join.
- The API has no login on `POST /admin/reset` and `POST /license`, as the PRD requires, so anyone on the network can
  reset it. Keep the laptop on a Wi-Fi you trust while judging, and turn the rule off afterwards.
- The address is `http`, not `https`. The PRD's example is HTTPS; on a private Wi-Fi the judges' tool should accept
  `http`, so check with the organisers.
