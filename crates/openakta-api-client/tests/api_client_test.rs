//! API client tests

#[cfg(test)]
mod tests {
    use openakta_api_client::{ApiClient, ClientConfig, ExecutionStrategy, FeatureFlags};
    use openakta_proto::provider_v1::{ChatMessage, ProviderRequest};
    use prost::Message;

    #[test]
    fn test_client_config_default() {
        let config = ClientConfig::default();
        assert_eq!(config.endpoint, "http://localhost:3030");
        assert_eq!(
            config.execution_strategy,
            ExecutionStrategy::LocalWithFallback
        );
        assert!(config.migration_mode);
    }

    #[test]
    fn test_feature_flags_default() {
        let flags = FeatureFlags::default();
        assert!(flags.api_enabled);
        assert!(!flags.hosted_completion_enabled);
        assert!(!flags.hosted_search_enabled);
        assert_eq!(flags.canary_percentage, 0);
    }

    #[test]
    fn test_feature_flags_canary() {
        let mut flags = FeatureFlags::default();
        flags.canary_percentage = 50;

        // Test that some tenants are in canary, some are not
        // We can't test the private is_in_canary method directly,
        // but we can test the public should_use_hosted_completion
        let mut in_canary = 0;
        for i in 0..100 {
            if flags.should_use_hosted_completion(&format!("tenant-{}", i)) {
                in_canary += 1;
            }
        }

        // With default flags (hosted_completion_enabled=false), none should be in canary
        assert_eq!(in_canary, 0);

        // Now enable hosted completion
        flags.hosted_completion_enabled = true;
        in_canary = 0;
        for i in 0..100 {
            if flags.should_use_hosted_completion(&format!("tenant-{}", i)) {
                in_canary += 1;
            }
        }

        // Should be roughly 50% (allow variance due to hashing)
        assert!(
            in_canary >= 40 && in_canary <= 60,
            "Canary percentage should be around 50%, got {}",
            in_canary
        );
    }

    #[test]
    fn test_execution_strategy() {
        assert!(ExecutionStrategy::LocalOnly.allows_local());
        assert!(!ExecutionStrategy::LocalOnly.allows_hosted());

        assert!(!ExecutionStrategy::HostedOnly.allows_local());
        assert!(ExecutionStrategy::HostedOnly.allows_hosted());

        assert!(ExecutionStrategy::LocalWithFallback.allows_local());
        assert!(ExecutionStrategy::LocalWithFallback.allows_hosted());
        assert!(ExecutionStrategy::LocalWithFallback.has_fallback());

        assert!(ExecutionStrategy::HostedWithFallback.allows_local());
        assert!(ExecutionStrategy::HostedWithFallback.allows_hosted());
        assert!(ExecutionStrategy::HostedWithFallback.has_fallback());
    }

    #[tokio::test]
    async fn test_client_creation() {
        let config = ClientConfig::default();
        let result = ApiClient::new(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_request_serialization() {
        let request = ProviderRequest {
            request_id: "test-123".to_string(),
            tenant_id: "tenant-456".to_string(),
            model: "gpt-4".to_string(),
            system_prompt: "You are a helpful assistant.".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some("Hello!".to_string()),
                name: None,
                content_parts: vec![],
                tool_call: None,
                tool_call_id: None,
            }],
            stream: false,
            ..Default::default()
        };

        // Serialize and deserialize
        let encoded = prost::Message::encode_to_vec(&request);
        let decoded = ProviderRequest::decode(&encoded[..]).unwrap();

        assert_eq!(request.request_id, decoded.request_id);
        assert_eq!(request.tenant_id, decoded.tenant_id);
        assert_eq!(request.model, decoded.model);
        assert_eq!(request.messages.len(), decoded.messages.len());
    }
}
