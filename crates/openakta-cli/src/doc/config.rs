use serde::{Deserialize, Serialize};

pub const DEFAULT_CACHE_TTL_HOURS: u64 = 24;
pub const DEFAULT_MAX_CHUNK_SIZE: usize = 1200;
pub const DEFAULT_CHANGELOG_PATTERN: &str = "{timestamp}_{doc_id}_{type}_{slug}.md";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AktaConfig {
    pub schema_version: String,
    pub project: ProjectMetadata,
    pub registry: RegistryConfig,
    pub linter: LinterConfig,
    pub changelog: ChangelogConfig,
}

impl AktaConfig {
    pub fn new(project_name: impl Into<String>) -> Self {
        Self {
            schema_version: "1".to_string(),
            project: ProjectMetadata::new(project_name),
            registry: RegistryConfig::default(),
            linter: LinterConfig::default(),
            changelog: ChangelogConfig::default(),
        }
    }
}

impl Default for AktaConfig {
    fn default() -> Self {
        Self::new("openakta-project")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectMetadata {
    pub name: String,
    pub slug: String,
    pub docs_version: String,
    pub initialized_by: String,
}

impl ProjectMetadata {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let slug = slugify(&name);

        Self {
            name,
            slug,
            docs_version: "1".to_string(),
            initialized_by: "openakta doc init".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryConfig {
    /// Deprecated: ignored. Templates ship inside the `openakta` binary (no remote registry).
    #[serde(default)]
    pub remote_base_url: String,
    /// Local snapshot filename under `cache_dir` (offline copy of the bundled manifest).
    pub manifest_path: String,
    pub cache_dir: String,
    pub cache_ttl_hours: u64,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            remote_base_url: String::new(),
            manifest_path: "manifest.json".to_string(),
            cache_dir: ".openakta/templates_cache".to_string(),
            cache_ttl_hours: DEFAULT_CACHE_TTL_HOURS,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LinterConfig {
    pub max_chunk_size: usize,
    pub required_frontmatter: bool,
    pub require_template_links: bool,
    pub enforce_readme_per_directory: bool,
}

impl Default for LinterConfig {
    fn default() -> Self {
        Self {
            max_chunk_size: DEFAULT_MAX_CHUNK_SIZE,
            required_frontmatter: true,
            require_template_links: true,
            enforce_readme_per_directory: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangelogConfig {
    pub auto_create_init_entry: bool,
    pub entry_file_pattern: String,
    pub timezone: String,
}

impl Default for ChangelogConfig {
    fn default() -> Self {
        Self {
            auto_create_init_entry: true,
            entry_file_pattern: DEFAULT_CHANGELOG_PATTERN.to_string(),
            timezone: "UTC".to_string(),
        }
    }
}

fn slugify(value: &str) -> String {
    let mut slug = String::with_capacity(value.len());
    let mut last_was_dash = false;

    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_was_dash = false;
        } else if !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }

    slug.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::{AktaConfig, DEFAULT_CHANGELOG_PATTERN};

    #[test]
    fn default_config_is_stable() {
        let config = AktaConfig::new("OPENAKTA Runtime");

        assert_eq!(config.schema_version, "1");
        assert_eq!(config.project.slug, "openakta-runtime");
        assert!(config.registry.remote_base_url.is_empty());
        assert!(config.linter.required_frontmatter);
        assert!(config.changelog.auto_create_init_entry);
        assert_eq!(
            config.changelog.entry_file_pattern,
            DEFAULT_CHANGELOG_PATTERN
        );
    }

    #[test]
    fn config_round_trips_through_yaml() {
        let config = AktaConfig::new("OPENAKTA Runtime");
        let yaml = serde_yaml::to_string(&config).expect("serialize");
        let parsed: AktaConfig = serde_yaml::from_str(&yaml).expect("deserialize");

        assert_eq!(parsed, config);
    }
}
