use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::fileops::move_path;
use crate::index::TEMPLATE as INDEX_TEMPLATE;
use crate::paths::TEMPLATE as PATHS_TEMPLATE;

/// Localização de manabox.index.md e manabox.paths.md, guardada em `<pasta de config>/config` como `chave=caminho`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Config {
    pub index: PathBuf,
    pub paths: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigFile {
    Index,
    Paths,
}

impl ConfigFile {
    /// Nome fixo, com prefixo do programa para ser reconhecível em qualquer pasta.
    pub fn file_name(self) -> &'static str {
        match self {
            ConfigFile::Index => "manabox.index.md",
            ConfigFile::Paths => "manabox.paths.md",
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum MoveOutcome {
    Moved,
    /// Já existe um arquivo com o mesmo nome no destino; nada foi alterado.
    Exists(PathBuf),
}

/// `~/.config/manabox` no Linux.
pub fn default_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("manabox")
}

impl Config {
    /// Lê a configuração de `dir` e cria manabox.index.md e manabox.paths.md a partir dos modelos se não existirem.
    pub fn load(dir: &Path) -> io::Result<Config> {
        let mut config = Config {
            index: dir.join(ConfigFile::Index.file_name()),
            paths: dir.join(ConfigFile::Paths.file_name()),
        };
        if let Ok(text) = fs::read_to_string(dir.join("config")) {
            for line in text.lines() {
                match line.split_once('=').map(|(k, v)| (k.trim(), v.trim())) {
                    Some(("index", v)) if !v.is_empty() => config.index = v.into(),
                    Some(("paths", v)) if !v.is_empty() => config.paths = v.into(),
                    _ => {}
                }
            }
        }
        create_if_missing(&config.index, INDEX_TEMPLATE)?;
        create_if_missing(&config.paths, PATHS_TEMPLATE)?;
        Ok(config)
    }

    pub fn save(&self, dir: &Path) -> io::Result<()> {
        fs::create_dir_all(dir)?;
        fs::write(
            dir.join("config"),
            format!("index={}\npaths={}\n", self.index.display(), self.paths.display()),
        )
    }

    fn slot(&mut self, file: ConfigFile) -> &mut PathBuf {
        match file {
            ConfigFile::Index => &mut self.index,
            ConfigFile::Paths => &mut self.paths,
        }
    }

    /// Move manabox.index.md ou manabox.paths.md para `dest_dir` com o nome fixo (renomeia nomes antigos)
    /// e grava a config em `config_dir`.
    /// Sem `overwrite`, um arquivo de mesmo nome no destino não é tocado e o retorno é `Exists`.
    /// Se gravar a config falhar, o arquivo volta para o lugar original.
    pub fn move_file(&mut self, file: ConfigFile, dest_dir: &Path, overwrite: bool, config_dir: &Path) -> io::Result<MoveOutcome> {
        let from = self.slot(file).clone();
        let to = dest_dir.join(file.file_name());
        if to.exists() && fs::canonicalize(&to)? == fs::canonicalize(&from)? {
            return Ok(MoveOutcome::Moved);
        }
        if to.exists() && !overwrite {
            return Ok(MoveOutcome::Exists(to));
        }
        move_path(&from, &to)?;
        *self.slot(file) = to.clone();
        if let Err(e) = self.save(config_dir) {
            let _ = move_path(&to, &from);
            *self.slot(file) = from;
            return Err(e);
        }
        Ok(MoveOutcome::Moved)
    }
}

fn create_if_missing(path: &Path, template: &str) -> io::Result<()> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, template)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_files_from_templates_in_default_location() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::load(dir.path()).unwrap();
        assert_eq!(config.index, dir.path().join("manabox.index.md"));
        assert_eq!(fs::read_to_string(&config.paths).unwrap(), PATHS_TEMPLATE);
    }

    #[test]
    fn keeps_existing_files_and_saved_locations() {
        let dir = tempfile::tempdir().unwrap();
        let other = dir.path().join("outro/manabox.index.md");
        fs::create_dir_all(other.parent().unwrap()).unwrap();
        fs::write(&other, "# Meu").unwrap();

        let config = Config { index: other.clone(), paths: dir.path().join("manabox.paths.md") };
        config.save(dir.path()).unwrap();

        let loaded = Config::load(dir.path()).unwrap();
        assert_eq!(loaded, config);
        assert_eq!(fs::read_to_string(other).unwrap(), "# Meu");
    }

    #[test]
    fn move_file_updates_config_and_keeps_content() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("dest");
        fs::create_dir(&dest).unwrap();
        let mut config = Config::load(dir.path()).unwrap();
        fs::write(&config.index, "# Meu").unwrap();

        let outcome = config.move_file(ConfigFile::Index, &dest, false, dir.path()).unwrap();
        assert_eq!(outcome, MoveOutcome::Moved);
        assert_eq!(config.index, dest.join("manabox.index.md"));
        assert!(!dir.path().join("manabox.index.md").exists());
        assert_eq!(fs::read_to_string(dest.join("manabox.index.md")).unwrap(), "# Meu");
        assert_eq!(Config::load(dir.path()).unwrap(), config);
    }

    #[test]
    fn move_file_detects_conflict_and_overwrites_on_request() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("dest");
        fs::create_dir(&dest).unwrap();
        fs::write(dest.join("manabox.paths.md"), "antigo").unwrap();
        let mut config = Config::load(dir.path()).unwrap();
        fs::write(&config.paths, "novo").unwrap();

        let outcome = config.move_file(ConfigFile::Paths, &dest, false, dir.path()).unwrap();
        assert_eq!(outcome, MoveOutcome::Exists(dest.join("manabox.paths.md")));
        assert_eq!(config.paths, dir.path().join("manabox.paths.md"));
        assert_eq!(fs::read_to_string(dest.join("manabox.paths.md")).unwrap(), "antigo");

        config.move_file(ConfigFile::Paths, &dest, true, dir.path()).unwrap();
        assert_eq!(config.paths, dest.join("manabox.paths.md"));
        assert_eq!(fs::read_to_string(dest.join("manabox.paths.md")).unwrap(), "novo");
    }

    #[test]
    fn move_to_same_folder_does_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = Config::load(dir.path()).unwrap();
        let outcome = config.move_file(ConfigFile::Index, dir.path(), false, dir.path()).unwrap();
        assert_eq!(outcome, MoveOutcome::Moved);
        assert!(config.index.exists());
    }

    #[test]
    fn move_renames_file_with_old_name() {
        let dir = tempfile::tempdir().unwrap();
        let old = dir.path().join("outro/index.md");
        fs::create_dir_all(old.parent().unwrap()).unwrap();
        fs::write(&old, "# Antigo").unwrap();
        let mut config = Config { index: old.clone(), paths: dir.path().join("manabox.paths.md") };

        config.move_file(ConfigFile::Index, old.parent().unwrap(), false, dir.path()).unwrap();
        assert_eq!(config.index, dir.path().join("outro/manabox.index.md"));
        assert!(!old.exists());
        assert_eq!(fs::read_to_string(&config.index).unwrap(), "# Antigo");
    }
}
