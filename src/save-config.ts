import { invoke } from "@tauri-apps/api/core";
import { ask } from "@tauri-apps/plugin-dialog";

import { t } from "./i18n";
import type { State } from "./types";

type SaveResult = { status: "saved"; state: State } | { status: "changed" };

/** Grava pelo comando do editor (`save_index`/`save_folders`). Se o arquivo mudou fora do programa
 * desde `version` (lida ao abrir o editor), pergunta antes de substituir. `null` = não gravado. */
export async function saveConfig(command: string, args: Record<string, unknown>, version: string): Promise<State | null> {
  let result = await invoke<SaveResult>(command, { ...args, expected: version, overwrite: false });
  if (result.status === "changed") {
    const replace = await ask(
      t("save.changed"),
      { title: t("save.changedTitle"), kind: "warning" },
    );
    if (!replace) return null;
    result = await invoke<SaveResult>(command, { ...args, expected: version, overwrite: true });
  }
  return result.status === "saved" ? result.state : null;
}

export const notSaved = () => t("save.notSaved");
