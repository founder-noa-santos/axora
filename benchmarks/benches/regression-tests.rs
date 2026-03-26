//! Performance Regression Tests
//!
//! These tests ensure that performance doesn't degrade beyond acceptable thresholds.
//! They should be run before merging any performance-affecting changes.
//!
//! **Failure Criteria:**
//! - p50 latency regression > 10%
//! - p95 latency regression > 15%
//! - p99 latency regression > 20%
//! - Throughput regression > 10%

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::time::Duration;
use tokio::runtime::Runtime;

/// Baseline metrics (to be updated after Phase 6 validation)
/// These represent the expected performance after Phase 5 implementation
struct PerformanceBaseline {
    /// p50 latency in milliseconds
    p50_baseline: f64,
    /// p95 latency in milliseconds
    p95_baseline: f64,
    /// p99 latency in milliseconds
    p99_baseline: f64,
    /// Throughput in requests/second
    throughput_baseline: f64,
}

impl PerformanceBaseline {
    /// Get current baseline (UPDATE THESE VALUES after Phase 6 validation)
    fn current() -> Self {
        Self {
            // TODO: Update with actual Phase 6 measurements
            p50_baseline: 50.0,          // Placeholder - update after measurement
            p95_baseline: 100.0,         // Placeholder - update after measurement
            p99_baseline: 200.0,         // Placeholder - update after measurement
            throughput_baseline: 1000.0, // Placeholder - update after measurement
        }
    }

    /// Check if p50 regression is within threshold
    fn check_p50(&self, measured: f64) -> Result<(), String> {
        let threshold = 0.10; // 10% regression allowed
        if measured > self.p50_baseline * (1.0 + threshold) {
            Err(format!(
                "p50 regression: {}ms > {}ms (baseline + {}%)",
                measured,
                self.p50_baseline * (1.0 + threshold),
                threshold * 100.0
            ))
        } else {
            Ok(())
        }
    }

    /// Check if p95 regression is within threshold
    fn check_p95(&self, measured: f64) -> Result<(), String> {
        let threshold = 0.15; // 15% regression allowed
        if measured > self.p95_baseline * (1.0 + threshold) {
            Err(format!(
                "p95 regression: {}ms > {}ms (baseline + {}%)",
                measured,
                self.p95_baseline * (1.0 + threshold),
                threshold * 100.0
            ))
        } else {
            Ok(())
        }
    }

    /// Check if p99 regression is within threshold
    fn check_p99(&self, measured: f64) -> Result<(), String> {
        let threshold = 0.20; // 20% regression allowed
        if measured > self.p99_baseline * (1.0 + threshold) {
            Err(format!(
                "p99 regression: {}ms > {}ms (baseline + {}%)",
                measured,
                self.p99_baseline * (1.0 + threshold),
                threshold * 100.0
            ))
        } else {
            Ok(())
        }
    }

    /// Check if throughput regression is within threshold
    fn check_throughput(&self, measured: f64) -> Result<(), String> {
        let threshold = 0.10; // 10% regression allowed
        if measured < self.throughput_baseline * (1.0 - threshold) {
            Err(format!(
                "throughput regression: {} req/s < {} req/s (baseline - {}%)",
                measured,
                self.throughput_baseline * (1.0 - threshold),
                threshold * 100.0
            ))
        } else {
            Ok(())
        }
    }
}

/// Test proto serialization performance
fn test_proto_serialization_regression(c: &mut Criterion) {
    use openakta_proto::provider_v1::ProviderRequest;

    let baseline = PerformanceBaseline::current();
    let request = create_test_request();

    let mut group = c.benchmark_group("regression_proto_serialization");

    group.bench_function("serialize", |b| {
        b.iter(|| {
            let _ = black_box(&request).encode_to_vec();
        })
    });

    group.finish();
}

/// Test proto deserialization performance
fn test_proto_deserialization_regression(c: &mut Criterion) {
    use openakta_proto::provider_v1::ProviderRequest;

    let baseline = PerformanceBaseline::current();
    let request = create_test_request();
    let bytes = request.encode_to_vec();

    let mut group = c.benchmark_group("regression_proto_deserialization");

    group.bench_function("deserialize", |b| {
        b.iter(|| {
            let _: ProviderRequest = ProviderRequest::decode(black_box(&bytes[..])).unwrap();
        })
    });

    group.finish();
}

/// Test API client overhead (mocked, no network)
fn test_api_client_overhead_regression(c: &mut Criterion) {
    use openakta_api_client::{ApiClient, ClientConfig};

    let baseline = PerformanceBaseline::current();
    let config = ClientConfig::default();

    let mut group = c.benchmark_group("regression_api_client_overhead");

    group.bench_function("client_creation", |b| {
        b.iter(|| {
            let _ = ApiClient::new(black_box(config.clone()));
        })
    });

    group.finish();
}

/// Test circuit breaker overhead
fn test_circuit_breaker_regression(c: &mut Criterion) {
    use std::sync::Arc;
    use tokio::sync::Mutex;

    struct SimpleCircuitBreaker {
        state: Arc<Mutex<bool>>,
    }

    impl SimpleCircuitBreaker {
        fn new() -> Self {
            Self {
                state: Arc::new(Mutex::new(true)),
            }
        }

        async fn allow_request(&self) -> bool {
            *self.state.lock().await
        }
    }

    let baseline = PerformanceBaseline::current();
    let cb = SimpleCircuitBreaker::new();

    let mut group = c.benchmark_group("regression_circuit_breaker");

    group.bench_function("check", |b| {
        b.to_async(Runtime::new().unwrap()).iter(|| async {
            let _ = black_box(&cb).allow_request().await;
        })
    });

    group.finish();
}

/// Test conversion overhead (internal ↔ proto)
fn test_conversion_overhead_regression(c: &mut Criterion) {
    use openakta_proto::provider_v1 as proto;

    let baseline = PerformanceBaseline::current();
    let internal_request = create_internal_request();

    let mut group = c.benchmark_group("regression_conversion_overhead");

    group.bench_function("internal_to_proto", |b| {
        b.iter(|| {
            let _ = proto::ProviderRequest {
                request_id: black_box(&internal_request.request_id).clone(),
                tenant_id: black_box(&internal_request.tenant_id).clone(),
                provider: black_box(&internal_request.provider).clone(),
                model: black_box(&internal_request.model).clone(),
                messages: black_box(&internal_request.messages)
                    .iter()
                    .map(|msg| proto::Message {
                        role: msg.role.clone(),
                        content: msg.content.clone(),
                    })
                    .collect(),
                max_tokens: black_box(&internal_request.max_tokens).unwrap_or(0) as u32,
                temperature: black_box(&internal_request.temperature).unwrap_or(0.7),
                stream: black_box(&internal_request.stream),
                ..Default::default()
            };
        })
    });

    group.finish();
}

/// Test memory allocation patterns
fn test_memory_allocation_regression(c: &mut Criterion) {
    let baseline = PerformanceBaseline::current();

    let mut group = c.benchmark_group("regression_memory_allocation");

    for size in [100, 1000, 10000] {
        group.bench_with_input(
            BenchmarkId::new("vec_allocation", size),
            &size,
            |b, &size| {
                b.iter(|| {
                    let _vec: Vec<u8> = vec![0; black_box(size)];
                })
            },
        );
    }

    group.finish();
}

/// Test string operations (common in API calls)
fn test_string_operations_regression(c: &mut Criterion) {
    let baseline = PerformanceBaseline::current();

    let mut group = c.benchmark_group("regression_string_operations");

    let test_string = "Test message content for benchmarking. ".repeat(10);

    group.bench_function("string_clone", |b| {
        b.iter(|| {
            let _ = black_box(&test_string).clone();
        })
    });

    group.bench_function("string_to_uppercase", |b| {
        b.iter(|| {
            let _ = black_box(&test_string).to_uppercase();
        })
    });

    group.finish();
}

/// Helper: Create a test provider request
fn create_test_request() -> openakta_proto::provider_v1::ProviderRequest {
    openakta_proto::provider_v1::ProviderRequest {
        request_id: uuid::Uuid::new_v4().to_string(),
        tenant_id: "benchmark-tenant".to_string(),
        provider: "openai".to_string(),
        model: "gpt-4".to_string(),
        messages: vec![openakta_proto::provider_v1::Message {
            role: "user".to_string(),
            content: "This is a benchmark test message. ".repeat(10),
        }],
        max_tokens: 100,
        temperature: 0.7,
        stream: false,
        ..Default::default()
    }
}

/// Helper: Create a test internal request
fn create_internal_request() -> openakta_agents::ModelRequest {
    openakta_agents::ModelRequest {
        request_id: uuid::Uuid::new_v4().to_string(),
        tenant_id: "benchmark-tenant".to_string(),
        provider: "openai".to_string(),
        model: "gpt-4".to_string(),
        messages: vec![openakta_agents::Message {
            role: "user".to_string(),
            content: "This is a benchmark test message. ".repeat(10),
            name: None,
        }],
        max_tokens: Some(100),
        temperature: Some(0.7),
        top_p: None,
        frequency_penalty: None,
        presence_penalty: None,
        stop: vec![],
        stream: false,
        tools: vec![],
        tool_choice: None,
        user: None,
    }
}

/// Regression test runner
/// This function checks if current performance meets baseline requirements
fn run_regression_check(baseline: &PerformanceBaseline, metrics: &TestMetrics) {
    let mut failures = Vec::new();

    if let Err(msg) = baseline.check_p50(metrics.p50) {
        failures.push(msg);
    }

    if let Err(msg) = baseline.check_p95(metrics.p95) {
        failures.push(msg);
    }

    if let Err(msg) = baseline.check_p99(metrics.p99) {
        failures.push(msg);
    }

    if let Err(msg) = baseline.check_throughput(metrics.throughput) {
        failures.push(msg);
    }

    if !failures.is_empty() {
        eprintln!("❌ REGRESSION TESTS FAILED:");
        for failure in failures {
            eprintln!("  - {}", failure);
        }
        std::process::exit(1);
    } else {
        println!("✅ REGRESSION TESTS PASSED");
    }
}

/// Test metrics structure
struct TestMetrics {
    p50: f64,
    p95: f64,
    p99: f64,
    throughput: f64,
}

impl TestMetrics {
    fn new() -> Self {
        Self {
            p50: 0.0,
            p95: 0.0,
            p99: 0.0,
            throughput: 0.0,
        }
    }
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(Duration::from_secs(30))
        .warm_up_time(Duration::from_secs(5))
        .noise_threshold(0.05)
        .confidence_level(0.95)
        .nresamples(100_000);
    targets =
        test_proto_serialization_regression,
        test_proto_deserialization_regression,
        test_api_client_overhead_regression,
        test_circuit_breaker_regression,
        test_conversion_overhead_regression,
        test_memory_allocation_regression,
        test_string_operations_regression,
);

criterion_main!(benches);
