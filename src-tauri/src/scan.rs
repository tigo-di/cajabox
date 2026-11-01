use std::fs;
use std::io;
use std::path::Path;

use serde::Serialize;

use crate::classify::{Classification, Classifier};

#[derive(Debug, Clone, Serialize)]
pub struct FileEntry {
    pub name: String,
    /// Caminho relativo à pasta listada, incluindo o nome.
    pub relative_path: String,
    pub path: String,
    #[serde(flatten)]
    pub classification: Classification,
}

/// Lista recursivamente os arquivos de `root`, ignorando ocultos (nome iniciado por `.`).
/// Links simbólicos para pastas não são seguidos. Subpastas sem permissão de leitura são puladas.
pub fn list_files(root: &Path, classifier: &Classifier) -> io::Result<Vec<FileEntry>> {
    if !fs::metadata(root)?.is_dir() {
        return Err(io::Error::new(io::ErrorKind::NotADirectory, "não é uma pasta"));
    }
    let mut files = Vec::new();
    walk(root, root, classifier, &mut files);
    files.sort_by(|a, b| a.relative_path.to_lowercase().cmp(&b.relative_path.to_lowercase()));
    Ok(files)
}

fn walk(root: &Path, dir: &Path, classifier: &Classifier, files: &mut Vec<FileEntry>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else { continue };
        if file_type.is_dir() {
            walk(root, &path, classifier, files);
        } else if file_type.is_file() || fs::metadata(&path).is_ok_and(|m| m.is_file()) {
            files.push(FileEntry {
                classification: classifier.classify(&name),
                relative_path: path.strip_prefix(root).unwrap_or(&path).to_string_lossy().into_owned(),
                path: path.to_string_lossy().into_owned(),
                name,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::parse;

    #[test]
    fn lists_recursively_without_hidden_entries() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("sub/deep")).unwrap();
        fs::create_dir_all(root.join(".hidden")).unwrap();
        for file in ["b-news-2410.mp4", "sub/a.png", "sub/deep/c.txt", ".oculto", ".hidden/x.txt"] {
            fs::write(root.join(file), "").unwrap();
        }

        let classifier = Classifier::new(&parse("# Channels\nnews\n"));
        let files = list_files(root, &classifier).unwrap();
        let names: Vec<_> = files.iter().map(|f| f.relative_path.as_str()).collect();
        assert_eq!(names, ["b-news-2410.mp4", "sub/a.png", "sub/deep/c.txt"]);
        assert_eq!(files[0].classification.terms, vec![vec![0]]);
        assert!(files[0].classification.date.is_some());
    }

    #[test]
    fn missing_folder_is_not_found() {
        let err = list_files(Path::new("/nao/existe/manabox"), &Classifier::new(&Default::default())).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }
}
