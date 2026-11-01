Gerenciamento de conteúdo (news, videos, instagram stories, etc).

# Objetivo
Listar e permitir filtrar arquivos de pastas (manabox.paths.md) de acordo com classificações baseadas em termos dos nomes de arquivos (manabox.index.md).

# Tecnologia
- Tauri (backend Rust, interface HTML/JS)
- Sistema operacional: Ubuntu; distribuído como AppImage
- Licença: GPL-3.0-or-later (LICENSE); autor Tiago Dias <tiago@webdeleve.net>
- Desenvolvimento no host (Rust + libs do Tauri instalados localmente)
- Build do AppImage dentro de container Docker com Ubuntu 22.04 (glibc antiga = AppImage compatível com mais sistemas)

## Comandos
- `pnpm tauri dev` — roda o app em modo desenvolvimento
- `cd src-tauri && cargo test` — testes do núcleo
- `pnpm build` — checagem de tipos e build do frontend
- Node 22.12.0 fixado no projeto via asdf (`.tool-versions`); Vite 8 exige Node ≥ 20.19

## Código (src-tauri/src)
- `index.rs` / `paths.rs` — leitura, validação e gravação de manabox.index.md / manabox.paths.md
- `classify.rs` — termos e datas a partir do nome do arquivo
- `scan.rs` — listagem recursiva de uma pasta
- `config.rs` — localização de manabox.index.md e manabox.paths.md (mover, local padrão)
- `clipboard.rs` — arquivos na área de transferência do sistema (GTK)
- `fileops.rs` — renomear e mover arquivos
- `watch.rs` — observa a pasta aberta e avisa a interface
- `lib.rs` — comandos Tauri expostos à interface

## Interface (src)
- `main.ts` — pastas, filtros, listagem, seleção, operações e atalhos
- `filter.ts` — lógica pura dos filtros
- `folders-editor.ts` / `index-editor.ts` / `settings.ts` — diálogos de edição e configurações
- `about.ts` — diálogo "Sobre" (créditos, versão, licença)
- `dom.ts` — helpers de DOM (textos sempre via textContent)
- `types.ts` — tipos espelhando as structs do Rust

# Arquivos de configuração
Arquivos de exemplo: index-sample.md, paths-sample.md

O programa gera manabox.index.md e manabox.paths.md (usando os samples como modelo) se não existirem.
- Local padrão: `~/.config/manabox/`
- `~/.config/manabox/config` guarda a localização atual de manabox.index.md e manabox.paths.md
- O programa mostra onde eles estão salvos e permite movê-los
- São editados dentro do programa; o usuário não precisa abrir os arquivos
- Blocos `<!-- -->` são comentários e ignorados na leitura

## manabox.paths.md
Uma pasta por linha: `nome|descrição|caminho absoluto`
- Cada listagem mostra uma pasta por vez: nome, descrição e arquivos
- Leitura recursiva (inclui subpastas)
- Arquivos e pastas ocultos (iniciados por `.`) não são mostrados
- Caminho inexistente: mostrar aviso de que não foi possível encontrar o caminho

## manabox.index.md
- `# Nome` inicia uma seção; linhas seguintes são termos da seção
- Espaços, tabs e linhas vazias são ignorados (só servem para legibilidade)
- Termo seguido de linhas `- sinônimo` forma um grupo: qualquer sinônimo classifica o arquivo no termo pai. O pai é sinônimo de si mesmo (não precisa ser repetido como filho).

# Classificação
## Termos
- Nome do arquivo (sem extensão) é dividido em tokens por `-` ou espaço
- Comparação case insensitive
- Um termo casa somente com tokens inteiros (`news` ≠ `newsletter`)
- Termos compostos (`mala-direta`) casam com a sequência de tokens; o termo mais longo tem preferência: tokens consumidos por `mala-direta` não casam com `mala` ou `direta`
- Um arquivo pode ter vários termos da mesma seção

## Datas
- Token de 6 dígitos `YYMMDD` ou 4 dígitos `YYMM` (sempre separado por `-` ou espaço)
- Ano = 20YY; mês 01–12 e dia 01–31 validados, senão o token não é data
- Mais de uma data no nome: arquivo considerado sem data

# Filtros
- Termos selecionados na mesma seção: E (arquivo precisa ter todos)
- Entre seções: E
- Datas: filtro por ano e mês

# Operações com arquivos
- Copiar, recortar, colar, excluir, renomear arquivos das pastas listadas
- Cópia pode ser colada várias vezes; recorte, uma vez
- Copiar/recortar/colar usam a área de transferência do sistema (troca com qualquer gerenciador de arquivos): `x-special/gnome-copied-files` (Nautilus, Nemo, Caja, Thunar), `text/uri-list` + `application/x-kde-cutselection` (Dolphin) e texto com os caminhos; pastas são ignoradas com aviso
- Excluir envia para a lixeira (não apaga definitivamente)
- Colar sobre arquivo de mesmo nome: perguntar se quer substituir
- Após renomear/mover, a classificação é recalculada
