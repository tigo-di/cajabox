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
      el("div", { class: "editor-actions" }, button("Cancelar", () => dialog.close()), el("button", { type: "submit", class: "primary" }, "OK")),
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

export function button(label: string, onClick: () => void, className = ""): HTMLButtonElement {
  const node = el("button", { type: "button", class: className }, label);
  node.addEventListener("click", onClick);
  return node;
}
