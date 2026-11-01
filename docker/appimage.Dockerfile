# Build do AppImage do manabox em Ubuntu 22.04 (glibc 2.35): o AppImage roda em 22.04 ou mais novo.
# Usado por scripts/build-appimage.sh; o projeto é montado em /app, não copiado para a imagem.
FROM ubuntu:22.04

ARG RUST_VERSION=1.99.1
ARG NODE_VERSION=22.12.0
ARG PNPM_VERSION=9.5.0

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential curl wget file ca-certificates xz-utils pkg-config \
        libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev libayatana-appindicator3-dev \
        libssl-dev libxdo-dev patchelf desktop-file-utils \
    && rm -rf /var/lib/apt/lists/*

# Rust em /usr/local (só leitura para o usuário do build; o cache do cargo vai para /cache)
ENV RUSTUP_HOME=/usr/local/rustup PATH=/usr/local/cargo/bin:$PATH
RUN curl -sSf https://sh.rustup.rs | CARGO_HOME=/usr/local/cargo \
        sh -s -- -y --no-modify-path --profile minimal --default-toolchain ${RUST_VERSION} \
    && chmod -R a+rX /usr/local/rustup /usr/local/cargo

RUN curl -fsSL https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-linux-x64.tar.xz \
        | tar -xJ -C /usr/local --strip-components=1 \
    && npm install -g pnpm@${PNPM_VERSION}

# Caches graváveis pelo usuário do host (volume Docker em /cache)
ENV HOME=/cache/home CARGO_HOME=/cache/cargo npm_config_store_dir=/cache/pnpm-store \
    APPIMAGE_EXTRACT_AND_RUN=1 CI=true

WORKDIR /app
