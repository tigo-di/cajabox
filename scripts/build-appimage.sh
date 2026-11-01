#!/usr/bin/env bash
# Gera o AppImage do manabox dentro do Docker (Ubuntu 22.04) e copia para release/.
# node_modules, target e caches do container ficam em volumes Docker próprios: não se misturam
# com os do host, compilados com outra glibc.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IMAGE=manabox-appimage
VOLUMES=(
    -v "$ROOT:/app"
    -v manabox-appimage-node_modules:/app/node_modules
    -v manabox-appimage-target:/app/src-tauri/target
    -v manabox-appimage-cache:/cache
)

echo "==> Imagem $IMAGE"
docker build -t "$IMAGE" -f "$ROOT/docker/appimage.Dockerfile" "$ROOT/docker"

# Volumes novos nascem do root; passam para o usuário do host
docker run --rm "${VOLUMES[@]}" -u 0 "$IMAGE" \
    chown "$(id -u):$(id -g)" /app/node_modules /app/src-tauri/target /cache

echo "==> Build"
docker run --rm "${VOLUMES[@]}" -u "$(id -u):$(id -g)" "$IMAGE" bash -c '
    set -euo pipefail
    mkdir -p "$HOME"
    pnpm install --frozen-lockfile
    pnpm tauri build --bundles appimage
    mkdir -p release
    cp src-tauri/target/release/bundle/appimage/*.AppImage release/
'

echo "==> Pronto:"
ls -lh "$ROOT"/release/*.AppImage
