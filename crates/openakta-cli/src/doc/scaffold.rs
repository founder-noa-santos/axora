use crate::doc::config::AktaConfig;
use crate::doc::registry::{
    section_specs, BundledTemplateRegistry, RegistryResolution, TemplateAsset, TemplateSource,
};
use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const DOCS_ROOT_DIR: &str = "akta-docs";
const INIT_CHANGELOG_DOC_ID: &str = "akta-docs";
const INIT_CHANGELOG_TYPE: &str = "init";
const INIT_CHANGELOG_SLUG: &str = "ssot-foundation";

#[derive(Debug, Clone)]
pub struct DocInitOptions {
    pub workspace_root: PathBuf,
    pub allow_non_empty: bool,
    pub overwrite: bool,
    pub project_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DocInitReport {
    pub docs_root: PathBuf,
    pub created_directories: Vec<PathBuf>,
    pub overwritten_files: Vec<PathBuf>,
    pub template_sources: Vec<(String, TemplateSource)>,
    pub warnings: Vec<String>,
}

pub async fn run_doc_init(options: DocInitOptions) -> Result<DocInitReport> {
    let docs_root = options.workspace_root.join(DOCS_ROOT_DIR);
    validate_target(&docs_root, options.allow_non_empty)?;

    let project_name = options
        .project_name
        .unwrap_or_else(|| infer_project_name(&options.workspace_root));
    let config = AktaConfig::new(project_name);
    let init_timestamp = Utc::now();
    let changelog_entry_path = changelog_entry_path(&docs_root, &config, init_timestamp)?;
    let managed_files = managed_files(&docs_root, &config, &changelog_entry_path);
    let overwritten_files = validate_managed_files(&managed_files, options.overwrite)?;

    let fetcher = BundledTemplateRegistry::from_config(&docs_root, &config)?;
    let RegistryResolution {
        assets: template_assets,
        warnings,
    } = fetcher.resolve_templates().await?;

    create_root(&docs_root)?;
    write_docs_root_readme(&docs_root, &config)?;
    write_gitignore(&docs_root)?;
    write_config(&docs_root, &config, options.overwrite)?;
    write_templates(&docs_root, &template_assets, options.overwrite)?;
    let created_directories = write_sections(&docs_root, &template_assets, options.overwrite)?;
    write_changelog_entry(&docs_root, &config, init_timestamp, options.overwrite)?;

    Ok(DocInitReport {
        docs_root,
        created_directories,
        overwritten_files,
        template_sources: template_assets
            .into_iter()
            .map(|asset| (asset.descriptor.id, asset.source))
            .collect(),
        warnings,
    })
}

fn validate_target(docs_root: &Path, allow_non_empty: bool) -> Result<()> {
    if !docs_root.exists() {
        return Ok(());
    }

    if !docs_root.is_dir() {
        bail!("{} exists but is not a directory", docs_root.display());
    }

    let mut entries = fs::read_dir(docs_root)
        .with_context(|| format!("failed to inspect {}", docs_root.display()))?;
    if entries.next().is_none() || allow_non_empty {
        return Ok(());
    }

    bail!(
        "{} already exists and is not empty; rerun with --allow-non-empty to reuse it",
        docs_root.display()
    );
}

fn validate_managed_files(managed_files: &[PathBuf], overwrite: bool) -> Result<Vec<PathBuf>> {
    let existing: Vec<PathBuf> = managed_files
        .iter()
        .filter(|path| path.exists())
        .cloned()
        .collect();

    if existing.is_empty() || overwrite {
        return Ok(existing);
    }

    let preview = existing
        .iter()
        .take(5)
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    bail!(
        "managed documentation files already exist: {preview}; rerun with --overwrite to replace managed scaffold files"
    );
}

fn managed_files(
    docs_root: &Path,
    config: &AktaConfig,
    changelog_entry_path: &Path,
) -> Vec<PathBuf> {
    let mut files = vec![
        docs_root.join("README.md"),
        docs_root.join(".gitignore"),
        docs_root.join(".akta-config.yaml"),
        docs_root
            .join("00-meta")
            .join("templates")
            .join("README.md"),
        changelog_entry_path.to_path_buf(),
    ];

    for spec in section_specs() {
        files.push(docs_root.join(spec.id).join("README.md"));
        files.push(
            docs_root
                .join("00-meta")
                .join("templates")
                .join(format!("{}.md", spec.template_id)),
        );
    }

    if !config.changelog.auto_create_init_entry {
        files.retain(|path| path != changelog_entry_path);
    }

    files
}

fn create_root(docs_root: &Path) -> Result<()> {
    fs::create_dir_all(docs_root)
        .with_context(|| format!("failed to create docs root {}", docs_root.display()))
}

fn write_docs_root_readme(docs_root: &Path, config: &AktaConfig) -> Result<()> {
    let body = format!(
        "# akta-docs\n\nAI-optimized documentation scaffold for `{}`.\n\nStart with [00-meta](00-meta/README.md) for standards, templates, and documentation governance.\n",
        config.project.name
    );
    write_file(docs_root.join("README.md"), body)
}

fn write_gitignore(docs_root: &Path) -> Result<()> {
    write_file(docs_root.join(".gitignore"), ".openakta/\n".to_string())
}

fn write_config(docs_root: &Path, config: &AktaConfig, overwrite: bool) -> Result<()> {
    let yaml = serde_yaml::to_string(config).context("failed to serialize .akta-config.yaml")?;
    let config_path = docs_root.join(".akta-config.yaml");
    write_managed_file(&config_path, yaml, overwrite)
}

fn write_templates(docs_root: &Path, assets: &[TemplateAsset], overwrite: bool) -> Result<()> {
    let templates_dir = docs_root.join("00-meta").join("templates");
    fs::create_dir_all(&templates_dir)
        .with_context(|| format!("failed to create {}", templates_dir.display()))?;

    for asset in assets {
        let path = templates_dir.join(format!("{}.md", asset.descriptor.id));
        write_managed_file(&path, asset.content.clone(), overwrite)?;
    }

    let readme = "# Base Templates\n\nCanonical templates cached for offline-safe documentation scaffolding.\n";
    write_managed_file(
        templates_dir.join("README.md"),
        readme.to_string(),
        overwrite,
    )
}

fn write_sections(
    docs_root: &Path,
    assets: &[TemplateAsset],
    overwrite: bool,
) -> Result<Vec<PathBuf>> {
    let by_id: HashMap<&str, &TemplateAsset> = assets
        .iter()
        .map(|asset| (asset.descriptor.id.as_str(), asset))
        .collect();
    let mut created = Vec::new();

    for spec in section_specs() {
        let section_dir = docs_root.join(spec.id);
        fs::create_dir_all(&section_dir)
            .with_context(|| format!("failed to create {}", section_dir.display()))?;

        let template = by_id.get(spec.template_id).copied().ok_or_else(|| {
            anyhow!(
                "missing template {} required for section {}",
                spec.template_id,
                spec.id
            )
        })?;

        let readme =
            build_section_readme(spec.id, spec.description, spec.template_id, template.source);
        write_managed_file(section_dir.join("README.md"), readme, overwrite)?;
        created.push(section_dir);
    }

    Ok(created)
}

fn write_changelog_entry(
    docs_root: &Path,
    config: &AktaConfig,
    timestamp: chrono::DateTime<Utc>,
    overwrite: bool,
) -> Result<()> {
    if !config.changelog.auto_create_init_entry {
        return Ok(());
    }

    let entry_path = changelog_entry_path(docs_root, config, timestamp)?;
    let entry_body = format!(
        "# Documentation Init\n\n- timestamp: {}\n- doc_id: {}\n- type: {}\n- slug: {}\n- actor: openakta doc init\n\nInitialized the SSOT documentation scaffold and canonical template cache.\n",
        timestamp.to_rfc3339(),
        INIT_CHANGELOG_DOC_ID,
        INIT_CHANGELOG_TYPE,
        INIT_CHANGELOG_SLUG
    );
    write_managed_file(entry_path, entry_body, overwrite)
}

fn changelog_entry_path(
    docs_root: &Path,
    config: &AktaConfig,
    timestamp: chrono::DateTime<Utc>,
) -> Result<PathBuf> {
    let rendered = config
        .changelog
        .entry_file_pattern
        .replace(
            "{timestamp}",
            &timestamp.format("%Y%m%dT%H%M%SZ").to_string(),
        )
        .replace("{doc_id}", INIT_CHANGELOG_DOC_ID)
        .replace("{type}", INIT_CHANGELOG_TYPE)
        .replace("{slug}", INIT_CHANGELOG_SLUG);

    if rendered.is_empty() || rendered.contains(std::path::MAIN_SEPARATOR) {
        bail!(
            "invalid changelog entry_file_pattern: {}",
            config.changelog.entry_file_pattern
        );
    }

    Ok(docs_root.join("10-changelog").join(rendered))
}

fn build_section_readme(
    section_id: &str,
    description: &str,
    template_id: &str,
    source: TemplateSource,
) -> String {
    let mut body = format!(
        "# {section_id}\n\n{description}\n\n## Canonical Template\n\n- Base template: [../00-meta/templates/{template_id}.md](../00-meta/templates/{template_id}.md)\n- Source: {}\n",
        source.as_str()
    );

    if section_id == "10-changelog" {
        body.push_str(
            "\n## Entry Model\n\nUse timestamped migration-style files matching `{timestamp}_{doc_id}_{type}_{slug}.md`.\n",
        );
    }

    body
}

fn infer_project_name(workspace_root: &Path) -> String {
    workspace_root
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("openakta-project")
        .to_string()
}

fn write_managed_file(path: impl AsRef<Path>, body: String, overwrite: bool) -> Result<()> {
    let path = path.as_ref();
    if path.exists() && !overwrite {
        bail!(
            "refusing to overwrite managed file {}; rerun with --overwrite to replace it",
            path.display()
        );
    }

    write_file(path, body)
}

fn write_file(path: impl AsRef<Path>, body: String) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(path, body).with_context(|| format!("failed to write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::{run_doc_init, DocInitOptions, DOCS_ROOT_DIR};
    use std::fs;
    use tempfile::tempdir;

    #[tokio::test]
    async fn creates_scaffold_with_config_and_templates() {
        let tmp = tempdir().expect("tempdir");

        let report = run_doc_init(DocInitOptions {
            workspace_root: tmp.path().to_path_buf(),
            allow_non_empty: false,
            overwrite: false,
            project_name: Some("demo-runtime".to_string()),
        })
        .await
        .expect("doc init");

        let docs_root = tmp.path().join(DOCS_ROOT_DIR);
        assert_eq!(report.docs_root, docs_root);
        assert!(docs_root.join(".akta-config.yaml").is_file());
        assert!(docs_root.join("README.md").is_file());
        assert!(docs_root.join(".gitignore").is_file());
        assert!(docs_root.join("00-meta/README.md").is_file());
        assert!(docs_root.join("00-meta/templates/meta-base.md").is_file());
        assert!(docs_root.join("99-archive/README.md").is_file());
        assert!(fs::read_dir(docs_root.join("10-changelog"))
            .expect("changelog dir")
            .any(|entry| {
                entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .contains("_akta-docs_init_ssot-foundation.md")
            }));
    }

    #[tokio::test]
    async fn rejects_existing_managed_files_without_overwrite() {
        let tmp = tempdir().expect("tempdir");
        let docs_root = tmp.path().join(DOCS_ROOT_DIR);
        fs::create_dir_all(&docs_root).expect("docs root");
        fs::write(docs_root.join("README.md"), "custom").expect("readme");

        let error = run_doc_init(DocInitOptions {
            workspace_root: tmp.path().to_path_buf(),
            allow_non_empty: true,
            overwrite: false,
            project_name: Some("demo-runtime".to_string()),
        })
        .await
        .expect_err("expected collision");

        assert!(error
            .to_string()
            .contains("managed documentation files already exist"));
    }

    #[tokio::test]
    async fn overwrites_managed_files_only_when_enabled() {
        let tmp = tempdir().expect("tempdir");
        let docs_root = tmp.path().join(DOCS_ROOT_DIR);
        fs::create_dir_all(&docs_root).expect("docs root");
        fs::write(docs_root.join("README.md"), "custom").expect("readme");

        let report = run_doc_init(DocInitOptions {
            workspace_root: tmp.path().to_path_buf(),
            allow_non_empty: true,
            overwrite: true,
            project_name: Some("demo-runtime".to_string()),
        })
        .await
        .expect("doc init");

        assert!(report
            .overwritten_files
            .iter()
            .any(|path| path.ends_with("README.md")));
        assert!(fs::read_to_string(docs_root.join("README.md"))
            .expect("readme")
            .contains("AI-optimized documentation scaffold"));
    }
}
