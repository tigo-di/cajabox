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
  config: { index: string; paths: string };
  /** Pasta padrão de manabox.index.md e manabox.paths.md. */
  default_dir: string;
  index: { sections: Section[] };
  folders: Folder[];
  /** Versão (hash do conteúdo) de cada arquivo lido; o editor a devolve ao salvar. */
  versions: { index: string; paths: string };
  /** Pasta das pastas de exemplo. */
  example_dir: string;
}

export interface FileDate {
  year: number;
  month: number;
  day: number | null;
}

export interface FileEntry {
  name: string;
  /** Relativo à pasta listada, incluindo o nome. */
  relative_path: string;
  path: string;
  /** Tamanho em bytes. */
  size: number;
  /** Uma lista por seção do índice, com as posições dos termos encontrados. */
  terms: number[][];
  date: FileDate | null;
}
