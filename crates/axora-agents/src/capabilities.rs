//! Agent capabilities for task matching

use serde::{Deserialize, Serialize};

/// Agent capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapabilities {
    /// Programming languages the agent knows
    pub languages: Vec<String>,
    /// Frameworks the agent knows
    pub frameworks: Vec<String>,
    /// Tools the agent can use
    pub tools: Vec<String>,
    /// Maximum concurrent tasks
    pub max_concurrent_tasks: usize,
    /// Specializations (e.g., "frontend", "backend", "database")
    pub specializations: Vec<String>,
}

impl AgentCapabilities {
    /// Create new capabilities
    pub fn new() -> Self {
        Self {
            languages: Vec::new(),
            frameworks: Vec::new(),
            tools: Vec::new(),
            max_concurrent_tasks: 3,
            specializations: Vec::new(),
        }
    }

    /// Add a language
    pub fn with_language(mut self, language: &str) -> Self {
        self.languages.push(language.to_string());
        self
    }

    /// Add a framework
    pub fn with_framework(mut self, framework: &str) -> Self {
        self.frameworks.push(framework.to_string());
        self
    }

    /// Add a tool
    pub fn with_tool(mut self, tool: &str) -> Self {
        self.tools.push(tool.to_string());
        self
    }

    /// Add a specialization
    pub fn with_specialization(mut self, spec: &str) -> Self {
        self.specializations.push(spec.to_string());
        self
    }

    /// Set max concurrent tasks
    pub fn with_max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent_tasks = max;
        self
    }

    /// Check if agent has a specific language
    pub fn has_language(&self, language: &str) -> bool {
        self.languages
            .iter()
            .any(|l| l.to_lowercase() == language.to_lowercase())
    }

    /// Check if agent has a specific framework
    pub fn has_framework(&self, framework: &str) -> bool {
        self.frameworks
            .iter()
            .any(|f| f.to_lowercase() == framework.to_lowercase())
    }

    /// Check if agent has a specific tool
    pub fn has_tool(&self, tool: &str) -> bool {
        self.tools
            .iter()
            .any(|t| t.to_lowercase() == tool.to_lowercase())
    }

    /// Check if agent has a specific specialization
    pub fn has_specialization(&self, spec: &str) -> bool {
        self.specializations
            .iter()
            .any(|s| s.to_lowercase() == spec.to_lowercase())
    }

    /// Calculate match score against requirements
    pub fn match_score(&self, requirements: &TaskRequirements) -> f32 {
        let mut score = 0.0;
        let mut max_score = 0.0;

        // Language match (weight: 3)
        if !requirements.required_languages.is_empty() {
            max_score += 3.0;
            for lang in &requirements.required_languages {
                if self.has_language(lang) {
                    score += 3.0 / requirements.required_languages.len() as f32;
                }
            }
        }

        // Framework match (weight: 2)
        if !requirements.required_frameworks.is_empty() {
            max_score += 2.0;
            for framework in &requirements.required_frameworks {
                if self.has_framework(framework) {
                    score += 2.0 / requirements.required_frameworks.len() as f32;
                }
            }
        }

        // Tool match (weight: 1)
        if !requirements.required_tools.is_empty() {
            max_score += 1.0;
            for tool in &requirements.required_tools {
                if self.has_tool(tool) {
                    score += 1.0 / requirements.required_tools.len() as f32;
                }
            }
        }

        // Specialization match (weight: 2)
        if !requirements.specializations.is_empty() {
            max_score += 2.0;
            for spec in &requirements.specializations {
                if self.has_specialization(spec) {
                    score += 2.0 / requirements.specializations.len() as f32;
                }
            }
        }

        if max_score == 0.0 {
            1.0 // No requirements = perfect match
        } else {
            score / max_score
        }
    }
}

impl Default for AgentCapabilities {
    fn default() -> Self {
        Self::new()
    }
}

/// Task requirements for matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequirements {
    /// Required programming languages
    pub required_languages: Vec<String>,
    /// Required frameworks
    pub required_frameworks: Vec<String>,
    /// Required tools
    pub required_tools: Vec<String>,
    /// Required specializations
    pub specializations: Vec<String>,
    /// Minimum match score (0.0 - 1.0)
    pub min_score: f32,
}

impl TaskRequirements {
    /// Create new requirements
    pub fn new() -> Self {
        Self {
            required_languages: Vec::new(),
            required_frameworks: Vec::new(),
            required_tools: Vec::new(),
            specializations: Vec::new(),
            min_score: 0.5,
        }
    }

    /// Add required language
    pub fn with_language(mut self, language: &str) -> Self {
        self.required_languages.push(language.to_string());
        self
    }

    /// Add required framework
    pub fn with_framework(mut self, framework: &str) -> Self {
        self.required_frameworks.push(framework.to_string());
        self
    }

    /// Add required tool
    pub fn with_tool(mut self, tool: &str) -> Self {
        self.required_tools.push(tool.to_string());
        self
    }

    /// Add required specialization
    pub fn with_specialization(mut self, spec: &str) -> Self {
        self.specializations.push(spec.to_string());
        self
    }

    /// Set minimum score
    pub fn with_min_score(mut self, score: f32) -> Self {
        self.min_score = score;
        self
    }
}

impl Default for TaskRequirements {
    fn default() -> Self {
        Self::new()
    }
}

/// Capability registry for all agents
pub struct CapabilityRegistry {
    /// Agent capabilities indexed by agent ID
    capabilities: std::collections::HashMap<String, AgentCapabilities>,
}

impl CapabilityRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            capabilities: std::collections::HashMap::new(),
        }
    }

    /// Register agent capabilities
    pub fn register(&mut self, agent_id: &str, capabilities: AgentCapabilities) {
        self.capabilities.insert(agent_id.to_string(), capabilities);
    }

    /// Get agent capabilities
    pub fn get(&self, agent_id: &str) -> Option<&AgentCapabilities> {
        self.capabilities.get(agent_id)
    }

    /// Find best agent for task
    pub fn find_best_agent(&self, requirements: &TaskRequirements) -> Option<String> {
        let mut best_agent: Option<String> = None;
        let mut best_score = requirements.min_score;

        for (agent_id, capabilities) in &self.capabilities {
            let score = capabilities.match_score(requirements);
            if score > best_score {
                best_score = score;
                best_agent = Some(agent_id.clone());
            }
        }

        best_agent
    }

    /// Find all suitable agents for task
    pub fn find_suitable_agents(&self, requirements: &TaskRequirements) -> Vec<String> {
        self.capabilities
            .iter()
            .filter(|(_, caps)| caps.match_score(requirements) >= requirements.min_score)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get number of registered agents
    pub fn agent_count(&self) -> usize {
        self.capabilities.len()
    }
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capabilities_creation() {
        let caps = AgentCapabilities::new()
            .with_language("Rust")
            .with_language("Python")
            .with_framework("Tokio")
            .with_tool("Git")
            .with_specialization("backend");

        assert!(caps.has_language("rust"));
        assert!(caps.has_language("python"));
        assert!(caps.has_framework("tokio"));
        assert!(caps.has_tool("git"));
        assert!(caps.has_specialization("backend"));
    }

    #[test]
    fn test_match_score_perfect() {
        let caps = AgentCapabilities::new()
            .with_language("Rust")
            .with_framework("Tokio");

        let requirements = TaskRequirements::new()
            .with_language("Rust")
            .with_framework("Tokio");

        let score = caps.match_score(&requirements);
        assert!((score - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_match_score_partial() {
        let caps = AgentCapabilities::new()
            .with_language("Rust")
            .with_language("Python");

        let requirements = TaskRequirements::new()
            .with_language("Rust")
            .with_language("JavaScript");

        let score = caps.match_score(&requirements);
        // Should match Rust but not JavaScript
        assert!(score > 0.4 && score < 0.6);
    }

    #[test]
    fn test_capability_registry() {
        let mut registry = CapabilityRegistry::new();

        let caps1 = AgentCapabilities::new().with_language("Rust");
        let caps2 = AgentCapabilities::new().with_language("Python");

        registry.register("agent1", caps1);
        registry.register("agent2", caps2);

        assert_eq!(registry.agent_count(), 2);

        let requirements = TaskRequirements::new().with_language("Rust");
        let best = registry.find_best_agent(&requirements);

        assert_eq!(best, Some("agent1".to_string()));
    }

    #[test]
    fn test_find_suitable_agents() {
        let mut registry = CapabilityRegistry::new();

        registry.register("agent1", AgentCapabilities::new().with_language("Rust"));
        registry.register("agent2", AgentCapabilities::new().with_language("Python"));
        registry.register("agent3", AgentCapabilities::new().with_language("Rust"));

        let requirements = TaskRequirements::new().with_language("Rust");
        let suitable = registry.find_suitable_agents(&requirements);

        assert_eq!(suitable.len(), 2);
        assert!(suitable.contains(&"agent1".to_string()));
        assert!(suitable.contains(&"agent3".to_string()));
    }
}
