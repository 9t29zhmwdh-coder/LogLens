// FTS5 helper: builds safe match expressions
pub fn build_fts_query(raw: &str) -> String {
    // If user uses operators, pass through; otherwise treat as phrase
    let has_operators = raw.contains(" AND ")
        || raw.contains(" OR ")
        || raw.contains(" NOT ")
        || raw.starts_with('"');

    if has_operators {
        raw.to_string()
    } else {
        // Escape quotes, wrap each token
        raw.split_whitespace()
            .map(|token| format!("\"{}\"", token.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_phrase_quoted() {
        let q = build_fts_query("connection refused");
        assert_eq!(q, "\"connection\" \"refused\"");
    }

    #[test]
    fn operator_passthrough() {
        let q = build_fts_query("error AND timeout");
        assert_eq!(q, "error AND timeout");
    }
}
