import { invoke } from "@tauri-apps/api/core";
import { ask, open } from "@tauri-apps/plugin-dialog";

import { button, el } from "./dom";
import type { State } from "./types";

type ConfigFile = "index" | "paths";

type MoveResult = { status: "moved"; state: State } | { status: "exists"; path: string };

const parentOf = (path: string) => path.slice(0, path.lastIndexOf("/")) || "/";

/** Mostra onde manabox.index.md e manabox.paths.md estão e permite movê-los; `onMoved` recebe o estado atualizado. */
export function openSettings(state: State, onMoved: (state: State) => void) {
  const error = el("p", { class: "editor-error" });
  const rows = el("div", { class: "settings-files" });

  function render(current: State) {
    const entries: [ConfigFile, string, string][] = [
      ["index", "Índice", current.config.index],
      ["paths", "Pastas", current.config.paths],
    ];
    rows.replaceChildren(
      ...entries.map(([file, label, path]) => {
        const actions = el("div", { class: "settings-actions" });
        // "Local padrão" só aparece fora de ~/.config/manabox (ou com nome antigo)
        if (path !== `${current.default_dir}/manabox.${file}.md`) {
          actions.append(button("Local padrão", () => move(file, current.default_dir)));
        }
        actions.append(button("Mover…", () => choose(file, path)));
        return el(
          "div",
          { class: "settings-file" },
          el("div", {}, el("div", { class: "settings-label" }, label), el("div", { class: "path" }, path)),
          actions,
        );
      }),
    );
  }

  async function choose(file: ConfigFile, path: string) {
    const dest = await open({ directory: true, defaultPath: parentOf(path), title: "Mover para…" });
    if (typeof dest === "string") await move(file, dest);
  }

  async function move(file: ConfigFile, dest: string) {
    error.textContent = "";
    try {
      let result = await invoke<MoveResult>("move_config_file", { file, destDir: dest, overwrite: false });
      if (result.status === "exists") {
        const replace = await ask(`Já existe ${result.path}.\nSubstituir pelo arquivo atual?`, {
          title: "Arquivo já existe",
          kind: "warning",
        });
        if (!replace) return;
        result = await invoke<MoveResult>("move_config_file", { file, destDir: dest, overwrite: true });
      }
      if (result.status === "moved") {
        render(result.state);
        onMoved(result.state);
      }
    } catch (e) {
      error.textContent = String(e);
    }
  }

  render(state);
  const dialog = el(
    "dialog",
    { class: "editor settings" },
    el("h2", {}, "Configurações"),
    el("p", { class: "hint" }, "Arquivos onde o programa guarda o índice e as pastas. Ao mover, o nome é sempre manabox.index.md / manabox.paths.md."),
    rows,
    error,
    el("div", { class: "editor-actions" }, button("Fechar", () => dialog.close(), "primary")),
  );
  dialog.addEventListener("close", () => dialog.remove());
  document.body.append(dialog);
  dialog.showModal();
}
