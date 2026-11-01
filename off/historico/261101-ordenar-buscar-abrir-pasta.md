---
periodo: 2026-11-01 a 2026-11-01
commits: ffecec0, 328a4a4, 474046a
tasks: 261101-ordenar-buscar-abrir-pasta (concluída)
---

# Ordenar, buscar por nome e abrir a pasta

## 1. Ordenação
- **Solução/decisão:** clique nos títulos "Nome" e "Data"; clicar de novo inverte; escolha lembrada. Por decisão do usuário, data = data do nome do arquivo (a mesma dos filtros) e nome = só o nome do arquivo (sem subpasta). Ordem natural ("ep2" < "ep10") e sem diferenciar acentos; sem data sempre no fim; data começa pelos mais recentes.
- **Ajuste:** Shift+clique mostrava por um instante a seleção de texto nativa se espalhando pela página; bloqueada no `mousedown`.

## 2. Busca por nome
- **Solução/decisão:** campo fixo acima da lista (não é redesenhado, o foco não se perde); todas as palavras no nome, sem diferenciar maiúsculas e acentos; soma-se aos filtros. Ctrl+F foca, Esc limpa.

## 3. Abrir a pasta
- **Solução/decisão:** caminho no cabeçalho vira link; abertura por comando Rust próprio em vez de liberar abertura de caminhos quaisquer para a interface.

## Resultado
- Cada step verificado pelo usuário no `pnpm tauri dev`; detalhes em off/concluidos/steps/261101-ordenar-buscar-abrir-pasta/.

## Pendências
- Dockerfile e script de build do AppImage
- Trocar as pastas de demonstração (em /tmp) pelas pastas reais
- Repositório remoto (GitHub) e push
