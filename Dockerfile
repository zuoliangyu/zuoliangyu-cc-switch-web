FROM node:20-bookworm AS frontend-builder

WORKDIR /app

RUN corepack enable

COPY package.json pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile

COPY . .
RUN pnpm build:web


FROM rust:1.85-bookworm AS service-builder

WORKDIR /app

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY . .
COPY --from=frontend-builder /app/dist ./dist

RUN cargo build --release --manifest-path backend/Cargo.toml --bin cc-switch-web


FROM debian:bookworm-slim AS package-linux-dir

WORKDIR /out/cc-switch-web-linux-x64

COPY --from=service-builder /app/backend/target/release/cc-switch-web ./cc-switch-web
COPY --from=frontend-builder /app/dist ./dist
COPY scripts/run-web-release.sh ./run-web.sh

RUN chmod +x ./cc-switch-web ./run-web.sh


FROM debian:bookworm-slim AS package-linux-tar

WORKDIR /work

COPY --from=package-linux-dir /out/cc-switch-web-linux-x64 ./cc-switch-web-linux-x64

RUN tar -czf /out/cc-switch-web-linux-x64.tar.gz cc-switch-web-linux-x64


FROM debian:bookworm-slim

WORKDIR /app

ENV HOME=/data \
    CC_SWITCH_WEB_HOST=0.0.0.0 \
    CC_SWITCH_WEB_PORT=8788 \
    CC_SWITCH_WEB_DIST_DIR=/app/dist

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=service-builder /app/backend/target/release/cc-switch-web /usr/local/bin/cc-switch-web
COPY --from=frontend-builder /app/dist ./dist

VOLUME ["/data"]

EXPOSE 8788

CMD ["cc-switch-web"]
