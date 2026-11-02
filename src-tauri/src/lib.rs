mod classify;
mod clipboard;
mod config;
mod duplicates;
mod examples;
#[cfg(target_os = "linux")]
mod dragout;
mod fileops;
mod i18n;
mod index;
mod launch;
mod markdown;
mod paths;
mod scan;
mod watch;

use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::io;
use std::path::Path;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use classify::Classifier;
use config::Config;

#[derive(Serialize)]
struct State {
    config: Config,
    /// Pasta padrão de cajabox.index.md e cajabox.paths.md (`~/.config/cajabox`).
    default_dir: String,
    index: index::Index,
    folders: Vec<paths::Folder>,
    /// Versão (hash do conteúdo) de cada arquivo lido: o editor interno a devolve ao salvar.
    versions: Versions,
    /// Pasta das pastas de exemplo no idioma atual.
    example_dir: String,
    /// Pastas de exemplo de todos os idiomas (aviso na interface).
    example_dirs: Vec<String>,
    /// Primeira execução: a interface pergunta o idioma e então cria os exemplos.
    first_run: bool,
}

/// Primeira execução (sem índice nem pastas ao abrir), até os exemplos serem criados.
#[derive(Default)]
struct FirstRun(AtomicBool);

#[derive(Serialize)]
struct Versions {
    index: String,
    paths: String,
}

/// Hash do conteúdo do arquivo, para saber se ele mudou desde que foi lido.
fn version(path: &Path) -> Result<String, String> {
    use std::hash::{Hash, Hasher};
    let content = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    content.hash(&mut hasher);
    Ok(format!("{:016x}", hasher.finish()))
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
enum SaveResult {
    Saved { state: State },
    /// O arquivo mudou fora do programa desde a versão esperada; nada foi gravado.
    Changed,
}

/// Grava `content` em `path` se o arquivo ainda está na versão `expected` (ou com `overwrite`).
fn save_checked(path: &Path, content: String, expected: &str, overwrite: bool) -> Result<SaveResult, String> {
    if !overwrite && version(path)? != expected {
        return Ok(SaveResult::Changed);
    }
    fs::write(path, content).map_err(|e| tr!("Erro ao gravar {}: {e}", "Error writing {}: {e}", path.display()))?;
    Ok(SaveResult::Saved { state: read_state(false)? })
}

fn load() -> Result<(Config, index::Index), String> {
    let config = Config::load(&config::default_dir()).map_err(|e| e.to_string())?;
    let index = index::parse(&fs::read_to_string(&config.index).map_err(|e| e.to_string())?);
    Ok((config, index))
}

/// Configuração, índice e pastas atuais.
#[tauri::command]
fn load_state(first_run: tauri::State<'_, FirstRun>) -> Result<State, String> {
    read_state(first_run.0.load(Ordering::Relaxed))
}

fn read_state(first_run: bool) -> Result<State, String> {
    let (config, index) = load()?;
    let folders = paths::parse(&fs::read_to_string(&config.paths).map_err(|e| e.to_string())?);
    let default_dir = config::default_dir().display().to_string();
    let versions = Versions { index: version(&config.index)?, paths: version(&config.paths)? };
    let example_dirs = [i18n::Lang::Pt, i18n::Lang::En].map(|lang| examples::base_dir(lang).display().to_string()).to_vec();
    let example_dir = examples::base_dir(i18n::current()).display().to_string();
    Ok(State { config, default_dir, index, folders, versions, example_dir, example_dirs, first_run })
}

/// Grava a lista de pastas no cajabox.paths.md e devolve o estado atualizado. Se o arquivo mudou
/// fora do programa desde `expected`, sem `overwrite` devolve `changed` sem gravar.
#[tauri::command]
fn save_folders(folders: Vec<paths::Folder>, expected: String, overwrite: bool) -> Result<SaveResult, String> {
    paths::validate(&folders)?;
    let (config, _) = load()?;
    save_checked(&config.paths, paths::serialize(&folders), &expected, overwrite)
}

/// Grava o índice no cajabox.index.md (padrões recalculados na releitura); conflito como em `save_folders`.
#[tauri::command]
fn save_index(index: index::Index, expected: String, overwrite: bool) -> Result<SaveResult, String> {
    index::validate(&index)?;
    let (config, _) = load()?;
    save_checked(&config.index, index::serialize(&index), &expected, overwrite)
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
enum MoveResult {
    Moved { state: State },
    Exists { path: String },
}

/// Passa a observar cajabox.index.md e cajabox.paths.md (nos locais atuais); mudança feita fora
/// do programa emite `config-changed`.
fn watch_config(app: &AppHandle) -> Result<(), String> {
    let config = Config::load(&config::default_dir()).map_err(|e| e.to_string())?;
    let emitter = app.clone();
    app.state::<watch::ConfigWatcher>().watch(&[config.index, config.paths], move || {
        let _ = emitter.emit("config-changed", ());
    })
}

/// Move cajabox.index.md ou cajabox.paths.md para outra pasta. Sem `overwrite`, conflito volta como `exists`.
#[tauri::command]
fn move_config_file(file: config::ConfigFile, dest_dir: String, overwrite: bool, app: AppHandle) -> Result<MoveResult, String> {
    let dir = config::default_dir();
    let mut config = Config::load(&dir).map_err(|e| e.to_string())?;
    match config.move_file(file, Path::new(&dest_dir), overwrite, &dir) {
        Ok(config::MoveOutcome::Moved) => {
            watch_config(&app)?; // observação acompanha o novo local
            Ok(MoveResult::Moved { state: read_state(false)? })
        }
        Ok(config::MoveOutcome::Exists(path)) => Ok(MoveResult::Exists { path: path.display().to_string() }),
        Err(e) => Err(tr!("Erro ao mover para {dest_dir}: {e}", "Error moving to {dest_dir}: {e}")),
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
    trash::delete_all(&paths).map_err(|e| tr!("Erro ao enviar para a lixeira: {e}", "Error moving to the trash: {e}"))
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
        .map_err(|e| tr!("Erro ao mover {path}: {e}", "Error moving {path}: {e}"))
}

/// Copia um arquivo para outra pasta. Sem `overwrite`, conflito volta como `exists`.
#[tauri::command]
fn copy_file(path: String, dest_dir: String, overwrite: bool) -> Result<FileResult, String> {
    fileops::copy_to(Path::new(&path), Path::new(&dest_dir), overwrite)
        .map(FileResult::from)
        .map_err(|e| tr!("Erro ao copiar {path}: {e}", "Error copying {path}: {e}"))
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
    /// Pastas e caminhos inexistentes (o cajabox opera só arquivos).
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

/// Arquivos na área de transferência do sistema, vindos do cajabox ou de um gerenciador de arquivos.
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
/// precisar de permissão para abrir caminhos quaisquer. Síncrono: GTK na thread principal.
#[tauri::command]
fn open_folder(path: String) -> Result<(), String> {
    if !Path::new(&path).is_dir() {
        return Err(tr!("Não foi possível encontrar o caminho: {path}", "Could not find the path: {path}"));
    }
    launch::open(Path::new(&path)).map_err(|e| tr!("Não foi possível abrir {path}: {e}", "Could not open {path}: {e}"))
}

/// Abre um arquivo com o programa padrão do sistema (mesmo motivo de `open_folder`).
#[tauri::command]
fn open_file(path: String) -> Result<(), String> {
    if !Path::new(&path).is_file() {
        return Err(tr!("Arquivo não encontrado: {path}", "File not found: {path}"));
    }
    launch::open(Path::new(&path)).map_err(|e| tr!("Não foi possível abrir {path}: {e}", "Could not open {path}: {e}"))
}

/// Grava o idioma escolhido (vale para as mensagens do núcleo na hora).
#[tauri::command]
fn set_language(language: i18n::Lang) -> Result<(), String> {
    let dir = config::default_dir();
    let mut config = Config::locate(&dir);
    config.language = Some(language);
    config.save(&dir).map_err(|e| e.to_string())?;
    i18n::set(language);
    Ok(())
}

/// Cria as pastas de exemplo e acrescenta à configuração o que falta (nada é sobrescrito).
#[tauri::command]
fn create_examples(first_run: tauri::State<'_, FirstRun>) -> Result<State, String> {
    first_run.0.store(false, Ordering::Relaxed);
    let (config, mut index) = load()?;
    let mut folders = paths::parse(&fs::read_to_string(&config.paths).map_err(|e| e.to_string())?);
    let base = examples::base_dir(i18n::current());
    examples::install(&base, i18n::current(), &mut index, &mut folders)
        .map_err(|e| tr!("Erro ao criar {}: {e}", "Error creating {}: {e}", base.display()))?;
    index::validate(&index)?;
    paths::validate(&folders)?;
    fs::write(&config.index, index::serialize(&index)).map_err(|e| tr!("Erro ao gravar {}: {e}", "Error writing {}: {e}", config.index.display()))?;
    fs::write(&config.paths, paths::serialize(&folders)).map_err(|e| tr!("Erro ao gravar {}: {e}", "Error writing {}: {e}", config.paths.display()))?;
    read_state(false)
}

/// Encerra o programa.
#[tauri::command]
fn quit(app: AppHandle) {
    app.exit(0);
}

/// Grupos de arquivos duplicados (caminhos) na pasta, por nome ou conteúdo; `None` = cancelado.
/// Roda fora da thread principal; por conteúdo emite `duplicates-progress` com (feitos, total).
#[tauri::command]
async fn find_duplicates(path: String, mode: duplicates::Mode, app: AppHandle) -> Result<Option<Vec<Vec<String>>>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (_, index) = load()?;
        let files = scan::list_files(Path::new(&path), &Classifier::new(&index)).map_err(|e| format!("{path}: {e}"))?;
        Ok(match mode {
            duplicates::Mode::Name => Some(duplicates::by_name(&files)),
            duplicates::Mode::Content => {
                let last = std::cell::Cell::new(std::time::Instant::now());
                app.state::<duplicates::Search>().by_content(&files, |done, total| {
                    // no máximo ~10 avisos por segundo, mais o último
                    if done == total || last.get().elapsed().as_millis() >= 100 {
                        last.set(std::time::Instant::now());
                        let _ = app.emit("duplicates-progress", (done, total));
                    }
                })
            }
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Cancela a busca de duplicados por conteúdo em andamento.
#[tauri::command]
fn cancel_duplicates(search: tauri::State<'_, duplicates::Search>) {
    search.cancel();
}

/// Cria uma subpasta (`relative` pode ter subpastas) dentro da pasta listada; devolve o caminho.
#[tauri::command]
fn create_folder(root: String, relative: String) -> Result<String, String> {
    fileops::create_folder(Path::new(&root), &relative).map(|p| p.display().to_string())
}

/// Arquivos classificados e subpastas de uma pasta.
#[tauri::command]
fn list_folder(path: String) -> Result<scan::Listing, String> {
    let (_, index) = load()?;
    scan::list(Path::new(&path), &Classifier::new(&index)).map_err(|e| match e.kind() {
        io::ErrorKind::NotFound => tr!("Não foi possível encontrar o caminho: {path}", "Could not find the path: {path}"),
        _ => format!("{path}: {e}"),
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if let Some(root) = dirs::config_dir()
        && let Err(error) = config::migrate_old_name(&root)
    {
        eprintln!("{error}"); // segue com a configuração nova (vazia); a antiga fica em ~/.config/manabox
    }
    launch::prefer_host_data();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(watch::FolderWatcher::default())
        .manage(watch::ConfigWatcher::default())
        .manage(duplicates::Search::default())
        .manage(FirstRun::default())
        .setup(|app| {
            // primeira execução (sem índice nem pastas): a interface pergunta o idioma e cria os exemplos
            let located = Config::locate(&config::default_dir());
            if let Some(language) = located.language {
                i18n::set(language);
            }
            if !located.index.exists() && !located.paths.exists() {
                app.state::<FirstRun>().0.store(true, Ordering::Relaxed);
            }
            if let Err(error) = watch_config(app.handle()) {
                eprintln!("{error}"); // sem observação o programa funciona; só não recarrega sozinho
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![load_state, save_folders, save_index, move_config_file, list_folder, rename_file, trash_files, move_file, copy_file, watch_folder, clipboard_set, clipboard_get, clipboard_clear, split_files, drag_files, find_duplicates, cancel_duplicates, create_examples, set_language, create_folder, open_folder, open_file, quit])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
