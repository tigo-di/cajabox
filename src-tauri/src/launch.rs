//! Abre arquivos e pastas com o programa padrão do sistema via GIO, usando o contexto de
//! lançamento do GDK: no Wayland ele entrega o token de ativação e o programa aberto vem para a
//! frente (sem isso, um Editor de Texto já aberto recebe o arquivo numa aba e fica atrás).
//! Precisa rodar na thread principal.

use std::ffi::{OsStr, OsString};
use std::path::Path;

use gtk::gdk;
use gtk::gio::{self, prelude::*};

/// Variáveis que o AppImage define para as próprias bibliotecas (runtime e linuxdeploy-plugin-gtk);
/// programas abertos pelo manabox não podem herdá-las.
const APPIMAGE_VARS: [&str; 13] = [
    "APPDIR",
    "APPIMAGE",
    "ARGV0",
    "OWD",
    "GTK_DATA_PREFIX",
    "GTK_THEME",
    "GSETTINGS_SCHEMA_DIR",
    "GI_TYPELIB_PATH",
    "GIO_MODULE_DIR",
    "GTK_EXE_PREFIX",
    "GTK_PATH",
    "GTK_IM_MODULE_FILE",
    "GDK_PIXBUF_MODULE_FILE",
];

/// Abre `path` com o programa padrão (pasta: gerenciador de arquivos).
pub fn open(path: &Path) -> Result<(), String> {
    let context = gdk::Display::default()
        .and_then(|display| display.app_launch_context())
        .ok_or("Sem acesso à sessão gráfica")?;
    if let Some(appdir) = std::env::var_os("APPDIR") {
        for var in APPIMAGE_VARS {
            context.unsetenv(var);
        }
        if let Some(dirs) = std::env::var_os("XDG_DATA_DIRS") {
            context.setenv("XDG_DATA_DIRS", without_dir(&dirs, Path::new(&appdir)));
        }
    }
    let uri = gio::File::for_path(path).uri();
    gio::AppInfo::launch_default_for_uri(&uri, Some(&context)).map_err(|e| e.to_string())
}

/// Lista de pastas (`a:b:c`) sem as que ficam dentro de `dir`.
fn without_dir(dirs: &OsStr, dir: &Path) -> OsString {
    std::env::join_paths(std::env::split_paths(dirs).filter(|d| !d.starts_with(dir))).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_appimage_entries_from_path_list() {
        let dirs = OsStr::new("/tmp/.mount_manaXYZ/usr/share:/usr/share:/usr/local/share");
        assert_eq!(without_dir(dirs, Path::new("/tmp/.mount_manaXYZ")), "/usr/share:/usr/local/share");
    }
}
