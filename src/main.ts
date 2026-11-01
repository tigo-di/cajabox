import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { ask, message, open } from "@tauri-apps/plugin-dialog";

import { openAbout } from "./about";
import { $, button, choose, el, promptText } from "./dom";
import { emptyFilters, isEmpty, matches, monthsOf, termKey, yearsOf, type YearFilter } from "./filter";
import { openFoldersEditor } from "./folders-editor";
import { openIndexEditor } from "./index-editor";
import { openSettings } from "./settings";
import type { FileDate, FileEntry, Folder, State } from "./types";

const LAST_FOLDER_KEY = "manabox.lastFolder";
const MONTHS = [
  "janeiro", "fevereiro", "março", "abril", "maio", "junho",
  "julho", "agosto", "setembro", "outubro", "novembro", "dezembro",
];

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

type FileResult = { status: "done" | "exists"; path: string };

function formatDate(date: FileDate): string {
  const mm = String(date.month).padStart(2, "0");
  return date.day === null ? `${mm}/${date.year}` : `${String(date.day).padStart(2, "0")}/${mm}/${date.year}`;
}

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
  const noun = total === 1 ? "arquivo" : "arquivos";
  return shown === total ? `${total} ${noun}` : `${shown} de ${total} ${noun}`;
}

/** `shown` null = ainda carregando. */
function renderHeader(shown: number | null) {
  const header = $("#folder-header");
  header.replaceChildren();
  if (!current) return;
  header.append(
    el(
      "div",
      { class: "title-row" },
      el("h1", {}, current.name),
      shown === null ? "" : el("span", { class: "count" }, countLabel(shown, files.length)),
    ),
    current.description ? el("p", { class: "desc" }, current.description) : "",
    el("p", { class: "path" }, current.path),
  );
}

function renderMessage(text: string, kind: "hint" | "error" = "hint") {
  $("#content").replaceChildren(el("p", { class: `message ${kind}` }, text));
}

function termChips(file: FileEntry): HTMLElement {
  const cell = el("td", { class: "terms" });
  const sections = state?.index.sections ?? [];
  file.terms.forEach((found, s) => {
    for (const t of found) {
      const section = sections[s];
      cell.append(
        el("span", { class: `chip s${s % 6}`, title: section?.name ?? "" }, section?.terms[t]?.name ?? "?"),
      );
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
  const rename = button("Renomear", renameSelected);
  rename.disabled = count !== 1;
  rename.title = "F2";
  const copy = button("Copiar", () => toClipboard("copy"));
  copy.disabled = count === 0;
  copy.title = "Ctrl+C";
  const cut = button("Recortar", () => toClipboard("cut"));
  cut.disabled = count === 0;
  cut.title = "Ctrl+X";
  const remove = button("Excluir", trashSelected, "danger");
  remove.disabled = count === 0;
  remove.title = "Delete";
  const info = count ? `${count} ${count === 1 ? "selecionado" : "selecionados"}` : "Nenhum selecionado";
  bar.append(el("div", { class: "toolbar-group" }, el("span", { class: "toolbar-info" }, info), rename, copy, cut, remove));

  const pending = clipboard.paths.length;
  if (pending) {
    const label = clipboard.mode === "copy" ? ["copiado", "copiados"] : ["recortado", "recortados"];
    const here = button("Colar aqui", () => current && paste(current.path), "primary");
    here.title = "Ctrl+V — na raiz da pasta aberta";
    bar.append(
      el(
        "div",
        { class: "toolbar-group" },
        el("span", { class: "toolbar-info" }, `${pending} ${label[pending === 1 ? 0 : 1]}`),
        here,
        button("Colar em…", pasteInto),
        button("Cancelar", () => {
          setClipboard([], "copy");
        }),
      ),
    );
  }
}

function renderFiles() {
  visible = files.filter((f) => matches(f, filters));
  const shown = new Set(visible.map((f) => f.path));
  selected = new Set([...selected].filter((p) => shown.has(p)));
  renderHeader(visible.length);
  renderToolbar();
  if (files.length === 0) {
    renderMessage("Pasta vazia.");
    return;
  }
  if (visible.length === 0) {
    renderMessage("Nenhum arquivo corresponde aos filtros.");
    return;
  }
  const body = el("tbody");
  for (const file of visible) {
    const slash = file.relative_path.lastIndexOf("/");
    const row = el(
      "tr",
      { title: file.path },
      el("td", { class: "name" }, file.name),
      el("td", { class: "subdir" }, slash > 0 ? file.relative_path.slice(0, slash) : ""),
      termChips(file),
      el("td", { class: "date" }, file.date ? formatDate(file.date) : ""),
    );
    row.classList.toggle("selected", selected.has(file.path));
    row.classList.toggle("cut", clipboard.mode === "cut" && clipboard.paths.includes(file.path));
    row.addEventListener("click", (event) => clickRow(file, event));
    body.append(row);
  }
  const table = el(
    "table",
    { class: "files" },
    el(
      "thead",
      {},
      el("tr", {}, el("th", {}, "Nome"), el("th", {}, "Subpasta"), el("th", {}, "Termos"), el("th", {}, "Data")),
    ),
    body,
  );
  const content = $("#content");
  const scroll = content.scrollTop;
  content.replaceChildren(table);
  content.scrollTop = scroll;
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
  } catch (error) {
    if (current !== folder) return;
    $("#filters").replaceChildren();
    $("#toolbar").replaceChildren();
    renderMessage(String(error), "error");
  }
}

async function renameSelected() {
  if (selected.size !== 1) return;
  const path = [...selected][0];
  const name = path.slice(path.lastIndexOf("/") + 1);
  const dot = name.lastIndexOf(".");
  const newName = await promptText("Renomear", name, dot > 0 ? dot : name.length);
  if (newName === null || newName === name) return;
  try {
    const newPath = await invoke<string>("rename_file", { path, newName });
    if (clipboard.paths.includes(path)) {
      await setClipboard(clipboard.paths.map((p) => (p === path ? newPath : p)), clipboard.mode);
    }
    selected = new Set([newPath]);
    anchor = newPath;
  } catch (error) {
    await message(String(error), { title: "Renomear", kind: "error" });
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
    await message(String(error), { title: "Área de transferência", kind: "error" });
  }
  renderFiles();
}

function toClipboard(mode: "copy" | "cut") {
  if (selected.size) setClipboard([...selected], mode);
}

async function trashSelected() {
  const paths = [...selected];
  if (paths.length === 0) return;
  const what = paths.length === 1 ? `“${paths[0].slice(paths[0].lastIndexOf("/") + 1)}”` : `${paths.length} arquivos`;
  if (!(await ask(`Enviar ${what} para a lixeira?`, { title: "Excluir", kind: "warning" }))) return;
  try {
    await invoke("trash_files", { paths });
    if (clipboard.paths.some((p) => paths.includes(p))) {
      await setClipboard(clipboard.paths.filter((p) => !paths.includes(p)), clipboard.mode);
    }
    selected.clear();
  } catch (error) {
    await message(String(error), { title: "Excluir", kind: "error" });
  }
  await reload();
}

async function warnSkipped(skipped: string[]) {
  if (skipped.length) await message(`Só arquivos podem ser colados. Ignorados:\n${skipped.join("\n")}`, { title: "Colar", kind: "warning" });
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
        const replace = await ask(`Já existe ${result.path}.\nSubstituir?`, { title: "Arquivo já existe", kind: "warning" });
        if (!replace) continue;
        result = await invoke<FileResult>(command, { path, destDir: dest, overwrite: true });
      }
      pasted.push(result.path);
    } catch (error) {
      errors.push(String(error));
    }
  }
  selected = new Set(pasted);
  if (errors.length) await message(errors.join("\n"), { title: "Colar", kind: "error" });
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
  const what = paths.length === 1 ? `“${paths[0].slice(paths[0].lastIndexOf("/") + 1)}”` : `${paths.length} arquivos`;
  const choice = await choose("Soltar arquivos", `${what} em ${folder.name}`, ["Copiar aqui", "Mover aqui"]);
  if (choice) await transfer(paths, choice === "Copiar aqui" ? "copy" : "cut", folder.path);
}

function setDropHighlight(on: boolean) {
  const content = $("#content");
  content.classList.toggle("drop-target", on && !!current && !content.querySelector(".message.error"));
  if (current) content.dataset.drop = `Soltar em ${current.name}`;
}

async function pasteInto() {
  const dest = await open({ directory: true, defaultPath: current?.path, title: "Colar em…" });
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
  if (!current || document.querySelector("dialog[open]") || target.closest("input, select, textarea")) return;
  const ctrl = event.ctrlKey || event.metaKey;
  const key = event.key.toLowerCase();
  if (event.key === "F2") renameSelected();
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

function renderFilters() {
  const panel = $("#filters");
  panel.replaceChildren();
  $("#clear-filters").hidden = isEmpty(filters);
  const sections = state?.index.sections ?? [];
  if (sections.length === 0) {
    panel.append(el("p", { class: "hint" }, "Índice vazio. Use “Editar índice” para criar seções e termos."));
  }
  if (!current || files.length === 0) return;

  const yearOptions: [string, string][] = [["", "Todos os anos"], ...yearsOf(files).map((y): [string, string] => [String(y), String(y)])];
  if (files.some((f) => f.date === null)) yearOptions.push(["none", "Sem data"]);
  const monthOptions: [string, string][] = [
    ["", "Todos os meses"],
    ...monthsOf(files, filters.year).map((m): [string, string] => [String(m), MONTHS[m - 1]]),
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
  panel.append(el("div", { class: "filter-group" }, el("h3", {}, "Data"), el("div", { class: "date-selects" }, yearSelect, monthSelect)));

  sections.forEach((section, s) => {
    const group = el("div", { class: "filter-group" }, el("h3", {}, section.name));
    const chips = el("div", { class: "filter-chips" });
    section.terms.forEach((term, t) => {
      const key = termKey(s, t);
      const count = files.filter((f) => f.terms[s]?.includes(t)).length;
      const chip = el(
        "button",
        { type: "button", class: `chip toggle s${s % 6}`, title: term.synonyms.length ? `também: ${term.synonyms.join(", ")}` : "" },
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
    group.append(chips);
    panel.append(group);
  });
}

/** Redesenha filtros e listagem a partir do estado atual. */
function update() {
  renderFilters();
  renderFiles();
}

async function selectFolder(folder: Folder) {
  current = folder;
  files = [];
  selected.clear();
  anchor = null;
  saveLastFolder(folder.name);
  renderFolders();
  renderFilters();
  renderHeader(null);
  $("#toolbar").replaceChildren();
  renderMessage("Carregando…");
  $("#content").scrollTop = 0;
  invoke("watch_folder", { path: folder.path }).catch((error) => console.error(error));
  await reload(); // ignora o resultado se outra pasta for escolhida enquanto carrega
}

/** Aplica um estado novo mantendo a pasta atual (pelo nome) quando ela ainda existe. */
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
    renderMessage("Cadastre pastas para começar.");
  }
}

async function init() {
  $("#edit-folders").addEventListener("click", () => openFoldersEditor(state?.folders ?? [], applyState));
  $("#edit-index").addEventListener("click", () =>
    openIndexEditor(state?.index.sections ?? [], (next) => {
      filters.terms.clear(); // posições dos termos podem ter mudado
      applyState(next);
    }),
  );
  $("#open-settings").addEventListener("click", () => {
    if (state) openSettings(state, applyState);
  });
  $("#open-about").addEventListener("click", openAbout);
  $("#quit").addEventListener("click", quit);
  $("#clear-filters").addEventListener("click", () => {
    filters = emptyFilters();
    update();
  });
  document.addEventListener("keydown", onKeyDown);
  window.addEventListener("focus", refreshClipboard); // o usuário pode ter copiado no gerenciador de arquivos
  await getCurrentWebview().onDragDropEvent((event) => {
    const { type } = event.payload;
    setDropHighlight(type === "enter" || type === "over");
    if (type === "drop") dropFiles(event.payload.paths);
  });
  await listen<string>("folder-changed", (event) => {
    if (event.payload === current?.path) reload();
  });
  try {
    applyState(await invoke<State>("load_state"));
    refreshClipboard();
  } catch (error) {
    renderMessage(`Erro ao carregar configuração: ${error}`, "error");
  }
}

window.addEventListener("DOMContentLoaded", init);
