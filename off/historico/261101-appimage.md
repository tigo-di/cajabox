---
periodo: 2026-11-01 a 2026-11-01
commits: 8fd19b3, 0945011, c30d7ef, 62bd1e7
tasks: 261101-appimage (concluída)
---

# AppImage

## 1. Versão
- **Problema:** as tags (até v0.3.0) não atualizavam a versão nos arquivos; o "Sobre" mostrava 0.1.0.
- **Solução/decisão:** package.json como fonte (tauri.conf.json lê dele), Cargo.toml acompanha; regra na skill: commit "Versão X.Y.Z" antes de cada tag. Tag v0.3.0 não foi movida (seria reescrita de histórico).

## 2. Ícone
- **Solução/decisão:** baú original em pixel art 16×16, com musgo no canto inferior e fecho dourado (ajustes do usuário). Gerado por scripts/icon.py, com PNGs ampliados sem suavização para manter os pixels nítidos.
- **Observação:** no Wayland (GNOME) o ícone da barra vem da entrada .desktop, por isso só apareceu após a instalação.

## 3. Build e instalação
- **Solução/decisão:** build em Docker com Ubuntu 22.04 (scripts/build-appimage.sh), rodando com o usuário do host e com node_modules, target e caches em volumes próprios, para não misturar com os do host (glibc diferente). Instalação por usuário com scripts/install.sh (menu, ícone, `--uninstall`); a entrada usa o nome com.tigo.manabox.desktop, igual ao app_id da janela no Wayland.
- **Resultado/testes:** 1º build ~6 min, seguintes ~2 min; AppImage de 78 MB exige no máximo GLIBC_2.35; usuário testou o AppImage instalado.

## 4. Edição manual da configuração
- **Problema:** editar manabox.index.md/paths.md com o programa aberto deixa os filtros desencontrados, e o editor interno pode sobrescrever a edição.
- **Decisão:** o usuário edita com o programa fechado; sem mudança no código por enquanto.

## Pendências
- Trocar as pastas de demonstração (em /tmp) pelas pastas reais
- Repositório remoto (GitHub) e push
- (Opcional) recarregar a configuração quando editada fora do programa
