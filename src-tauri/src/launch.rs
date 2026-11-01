//! Abre arquivos e pastas com o programa padrão do sistema via GIO, usando o contexto de
//! lançamento do GDK: no Wayland ele entrega o token de ativação e o programa aberto vem para a
//! frente (sem isso, um Editor de Texto já aberto recebe o arquivo numa aba e fica atrás).
//! Precisa rodar na thread principal.

use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use gtk::gdk;
use gtk::gio::{self, prelude::*};

/// Variáveis próprias do AppImage ou impostas por ele sem apontar para dentro dele.
const APPIMAGE_ONLY: [&str; 5] = ["APPDIR", "APPIMAGE", "ARGV0", "OWD", "GTK_THEME"];

/// Abre `path` com o programa padrão (pasta: gerenciador de arquivos).
pub fn open(path: &Path) -> Result<(), String> {
    let context = gdk::Display::default()
        .and_then(|display| display.app_launch_context())
        .ok_or("Sem acesso à sessão gráfica")?;
    if let Some(appdir) = std::env::var_os("APPDIR") {
        for (name, value) in clean_env(std::env::vars_os(), Path::new(&appdir)) {
            match value {
                Some(value) => context.setenv(&name, value),
                None => context.unsetenv(&name),
            }
        }
    }
    let uri = gio::File::for_path(path).uri();
    gio::AppInfo::launch_default_for_uri(&uri, Some(&context)).map_err(|e| e.to_string())
}

/// Ajustes para o programa aberto não herdar o ambiente do AppImage (senão carrega as bibliotecas
/// dele e quebra, ex.: `LD_LIBRARY_PATH`): variáveis que apontam para dentro de `appdir` perdem
/// essas entradas (`a:b:c`) e somem se não sobrar nada. `None` = remover.
fn clean_env(vars: impl Iterator<Item = (OsString, OsString)>, appdir: &Path) -> Vec<(OsString, Option<OsString>)> {
    let appdir_bytes = appdir.as_os_str().as_bytes();
    let mut changes = Vec::new();
    for (name, value) in vars {
        if APPIMAGE_ONLY.iter().any(|v| name == *v) {
            changes.push((name, None));
        } else if value.as_bytes().windows(appdir_bytes.len()).any(|w| w == appdir_bytes) {
            let kept = without_dir(&value, appdir);
            changes.push((name, (!kept.is_empty()).then_some(kept)));
        }
    }
    changes
}

/// Lista de caminhos (`a:b:c`) sem os que ficam dentro de `dir` e sem entradas vazias.
fn without_dir(dirs: &OsStr, dir: &Path) -> OsString {
    let kept = std::env::split_paths(dirs).filter(|d| !d.as_os_str().is_empty() && !d.starts_with(dir));
    std::env::join_paths(kept).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &[(&str, &str)]) -> Vec<(OsString, OsString)> {
        pairs.iter().map(|(k, v)| (k.into(), v.into())).collect()
    }

    #[test]
    fn removes_appimage_entries_and_variables() {
        let appdir = Path::new("/tmp/.mount_manaXYZ");
        let vars = env(&[
            ("LD_LIBRARY_PATH", "/tmp/.mount_manaXYZ/usr/lib/:/tmp/.mount_manaXYZ/lib/:"),
            ("PATH", "/tmp/.mount_manaXYZ/usr/bin/:/usr/local/bin:/usr/bin"),
            ("XDG_DATA_DIRS", "/tmp/.mount_manaXYZ/usr/share:/usr/share"),
            ("PYTHONHOME", "/tmp/.mount_manaXYZ/usr/"),
            ("GTK_THEME", "Adwaita:light"),
            ("APPDIR", "/tmp/.mount_manaXYZ"),
            ("HOME", "/home/u"),
        ]);
        let changes = clean_env(vars.into_iter(), appdir);
        let get = |name: &str| changes.iter().find(|(n, _)| n == name).map(|(_, v)| v.clone());
        assert_eq!(get("LD_LIBRARY_PATH"), Some(None));
        assert_eq!(get("PATH"), Some(Some("/usr/local/bin:/usr/bin".into())));
        assert_eq!(get("XDG_DATA_DIRS"), Some(Some("/usr/share".into())));
        assert_eq!(get("PYTHONHOME"), Some(None));
        assert_eq!(get("GTK_THEME"), Some(None));
        assert_eq!(get("APPDIR"), Some(None));
        assert_eq!(get("HOME"), None, "variáveis sem relação com o AppImage não mudam");
    }
}
