use std::cmp::Reverse;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use walkdir::{DirEntry, WalkDir};

const MAX_CONTEXT_FILES: usize = 6;
const MAX_FILE_BYTES: u64 = 64 * 1024;
const MAX_FILE_CHARS: usize = 4_000;

pub fn build_workspace_context(workspace_root: &Path, prompt: &str) -> anyhow::Result<String> {
    let top_level = top_level_entries(workspace_root)?;
    let prompt_tokens = prompt_tokens(prompt);
    let relevant_files = relevant_files(workspace_root, &prompt_tokens)?;

    let mut sections = Vec::new();
    sections.push(format!("Workspace root: {}", workspace_root.display()));

    if !top_level.is_empty() {
        sections.push(format!("Top-level entries:\n{}", top_level.join("\n")));
    }

    if !relevant_files.is_empty() {
        let mut rendered = Vec::new();
        for path in relevant_files {
            let relative = path
                .strip_prefix(workspace_root)
                .unwrap_or(&path)
                .display()
                .to_string();
            let content = std::fs::read_to_string(&path)?;
            let snippet = truncate_chars(&content, MAX_FILE_CHARS);
            rendered.push(format!("File: {relative}\n{snippet}"));
        }
        sections.push(format!("Relevant code context:\n{}", rendered.join("\n\n---\n\n")));
    }

    Ok(sections.join("\n\n"))
}

fn top_level_entries(root: &Path) -> anyhow::Result<Vec<String>> {
    let mut entries = std::fs::read_dir(root)?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| !ignored_name(name))
        .collect::<Vec<_>>();
    entries.sort();
    Ok(entries)
}

fn prompt_tokens(prompt: &str) -> HashSet<String> {
    prompt
        .split(|c: char| !c.is_ascii_alphanumeric())
        .map(|token| token.trim().to_ascii_lowercase())
        .filter(|token| token.len() >= 3)
        .collect()
}

fn relevant_files(root: &Path, tokens: &HashSet<String>) -> anyhow::Result<Vec<PathBuf>> {
    let mut scored = Vec::new();

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| should_descend(entry))
        .filter_map(|entry| entry.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.metadata().map(|m| m.len() > MAX_FILE_BYTES).unwrap_or(true) {
            continue;
        }

        let path = entry.path();
        let relative = path.strip_prefix(root).unwrap_or(path);
        let path_text = relative.display().to_string().to_ascii_lowercase();
        let mut score = 0usize;

        for token in tokens {
            if path_text.contains(token) {
                score += 10;
            }
        }

        if score == 0 && tokens.is_empty() {
            score = 1;
        }

        if score == 0 {
            continue;
        }

        if std::fs::read_to_string(path).is_err() {
            continue;
        }

        scored.push((Reverse(score), relative.to_path_buf(), path.to_path_buf()));
    }

    scored.sort();
    Ok(scored
        .into_iter()
        .take(MAX_CONTEXT_FILES)
        .map(|(_, _, absolute)| absolute)
        .collect())
}

fn truncate_chars(content: &str, max_chars: usize) -> String {
    let mut result = content.chars().take(max_chars).collect::<String>();
    if content.chars().count() > max_chars {
        result.push_str("\n...[truncated]");
    }
    result
}

fn should_descend(entry: &DirEntry) -> bool {
    !ignored_name(&entry.file_name().to_string_lossy())
}

fn ignored_name(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".openakta"
            | "target"
            | "node_modules"
            | "dist"
            | "build"
            | ".DS_Store"
    )
}
