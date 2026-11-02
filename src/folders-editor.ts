import { open } from "@tauri-apps/plugin-dialog";

import { button, el } from "./dom";
import { t } from "./i18n";
import { notSaved, saveConfig } from "./save-config";
import type { Folder, State } from "./types";

/** Abre o editor de pastas (cajabox.paths.md); `onSaved` recebe o estado já gravado. */
export function openFoldersEditor(folders: Folder[], version: string, onSaved: (state: State) => void) {
  const rows = el("div", { class: "editor-rows" });
  const error = el("p", { class: "editor-error" });

  function addRow(folder: Folder) {
    const name = el("input", { placeholder: t("folders.namePlaceholder"), value: folder.name, "aria-label": t("folders.name") });
    const description = el("input", { placeholder: t("folders.descriptionPlaceholder"), value: folder.description, "aria-label": t("folders.description") });
    const path = el("input", { placeholder: t("folders.pathPlaceholder"), value: folder.path, "aria-label": t("folders.path") });
    const row = el("div", { class: "editor-row folder-row" }, name, description, path);
    row.append(
      button(t("folders.choose"), async () => {
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
        error.textContent = notSaved();
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
    el("h2", {}, t("folders.title")),
    el("p", { class: "hint" }, t("folders.hint")),
    el("div", { class: "editor-row folder-row editor-head" }, el("span", {}, t("folders.name")), el("span", {}, t("folders.description")), el("span", {}, t("folders.path"))),
    rows,
    button(t("folders.add"), () => addRow({ name: "", description: "", path: "" }).focus(), "link"),
    error,
    el("div", { class: "editor-actions" }, button(t("common.cancel"), () => dialog.close()), button(t("common.save"), save, "primary")),
  );
  dialog.addEventListener("close", () => dialog.remove());
  document.body.append(dialog);
  if (folders.length === 0) addRow({ name: "", description: "", path: "" });
  dialog.showModal();
}
