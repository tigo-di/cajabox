import type { FileDate, FileEntry } from "./types";

/** Data usada pelo programa: a do nome (escolhida pelo usuário) ou, sem ela, a de modificação do
 * arquivo, no fuso horário local. `fromFile` diferencia as duas na tela. */
export type EffectiveDate = FileDate & { fromFile: boolean };

export function effectiveDate(file: FileEntry): EffectiveDate | null {
  if (file.date) return { ...file.date, fromFile: false };
  if (!file.modified) return null;
  const date = new Date(file.modified);
  return { year: date.getFullYear(), month: date.getMonth() + 1, day: date.getDate(), fromFile: true };
}

/** "none" = só arquivos sem data no nome. */
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
  const date = effectiveDate(file);
  if (filters.year !== null && date?.year !== filters.year) return false;
  if (filters.month !== null && date?.month !== filters.month) return false;
  return true;
}

/** Algum filtro de termo ou data marcado (esses filtros classificam arquivos: escondem as pastas). */
export const filtersFiles = (filters: Filters) => filters.terms.size > 0 || filters.year !== null || filters.month !== null;

/** Pasta casa com a busca: todas as palavras no nome. */
export function matchesFolder(name: string, filters: Filters): boolean {
  const target = normalize(name);
  return normalize(filters.search).split(/\s+/).filter(Boolean).every((w) => target.includes(w));
}

/** Anos presentes nos arquivos, do mais recente para o mais antigo. */
export function yearsOf(files: FileEntry[]): number[] {
  return [...new Set(files.flatMap((f) => effectiveDate(f)?.year ?? []))].sort((a, b) => b - a);
}

/** Meses presentes nos arquivos (do ano escolhido, se houver), em ordem. */
export function monthsOf(files: FileEntry[], year: YearFilter): number[] {
  const months = files.flatMap((f) => {
    const date = effectiveDate(f);
    return date && (year === null || date.year === year) ? [date.month] : [];
  });
  return [...new Set(months)].sort((a, b) => a - b);
}
