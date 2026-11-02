import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";

import { button, el } from "./dom";
import { t } from "./i18n";

const AUTHOR = "Tiago Dias";
const EMAIL = "tigo@webdeleve.net";
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
    el("p", { class: "about-version" }, t("about.version", { version })),
    el("p", {}, t("about.description")),
    el("p", {}, `© 2026 ${AUTHOR} — `, externalLink(EMAIL, `mailto:${EMAIL}`)),
    el(
      "p",
      { class: "about-license" },
      t("about.licensePrefix"),
      externalLink(t("about.licenseName"), LICENSE_URL),
      t("about.licenseSuffix"),
    ),
    el("div", { class: "editor-actions" }, button(t("common.close"), () => dialog.close(), "primary")),
  );
  dialog.addEventListener("close", () => dialog.remove());
  document.body.append(dialog);
  dialog.showModal();
}
