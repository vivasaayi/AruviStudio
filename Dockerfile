FROM node:22-bookworm-slim AS frontend
WORKDIR /build
COPY package.json package-lock.json ./
RUN npm ci
COPY index.html tsconfig*.json vite.config.ts ./
COPY src ./src
RUN npm run build

FROM rust:1-bookworm AS build
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential pkg-config libssl-dev libgtk-3-dev \
    libayatana-appindicator3-dev librsvg2-dev libwebkit2gtk-4.1-dev \
    libclang-dev clang cmake && rm -rf /var/lib/apt/lists/*
WORKDIR /build
COPY crates ./crates
COPY src-tauri ./src-tauri
COPY docs ./docs
COPY LICENSE NOTICE ./
COPY --from=frontend /build/dist ./dist
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,id=aruvistudio-cargo-target,target=/build/src-tauri/target \
    GGML_NATIVE=OFF cargo build --locked --release --manifest-path src-tauri/Cargo.toml --bin aruvi-studio-server && \
    cp src-tauri/target/release/aruvi-studio-server /tmp/aruvi-studio-server

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates git libssl3 libgtk-3-0 libayatana-appindicator3-1 \
    librsvg2-2 libwebkit2gtk-4.1-0 libgomp1 && \
    rm -rf /var/lib/apt/lists/* && \
    useradd --create-home --uid 10001 aruvi && mkdir -p /data /repositories && chown aruvi:aruvi /data /repositories
COPY --from=build /tmp/aruvi-studio-server /usr/local/bin/aruvi-studio-server
USER aruvi
WORKDIR /data
ENV ARUVI_APP_DATA_DIR=/data ARUVI_DB_PATH=/data/aruvi-studio.db \
    ARUVI_WEBHOOK_HOST=0.0.0.0 ARUVI_WEBHOOK_PORT=8787
EXPOSE 8787
CMD ["aruvi-studio-server"]
