mod classify;
mod config;
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
enum FileMoveResult {
    Moved { path: String },
    Exists { path: String },
}

/// Move um arquivo para outra pasta. Sem `overwrite`, conflito volta como `exists`.
#[tauri::command]
fn move_file(path: String, dest_dir: String, overwrite: bool) -> Result<FileMoveResult, String> {
    match fileops::move_to(Path::new(&path), Path::new(&dest_dir), overwrite) {
        Ok(fileops::MoveOutcome::Moved(to)) => Ok(FileMoveResult::Moved { path: to.display().to_string() }),
        Ok(fileops::MoveOutcome::Exists(to)) => Ok(FileMoveResult::Exists { path: to.display().to_string() }),
        Err(e) => Err(format!("Erro ao mover {path}: {e}")),
    }
}

/// Passa a observar `path` (e para a pasta anterior); mudanças emitem o evento `folder-changed` com o caminho.
#[tauri::command]
fn watch_folder(path: String, app: AppHandle, watcher: tauri::State<'_, watch::FolderWatcher>) -> Result<(), String> {
    let changed = path.clone();
    watcher.watch(Path::new(&path), move || {
        let _ = app.emit("folder-changed", &changed);
    })
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
        .invoke_handler(tauri::generate_handler![load_state, save_folders, save_index, move_config_file, list_folder, rename_file, trash_files, move_file, watch_folder, quit])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
