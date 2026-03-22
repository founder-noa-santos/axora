#![cfg(feature = "local-memory")]

use openakta_research::{EmbeddingProvider, ResearchStorage, SearchResult, RESEARCH_EMBED_DIM};

struct MockEmbedding;

impl EmbeddingProvider for MockEmbedding {
    fn canonicalize(&self, title: &str, url: &str, snippet: &str) -> String {
        format!("{title}\n{url}\n{snippet}")
    }

    fn embed_text(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        let mut v = vec![0f32; RESEARCH_EMBED_DIM];
        for (i, b) in text.bytes().enumerate() {
            v[i % RESEARCH_EMBED_DIM] += b as f32 / 255.0;
        }
        let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        if n > 0.0 {
            for x in &mut v {
                *x /= n;
            }
        }
        Ok(v)
    }
}

#[test]
fn append_and_search_semantic_and_fts() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("r.db");
    let mut store = ResearchStorage::open(&db).unwrap();
    let embedder = MockEmbedding;

    let hits = vec![
        SearchResult {
            title: "Rust book".into(),
            url: "https://doc.rust-lang.org/book/".into(),
            snippet: "ownership and borrowing".into(),
        },
        SearchResult {
            title: "Tokio".into(),
            url: "https://tokio.rs/".into(),
            snippet: "async runtime".into(),
        },
    ];

    let ws = "/tmp/ws";
    store
        .append_session(ws, "async rust", Some("serper"), &hits, &embedder)
        .unwrap();

    let q = embedder.embed_text("async runtime").unwrap();
    let sem = store.search_historical_research(ws, &q, 2).unwrap();
    assert!(!sem.is_empty());

    let kw = store.search_keywords(ws, "tokio", 5).unwrap();
    assert_eq!(kw.len(), 1);
    assert!(kw[0].title.contains("Tokio"));
}
