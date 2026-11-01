---
periodo: 2026-11-01 a 2026-11-01
commits: 87f7e00, 5438ca9
tasks: 261101-abrir-arquivos-e-subpastas (concluída)
---

# Abrir arquivos e subpastas pela lista

## 1. Abrir arquivo
- **Solução/decisão:** duplo clique na linha ou Enter (decisão do usuário: clique simples continua selecionando, como nos gerenciadores de arquivos).
- **Problema:** `.txt`/`.md` pareciam não abrir. O Editor de Texto do GNOME (instância única) recebia o arquivo numa aba, mas a janela ficava atrás: no Wayland o shell só traz para a frente com token de ativação, que o plugin opener não entrega.
- **Solução/decisão:** abertura via GIO com o contexto de lançamento do GDK (launch.rs), que entrega o token; usada também para pastas. No AppImage, o programa aberto não herda as variáveis do runtime/plugin GTK (evita carregar bibliotecas do AppImage em programas do sistema).

## 2. Abrir subpasta
- **Solução/decisão:** nome da subpasta vira link (clique simples), sem selecionar, arrastar ou abrir o arquivo da linha.

## Resultado
- `cargo test` 40 passando; cada step verificado pelo usuário no `pnpm tauri dev`.

## Pendências
- Repositório remoto (GitHub) e push
- (Opcional) recarregar a configuração quando editada fora do programa
- Arquivos de teste nas pastas de demonstração em /tmp: recuperar antes de reiniciar o computador
