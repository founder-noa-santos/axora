//! Proto Conversion Benchmarks
//!
//! Measures the overhead of converting between internal types and proto types:
//! - ModelRequest → ProviderRequest
//! - ProviderResponse → ModelResponse
//! - Round-trip conversion fidelity

mod support;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use openakta_proto::provider_v1 as proto;
use prost::Message;

use support::{
    build_model_response, build_provider_request, create_internal_request, create_proto_response,
};

/// Benchmark ModelRequest → ProviderRequest conversion
fn bench_internal_to_proto(c: &mut Criterion) {
    let internal_request = create_internal_request();
    let request_id = "benchmark-request";
    let tenant_id = "benchmark-tenant";

    let mut group = c.benchmark_group("internal_to_proto");
    group.throughput(Throughput::Elements(1));

    group.bench_function("convert_model_request", |b| {
        b.iter(|| {
            let _ = build_provider_request(
                black_box(request_id),
                black_box(tenant_id),
                black_box(&internal_request),
            );
        })
    });

    group.finish();
}

/// Benchmark ProviderResponse → ModelResponse conversion
fn bench_proto_to_internal(c: &mut Criterion) {
    let proto_response = create_proto_response();

    let mut group = c.benchmark_group("proto_to_internal");
    group.throughput(Throughput::Elements(1));

    group.bench_function("convert_provider_response", |b| {
        b.iter(|| {
            let _ = build_model_response(black_box(&proto_response));
        })
    });

    group.finish();
}

/// Benchmark round-trip conversion (internal → proto → internal)
fn bench_roundtrip_conversion(c: &mut Criterion) {
    let internal_request = create_internal_request();
    let request_id = "benchmark-request";
    let tenant_id = "benchmark-tenant";

    let mut group = c.benchmark_group("roundtrip_conversion");
    group.throughput(Throughput::Elements(1));

    group.bench_function("full_roundtrip", |b| {
        b.iter(|| {
            let proto_request = build_provider_request(request_id, tenant_id, &internal_request);
            let mut proto_response = create_proto_response();
            proto_response.request_id = proto_request.request_id.clone();
            proto_response.model = proto_request.model.clone();
            let _ = build_model_response(&proto_response);
        })
    });

    group.finish();
}

/// Benchmark proto serialization size for different message sizes
fn bench_serialization_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization_size");

    for message_count in [1, 10, 50, 100] {
        let messages: Vec<proto::ChatMessage> = (0..message_count)
            .map(|_| proto::ChatMessage {
                role: "user".to_string(),
                content: Some("Test message content. ".repeat(10)),
                name: None,
                content_parts: vec![],
                tool_call: None,
                tool_call_id: None,
            })
            .collect();

        let request = proto::ProviderRequest {
            request_id: uuid::Uuid::new_v4().to_string(),
            tenant_id: "benchmark".to_string(),
            model: "gpt-4o-mini".to_string(),
            model_hint: None,
            system_prompt: "Benchmark serialization request".to_string(),
            messages,
            tools: vec![],
            tool_choice: proto::ToolChoice::Auto as i32,
            max_tokens: Some(100),
            temperature: Some(0.7),
            top_p: None,
            stop_sequences: vec![],
            frequency_penalty: None,
            presence_penalty: None,
            stream: false,
            provider_extensions: std::collections::HashMap::new(),
            required_capabilities: vec![],
            execution_strategy: proto::ExecutionStrategy::HostedOnly as i32,
        };

        group.bench_with_input(
            BenchmarkId::new("message_count", message_count),
            &request,
            |b, req| {
                b.iter(|| {
                    let bytes = black_box(req).encode_to_vec();
                    black_box(bytes.len())
                })
            },
        );
    }

    group.finish();
}

/// Benchmark field preservation during conversion
fn bench_field_preservation(c: &mut Criterion) {
    let internal_request = create_internal_request();
    let request_id = "benchmark-request";
    let tenant_id = "benchmark-tenant";

    let mut group = c.benchmark_group("field_preservation");

    group.bench_function("verify_all_fields", |b| {
        b.iter(|| {
            let proto_request = build_provider_request(request_id, tenant_id, &internal_request);

            assert_eq!(proto_request.request_id, request_id);
            assert_eq!(proto_request.tenant_id, tenant_id);
            assert_eq!(proto_request.model, internal_request.model);
            assert_eq!(
                proto_request.messages.len(),
                internal_request.recent_messages.len()
            );
            assert_eq!(
                proto_request.system_prompt,
                internal_request.system_instructions.join("\n")
            );
            assert_eq!(
                proto_request.max_tokens,
                Some(internal_request.max_output_tokens)
            );
            assert_eq!(proto_request.temperature, internal_request.temperature);
            assert_eq!(proto_request.stream, internal_request.stream);
        })
    });

    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(std::time::Duration::from_secs(30))
        .warm_up_time(std::time::Duration::from_secs(5))
        .noise_threshold(0.05)
        .confidence_level(0.95)
        .nresamples(100_000);
    targets =
        bench_internal_to_proto,
        bench_proto_to_internal,
        bench_roundtrip_conversion,
        bench_serialization_size,
        bench_field_preservation,
);

criterion_main!(benches);
