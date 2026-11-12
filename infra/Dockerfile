# One image: the React website built, the Rust API built, both in a small runtime image.
# Build from the repo root (on the server itself, so the binary matches its CPU, x86 or ARM):
#   docker build -f infra/Dockerfile -t scam-map .
# The API serves the website at / and the PRD routes beside it. SQLite lives on the /data volume.
# Owner: A. Filled in by: A0, D1 (Rust and React).

FROM node:24.19.0-trixie-slim AS website
WORKDIR /src/ui
COPY ui/package.json ui/package-lock.json ./
RUN npm ci --no-fund --no-audit --loglevel=error
COPY shared /src/shared
COPY ui ./
RUN npm run build

FROM rust:1.99.1-slim-trixie AS api
WORKDIR /src/api-rs
COPY api-rs ./
COPY shared /src/shared
RUN cargo build --release --locked

FROM debian:trixie-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --user-group --uid 10001 app \
    && mkdir -p /data /data/backups \
    && chown -R app:app /data
COPY --from=api /src/api-rs/target/release/scam-map-api /usr/local/bin/scam-map-api
COPY --from=website /src/ui/dist /srv/web
COPY infra/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod 0755 /usr/local/bin/entrypoint.sh

ENV SCAM_MAP_DB_PATH=/data/scam_map.sqlite \
    SCAM_MAP_WEB_DIR=/srv/web \
    SCAM_MAP_HOST=0.0.0.0 \
    SCAM_MAP_PORT=8000
VOLUME ["/data"]
EXPOSE 8000
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s CMD curl -fsS http://127.0.0.1:8000/health || exit 1

# Starts as root only to take ownership of /data, then runs the server as the app user.
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
CMD ["scam-map-api"]
