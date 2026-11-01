import type { FileEntry } from "./types";

/** "none" = só arquivos sem data. */
export type YearFilter = number | "none" | null;

export interface Filters {
  /** Termos selecionados como chave `seção:termo`. */
  terms: Set<string>;
  year: YearFilter;
  month: number | null;
}

export const termKey = (section: number, term: number) => `${section}:${term}`;

export function emptyFilters(): Filters {
  return { terms: new Set(), year: null, month: null };
}

export function isEmpty(filters: Filters): boolean {
  return filters.terms.size === 0 && filters.year === null && filters.month === null;
}

/** Arquivo precisa ter todos os termos selecionados (E dentro da seção e entre seções) e a data escolhida. */
export function matches(file: FileEntry, filters: Filters): boolean {
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
