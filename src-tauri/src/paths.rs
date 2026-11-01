use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::markdown::strip_comments;

/// Conteúdo inicial do manabox.paths.md; o comentário de ajuda é mantido ao gravar.
pub const TEMPLATE: &str = include_str!("../../paths-sample.md");

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Folder {
    pub name: String,
    pub description: String,
    pub path: String,
}

/// Lê o conteúdo de um manabox.paths.md: uma pasta por linha, `nome|descrição|caminho`.
/// Linhas sem os três campos são ignoradas.
pub fn parse(text: &str) -> Vec<Folder> {
    strip_comments(text)
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('|').map(str::trim);
            let (name, description, path) = (fields.next()?, fields.next()?, fields.next()?);
            if fields.next().is_some() || name.is_empty() || path.is_empty() {
                return None;
            }
            Some(Folder {
                name: name.to_string(),
                description: description.to_string(),
                path: path.to_string(),
            })
        })
        .collect()
}

/// Nome e caminho absoluto obrigatórios, nomes únicos, nenhum campo com `|` ou quebra de linha.
pub fn validate(folders: &[Folder]) -> Result<(), String> {
    let mut names = HashSet::new();
    for folder in folders {
        let label = if folder.name.is_empty() { &folder.path } else { &folder.name };
        if folder.name.trim().is_empty() {
            return Err(format!("Pasta sem nome: {label}"));
        }
        if !Path::new(&folder.path).is_absolute() {
            return Err(format!("{label}: o caminho precisa ser absoluto (começar com /)"));
        }
        for field in [&folder.name, &folder.description, &folder.path] {
            if field.contains(['|', '\n', '\r']) || field.trim() != field {
                return Err(format!("{label}: campos não podem conter | nem quebras de linha, nem espaços nas pontas"));
            }
        }
        if !names.insert(folder.name.to_lowercase()) {
            return Err(format!("Nome repetido: {}", folder.name));
        }
    }
    Ok(())
}

/// Gera o manabox.paths.md: comentário de ajuda seguido de uma pasta por linha.
pub fn serialize(folders: &[Folder]) -> String {
    let mut out = format!("{}\n\n", TEMPLATE.trim_end());
    for folder in folders {
        out.push_str(&format!("{}|{}|{}\n", folder.name, folder.description, folder.path));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str, description: &str, path: &str) -> Folder {
        Folder { name: name.into(), description: description.into(), path: path.into() }
    }

    #[test]
    fn serialize_round_trips_and_keeps_help_comment() {
        let folders = vec![folder("published", "mídias publicadas", "/a/b"), folder("ready", "", "/c d/e")];
        let text = serialize(&folders);
        assert!(text.starts_with(TEMPLATE.trim_end()));
        assert_eq!(parse(&text), folders);
    }

    #[test]
    fn validate_rejects_invalid_folders() {
        assert!(validate(&[folder("a", "", "/x"), folder("b", "desc", "/y")]).is_ok());
        for invalid in [
            vec![folder("", "", "/x")],
            vec![folder("a", "", "relativo/x")],
            vec![folder("a", "com|barra", "/x")],
            vec![folder("a", "linha\nnova", "/x")],
            vec![folder(" a", "", "/x")],
            vec![folder("a", "", "/x"), folder("A", "", "/y")],
        ] {
            assert!(validate(&invalid).is_err(), "{invalid:?}");
        }
    }

    #[test]
    fn parses_folders_and_skips_invalid_lines() {
        let folders = parse(
            "<!--\nx|y|/z\n-->\npublished | midias publicadas | /a/published\nlixo\n|sem nome|/b\nready||/c\na|b|c|d\n",
        );
        assert_eq!(
            folders,
            vec![
                Folder { name: "published".into(), description: "midias publicadas".into(), path: "/a/published".into() },
                Folder { name: "ready".into(), description: "".into(), path: "/c".into() },
            ]
        );
    }

    #[test]
    fn sample_file_has_no_folders() {
        assert!(parse(include_str!("../../paths-sample.md")).is_empty());
    }
}
