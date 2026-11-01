use std::path::{Component, Path, PathBuf};
use std::sync::{mpsc, Mutex};
use std::thread;
use std::time::Duration;

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

/// Tempo sem novos eventos para considerar terminada uma rajada de mudanças.
const DEBOUNCE: Duration = Duration::from_millis(300);

/// Observa uma pasta por vez (a aberta na interface), recursivamente.
#[derive(Default)]
pub struct FolderWatcher(Mutex<Option<RecommendedWatcher>>);

impl FolderWatcher {
    /// Troca a pasta observada. `on_change` é chamado uma vez por rajada de mudanças (~300 ms),
    /// ignorando leituras (a própria listagem gera eventos de acesso) e mudanças só em
    /// arquivos/pastas ocultos. Pasta inexistente: apenas para de observar.
    pub fn watch(&self, root: &Path, on_change: impl Fn() + Send + 'static) -> Result<(), String> {
        let mut current = self.0.lock().unwrap();
        *current = None; // para a observação anterior (e encerra a thread dela)
        if !root.is_dir() {
            return Ok(());
        }
        let base = root.to_path_buf();
        let (tx, rx) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(move |result: notify::Result<Event>| {
            if let Ok(event) = result
                && !matches!(event.kind, EventKind::Access(_))
                && (event.paths.is_empty() || event.paths.iter().any(|p| !is_hidden(&base, p)))
            {
                let _ = tx.send(());
            }
        })
        .map_err(|e| e.to_string())?;
        watcher
            .watch(root, RecursiveMode::Recursive)
            .map_err(|e| format!("Não foi possível observar {}: {e}", root.display()))?;
        debounce(rx, on_change);
        *current = Some(watcher);
        Ok(())
    }
}

/// Observa os arquivos de configuração (manabox.index.md e manabox.paths.md). Observa as pastas
/// deles, sem subpastas, e filtra pelo caminho: editores salvam gravando um arquivo novo e trocando
/// pelo antigo, o que uma observação só do arquivo perderia.
#[derive(Default)]
pub struct ConfigWatcher(Mutex<Option<RecommendedWatcher>>);

impl ConfigWatcher {
    /// Troca os arquivos observados. `on_change` é chamado uma vez por rajada de mudanças (~300 ms).
    pub fn watch(&self, files: &[PathBuf], on_change: impl Fn() + Send + 'static) -> Result<(), String> {
        let mut current = self.0.lock().unwrap();
        *current = None;
        let watched = files.to_vec();
        let (tx, rx) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(move |result: notify::Result<Event>| {
            if let Ok(event) = result
                && !matches!(event.kind, EventKind::Access(_))
                && event.paths.iter().any(|p| watched.contains(p))
            {
                let _ = tx.send(());
            }
        })
        .map_err(|e| e.to_string())?;
        let mut dirs: Vec<&Path> = files.iter().filter_map(|f| f.parent()).collect();
        dirs.sort();
        dirs.dedup();
        for dir in dirs {
            watcher
                .watch(dir, RecursiveMode::NonRecursive)
                .map_err(|e| format!("Não foi possível observar {}: {e}", dir.display()))?;
        }
        debounce(rx, on_change);
        *current = Some(watcher);
        Ok(())
    }
}

/// Thread que chama `on_change` uma vez por rajada (pausa de `DEBOUNCE` sem eventos); termina
/// quando o observador é descartado.
fn debounce(rx: mpsc::Receiver<()>, on_change: impl Fn() + Send + 'static) {
    thread::spawn(move || {
        while rx.recv().is_ok() {
            loop {
                match rx.recv_timeout(DEBOUNCE) {
                    Ok(()) => continue,
                    Err(mpsc::RecvTimeoutError::Timeout) => break,
                    Err(mpsc::RecvTimeoutError::Disconnected) => return,
                }
            }
            on_change();
        }
    });
}

/// Algum componente do caminho (relativo à pasta observada) começa com `.`.
fn is_hidden(root: &Path, path: &Path) -> bool {
    let relative: PathBuf = path.strip_prefix(root).unwrap_or(path).to_path_buf();
    relative
        .components()
        .any(|c| matches!(c, Component::Normal(name) if name.to_string_lossy().starts_with('.')))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn hidden_paths_are_detected_relative_to_root() {
        let root = Path::new("/home/u/.media/pasta");
        assert!(!is_hidden(root, Path::new("/home/u/.media/pasta/a.mp4")));
        assert!(is_hidden(root, Path::new("/home/u/.media/pasta/.cache/x")));
        assert!(is_hidden(root, Path::new("/home/u/.media/pasta/sub/.oculto")));
    }

    #[test]
    fn reading_does_not_notify() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();
        fs::write(dir.path().join("sub/a.mp4"), "x").unwrap();
        let watcher = FolderWatcher::default();
        let (tx, rx) = mpsc::channel();
        watcher.watch(dir.path(), move || tx.send(()).unwrap()).unwrap();
        thread::sleep(Duration::from_millis(100));

        for entry in fs::read_dir(dir.path().join("sub")).unwrap() {
            entry.unwrap().metadata().unwrap();
        }
        fs::read(dir.path().join("sub/a.mp4")).unwrap();
        assert!(rx.recv_timeout(Duration::from_millis(1000)).is_err());
    }

    #[test]
    fn notifies_visible_changes_only() {
        let dir = tempfile::tempdir().unwrap();
        let watcher = FolderWatcher::default();
        let (tx, rx) = mpsc::channel();
        watcher.watch(dir.path(), move || tx.send(()).unwrap()).unwrap();
        thread::sleep(Duration::from_millis(100));

        fs::write(dir.path().join(".oculto"), "").unwrap();
        assert!(rx.recv_timeout(Duration::from_millis(1000)).is_err());

        fs::create_dir(dir.path().join("sub")).unwrap();
        fs::write(dir.path().join("sub/a.mp4"), "").unwrap();
        assert!(rx.recv_timeout(Duration::from_secs(3)).is_ok());
    }

    #[test]
    fn config_watcher_notifies_only_watched_files_including_replace() {
        let dir = tempfile::tempdir().unwrap();
        let index = dir.path().join("manabox.index.md");
        fs::write(&index, "# A\n").unwrap();
        let watcher = ConfigWatcher::default();
        let (tx, rx) = mpsc::channel();
        watcher.watch(&[index.clone(), dir.path().join("manabox.paths.md")], move || tx.send(()).unwrap()).unwrap();
        thread::sleep(Duration::from_millis(100));

        fs::write(dir.path().join("outro.txt"), "x").unwrap();
        assert!(rx.recv_timeout(Duration::from_millis(1000)).is_err());

        // como editores salvam: grava arquivo temporário e troca pelo original
        let temp = dir.path().join(".manabox.index.md.tmp");
        fs::write(&temp, "# B\n").unwrap();
        fs::rename(&temp, &index).unwrap();
        assert!(rx.recv_timeout(Duration::from_secs(3)).is_ok());
    }
}
