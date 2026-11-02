use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::fileops::move_path;
use crate::i18n::Lang;
use crate::{index, paths};

/// Localização de cajabox.index.md e cajabox.paths.md e idioma, guardados em `<pasta de config>/config`
/// como `chave=valor`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Config {
    pub index: PathBuf,
    pub paths: PathBuf,
    /// `None` = ainda não escolhido (a interface pergunta).
    pub language: Option<Lang>,
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
            ConfigFile::Index => "cajabox.index.md",
            ConfigFile::Paths => "cajabox.paths.md",
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum MoveOutcome {
    Moved,
    /// Já existe um arquivo com o mesmo nome no destino; nada foi alterado.
    Exists(PathBuf),
}

/// Programa renomeado de manabox para cajabox: move `<root>/manabox` para `<root>/cajabox` e renomeia
/// manabox.index.md/manabox.paths.md para cajabox.*.md onde estiverem (inclusive fora da pasta de
/// configuração), atualizando o arquivo `config`. Nada é apagado. Devolve se migrou.
pub fn migrate_old_name(root: &Path) -> io::Result<bool> {
    let (old, new) = (root.join("manabox"), root.join("cajabox"));
    if new.exists() || !old.is_dir() {
        return Ok(false);
    }
    fs::rename(&old, &new)?;
    let text = fs::read_to_string(new.join("config")).unwrap_or_default();
    let mut config = Config::locate(&new); // mantém o idioma
    for file in [ConfigFile::Index, ConfigFile::Paths] {
        let key = match file {
            ConfigFile::Index => "index",
            ConfigFile::Paths => "paths",
        };
        let old_name = format!("manabox.{key}.md");
        let saved = text.lines().find_map(|line| line.strip_prefix(&format!("{key}=")).map(PathBuf::from));
        let current = match saved {
            Some(path) => path.strip_prefix(&old).map(|rest| new.join(rest)).unwrap_or(path),
            None => new.join(&old_name),
        };
        let renamed = current.with_file_name(file.file_name());
        let path = if current.file_name() == Some(old_name.as_ref()) && current.exists() && !renamed.exists() {
            fs::rename(&current, &renamed)?;
            renamed
        } else {
            current
        };
        match file {
            ConfigFile::Index => config.index = path,
            ConfigFile::Paths => config.paths = path,
        }
    }
    config.save(&new)?;
    Ok(true)
}

/// `~/.config/cajabox` no Linux.
pub fn default_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("cajabox")
}

impl Config {
    /// Lê a configuração de `dir` e cria cajabox.index.md e cajabox.paths.md a partir dos modelos se não existirem.
    pub fn load(dir: &Path) -> io::Result<Config> {
        let config = Config::locate(dir);
        create_if_missing(&config.index, index::template())?;
        create_if_missing(&config.paths, paths::template())?;
        Ok(config)
    }

    /// Onde estão cajabox.index.md e cajabox.paths.md (sem criar nada).
    pub fn locate(dir: &Path) -> Config {
        let mut config = Config {
            index: dir.join(ConfigFile::Index.file_name()),
            paths: dir.join(ConfigFile::Paths.file_name()),
            language: None,
        };
        if let Ok(text) = fs::read_to_string(dir.join("config")) {
            for line in text.lines() {
                match line.split_once('=').map(|(k, v)| (k.trim(), v.trim())) {
                    Some(("index", v)) if !v.is_empty() => config.index = v.into(),
                    Some(("paths", v)) if !v.is_empty() => config.paths = v.into(),
                    Some(("language", v)) => config.language = Lang::parse(v),
                    _ => {}
                }
            }
        }
        config
    }

    pub fn save(&self, dir: &Path) -> io::Result<()> {
        fs::create_dir_all(dir)?;
        fs::write(
            dir.join("config"),
            format!(
                "index={}\npaths={}\n{}",
                self.index.display(),
                self.paths.display(),
                self.language.map(|l| format!("language={}\n", l.code())).unwrap_or_default()
            ),
        )
    }

    fn slot(&mut self, file: ConfigFile) -> &mut PathBuf {
        match file {
            ConfigFile::Index => &mut self.index,
            ConfigFile::Paths => &mut self.paths,
        }
    }

    /// Move cajabox.index.md ou cajabox.paths.md para `dest_dir` com o nome fixo (renomeia nomes antigos)
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
    fn migrates_old_name_in_default_location() {
        let root = tempfile::tempdir().unwrap();
        let old = root.path().join("manabox");
        fs::create_dir(&old).unwrap();
        fs::write(old.join("manabox.index.md"), "# A\n").unwrap();
        fs::write(old.join("manabox.paths.md"), "x|y|/z\n").unwrap();
        fs::write(old.join("config"), "language=en\n").unwrap();

        assert!(migrate_old_name(root.path()).unwrap());
        let new = root.path().join("cajabox");
        assert!(!old.exists());
        assert_eq!(fs::read_to_string(new.join("cajabox.index.md")).unwrap(), "# A\n");
        assert_eq!(fs::read_to_string(new.join("cajabox.paths.md")).unwrap(), "x|y|/z\n");
        let config = Config::locate(&new);
        assert_eq!((config.index, config.paths, config.language), (new.join("cajabox.index.md"), new.join("cajabox.paths.md"), Some(Lang::En)));
        assert!(!migrate_old_name(root.path()).unwrap(), "segunda vez não faz nada");
    }

    #[test]
    fn migrates_files_moved_elsewhere() {
        let root = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        let old = root.path().join("manabox");
        fs::create_dir(&old).unwrap();
        fs::write(old.join("manabox.paths.md"), "p").unwrap();
        fs::write(elsewhere.path().join("manabox.index.md"), "i").unwrap();
        fs::write(old.join("config"), format!("index={}\npaths={}\n", elsewhere.path().join("manabox.index.md").display(), old.join("manabox.paths.md").display())).unwrap();

        migrate_old_name(root.path()).unwrap();
        let new = root.path().join("cajabox");
        let config = Config::locate(&new);
        assert_eq!(config.index, elsewhere.path().join("cajabox.index.md"));
        assert_eq!(config.paths, new.join("cajabox.paths.md"));
        assert_eq!(fs::read_to_string(&config.index).unwrap(), "i");
        assert_eq!(fs::read_to_string(&config.paths).unwrap(), "p");
    }

    #[test]
    fn language_is_saved_and_read() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = Config::locate(dir.path());
        assert_eq!(config.language, None);
        config.language = Some(Lang::En);
        config.save(dir.path()).unwrap();
        assert_eq!(Config::locate(dir.path()).language, Some(Lang::En));
    }

    #[test]
    fn creates_files_from_templates_in_default_location() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::load(dir.path()).unwrap();
        assert_eq!(config.index, dir.path().join("cajabox.index.md"));
        assert_eq!(fs::read_to_string(&config.paths).unwrap(), paths::TEMPLATE);
    }

    #[test]
    fn keeps_existing_files_and_saved_locations() {
        let dir = tempfile::tempdir().unwrap();
        let other = dir.path().join("outro/cajabox.index.md");
        fs::create_dir_all(other.parent().unwrap()).unwrap();
        fs::write(&other, "# Meu").unwrap();

        let config = Config { index: other.clone(), paths: dir.path().join("cajabox.paths.md"), language: None };
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
        assert_eq!(config.index, dest.join("cajabox.index.md"));
        assert!(!dir.path().join("cajabox.index.md").exists());
        assert_eq!(fs::read_to_string(dest.join("cajabox.index.md")).unwrap(), "# Meu");
        assert_eq!(Config::load(dir.path()).unwrap(), config);
    }

    #[test]
    fn move_file_detects_conflict_and_overwrites_on_request() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("dest");
        fs::create_dir(&dest).unwrap();
        fs::write(dest.join("cajabox.paths.md"), "antigo").unwrap();
        let mut config = Config::load(dir.path()).unwrap();
        fs::write(&config.paths, "novo").unwrap();

        let outcome = config.move_file(ConfigFile::Paths, &dest, false, dir.path()).unwrap();
        assert_eq!(outcome, MoveOutcome::Exists(dest.join("cajabox.paths.md")));
        assert_eq!(config.paths, dir.path().join("cajabox.paths.md"));
        assert_eq!(fs::read_to_string(dest.join("cajabox.paths.md")).unwrap(), "antigo");

        config.move_file(ConfigFile::Paths, &dest, true, dir.path()).unwrap();
        assert_eq!(config.paths, dest.join("cajabox.paths.md"));
        assert_eq!(fs::read_to_string(dest.join("cajabox.paths.md")).unwrap(), "novo");
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
        let mut config = Config { index: old.clone(), paths: dir.path().join("cajabox.paths.md"), language: None };

        config.move_file(ConfigFile::Index, old.parent().unwrap(), false, dir.path()).unwrap();
        assert_eq!(config.index, dir.path().join("outro/cajabox.index.md"));
        assert!(!old.exists());
        assert_eq!(fs::read_to_string(&config.index).unwrap(), "# Antigo");
    }
}
