# cajabox

[Português](README.pt-BR.md)

cajabox lists the files in registered folders and lets you filter them by the terms and dates in their names, according to an index you edit inside the program. It also renames, copies, cuts, pastes and moves files to the trash, finds duplicates, and exchanges files with any file manager (clipboard and drag and drop).

## Naming convention

- File names are split into terms by `-`, `_` or space: `news-promo-black-friday-241129.png`.
- Terms are looked up in the index, organized in sections (e.g. Channels, Campaigns), with synonyms.
- The date of use goes in the name as `YYMMDD` or `YYMM` (`241129` = 2024-11-29). Without a date in the name, the file's modification date is used.

## Requirements

- Linux with a graphical desktop (tested on Ubuntu 22.04 and 24.04, with GNOME)
- The `libfuse2` package, to run the AppImage
- Docker, only to build the AppImage from source

## Install

Download `cajabox_X.Y.Z_amd64.AppImage` from the [Releases](https://github.com/tigo-di/cajabox/releases) page, make it executable (`chmod +x`) and run it.

To add it to the application menu, from a clone of this repository:

1. Put the AppImage in `release/` — or build it from source (requires Docker): `scripts/build-appimage.sh`.
2. Install it for your user: `scripts/install.sh` — copies the AppImage to `~/.local/bin` and creates the application menu entry.

To update, install the new AppImage the same way. To remove: `scripts/install.sh --uninstall` (the configuration in `~/.config/cajabox` is kept).

On the first run, cajabox asks for the language (Portuguese or English) and creates example folders in your Documents folder (`cajabox-examples`, or `cajabox-exemplo` in Portuguese) to show how it works (files with `.example` in the name). When you no longer need them, delete the folder and remove the example folders under Folders › Edit. To recreate them: Settings → "Create example folders".

## Development

- `pnpm tauri dev` — runs the app
- `cd src-tauri && cargo test` — core tests
- `pnpm build` — type checking and interface build

Specification and code map (in Portuguese) in [CLAUDE.md](CLAUDE.md).

## Author and license

© 2026 Tiago Dias — tigo@webdeleve.net

Free software under the [GNU General Public License v3.0 or later](LICENSE): it may be used, modified, distributed and sold; derived versions must keep the credits and the same license.
