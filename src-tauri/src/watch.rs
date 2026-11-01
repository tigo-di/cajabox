use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use notify_debouncer_mini::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};

/// Observa uma pasta por vez (a aberta na interface), recursivamente.
#[derive(Default)]
pub struct FolderWatcher(Mutex<Option<Debouncer<RecommendedWatcher>>>);

impl FolderWatcher {
    /// Troca a pasta observada. `on_change` é chamado uma vez por rajada de mudanças (~300 ms),
    /// ignorando mudanças só em arquivos/pastas ocultos. Pasta inexistente: apenas para de observar.
    pub fn watch(&self, root: &Path, on_change: impl Fn() + Send + 'static) -> Result<(), String> {
        let mut current = self.0.lock().unwrap();
        *current = None; // para a observação anterior
        if !root.is_dir() {
            return Ok(());
        }
        let base = root.to_path_buf();
        let mut debouncer = new_debouncer(Duration::from_millis(300), move |result: DebounceEventResult| {
            if let Ok(events) = result
                && events.iter().any(|e| !is_hidden(&base, &e.path))
            {
                on_change();
            }
        })
        .map_err(|e| e.to_string())?;
        debouncer
            .watcher()
            .watch(root, RecursiveMode::Recursive)
            .map_err(|e| format!("Não foi possível observar {}: {e}", root.display()))?;
        *current = Some(debouncer);
        Ok(())
    }
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
    use std::sync::mpsc;
    use std::{fs, thread};

    #[test]
    fn hidden_paths_are_detected_relative_to_root() {
        let root = Path::new("/home/u/.media/pasta");
        assert!(!is_hidden(root, Path::new("/home/u/.media/pasta/a.mp4")));
        assert!(is_hidden(root, Path::new("/home/u/.media/pasta/.cache/x")));
        assert!(is_hidden(root, Path::new("/home/u/.media/pasta/sub/.oculto")));
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
}
