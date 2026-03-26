//! API Overhead Benchmarks
//!
//! Measures the overhead introduced by the API client layer:
//! - Proto serialization/deserialization
//! - Network round-trip latency
//! - Circuit breaker check overhead
//! - Total API overhead

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use openakta_api_client::{ApiClient, ApiClientPool, ClientConfig};
use openakta_proto::provider_v1::{ProviderRequest, ProviderResponse, ProviderResponseChunk};
use std::time::Duration;
use tokio::runtime::Runtime;

/// Create a test provider request for benchmarks
fn create_test_request() -> ProviderRequest {
    ProviderRequest {
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

/// Benchmark proto serialization overhead
fn bench_proto_serialization(c: &mut Criterion) {
    let request = create_test_request();

    let mut group = c.benchmark_group("proto_serialization");
    group.throughput(Throughput::Bytes(request.encoded_len() as u64));

    group.bench_function("serialize_request", |b| {
        b.iter(|| {
            let _ = black_box(&request).encode_to_vec();
        })
    });

    group.finish();
}

/// Benchmark proto deserialization overhead
fn bench_proto_deserialization(c: &mut Criterion) {
    let request = create_test_request();
    let bytes = request.encode_to_vec();

    let mut group = c.benchmark_group("proto_deserialization");
    group.throughput(Throughput::Bytes(bytes.len() as u64));

    group.bench_function("deserialize_request", |b| {
        b.iter(|| {
            let _: ProviderRequest = ProviderRequest::decode(black_box(&bytes[..])).unwrap();
        })
    });

    group.finish();
}

/// Benchmark circuit breaker check overhead
fn bench_circuit_breaker(c: &mut Criterion) {
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // Simple circuit breaker simulation
    struct SimpleCircuitBreaker {
        state: Arc<Mutex<bool>>, // true = closed (allow requests)
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

    let cb = SimpleCircuitBreaker::new();

    let mut group = c.benchmark_group("circuit_breaker");

    group.bench_function("check_circuit_closed", |b| {
        b.to_async(Runtime::new().unwrap()).iter(|| async {
            let _ = black_box(&cb).allow_request().await;
        })
    });

    group.finish();
}

/// Benchmark API client creation overhead
fn bench_client_creation(c: &mut Criterion) {
    let config = ClientConfig::default();

    let mut group = c.benchmark_group("client_creation");

    group.bench_function("create_api_client", |b| {
        b.iter(|| {
            let _ = ApiClient::new(black_box(config.clone()));
        })
    });

    group.finish();
}

/// Benchmark connection pool acquisition
fn bench_pool_acquisition(c: &mut Criterion) {
    let mut group = c.benchmark_group("pool_acquisition");

    group.bench_function("get_client_from_pool", |b| {
        b.iter(|| {
            let _pool = ApiClientPool::global();
        })
    });

    group.finish();
}

/// Benchmark full API round-trip (mocked)
/// Note: This requires a running API server
fn bench_api_roundtrip(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = ClientConfig {
        endpoint: "localhost:3030".to_string(),
        use_tls: false,
        connect_timeout: Duration::from_secs(5),
        timeout: Duration::from_secs(30),
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    let client = match ApiClient::new(config) {
        Ok(c) => c,
        Err(_) => {
            eprintln!(
                "Warning: Could not create API client for benchmark (server may not be running)"
            );
            return;
        }
    };

    let request = create_test_request();

    let mut group = c.benchmark_group("api_roundtrip");
    group.measurement_time(Duration::from_secs(60)); // Longer measurement for network calls
    group.sample_size(50); // Fewer samples for network calls

    group.bench_function("execute_request", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = client.execute(black_box(request.clone())).await;
        })
    });

    group.finish();
}

/// Benchmark streaming API call
fn bench_api_streaming(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = ClientConfig {
        endpoint: "localhost:3030".to_string(),
        use_tls: false,
        connect_timeout: Duration::from_secs(5),
        timeout: Duration::from_secs(30),
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    let client = match ApiClient::new(config) {
        Ok(c) => c,
        Err(_) => {
            eprintln!(
                "Warning: Could not create API client for benchmark (server may not be running)"
            );
            return;
        }
    };

    let mut request = create_test_request();
    request.stream = true;

    let mut group = c.benchmark_group("api_streaming");
    group.measurement_time(Duration::from_secs(60));
    group.sample_size(50);

    group.bench_function("execute_stream", |b| {
        b.to_async(&rt).iter(|| async {
            let stream = client.execute_stream(black_box(request.clone())).await;
            if let Ok(mut s) = stream {
                // Consume stream
                use tokio_stream::StreamExt;
                while let Some(_) = s.next().await {
                    // Process chunk
                }
            }
        })
    });

    group.finish();
}

/// Benchmark embedding API call
fn bench_embedding_api(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = ClientConfig {
        endpoint: "localhost:3030".to_string(),
        use_tls: false,
        connect_timeout: Duration::from_secs(5),
        timeout: Duration::from_secs(30),
        migration_mode: false,
        feature_flags: Default::default(),
        execution_strategy: Default::default(),
    };

    let client = match ApiClient::new(config) {
        Ok(c) => c,
        Err(_) => {
            eprintln!(
                "Warning: Could not create API client for benchmark (server may not be running)"
            );
            return;
        }
    };

    let text = "This is a test text for embedding benchmark. ".repeat(5);

    let mut group = c.benchmark_group("embedding_api");
    group.measurement_time(Duration::from_secs(60));
    group.sample_size(50);

    group.bench_function("embed_text", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = client
                .embed(
                    black_box(text.clone()),
                    black_box(Some("text-embedding-3-small".to_string())),
                    black_box(openakta_api_client::ExecutionStrategy::HostedOnly),
                )
                .await;
        })
    });

    group.finish();
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
        bench_proto_serialization,
        bench_proto_deserialization,
        bench_circuit_breaker,
        bench_client_creation,
        bench_pool_acquisition,
        bench_api_roundtrip,
        bench_api_streaming,
        bench_embedding_api,
);

criterion_main!(benches);
