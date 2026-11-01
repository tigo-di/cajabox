use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
pub enum MoveOutcome {
    Moved(PathBuf),
    /// Já existe um arquivo com o mesmo nome no destino; nada foi alterado.
    Exists(PathBuf),
}

/// `rename`, ou cópia + remoção quando origem e destino estão em discos diferentes.
pub fn move_path(from: &Path, to: &Path) -> io::Result<()> {
    match fs::rename(from, to) {
        Err(e) if e.kind() == io::ErrorKind::CrossesDevices => {
            fs::copy(from, to)?;
            fs::remove_file(from)
        }
        result => result,
    }
}

/// Renomeia um arquivo na mesma pasta. O nome novo não pode existir, conter `/` nem começar com `.`
/// (arquivo oculto sumiria da listagem).
pub fn rename(path: &Path, new_name: &str) -> Result<PathBuf, String> {
    if new_name.trim().is_empty() || new_name.contains('/') || new_name.starts_with('.') || new_name.trim() != new_name {
        return Err("Nome inválido: não pode ser vazio, conter /, começar com . nem ter espaços nas pontas".into());
    }
    let to = path.with_file_name(new_name);
    if to == path {
        return Ok(to);
    }
    if to.exists() {
        return Err(format!("Já existe um arquivo chamado {new_name}"));
    }
    fs::rename(path, &to).map_err(|e| format!("Erro ao renomear: {e}"))?;
    Ok(to)
}

/// Move um arquivo para `dest_dir`, mantendo o nome. Sem `overwrite`, conflito volta como `Exists`.
pub fn move_to(path: &Path, dest_dir: &Path, overwrite: bool) -> io::Result<MoveOutcome> {
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "caminho sem nome de arquivo"))?;
    let to = dest_dir.join(name);
    if to.exists() && fs::canonicalize(&to)? == fs::canonicalize(path)? {
        return Ok(MoveOutcome::Moved(to));
    }
    if to.exists() && !overwrite {
        return Ok(MoveOutcome::Exists(to));
    }
    move_path(path, &to)?;
    Ok(MoveOutcome::Moved(to))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rename_in_same_folder() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.mp4");
        fs::write(&file, "x").unwrap();
        let to = rename(&file, "news-a.mp4").unwrap();
        assert_eq!(to, dir.path().join("news-a.mp4"));
        assert!(!file.exists());
        assert_eq!(fs::read_to_string(to).unwrap(), "x");
    }

    #[test]
    fn rename_rejects_invalid_names_and_conflicts() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.mp4");
        fs::write(&file, "x").unwrap();
        fs::write(dir.path().join("b.mp4"), "y").unwrap();
        for name in ["", "sub/a.mp4", ".oculto", " a.mp4", "b.mp4"] {
            assert!(rename(&file, name).is_err(), "{name:?}");
        }
        assert_eq!(fs::read_to_string(dir.path().join("b.mp4")).unwrap(), "y");
        assert_eq!(rename(&file, "a.mp4").unwrap(), file);
    }

    #[test]
    fn move_detects_conflict_and_overwrites_on_request() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("dest");
        fs::create_dir(&dest).unwrap();
        let a = dir.path().join("a.txt");
        let b = dir.path().join("b.txt");
        fs::write(&a, "novo").unwrap();
        fs::write(&b, "b").unwrap();
        fs::write(dest.join("a.txt"), "antigo").unwrap();

        assert_eq!(move_to(&b, &dest, false).unwrap(), MoveOutcome::Moved(dest.join("b.txt")));
        assert!(!b.exists());

        assert_eq!(move_to(&a, &dest, false).unwrap(), MoveOutcome::Exists(dest.join("a.txt")));
        assert_eq!(fs::read_to_string(dest.join("a.txt")).unwrap(), "antigo");
        assert!(a.exists());

        assert_eq!(move_to(&a, &dest, true).unwrap(), MoveOutcome::Moved(dest.join("a.txt")));
        assert_eq!(fs::read_to_string(dest.join("a.txt")).unwrap(), "novo");
        assert!(!a.exists());
    }

    #[test]
    fn move_to_same_folder_does_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.txt");
        fs::write(&a, "x").unwrap();
        assert_eq!(move_to(&a, dir.path(), false).unwrap(), MoveOutcome::Moved(a.clone()));
        assert!(a.exists());
    }
}
