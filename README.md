# manabox

Gerenciamento de conteúdo (news, vídeos, stories etc.) para Ubuntu.

Lista os arquivos de pastas cadastradas e permite filtrá-los pelos termos e datas presentes nos nomes, conforme um índice editável dentro do programa. Também renomeia, recorta, cola e envia arquivos para a lixeira.

## Instalar

1. Gere o AppImage (precisa de Docker): `scripts/build-appimage.sh` — o arquivo fica em `release/`.
2. Instale para o seu usuário: `scripts/install.sh` — copia o AppImage para `~/.local/bin` e cria a entrada no menu de aplicativos.

Para atualizar, gere e instale de novo. Para remover: `scripts/install.sh --uninstall` (a configuração em `~/.config/manabox` é mantida).

O AppImage roda em Ubuntu 22.04 ou mais novo e precisa do pacote `libfuse2`.

## Desenvolvimento

- `pnpm tauri dev` — roda o app
- `cd src-tauri && cargo test` — testes do núcleo
- `pnpm build` — checagem de tipos e build da interface

Detalhes da especificação em [CLAUDE.md](CLAUDE.md).

## Autoria e licença

© 2026 Tiago Dias — tigo@webdeleve.net

Software livre sob a [GNU General Public License v3.0 ou posterior](LICENSE): pode ser usado, alterado, distribuído e vendido; versões derivadas devem manter os créditos e a mesma licença.
