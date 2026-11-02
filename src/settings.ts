import { invoke } from "@tauri-apps/api/core";
import { ask, message, open } from "@tauri-apps/plugin-dialog";

import { button, el } from "./dom";
import { currentLang, t } from "./i18n";
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
      ["index", t("settings.index"), current.config.index],
      ["paths", t("settings.folders"), current.config.paths],
    ];
    rows.replaceChildren(
      ...entries.map(([file, label, path]) => {
        const actions = el("div", { class: "settings-actions" });
        // "Local padrão" só aparece fora de ~/.config/manabox (ou com nome antigo)
        if (path !== `${current.default_dir}/manabox.${file}.md`) {
          actions.append(button(t("settings.defaultLocation"), () => move(file, current.default_dir)));
        }
        actions.append(button(t("settings.move"), () => choose(file, path)));
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
    const dest = await open({ directory: true, defaultPath: parentOf(path), title: t("settings.moveTo") });
    if (typeof dest === "string") await move(file, dest);
  }

  async function move(file: ConfigFile, dest: string) {
    error.textContent = "";
    try {
      let result = await invoke<MoveResult>("move_config_file", { file, destDir: dest, overwrite: false });
      if (result.status === "exists") {
        const replace = await ask(t("settings.exists", { path: result.path }), {
          title: t("exists.title"),
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

  async function createExamples() {
    error.textContent = "";
    try {
      const next = await invoke<State>("create_examples");
      render(next);
      onMoved(next);
      await message(
        t("settings.examplesDone", { dir: next.example_dir }),
        { title: t("settings.examplesTitle") },
      );
    } catch (e) {
      error.textContent = String(e);
    }
  }

  const languageSelect = el(
    "select",
    { "aria-label": t("language.title") },
    el("option", { value: "pt" }, "Português"),
    el("option", { value: "en" }, "English"),
  );
  languageSelect.value = currentLang();
  languageSelect.addEventListener("change", async () => {
    try {
      await invoke("set_language", { language: languageSelect.value });
      location.reload(); // textos da interface são montados na abertura
    } catch (e) {
      error.textContent = String(e);
    }
  });

  render(state);
  const dialog = el(
    "dialog",
    { class: "editor settings" },
    el("h2", {}, t("settings.title")),
    el("p", { class: "hint" }, t("settings.hint")),
    el(
      "div",
      { class: "settings-file" },
      el("div", {}, el("div", { class: "settings-label" }, t("language.title")), el("div", { class: "hint" }, t("language.hint"))),
      el("div", { class: "settings-actions" }, languageSelect),
    ),
    rows,
    el(
      "div",
      { class: "settings-file" },
      el(
        "div",
        {},
        el("div", { class: "settings-label" }, t("settings.examples")),
        el("div", { class: "hint" }, t("settings.examplesHint")),
      ),
      el("div", { class: "settings-actions" }, button(t("settings.createExamples"), createExamples)),
    ),
    error,
    el("div", { class: "editor-actions" }, button(t("common.close"), () => dialog.close(), "primary")),
  );
  dialog.addEventListener("close", () => dialog.remove());
  document.body.append(dialog);
  dialog.showModal();
}
