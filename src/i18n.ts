import { button, el } from "./dom";
import type { FileDate } from "./types";

export type Lang = "pt" | "en";

/** Textos da interface em português; `en` precisa ter as mesmas chaves (checado pelo TypeScript). */
const pt = {
  "language.title": "Idioma",
  "language.hint": "Idioma da interface, das mensagens e das pastas de exemplo. A janela é recarregada ao trocar.",
} as const;

type Key = keyof typeof pt;

const en: Record<Key, string> = {
  "language.title": "Language",
  "language.hint": "Language of the interface, messages and example folders. The window reloads when you change it.",
};

const dictionaries: Record<Lang, Record<Key, string>> = { pt, en };

let lang: Lang = "pt";

export const currentLang = () => lang;

export function setLang(next: Lang) {
  lang = next;
  document.documentElement.lang = next === "pt" ? "pt-BR" : "en";
}

/** Idioma do sistema: português se for pt-*, senão inglês. */
export const systemLang = (): Lang => (navigator.language.toLowerCase().startsWith("pt") ? "pt" : "en");

/** Texto no idioma atual; `{nome}` é trocado por `params.nome`. */
export function t(key: Key, params: Record<string, string | number> = {}): string {
  return dictionaries[lang][key].replace(/\{(\w+)\}/g, (_, name) => String(params[name] ?? `{${name}}`));
}

const pad = (n: number) => String(n).padStart(2, "0");

/** Português: 29/11/2024 ou 11/2024. Inglês (ISO): 2024-11-29 ou 2024-11. */
export function formatDate(date: FileDate): string {
  if (lang === "en") return date.day === null ? `${date.year}-${pad(date.month)}` : `${date.year}-${pad(date.month)}-${pad(date.day)}`;
  return date.day === null ? `${pad(date.month)}/${date.year}` : `${pad(date.day)}/${pad(date.month)}/${date.year}`;
}

/** Nome do mês (1–12) no idioma atual, pelo sistema. */
export function monthName(month: number): string {
  const name = new Intl.DateTimeFormat(lang === "pt" ? "pt-BR" : "en", { month: "long" }).format(new Date(2000, month - 1, 1));
  return lang === "pt" ? name : name[0].toUpperCase() + name.slice(1);
}

/** Primeira execução: escolha do idioma (bilíngue), com `initial` pré-selecionado. */
export function chooseLanguage(initial: Lang): Promise<Lang> {
  return new Promise((resolve) => {
    let chosen = initial;
    const dialog = el("dialog", { class: "editor prompt language" }, el("h2", {}, "Idioma / Language"));
    const option = (value: Lang, label: string) => {
      const node = button(label, () => {
        chosen = value;
        dialog.close();
      }, value === initial ? "primary" : "");
      return node;
    };
    const buttons = [option("pt", "Português"), option("en", "English")];
    dialog.append(el("div", { class: "editor-actions" }, ...buttons));
    dialog.addEventListener("close", () => {
      dialog.remove();
      resolve(chosen); // Esc = idioma pré-selecionado
    });
    document.body.append(dialog);
    dialog.showModal();
    buttons[initial === "pt" ? 0 : 1].focus();
  });
}
