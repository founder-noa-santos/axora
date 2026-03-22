//! Token-bounded normalization of search hits.

use crate::types::{SearchOptions, SearchResult};

/// Truncate result count and string fields for LLM token budget.
pub fn normalize_results(mut v: Vec<SearchResult>, opts: &SearchOptions) -> Vec<SearchResult> {
    v.truncate(opts.max_results);
    v.into_iter()
        .map(|mut r| {
            r.title = truncate_chars(&r.title, opts.max_title_chars);
            r.snippet = truncate_chars(&r.snippet, opts.max_snippet_chars);
            r
        })
        .collect()
}

fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    s.chars().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_count_and_strings() {
        let v = vec![
            SearchResult {
                title: "a".repeat(200),
                url: "https://x".into(),
                snippet: "b".repeat(400),
            },
            SearchResult {
                title: "t2".into(),
                url: "https://y".into(),
                snippet: "s2".into(),
            },
        ];
        let opts = SearchOptions {
            max_results: 1,
            max_snippet_chars: 10,
            max_title_chars: 5,
        };
        let out = normalize_results(v, &opts);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].title.len(), 5);
        assert_eq!(out[0].snippet.len(), 10);
    }

    #[test]
    fn truncates_on_unicode_char_boundaries_not_bytes() {
        // "🦀" is 4 bytes each; 3 chars = 3 crabs, must not split a codepoint.
        let s = "🦀".repeat(10);
        let opts = SearchOptions {
            max_results: 5,
            max_snippet_chars: 3,
            max_title_chars: 100,
        };
        let v = vec![SearchResult {
            title: "t".into(),
            url: "u".into(),
            snippet: s,
        }];
        let out = normalize_results(v, &opts);
        assert_eq!(out[0].snippet.chars().count(), 3);
        assert_eq!(out[0].snippet, "🦀🦀🦀");
    }

    #[test]
    fn truncates_mixed_ascii_and_emoji() {
        let snippet = "hello 世界 🦀🦀🦀 extra";
        let opts = SearchOptions {
            max_results: 1,
            max_snippet_chars: 8,
            max_title_chars: 50,
        };
        let v = vec![SearchResult {
            title: "t".into(),
            url: "u".into(),
            snippet: snippet.into(),
        }];
        let out = normalize_results(v, &opts);
        assert_eq!(out[0].snippet.chars().count(), 8);
        assert!(out[0].snippet.starts_with("hello 世"));
    }

    #[test]
    fn max_results_zero_yields_empty() {
        let v = vec![SearchResult {
            title: "a".into(),
            url: "u".into(),
            snippet: "s".into(),
        }];
        let opts = SearchOptions {
            max_results: 0,
            max_snippet_chars: 100,
            max_title_chars: 100,
        };
        assert!(normalize_results(v, &opts).is_empty());
    }
}
