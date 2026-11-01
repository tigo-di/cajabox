mod classify;
mod clipboard;
mod config;
#[cfg(target_os = "linux")]
mod dragout;
mod fileops;
mod index;
mod markdown;
mod paths;
mod scan;
mod watch;

use std::fs;
use std::io;
use std::path::Path;

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_opener::OpenerExt;

use classify::Classifier;
use config::Config;
use scan::FileEntry;

#[derive(Serialize)]
struct State {
    config: Config,
    /// Pasta padrão de manabox.index.md e manabox.paths.md (`~/.config/manabox`).
    default_dir: String,
    index: index::Index,
    folders: Vec<paths::Folder>,
}

fn load() -> Result<(Config, index::Index), String> {
    let config = Config::load(&config::default_dir()).map_err(|e| e.to_string())?;
    let index = index::parse(&fs::read_to_string(&config.index).map_err(|e| e.to_string())?);
    Ok((config, index))
}

/// Configuração, índice e pastas atuais.
#[tauri::command]
fn load_state() -> Result<State, String> {
    let (config, index) = load()?;
    let folders = paths::parse(&fs::read_to_string(&config.paths).map_err(|e| e.to_string())?);
    let default_dir = config::default_dir().display().to_string();
    Ok(State { config, default_dir, index, folders })
}

/// Grava a lista de pastas no manabox.paths.md e devolve o estado atualizado.
#[tauri::command]
fn save_folders(folders: Vec<paths::Folder>) -> Result<State, String> {
    paths::validate(&folders)?;
    let (config, _) = load()?;
    fs::write(&config.paths, paths::serialize(&folders)).map_err(|e| format!("Erro ao gravar {}: {e}", config.paths.display()))?;
    load_state()
}

/// Grava o índice no manabox.index.md e devolve o estado atualizado (padrões recalculados na releitura).
#[tauri::command]
fn save_index(index: index::Index) -> Result<State, String> {
    index::validate(&index)?;
    let (config, _) = load()?;
    fs::write(&config.index, index::serialize(&index)).map_err(|e| format!("Erro ao gravar {}: {e}", config.index.display()))?;
    load_state()
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
enum MoveResult {
    Moved { state: State },
    Exists { path: String },
}

/// Move manabox.index.md ou manabox.paths.md para outra pasta. Sem `overwrite`, conflito volta como `exists`.
#[tauri::command]
fn move_config_file(file: config::ConfigFile, dest_dir: String, overwrite: bool) -> Result<MoveResult, String> {
    let dir = config::default_dir();
    let mut config = Config::load(&dir).map_err(|e| e.to_string())?;
    match config.move_file(file, Path::new(&dest_dir), overwrite, &dir) {
        Ok(config::MoveOutcome::Moved) => Ok(MoveResult::Moved { state: load_state()? }),
        Ok(config::MoveOutcome::Exists(path)) => Ok(MoveResult::Exists { path: path.display().to_string() }),
        Err(e) => Err(format!("Erro ao mover para {dest_dir}: {e}")),
    }
}

/// Renomeia um arquivo; devolve o caminho novo.
#[tauri::command]
fn rename_file(path: String, new_name: String) -> Result<String, String> {
    fileops::rename(Path::new(&path), &new_name).map(|p| p.display().to_string())
}

/// Envia arquivos para a lixeira do sistema.
#[tauri::command]
fn trash_files(paths: Vec<String>) -> Result<(), String> {
    trash::delete_all(&paths).map_err(|e| format!("Erro ao enviar para a lixeira: {e}"))
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
enum FileResult {
    Done { path: String },
    Exists { path: String },
}

impl From<fileops::Outcome> for FileResult {
    fn from(outcome: fileops::Outcome) -> Self {
        match outcome {
            fileops::Outcome::Done(to) => FileResult::Done { path: to.display().to_string() },
            fileops::Outcome::Exists(to) => FileResult::Exists { path: to.display().to_string() },
        }
    }
}

/// Move um arquivo para outra pasta. Sem `overwrite`, conflito volta como `exists`.
#[tauri::command]
fn move_file(path: String, dest_dir: String, overwrite: bool) -> Result<FileResult, String> {
    fileops::move_to(Path::new(&path), Path::new(&dest_dir), overwrite)
        .map(FileResult::from)
        .map_err(|e| format!("Erro ao mover {path}: {e}"))
}

/// Copia um arquivo para outra pasta. Sem `overwrite`, conflito volta como `exists`.
#[tauri::command]
fn copy_file(path: String, dest_dir: String, overwrite: bool) -> Result<FileResult, String> {
    fileops::copy_to(Path::new(&path), Path::new(&dest_dir), overwrite)
        .map(FileResult::from)
        .map_err(|e| format!("Erro ao copiar {path}: {e}"))
}

/// Passa a observar `path` (e para a pasta anterior); mudanças emitem o evento `folder-changed` com o caminho.
#[tauri::command]
fn watch_folder(path: String, app: AppHandle, watcher: tauri::State<'_, watch::FolderWatcher>) -> Result<(), String> {
    let changed = path.clone();
    watcher.watch(Path::new(&path), move || {
        let _ = app.emit("folder-changed", &changed);
    })
}

#[derive(Serialize)]
struct FileList {
    /// Arquivos existentes.
    paths: Vec<String>,
    /// Pastas e caminhos inexistentes (o manabox opera só arquivos).
    skipped: Vec<String>,
}

impl FileList {
    fn new(paths: impl IntoIterator<Item = std::path::PathBuf>) -> Self {
        let (files, skipped): (Vec<_>, Vec<_>) = paths.into_iter().partition(|p| p.is_file());
        let text = |paths: Vec<std::path::PathBuf>| paths.iter().map(|p| p.display().to_string()).collect();
        FileList { paths: text(files), skipped: text(skipped) }
    }
}

/// Separa arquivos de pastas e caminhos inexistentes (ex.: itens soltos na janela).
#[tauri::command]
fn split_files(paths: Vec<String>) -> FileList {
    FileList::new(paths.into_iter().map(Into::into))
}

#[derive(Serialize)]
struct ClipboardFiles {
    mode: clipboard::Mode,
    #[serde(flatten)]
    files: FileList,
}

/// Coloca arquivos na área de transferência do sistema (copiar ou recortar).
/// Síncrono de propósito: roda na thread principal, exigida pelo GTK.
#[tauri::command]
fn clipboard_set(paths: Vec<String>, mode: clipboard::Mode) -> Result<(), String> {
    clipboard::system::set(clipboard::Contents { mode, paths: paths.into_iter().map(Into::into).collect() })
}

/// Arquivos na área de transferência do sistema, vindos do manabox ou de um gerenciador de arquivos.
#[tauri::command]
fn clipboard_get() -> Option<ClipboardFiles> {
    let contents = clipboard::system::get()?;
    Some(ClipboardFiles { mode: contents.mode, files: FileList::new(contents.paths) })
}

/// Esvazia a área de transferência do sistema.
#[tauri::command]
fn clipboard_clear() {
    clipboard::system::clear();
}

/// Inicia o arrasto de arquivos para outro programa; emite `drag-out-ended` ao terminar.
/// Síncrono de propósito: roda na thread principal, exigida pelo GTK.
#[cfg(target_os = "linux")]
#[tauri::command]
fn drag_files(paths: Vec<String>, window: tauri::WebviewWindow, app: AppHandle) -> Result<(), String> {
    let gtk_window = window.gtk_window().map_err(|e| e.to_string())?;
    let paths: Vec<std::path::PathBuf> = paths.into_iter().map(Into::into).collect();
    dragout::start(&gtk_window, &paths, move || {
        let _ = app.emit("drag-out-ended", ());
    })
}

/// Abre uma pasta no gerenciador de arquivos padrão. Feito no Rust para a interface não
/// precisar de permissão do plugin opener para abrir caminhos quaisquer.
#[tauri::command]
fn open_folder(path: String, app: AppHandle) -> Result<(), String> {
    if !Path::new(&path).is_dir() {
        return Err(format!("Não foi possível encontrar o caminho: {path}"));
    }
    app.opener()
        .open_path(&path, None::<&str>)
        .map_err(|e| format!("Não foi possível abrir {path}: {e}"))
}

/// Encerra o programa.
#[tauri::command]
fn quit(app: AppHandle) {
    app.exit(0);
}

/// Arquivos classificados de uma pasta.
#[tauri::command]
fn list_folder(path: String) -> Result<Vec<FileEntry>, String> {
    let (_, index) = load()?;
    scan::list_files(Path::new(&path), &Classifier::new(&index)).map_err(|e| match e.kind() {
        io::ErrorKind::NotFound => format!("Não foi possível encontrar o caminho: {path}"),
        _ => format!("{path}: {e}"),
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(watch::FolderWatcher::default())
        .invoke_handler(tauri::generate_handler![load_state, save_folders, save_index, move_config_file, list_folder, rename_file, trash_files, move_file, copy_file, watch_folder, clipboard_set, clipboard_get, clipboard_clear, split_files, drag_files, open_folder, quit])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
