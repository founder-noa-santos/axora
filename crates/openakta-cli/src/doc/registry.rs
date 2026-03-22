//! Local-first GEO template registry: all section templates are **embedded in the binary**.
//!
//! No HTTP(S) fetches — supply-chain safe, offline by default.

use crate::doc::config::{AktaConfig, DEFAULT_CACHE_TTL_HOURS};
use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tracing::warn;

const MANIFEST_CACHE_FILE: &str = "manifest.json";
const TEMPLATE_CACHE_SUBDIR: &str = "templates";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryManifest {
    pub version: String,
    pub templates: Vec<TemplateDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateDescriptor {
    pub id: String,
    pub path: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct TemplateAsset {
    pub descriptor: TemplateDescriptor,
    pub content: String,
    pub source: TemplateSource,
}

#[derive(Debug, Clone)]
pub struct RegistryResolution {
    pub assets: Vec<TemplateAsset>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateSource {
    /// Bytes came from `include_str!(...)` embedded templates.
    Bundled,
    /// Previously materialized copy under `.openakta/templates_cache` (local disk only).
    Cache,
}

impl TemplateSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bundled => "bundled-embedded",
            Self::Cache => "local-cache",
        }
    }
}

/// Resolves documentation templates from the **bundled** manifest only (no network I/O).
#[derive(Debug)]
pub struct BundledTemplateRegistry {
    cache_dir: PathBuf,
    cache_ttl: Duration,
}

impl BundledTemplateRegistry {
    pub fn from_config(docs_root: &Path, config: &AktaConfig) -> Result<Self> {
        let configured_cache_dir = PathBuf::from(&config.registry.cache_dir);
        let cache_dir = if configured_cache_dir.is_absolute() {
            configured_cache_dir
        } else {
            docs_root.join(configured_cache_dir)
        };

        Self::new(
            cache_dir,
            Duration::from_secs(config.registry.cache_ttl_hours.max(1) * 60 * 60),
        )
    }

    pub fn new(cache_dir: PathBuf, cache_ttl: Duration) -> Result<Self> {
        Ok(Self { cache_dir, cache_ttl })
    }

    /// Materialize every required template from embedded sources; optionally refresh local cache files.
    pub async fn resolve_templates(&self) -> Result<RegistryResolution> {
        fs::create_dir_all(self.templates_dir()).with_context(|| {
            format!(
                "failed to create cache directory {}",
                self.cache_dir.display()
            )
        })?;

        let mut warnings = Vec::new();
        let manifest = bundled_manifest();
        self.validate_manifest(&manifest)?;
        let required_descriptors = self.required_descriptors(&manifest)?;

        let mut assets = Vec::with_capacity(required_descriptors.len());
        for descriptor in &required_descriptors {
            assets.push(self.load_bundled_or_refresh_cache(descriptor, &mut warnings)?);
        }

        self.write_manifest_snapshot(&manifest)?;

        Ok(RegistryResolution { assets, warnings })
    }

    fn load_bundled_or_refresh_cache(
        &self,
        descriptor: &TemplateDescriptor,
        warnings: &mut Vec<String>,
    ) -> Result<TemplateAsset> {
        let cache_path = self.template_cache_path(&descriptor.id);
        if cache_path.is_file() && is_fresh(&cache_path, self.cache_ttl) {
            let content = fs::read_to_string(&cache_path).with_context(|| {
                format!("failed to read cached template {}", cache_path.display())
            })?;
            return Ok(TemplateAsset {
                descriptor: descriptor.clone(),
                content,
                source: TemplateSource::Cache,
            });
        }

        let bundled = bundled_template(&descriptor.id).with_context(|| {
            format!(
                "template {} not available in bundled fallback set",
                descriptor.id
            )
        })?;

        self.write_template_cache(descriptor, bundled)?;
        self.record_warning(
            warnings,
            format!(
                "template {} materialized from embedded bundle to {}",
                descriptor.id,
                cache_path.display()
            ),
        );

        Ok(TemplateAsset {
            descriptor: descriptor.clone(),
            content: bundled.to_string(),
            source: TemplateSource::Bundled,
        })
    }

    fn validate_manifest(&self, manifest: &RegistryManifest) -> Result<()> {
        let available: BTreeSet<&str> = manifest
            .templates
            .iter()
            .map(|item| item.id.as_str())
            .collect();
        let missing: Vec<&str> = required_template_ids()
            .into_iter()
            .filter(|template_id| !available.contains(template_id))
            .collect();

        if missing.is_empty() {
            Ok(())
        } else {
            bail!(
                "manifest is missing required templates: {}",
                missing.join(", ")
            )
        }
    }

    fn required_descriptors(&self, manifest: &RegistryManifest) -> Result<Vec<TemplateDescriptor>> {
        let by_id: BTreeMap<&str, &TemplateDescriptor> = manifest
            .templates
            .iter()
            .map(|descriptor| (descriptor.id.as_str(), descriptor))
            .collect();

        required_template_ids()
            .into_iter()
            .map(|template_id| {
                by_id.get(template_id).cloned().cloned().ok_or_else(|| {
                    anyhow!("required template {} is missing from manifest", template_id)
                })
            })
            .collect()
    }

    fn record_warning(&self, warnings: &mut Vec<String>, message: String) {
        warn!("{message}");
        warnings.push(message);
    }

    fn write_manifest_snapshot(&self, manifest: &RegistryManifest) -> Result<()> {
        fs::create_dir_all(&self.cache_dir).with_context(|| {
            format!(
                "failed to create cache directory {}",
                self.cache_dir.display()
            )
        })?;
        let body = serde_json::to_string_pretty(manifest).context("serialize manifest")?;
        fs::write(self.manifest_cache_path(), body).context("failed to write local manifest copy")
    }

    fn write_template_cache(&self, descriptor: &TemplateDescriptor, body: &str) -> Result<()> {
        let path = self.template_cache_path(&descriptor.id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create template cache directory {}",
                    parent.display()
                )
            })?;
        }
        fs::write(path, body).with_context(|| format!("failed to cache template {}", descriptor.id))
    }

    fn manifest_cache_path(&self) -> PathBuf {
        self.cache_dir.join(MANIFEST_CACHE_FILE)
    }

    fn template_cache_path(&self, template_id: &str) -> PathBuf {
        self.templates_dir().join(format!("{template_id}.md"))
    }

    fn templates_dir(&self) -> PathBuf {
        self.cache_dir.join(TEMPLATE_CACHE_SUBDIR)
    }
}

fn required_template_ids() -> Vec<&'static str> {
    section_specs()
        .into_iter()
        .map(|spec| spec.template_id)
        .collect()
}

fn is_fresh(path: &Path, ttl: Duration) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    let Ok(modified) = metadata.modified() else {
        return false;
    };
    let Ok(elapsed) = SystemTime::now().duration_since(modified) else {
        return false;
    };

    elapsed <= ttl
}

fn bundled_manifest() -> RegistryManifest {
    RegistryManifest {
        version: "1".to_string(),
        templates: section_specs()
            .into_iter()
            .map(|spec| TemplateDescriptor {
                id: spec.template_id.to_string(),
                path: format!("templates/{}.md", spec.template_id),
                description: spec.template_description.to_string(),
            })
            .collect(),
    }
}

pub fn section_specs() -> Vec<SectionSpec> {
    vec![
        SectionSpec::new(
            "00-meta",
            "Documentation system metadata, style guides, templates, and configuration.",
            "meta-base",
        ),
        SectionSpec::new(
            "01-adrs",
            "Architecture Decision Records and decision history.",
            "adr-base",
        ),
        SectionSpec::new(
            "02-business-core",
            "Vision, strategy, roadmap, personas, and business metrics.",
            "business-core-base",
        ),
        SectionSpec::new(
            "03-business-logic",
            "Business rules, state machines, domain models, and invariants.",
            "business-logic-base",
        ),
        SectionSpec::new(
            "04-research",
            "Research reports, technical evaluations, and discovery artifacts.",
            "research-base",
        ),
        SectionSpec::new(
            "05-features",
            "Feature specifications, acceptance criteria, and delivery contracts.",
            "feature-base",
        ),
        SectionSpec::new(
            "06-technical",
            "C4 models, APIs, security, infrastructure, and deployment docs.",
            "technical-base",
        ),
        SectionSpec::new(
            "07-guides",
            "Tutorials, how-to guides, FAQs, and operational guides using Diataxis.",
            "guide-base",
        ),
        SectionSpec::new(
            "08-references",
            "Reference material for commands, schemas, and error catalogs.",
            "reference-base",
        ),
        SectionSpec::new(
            "09-explanations",
            "Rationale, trade-off analysis, and explanatory material.",
            "explanation-base",
        ),
        SectionSpec::new(
            "10-changelog",
            "Temporal log of documentation evolution and automated changes.",
            "changelog-base",
        ),
        SectionSpec::new(
            "99-archive",
            "Deprecated and retired documentation retained for auditability.",
            "archive-base",
        ),
    ]
}

#[derive(Debug, Clone, Copy)]
pub struct SectionSpec {
    pub id: &'static str,
    pub description: &'static str,
    pub template_id: &'static str,
    pub template_description: &'static str,
}

impl SectionSpec {
    const fn new(id: &'static str, description: &'static str, template_id: &'static str) -> Self {
        Self {
            id,
            description,
            template_id,
            template_description: "Canonical base template for this section.",
        }
    }
}

fn bundled_template(template_id: &str) -> Result<&'static str> {
    match template_id {
        "meta-base" => Ok(include_str!("templates/meta-base.md")),
        "adr-base" => Ok(include_str!("templates/adr-base.md")),
        "business-core-base" => Ok(include_str!("templates/business-core-base.md")),
        "business-logic-base" => Ok(include_str!("templates/business-logic-base.md")),
        "research-base" => Ok(include_str!("templates/research-base.md")),
        "feature-base" => Ok(include_str!("templates/feature-base.md")),
        "technical-base" => Ok(include_str!("templates/technical-base.md")),
        "guide-base" => Ok(include_str!("templates/guide-base.md")),
        "reference-base" => Ok(include_str!("templates/reference-base.md")),
        "explanation-base" => Ok(include_str!("templates/explanation-base.md")),
        "changelog-base" => Ok(include_str!("templates/changelog-base.md")),
        "archive-base" => Ok(include_str!("templates/archive-base.md")),
        _ => Err(anyhow!("unknown bundled template id {template_id}")),
    }
}

impl Default for BundledTemplateRegistry {
    fn default() -> Self {
        Self::new(
            PathBuf::from(".openakta/templates_cache"),
            Duration::from_secs(DEFAULT_CACHE_TTL_HOURS * 60 * 60),
        )
        .expect("bundled registry config")
    }
}

#[cfg(test)]
mod tests {
    use super::{bundled_manifest, BundledTemplateRegistry, TemplateSource};
    use std::time::Duration;
    use tempfile::tempdir;

    #[tokio::test]
    async fn resolves_all_templates_from_bundle_without_network() {
        let tmp = tempdir().expect("tempdir");
        let fetcher = BundledTemplateRegistry::new(
            tmp.path().join(".openakta/templates_cache"),
            Duration::from_secs(60),
        )
        .expect("registry");

        let resolution = fetcher.resolve_templates().await.expect("assets");

        assert_eq!(resolution.assets.len(), bundled_manifest().templates.len());
        assert!(resolution
            .assets
            .iter()
            .all(|asset| matches!(asset.source, TemplateSource::Bundled | TemplateSource::Cache)));
    }
}
