# cajabox

Gerenciamento de conteúdo (news, vídeos, stories etc.) para Ubuntu. · [English](README.md)

O cajabox lista os arquivos de pastas cadastradas e permite filtrá-los pelos termos e datas presentes nos nomes, conforme um índice editável dentro do programa. Também renomeia, copia, recorta, cola e envia arquivos para a lixeira, encontra duplicados e troca arquivos com qualquer gerenciador de arquivos (área de transferência e arrastar e soltar).

## Convenção de nomes

- O nome do arquivo é dividido em termos por `-`, `_` ou espaço: `news-promo-black-friday-241129.png`.
- Os termos são procurados no índice, organizado em seções (ex.: Canais, Campanhas), com sinônimos.
- A data de uso vai no nome como `AAMMDD` ou `AAMM` (`241129` = 29/11/2024). Sem data no nome, vale a data de modificação do arquivo.

## Instalar

1. Gere o AppImage (precisa de Docker): `scripts/build-appimage.sh` — o arquivo fica em `release/`.
2. Instale para o seu usuário: `scripts/install.sh` — copia o AppImage para `~/.local/bin` e cria a entrada no menu de aplicativos.

Para atualizar, gere e instale de novo. Para remover: `scripts/install.sh --uninstall` (a configuração em `~/.config/cajabox` é mantida).

O AppImage roda em Ubuntu 22.04 ou mais novo e precisa do pacote `libfuse2`.

Na primeira execução, o programa pergunta o idioma (português ou inglês) e cria pastas de exemplo na sua pasta de Documentos (`cajabox-exemplo`, ou `cajabox-examples` em inglês) para mostrar como funciona (arquivos com `.example` no nome). Quando não precisar mais, apague a pasta e remova as pastas de exemplo em "Editar pastas". Para recriá-las: Configurações → "Criar pastas de exemplo".

## Desenvolvimento

- `pnpm tauri dev` — roda o app
- `cd src-tauri && cargo test` — testes do núcleo
- `pnpm build` — checagem de tipos e build da interface

Detalhes da especificação em [CLAUDE.md](CLAUDE.md).

## Autoria e licença

© 2026 Tiago Dias — tigo@webdeleve.net

Software livre sob a [GNU General Public License v3.0 ou posterior](LICENSE): pode ser usado, alterado, distribuído e vendido; versões derivadas devem manter os créditos e a mesma licença.
