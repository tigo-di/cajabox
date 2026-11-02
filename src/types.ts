// Espelham as estruturas serializadas pelo Rust (src-tauri/src).

export interface Term {
  name: string;
  synonyms: string[];
}

export interface Section {
  name: string;
  terms: Term[];
}

export interface Folder {
  name: string;
  description: string;
  path: string;
}

export interface State {
  config: { index: string; paths: string; language: "pt" | "en" | null };
  /** Pasta padrão de cajabox.index.md e cajabox.paths.md. */
  default_dir: string;
  index: { sections: Section[] };
  folders: Folder[];
  /** Versão (hash do conteúdo) de cada arquivo lido; o editor a devolve ao salvar. */
  versions: { index: string; paths: string };
  /** Pasta das pastas de exemplo no idioma atual. */
  example_dir: string;
  /** Pastas de exemplo de todos os idiomas (aviso na interface). */
  example_dirs: string[];
  /** Primeira execução: perguntar o idioma e criar os exemplos. */
  first_run: boolean;
}

export interface FileDate {
  year: number;
  month: number;
  day: number | null;
}

/** Subpasta da pasta listada (também as vazias). */
export interface FolderEntry {
  name: string;
  /** Relativo à pasta listada. */
  relative_path: string;
  path: string;
}

export interface FileEntry {
  name: string;
  /** Relativo à pasta listada, incluindo o nome. */
  relative_path: string;
  path: string;
  /** Tamanho em bytes. */
  size: number;
  /** Data de modificação (milissegundos desde 1970); usada quando não há data no nome. */
  modified: number;
  /** Uma lista por seção do índice, com as posições dos termos encontrados. */
  terms: number[][];
  date: FileDate | null;
}
