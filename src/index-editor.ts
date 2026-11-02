import { confirm } from "@tauri-apps/plugin-dialog";

import { button, el } from "./dom";
import { t } from "./i18n";
import { notSaved, saveConfig } from "./save-config";
import type { Section, State, Term } from "./types";

const COLLAPSED_KEY = "cajabox.indexEditorCollapsed";

/** Seções recolhidas no editor (pelo nome), lembradas entre aberturas. */
function readCollapsed(): Set<string> {
  try {
    const saved = JSON.parse(localStorage.getItem(COLLAPSED_KEY) ?? "[]");
    return new Set(Array.isArray(saved) ? saved.filter((k) => typeof k === "string") : []);
  } catch {
    return new Set();
  }
}

function saveCollapsed(names: string[]) {
  try {
    localStorage.setItem(COLLAPSED_KEY, JSON.stringify(names));
  } catch {
    // sem localStorage o editor só não lembra as seções recolhidas
  }
}

/** Abre o editor do índice (cajabox.index.md); `onSaved` recebe o estado já gravado. */
export function openIndexEditor(sections: Section[], version: string, onSaved: (state: State) => void) {
  const list = el("div", { class: "index-sections" });
  const collapsed = readCollapsed();
  /** Função que recolhe/expande cada seção. */
  const togglers = new Map<HTMLElement, (value: boolean) => void>();
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
    const count = el("span", { class: "term-count" });
    const toggle = el("button", { type: "button", class: "section-toggle" });
    function setCollapsed(value: boolean) {
      card.classList.toggle("collapsed", value);
      toggle.textContent = value ? "▸" : "▾";
      toggle.setAttribute("aria-expanded", String(!value));
      const n = terms.querySelectorAll(".term-row").length;
      count.textContent = value ? t(n === 1 ? "index.termCountOne" : "index.termCountMany", { n }) : "";
    }
    toggle.addEventListener("click", () => setCollapsed(!card.classList.contains("collapsed")));
    togglers.set(card, setCollapsed);
    async function remove() {
      const label = name.value.trim();
      const blank = !label && terms.querySelectorAll(".term-row").length === 0;
      if (blank || (await confirm(t("index.removeConfirm", { name: label || t("index.unnamed") }), { title: t("index.removeSection"), kind: "warning" }))) {
        card.remove();
      }
    }
    card.append(
      el("div", { class: "editor-row section-row" }, toggle, name, count, button(t("index.removeSection"), remove)),
      terms,
      button(t("index.addTerm"), () => addTerm(terms, { name: "", synonyms: [] }).focus(), "link"),
    );
    section.terms.forEach((term) => addTerm(terms, term));
    setCollapsed(section.name !== "" && collapsed.has(section.name)); // seção nova abre
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

  /** Recolhe ou expande todas as seções. */
  function setAll(value: boolean) {
    for (const card of list.querySelectorAll<HTMLElement>(".index-section")) togglers.get(card)?.(value);
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
    el(
      "div",
      { class: "index-toolbar" },
      button(t("index.collapseAll"), () => setAll(true), "link"),
      button(t("index.expandAll"), () => setAll(false), "link"),
    ),
    list,
    button(t("index.addSection"), () => addSection({ name: "", terms: [] }).focus(), "link"),
    error,
    el("div", { class: "editor-actions" }, button(t("common.cancel"), () => dialog.close()), button(t("common.save"), save, "primary")),
  );
  dialog.addEventListener("close", () => {
    // lembra as seções recolhidas pelo nome que têm ao fechar (salvando ou não)
    saveCollapsed(
      [...list.querySelectorAll<HTMLElement>(".index-section.collapsed")]
        .map((card) => card.querySelector<HTMLInputElement>(".section-name")!.value.trim())
        .filter(Boolean),
    );
    dialog.remove();
  });
  document.body.append(dialog);
  dialog.showModal();
}
