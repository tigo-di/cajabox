//! Pastas de exemplo (~/Documentos/manabox-exemplo): mostram o programa funcionando na primeira
//! execução, ou a pedido em Configurações. Nunca sobrescrevem arquivos; na configuração entra só o
//! que falta. Os arquivos vêm de src-tauri/assets/examples/{pt,en} (gerados por scripts/example-assets.py).

use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::i18n::Lang;
use crate::index::{self, Index, tokenize};
use crate::paths::Folder;

/// Pasta dos exemplos no idioma, dentro da pasta Documentos do sistema (o nome varia com o idioma).
pub fn base_dir(lang: Lang) -> PathBuf {
    let documents = dirs::document_dir()
        .or_else(|| dirs::home_dir().map(|home| home.join("Documents")))
        .unwrap_or_default();
    documents.join(match lang {
        Lang::Pt => "manabox-exemplo",
        Lang::En => "manabox-examples",
    })
}

/// Arquivos de uma pasta de exemplo: caminho relativo e conteúdo.
type Files = &'static [(&'static str, &'static [u8])];

/// Exemplos de um idioma: termos que casam com os nomes dos arquivos, pastas e sufixo de nome em uso.
struct Examples {
    index: &'static str,
    /// (subpasta, descrição, arquivos)
    folders: [(&'static str, &'static str, Files); 2],
    taken_suffix: &'static str,
}

// O mesmo cartaz em todas as pastas; em Publicados/Published também numa subpasta, com o mesmo
// nome: o modo Duplicados (por nome e por conteúdo) tem o que mostrar.
const POSTER_PT: &[u8] = include_bytes!("../assets/examples/pt/poster.png");
const POSTER_EN: &[u8] = include_bytes!("../assets/examples/en/poster.png");

const PT: Examples = Examples {
    index: "# Canais\nnews\n- newsletter\ninsta\n- instagram\nemail\n- mala-direta\n# Campanhas\npromo\nlancamento\nblack-friday\n",
    folders: [
        (
            "Prontos",
            "pronto para publicar (exemplo)",
            &[
                ("insta-promo-black-friday-241129.example.png", POSTER_PT),
                ("newsletter-lancamento-250110.example.pdf", include_bytes!("../assets/examples/pt/lancamento-newsletter.pdf")),
                ("mala-direta-promo-2412.example.txt", include_bytes!("../assets/examples/pt/mala-direta.txt")),
            ],
        ),
        (
            "Publicados",
            "mídias publicadas (exemplo)",
            &[
                ("news-promo-black-friday-241129.example.png", POSTER_PT),
                ("2024/news-promo-black-friday-241129.example.png", POSTER_PT),
                ("instagram-lancamento-2501.example.pdf", include_bytes!("../assets/examples/pt/lancamento-instagram.pdf")),
                ("email-lancamento-250115.example.txt", include_bytes!("../assets/examples/pt/email.txt")),
            ],
        ),
    ],
    taken_suffix: " (exemplo)",
};

const EN: Examples = Examples {
    index: "# Channels\nnews\n- newsletter\ninsta\n- instagram\nemail\n- direct-mail\n# Campaigns\npromo\nlaunch\nblack-friday\n",
    folders: [
        (
            "Ready",
            "ready to publish (example)",
            &[
                ("insta-promo-black-friday-241129.example.png", POSTER_EN),
                ("newsletter-launch-250110.example.pdf", include_bytes!("../assets/examples/en/launch-newsletter.pdf")),
                ("direct-mail-promo-2412.example.txt", include_bytes!("../assets/examples/en/direct-mail.txt")),
            ],
        ),
        (
            "Published",
            "published media (example)",
            &[
                ("news-promo-black-friday-241129.example.png", POSTER_EN),
                ("2024/news-promo-black-friday-241129.example.png", POSTER_EN),
                ("instagram-launch-2501.example.pdf", include_bytes!("../assets/examples/en/launch-instagram.pdf")),
                ("email-launch-250115.example.txt", include_bytes!("../assets/examples/en/email.txt")),
            ],
        ),
    ],
    taken_suffix: " (example)",
};

/// Cria em `base` as pastas e arquivos de exemplo que faltam e acrescenta ao índice e às pastas
/// cadastradas o que falta (seções e termos pelo nome, pastas pelo caminho).
pub fn install(base: &Path, lang: Lang, index: &mut Index, folders: &mut Vec<Folder>) -> io::Result<()> {
    let examples = match lang {
        Lang::Pt => &PT,
        Lang::En => &EN,
    };
    for (dir, description, files) in &examples.folders {
        let path = base.join(dir);
        fs::create_dir_all(&path)?;
        for (name, content) in files.iter() {
            let file_path = path.join(name);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)?;
            }
            match OpenOptions::new().write(true).create_new(true).open(&file_path) {
                Ok(mut file) => file.write_all(content)?,
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(e),
            }
        }
        add_folder(folders, dir, description, &path, examples.taken_suffix);
    }
    merge_index(index, index::parse(examples.index));
    Ok(())
}

/// Cadastra a pasta se o caminho ainda não está cadastrado; nome em uso ganha `taken_suffix`.
fn add_folder(folders: &mut Vec<Folder>, name: &str, description: &str, path: &Path, taken_suffix: &str) {
    let path = path.to_string_lossy().into_owned();
    if folders.iter().any(|f| f.path == path) {
        return;
    }
    let taken = folders.iter().any(|f| f.name.to_lowercase() == name.to_lowercase());
    let name = if taken { format!("{name}{taken_suffix}") } else { name.to_string() };
    folders.push(Folder { name, description: description.into(), path });
}

/// Acrescenta seções e termos de `extra` que faltam em `index`; termo cujo nome ou sinônimo já
/// existe na seção fica de fora (evita repetição, que a validação recusaria).
fn merge_index(index: &mut Index, extra: Index) {
    for section in extra.sections {
        let position = match index.sections.iter().position(|s| s.name.to_lowercase() == section.name.to_lowercase()) {
            Some(position) => position,
            None => {
                index.sections.push(index::Section { name: section.name.clone(), terms: Vec::new() });
                index.sections.len() - 1
            }
        };
        let target = &mut index.sections[position];
        let mut used: HashSet<Vec<String>> =
            target.terms.iter().flat_map(|t| std::iter::once(&t.name).chain(&t.synonyms)).map(|n| tokenize(n)).collect();
        for term in section.terms {
            let names: Vec<Vec<String>> = std::iter::once(&term.name).chain(&term.synonyms).map(|n| tokenize(n)).collect();
            if names.iter().any(|n| used.contains(n)) {
                continue;
            }
            used.extend(names);
            target.terms.push(term);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classify::Classifier;
    use crate::paths;
    use crate::scan::list_files;

    #[test]
    fn creates_folders_files_index_and_registrations() {
        let base = tempfile::tempdir().unwrap();
        let (mut index, mut folders) = (Index::default(), Vec::new());
        install(base.path(), Lang::Pt, &mut index, &mut folders).unwrap();

        assert_eq!(folders.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["Prontos", "Publicados"]);
        paths::validate(&folders).unwrap();
        index::validate(&index).unwrap();
        let files = list_files(&base.path().join("Prontos"), &Classifier::new(&index::parse(&index::serialize(&index)))).unwrap();
        assert_eq!(files.len(), 3);
        // termos e data reconhecidos apesar do .example
        let png = files.iter().find(|f| f.name.ends_with(".png")).unwrap();
        assert_eq!(png.classification.terms, vec![vec![1], vec![0, 2]]); // insta; promo, black-friday
        assert_eq!(png.classification.date.map(|d| (d.year, d.month, d.day)), Some((2024, 11, Some(29))));

        // Publicados tem o cartaz repetido numa subpasta: duplicado por nome e por conteúdo
        let publicados = list_files(&base.path().join("Publicados"), &Classifier::new(&index)).unwrap();
        assert_eq!(publicados.len(), 4);
        assert_eq!(crate::duplicates::by_name(&publicados).len(), 1);
        let by_content = crate::duplicates::Search::default().by_content(&publicados, |_, _| {}).unwrap();
        assert_eq!(by_content.len(), 1);
    }

    #[test]
    fn never_overwrites_and_merges_only_what_is_missing() {
        let base = tempfile::tempdir().unwrap();
        let prontos = base.path().join("Prontos");
        fs::create_dir_all(&prontos).unwrap();
        let edited = prontos.join("mala-direta-promo-2412.example.txt");
        fs::write(&edited, "editado pelo usuário").unwrap();
        let mut index = index::parse("# canais\nnews\n- boletim\n# Cores\nazul\n");
        let mut folders = vec![Folder { name: "prontos".into(), description: "minha".into(), path: "/outro/lugar".into() }];

        install(base.path(), Lang::Pt, &mut index, &mut folders).unwrap();
        install(base.path(), Lang::Pt, &mut index, &mut folders).unwrap(); // segunda vez não repete nada

        assert_eq!(fs::read_to_string(&edited).unwrap(), "editado pelo usuário");
        let names: Vec<&str> = folders.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["prontos", "Prontos (exemplo)", "Publicados"]);
        let canais = &index.sections[0];
        assert_eq!(canais.name, "canais");
        assert_eq!(canais.terms.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(), ["news", "insta", "email"]);
        assert_eq!(canais.terms[0].synonyms, ["boletim"], "termo do usuário intacto");
        assert_eq!(index.sections.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["canais", "Cores", "Campanhas"]);
        index::validate(&index).unwrap();
    }

    #[test]
    fn english_examples_are_classified() {
        let base = tempfile::tempdir().unwrap();
        let (mut index, mut folders) = (Index::default(), Vec::new());
        install(base.path(), Lang::En, &mut index, &mut folders).unwrap();
        assert_eq!(folders.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["Ready", "Published"]);
        assert_eq!(index.sections.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["Channels", "Campaigns"]);
        let classifier = Classifier::new(&index::parse(&index::serialize(&index)));
        for file in list_files(&base.path().join("Ready"), &classifier).unwrap() {
            assert!(file.classification.terms.iter().all(|t| !t.is_empty()), "{} sem termo em alguma seção", file.name);
            assert!(file.classification.date.is_some(), "{} sem data", file.name);
        }
    }
}
