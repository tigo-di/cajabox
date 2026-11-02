use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::markdown::strip_comments;

/// Conteúdo inicial do manabox.index.md; o comentário de ajuda é mantido ao gravar.
pub const TEMPLATE: &str = include_str!("../../index-sample.md");
const TEMPLATE_EN: &str = include_str!("../../index-sample.en.md");

/// Comentário de ajuda no topo do arquivo, no idioma atual.
pub fn template() -> &'static str {
    match crate::i18n::current() {
        crate::i18n::Lang::Pt => TEMPLATE,
        crate::i18n::Lang::En => TEMPLATE_EN,
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Term {
    pub name: String,
    /// Sinônimos já normalizados em tokens; o próprio nome do termo é o primeiro.
    /// Não vem da interface: é recalculado ao reler o arquivo gravado.
    #[serde(skip)]
    pub patterns: Vec<Vec<String>>,
    /// Sinônimos como escritos no manabox.index.md (sem o nome do termo).
    pub synonyms: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Section {
    pub name: String,
    pub terms: Vec<Term>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Index {
    pub sections: Vec<Section>,
}

/// Divide um texto em tokens minúsculos separados por `-` ou espaço.
pub fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| c == '-' || c == '_' || c.is_whitespace())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_lowercase())
        .collect()
}

impl Term {
    fn new(name: &str) -> Self {
        Term {
            name: name.to_string(),
            patterns: vec![tokenize(name)],
            synonyms: Vec::new(),
        }
    }

    fn add_synonym(&mut self, synonym: &str) {
        let tokens = tokenize(synonym);
        if !tokens.is_empty() && !self.patterns.contains(&tokens) {
            self.patterns.push(tokens);
            self.synonyms.push(synonym.to_string());
        }
    }
}

/// Lê o conteúdo de um manabox.index.md.
///
/// `# Nome` inicia seção, linha comum é termo, `- x` é sinônimo do termo anterior.
/// Linhas antes da primeira seção e sinônimos sem termo são ignorados.
pub fn parse(text: &str) -> Index {
    let mut index = Index::default();
    for line in strip_comments(text).lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(name) = line.strip_prefix('#') {
            index.sections.push(Section {
                name: name.trim().to_string(),
                terms: Vec::new(),
            });
            continue;
        }
        let Some(section) = index.sections.last_mut() else {
            continue;
        };
        if let Some(synonym) = line.strip_prefix('-') {
            if let Some(term) = section.terms.last_mut() {
                term.add_synonym(synonym.trim());
            }
        } else if !tokenize(line).is_empty() {
            section.terms.push(Term::new(line));
        }
    }
    index
}

/// Seções com nome único; termos e sinônimos não vazios, sem quebra de linha,
/// sem `#`/`-` no início (seriam lidos como seção/sinônimo). Dentro de uma seção nenhum
/// termo ou sinônimo se repete (`mala-direta` = `Mala Direta`); entre seções pode.
pub fn validate(index: &Index) -> Result<(), String> {
    let single_line = |text: &str| !text.contains(['\n', '\r']);
    let mut sections = HashSet::new();
    for section in &index.sections {
        let name = section.name.trim();
        if name.is_empty() || !single_line(name) {
            return Err(crate::tr!("Seção sem nome ou com quebra de linha", "Section without a name or with a line break"));
        }
        if !sections.insert(name.to_lowercase()) {
            return Err(crate::tr!("Seção repetida: {name}", "Repeated section: {name}"));
        }
        let mut seen = HashSet::new();
        for term in &section.terms {
            let label = format!("{name} › {}", term.name.trim());
            let term_name = term.name.trim();
            if tokenize(term_name).is_empty() || !single_line(term_name) {
                return Err(crate::tr!("{name}: termo vazio ou com quebra de linha", "{name}: empty term or term with a line break"));
            }
            if term_name.starts_with(['#', '-']) {
                return Err(crate::tr!("{label}: termo não pode começar com # ou -", "{label}: a term cannot start with # or -"));
            }
            if !seen.insert(tokenize(term_name)) {
                return Err(crate::tr!("{label}: repetido na seção", "{label}: repeated in the section"));
            }
            for synonym in &term.synonyms {
                if tokenize(synonym).is_empty() || !single_line(synonym) {
                    return Err(crate::tr!("{label}: sinônimo vazio ou com quebra de linha", "{label}: empty synonym or synonym with a line break"));
                }
                if !seen.insert(tokenize(synonym)) {
                    return Err(crate::tr!(
                        "{label}: sinônimo “{}” repetido na seção",
                        "{label}: synonym “{}” repeated in the section",
                        synonym.trim()
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Gera o manabox.index.md: comentário de ajuda seguido das seções, termos e sinônimos indentados.
pub fn serialize(index: &Index) -> String {
    let mut out = format!("{}\n", template().trim_end());
    for section in &index.sections {
        out.push_str(&format!("\n# {}\n", section.name.trim()));
        for term in &section.terms {
            out.push_str(&format!("{}\n", term.name.trim()));
            for synonym in &term.synonyms {
                out.push_str(&format!("    - {}\n", synonym.trim()));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn term(name: &str, synonyms: &[&str]) -> Term {
        Term { name: name.into(), patterns: Vec::new(), synonyms: synonyms.iter().map(|s| s.to_string()).collect() }
    }

    fn index(sections: Vec<(&str, Vec<Term>)>) -> Index {
        Index { sections: sections.into_iter().map(|(name, terms)| Section { name: name.into(), terms }).collect() }
    }

    #[test]
    fn serialize_round_trips_and_keeps_help_comment() {
        let original = index(vec![
            ("Channels", vec![term("news", &["newsletter", "mala-direta"]), term("insta", &[])]),
            ("Colors", vec![term("orange", &[])]),
            ("Vazia", vec![]),
        ]);
        let text = serialize(&original);
        assert!(text.starts_with(TEMPLATE.trim_end()));
        let parsed = parse(&text);
        let names = |i: &Index| -> Vec<(String, Vec<(String, Vec<String>)>)> {
            i.sections
                .iter()
                .map(|s| (s.name.clone(), s.terms.iter().map(|t| (t.name.clone(), t.synonyms.clone())).collect()))
                .collect()
        };
        assert_eq!(names(&parsed), names(&original));
        assert_eq!(parsed.sections[0].terms[0].patterns.len(), 3);
    }

    #[test]
    fn validate_rejects_invalid_index() {
        assert!(validate(&index(vec![("A", vec![term("x", &["y"])]), ("B", vec![term("x", &[])])])).is_ok());
        for invalid in [
            index(vec![("", vec![])]),
            index(vec![("A", vec![]), ("a", vec![])]),
            index(vec![("A", vec![term(" ", &[])])]),
            index(vec![("A", vec![term("#x", &[])])]),
            index(vec![("A", vec![term("-x", &[])])]),
            index(vec![("A", vec![term("x\ny", &[])])]),
            index(vec![("A", vec![term("mala-direta", &[]), term("Mala Direta", &[])])]),
            index(vec![("A", vec![term("news", &["mala-direta", "Mala Direta"])])]),
            index(vec![("A", vec![term("news", &["mala-direta"]), term("Mala Direta", &[])])]),
            index(vec![("A", vec![term("news", &["insta"]), term("insta", &[])])]),
            index(vec![("A", vec![term("news", &["News"])])]),
            index(vec![("A", vec![term("x", &[""])])]),
        ] {
            assert!(validate(&invalid).is_err(), "{invalid:?}");
        }
    }

    #[test]
    fn tokenize_splits_on_dash_underscore_and_space_lowercased() {
        assert_eq!(tokenize(" Mala-Direta  news\tX "), ["mala", "direta", "news", "x"]);
        assert_eq!(tokenize("mala_direta__News-x y"), ["mala", "direta", "news", "x", "y"]);
    }

    #[test]
    fn parses_sections_terms_and_synonyms() {
        let index = parse(
            "<!-- comentário\n# Ignorada\n-->\nsolto\n# Colors\norange\n  blue\n\n# Channels\nnews\n    - newsletter\n    - mala-direta\n  - news\ninsta\n - instagram\n",
        );
        assert_eq!(index.sections.len(), 2);
        assert_eq!(index.sections[0].name, "Colors");
        let colors: Vec<_> = index.sections[0].terms.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(colors, ["orange", "blue"]);

        let news = &index.sections[1].terms[0];
        assert_eq!(news.name, "news");
        assert_eq!(news.synonyms, ["newsletter", "mala-direta"]);
        assert_eq!(
            news.patterns,
            vec![vec!["news".to_string()], vec!["newsletter".into()], vec!["mala".into(), "direta".into()]]
        );
        assert_eq!(index.sections[1].terms[1].synonyms, ["instagram"]);
    }

    #[test]
    fn sample_file_has_no_sections() {
        assert!(parse(include_str!("../../index-sample.md")).sections.is_empty());
    }
}
