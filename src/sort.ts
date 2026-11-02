import { effectiveDate } from "./filter";
import type { FileEntry } from "./types";

export type SortKey = "name" | "date";
export type Sort = { key: SortKey; dir: "asc" | "desc" };

const SORT_KEY = "cajabox.sort";
const DEFAULT_SORT: Sort = { key: "name", dir: "asc" };

/** Ordem natural ("ep2" antes de "ep10"), sem diferenciar maiúsculas nem acentos. */
const collator = new Intl.Collator("pt-BR", { numeric: true, sensitivity: "base" });

const byName = (a: FileEntry, b: FileEntry) => collator.compare(a.name, b.name);

/** Data efetiva (nome ou arquivo) como número comparável; mês sem dia (dia 0) vem antes dos dias do mês. */
const dateValue = (f: FileEntry) => {
  const date = effectiveDate(f);
  return date ? date.year * 10000 + date.month * 100 + (date.day ?? 0) : null;
};

/** Ordena uma cópia de `files`. Por data: sem data sempre no fim; empate pelo nome (A–Z). */
export function sortFiles(files: FileEntry[], sort: Sort): FileEntry[] {
  const sign = sort.dir === "asc" ? 1 : -1;
  return [...files].sort((a, b) => {
    if (sort.key === "name") return sign * byName(a, b);
    const [da, db] = [dateValue(a), dateValue(b)];
    if (da === null || db === null) return da === db ? byName(a, b) : da === null ? 1 : -1;
    return sign * (da - db) || byName(a, b);
  });
}

/** Clique no título de uma coluna: a ativa inverte o sentido; outra passa a valer (data começa pelos mais recentes). */
export function toggleSort(sort: Sort, key: SortKey): Sort {
  if (sort.key === key) return { key, dir: sort.dir === "asc" ? "desc" : "asc" };
  return { key, dir: key === "date" ? "desc" : "asc" };
}

export function readSort(): Sort {
  try {
    const saved = JSON.parse(localStorage.getItem(SORT_KEY) ?? "null");
    if ((saved?.key === "name" || saved?.key === "date") && (saved.dir === "asc" || saved.dir === "desc")) return saved;
  } catch {
    // valor inválido ou sem localStorage: ordenação padrão
  }
  return DEFAULT_SORT;
}

export function saveSort(sort: Sort) {
  try {
    localStorage.setItem(SORT_KEY, JSON.stringify(sort));
  } catch {
    // sem localStorage o app só não lembra a ordenação
  }
}
