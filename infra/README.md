<!-- How the app is hosted: a public VM with HTTPS, or a laptop. Owner: A. Filled in by: A0, D1. -->
# infra: hosting

One container serves everything: the Rust API with the PRD routes, and the React website at `/` (DECISIONS D63 to D66).
SQLite lives on a Docker volume.

| File | What it is |
| --- | --- |
| `Dockerfile` | Builds the website (Node) and the API (Rust), then a small Debian runtime image (about 150 MB) that runs as an unprivileged user |
| `compose.yml` | The public server: the app behind Caddy, which gets and renews the HTTPS certificate itself |
| `Caddyfile` | HTTPS, http to https redirect, compression, security headers; no access log |
| `env.example` | The three settings the server needs; copy to `infra/.env` (gitignored) |
| `entrypoint.sh` | Hands the `/data` volume to the app user, then drops root |

## Public server: a free VM with HTTPS

You need an always-on Linux VM with a public IP, and a domain name pointing at it. Oracle Cloud's Always Free tier
works (an Ampere A1 VM, ARM, with up to 4 cores and 24 GB), and DuckDNS gives a free name. Any other VM works the
same way. Account sign-ups are yours to do; every command below runs on the VM unless it says otherwise.

**1. Create the VM.** In Oracle Cloud: Compute, Instances, Create. Image Ubuntu 24.04, shape `VM.Standard.A1.Flex`
(2 OCPU, 12 GB is plenty), add your SSH public key. Note the public IP. If A1 capacity is not available in your
region, try another availability domain or try again later.

**2. Open ports 80 and 443.** Two places, both needed on Oracle:
- In the console: the instance's subnet, Security List, Add Ingress Rules: source `0.0.0.0/0`, TCP, destination
  ports `80` and `443`.
- On the VM, Oracle's Ubuntu image rejects everything but SSH in iptables. Allow the two ports and keep the rule:

```bash
sudo iptables -I INPUT 6 -p tcp -m multiport --dports 80,443 -j ACCEPT && sudo netfilter-persistent save
```

**3. Name it.** At duckdns.org, sign in, create a subdomain (for example `block128`), and set its IP to the VM's
public IP. The name is then `block128.duckdns.org`.

**4. Install Docker** (Docker's official install script) and let your user run it:

```bash
curl -fsSL https://get.docker.com | sh && sudo usermod -aG docker "$USER"
```

Log out and back in, then check with `docker run --rm hello-world`.

**5. Get the code and the settings.**

```bash
git clone <your repo url> scam-map && cd scam-map && cp infra/env.example infra/.env
```

Edit `infra/.env`: `DOMAIN` is your DuckDNS name; make the two secrets with `openssl rand -hex 32` and
`openssl rand -hex 24`. Keep the HMAC key safe: changing it makes every stored report unreachable.

**6. Start it.** The first build compiles the API on the VM and takes several minutes:

```bash
docker compose -f infra/compose.yml --env-file infra/.env up -d --build
```

Caddy gets the certificate within a minute. Then, from anywhere:

```bash
curl https://block128.duckdns.org/health
```

**7. Load a licence.** On your laptop, where the private key is, sign one and upload it:

```bash
uv run --project judge python scripts/license_sign.py --licensee "Demo Bank" --expires-in 365d --out .local/licence-long.json
```

```bash
curl -X POST -H "content-type: application/json" --data-binary @.local/licence-long.json https://block128.duckdns.org/license
```

The website is now live at `https://block128.duckdns.org`. Every data route answers `403 LICENSE_EXPIRED` until a
licence is loaded; the website itself always loads and says so.

### Backups

`scam-map-api backup <file>` copies the database safely while the server runs. Add a nightly copy and keep two
weeks of them with `crontab -e` on the VM (one line; `%` must be written `\%` in crontab):

```text
0 3 * * * cd ~/scam-map && docker compose -f infra/compose.yml --env-file infra/.env exec -T -u app app sh -c 'scam-map-api backup /data/backups/scam_map-$(date +\%F).sqlite && find /data/backups -name "*.sqlite" -mtime +14 -delete'
```

To copy them off the server: `docker compose -f infra/compose.yml --env-file infra/.env cp app:/data/backups ./backups`.

### Updating and looking at it

```bash
git pull && docker compose -f infra/compose.yml --env-file infra/.env up -d --build
```

`docker compose -f infra/compose.yml --env-file infra/.env ps` shows both services (the app reports `healthy`);
`... logs app` shows only startup lines. Neither the app nor Caddy logs requests: no URL, query string, identifier
or address is ever written down.

### Checking it from your laptop

The judge tools work against the public address. They reset the server first, so never run them on real data.
Give them the admin token so reset is allowed:

```bash
SCAM_MAP_ADMIN_TOKEN=<token from infra/.env> scripts/verify_deployed.sh https://block128.duckdns.org --yes
```

The same variable works for `scripts/curl_load.sh` and every `judge/` tool.

### Public-server settings

| Variable | Effect |
| --- | --- |
| `SCAM_MAP_ADMIN_TOKEN` | When set, `POST /admin/reset` needs it in the `x-admin-token` header, so strangers cannot erase the server. The PRD wants reset open for the judges' tool: leave it empty only for judging. `POST /license` stays open; it accepts only files signed with your private key. |
| `SCAM_MAP_TRUST_PROXY=1` | Set by `compose.yml`. Every connection comes from Caddy, so the client's address is read from the `X-Forwarded-For` entry Caddy adds. Without it, all website reporters would look like one connection. |

## On a laptop

From the repo root, in Git Bash on Windows (needs Rust, Node and uv: docs/SETUP.md):

```bash
scripts/run_local.sh
```

- The website and the API on http://127.0.0.1:8000.
- Loads a 30-day licence signed with your key (`~/.scam-map/license.key`). Demo data is loaded only the first time;
  after that your own reports are kept. `--fresh` clears everything and reloads demo data; `--empty` clears
  everything and loads none.
- Data, the local HMAC key and the licence file live in `.local/` (gitignored). Ctrl+C stops it.

The whole public stack also runs on a laptop with Docker Desktop, using `DOMAIN=localhost` in the env file; Caddy
then uses its own local certificate authority instead of Let's Encrypt.

### Other machines on the same Wi-Fi

`scripts/run_local.sh` listens on all network interfaces and prints the address, for example
`http://192.168.60.86:8000`. If the Wi-Fi is classed as Public, Windows blocks incoming connections: in PowerShell
**as administrator**, allow port 8000 from your own subnet only, and remove the rule afterwards:

```powershell
New-NetFirewallRule -DisplayName "Scam Map" -Direction Inbound -Protocol TCP -LocalPort 8000 -Action Allow -RemoteAddress LocalSubnet -Profile Any
```

```powershell
Remove-NetFirewallRule -DisplayName "Scam Map"
```

Some Wi-Fi networks block devices from reaching each other; a phone hotspot that both devices join avoids that. This
address is plain `http`, and reset has no token here: keep the laptop on a network you trust.
