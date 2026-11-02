import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { ask, message, open } from "@tauri-apps/plugin-dialog";

import { openAbout } from "./about";
import { currentLang, formatDate, monthName, setLang, systemLang, t, type Key } from "./i18n";
import { $, button, choose, chooseLanguage, el, promptText } from "./dom";
import { effectiveDate, emptyFilters, isEmpty, matches, monthsOf, termKey, yearsOf, type YearFilter } from "./filter";
import { openFoldersEditor } from "./folders-editor";
import { openIndexEditor } from "./index-editor";
import { openSettings } from "./settings";
import { readSort, saveSort, sortFiles, toggleSort, type SortKey } from "./sort";
import type { FileEntry, Folder, State } from "./types";

const LAST_FOLDER_KEY = "manabox.lastFolder";

let state: State | null = null;
let current: Folder | null = null;
let files: FileEntry[] = [];
let filters = emptyFilters();
/** Arquivos exibidos após os filtros, na ordem da tabela. */
let visible: FileEntry[] = [];
/** Caminhos selecionados (sempre um subconjunto de `visible`). */
let selected = new Set<string>();
/** Âncora do Shift+clique. */
let anchor: string | null = null;
type ClipboardFiles = { mode: "copy" | "cut"; paths: string[]; skipped: string[] };
const EMPTY_CLIPBOARD: ClipboardFiles = { mode: "copy", paths: [], skipped: [] };
/** Espelho da área de transferência do sistema (arquivos copiados/recortados no manabox ou num gerenciador). */
let clipboard = EMPTY_CLIPBOARD;

let sort = readSort();

type DuplicateMode = "name" | "content";
/** Modo Duplicados: critério, grupos de caminhos (`null` = buscando) e progresso da busca por
 * conteúdo. Considera todos os arquivos da pasta aberta (filtros e busca ficam suspensos). */
let duplicates: { mode: DuplicateMode; groups: string[][] | null; progress: [number, number] | null } | null = null;
/** Identifica a busca mais recente; resultados de buscas anteriores são descartados. */
let duplicatesRun = 0;

const COLLAPSED_KEY = "manabox.collapsed";
/** Grupos de filtro recolhidos: `date` ou `section:<nome>`; lembrados entre execuções. */
let collapsed = readCollapsed();

function readCollapsed(): Set<string> {
  try {
    const saved = JSON.parse(localStorage.getItem(COLLAPSED_KEY) ?? "[]");
    return new Set(Array.isArray(saved) ? saved.filter((k) => typeof k === "string") : []);
  } catch {
    return new Set();
  }
}

function saveCollapsed() {
  try {
    localStorage.setItem(COLLAPSED_KEY, JSON.stringify([...collapsed]));
  } catch {
    // sem localStorage o app só não lembra as seções recolhidas
  }
}

/** Linha onde o botão do mouse foi pressionado (possível início de arrasto para fora). */
let pressed: { file: FileEntry; x: number; y: number } | null = null;
/** Arrasto para fora em andamento: soltar na própria janela é ignorado. */
let draggingOut = false;

type FileResult = { status: "done" | "exists"; path: string };


function readLastFolder(): string | null {
  try {
    return localStorage.getItem(LAST_FOLDER_KEY);
  } catch {
    return null;
  }
}

function saveLastFolder(name: string) {
  try {
    localStorage.setItem(LAST_FOLDER_KEY, name);
  } catch {
    // sem localStorage o app só não lembra a última pasta
  }
}

function renderFolders() {
  const list = $("#folders");
  list.replaceChildren();
  if (!state || state.folders.length === 0) {
    list.append(el("p", { class: "hint" }, "Nenhuma pasta cadastrada. Use “Editar” para adicionar."));
    return;
  }
  for (const folder of state.folders) {
    const button = el(
      "button",
      { class: "folder", type: "button", title: folder.path },
      el("span", { class: "folder-name" }, folder.name),
      el("span", { class: "folder-desc" }, folder.description),
    );
    if (folder === current) button.classList.add("active");
    button.addEventListener("click", () => selectFolder(folder));
    list.append(button);
  }
}

function countLabel(shown: number, total: number): string {
  const noun = t(total === 1 ? "count.file" : "count.files");
  return shown === total ? `${total} ${noun}` : t("count.of", { shown, total, noun });
}

/** `shown` null = ainda carregando. */
function renderHeader(shown: number | null) {
  const header = $("#folder-header");
  header.replaceChildren();
  $("#searchbar").hidden = !current || !!duplicates; // busca suspensa no modo Duplicados
  if (!current) return;
  header.append(
    el(
      "div",
      { class: "title-row" },
      el("h1", {}, current.name),
      shown === null
        ? ""
        : el(
            "span",
            { class: "count" },
            duplicates?.groups ? t("duplicates.count", { shown, total: files.length }) : countLabel(shown, files.length),
          ),
    ),
    current.description ? el("p", { class: "desc" }, current.description) : "",
    el("p", { class: "path" }, folderLink(current.path, current.path)),
    state?.example_dirs.some((dir) => current?.path.startsWith(`${dir}/`))
      ? el(
          "p",
          { class: "notice" },
          t("example.notice"),
        )
      : "",
  );
}

/** Caminho da pasta: clique abre no gerenciador de arquivos. */
function folderLink(label: string, path: string) {
  const link = button(label, async () => {
    try {
      await invoke("open_folder", { path });
    } catch (error) {
      await message(String(error), { title: t("openFolder.title"), kind: "error" });
    }
  }, "link path-link");
  link.title = t("openFolder.hint");
  return link;
}

/** Subpasta do arquivo na lista: clique abre no gerenciador sem selecionar, arrastar nem abrir a linha. */
function subdirLink(file: FileEntry) {
  const subdir = file.relative_path.slice(0, file.relative_path.lastIndexOf("/"));
  const link = folderLink(subdir, file.path.slice(0, file.path.lastIndexOf("/")));
  for (const type of ["click", "mousedown", "dblclick"]) link.addEventListener(type, (event) => event.stopPropagation());
  return link;
}

function renderMessage(text: string, kind: "hint" | "error" = "hint") {
  $("#content").replaceChildren(el("p", { class: `message ${kind}` }, text));
}

function termChips(file: FileEntry): HTMLElement {
  const cell = el("td", { class: "terms" });
  const sections = state?.index.sections ?? [];
  file.terms.forEach((found, s) => {
    for (const position of found) {
      const section = sections[s];
      const name = section?.terms[position]?.name ?? "?";
      const key = termKey(s, position);
      const active = filters.terms.has(key);
      const title = t(active ? "chip.removeFilter" : "chip.filterBy", { section: section?.name ?? "", term: name });
      const chip = el("span", { class: `chip row-chip s${s % 6}`, title, "data-term": key }, name);
      chip.classList.toggle("selected", active);
      cell.append(chip);
    }
  });
  return cell;
}

/** Clique seleciona um; Ctrl+clique alterna; Shift+clique seleciona o intervalo desde a âncora. */
function clickRow(file: FileEntry, event: MouseEvent) {
  const paths = visible.map((f) => f.path);
  if (event.shiftKey && anchor && paths.includes(anchor)) {
    const [a, b] = [paths.indexOf(anchor), paths.indexOf(file.path)].sort((x, y) => x - y);
    if (!event.ctrlKey) selected.clear();
    paths.slice(a, b + 1).forEach((p) => selected.add(p));
  } else if (event.ctrlKey || event.metaKey) {
    if (!selected.delete(file.path)) selected.add(file.path);
    anchor = file.path;
  } else {
    selected = new Set([file.path]);
    anchor = file.path;
  }
  renderFiles();
}

function renderToolbar() {
  const bar = $("#toolbar");
  bar.replaceChildren();
  if (!current || (files.length === 0 && clipboard.paths.length === 0)) return;

  const count = selected.size;
  const rename = button(t("toolbar.rename"), renameSelected);
  rename.disabled = count !== 1;
  rename.title = "F2";
  const copy = button(t("toolbar.copy"), () => toClipboard("copy"));
  copy.disabled = count === 0;
  copy.title = "Ctrl+C";
  const cut = button(t("toolbar.cut"), () => toClipboard("cut"));
  cut.disabled = count === 0;
  cut.title = "Ctrl+X";
  const remove = button(t("toolbar.delete"), trashSelected, "danger");
  remove.disabled = count === 0;
  remove.title = "Delete";
  const info = count ? t(count === 1 ? "toolbar.selectedOne" : "toolbar.selectedMany", { n: count }) : t("toolbar.noneSelected");
  bar.append(el("div", { class: "toolbar-group" }, el("span", { class: "toolbar-info" }, info), rename, copy, cut, remove));

  const searching = duplicates?.groups === null;
  const modeButton = (label: string, mode: DuplicateMode) => {
    const active = duplicates?.mode === mode;
    const node = button(label, () => (active ? exitDuplicates() : findDuplicates(mode)), active ? "primary" : "");
    node.title = t(active ? "duplicates.exitHint" : mode === "name" ? "duplicates.nameHint" : "duplicates.contentHint");
    return node;
  };
  bar.append(
    el(
      "div",
      { class: "toolbar-group" },
      el("span", { class: "toolbar-info" }, t("duplicates.label")),
      modeButton(t("duplicates.byName"), "name"),
      modeButton(t("duplicates.byContent"), "content"),
      duplicates ? button(searching ? t("common.cancel") : t("duplicates.exit"), exitDuplicates) : "",
    ),
  );

  const pending = clipboard.paths.length;
  if (pending) {
    const label = clipboard.mode === "copy" ? ["clipboard.copiedOne", "clipboard.copiedMany"] as const : ["clipboard.cutOne", "clipboard.cutMany"] as const;
    const here = button(t("paste.here"), () => current && paste(current.path), "primary");
    here.title = t("paste.hereHint");
    bar.append(
      el(
        "div",
        { class: "toolbar-group" },
        el("span", { class: "toolbar-info" }, t(label[pending === 1 ? 0 : 1], { n: pending })),
        here,
        button(t("paste.into"), pasteInto),
        button(t("common.cancel"), () => {
          setClipboard([], "copy");
        }),
      ),
    );
  }
}

/** Título de coluna ordenável: clique alterna a ordenação; seta mostra coluna e sentido ativos. */
function sortHeader(label: string, key: SortKey) {
  const active = sort.key === key;
  const arrow = active ? (sort.dir === "asc" ? " ▲" : " ▼") : "";
  const th = el("th", { class: active ? "sortable sorted" : "sortable", title: t("sort.hint") }, label + arrow);
  if (active) th.setAttribute("aria-sort", sort.dir === "asc" ? "ascending" : "descending");
  th.addEventListener("click", () => {
    sort = toggleSort(sort, key);
    saveSort(sort);
    renderFiles();
  });
  return th;
}


/** Tamanho legível (unidades de 1000, como no Nautilus). */
function formatSize(bytes: number): string {
  const units = ["bytes", "kB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1000 && unit < units.length - 1) {
    value /= 1000;
    unit++;
  }
  const number = new Intl.NumberFormat(currentLang() === "pt" ? "pt-BR" : "en", { maximumFractionDigits: 1 });
  return `${number.format(value)} ${units[unit]}`;
}

/** Entra no modo Duplicados (ou troca o critério) e busca; sem `mode`, refaz a busca atual. */
async function findDuplicates(mode?: DuplicateMode) {
  const folder = current;
  if (!folder) return;
  if (mode && mode !== duplicates?.mode) duplicates = { mode, groups: null, progress: null };
  if (!duplicates) return;
  const run = ++duplicatesRun;
  const searching = duplicates;
  update();
  try {
    const groups = await invoke<string[][] | null>("find_duplicates", { path: folder.path, mode: searching.mode });
    if (run !== duplicatesRun || current !== folder || duplicates !== searching) return;
    if (groups === null) duplicates = null; // cancelado
    else {
      duplicates.groups = groups;
      duplicates.progress = null;
    }
  } catch (error) {
    if (run !== duplicatesRun) return;
    duplicates = null;
    await message(String(error), { title: t("duplicates.title"), kind: "error" });
  }
  update();
}

function exitDuplicates() {
  if (duplicates?.groups === null) invoke("cancel_duplicates");
  duplicates = null;
  duplicatesRun++;
  update();
}

/** Grupos de duplicados ainda presentes na lista (arquivo excluído ou movido sai do grupo). */
function duplicateGroups(): FileEntry[][] {
  const byPath = new Map(files.map((f) => [f.path, f]));
  return (duplicates?.groups ?? [])
    .map((group) => group.flatMap((path) => byPath.get(path) ?? []))
    .filter((group) => group.length > 1);
}

function renderFiles() {
  if (duplicates) return renderDuplicates();
  visible = sortFiles(
    files.filter((f) => matches(f, filters)),
    sort,
  );
  const shown = new Set(visible.map((f) => f.path));
  selected = new Set([...selected].filter((p) => shown.has(p)));
  renderHeader(visible.length);
  renderToolbar();
  if (files.length === 0) {
    renderMessage(t("list.empty"));
    return;
  }
  if (visible.length === 0) {
    renderMessage(t("list.noMatch"));
    return;
  }
  const body = el("tbody");
  for (const file of visible) body.append(fileRow(file));
  const table = el(
    "table",
    { class: "files" },
    el(
      "thead",
      {},
      el("tr", {}, sortHeader(t("col.name"), "name"), el("th", {}, t("col.subfolder")), el("th", {}, t("col.terms")), sortHeader(t("col.date"), "date")),
    ),
    body,
  );
  showTable(table);
}

/** Lista do modo Duplicados: grupos com título, coluna Tamanho, sem filtros nem ordenação. */
function renderDuplicates() {
  const groups = duplicateGroups();
  visible = groups.flat();
  const shown = new Set(visible.map((f) => f.path));
  selected = new Set([...selected].filter((p) => shown.has(p)));
  renderHeader(visible.length);
  renderToolbar();
  if (!duplicates || duplicates.groups === null) {
    const [done, total] = duplicates?.progress ?? [0, 0];
    renderMessage(total ? t("duplicates.checking", { done, total }) : t("duplicates.searching"));
    return;
  }
  if (groups.length === 0) {
    renderMessage(t("duplicates.none"));
    return;
  }
  const body = el("tbody");
  for (const group of groups) {
    const title =
      duplicates.mode === "content"
        ? t("duplicates.groupContent", { n: group.length, size: formatSize(group[0].size) })
        : t("duplicates.groupName", { n: group.length, name: group[0].name });
    body.append(el("tr", { class: "group-row" }, el("td", { colspan: "5" }, title)));
    for (const file of group) body.append(fileRow(file, true));
  }
  const table = el(
    "table",
    { class: "files" },
    el(
      "thead",
      {},
      el("tr", {}, el("th", {}, t("col.name")), el("th", {}, t("col.subfolder")), el("th", {}, t("col.terms")), el("th", {}, t("col.date")), el("th", { class: "size" }, t("col.size"))),
    ),
    body,
  );
  showTable(table);
}

function showTable(table: HTMLElement) {
  const content = $("#content");
  const scroll = content.scrollTop;
  content.replaceChildren(table);
  content.scrollTop = scroll;
}

/** Data efetiva; a que vem do arquivo (sem data no nome) aparece em cinza e itálico, com dica. */
function dateCell(file: FileEntry) {
  const date = effectiveDate(file);
  const cell = el("td", { class: date?.fromFile ? "date from-file" : "date" }, date ? formatDate(date) : "");
  if (date?.fromFile) {
    cell.title = t("date.fromFile");
  }
  return cell;
}

/** Linha de um arquivo (com coluna Tamanho no modo Duplicados). */
function fileRow(file: FileEntry, withSize = false): HTMLTableRowElement {
  const slash = file.relative_path.lastIndexOf("/");
  const row = el(
    "tr",
    { title: file.path },
    el("td", { class: "name" }, file.name),
    el("td", { class: "subdir" }, slash > 0 ? subdirLink(file) : ""),
    termChips(file),
    dateCell(file),
    withSize ? el("td", { class: "size" }, formatSize(file.size)) : "",
  );
  row.classList.toggle("selected", selected.has(file.path));
  row.classList.toggle("cut", clipboard.mode === "cut" && clipboard.paths.includes(file.path));
  // Termo na linha (chip): alterna o filtro, sem selecionar, arrastar nem abrir o arquivo
  const chipOf = (event: Event) => (event.target as HTMLElement).closest<HTMLElement>(".row-chip");
  row.addEventListener("click", (event) => {
    const chip = chipOf(event);
    if (!chip || duplicates) return clickRow(file, event); // filtros suspensos no modo Duplicados
    const key = chip.dataset.term!;
    if (!filters.terms.delete(key)) filters.terms.add(key);
    update();
  });
  row.addEventListener("dblclick", (event) => chipOf(event) || openFiles([file.path]));
  row.addEventListener("mousedown", (event) => {
    if (chipOf(event)) return;
    if (event.shiftKey) event.preventDefault(); // Shift+clique não estende seleção de texto pela página
    if (event.button === 0 && !event.ctrlKey && !event.shiftKey && !event.metaKey) {
      pressed = { file, x: event.clientX, y: event.clientY };
    }
  });
  return row;
}

/** Relista a pasta atual sem apagar a tela, mantendo filtros e a seleção que ainda existir. */
async function reload() {
  const folder = current;
  if (!folder) return;
  try {
    const result = await invoke<FileEntry[]>("list_folder", { path: folder.path });
    if (current !== folder) return;
    files = result;
    normalizeDateFilter();
    update();
    if (duplicates?.groups) findDuplicates(); // novos arquivos podem ser cópias (conteúdo usa cache)
  } catch (error) {
    if (current !== folder) return;
    $("#filters").replaceChildren();
    $("#toolbar").replaceChildren();
    renderMessage(String(error), "error");
  }
}

/** Abre arquivos com o programa padrão do sistema; erros juntos num só aviso. */
async function openFiles(paths: string[]) {
  const errors: string[] = [];
  for (const path of paths) {
    try {
      await invoke("open_file", { path });
    } catch (error) {
      errors.push(String(error));
    }
  }
  if (errors.length) await message(errors.join("\n"), { title: t("open.title"), kind: "error" });
}

async function renameSelected() {
  if (selected.size !== 1) return;
  const path = [...selected][0];
  const name = path.slice(path.lastIndexOf("/") + 1);
  const dot = name.lastIndexOf(".");
  const newName = await promptText(t("rename.title"), name, dot > 0 ? dot : name.length);
  if (newName === null || newName === name) return;
  try {
    const newPath = await invoke<string>("rename_file", { path, newName });
    if (clipboard.paths.includes(path)) {
      await setClipboard(clipboard.paths.map((p) => (p === path ? newPath : p)), clipboard.mode);
    }
    selected = new Set([newPath]);
    anchor = newPath;
  } catch (error) {
    await message(String(error), { title: t("rename.title"), kind: "error" });
  }
  await reload();
}

/** Lê a área de transferência do sistema (pode ter mudado em outro programa). */
async function refreshClipboard() {
  try {
    clipboard = (await invoke<ClipboardFiles | null>("clipboard_get")) ?? EMPTY_CLIPBOARD;
  } catch (error) {
    console.error(error);
    clipboard = EMPTY_CLIPBOARD;
  }
  if (current) renderFiles();
}

/** Grava (ou, sem caminhos, esvazia) a área de transferência do sistema. */
async function setClipboard(paths: string[], mode: "copy" | "cut") {
  try {
    if (paths.length) await invoke("clipboard_set", { paths, mode });
    else await invoke("clipboard_clear");
    clipboard = { mode, paths, skipped: [] };
  } catch (error) {
    await message(String(error), { title: t("clipboard.title"), kind: "error" });
  }
  renderFiles();
}

function toClipboard(mode: "copy" | "cut") {
  if (selected.size) setClipboard([...selected], mode);
}

async function trashSelected() {
  const paths = [...selected];
  if (paths.length === 0) return;
  const what = paths.length === 1 ? `“${paths[0].slice(paths[0].lastIndexOf("/") + 1)}”` : t("files.many", { n: paths.length });
  if (!(await ask(t("trash.confirm", { what }), { title: t("trash.title"), kind: "warning" }))) return;
  try {
    await invoke("trash_files", { paths });
    if (clipboard.paths.some((p) => paths.includes(p))) {
      await setClipboard(clipboard.paths.filter((p) => !paths.includes(p)), clipboard.mode);
    }
    selected.clear();
  } catch (error) {
    await message(String(error), { title: t("trash.title"), kind: "error" });
  }
  await reload();
}

async function warnSkipped(skipped: string[]) {
  if (skipped.length) await message(t("paste.skipped", { list: skipped.join("\n") }), { title: t("paste.title"), kind: "warning" });
}

/** Copia ou move `paths` para `dest`; em conflito pergunta por arquivo se substitui. */
async function transfer(paths: string[], mode: "copy" | "cut", dest: string) {
  const command = mode === "copy" ? "copy_file" : "move_file";
  const errors: string[] = [];
  const pasted: string[] = [];
  for (const path of paths) {
    try {
      let result = await invoke<FileResult>(command, { path, destDir: dest, overwrite: false });
      if (result.status === "exists") {
        const replace = await ask(t("exists.replace", { path: result.path }), { title: t("exists.title"), kind: "warning" });
        if (!replace) continue;
        result = await invoke<FileResult>(command, { path, destDir: dest, overwrite: true });
      }
      pasted.push(result.path);
    } catch (error) {
      errors.push(String(error));
    }
  }
  selected = new Set(pasted);
  if (errors.length) await message(errors.join("\n"), { title: t("paste.title"), kind: "error" });
  await reload();
}

/** Cola a área de transferência em `dest`. Recorte é colado uma vez só; cópia pode ser colada de novo. */
async function paste(dest: string) {
  await refreshClipboard();
  await warnSkipped(clipboard.skipped);
  if (clipboard.paths.length === 0) return;
  const { paths, mode } = clipboard;
  if (mode === "cut") await setClipboard([], "copy"); // recorte colado sai da área de transferência
  await transfer(paths, mode, dest);
}

/** Itens soltos na janela (arrastados de um gerenciador de arquivos): copiar ou mover para a pasta aberta. */
async function dropFiles(dropped: string[]) {
  const folder = current;
  if (!folder || dropped.length === 0 || document.querySelector("dialog[open]")) return;
  const { paths, skipped } = await invoke<{ paths: string[]; skipped: string[] }>("split_files", { paths: dropped });
  await warnSkipped(skipped);
  if (paths.length === 0) return;
  const what = paths.length === 1 ? `“${paths[0].slice(paths[0].lastIndexOf("/") + 1)}”` : t("files.many", { n: paths.length });
  const choice = await choose(t("drop.title"), t("drop.where", { what, folder: folder.name }), [t("drop.copy"), t("drop.move")]);
  if (choice) await transfer(paths, choice === t("drop.copy") ? "copy" : "cut", folder.path);
}

/** Arrasta a seleção (ou só a linha, se não estiver selecionada) para outro programa. */
async function dragOut(file: FileEntry) {
  if (!selected.has(file.path)) {
    selected = new Set([file.path]);
    anchor = file.path;
    renderFiles();
  }
  const paths = visible.filter((f) => selected.has(f.path)).map((f) => f.path);
  draggingOut = true;
  try {
    await invoke("drag_files", { paths });
  } catch (error) {
    draggingOut = false;
    console.error(error);
  }
}

function onMouseMove(event: MouseEvent) {
  if (!pressed) return;
  if (!(event.buttons & 1)) {
    pressed = null;
  } else if (Math.hypot(event.clientX - pressed.x, event.clientY - pressed.y) > 6) {
    const { file } = pressed;
    pressed = null;
    dragOut(file);
  }
}

function setDropHighlight(on: boolean) {
  const content = $("#content");
  content.classList.toggle("drop-target", on && !!current && !content.querySelector(".message.error"));
  if (current) content.dataset.drop = t("drop.target", { folder: current.name });
}

async function pasteInto() {
  const dest = await open({ directory: true, defaultPath: current?.path, title: t("paste.into") });
  if (typeof dest === "string") await paste(dest);
}

/** Atalhos da listagem; ignorados com diálogo aberto ou foco em campo de texto. */
function quit() {
  invoke("quit");
}

function onKeyDown(event: KeyboardEvent) {
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "q") {
    event.preventDefault();
    quit(); // vale também com diálogo aberto ou campo em foco
    return;
  }
  const target = event.target as HTMLElement;
  if (current && (event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "f" && !document.querySelector("dialog[open]")) {
    event.preventDefault();
    const search = $<HTMLInputElement>("#search");
    search.focus();
    search.select();
    return;
  }
  if (!current || document.querySelector("dialog[open]") || target.closest("input, select, textarea")) return;
  const ctrl = event.ctrlKey || event.metaKey;
  const key = event.key.toLowerCase();
  if (event.key === "F2") renameSelected();
  else if (event.key === "Enter" && selected.size && !target.closest("button")) openFiles(visible.filter((f) => selected.has(f.path)).map((f) => f.path));
  else if (event.key === "Delete") trashSelected();
  else if (ctrl && key === "c") toClipboard("copy");
  else if (ctrl && key === "x") toClipboard("cut");
  else if (ctrl && key === "v") paste(current.path);
  else if (ctrl && key === "a") {
    selected = new Set(visible.map((f) => f.path));
    renderFiles();
  } else if (event.key === "Escape" && selected.size) {
    selected.clear();
    renderFiles();
  } else return;
  event.preventDefault();
}

function select(options: [string, string][], value: string, onChange: (value: string) => void) {
  const node = el("select");
  for (const [optionValue, label] of options) node.append(el("option", { value: optionValue }, label));
  node.value = value;
  node.addEventListener("change", () => onChange(node.value));
  return node;
}

/** Ano/mês escolhidos que não existem nos arquivos atuais voltam para "todos". */
function normalizeDateFilter() {
  const years = yearsOf(files);
  if (typeof filters.year === "number" && !years.includes(filters.year)) filters.year = null;
  if (filters.year === "none" && !files.some((f) => f.date === null)) filters.year = null;
  if (filters.month !== null && (filters.year === "none" || !monthsOf(files, filters.year).includes(filters.month))) {
    filters.month = null;
  }
}

/** Grupo de filtros com título que recolhe/mostra o conteúdo; recolhido, mostra `badge` (ex.: termos marcados). */
function filterGroup(key: string, title: string, badge: string, content: HTMLElement) {
  const isCollapsed = collapsed.has(key);
  const toggle = el(
    "button",
    { type: "button", class: "group-toggle", "aria-expanded": String(!isCollapsed) },
    el("span", { class: "group-arrow" }, isCollapsed ? "▸" : "▾"),
    title,
    isCollapsed && badge ? el("span", { class: "group-badge" }, ` · ${badge}`) : "",
  );
  toggle.addEventListener("click", () => {
    if (!collapsed.delete(key)) collapsed.add(key);
    saveCollapsed();
    renderFilters();
  });
  return el("div", { class: "filter-group" }, el("h3", {}, toggle), isCollapsed ? "" : content);
}

function renderFilters() {
  const panel = $("#filters");
  panel.replaceChildren();
  $("#clear-filters").hidden = isEmpty(filters);
  const search = $<HTMLInputElement>("#search");
  if (search.value !== filters.search) search.value = filters.search; // ex.: após "Limpar filtros"
  const sections = state?.index.sections ?? [];
  if (sections.length === 0) {
    panel.append(el("p", { class: "hint" }, t("filters.emptyIndex")));
  }
  if (!current || files.length === 0) return;
  if (duplicates) {
    panel.replaceChildren(el("p", { class: "hint" }, t("filters.suspended")));
    return;
  }

  const yearOptions: [string, string][] = [["", t("filters.allYears")], ...yearsOf(files).map((y): [string, string] => [String(y), String(y)])];
  if (files.some((f) => f.date === null)) yearOptions.push(["none", t("filters.noNameDate")]);
  const monthOptions: [string, string][] = [
    ["", t("filters.allMonths")],
    ...monthsOf(files, filters.year).map((m): [string, string] => [String(m), monthName(m)]),
  ];
  const yearSelect = select(yearOptions, filters.year === null ? "" : String(filters.year), (value) => {
    filters.year = value === "" ? null : value === "none" ? "none" : (Number(value) as YearFilter);
    normalizeDateFilter();
    update();
  });
  const monthSelect = select(monthOptions, filters.month === null ? "" : String(filters.month), (value) => {
    filters.month = value === "" ? null : Number(value);
    update();
  });
  monthSelect.disabled = filters.year === "none";
  const dateActive = filters.year !== null || filters.month !== null;
  panel.append(filterGroup("date", t("filters.date"), dateActive ? t("filters.dateActive") : "", el("div", { class: "date-selects" }, yearSelect, monthSelect)));

  sections.forEach((section, s) => {
    const chips = el("div", { class: "filter-chips" });
    section.terms.forEach((term, position) => {
      const key = termKey(s, position);
      const count = files.filter((f) => f.terms[s]?.includes(position)).length;
      const chip = el(
        "button",
        { type: "button", class: `chip toggle s${s % 6}`, title: term.synonyms.length ? t("filters.also", { list: term.synonyms.join(", ") }) : "" },
        term.name,
        el("span", { class: "chip-count" }, String(count)),
      );
      chip.classList.toggle("selected", filters.terms.has(key));
      chip.addEventListener("click", () => {
        if (!filters.terms.delete(key)) filters.terms.add(key);
        update();
      });
      chips.append(chip);
    });
    const marked = section.terms.filter((_, t) => filters.terms.has(termKey(s, t))).length;
    panel.append(filterGroup(`section:${section.name}`, section.name, marked ? String(marked) : "", chips));
  });
}

/** Redesenha filtros e listagem a partir do estado atual. */
function update() {
  renderFilters();
  renderFiles();
}

async function selectFolder(folder: Folder) {
  if (duplicates) exitDuplicates();
  current = folder;
  files = [];
  selected.clear();
  anchor = null;
  saveLastFolder(folder.name);
  renderFolders();
  renderFilters();
  renderHeader(null);
  $("#toolbar").replaceChildren();
  renderMessage(t("list.loading"));
  $("#content").scrollTop = 0;
  invoke("watch_folder", { path: folder.path }).catch((error) => console.error(error));
  await reload(); // ignora o resultado se outra pasta for escolhida enquanto carrega
}

/** Aplica um estado novo mantendo a pasta atual (pelo nome) quando ela ainda existe. */
/** Configuração alterada fora do programa: recarrega mantendo a pasta aberta, a seleção e os
 * filtros marcados (pelo nome da seção e do termo; termos que sumiram são desmarcados). */
async function reloadState() {
  let next: State;
  try {
    next = await invoke<State>("load_state");
  } catch (error) {
    console.error(error);
    return;
  }
  const sections = state?.index.sections ?? [];
  const marked = new Set(
    [...filters.terms].map((key) => {
      const [s, t] = key.split(":").map(Number);
      return `${sections[s]?.name}\u0000${sections[s]?.terms[t]?.name}`;
    }),
  );
  filters.terms = new Set();
  next.index.sections.forEach((section, s) =>
    section.terms.forEach((term, t) => {
      if (marked.has(`${section.name}\u0000${term.name}`)) filters.terms.add(termKey(s, t));
    }),
  );
  const open = current;
  const same = open && next.folders.find((f) => f.name === open.name && f.path === open.path);
  state = next;
  if (same) {
    current = same; // descrição pode ter mudado
    renderFolders();
    await reload();
    return;
  }
  if (open && !next.folders.some((f) => f.name === open.name)) {
    await message(t("config.folderGone", { name: open.name }), { title: t("config.changedTitle"), kind: "warning" });
  }
  applyState(next);
}

function applyState(next: State) {
  state = next;
  const name = current?.name ?? readLastFolder();
  const folder = state.folders.find((f) => f.name === name) ?? state.folders[0];
  current = null;
  if (folder) {
    selectFolder(folder);
  } else {
    files = [];
    invoke("watch_folder", { path: "" }).catch((error) => console.error(error)); // para de observar
    renderFolders();
    renderFilters();
    renderHeader(null);
    renderMessage(t("list.start"));
  }
}

/** Textos fixos do index.html, marcados com data-i18n (texto), data-i18n-placeholder e data-i18n-label. */
function translatePage() {
  for (const node of document.querySelectorAll<HTMLElement>("[data-i18n]")) node.textContent = t(node.dataset.i18n as Key);
  for (const node of document.querySelectorAll<HTMLElement>("[data-i18n-placeholder]")) {
    node.setAttribute("placeholder", t(node.dataset.i18nPlaceholder as Key));
  }
  for (const node of document.querySelectorAll<HTMLElement>("[data-i18n-label]")) {
    node.setAttribute("aria-label", t(node.dataset.i18nLabel as Key));
  }
}

async function init() {
  $("#edit-folders").addEventListener("click", () => openFoldersEditor(state?.folders ?? [], state?.versions.paths ?? "", applyState));
  $("#edit-index").addEventListener("click", () =>
    openIndexEditor(state?.index.sections ?? [], state?.versions.index ?? "", (next) => {
      filters.terms.clear(); // posições dos termos podem ter mudado
      applyState(next);
    }),
  );
  $("#open-settings").addEventListener("click", () => {
    if (state) openSettings(state, applyState);
  });
  $("#open-about").addEventListener("click", openAbout);
  $("#quit").addEventListener("click", quit);
  const search = $<HTMLInputElement>("#search");
  search.addEventListener("input", () => {
    filters.search = search.value;
    update();
  });
  search.addEventListener("keydown", (event) => {
    if (event.key !== "Escape") return;
    event.preventDefault();
    if (search.value) {
      search.value = "";
      filters.search = "";
      update();
    } else {
      search.blur(); // volta aos atalhos da lista
    }
  });
  $("#clear-filters").addEventListener("click", () => {
    filters = emptyFilters();
    update();
  });
  document.addEventListener("keydown", onKeyDown);
  window.addEventListener("focus", refreshClipboard); // o usuário pode ter copiado no gerenciador de arquivos
  await getCurrentWebview().onDragDropEvent((event) => {
    const { type } = event.payload;
    if (draggingOut) return setDropHighlight(false); // arrasto saindo do próprio manabox
    setDropHighlight(type === "enter" || type === "over");
    if (type === "drop") dropFiles(event.payload.paths);
  });
  document.addEventListener("mousemove", onMouseMove);
  document.addEventListener("mouseup", () => (pressed = null));
  await listen("drag-out-ended", () => (draggingOut = false));
  await listen("config-changed", reloadState);
  await listen<[number, number]>("duplicates-progress", (event) => {
    if (duplicates?.groups !== null || !duplicates) return;
    duplicates.progress = event.payload;
    renderFiles();
  });
  await listen<string>("folder-changed", (event) => {
    if (event.payload === current?.path) reload();
  });
  try {
    let loaded = await invoke<State>("load_state");
    if (loaded.config.language) {
      setLang(loaded.config.language);
    } else {
      // sem idioma escolhido (primeira execução, ou versão anterior): pergunta, com o do sistema marcado
      const language = await chooseLanguage(systemLang());
      await invoke("set_language", { language });
      setLang(language);
      loaded = loaded.first_run ? await invoke<State>("create_examples") : await invoke<State>("load_state");
    }
    translatePage();
    applyState(loaded);
    refreshClipboard();
  } catch (error) {
    renderMessage(t("config.loadError", { error: String(error) }), "error");
  }
}

window.addEventListener("DOMContentLoaded", init);
