//! Idioma escolhido pelo usuário (português ou inglês), para as mensagens do núcleo.

use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    Pt,
    En,
}

impl Lang {
    pub fn code(self) -> &'static str {
        match self {
            Lang::Pt => "pt",
            Lang::En => "en",
        }
    }

    pub fn parse(code: &str) -> Option<Lang> {
        match code {
            "pt" => Some(Lang::Pt),
            "en" => Some(Lang::En),
            _ => None,
        }
    }
}

static ENGLISH: AtomicBool = AtomicBool::new(false);

pub fn set(lang: Lang) {
    ENGLISH.store(lang == Lang::En, Ordering::Relaxed);
}

pub fn current() -> Lang {
    if ENGLISH.load(Ordering::Relaxed) { Lang::En } else { Lang::Pt }
}

/// Texto no idioma atual, com a sintaxe de `format!`: `tr!("Erro em {path}", "Error in {path}")`.
#[macro_export]
macro_rules! tr {
    ($pt:literal, $en:literal $(, $arg:expr)* $(,)?) => {
        match $crate::i18n::current() {
            $crate::i18n::Lang::Pt => format!($pt $(, $arg)*),
            $crate::i18n::Lang::En => format!($en $(, $arg)*),
        }
    };
}
