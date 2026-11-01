use serde::Serialize;

use crate::index::{tokenize, Index};

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct FileDate {
    pub year: u16,
    pub month: u8,
    pub day: Option<u8>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Classification {
    /// Uma lista por seção do índice, com as posições dos termos encontrados.
    pub terms: Vec<Vec<usize>>,
    pub date: Option<FileDate>,
}

/// Classifica nomes de arquivo segundo os termos de um índice.
pub struct Classifier {
    /// Cada sequência de tokens com os (seção, termo) a que pertence, da mais longa para a mais curta.
    patterns: Vec<(Vec<String>, Vec<(usize, usize)>)>,
    sections: usize,
}

impl Classifier {
    pub fn new(index: &Index) -> Self {
        let mut patterns: Vec<(Vec<String>, Vec<(usize, usize)>)> = Vec::new();
        for (s, section) in index.sections.iter().enumerate() {
            for (t, term) in section.terms.iter().enumerate() {
                for pattern in &term.patterns {
                    match patterns.iter_mut().find(|(p, _)| p == pattern) {
                        Some((_, owners)) => owners.push((s, t)),
                        None => patterns.push((pattern.clone(), vec![(s, t)])),
                    }
                }
            }
        }
        patterns.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then_with(|| a.0.cmp(&b.0)));
        Classifier { patterns, sections: index.sections.len() }
    }

    /// Termos compostos têm preferência: tokens usados por `mala-direta` não casam com `mala` ou `direta`.
    pub fn classify(&self, file_name: &str) -> Classification {
        let tokens = tokenize(stem(file_name));
        let mut used = vec![false; tokens.len()];
        let mut terms = vec![Vec::new(); self.sections];

        for (pattern, owners) in &self.patterns {
            let len = pattern.len();
            let mut i = 0;
            while i + len <= tokens.len() {
                if !used[i..i + len].iter().any(|&u| u) && tokens[i..i + len] == pattern[..] {
                    used[i..i + len].fill(true);
                    for &(s, t) in owners {
                        terms[s].push(t);
                    }
                    i += len;
                } else {
                    i += 1;
                }
            }
        }
        for found in &mut terms {
            found.sort_unstable();
            found.dedup();
        }

        let mut dates = tokens.iter().filter_map(|t| parse_date(t));
        let date = match (dates.next(), dates.next()) {
            (Some(date), None) => Some(date),
            _ => None,
        };
        Classification { terms, date }
    }
}

/// Nome sem extensão. Arquivos como `.bashrc` não têm extensão.
fn stem(file_name: &str) -> &str {
    match file_name.rfind('.') {
        Some(dot) if dot > 0 => &file_name[..dot],
        _ => file_name,
    }
}

/// `YYMM` ou `YYMMDD`, ano 20YY, com mês e dia válidos.
fn parse_date(token: &str) -> Option<FileDate> {
    if !matches!(token.len(), 4 | 6) || !token.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let two = |i: usize| token[i..i + 2].parse::<u8>().unwrap();
    let (year, month) = (2000 + two(0) as u16, two(2));
    if !(1..=12).contains(&month) {
        return None;
    }
    let day = if token.len() == 6 {
        let day = two(4);
        if day < 1 || day > days_in_month(year, month) {
            return None;
        }
        Some(day)
    } else {
        None
    };
    Some(FileDate { year, month, day })
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::parse;

    fn none() -> Vec<usize> {
        Vec::new()
    }

    fn classifier() -> Classifier {
        Classifier::new(&parse(
            "# Channels\nnews\n - newsletter\n - mala-direta\n - email\ninsta\n - instagram\n# Words\nmala\ndireta\nemail\n",
        ))
    }

    #[test]
    fn matches_whole_tokens_case_insensitive() {
        let c = classifier();
        assert_eq!(c.classify("Newsletter-Promo.mp4").terms, vec![vec![0], none()]);
        assert_eq!(c.classify("newsx-promo.mp4").terms, vec![none(), none()]);
        assert_eq!(c.classify("post INSTA news.png").terms, vec![vec![0, 1], none()]);
    }

    #[test]
    fn compound_terms_take_precedence() {
        let c = classifier();
        assert_eq!(c.classify("mala-direta-promo.pdf").terms, vec![vec![0], none()]);
        assert_eq!(c.classify("mala promo direta.pdf").terms, vec![none(), vec![0, 1]]);
    }

    #[test]
    fn same_synonym_in_two_sections_matches_both() {
        assert_eq!(classifier().classify("email-x.html").terms, vec![vec![0], vec![2]]);
    }

    #[test]
    fn extension_is_not_a_token() {
        assert_eq!(classifier().classify("promo.news").terms, vec![none(), none()]);
    }

    #[test]
    fn parses_dates() {
        let c = classifier();
        assert_eq!(c.classify("news-241031.mp4").date, Some(FileDate { year: 2024, month: 10, day: Some(31) }));
        assert_eq!(c.classify("2410 news.mp4").date, Some(FileDate { year: 2024, month: 10, day: None }));
        assert_eq!(c.classify("news-240229.mp4").date.map(|d| d.day), Some(Some(29)));
    }

    #[test]
    fn rejects_invalid_or_embedded_dates() {
        let c = classifier();
        for name in ["news-241301.mp4", "news-230229.mp4", "news-241032.mp4", "news_241031.mp4", "news-24103.mp4", "1080.mp4"] {
            assert_eq!(c.classify(name).date, None, "{name}");
        }
    }

    #[test]
    fn two_dates_means_no_date() {
        assert_eq!(classifier().classify("2410-241031.mp4").date, None);
        assert!(classifier().classify("2410-1080.mp4").date.is_some());
    }
}
