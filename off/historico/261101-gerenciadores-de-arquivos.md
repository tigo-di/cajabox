---
periodo: 2026-10-31 a 2026-11-01
commits: 7f55cae, 2ef53f9, 983323b, 25ac7c4, 97c9565, 5ed47db, 89dd29d, 920c260
tasks: 261031-fechar-sobre-gerenciadores (concluída)
---

# Fechar, Sobre e integração com gerenciadores de arquivos

## 1. Versionamento
- **Solução/decisão:** commit inicial com tudo até então (tag v0.1.0); a partir daí, um branch por task, um commit por step após verificação do usuário, merge `--no-ff` em main ao concluir e tag por release. Regras gravadas na skill de tasks.

## 2. Fechar e Sobre
- **Solução/decisão:** botão "Fechar" e Ctrl+Q (funciona mesmo com diálogo aberto); diálogo "Sobre" com versão, autoria (Tiago Dias, tiago@webdeleve.net) e licença.
- **Licença:** GPL-3.0-or-later, que corresponde ao pedido: qualquer um pode alterar, distribuir e vender; derivados mantêm créditos e a mesma licença (ninguém fecha o código).

## 3. Atualização contínua (bug do observador de pasta)
- **Problema:** cliques em filtros se perdiam. O observador reagia aos eventos de leitura gerados pela própria listagem, em ciclo a cada ~300 ms.
- **Solução/decisão:** `notify` direto no lugar de `notify-debouncer-mini`, ignorando eventos de acesso; teste que garante que leitura não dispara atualização.

## 4. Troca de arquivos com gerenciadores
- **Problema:** o usuário usa qualquer gerenciador do Ubuntu, não só o Nautilus.
- **Solução/decisão:** área de transferência do sistema como fonte única, em todos os formatos padrão (`x-special/gnome-copied-files`, `text/uri-list` + `application/x-kde-cutselection`, texto), via GTK, que o Tauri já usa e funciona em X11 e Wayland. Soltar na janela usa o recurso nativo do Tauri e pergunta se copia ou move. Pastas são ignoradas com aviso.
- **Arrastar para fora:** o plugin `tauri-plugin-drag` foi descartado (só uma ação fixa e URIs sem codificação); arrasto feito direto pelo GTK, oferecendo copiar e mover para o programa que recebe decidir.
- **Resultado/testes:** `cargo test` 39 passando; usuário testou com o Nautilus no Wayland nos dois sentidos.

## Pendências
- Dockerfile e script de build do AppImage
- Trocar as pastas de demonstração (em /tmp) pelas pastas reais
- Repositório remoto (GitHub) e push
- Testar a troca de arquivos em outro gerenciador (Dolphin, Thunar), se houver acesso
