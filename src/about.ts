import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";

import { button, el } from "./dom";

const AUTHOR = "Tiago Dias";
const EMAIL = "tiago@webdeleve.net";
const LICENSE_URL = "https://www.gnu.org/licenses/gpl-3.0.html";

function externalLink(label: string, url: string) {
  const link = button(label, () => openUrl(url), "link");
  link.title = url;
  return link;
}

/** Créditos de autoria, versão e licença. */
export async function openAbout() {
  const version = await getVersion();
  const dialog = el(
    "dialog",
    { class: "editor about" },
    el("h2", {}, "manabox"),
    el("p", { class: "about-version" }, `Versão ${version}`),
    el("p", {}, "Gerenciamento de conteúdo: lista e filtra arquivos de pastas pelos termos e datas dos nomes."),
    el("p", {}, `© 2026 ${AUTHOR} — `, externalLink(EMAIL, `mailto:${EMAIL}`)),
    el(
      "p",
      { class: "about-license" },
      "Software livre sob a ",
      externalLink("GNU GPL v3 ou posterior", LICENSE_URL),
      ": pode ser usado, alterado, distribuído e vendido; versões derivadas devem manter os créditos e a mesma licença.",
    ),
    el("div", { class: "editor-actions" }, button("Fechar", () => dialog.close(), "primary")),
  );
  dialog.addEventListener("close", () => dialog.remove());
  document.body.append(dialog);
  dialog.showModal();
}
