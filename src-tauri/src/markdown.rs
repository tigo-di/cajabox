/// Remove blocos `<!-- -->` (podem ocupar várias linhas). Bloco não fechado vai até o fim do texto.
pub fn strip_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start + 4..].find("-->") {
            Some(end) => rest = &rest[start + 4 + end + 3..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_multiline_comments() {
        assert_eq!(strip_comments("a<!--x\ny-->b<!--z-->c"), "abc");
    }

    #[test]
    fn unclosed_comment_runs_to_end() {
        assert_eq!(strip_comments("a<!-- x"), "a");
    }
}
