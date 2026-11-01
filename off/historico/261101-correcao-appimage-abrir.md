---
periodo: 2026-11-01 a 2026-11-01
commits: (ver tag v0.6.1)
tasks: correção fora de task (branch fix/appimage-abrir-arquivos)
---

# Correção: abrir arquivos pelo AppImage

## 1. Programas abertos travavam
- **Problema:** no AppImage, imagens e textos não abriam. O registro do sistema mostrava eog e gnome-text-editor falhando com "symbol lookup error": herdavam `LD_LIBRARY_PATH` (definida pelo AppRun do AppImage, além das variáveis do plugin GTK já tratadas) e carregavam bibliotecas antigas de dentro do AppImage.
- **Solução/decisão:** launch.rs limpa o ambiente do programa aberto de forma genérica: toda variável que aponta para dentro do AppImage perde essas entradas (some se ficar vazia); variáveis próprias do AppImage e `GTK_THEME` são removidas. Lista fixa descartada porque o AppRun define mais variáveis do que o plugin GTK (PATH, PYTHONHOME, QT_PLUGIN_PATH, GStreamer…).
- **Observação:** no `pnpm tauri dev`, as "imagens" que abriam no Editor de Texto eram arquivos vazios criados para testes (o sistema identifica pelo conteúdo); não era defeito.
- **Resultado/testes:** teste de `clean_env`; usuário testou imagens, texto e pastas pelo AppImage instalado.
