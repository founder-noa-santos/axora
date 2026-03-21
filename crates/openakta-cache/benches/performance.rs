//! Performance Benchmarks for Phase 2 Features
//!
//! This benchmark suite measures the performance of:
//! - Prefix caching operations (Sprint 1)
//! - Living docs change detection (Sprint 6)
//! - Document indexing and retrieval (Sprint 6)
//! - ADR operations (Sprint 6)

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use openakta_cache::{CodeMinifier, PrefixCache};
use openakta_docs::{Adr, AdrLog, DocIndex, DocSchema, Document, LivingDocs};
use std::path::Path;
use std::time::Duration;

fn benchmark_prefix_cache_operations(c: &mut Criterion) {
    let mut cache = PrefixCache::new(100);

    // Add some prefixes
    cache.add("system", "You are a helpful AI assistant", 10);
    cache.add("context", "The user is asking about Rust programming", 10);
    cache.add("format", "Respond in markdown format", 10);

    let mut group = c.benchmark_group("prefix_cache");

    group.bench_function("cache_hit", |b| {
        b.iter(|| {
            let key = "You are a helpful AI assistant";
            let result = cache.get(key);
            black_box(result);
        })
    });

    group.bench_function("cache_miss", |b| {
        b.iter(|| {
            let key = "Unknown prefix that doesn't exist";
            let result = cache.get(key);
            black_box(result);
        })
    });

    group.bench_function("add_prefix", |b| {
        b.iter(|| {
            cache.add(
                black_box("test"),
                black_box("Test prefix content for benchmarking"),
                black_box(10),
            );
        })
    });

    group.finish();
}

fn benchmark_code_minification_performance(c: &mut Criterion) {
    let minifier = CodeMinifier::new();

    let code_sizes = vec![
        ("small", "fn add(a: i32, b: i32) -> i32 { a + b }"),
        (
            "medium",
            r#"
pub fn authenticate_user(username: &str, password: &str) -> Result<Token, AuthError> {
    let user = find_user(username)?;
    if verify_password(&user.password_hash, password) {
        Ok(Token::new(user.id))
    } else {
        Err(AuthError::InvalidCredentials)
    }
}
"#,
        ),
        ("large", include_str!("../benches/token_savings.rs")),
    ];

    let mut group = c.benchmark_group("code_minification");

    for (size_name, code) in code_sizes {
        group.bench_with_input(BenchmarkId::from_parameter(size_name), code, |b, code| {
            b.iter(|| {
                let minified = minifier.minify(black_box(code), "rust").unwrap();
                black_box(minified);
            })
        });
    }

    group.finish();
}

fn benchmark_living_docs_performance(c: &mut Criterion) {
    let mut living_docs = LivingDocs::new();

    // Pre-register some files
    for i in 0..10 {
        let doc = Document::new(
            &format!("doc-{}", i),
            DocSchema::new("test", "1.0", "benchmark"),
            format!("Documentation content for doc {}", i),
            "1.0.0",
        );
        living_docs.add_document(doc).expect("Failed to add doc");
        living_docs.register_file(
            Path::new(&format!("src/file_{}.rs", i)),
            &format!("doc-{}", i),
            &format!("fn test_{}() {{}}", i),
        );
    }

    let old_code = "fn test() { let x = 1; let y = 2; let z = 3; }";
    let new_code = "fn test() { let x = 1; let y = 2; let z = 4; }";

    let mut group = c.benchmark_group("living_docs");

    group.bench_function("code_change_detection", |b| {
        b.iter(|| {
            let updates = living_docs.on_code_change(
                black_box(Path::new("src/file_0.rs")),
                black_box(old_code),
                black_box(new_code),
            );
            black_box(updates);
        })
    });

    group.bench_function("register_file", |b| {
        b.iter(|| {
            living_docs.register_file(
                black_box(Path::new("src/new_file.rs")),
                black_box("new-doc"),
                black_box("fn new_function() {}"),
            );
        })
    });

    group.bench_function("find_stale_docs", |b| {
        b.iter(|| {
            let stale = living_docs.find_stale_docs();
            black_box(stale);
        })
    });

    group.finish();
}

fn benchmark_document_index_performance(c: &mut Criterion) {
    let mut index = DocIndex::new();

    // Add documents
    for i in 0..100 {
        let doc = Document::new(
            &format!("doc-{:03}", i),
            DocSchema::new(&format!("module-{}", i % 10), "1.0", "benchmark"),
            format!(
                "Documentation for module {} with content about authentication and user management",
                i
            ),
            "1.0.0",
        );
        index.add(doc).expect("Failed to add doc");
    }

    let mut group = c.benchmark_group("doc_index");

    group.bench_function("retrieve_by_keyword", |b| {
        b.iter(|| {
            let results = index.retrieve(&openakta_docs::DocQuery::new(&["authentication"]));
            black_box(results);
        })
    });

    group.bench_function("retrieve_with_module_filter", |b| {
        b.iter(|| {
            let results = index.retrieve(
                &openakta_docs::DocQuery::new(&["documentation"]).with_module("module-5"),
            );
            black_box(results);
        })
    });

    group.bench_function("search_simple", |b| {
        b.iter(|| {
            let results = index.search(black_box(&["user", "management"]));
            black_box(results);
        })
    });

    group.bench_function("find_stale", |b| {
        b.iter(|| {
            let stale = index.find_stale(black_box(30));
            black_box(stale);
        })
    });

    group.bench_function("get_document", |b| {
        b.iter(|| {
            let doc = index.get(black_box("doc-050"));
            black_box(doc);
        })
    });

    group.finish();
}

fn benchmark_adr_operations(c: &mut Criterion) {
    let mut adr_log = AdrLog::new();

    // Add ADRs
    for i in 0..50 {
        let adr = Adr::new(
            &format!("ADR-{:03}", i),
            &format!("Decision title {}", i),
            &format!("Context and background for decision {}", i),
            &format!("The decision made for item {}", i),
            "benchmark",
        );
        adr_log.add(adr).expect("Failed to add ADR");
    }

    // Link some ADRs
    for i in 0..49 {
        adr_log
            .link(&format!("ADR-{:03}", i), &format!("ADR-{:03}", i + 1))
            .ok();
    }

    let mut group = c.benchmark_group("adr_log");

    group.bench_function("get_adr", |b| {
        b.iter(|| {
            let adr = adr_log.get(black_box("ADR-025"));
            black_box(adr);
        })
    });

    group.bench_function("search_adr", |b| {
        b.iter(|| {
            let results = adr_log.search(black_box(&["decision", "context"]));
            black_box(results);
        })
    });

    group.bench_function("by_category", |b| {
        b.iter(|| {
            let adrs = adr_log.by_category(black_box("ADR"));
            black_box(adrs);
        })
    });

    group.bench_function("active_adrs", |b| {
        b.iter(|| {
            let active = adr_log.active();
            black_box(active);
        })
    });

    // Note: link_adrs benchmark removed - AdrLog doesn't implement Clone
    // Linking is a fast operation and is covered by other benchmarks

    group.finish();
}

fn benchmark_decompress_performance(c: &mut Criterion) {
    let minifier = CodeMinifier::new();

    let code = r#"
pub fn calculateMonthlyRevenueMetrics(
    totalRevenueAmount: number,
    previousMonthRevenue: number,
    transactionCount: number
) -> RevenueMetricsObject {
    const averageTransactionValue = totalRevenueAmount / transactionCount;
    const revenueGrowthRate = (totalRevenueAmount - previousMonthRevenue) / previousMonthRevenue;
    return {
        totalRevenue: totalRevenueAmount,
        averageTransaction: averageTransactionValue,
        growthRate: revenueGrowthRate,
    };
}
"#;

    let minified = minifier.minify(code, "rust").unwrap();

    c.bench_function("decompress_identifiers", |b| {
        b.iter(|| {
            let decompressed = minifier.decompress(black_box(&minified));
            let _ = black_box(decompressed);
        })
    });
}

fn benchmark_combined_workflow(c: &mut Criterion) {
    let mut living_docs = LivingDocs::new();
    let minifier = CodeMinifier::new();

    // Setup
    let doc = Document::new(
        "auth-api",
        DocSchema::new("auth", "1.0", "benchmark"),
        "# Auth API Documentation".to_string(),
        "1.0.0",
    );
    living_docs.add_document(doc).expect("Failed to add");

    let initial_code = r#"
pub fn login(username: &str, password: &str) -> Result<Token, AuthError> {
    let user = find_user(username)?;
    verify_password(&user.password_hash, password)
        .then(|| Token::new(user.id))
        .ok_or(AuthError::InvalidCredentials)
}
"#;

    living_docs.register_file(Path::new("src/auth.rs"), "auth-api", initial_code);

    let modified_code = r#"
pub fn login(username: &str, password: &str) -> Result<Token, AuthError> {
    let user = find_user(username)?;
    if verify_password(&user.password_hash, password) {
        Ok(Token::new(user.id))
    } else {
        Err(AuthError::InvalidCredentials)
    }
}
"#;

    c.bench_function("full_workflow", |b| {
        b.iter(|| {
            // Step 1: Detect code change
            let updates =
                living_docs.on_code_change(Path::new("src/auth.rs"), initial_code, modified_code);

            // Step 2: Minify the modified code
            let minified = minifier.minify(modified_code, "rust").unwrap();

            // Step 3: Verify decompression works
            let _decompressed = minifier.decompress(&minified).unwrap();

            black_box((updates, minified));
        })
    });
}

criterion_group!(
    name = performance_benches;
    config = Criterion::default().measurement_time(Duration::from_secs(10));
    targets =
        benchmark_prefix_cache_operations,
        benchmark_code_minification_performance,
        benchmark_living_docs_performance,
        benchmark_document_index_performance,
        benchmark_adr_operations,
        benchmark_decompress_performance,
        benchmark_combined_workflow,
);
criterion_main!(performance_benches);
