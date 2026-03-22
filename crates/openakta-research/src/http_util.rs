//! Shared HTTP error mapping.

use crate::error::SearchError;

pub(crate) fn map_reqwest(provider: &'static str, e: reqwest::Error) -> SearchError {
    if e.is_timeout() || e.is_connect() || e.is_request() {
        return SearchError::Transport {
            provider,
            message: e.to_string(),
        };
    }
    if let Some(status) = e.status() {
        return SearchError::Http {
            status: Some(status.as_u16()),
            provider,
            message: e.to_string(),
        };
    }
    SearchError::Transport {
        provider,
        message: e.to_string(),
    }
}

pub(crate) fn truncate_body(s: &str) -> String {
    const MAX: usize = 512;
    if s.len() <= MAX {
        return s.to_string();
    }
    let mut end = MAX;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_body_does_not_split_utf8_char() {
        let s = "€".repeat(300);
        assert!(s.len() > 512);
        let t = truncate_body(&s);
        assert!(t.ends_with('…'));
        assert!(t.chars().count() < s.chars().count());
    }
}
