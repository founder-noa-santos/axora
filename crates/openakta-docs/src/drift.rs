//! AST-to-documentation drift detection for LivingDocs.

use crate::ast::{AstFileSnapshot, AstSymbol, AstSymbolKind, IncrementalAstParser};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser as MarkdownParser, Tag, TagEnd};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_yaml::from_str as parse_yaml;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::warn;

/// Result type for drift detection.
pub type Result<T> = std::result::Result<T, DriftError>;

/// Drift detection error.
#[derive(Debug, thiserror::Error)]
pub enum DriftError {
    /// Filesystem error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// AST parsing error.
    #[error("ast error: {0}")]
    Ast(#[from] crate::ast::AstError),
}

/// Expected code symbol described by docs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocSymbolExpectation {
    /// Source document path.
    pub doc_path: PathBuf,
    /// Referenced code file path.
    pub code_path: PathBuf,
    /// Expected symbol name.
    pub symbol_name: String,
    /// Expected symbol kind when declared.
    pub kind: Option<AstSymbolKind>,
    /// Expected signature when declared.
    pub signature: Option<String>,
    /// Expected rule IDs.
    pub rule_ids: Vec<String>,
    /// Optional stored structural hash from docs.
    pub structural_hash: Option<String>,
    /// 1-based line in the doc where the marker appeared.
    pub line: usize,
}

/// Generic code reference embedded in docs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeReference {
    /// Source document.
    pub doc_path: PathBuf,
    /// Referenced code path.
    pub code_path: PathBuf,
    /// Optional symbol selector.
    pub symbol_name: Option<String>,
    /// 1-based line in doc.
    pub line: usize,
}

/// Parsed documentation expectations index.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocExpectationIndex {
    /// Structured symbol expectations.
    pub symbol_expectations: Vec<DocSymbolExpectation>,
    /// Loose code references.
    pub code_references: Vec<CodeReference>,
}

impl DocExpectationIndex {
    /// Parse the relevant akta-docs directories.
    pub fn parse_from_root(docs_root: &Path) -> Result<Self> {
        let mut index = Self::default();
        for subdir in ["03-business-logic", "06-technical"] {
            let directory = docs_root.join(subdir);
            if !directory.exists() {
                continue;
            }
            for path in walk_markdown_files(&directory)? {
                let content = fs::read_to_string(&path)?;
                parse_markdown_expectations(docs_root, &path, &content, &mut index);
            }
        }
        Ok(index)
    }
}

/// Code-side snapshot index used by the detector.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeRealityIndex {
    /// Parsed files by path.
    pub files: HashMap<PathBuf, AstFileSnapshot>,
}

impl CodeRealityIndex {
    /// Upsert one parsed file snapshot into the index.
    pub fn upsert_snapshot(&mut self, snapshot: AstFileSnapshot) {
        self.files.insert(snapshot.file_path.clone(), snapshot);
    }

    /// Remove a file from the tracked code reality.
    pub fn remove_file(&mut self, code_path: &Path) {
        self.files.remove(code_path);
    }

    /// Build from a set of changed files using the incremental parser.
    pub fn from_files<'a, I>(parser: &mut IncrementalAstParser, files: I) -> Result<Self>
    where
        I: IntoIterator<Item = (&'a Path, &'a str)>,
    {
        let mut index = Self::default();
        for (path, source) in files {
            let snapshot = parser.parse_changed_file(path, source)?;
            index.files.insert(path.to_path_buf(), snapshot);
        }
        Ok(index)
    }

    fn find_symbol(&self, code_path: &Path, symbol_name: &str) -> Option<&AstSymbol> {
        self.files.get(code_path).and_then(|file| {
            file.symbols
                .iter()
                .find(|symbol| symbol.name == symbol_name)
        })
    }
}

/// Drift domain used by the confidence scorer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriftDomain {
    /// API, signature, and structural contract drift.
    ApiSurface,
    /// Business-rule linkage drift.
    BusinessRule,
    /// Broken doc-to-code references.
    CodeReference,
}

/// Drift finding severity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriftSeverity {
    /// User-facing drift or hard mismatch.
    Critical,
    /// Significant semantic mismatch requiring review.
    Warning,
    /// Informational signal.
    Info,
}

/// Drift category for downstream resolution systems.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriftKind {
    /// Symbol documented in docs is missing in code.
    MissingSymbol,
    /// Symbol exists but signature does not match.
    SignatureMismatch,
    /// Rule-to-code linkage is missing.
    MissingRuleBinding,
    /// Stored structure marker no longer matches current code.
    StructuralDrift,
    /// Markdown code reference no longer resolves.
    DeadCodeReference,
}

/// Single inconsistency flag emitted by the detector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InconsistencyFlag {
    /// High-level domain category for downstream confidence scoring.
    pub domain: DriftDomain,
    /// Drift category.
    pub kind: DriftKind,
    /// Severity for later confidence routing.
    pub severity: DriftSeverity,
    /// Human-readable explanation.
    pub message: String,
    /// Source documentation file.
    pub doc_path: PathBuf,
    /// Optional code file implicated in drift.
    pub code_path: Option<PathBuf>,
    /// Symbol name when specific.
    pub symbol_name: Option<String>,
    /// Business rule IDs implicated.
    pub rule_ids: Vec<String>,
    /// Stable dedupe key.
    pub fingerprint: String,
}

/// Summary report produced for one evaluation pass.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriftReport {
    /// Total findings.
    pub total_flags: usize,
    /// API contract and structural drift count.
    pub api_surface_flags: usize,
    /// Business-rule linkage drift count.
    pub business_rule_flags: usize,
    /// Broken documentation reference count.
    pub code_reference_flags: usize,
    /// Count by severity.
    pub critical_flags: usize,
    /// Count by severity.
    pub warning_flags: usize,
    /// Count by severity.
    pub info_flags: usize,
    /// Highest severity seen in this report.
    pub highest_severity: Option<DriftSeverity>,
    /// Detailed findings.
    pub flags: Vec<InconsistencyFlag>,
}

/// Drift detector comparing docs to AST-extracted code reality.
pub struct DriftDetector;

impl DriftDetector {
    /// Evaluate doc expectations against code snapshots.
    pub fn detect(
        docs: &DocExpectationIndex,
        code: &CodeRealityIndex,
        repo_root: &Path,
    ) -> DriftReport {
        let mut flags = Vec::new();

        for expectation in &docs.symbol_expectations {
            let normalized_path = normalize_path(repo_root, &expectation.code_path);
            let actual_symbol = code.find_symbol(&normalized_path, &expectation.symbol_name);

            let Some(actual_symbol) = actual_symbol else {
                flags.push(make_flag(
                    DriftDomain::ApiSurface,
                    DriftKind::MissingSymbol,
                    DriftSeverity::Critical,
                    format!(
                        "Documented symbol `{}` no longer exists at `{}`.",
                        expectation.symbol_name,
                        normalized_path.display()
                    ),
                    expectation.doc_path.clone(),
                    Some(normalized_path),
                    Some(expectation.symbol_name.clone()),
                    expectation.rule_ids.clone(),
                ));
                continue;
            };

            if let Some(expected_kind) = &expectation.kind {
                if expected_kind != &actual_symbol.kind {
                    flags.push(make_flag(
                        classify_structural_domain(&expectation.rule_ids),
                        DriftKind::StructuralDrift,
                        DriftSeverity::Warning,
                        format!(
                            "Documented symbol `{}` changed kind from `{:?}` to `{:?}`.",
                            expectation.symbol_name, expected_kind, actual_symbol.kind
                        ),
                        expectation.doc_path.clone(),
                        Some(normalized_path.clone()),
                        Some(expectation.symbol_name.clone()),
                        expectation.rule_ids.clone(),
                    ));
                }
            }

            if let Some(expected_signature) = &expectation.signature {
                if normalize_signature(expected_signature)
                    != normalize_signature(&actual_symbol.signature.raw)
                {
                    flags.push(make_flag(
                        DriftDomain::ApiSurface,
                        DriftKind::SignatureMismatch,
                        DriftSeverity::Critical,
                        format!(
                            "Signature drift for `{}`. docs=`{}` code=`{}`",
                            expectation.symbol_name,
                            expected_signature,
                            actual_symbol.signature.raw
                        ),
                        expectation.doc_path.clone(),
                        Some(normalized_path.clone()),
                        Some(expectation.symbol_name.clone()),
                        expectation.rule_ids.clone(),
                    ));
                }
            }

            let actual_rules: HashSet<&str> =
                actual_symbol.rule_ids.iter().map(String::as_str).collect();
            for rule_id in &expectation.rule_ids {
                if !actual_rules.contains(rule_id.as_str()) {
                    flags.push(make_flag(
                        DriftDomain::BusinessRule,
                        DriftKind::MissingRuleBinding,
                        DriftSeverity::Warning,
                        format!(
                            "Rule `{}` is documented for `{}` but not present in extracted annotations.",
                            rule_id, expectation.symbol_name
                        ),
                        expectation.doc_path.clone(),
                        Some(normalized_path.clone()),
                        Some(expectation.symbol_name.clone()),
                        vec![rule_id.clone()],
                    ));
                }
            }

            if let Some(expected_hash) = &expectation.structural_hash {
                if expected_hash != &actual_symbol.structural_hash {
                    flags.push(make_flag(
                        classify_structural_domain(&expectation.rule_ids),
                        DriftKind::StructuralDrift,
                        DriftSeverity::Warning,
                        format!(
                            "Structural hash drift for `{}`. docs=`{}` code=`{}`",
                            expectation.symbol_name, expected_hash, actual_symbol.structural_hash
                        ),
                        expectation.doc_path.clone(),
                        Some(normalized_path),
                        Some(expectation.symbol_name.clone()),
                        expectation.rule_ids.clone(),
                    ));
                }
            }
        }

        for reference in &docs.code_references {
            let normalized_path = normalize_path(repo_root, &reference.code_path);
            let Some(file) = code.files.get(&normalized_path) else {
                flags.push(make_flag(
                    DriftDomain::CodeReference,
                    DriftKind::DeadCodeReference,
                    DriftSeverity::Critical,
                    format!(
                        "Documentation reference points to missing file `{}`.",
                        normalized_path.display()
                    ),
                    reference.doc_path.clone(),
                    Some(normalized_path),
                    reference.symbol_name.clone(),
                    Vec::new(),
                ));
                continue;
            };

            if let Some(symbol_name) = &reference.symbol_name {
                if !file
                    .symbols
                    .iter()
                    .any(|symbol| symbol.name == *symbol_name)
                {
                    flags.push(make_flag(
                        DriftDomain::CodeReference,
                        DriftKind::DeadCodeReference,
                        DriftSeverity::Warning,
                        format!(
                            "Documentation reference `{}` no longer resolves in `{}`.",
                            symbol_name,
                            normalized_path.display()
                        ),
                        reference.doc_path.clone(),
                        Some(normalized_path),
                        Some(symbol_name.clone()),
                        Vec::new(),
                    ));
                }
            }
        }

        summarize_flags(flags)
    }
}

fn summarize_flags(flags: Vec<InconsistencyFlag>) -> DriftReport {
    let mut report = DriftReport::default();
    let mut flags = flags;
    flags.sort_by(|left, right| {
        severity_rank(&right.severity)
            .cmp(&severity_rank(&left.severity))
            .then_with(|| domain_rank(&left.domain).cmp(&domain_rank(&right.domain)))
            .then_with(|| left.doc_path.cmp(&right.doc_path))
            .then_with(|| left.code_path.cmp(&right.code_path))
            .then_with(|| left.symbol_name.cmp(&right.symbol_name))
    });

    for flag in flags {
        match flag.severity {
            DriftSeverity::Critical => {
                report.critical_flags += 1;
                report.highest_severity = Some(DriftSeverity::Critical);
            }
            DriftSeverity::Warning => {
                report.warning_flags += 1;
                if report.highest_severity.is_none() {
                    report.highest_severity = Some(DriftSeverity::Warning);
                }
            }
            DriftSeverity::Info => {
                report.info_flags += 1;
                if report.highest_severity.is_none() {
                    report.highest_severity = Some(DriftSeverity::Info);
                }
            }
        }
        match flag.domain {
            DriftDomain::ApiSurface => report.api_surface_flags += 1,
            DriftDomain::BusinessRule => report.business_rule_flags += 1,
            DriftDomain::CodeReference => report.code_reference_flags += 1,
        }
        report.flags.push(flag);
    }
    report.total_flags = report.flags.len();
    report
}

#[allow(clippy::too_many_arguments)]
fn make_flag(
    domain: DriftDomain,
    kind: DriftKind,
    severity: DriftSeverity,
    message: String,
    doc_path: PathBuf,
    code_path: Option<PathBuf>,
    symbol_name: Option<String>,
    rule_ids: Vec<String>,
) -> InconsistencyFlag {
    let fingerprint = blake3::hash(
        format!(
            "{kind:?}|{severity:?}|{}|{}|{}",
            doc_path.display(),
            code_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            symbol_name.as_deref().unwrap_or_default()
        )
        .as_bytes(),
    )
    .to_hex()
    .to_string();

    InconsistencyFlag {
        domain,
        kind,
        severity,
        message,
        doc_path,
        code_path,
        symbol_name,
        rule_ids,
        fingerprint,
    }
}

fn severity_rank(severity: &DriftSeverity) -> u8 {
    match severity {
        DriftSeverity::Critical => 3,
        DriftSeverity::Warning => 2,
        DriftSeverity::Info => 1,
    }
}

fn domain_rank(domain: &DriftDomain) -> u8 {
    match domain {
        DriftDomain::ApiSurface => 0,
        DriftDomain::BusinessRule => 1,
        DriftDomain::CodeReference => 2,
    }
}

fn parse_markdown_expectations(
    docs_root: &Path,
    doc_path: &Path,
    content: &str,
    index: &mut DocExpectationIndex,
) {
    parse_fenced_expectation_blocks(docs_root, doc_path, content, index);
    parse_legacy_expectations(docs_root, doc_path, content, index);

    let code_ref_re =
        Regex::new(r"`(?P<path>[^`#]+?\.(?:ts|tsx|js|jsx))(?:#(?P<symbol>[A-Za-z0-9_.$]+))?`")
            .expect("valid code reference regex");

    for (idx, line) in content.lines().enumerate() {
        let line_no = idx + 1;
        for captures in code_ref_re.captures_iter(line) {
            let path = captures
                .name("path")
                .map(|m| m.as_str())
                .unwrap_or_default();
            let symbol_name = captures.name("symbol").map(|m| m.as_str().to_string());
            index.code_references.push(CodeReference {
                doc_path: doc_path.to_path_buf(),
                code_path: resolve_doc_relative_path(docs_root, doc_path, path),
                symbol_name,
                line: line_no,
            });
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct FencedExpectationBlock {
    code_path: Option<String>,
    symbol: Option<String>,
    kind: Option<String>,
    signature: Option<String>,
    rule_ids: Option<Vec<String>>,
    structural_hash: Option<String>,
}

fn parse_fenced_expectation_blocks(
    docs_root: &Path,
    doc_path: &Path,
    content: &str,
    index: &mut DocExpectationIndex,
) {
    let initial_expectation_count = index.symbol_expectations.len();
    let parser = MarkdownParser::new_ext(content, Options::all());
    let mut in_expectation_block = false;
    let mut block_start = 0usize;
    let mut block_body_start = 0usize;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(language)))
                if language.as_ref() == "akta-expect" =>
            {
                in_expectation_block = true;
                block_start = range.start;
                block_body_start = range.end;
            }
            Event::End(TagEnd::CodeBlock) if in_expectation_block => {
                let line = line_number_for_offset(content, block_start);
                let block_body = content
                    .get(block_body_start..range.start)
                    .unwrap_or_default()
                    .trim_matches('\n');
                push_fenced_expectation_block(docs_root, doc_path, block_body, line, index);
                in_expectation_block = false;
            }
            _ => {}
        }
    }

    if index.symbol_expectations.len() > initial_expectation_count {
        return;
    }

    let mut fallback_body = String::new();
    let mut fallback_start_line = 0usize;
    let mut in_fallback_block = false;
    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if !in_fallback_block {
            if trimmed == "```akta-expect" {
                in_fallback_block = true;
                fallback_start_line = idx + 1;
                fallback_body.clear();
            }
            continue;
        }

        if trimmed.starts_with("```") {
            push_fenced_expectation_block(
                docs_root,
                doc_path,
                fallback_body.trim_end(),
                fallback_start_line,
                index,
            );
            in_fallback_block = false;
            fallback_body.clear();
            continue;
        }

        fallback_body.push_str(line);
        fallback_body.push('\n');
    }
}

fn push_fenced_expectation_block(
    docs_root: &Path,
    doc_path: &Path,
    block_body: &str,
    line: usize,
    index: &mut DocExpectationIndex,
) {
    match parse_fenced_expectation_yaml(block_body) {
        Ok(expectation) => {
            if let (Some(code_path), Some(symbol_name)) = (expectation.code_path, expectation.symbol)
            {
                index.symbol_expectations.push(DocSymbolExpectation {
                    doc_path: doc_path.to_path_buf(),
                    code_path: resolve_doc_relative_path(docs_root, doc_path, &code_path),
                    symbol_name,
                    kind: expectation.kind.as_deref().and_then(parse_kind),
                    signature: expectation.signature,
                    rule_ids: expectation.rule_ids.unwrap_or_default(),
                    structural_hash: expectation.structural_hash,
                    line,
                });
            } else {
                warn!(
                    doc = %doc_path.display(),
                    line,
                    "ignoring akta-expect block missing required `code_path` or `symbol`"
                );
            }
        }
        Err(error) => {
            warn!(
                doc = %doc_path.display(),
                line,
                error = %error,
                "failed to parse akta-expect fenced block"
            );
        }
    }
}

fn parse_fenced_expectation_yaml(block_body: &str) -> std::result::Result<FencedExpectationBlock, serde_yaml::Error> {
    parse_yaml::<FencedExpectationBlock>(block_body)
        .or_else(|_| parse_yaml::<FencedExpectationBlock>(&normalize_expectation_yaml(block_body)))
}

fn normalize_expectation_yaml(block_body: &str) -> String {
    let mut normalized = Vec::new();
    for line in block_body.lines() {
        let trimmed = line.trim_start();
        let indent_width = line.len() - trimmed.len();
        let indent = &line[..indent_width];

        if let Some((key, value)) = trimmed.split_once(':') {
            let key = key.trim();
            if matches!(
                key,
                "code_path" | "symbol" | "kind" | "signature" | "structural_hash"
            ) {
                let value = value.trim();
                if !value.is_empty()
                    && !matches!(value.chars().next(), Some('"') | Some('\'') | Some('|') | Some('>'))
                {
                    normalized.push(format!(
                        "{indent}{key}: {}",
                        serde_json::to_string(value).expect("serialize relaxed yaml scalar"),
                    ));
                    continue;
                }
            }
        }

        normalized.push(line.to_string());
    }

    normalized.join("\n")
}

fn parse_legacy_expectations(
    docs_root: &Path,
    doc_path: &Path,
    content: &str,
    index: &mut DocExpectationIndex,
) {
    let marker_re =
        Regex::new(r"^(CodePath|Symbol|Signature|Kind|RuleID|StructuralHash)\s*:\s*(?P<value>.+)$")
            .expect("valid marker regex");
    let mut pending_code_path: Option<PathBuf> = None;
    let mut pending_symbol: Option<String> = None;
    let mut pending_signature: Option<String> = None;
    let mut pending_kind: Option<AstSymbolKind> = None;
    let mut pending_rule_ids: Vec<String> = Vec::new();
    let mut pending_hash: Option<String> = None;
    let mut pending_line = 0usize;
    let mut warned_legacy = false;

    for (idx, line) in content.lines().enumerate() {
        let line_no = idx + 1;
        if let Some(captures) = marker_re.captures(line.trim()) {
            if !warned_legacy {
                warn!(
                    doc = %doc_path.display(),
                    line = line_no,
                    "legacy CodePath/Symbol markers are deprecated; migrate to ```akta-expect fenced blocks"
                );
                warned_legacy = true;
            }

            let key = captures.get(1).map(|m| m.as_str()).unwrap_or_default();
            let value = captures
                .name("value")
                .map(|m| m.as_str().trim())
                .unwrap_or_default();

            match key {
                "CodePath" => {
                    flush_pending_expectation(
                        doc_path,
                        &mut pending_code_path,
                        &mut pending_symbol,
                        &mut pending_signature,
                        &mut pending_kind,
                        &mut pending_rule_ids,
                        &mut pending_hash,
                        &mut pending_line,
                        index,
                    );
                    pending_code_path = Some(resolve_doc_relative_path(docs_root, doc_path, value));
                    pending_line = line_no;
                }
                "Symbol" => pending_symbol = Some(value.to_string()),
                "Signature" => pending_signature = Some(value.to_string()),
                "Kind" => pending_kind = parse_kind(value),
                "RuleID" => pending_rule_ids.push(value.to_string()),
                "StructuralHash" => pending_hash = Some(value.to_string()),
                _ => {}
            }
        }
    }

    flush_pending_expectation(
        doc_path,
        &mut pending_code_path,
        &mut pending_symbol,
        &mut pending_signature,
        &mut pending_kind,
        &mut pending_rule_ids,
        &mut pending_hash,
        &mut pending_line,
        index,
    );
}

#[allow(clippy::too_many_arguments)]
fn flush_pending_expectation(
    doc_path: &Path,
    pending_code_path: &mut Option<PathBuf>,
    pending_symbol: &mut Option<String>,
    pending_signature: &mut Option<String>,
    pending_kind: &mut Option<AstSymbolKind>,
    pending_rule_ids: &mut Vec<String>,
    pending_hash: &mut Option<String>,
    pending_line: &mut usize,
    index: &mut DocExpectationIndex,
) {
    let Some(code_path) = pending_code_path.take() else {
        return;
    };
    let Some(symbol_name) = pending_symbol.take() else {
        *pending_signature = None;
        *pending_kind = None;
        pending_rule_ids.clear();
        *pending_hash = None;
        *pending_line = 0;
        return;
    };

    index.symbol_expectations.push(DocSymbolExpectation {
        doc_path: doc_path.to_path_buf(),
        code_path,
        symbol_name,
        kind: pending_kind.take(),
        signature: pending_signature.take(),
        rule_ids: std::mem::take(pending_rule_ids),
        structural_hash: pending_hash.take(),
        line: *pending_line,
    });
    *pending_line = 0;
}

fn parse_kind(raw: &str) -> Option<AstSymbolKind> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "function" => Some(AstSymbolKind::Function),
        "class" => Some(AstSymbolKind::Class),
        "interface" => Some(AstSymbolKind::Interface),
        "type" | "type_alias" | "typealias" => Some(AstSymbolKind::TypeAlias),
        "re_export" | "reexport" => Some(AstSymbolKind::ReExport),
        _ => None,
    }
}

fn resolve_doc_relative_path(docs_root: &Path, doc_path: &Path, raw: &str) -> PathBuf {
    let raw = raw.trim().trim_matches('`');
    let path = Path::new(raw);
    if path.is_absolute() {
        return path.to_path_buf();
    }

    if raw.starts_with("./") || raw.starts_with("../") {
        return doc_path.parent().unwrap_or(docs_root).join(path);
    }

    docs_root.parent().unwrap_or(docs_root).join(path)
}

fn walk_markdown_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(walk_markdown_files(&path)?);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            files.push(path);
        }
    }
    Ok(files)
}

fn normalize_path(repo_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo_root.join(path)
    }
}

fn normalize_signature(raw: &str) -> String {
    raw.split_whitespace()
        .collect::<String>()
        .to_ascii_lowercase()
}

fn line_number_for_offset(content: &str, offset: usize) -> usize {
    content.as_bytes()[..offset.min(content.len())]
        .iter()
        .filter(|byte| **byte == b'\n')
        .count()
        + 1
}

fn classify_structural_domain(rule_ids: &[String]) -> DriftDomain {
    if rule_ids.is_empty() {
        DriftDomain::ApiSurface
    } else {
        DriftDomain::BusinessRule
    }
}

#[cfg(test)]
mod tests {
    use super::{CodeRealityIndex, DocExpectationIndex, DriftDetector};
    use crate::ast::IncrementalAstParser;
    use std::fs;

    #[test]
    fn flags_signature_and_rule_drift() {
        let root = tempfile::tempdir().expect("tempdir");
        let repo_root = root.path();
        let docs_root = repo_root.join("akta-docs");
        fs::create_dir_all(docs_root.join("03-business-logic")).expect("docs dir");

        let doc_path = docs_root.join("03-business-logic/rules.md");
        fs::write(
            &doc_path,
            r#"
CodePath: src/lib/rules.ts
Symbol: resolvePlan
Kind: function
Signature: export function resolvePlan(accountId: string): Promise<string>
RuleID: BR-001
"#,
        )
        .expect("write docs");

        let code_path = repo_root.join("src/lib/rules.ts");
        fs::create_dir_all(code_path.parent().expect("parent")).expect("src dir");
        fs::write(
            &code_path,
            r#"
/** @RuleID BR-999 */
export function resolvePlan(force: boolean): Promise<number> {
  return Promise.resolve(1);
}
"#,
        )
        .expect("write code");

        let docs = DocExpectationIndex::parse_from_root(&docs_root).expect("docs index");
        let source = fs::read_to_string(&code_path).expect("read code");
        let mut parser = IncrementalAstParser::new(1024 * 1024);
        let code =
            CodeRealityIndex::from_files(&mut parser, [(code_path.as_path(), source.as_str())])
                .expect("code index");

        let report = DriftDetector::detect(&docs, &code, repo_root);
        assert!(report.total_flags >= 2);
    }

    #[test]
    fn parses_fenced_expectation_blocks_and_doc_relative_paths() {
        let root = tempfile::tempdir().expect("tempdir");
        let repo_root = root.path();
        let docs_root = repo_root.join("akta-docs");
        let nested = docs_root.join("03-business-logic/payments");
        fs::create_dir_all(&nested).expect("docs dir");

        let doc_path = nested.join("rules.md");
        fs::write(
            &doc_path,
            r#"
```akta-expect
code_path: ./resolver.ts
symbol: resolvePlan
kind: function
signature: export function resolvePlan(accountId: string): Promise<string>
rule_ids:
  - BR-101
```

Reference: `src/lib/shared.ts#resolveShared`
"#,
        )
        .expect("write docs");

        let docs = DocExpectationIndex::parse_from_root(&docs_root).expect("docs index");
        assert_eq!(docs.symbol_expectations.len(), 1);
        assert_eq!(
            docs.symbol_expectations[0].code_path,
            nested.join("resolver.ts")
        );
        assert_eq!(docs.symbol_expectations[0].rule_ids, vec!["BR-101"]);
        assert_eq!(docs.code_references.len(), 1);
        assert_eq!(
            docs.code_references[0].code_path,
            repo_root.join("src/lib/shared.ts")
        );
    }

    #[test]
    fn keeps_legacy_markers_compatible() {
        let root = tempfile::tempdir().expect("tempdir");
        let repo_root = root.path();
        let docs_root = repo_root.join("akta-docs");
        fs::create_dir_all(docs_root.join("06-technical")).expect("docs dir");

        fs::write(
            docs_root.join("06-technical/api.md"),
            r#"
CodePath: src/lib/routes.ts
Symbol: GET
Kind: function
Signature: export async function GET(request: Request): Promise<Response>
RuleID: BR-202
"#,
        )
        .expect("write docs");

        let docs = DocExpectationIndex::parse_from_root(&docs_root).expect("docs index");
        assert_eq!(docs.symbol_expectations.len(), 1);
        let expectation = &docs.symbol_expectations[0];
        assert_eq!(expectation.symbol_name, "GET");
        assert_eq!(expectation.rule_ids, vec!["BR-202"]);
        assert_eq!(expectation.code_path, repo_root.join("src/lib/routes.ts"));
    }

    #[test]
    fn flags_dead_code_references_from_fenced_expectations() {
        let root = tempfile::tempdir().expect("tempdir");
        let repo_root = root.path();
        let docs_root = repo_root.join("akta-docs");
        fs::create_dir_all(docs_root.join("06-technical")).expect("docs dir");
        fs::write(
            docs_root.join("06-technical/refs.md"),
            "Reference: `src/api/missing.ts#GET`\n",
        )
        .expect("write docs");

        let docs = DocExpectationIndex::parse_from_root(&docs_root).expect("docs index");
        let report = DriftDetector::detect(&docs, &CodeRealityIndex::default(), repo_root);

        assert_eq!(report.total_flags, 1);
        assert_eq!(report.code_reference_flags, 1);
        assert!(report.flags[0].message.contains("missing file"));
    }
}
