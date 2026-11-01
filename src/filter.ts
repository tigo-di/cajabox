import type { FileEntry } from "./types";

/** "none" = só arquivos sem data. */
export type YearFilter = number | "none" | null;

export interface Filters {
  /** Termos selecionados como chave `seção:termo`. */
  terms: Set<string>;
  year: YearFilter;
  month: number | null;
  /** Texto buscado no nome do arquivo. */
  search: string;
}

export const termKey = (section: number, term: number) => `${section}:${term}`;

export function emptyFilters(): Filters {
  return { terms: new Set(), year: null, month: null, search: "" };
}

export function isEmpty(filters: Filters): boolean {
  return filters.terms.size === 0 && filters.year === null && filters.month === null && filters.search.trim() === "";
}

/** Minúsculas e sem acentos, para comparar textos. */
export const normalize = (text: string) => text.normalize("NFD").replace(/\p{Diacritic}/gu, "").toLowerCase();

/** Arquivo precisa ter todos os termos selecionados (E dentro da seção e entre seções), a data escolhida
 * e todas as palavras da busca no nome. */
export function matches(file: FileEntry, filters: Filters): boolean {
  const words = normalize(filters.search).split(/\s+/).filter(Boolean);
  if (words.length) {
    const name = normalize(file.name);
    if (!words.every((w) => name.includes(w))) return false;
  }
  for (const key of filters.terms) {
    const [s, t] = key.split(":").map(Number);
    if (!file.terms[s]?.includes(t)) return false;
  }
  if (filters.year === "none") return file.date === null;
  if (filters.year !== null && file.date?.year !== filters.year) return false;
  if (filters.month !== null && file.date?.month !== filters.month) return false;
  return true;
}

/** Anos presentes nos arquivos, do mais recente para o mais antigo. */
export function yearsOf(files: FileEntry[]): number[] {
  return [...new Set(files.flatMap((f) => (f.date ? [f.date.year] : [])))].sort((a, b) => b - a);
}

/** Meses presentes nos arquivos (do ano escolhido, se houver), em ordem. */
export function monthsOf(files: FileEntry[], year: YearFilter): number[] {
  const months = files.flatMap((f) =>
    f.date && (year === null || f.date.year === year) ? [f.date.month] : [],
  );
  return [...new Set(months)].sort((a, b) => a - b);
}
