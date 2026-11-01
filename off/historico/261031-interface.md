---
periodo: 2026-10-31 a 2026-10-31
commits: (nenhum ainda)
tasks: 261031-interface (concluída)
---

# Interface do manabox

## 1. Ambiente
- **Problema:** Vite 8 (vindo do template Tauri) exige Node ≥ 20.19; sistema tinha 20.15.1.
- **Solução/decisão:** Node 22.12.0 via asdf só no projeto (`.tool-versions`), em vez de rebaixar para Vite 6 (problema voltaria em atualizações).

## 2. Edição dos arquivos de configuração
- **Solução/decisão:** pastas e índice editados em diálogos; Rust valida e grava mantendo o comentário de ajuda. Dentro de uma seção nenhum termo/sinônimo se repete (antes, sinônimo repetido era descartado sem aviso). Remover seção pede confirmação.
- **Resultado/testes:** serialize → parse preserva conteúdo; casos de validação cobertos.

## 3. Nome e local dos arquivos
- **Problema:** `index.md`/`paths.md` não identificam o programa quando salvos numa pasta qualquer; seletor do sistema esconde `~/.config`.
- **Solução/decisão:** renomeados para `manabox.index.md`/`manabox.paths.md` (mover aplica o nome novo); botão "Local padrão" volta para `~/.config/manabox`. Mover o arquivo antes de gravar a config e desfazer se a gravação falhar (config nunca aponta para arquivo inexistente).

## 4. Operações com arquivos
- **Solução/decisão:** renomear, recortar/colar (recorte persiste entre pastas), excluir para a lixeira (crate `trash`). Conflito ao colar pergunta por arquivo. Nome iniciado por `.` recusado (sumiria da lista). Ações valem só para arquivos visíveis no filtro.

## 5. Atualização automática
- **Solução/decisão:** observa só a pasta aberta, com debounce de 300 ms, ignorando caminhos ocultos.

## Resultado
- `cargo test`: 32 passando. Cada step verificado pelo usuário no `pnpm tauri dev`; detalhes em off/concluidos/steps/261031-interface/.

## Pendências
- Dockerfile e script de build do AppImage
- Trocar as pastas de demonstração (em /tmp) pelas pastas reais
- Primeiro commit
- Função de fechar (pedido do usuário; a definir: botão/atalho Ctrl+Q para sair do programa?)
- Local para os créditos de autoria do programa (ex.: diálogo "Sobre")
- Integração com o Nautilus: colar no manabox arquivos copiados/recortados no Nautilus (e vice-versa); arrastar e soltar; operação "copiar" (hoje só há recortar)
