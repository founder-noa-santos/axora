use anyhow::{bail, Context, Result};
use openakta_docs::{format_diagnostics, LintResult, MarkdownLinter};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::doc::scaffold::DOCS_ROOT_DIR;

#[derive(Debug, Clone)]
pub struct DocLintOptions {
    pub workspace_root: PathBuf,
    pub targets: Vec<PathBuf>,
}

pub fn run_doc_lint(options: DocLintOptions) -> Result<LintResult> {
    let targets = if options.targets.is_empty() {
        vec![options.workspace_root.join(DOCS_ROOT_DIR)]
    } else {
        options
            .targets
            .iter()
            .map(|target| {
                if target.is_absolute() {
                    target.clone()
                } else {
                    options.workspace_root.join(target)
                }
            })
            .collect()
    };

    let files = collect_markdown_files(&targets)?;
    let linter = MarkdownLinter::new_strict_geo();
    let mut combined = LintResult::default();

    for file in files {
        let result = linter.lint_file(&file)?;
        combined.summary.error_count += result.summary.error_count;
        combined.summary.warn_count += result.summary.warn_count;
        combined.diagnostics.extend(result.diagnostics);
    }

        combined.diagnostics.sort_by(|a, b| {
            a.file
                .cmp(&b.file)
                .then(a.line.cmp(&b.line))
                .then(a.column.cmp(&b.column))
                .then(a.rule_id.cmp(&b.rule_id))
        });

    Ok(combined)
}

pub fn render_doc_lint(result: &LintResult, workspace_root: &Path) -> String {
    format_diagnostics(result, Some(workspace_root))
}

fn collect_markdown_files(targets: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for target in targets {
        if !target.exists() {
            bail!("lint target does not exist: {}", target.display());
        }

        if target.is_file() {
            if is_markdown_file(target) {
                files.push(target.clone());
            }
            continue;
        }

        for entry in WalkDir::new(target) {
            let entry = entry.with_context(|| format!("failed to read {}", target.display()))?;
            if entry.file_type().is_file() && is_markdown_file(entry.path()) {
                files.push(entry.path().to_path_buf());
            }
        }
    }

    files.sort();
    Ok(files)
}

fn is_markdown_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("md"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::{collect_markdown_files, is_markdown_file};
    use std::fs;

    #[test]
    fn recognizes_markdown_extensions() {
        assert!(is_markdown_file(std::path::Path::new("guide.md")));
        assert!(is_markdown_file(std::path::Path::new("GUIDE.MD")));
        assert!(!is_markdown_file(std::path::Path::new("guide.txt")));
    }

    #[test]
    fn recursively_collects_markdown_files() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let docs = tempdir.path().join("akta-docs");
        fs::create_dir_all(docs.join("nested")).expect("mkdir");
        fs::write(docs.join("root.md"), "# Root").expect("root");
        fs::write(docs.join("nested").join("child.md"), "# Child").expect("child");
        fs::write(docs.join("nested").join("skip.txt"), "ignored").expect("skip");

        let files = collect_markdown_files(&[docs]).expect("collect");
        assert_eq!(files.len(), 2);
    }
}
