# manabox

Content management (news, videos, stories, etc.) for Ubuntu. · [Português](README.pt-BR.md)

manabox lists the files in registered folders and lets you filter them by the terms and dates in their names, according to an index you edit inside the program. It also renames, copies, cuts, pastes and moves files to the trash, finds duplicates, and exchanges files with any file manager (clipboard and drag and drop).

## Naming convention

- File names are split into terms by `-`, `_` or space: `news-promo-black-friday-241129.png`.
- Terms are looked up in the index, organized in sections (e.g. Channels, Campaigns), with synonyms.
- The date of use goes in the name as `YYMMDD` or `YYMM` (`241129` = 2024-11-29). Without a date in the name, the file's modification date is used.

## Install

1. Build the AppImage (requires Docker): `scripts/build-appimage.sh` — the file goes to `release/`.
2. Install it for your user: `scripts/install.sh` — copies the AppImage to `~/.local/bin` and creates the application menu entry.

To update, build and install again. To remove: `scripts/install.sh --uninstall` (the configuration in `~/.config/manabox` is kept).

The AppImage runs on Ubuntu 22.04 or newer and needs the `libfuse2` package.

On the first run, manabox asks for the language (Portuguese or English) and creates example folders in your Documents folder (`manabox-examples`, or `manabox-exemplo` in Portuguese) to show how it works (files with `.example` in the name). When you no longer need them, delete the folder and remove the example folders under Folders › Edit. To recreate them: Settings → "Create example folders".

## Development

- `pnpm tauri dev` — runs the app
- `cd src-tauri && cargo test` — core tests
- `pnpm build` — type checking and interface build

Specification and code map (in Portuguese) in [CLAUDE.md](CLAUDE.md).

## Author and license

© 2026 Tiago Dias — tigo@webdeleve.net

Free software under the [GNU General Public License v3.0 or later](LICENSE): it may be used, modified, distributed and sold; derived versions must keep the credits and the same license.
