import { invoke } from "@tauri-apps/api/core";
import { ask } from "@tauri-apps/plugin-dialog";

import type { State } from "./types";

type SaveResult = { status: "saved"; state: State } | { status: "changed" };

/** Grava pelo comando do editor (`save_index`/`save_folders`). Se o arquivo mudou fora do programa
 * desde `version` (lida ao abrir o editor), pergunta antes de substituir. `null` = não gravado. */
export async function saveConfig(command: string, args: Record<string, unknown>, version: string): Promise<State | null> {
  let result = await invoke<SaveResult>(command, { ...args, expected: version, overwrite: false });
  if (result.status === "changed") {
    const replace = await ask(
      "O arquivo foi alterado fora do manabox depois que este editor foi aberto.\nSubstituir pelo conteúdo do editor?",
      { title: "Arquivo alterado", kind: "warning" },
    );
    if (!replace) return null;
    result = await invoke<SaveResult>(command, { ...args, expected: version, overwrite: true });
  }
  return result.status === "saved" ? result.state : null;
}

export const NOT_SAVED = "Não salvo. Feche o editor e abra de novo para ver a versão atual do arquivo.";
