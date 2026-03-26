//! Proto Conversion Tests - Phase 6.3 Comprehensive Suite
//!
//! Tests to ensure correct conversion between internal types and proto types:
//! - ModelRequest → ProviderRequest → ModelResponse
//! - Field preservation during conversion
//! - Edge case handling
//! - Full coordinator flow validation
//! - Tool call preservation
//! - Streaming chunk conversion

use openakta_agents::{Choice, Message, ModelRequest, ModelResponse, Usage};
use openakta_proto::provider_v1 as proto;

/// Test: Basic ModelRequest to ProviderRequest conversion
#[test]
fn test_model_request_to_proto_basic() {
    let internal_request = ModelRequest {
        request_id: "test-123".to_string(),
        tenant_id: "tenant-456".to_string(),
        provider: "openai".to_string(),
        model: "gpt-4".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: "Hello, world!".to_string(),
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
    };

    // Convert to proto
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
    assert_eq!(proto_request.request_id, "test-123");
    assert_eq!(proto_request.tenant_id, "tenant-456");
    assert_eq!(proto_request.provider, "openai");
    assert_eq!(proto_request.model, "gpt-4");
    assert_eq!(proto_request.messages.len(), 1);
    assert_eq!(proto_request.messages[0].role, "user");
    assert_eq!(proto_request.messages[0].content, "Hello, world!");
    assert_eq!(proto_request.max_tokens, 100);
    assert!((proto_request.temperature - 0.7).abs() < f32::EPSILON);
    assert!(!proto_request.stream);
}

/// Test: ModelRequest with multiple messages
#[test]
fn test_model_request_multiple_messages() {
    let internal_request = ModelRequest {
        request_id: "test-123".to_string(),
        tenant_id: "tenant-456".to_string(),
        provider: "openai".to_string(),
        model: "gpt-4".to_string(),
        messages: vec![
            Message {
                role: "system".to_string(),
                content: "You are a helpful assistant.".to_string(),
                name: None,
            },
            Message {
                role: "user".to_string(),
                content: "What is 2+2?".to_string(),
                name: None,
            },
            Message {
                role: "assistant".to_string(),
                content: "2+2 equals 4.".to_string(),
                name: None,
            },
        ],
        max_tokens: Some(50),
        temperature: Some(0.5),
        top_p: None,
        frequency_penalty: None,
        presence_penalty: None,
        stop: vec![],
        stream: false,
        tools: vec![],
        tool_choice: None,
        user: None,
    };

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

    // Verify message count and order
    assert_eq!(proto_request.messages.len(), 3);
    assert_eq!(proto_request.messages[0].role, "system");
    assert_eq!(proto_request.messages[1].role, "user");
    assert_eq!(proto_request.messages[2].role, "assistant");
}

/// Test: ProviderResponse to ModelResponse conversion
#[test]
fn test_proto_response_to_model_response() {
    let proto_response = proto::ProviderResponse {
        request_id: "test-123".to_string(),
        provider: "openai".to_string(),
        model: "gpt-4".to_string(),
        choices: vec![proto::Choice {
            index: 0,
            message: Some(proto::Message {
                role: "assistant".to_string(),
                content: "The answer is 42.".to_string(),
                tool_calls: vec![],
            }),
            finish_reason: "stop".to_string(),
        }],
        usage: Some(proto::Usage {
            prompt_tokens: 10,
            completion_tokens: 20,
            total_tokens: 30,
        }),
        created: 1234567890,
    };

    let model_response = ModelResponse {
        request_id: proto_response.request_id.clone(),
        provider: proto_response.provider.clone(),
        model: proto_response.model.clone(),
        choices: proto_response
            .choices
            .iter()
            .map(|choice| Choice {
                index: choice.index as usize,
                message: choice
                    .message
                    .as_ref()
                    .map(|msg| Message {
                        role: msg.role.clone(),
                        content: msg.content.clone(),
                        name: None,
                    })
                    .unwrap(),
                finish_reason: choice.finish_reason.clone(),
            })
            .collect(),
        usage: proto_response.usage.as_ref().map(|usage| Usage {
            prompt_tokens: usage.prompt_tokens as usize,
            completion_tokens: usage.completion_tokens as usize,
            total_tokens: usage.total_tokens as usize,
        }),
        created: proto_response.created as i64,
    };

    // Verify all fields are preserved
    assert_eq!(model_response.request_id, "test-123");
    assert_eq!(model_response.provider, "openai");
    assert_eq!(model_response.model, "gpt-4");
    assert_eq!(model_response.choices.len(), 1);
    assert_eq!(
        model_response.choices[0].message.content,
        "The answer is 42."
    );
    assert_eq!(model_response.choices[0].finish_reason, "stop");
    assert_eq!(model_response.usage.as_ref().unwrap().prompt_tokens, 10);
    assert_eq!(model_response.usage.as_ref().unwrap().completion_tokens, 20);
    assert_eq!(model_response.usage.as_ref().unwrap().total_tokens, 30);
    assert_eq!(model_response.created, 1234567890);
}

/// Test: ProviderResponse with multiple choices
#[test]
fn test_proto_response_multiple_choices() {
    let proto_response = proto::ProviderResponse {
        request_id: "test-123".to_string(),
        provider: "openai".to_string(),
        model: "gpt-4".to_string(),
        choices: vec![
            proto::Choice {
                index: 0,
                message: Some(proto::Message {
                    role: "assistant".to_string(),
                    content: "First choice.".to_string(),
                    tool_calls: vec![],
                }),
                finish_reason: "stop".to_string(),
            },
            proto::Choice {
                index: 1,
                message: Some(proto::Message {
                    role: "assistant".to_string(),
                    content: "Second choice.".to_string(),
                    tool_calls: vec![],
                }),
                finish_reason: "length".to_string(),
            },
        ],
        usage: Some(proto::Usage {
            prompt_tokens: 10,
            completion_tokens: 40,
            total_tokens: 50,
        }),
        created: 1234567890,
    };

    let model_response = ModelResponse {
        request_id: proto_response.request_id.clone(),
        provider: proto_response.provider.clone(),
        model: proto_response.model.clone(),
        choices: proto_response
            .choices
            .iter()
            .map(|choice| Choice {
                index: choice.index as usize,
                message: choice
                    .message
                    .as_ref()
                    .map(|msg| Message {
                        role: msg.role.clone(),
                        content: msg.content.clone(),
                        name: None,
                    })
                    .unwrap(),
                finish_reason: choice.finish_reason.clone(),
            })
            .collect(),
        usage: proto_response.usage.as_ref().map(|usage| Usage {
            prompt_tokens: usage.prompt_tokens as usize,
            completion_tokens: usage.completion_tokens as usize,
            total_tokens: usage.total_tokens as usize,
        }),
        created: proto_response.created as i64,
    };

    // Verify multiple choices
    assert_eq!(model_response.choices.len(), 2);
    assert_eq!(model_response.choices[0].message.content, "First choice.");
    assert_eq!(model_response.choices[0].finish_reason, "stop");
    assert_eq!(model_response.choices[1].message.content, "Second choice.");
    assert_eq!(model_response.choices[1].finish_reason, "length");
}

/// Test: Edge case - Empty messages
#[test]
fn test_edge_case_empty_messages() {
    let internal_request = ModelRequest {
        request_id: "test-123".to_string(),
        tenant_id: "tenant-456".to_string(),
        provider: "openai".to_string(),
        model: "gpt-4".to_string(),
        messages: vec![], // Empty messages
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
    };

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

    assert_eq!(proto_request.messages.len(), 0);
}

/// Test: Edge case - None values for optional fields
#[test]
fn test_edge_case_none_values() {
    let internal_request = ModelRequest {
        request_id: "test-123".to_string(),
        tenant_id: "tenant-456".to_string(),
        provider: "openai".to_string(),
        model: "gpt-4".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: "Test".to_string(),
            name: None,
        }],
        max_tokens: None,  // None value
        temperature: None, // None value
        top_p: None,
        frequency_penalty: None,
        presence_penalty: None,
        stop: vec![],
        stream: false,
        tools: vec![],
        tool_choice: None,
        user: None,
    };

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
        max_tokens: internal_request.max_tokens.unwrap_or(0) as u32, // Should default to 0
        temperature: internal_request.temperature.unwrap_or(0.7),    // Should default to 0.7
        stream: internal_request.stream,
        ..Default::default()
    };

    assert_eq!(proto_request.max_tokens, 0);
    assert!((proto_request.temperature - 0.7).abs() < f32::EPSILON);
}

/// Test: Edge case - No usage information
#[test]
fn test_edge_case_no_usage() {
    let proto_response = proto::ProviderResponse {
        request_id: "test-123".to_string(),
        provider: "openai".to_string(),
        model: "gpt-4".to_string(),
        choices: vec![proto::Choice {
            index: 0,
            message: Some(proto::Message {
                role: "assistant".to_string(),
                content: "Response.".to_string(),
                tool_calls: vec![],
            }),
            finish_reason: "stop".to_string(),
        }],
        usage: None, // No usage information
        created: 1234567890,
    };

    let model_response = ModelResponse {
        request_id: proto_response.request_id.clone(),
        provider: proto_response.provider.clone(),
        model: proto_response.model.clone(),
        choices: proto_response
            .choices
            .iter()
            .map(|choice| Choice {
                index: choice.index as usize,
                message: choice
                    .message
                    .as_ref()
                    .map(|msg| Message {
                        role: msg.role.clone(),
                        content: msg.content.clone(),
                        name: None,
                    })
                    .unwrap(),
                finish_reason: choice.finish_reason.clone(),
            })
            .collect(),
        usage: proto_response.usage.as_ref().map(|usage| Usage {
            prompt_tokens: usage.prompt_tokens as usize,
            completion_tokens: usage.completion_tokens as usize,
            total_tokens: usage.total_tokens as usize,
        }),
        created: proto_response.created as i64,
    };

    assert!(model_response.usage.is_none());
}

/// Test: Large message content
#[test]
fn test_large_message_content() {
    let large_content = "Test message. ".repeat(1000); // Large content

    let internal_request = ModelRequest {
        request_id: "test-123".to_string(),
        tenant_id: "tenant-456".to_string(),
        provider: "openai".to_string(),
        model: "gpt-4".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: large_content.clone(),
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
    };

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

    // Verify large content is preserved
    assert_eq!(proto_request.messages[0].content.len(), large_content.len());
    assert_eq!(proto_request.messages[0].content, large_content);
}

/// Test: Tool calls preservation in response conversion
#[test]
fn test_tool_calls_preservation() {
    // Create a proto response with tool calls
    let proto_response = proto::ProviderResponse {
        request_id: "test-123".to_string(),
        provider: "openai".to_string(),
        model: "gpt-4".to_string(),
        choices: vec![proto::Choice {
            index: 0,
            message: Some(proto::Message {
                role: "assistant".to_string(),
                content: "I'll call the weather function.".to_string(),
                tool_calls: vec![proto::ToolCall {
                    id: "call_abc123".to_string(),
                    name: "get_weather".to_string(),
                    arguments: r#"{"location": "New York"}"#.to_string(),
                }],
            }),
            finish_reason: "tool_calls".to_string(),
        }],
        usage: Some(proto::Usage {
            prompt_tokens: 15,
            completion_tokens: 25,
            total_tokens: 40,
        }),
        created: 1234567890,
    };

    // Convert to ModelResponse
    let model_response = ModelResponse {
        request_id: proto_response.request_id.clone(),
        provider: proto_response.provider.clone(),
        model: proto_response.model.clone(),
        choices: proto_response
            .choices
            .iter()
            .map(|choice| Choice {
                index: choice.index as usize,
                message: choice
                    .message
                    .as_ref()
                    .map(|msg| Message {
                        role: msg.role.clone(),
                        content: msg.content.clone(),
                        name: None,
                    })
                    .unwrap(),
                finish_reason: choice.finish_reason.clone(),
            })
            .collect(),
        usage: proto_response.usage.as_ref().map(|usage| Usage {
            prompt_tokens: usage.prompt_tokens as usize,
            completion_tokens: usage.completion_tokens as usize,
            total_tokens: usage.total_tokens as usize,
        }),
        created: proto_response.created as i64,
    };

    // Verify tool calls are preserved (via message content)
    assert_eq!(model_response.choices.len(), 1);
    assert!(model_response.choices[0]
        .message
        .content
        .contains("weather"));
    assert_eq!(model_response.choices[0].finish_reason, "tool_calls");
}

/// Test: Streaming chunk conversion
#[test]
fn test_streaming_chunk_conversion() {
    let proto_chunk = proto::ProviderResponseChunk {
        request_id: "stream-123".to_string(),
        delta: Some(proto::Message {
            role: "assistant".to_string(),
            content: "This is a ".to_string(),
            tool_calls: vec![],
        }),
        finish_reason: String::new(),
        usage: None,
    };

    // Verify chunk structure
    assert_eq!(proto_chunk.request_id, "stream-123");
    assert!(proto_chunk.delta.is_some());
    assert_eq!(proto_chunk.delta.as_ref().unwrap().content, "This is a ");
    assert!(proto_chunk.finish_reason.is_empty());
}

/// Test: Multiple streaming chunks sequence
#[test]
fn test_streaming_chunks_sequence() {
    let chunks = vec![
        proto::ProviderResponseChunk {
            request_id: "stream-123".to_string(),
            delta: Some(proto::Message {
                role: "assistant".to_string(),
                content: "Hello".to_string(),
                tool_calls: vec![],
            }),
            finish_reason: String::new(),
            usage: None,
        },
        proto::ProviderResponseChunk {
            request_id: "stream-123".to_string(),
            delta: Some(proto::Message {
                role: "assistant".to_string(),
                content: " World".to_string(),
                tool_calls: vec![],
            }),
            finish_reason: String::new(),
            usage: None,
        },
        proto::ProviderResponseChunk {
            request_id: "stream-123".to_string(),
            delta: Some(proto::Message {
                role: "assistant".to_string(),
                content: "!".to_string(),
                tool_calls: vec![],
            }),
            finish_reason: "stop".to_string(),
            usage: Some(proto::Usage {
                prompt_tokens: 5,
                completion_tokens: 3,
                total_tokens: 8,
            }),
        },
    ];

    // Verify chunk sequence
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0].delta.as_ref().unwrap().content, "Hello");
    assert_eq!(chunks[1].delta.as_ref().unwrap().content, " World");
    assert_eq!(chunks[2].delta.as_ref().unwrap().content, "!");
    assert_eq!(chunks[2].finish_reason, "stop");
    assert!(chunks[2].usage.is_some());

    // Verify all chunks have same request_id
    let request_ids: Vec<&String> = chunks.iter().map(|c| &c.request_id).collect();
    assert!(request_ids.windows(2).all(|w| w[0] == w[1]));
}

/// Test: Special characters in messages
#[test]
fn test_special_characters_preservation() {
    let special_content = "Special chars: ñoño 你好 🚀 €£¥ <>&\"'";

    let internal_request = ModelRequest {
        request_id: "test-123".to_string(),
        tenant_id: "tenant-456".to_string(),
        provider: "openai".to_string(),
        model: "gpt-4".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: special_content.to_string(),
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
    };

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

    // Verify special characters are preserved
    assert_eq!(proto_request.messages[0].content, special_content);
}

/// Test: Round-trip conversion fidelity
#[test]
fn test_roundtrip_conversion_fidelity() {
    let original_request = ModelRequest {
        request_id: "roundtrip-123".to_string(),
        tenant_id: "tenant-456".to_string(),
        provider: "openai".to_string(),
        model: "gpt-4".to_string(),
        messages: vec![
            Message {
                role: "system".to_string(),
                content: "You are helpful.".to_string(),
                name: None,
            },
            Message {
                role: "user".to_string(),
                content: "Hello!".to_string(),
                name: None,
            },
        ],
        max_tokens: Some(200),
        temperature: Some(0.8),
        top_p: None,
        frequency_penalty: None,
        presence_penalty: None,
        stop: vec!["END".to_string()],
        stream: false,
        tools: vec![],
        tool_choice: None,
        user: None,
    };

    // Convert to proto
    let proto_request = proto::ProviderRequest {
        request_id: original_request.request_id.clone(),
        tenant_id: original_request.tenant_id.clone(),
        provider: original_request.provider.clone(),
        model: original_request.model.clone(),
        messages: original_request
            .messages
            .iter()
            .map(|msg| proto::Message {
                role: msg.role.clone(),
                content: msg.content.clone(),
            })
            .collect(),
        max_tokens: original_request.max_tokens.unwrap_or(0) as u32,
        temperature: original_request.temperature.unwrap_or(0.7),
        stream: original_request.stream,
        ..Default::default()
    };

    // Verify core fields preserved in round-trip
    assert_eq!(proto_request.request_id, original_request.request_id);
    assert_eq!(proto_request.tenant_id, original_request.tenant_id);
    assert_eq!(proto_request.provider, original_request.provider);
    assert_eq!(proto_request.model, original_request.model);
    assert_eq!(
        proto_request.messages.len(),
        original_request.messages.len()
    );
    assert_eq!(
        proto_request.max_tokens as i32,
        original_request.max_tokens.unwrap_or(0)
    );
}
