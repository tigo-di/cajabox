use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Resultado de mover ou copiar.
#[derive(Debug, PartialEq)]
pub enum Outcome {
    Done(PathBuf),
    /// Já existe um arquivo com o mesmo nome no destino; nada foi alterado.
    Exists(PathBuf),
}

/// `rename`, ou cópia + remoção quando origem e destino estão em discos diferentes.
pub fn move_path(from: &Path, to: &Path) -> io::Result<()> {
    match fs::rename(from, to) {
        Err(e) if e.kind() == io::ErrorKind::CrossesDevices => {
            copy_file(from, to)?;
            fs::remove_file(from)
        }
        result => result,
    }
}

/// Copia mantendo a data de modificação (usada como data do arquivo quando não há data no nome).
fn copy_file(from: &Path, to: &Path) -> io::Result<()> {
    fs::copy(from, to)?;
    let modified = fs::metadata(from)?.modified()?;
    fs::File::options().write(true).open(to)?.set_modified(modified)
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

fn file_name(path: &Path) -> io::Result<&std::ffi::OsStr> {
    path.file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "caminho sem nome de arquivo"))
}

/// Move um arquivo para `dest_dir`, mantendo o nome. Sem `overwrite`, conflito volta como `Exists`.
pub fn move_to(path: &Path, dest_dir: &Path, overwrite: bool) -> io::Result<Outcome> {
    let to = dest_dir.join(file_name(path)?);
    if to.exists() && fs::canonicalize(&to)? == fs::canonicalize(path)? {
        return Ok(Outcome::Done(to));
    }
    if to.exists() && !overwrite {
        return Ok(Outcome::Exists(to));
    }
    move_path(path, &to)?;
    Ok(Outcome::Done(to))
}

/// Copia um arquivo para `dest_dir`, mantendo o nome. Sem `overwrite`, conflito volta como `Exists`.
/// Copia primeiro para um arquivo oculto temporário no destino e depois renomeia: o destino nunca
/// fica pela metade. Copiar um arquivo sobre ele mesmo é recusado.
pub fn copy_to(path: &Path, dest_dir: &Path, overwrite: bool) -> io::Result<Outcome> {
    let name = file_name(path)?;
    let to = dest_dir.join(name);
    if to.exists() && fs::canonicalize(&to)? == fs::canonicalize(path)? {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "origem e destino são o mesmo arquivo"));
    }
    if to.exists() && !overwrite {
        return Ok(Outcome::Exists(to));
    }
    let mut temp_name = std::ffi::OsString::from(".");
    temp_name.push(name);
    temp_name.push(".manabox-tmp");
    let temp = dest_dir.join(temp_name);
    if let Err(e) = copy_file(path, &temp).and_then(|_| fs::rename(&temp, &to)) {
        let _ = fs::remove_file(&temp);
        return Err(e);
    }
    Ok(Outcome::Done(to))
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

        assert_eq!(move_to(&b, &dest, false).unwrap(), Outcome::Done(dest.join("b.txt")));
        assert!(!b.exists());

        assert_eq!(move_to(&a, &dest, false).unwrap(), Outcome::Exists(dest.join("a.txt")));
        assert_eq!(fs::read_to_string(dest.join("a.txt")).unwrap(), "antigo");
        assert!(a.exists());

        assert_eq!(move_to(&a, &dest, true).unwrap(), Outcome::Done(dest.join("a.txt")));
        assert_eq!(fs::read_to_string(dest.join("a.txt")).unwrap(), "novo");
        assert!(!a.exists());
    }

    #[test]
    fn copy_keeps_original_and_handles_conflicts() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("dest");
        fs::create_dir(&dest).unwrap();
        let a = dir.path().join("a b ção.txt");
        fs::write(&a, "novo").unwrap();

        assert_eq!(copy_to(&a, &dest, false).unwrap(), Outcome::Done(dest.join("a b ção.txt")));
        assert_eq!(fs::read_to_string(dest.join("a b ção.txt")).unwrap(), "novo");
        assert!(a.exists());

        fs::write(dest.join("a b ção.txt"), "antigo").unwrap();
        assert_eq!(copy_to(&a, &dest, false).unwrap(), Outcome::Exists(dest.join("a b ção.txt")));
        assert_eq!(fs::read_to_string(dest.join("a b ção.txt")).unwrap(), "antigo");

        assert_eq!(copy_to(&a, &dest, true).unwrap(), Outcome::Done(dest.join("a b ção.txt")));
        assert_eq!(fs::read_to_string(dest.join("a b ção.txt")).unwrap(), "novo");
        assert_eq!(fs::read_dir(&dest).unwrap().count(), 1, "temporário removido");
    }

    #[test]
    fn copy_keeps_modification_date() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("dest");
        fs::create_dir(&dest).unwrap();
        let a = dir.path().join("antigo.png");
        fs::write(&a, "x").unwrap();
        let old = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
        fs::File::options().write(true).open(&a).unwrap().set_modified(old).unwrap();
        copy_to(&a, &dest, false).unwrap();
        assert_eq!(fs::metadata(dest.join("antigo.png")).unwrap().modified().unwrap(), old);
    }

    #[test]
    fn copy_onto_itself_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.txt");
        fs::write(&a, "x").unwrap();
        assert!(copy_to(&a, dir.path(), true).is_err());
        assert_eq!(fs::read_to_string(&a).unwrap(), "x");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn move_to_same_folder_does_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.txt");
        fs::write(&a, "x").unwrap();
        assert_eq!(move_to(&a, dir.path(), false).unwrap(), Outcome::Done(a.clone()));
        assert!(a.exists());
    }
}
