//! Arquivos na área de transferência do sistema, nos formatos dos gerenciadores de arquivos:
//! - `x-special/gnome-copied-files`: `copy`/`cut` + URIs (Nautilus, Nemo, Caja, Thunar)
//! - `text/uri-list`: URIs (todos); `application/x-kde-cutselection` = `1` marca recorte (Dolphin)
//! - texto simples: caminhos, um por linha (colar em editores)

use std::ffi::OsString;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Copy,
    Cut,
}

#[derive(Debug, PartialEq)]
pub struct Contents {
    pub mode: Mode,
    pub paths: Vec<PathBuf>,
}

/// `file://` + caminho com bytes fora de `[A-Za-z0-9-._~/]` codificados como `%XX`.
pub fn path_to_uri(path: &Path) -> String {
    let mut uri = String::from("file://");
    for &byte in path.as_os_str().as_bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~/".contains(&byte) {
            uri.push(byte as char);
        } else {
            uri.push_str(&format!("%{byte:02X}"));
        }
    }
    uri
}

/// Caminho local de uma URI `file://` (host vazio ou `localhost`); outras URIs dão `None`.
pub fn uri_to_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.trim().strip_prefix("file://")?;
    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
    if !rest.starts_with('/') {
        return None;
    }
    let bytes = rest.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = std::str::from_utf8(bytes.get(i + 1..i + 3)?).ok()?;
            decoded.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            decoded.push(bytes[i]);
            i += 1;
        }
    }
    Some(PathBuf::from(OsString::from_vec(decoded)))
}

fn uris(paths: &[PathBuf]) -> Vec<String> {
    paths.iter().map(|p| path_to_uri(p)).collect()
}

/// Conteúdo de `x-special/gnome-copied-files`.
pub fn gnome_copied_files(contents: &Contents) -> String {
    let mode = match contents.mode {
        Mode::Copy => "copy",
        Mode::Cut => "cut",
    };
    std::iter::once(mode.to_string()).chain(uris(&contents.paths)).collect::<Vec<_>>().join("\n")
}

/// Conteúdo de `text/uri-list` (linhas terminadas em CRLF, como pede a RFC 2483).
pub fn uri_list(paths: &[PathBuf]) -> String {
    uris(paths).iter().map(|u| format!("{u}\r\n")).collect()
}

/// Caminhos em texto simples, um por linha.
pub fn plain_text(paths: &[PathBuf]) -> String {
    paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join("\n")
}

/// Lê `x-special/gnome-copied-files`; aceita também o texto `x-special/nautilus-clipboard`
/// (mesmo conteúdo precedido dessa linha, usado por versões do Nautilus).
pub fn parse_gnome(text: &str) -> Option<Contents> {
    let text = text.strip_prefix("x-special/nautilus-clipboard\n").unwrap_or(text);
    let mut lines = text.lines();
    let mode = match lines.next()?.trim() {
        "copy" => Mode::Copy,
        "cut" => Mode::Cut,
        _ => return None,
    };
    let paths: Vec<PathBuf> = lines.filter_map(uri_to_path).collect();
    (!paths.is_empty()).then_some(Contents { mode, paths })
}

/// Lê `text/uri-list` (ignora comentários `#` e URIs que não são arquivos locais).
pub fn parse_uri_list(text: &str) -> Vec<PathBuf> {
    text.lines().filter(|l| !l.starts_with('#')).filter_map(uri_to_path).collect()
}

#[cfg(target_os = "linux")]
pub mod system {
    //! Área de transferência via GTK (X11 e Wayland). Precisa rodar na thread principal:
    //! comandos Tauri síncronos (sem `async`) já rodam nela.

    use super::*;
    use gtk::gdk::{Atom, SELECTION_CLIPBOARD};
    use gtk::{Clipboard, TargetEntry, TargetFlags};

    const GNOME: &str = "x-special/gnome-copied-files";
    const URI_LIST: &str = "text/uri-list";
    const KDE_CUT: &str = "application/x-kde-cutselection";
    const TEXT: [&str; 5] = ["UTF8_STRING", "text/plain;charset=utf-8", "text/plain", "STRING", "TEXT"];

    fn clipboard() -> Clipboard {
        Clipboard::get(&SELECTION_CLIPBOARD)
    }

    /// Coloca os arquivos na área de transferência em todos os formatos.
    pub fn set(contents: Contents) -> Result<(), String> {
        let mut names = vec![GNOME, URI_LIST];
        if contents.mode == Mode::Cut {
            names.push(KDE_CUT);
        }
        names.extend(TEXT);
        let targets: Vec<TargetEntry> =
            names.iter().enumerate().map(|(i, name)| TargetEntry::new(name, TargetFlags::empty(), i as u32)).collect();
        let gnome = gnome_copied_files(&contents);
        let uri_list = uri_list(&contents.paths);
        let text = plain_text(&contents.paths);
        let ok = clipboard().set_with_data(&targets, move |_, data, info| match names[info as usize] {
            GNOME => data.set(&Atom::intern(GNOME), 8, gnome.as_bytes()),
            URI_LIST => data.set(&Atom::intern(URI_LIST), 8, uri_list.as_bytes()),
            KDE_CUT => data.set(&Atom::intern(KDE_CUT), 8, b"1"),
            _ => {
                data.set_text(&text);
            }
        });
        if ok { Ok(()) } else { Err(crate::tr!("Não foi possível usar a área de transferência", "Could not use the clipboard")) }
    }

    fn read(clipboard: &Clipboard, target: &str) -> Option<String> {
        let data = clipboard.wait_for_contents(&Atom::intern(target))?;
        let bytes = data.data();
        (!bytes.is_empty()).then(|| String::from_utf8_lossy(&bytes).into_owned())
    }

    /// Arquivos na área de transferência, em qualquer dos formatos conhecidos.
    pub fn get() -> Option<Contents> {
        let clipboard = clipboard();
        if let Some(contents) = read(&clipboard, GNOME).and_then(|t| parse_gnome(&t)) {
            return Some(contents);
        }
        if let Some(text) = read(&clipboard, URI_LIST) {
            let paths = parse_uri_list(&text);
            if !paths.is_empty() {
                let cut = read(&clipboard, KDE_CUT).is_some_and(|t| t.trim() == "1");
                return Some(Contents { mode: if cut { Mode::Cut } else { Mode::Copy }, paths });
            }
        }
        clipboard.wait_for_text().and_then(|t| t.strip_prefix("x-special/nautilus-clipboard\n").and_then(parse_gnome))
    }

    /// Esvazia a área de transferência (após colar um recorte, como fazem os gerenciadores).
    pub fn clear() {
        clipboard().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uri_round_trip_with_spaces_and_accents() {
        let path = Path::new("/home/u/Vídeos/news 2024/mala-direta #1%.mp4");
        let uri = path_to_uri(path);
        assert_eq!(uri, "file:///home/u/V%C3%ADdeos/news%202024/mala-direta%20%231%25.mp4");
        assert_eq!(uri_to_path(&uri).unwrap(), path);
    }

    #[test]
    fn uri_to_path_accepts_only_local_files() {
        assert_eq!(uri_to_path("file://localhost/tmp/a").unwrap(), Path::new("/tmp/a"));
        assert_eq!(uri_to_path("file:///tmp/a%2"), None);
        assert_eq!(uri_to_path("file://servidor/tmp/a"), None);
        assert_eq!(uri_to_path("https://exemplo.com/a"), None);
        assert_eq!(uri_to_path("sftp://host/a"), None);
    }

    #[test]
    fn gnome_format_round_trip() {
        let contents = Contents { mode: Mode::Cut, paths: vec!["/tmp/a b.mp4".into(), "/tmp/c.png".into()] };
        let text = gnome_copied_files(&contents);
        assert_eq!(text, "cut\nfile:///tmp/a%20b.mp4\nfile:///tmp/c.png");
        assert_eq!(parse_gnome(&text).unwrap(), contents);
        assert_eq!(parse_gnome(&format!("x-special/nautilus-clipboard\n{text}\n")).unwrap(), contents);
        assert_eq!(parse_gnome("copy\n"), None);
        assert_eq!(parse_gnome("olá\nfile:///tmp/a"), None);
    }

    #[test]
    fn uri_list_round_trip() {
        let paths: Vec<PathBuf> = vec!["/tmp/a b.mp4".into(), "/tmp/ç.png".into()];
        let text = uri_list(&paths);
        assert_eq!(text, "file:///tmp/a%20b.mp4\r\nfile:///tmp/%C3%A7.png\r\n");
        assert_eq!(parse_uri_list(&format!("# comentário\r\n{text}https://x.com/y\r\n")), paths);
    }
}
