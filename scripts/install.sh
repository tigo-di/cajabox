#!/usr/bin/env bash
# Instala o cajabox para o usuário atual: AppImage, ícone e entrada no menu de aplicativos.
#   scripts/install.sh                    instala (ou atualiza) com o AppImage mais novo de release/
#   scripts/install.sh arquivo.AppImage   instala esse AppImage
#   scripts/install.sh --uninstall        remove o que foi instalado (a configuração é mantida)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP_ID=com.tigo.cajabox # identifier do tauri.conf.json = app_id da janela no Wayland
BIN="$HOME/.local/bin/cajabox.AppImage"
APPS="$HOME/.local/share/applications"
DESKTOP="$APPS/$APP_ID.desktop"
ICONS="$HOME/.local/share/icons/hicolor"
SIZES=(32 64 128 256)
ICON_SOURCES=(32x32.png 64x64.png 128x128.png 128x128@2x.png)

refresh_menu() {
    command -v update-desktop-database >/dev/null && update-desktop-database "$APPS" || true
    command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -q -t -f "$ICONS" || true
}

if [[ "${1:-}" == "--uninstall" ]]; then
    rm -f "$BIN" "$DESKTOP"
    for size in "${SIZES[@]}"; do rm -f "$ICONS/${size}x${size}/apps/cajabox.png"; done
    refresh_menu
    echo "cajabox removido."
    echo "A configuração continua em ~/.config/cajabox (apague a pasta se não for mais usar)."
    exit 0
fi

APPIMAGE="${1:-$(ls -t "$ROOT"/release/*.AppImage 2>/dev/null | head -1)}"
if [[ -z "$APPIMAGE" || ! -f "$APPIMAGE" ]]; then
    echo "Nenhum AppImage encontrado. Gere com scripts/build-appimage.sh ou informe o arquivo." >&2
    exit 1
fi

# Programa renomeado (manabox → cajabox): remove a instalação antiga; a configuração é migrada pelo programa
rm -f "$HOME/.local/bin/manabox.AppImage" "$APPS/com.tigo.manabox.desktop"
for size in "${SIZES[@]}"; do rm -f "$ICONS/${size}x${size}/apps/manabox.png"; done

install -Dm755 "$APPIMAGE" "$BIN"
for i in "${!SIZES[@]}"; do
    size=${SIZES[$i]}
    install -Dm644 "$ROOT/src-tauri/icons/${ICON_SOURCES[$i]}" "$ICONS/${size}x${size}/apps/cajabox.png"
done
mkdir -p "$APPS"
cat > "$DESKTOP" <<EOF
[Desktop Entry]
Type=Application
Name=cajabox
Comment=Gerenciamento de conteúdo
Exec="$BIN"
Icon=cajabox
Terminal=false
Categories=Utility;FileTools;
StartupWMClass=cajabox
EOF
refresh_menu
echo "cajabox instalado a partir de $(basename "$APPIMAGE")."
echo "Abra pelo menu de aplicativos. Para remover: scripts/install.sh --uninstall"
