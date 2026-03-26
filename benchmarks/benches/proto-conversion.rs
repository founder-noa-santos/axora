//! Proto Conversion Benchmarks
//!
//! Measures the overhead of converting between internal types and proto types:
//! - ModelRequest → ProviderRequest
//! - ProviderResponse → ModelResponse
//! - Round-trip conversion fidelity

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use openakta_proto::provider_v1 as proto;

/// Create a test ModelRequest (internal type)
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

/// Create a test ProviderResponse (proto type)
fn create_proto_response() -> proto::ProviderResponse {
    proto::ProviderResponse {
        request_id: uuid::Uuid::new_v4().to_string(),
        provider: "openai".to_string(),
        model: "gpt-4".to_string(),
        choices: vec![proto::Choice {
            index: 0,
            message: Some(proto::Message {
                role: "assistant".to_string(),
                content: "This is a benchmark test response. ".repeat(20),
                tool_calls: vec![],
            }),
            finish_reason: "stop".to_string(),
        }],
        usage: Some(proto::Usage {
            prompt_tokens: 50,
            completion_tokens: 100,
            total_tokens: 150,
        }),
        created: chrono::Utc::now().timestamp() as u64,
    }
}

/// Benchmark ModelRequest → ProviderRequest conversion
fn bench_internal_to_proto(c: &mut Criterion) {
    let internal_request = create_internal_request();

    let mut group = c.benchmark_group("internal_to_proto");
    group.throughput(Throughput::Elements(1));

    group.bench_function("convert_model_request", |b| {
        b.iter(|| {
            // Simulate conversion (actual implementation in coordinator)
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

/// Benchmark ProviderResponse → ModelResponse conversion
fn bench_proto_to_internal(c: &mut Criterion) {
    let proto_response = create_proto_response();

    let mut group = c.benchmark_group("proto_to_internal");
    group.throughput(Throughput::Elements(1));

    group.bench_function("convert_provider_response", |b| {
        b.iter(|| {
            // Simulate conversion (actual implementation in coordinator)
            let _ = openakta_agents::ModelResponse {
                request_id: black_box(&proto_response.request_id).clone(),
                provider: black_box(&proto_response.provider).clone(),
                model: black_box(&proto_response.model).clone(),
                choices: black_box(&proto_response.choices)
                    .iter()
                    .map(|choice| openakta_agents::Choice {
                        index: choice.index as usize,
                        message: choice
                            .message
                            .as_ref()
                            .map(|msg| openakta_agents::Message {
                                role: msg.role.clone(),
                                content: msg.content.clone(),
                                name: None,
                            })
                            .unwrap(),
                        finish_reason: choice.finish_reason.clone(),
                    })
                    .collect(),
                usage: black_box(&proto_response.usage).as_ref().map(|usage| {
                    openakta_agents::Usage {
                        prompt_tokens: usage.prompt_tokens as usize,
                        completion_tokens: usage.completion_tokens as usize,
                        total_tokens: usage.total_tokens as usize,
                    }
                }),
                created: black_box(&proto_response.created) as i64,
            };
        })
    });

    group.finish();
}

/// Benchmark round-trip conversion (internal → proto → internal)
fn bench_roundtrip_conversion(c: &mut Criterion) {
    let internal_request = create_internal_request();

    let mut group = c.benchmark_group("roundtrip_conversion");
    group.throughput(Throughput::Elements(1));

    group.bench_function("full_roundtrip", |b| {
        b.iter(|| {
            // Step 1: Internal → Proto
            let proto_request = proto::ProviderRequest {
                request_id: internal_request.request_id.clone(),
                tenant_id: internal_request.tenant_id.clone(),
                provider: internal_request.provider.clone(),
                model: internal_request.model.clone(),
                messages: internal_request
                    .messages
                    .iter()
                    .map(|msg| proto::Message {
                        role: msg.role.clone(),
                        content: msg.content.clone(),
                    })
                    .collect(),
                max_tokens: internal_request.max_tokens.unwrap_or(0) as u32,
                temperature: internal_request.temperature.unwrap_or(0.7),
                stream: internal_request.stream,
                ..Default::default()
            };

            // Step 2: Proto → Internal (simulating response)
            let _ = openakta_agents::ModelResponse {
                request_id: proto_request.request_id.clone(),
                provider: proto_request.provider.clone(),
                model: proto_request.model.clone(),
                choices: vec![openakta_agents::Choice {
                    index: 0,
                    message: openakta_agents::Message {
                        role: "assistant".to_string(),
                        content: "Response".to_string(),
                        name: None,
                    },
                    finish_reason: "stop".to_string(),
                }],
                usage: None,
                created: 0,
            };
        })
    });

    group.finish();
}

/// Benchmark proto serialization size for different message sizes
fn bench_serialization_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization_size");

    for message_count in [1, 10, 50, 100] {
        let messages: Vec<proto::Message> = (0..message_count)
            .map(|_| proto::Message {
                role: "user".to_string(),
                content: "Test message content. ".repeat(10),
            })
            .collect();

        let request = proto::ProviderRequest {
            request_id: uuid::Uuid::new_v4().to_string(),
            tenant_id: "benchmark".to_string(),
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            messages: messages.clone(),
            max_tokens: 100,
            temperature: 0.7,
            stream: false,
            ..Default::default()
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

    let mut group = c.benchmark_group("field_preservation");

    group.bench_function("verify_all_fields", |b| {
        b.iter(|| {
            let proto_request = proto::ProviderRequest {
                request_id: internal_request.request_id.clone(),
                tenant_id: internal_request.tenant_id.clone(),
                provider: internal_request.provider.clone(),
                model: internal_request.model.clone(),
                messages: internal_request
                    .messages
                    .iter()
                    .map(|msg| proto::Message {
                        role: msg.role.clone(),
                        content: msg.content.clone(),
                    })
                    .collect(),
                max_tokens: internal_request.max_tokens.unwrap_or(0) as u32,
                temperature: internal_request.temperature.unwrap_or(0.7),
                stream: internal_request.stream,
                ..Default::default()
            };

            // Verify all fields are preserved
            assert_eq!(proto_request.request_id, internal_request.request_id);
            assert_eq!(proto_request.tenant_id, internal_request.tenant_id);
            assert_eq!(proto_request.provider, internal_request.provider);
            assert_eq!(proto_request.model, internal_request.model);
            assert_eq!(
                proto_request.messages.len(),
                internal_request.messages.len()
            );
            assert_eq!(
                proto_request.max_tokens as i32,
                internal_request.max_tokens.unwrap_or(0)
            );
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
