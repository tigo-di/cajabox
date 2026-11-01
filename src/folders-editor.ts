import { open } from "@tauri-apps/plugin-dialog";

import { button, el } from "./dom";
import { NOT_SAVED, saveConfig } from "./save-config";
import type { Folder, State } from "./types";

/** Abre o editor de pastas (manabox.paths.md); `onSaved` recebe o estado já gravado. */
export function openFoldersEditor(folders: Folder[], version: string, onSaved: (state: State) => void) {
  const rows = el("div", { class: "editor-rows" });
  const error = el("p", { class: "editor-error" });

  function addRow(folder: Folder) {
    const name = el("input", { placeholder: "nome", value: folder.name, "aria-label": "Nome" });
    const description = el("input", { placeholder: "descrição", value: folder.description, "aria-label": "Descrição" });
    const path = el("input", { placeholder: "/caminho/absoluto", value: folder.path, "aria-label": "Caminho" });
    const row = el("div", { class: "editor-row folder-row" }, name, description, path);
    row.append(
      button("Escolher…", async () => {
        const chosen = await open({ directory: true, defaultPath: path.value || undefined });
        if (typeof chosen === "string") {
          path.value = chosen;
          if (!name.value) name.value = chosen.split("/").filter(Boolean).pop() ?? "";
        }
      }),
      button("✕", () => row.remove(), "icon"),
    );
    rows.append(row);
    return name;
  }

  function collect(): Folder[] {
    return [...rows.querySelectorAll(".folder-row")]
      .map((row) => {
        const [name, description, path] = [...row.querySelectorAll("input")].map((i) => i.value.trim());
        return { name, description, path };
      })
      .filter((f) => f.name || f.description || f.path);
  }

  async function save() {
    error.textContent = "";
    try {
      const state = await saveConfig("save_folders", { folders: collect() }, version);
      if (!state) {
        error.textContent = NOT_SAVED;
        return;
      }
      dialog.close();
      onSaved(state);
    } catch (e) {
      error.textContent = String(e);
    }
  }

  folders.forEach(addRow);
  const dialog = el(
    "dialog",
    { class: "editor" },
    el("h2", {}, "Pastas"),
    el("p", { class: "hint" }, "Pastas listadas pelo programa. O caminho é lido recursivamente."),
    el("div", { class: "editor-row folder-row editor-head" }, el("span", {}, "Nome"), el("span", {}, "Descrição"), el("span", {}, "Caminho")),
    rows,
    button("+ Adicionar pasta", () => addRow({ name: "", description: "", path: "" }).focus(), "link"),
    error,
    el("div", { class: "editor-actions" }, button("Cancelar", () => dialog.close()), button("Salvar", save, "primary")),
  );
  dialog.addEventListener("close", () => dialog.remove());
  document.body.append(dialog);
  if (folders.length === 0) addRow({ name: "", description: "", path: "" });
  dialog.showModal();
}
