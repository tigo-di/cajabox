Gerenciamento de conteúdo (news, videos, instagram stories, etc).

# Objetivo
Listar e permitir filtrar arquivos de pastas (cajabox.paths.md) de acordo com classificações baseadas em termos dos nomes de arquivos (cajabox.index.md).

# Tecnologia
- Tauri (backend Rust, interface HTML/JS)
- Sistema operacional: Ubuntu; distribuído como AppImage
- Ícone: `python3 scripts/icon.py` (baú em pixel art; ver instruções no script)
- Versão: `package.json` (lida pelo tauri.conf.json); `src-tauri/Cargo.toml` acompanha
- Idiomas: português e inglês, escolhidos na primeira execução (idioma do sistema pré-selecionado) ou em Configurações; gravado em `~/.config/cajabox/config` (`language=`). **Todo texto novo vai nos dois idiomas**: interface em `src/i18n.ts` (`t()`), núcleo com `tr!("pt", "en")`, modelos `index-sample(.en).md`/`paths-sample(.en).md`, exemplos em `src-tauri/assets/examples/{pt,en}`. Datas: pt `29/11/2024`, en ISO `2024-11-29`
- README.md em inglês, README.pt-BR.md em português (manter os dois)
- Licença: GPL-3.0-or-later (LICENSE); autor Tiago Dias <tigo@webdeleve.net>
- Desenvolvimento no host (Rust + libs do Tauri instalados localmente)
- Build do AppImage dentro de container Docker com Ubuntu 22.04 (glibc antiga = AppImage compatível com mais sistemas)

## Comandos
- `pnpm tauri dev` — roda o app em modo desenvolvimento
- `cd src-tauri && cargo test` — testes do núcleo
- `pnpm build` — checagem de tipos e build do frontend
- `scripts/build-appimage.sh` — gera o AppImage no Docker (Ubuntu 22.04) em `release/`
- `scripts/install.sh` — instala o AppImage no menu do usuário; `--uninstall` remove
- Node 22.12.0 fixado no projeto via asdf (`.tool-versions`); Vite 8 exige Node ≥ 20.19

## Código (src-tauri/src)
- `index.rs` / `paths.rs` — leitura, validação e gravação de cajabox.index.md / cajabox.paths.md
- `classify.rs` — termos e datas a partir do nome do arquivo
- `scan.rs` — listagem recursiva de uma pasta
- `config.rs` — localização de cajabox.index.md e cajabox.paths.md (mover, local padrão)
- `clipboard.rs` — arquivos na área de transferência do sistema (GTK)
- `launch.rs` — abrir arquivo/pasta com o programa padrão (GIO, traz a janela para a frente)
- `examples.rs` — pastas de exemplo (primeira execução e Configurações)
- `fileops.rs` — renomear e mover arquivos
- `watch.rs` — observa a pasta aberta e avisa a interface
- `i18n.rs` — idioma atual e macro `tr!`
- `lib.rs` — comandos Tauri expostos à interface

## Interface (src)
- `main.ts` — pastas, filtros, listagem, seleção, operações e atalhos
- `filter.ts` — lógica pura dos filtros
- `sort.ts` — ordenação da lista (nome/data)
- `folders-editor.ts` / `index-editor.ts` / `settings.ts` — diálogos de edição e configurações
- `save-config.ts` — salvar pelos editores com aviso se o arquivo mudou fora do programa
- `about.ts` — diálogo "Sobre" (créditos, versão, licença)
- `i18n.ts` — textos pt/en, `t()`, formato de datas e nomes dos meses
- `dom.ts` — helpers de DOM (textos sempre via textContent)
- `types.ts` — tipos espelhando as structs do Rust

# Arquivos de configuração
Arquivos de exemplo: index-sample.md, paths-sample.md

O programa gera cajabox.index.md e cajabox.paths.md (usando os samples como modelo) se não existirem.
- Local padrão: `~/.config/cajabox/`
- Nome anterior do programa: manabox. Na abertura, `~/.config/manabox` é movida para `~/.config/cajabox` e `manabox.*.md` renomeados para `cajabox.*.md` (config.rs `migrate_old_name`); `scripts/install.sh` remove a instalação antiga
- `~/.config/cajabox/config` guarda a localização atual de cajabox.index.md e cajabox.paths.md
- O programa mostra onde eles estão salvos e permite movê-los
- São editados dentro do programa; o usuário não precisa abrir os arquivos
- Primeira execução (sem índice nem pastas): cria ~/Documentos/cajabox-exemplo/{Prontos,Publicados} com arquivos `.example` e os termos no índice (examples.rs; arquivos em src-tauri/assets/exemplo, gerados por scripts/example-assets.py); "Criar pastas de exemplo" em Configurações faz o mesmo; nunca sobrescreve
- Editados fora do programa (com ele aberto): recarregados sozinhos; pasta aberta e filtros marcados (pelo nome) mantidos; salvar pelo editor interno depois disso pergunta antes de substituir
- Blocos `<!-- -->` são comentários e ignorados na leitura

## cajabox.paths.md
Uma pasta por linha: `nome|descrição|caminho absoluto`
- Cada listagem mostra uma pasta por vez: nome, descrição e arquivos
- Leitura recursiva (inclui subpastas)
- Arquivos e pastas ocultos (iniciados por `.`) não são mostrados
- Caminho inexistente: mostrar aviso de que não foi possível encontrar o caminho

## cajabox.index.md
- `# Nome` inicia uma seção; linhas seguintes são termos da seção
- Espaços, tabs e linhas vazias são ignorados (só servem para legibilidade)
- Termo seguido de linhas `- sinônimo` forma um grupo: qualquer sinônimo classifica o arquivo no termo pai. O pai é sinônimo de si mesmo (não precisa ser repetido como filho).

# Classificação
## Termos
- Nome do arquivo (sem extensão) é dividido em tokens por `-`, `_` ou espaço
- Comparação case insensitive
- Um termo casa somente com tokens inteiros (`news` ≠ `newsletter`)
- Termos compostos (`mala-direta`) casam com a sequência de tokens; o termo mais longo tem preferência: tokens consumidos por `mala-direta` não casam com `mala` ou `direta`
- Um arquivo pode ter vários termos da mesma seção

## Datas
- Token de 6 dígitos `YYMMDD` ou 4 dígitos `YYMM` (sempre separado por `-`, `_` ou espaço), opcionalmente com `v` na frente (`v241031`, de versão); a lista mostra só a data
- Ano = 20YY; mês 01–12 e dia 01–31 validados, senão o token não é data
- Mais de uma data no nome: arquivo considerado sem data no nome
- Sem data no nome: vale a data de modificação do arquivo (em cinza/itálico na lista); filtros e ordenação usam essa data efetiva; filtro "Sem data no nome" lista esses arquivos
- Copiar e mover (inclusive entre discos) mantêm a data de modificação
- `.example` antes da extensão é ignorado (arquivos de exemplo: `nome-241129.example.png`)

# Filtros
- Termos selecionados na mesma seção: E (arquivo precisa ter todos)
- Entre seções: E
- Datas: filtro por ano e mês
- Busca: palavras no nome do arquivo (todas), sem diferenciar maiúsculas e acentos; soma-se aos filtros

# Ordenação
- Clique no título "Nome" ou "Data"; clicar de novo inverte; escolha lembrada
- Nome: só o nome do arquivo, ordem natural (ep2 < ep10), sem diferenciar maiúsculas e acentos
- Data: a data efetiva (nome ou, sem ela, modificação do arquivo); começa pelos mais recentes; empate pelo nome

# Operações com arquivos
- Abrir arquivo com o programa padrão: duplo clique na linha ou Enter (selecionados)
- Copiar, recortar, colar, excluir, renomear arquivos das pastas listadas
- Cópia pode ser colada várias vezes; recorte, uma vez
- Copiar/recortar/colar usam a área de transferência do sistema (troca com qualquer gerenciador de arquivos): `x-special/gnome-copied-files` (Nautilus, Nemo, Caja, Thunar), `text/uri-list` + `application/x-kde-cutselection` (Dolphin) e texto com os caminhos; pastas são ignoradas com aviso
- Excluir envia para a lixeira (não apaga definitivamente)
- Colar sobre arquivo de mesmo nome: perguntar se quer substituir
- Após renomear/mover, a classificação é recalculada

# Duplicados
- Modo na barra de ferramentas: por nome (sem diferenciar maiúsculas) ou por conteúdo, na pasta aberta com subpastas
- Conteúdo: tamanho → amostra início/fim (64 kB) → BLAKE3 completo; cache em memória; vazios ignorados; progresso e cancelar
- No modo: grupos com título e coluna Tamanho; filtros, busca e ordenação suspensos; operações normais
