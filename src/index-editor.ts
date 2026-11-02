import { confirm } from "@tauri-apps/plugin-dialog";

import { button, el } from "./dom";
import { t } from "./i18n";
import { notSaved, saveConfig } from "./save-config";
import type { Section, State, Term } from "./types";

/** Abre o editor do índice (manabox.index.md); `onSaved` recebe o estado já gravado. */
export function openIndexEditor(sections: Section[], version: string, onSaved: (state: State) => void) {
  const list = el("div", { class: "index-sections" });
  const error = el("p", { class: "editor-error" });

  function addTerm(terms: HTMLElement, term: Term) {
    const name = el("input", { class: "term-name", placeholder: t("index.termPlaceholder"), value: term.name, "aria-label": t("index.term") });
    const synonyms = el("input", {
      class: "term-synonyms",
      placeholder: t("index.synonymsPlaceholder"),
      value: term.synonyms.join(", "),
      "aria-label": t("index.synonyms"),
    });
    const row = el("div", { class: "editor-row term-row" }, name, synonyms);
    row.append(button("✕", () => row.remove(), "icon"));
    terms.append(row);
    return name;
  }

  function addSection(section: Section) {
    const name = el("input", { class: "section-name", placeholder: t("index.sectionPlaceholder"), value: section.name, "aria-label": t("index.section") });
    const terms = el("div", { class: "index-terms" });
    const card = el("div", { class: "index-section" });
    async function remove() {
      const label = name.value.trim();
      const blank = !label && terms.querySelectorAll(".term-row").length === 0;
      if (blank || (await confirm(t("index.removeConfirm", { name: label || t("index.unnamed") }), { title: t("index.removeSection"), kind: "warning" }))) {
        card.remove();
      }
    }
    card.append(
      el("div", { class: "editor-row section-row" }, name, button(t("index.removeSection"), remove)),
      terms,
      button(t("index.addTerm"), () => addTerm(terms, { name: "", synonyms: [] }).focus(), "link"),
    );
    section.terms.forEach((term) => addTerm(terms, term));
    list.append(card);
    return name;
  }

  /** Linhas totalmente vazias são descartadas; o resto vai para a validação do Rust. */
  function collect(): Section[] {
    return [...list.querySelectorAll<HTMLElement>(".index-section")]
      .map((card) => ({
        name: card.querySelector<HTMLInputElement>(".section-name")!.value.trim(),
        terms: [...card.querySelectorAll<HTMLElement>(".term-row")]
          .map((row) => ({
            name: row.querySelector<HTMLInputElement>(".term-name")!.value.trim(),
            synonyms: row
              .querySelector<HTMLInputElement>(".term-synonyms")!
              .value.split(",")
              .map((s) => s.trim())
              .filter(Boolean),
          }))
          .filter((t) => t.name || t.synonyms.length),
      }))
      .filter((s) => s.name || s.terms.length);
  }

  async function save() {
    error.textContent = "";
    try {
      const state = await saveConfig("save_index", { index: { sections: collect() } }, version);
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

  sections.forEach(addSection);
  const dialog = el(
    "dialog",
    { class: "editor" },
    el("h2", {}, t("index.title")),
    el(
      "p",
      { class: "hint" },
      t("index.hint"),
    ),
    list,
    button(t("index.addSection"), () => addSection({ name: "", terms: [] }).focus(), "link"),
    error,
    el("div", { class: "editor-actions" }, button(t("common.cancel"), () => dialog.close()), button(t("common.save"), save, "primary")),
  );
  dialog.addEventListener("close", () => dialog.remove());
  document.body.append(dialog);
  dialog.showModal();
}
