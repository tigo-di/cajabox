---
periodo: 2026-10-31 a 2026-10-31
commits: (nenhum ainda)
tasks: 261031-interface (planejada)
---

# Especificação e núcleo de classificação

## 1. Regras de classificação
- **Problema:** especificação inicial não definia como casar termos, combinar filtros e ler datas.
- **Solução/decisão:** tokens separados por `-` ou espaço, case insensitive, só tokens inteiros (`news` ≠ `newsletter`); termo composto (`mala-direta`) tem preferência sobre as partes. Filtros: E dentro da seção e entre seções (um arquivo pode servir a vários canais). Datas YYMMDD/YYMM validadas; duas datas = sem data. Detalhes no CLAUDE.md.
- **Resultado/testes:** src-tauri/src/classify.rs, 7 testes.

## 2. Tecnologia
- **Problema:** stack para app portátil no Ubuntu.
- **Solução/decisão:** Tauri (leve, multiplataforma, seguro). Desenvolvimento no host; AppImage gerado em Docker com Ubuntu 22.04 (glibc antiga = roda em mais sistemas). GUI dentro do container daria atrito com X11/GPU.
- **Resultado/testes:** esqueleto Tauri vanilla-ts + pnpm.

## 3. Núcleo
- **Solução/decisão:** leitura de index.md/paths.md ignorando `<!-- -->`; listagem recursiva sem ocultos e sem seguir links simbólicos de pastas; `~/.config/manabox/config` aponta para index.md e paths.md, criados a partir dos samples se não existirem.
- **Resultado/testes:** `cargo test` — 18 passando.

## Pendências
- Interface (task 261031-interface)
- Dockerfile e script de build do AppImage
