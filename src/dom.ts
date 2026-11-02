import { t, type Lang } from "./i18n";

export const $ = <T extends HTMLElement>(selector: string) => document.querySelector<T>(selector)!;

/** Cria um elemento; textos sempre via textContent (nomes de arquivo não viram HTML). */
export function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  attrs: Record<string, string> = {},
  ...children: (Node | string)[]
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  for (const [key, value] of Object.entries(attrs)) node.setAttribute(key, value);
  node.append(...children);
  return node;
}

/** Diálogo com um campo de texto; resolve com o texto ou `null` se cancelado. `selectUntil` limita a seleção inicial. */
export function promptText(title: string, value: string, selectUntil = value.length): Promise<string | null> {
  return new Promise((resolve) => {
    const input = el("input", { value, "aria-label": title });
    const form = el("form", { method: "dialog" });
    const dialog = el("dialog", { class: "editor prompt" }, el("h2", {}, title), form);
    let result: string | null = null;
    form.append(
      input,
      el("div", { class: "editor-actions" }, button(t("common.cancel"), () => dialog.close()), el("button", { type: "submit", class: "primary" }, t("common.ok"))),
    );
    form.addEventListener("submit", () => {
      result = input.value;
    });
    dialog.addEventListener("close", () => {
      dialog.remove();
      resolve(result);
    });
    document.body.append(dialog);
    dialog.showModal();
    input.focus();
    input.setSelectionRange(0, selectUntil);
  });
}

/** Diálogo com uma mensagem e botões; resolve com o rótulo escolhido ou `null` (Cancelar/Esc). */
export function choose(title: string, text: string, options: string[]): Promise<string | null> {
  return new Promise((resolve) => {
    const dialog = el("dialog", { class: "editor prompt" }, el("h2", {}, title), el("p", {}, text));
    let result: string | null = null;
    const pick = (label: string) => () => {
      result = label;
      dialog.close();
    };
    const buttons = options.map((label, i) => button(label, pick(label), i === 0 ? "primary" : ""));
    dialog.append(el("div", { class: "editor-actions" }, button(t("common.cancel"), () => dialog.close()), ...buttons));
    dialog.addEventListener("close", () => {
      dialog.remove();
      resolve(result);
    });
    document.body.append(dialog);
    dialog.showModal();
    buttons[0]?.focus();
  });
}

export function button(label: string, onClick: () => void, className = ""): HTMLButtonElement {
  const node = el("button", { type: "button", class: className }, label);
  node.addEventListener("click", onClick);
  return node;
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
