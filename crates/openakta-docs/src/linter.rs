use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use regex::Regex;
use serde_yaml::{Mapping, Value};
use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use thiserror::Error;

const DOC_ID_PATTERN: &str = r"^[a-z0-9][a-z0-9._-]*$";
const ISO_DATE_PATTERN: &str =
    r"^\d{4}-\d{2}-\d{2}(?:T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})?)?$";
const QUICK_MIN_WORDS: usize = 40;
const QUICK_MAX_WORDS: usize = 80;
const SECTION_MIN_WORDS: usize = 150;
const SECTION_MAX_WORDS: usize = 300;
const QUESTION_RATIO_MIN: f64 = 0.70;
const DOC_TYPES: &[&str] = &[
    "adr",
    "business_rule",
    "feature",
    "guide",
    "reference",
    "explanation",
    "research",
    "meta",
    "changelog",
    "technical",
    "other",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warn,
}

impl Display for Severity {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error => f.write_str("error"),
            Self::Warn => f.write_str("warn"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RuleId {
    Meta001,
    Meta002,
    Meta003,
    Meta004,
    MetaQuick,
    Struct008,
    Content001,
}

impl RuleId {
    fn as_str(self) -> &'static str {
        match self {
            Self::Meta001 => "META-001",
            Self::Meta002 => "META-002",
            Self::Meta003 => "META-003",
            Self::Meta004 => "META-004",
            Self::MetaQuick => "META-QUICK",
            Self::Struct008 => "STRUCT-008",
            Self::Content001 => "CONTENT-001",
        }
    }
}

impl Display for RuleId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub end_line: Option<usize>,
    pub end_column: Option<usize>,
    pub rule_id: RuleId,
    pub severity: Severity,
    pub message: String,
    pub doc_url: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct LintSummary {
    pub error_count: usize,
    pub warn_count: usize,
}

#[derive(Debug, Clone, Default)]
pub struct LintResult {
    pub diagnostics: Vec<Diagnostic>,
    pub summary: LintSummary,
}

#[derive(Debug, Error)]
pub enum LintError {
    #[error("failed to read {path}: {source}")]
    ReadFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

pub struct MarkdownLinter;

impl MarkdownLinter {
    pub fn new_strict_geo() -> Self {
        Self
    }

    pub fn lint_file(&self, path: impl AsRef<Path>) -> Result<LintResult, LintError> {
        let path = path.as_ref();
        let raw = fs::read_to_string(path).map_err(|source| LintError::ReadFile {
            path: path.to_path_buf(),
            source,
        })?;
        Ok(self.lint_source(path, &raw))
    }

    pub fn lint_source(&self, path: impl AsRef<Path>, raw: &str) -> LintResult {
        let file = path.as_ref().to_path_buf();
        let mut diagnostics = Vec::new();
        let frontmatter = split_frontmatter(raw);

        lint_meta(&file, &frontmatter, &mut diagnostics);

        if !frontmatter.can_parse_body() {
            return finalize(diagnostics);
        }

        let body = frontmatter.body.unwrap_or("");
        let body_start_line = frontmatter.body_start_line;
        let body_index = LineIndex::new(body);
        let headings = collect_headings(body, body_start_line, &body_index);
        let quick_block = find_quick_code_after_h1(body, body_start_line, &body_index, &headings);

        lint_meta_quick(
            &file,
            &frontmatter,
            &headings,
            quick_block.as_ref(),
            &mut diagnostics,
        );
        lint_struct_008(&file, &frontmatter, body, &headings, &mut diagnostics);
        lint_content_001(&file, &headings, &mut diagnostics);

        finalize(diagnostics)
    }
}

impl Default for MarkdownLinter {
    fn default() -> Self {
        Self::new_strict_geo()
    }
}

pub fn format_diagnostics(result: &LintResult, workspace_root: Option<&Path>) -> String {
    let mut output = String::new();
    for diagnostic in &result.diagnostics {
        let display_path = workspace_root
            .and_then(|root| diagnostic.file.strip_prefix(root).ok())
            .map(Cow::Borrowed)
            .unwrap_or_else(|| Cow::Owned(diagnostic.file.clone()));
        output.push_str(&format!(
            "{}:{}:{}: {} [{}] {}\n",
            display_path.display(),
            diagnostic.line,
            diagnostic.column,
            diagnostic.severity,
            diagnostic.rule_id,
            diagnostic.message
        ));
    }
    output.push_str(&format!(
        "\n{} error(s), {} warning(s)\n",
        result.summary.error_count, result.summary.warn_count
    ));
    output
}

fn finalize(mut diagnostics: Vec<Diagnostic>) -> LintResult {
    diagnostics.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.line.cmp(&b.line))
            .then(a.column.cmp(&b.column))
            .then(a.rule_id.cmp(&b.rule_id))
    });

    LintResult {
        summary: LintSummary {
            error_count: diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.severity == Severity::Error)
                .count(),
            warn_count: diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.severity == Severity::Warn)
                .count(),
        },
        diagnostics,
    }
}

#[derive(Debug, Clone)]
struct FrontmatterSplit<'a> {
    has_frontmatter: bool,
    frontmatter: Option<Frontmatter>,
    frontmatter_error: Option<String>,
    body: Option<&'a str>,
    body_start_line: usize,
}

impl FrontmatterSplit<'_> {
    fn can_parse_body(&self) -> bool {
        self.has_frontmatter && self.frontmatter_error.is_none() && self.body.is_some()
    }
}

#[derive(Debug, Clone)]
struct Frontmatter {
    doc_id: Option<String>,
    doc_type: Option<String>,
    date: Option<String>,
}

fn split_frontmatter(raw: &str) -> FrontmatterSplit<'_> {
    if !raw.starts_with("---\n") && !raw.starts_with("---\r\n") {
        return FrontmatterSplit {
            has_frontmatter: false,
            frontmatter: None,
            frontmatter_error: None,
            body: None,
            body_start_line: 1,
        };
    }

    let yaml_start = raw.find('\n').map(|index| index + 1).unwrap_or(raw.len());
    let mut offset = yaml_start;
    for line in raw[offset..].split_inclusive('\n') {
        if line.trim_end_matches(['\r', '\n']) == "---" {
            let yaml = &raw[yaml_start..offset];
            let body_start = offset + line.len();
            let body_start_line = raw[..body_start]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                + 1;
            return match serde_yaml::from_str::<Value>(yaml) {
                Ok(Value::Mapping(mapping)) => FrontmatterSplit {
                    has_frontmatter: true,
                    frontmatter: Some(Frontmatter {
                        doc_id: yaml_string(&mapping, "doc_id"),
                        doc_type: yaml_string(&mapping, "doc_type"),
                        date: yaml_string(&mapping, "date"),
                    }),
                    frontmatter_error: None,
                    body: Some(&raw[body_start..]),
                    body_start_line,
                },
                Ok(_) => FrontmatterSplit {
                    has_frontmatter: true,
                    frontmatter: None,
                    frontmatter_error: Some("frontmatter root must be a YAML mapping".to_string()),
                    body: None,
                    body_start_line,
                },
                Err(error) => FrontmatterSplit {
                    has_frontmatter: true,
                    frontmatter: None,
                    frontmatter_error: Some(error.to_string()),
                    body: None,
                    body_start_line,
                },
            };
        }
        offset += line.len();
    }

    FrontmatterSplit {
        has_frontmatter: true,
        frontmatter: None,
        frontmatter_error: Some("frontmatter block is not closed with `---`".to_string()),
        body: None,
        body_start_line: 1,
    }
}

fn yaml_string(mapping: &Mapping, key: &str) -> Option<String> {
    mapping
        .get(Value::String(key.to_string()))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn lint_meta(file: &Path, frontmatter: &FrontmatterSplit<'_>, diagnostics: &mut Vec<Diagnostic>) {
    if !frontmatter.has_frontmatter {
        push_diag(
            diagnostics,
            file,
            RuleId::Meta001,
            Severity::Error,
            1,
            1,
            "Missing YAML frontmatter (expected leading --- block).".to_string(),
            None,
            None,
        );
        return;
    }

    if let Some(message) = &frontmatter.frontmatter_error {
        push_diag(
            diagnostics,
            file,
            RuleId::Meta001,
            Severity::Error,
            1,
            1,
            format!("Invalid frontmatter YAML: {message}"),
            None,
            None,
        );
        return;
    }

    let Some(frontmatter) = frontmatter.frontmatter.as_ref() else {
        return;
    };

    match frontmatter.doc_id.as_deref() {
        Some(doc_id) if doc_id_re().is_match(doc_id) => {}
        Some(doc_id) => push_diag(
            diagnostics,
            file,
            RuleId::Meta002,
            Severity::Error,
            2,
            1,
            format!("doc_id must match pattern {DOC_ID_PATTERN}: got \"{doc_id}\"."),
            None,
            None,
        ),
        None => push_diag(
            diagnostics,
            file,
            RuleId::Meta002,
            Severity::Error,
            2,
            1,
            "Frontmatter must include non-empty string `doc_id`.".to_string(),
            None,
            None,
        ),
    }

    match frontmatter.doc_type.as_deref() {
        Some(doc_type) if DOC_TYPES.contains(&doc_type) => {}
        Some(doc_type) => push_diag(
            diagnostics,
            file,
            RuleId::Meta003,
            Severity::Error,
            2,
            1,
            format!("Invalid doc_type \"{doc_type}\"."),
            None,
            None,
        ),
        None => push_diag(
            diagnostics,
            file,
            RuleId::Meta003,
            Severity::Error,
            2,
            1,
            format!(
                "Frontmatter must include string `doc_type` (one of: {}).",
                DOC_TYPES.join(", ")
            ),
            None,
            None,
        ),
    }

    match frontmatter.date.as_deref() {
        Some(date) if iso_date_re().is_match(date) => {}
        Some(date) => push_diag(
            diagnostics,
            file,
            RuleId::Meta004,
            Severity::Error,
            2,
            1,
            format!("date must be ISO8601: got \"{date}\"."),
            None,
            None,
        ),
        None => push_diag(
            diagnostics,
            file,
            RuleId::Meta004,
            Severity::Error,
            2,
            1,
            "Frontmatter must include ISO8601 `date` (YYYY-MM-DD or full instant).".to_string(),
            None,
            None,
        ),
    }
}

#[derive(Debug, Clone)]
struct HeadingInfo {
    depth: u8,
    text: String,
    start_offset: usize,
    end_offset: usize,
    line: usize,
    column: usize,
    end_line: usize,
    end_column: usize,
}

#[derive(Debug, Clone)]
struct QuickCodeBlock {
    text: String,
    line: usize,
    column: usize,
    end_line: usize,
    end_column: usize,
}

fn collect_headings(
    body: &str,
    body_start_line: usize,
    line_index: &LineIndex,
) -> Vec<HeadingInfo> {
    let mut headings = Vec::new();
    let mut active_heading: Option<(u8, usize, usize, usize, usize, String)> = None;

    for (event, range) in Parser::new_ext(body, Options::all()).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                let (line, column) = line_index.line_column(range.start, body_start_line);
                active_heading = Some((
                    heading_level_to_u8(level),
                    range.start,
                    range.end,
                    line,
                    column,
                    String::new(),
                ));
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((depth, start_offset, _start_end_offset, line, column, text)) =
                    active_heading.take()
                {
                    let (end_line, end_column) = line_index.line_column(range.end, body_start_line);
                    headings.push(HeadingInfo {
                        depth,
                        text: text.trim().to_string(),
                        start_offset,
                        end_offset: range.end,
                        line,
                        column,
                        end_line,
                        end_column,
                    });
                }
            }
            Event::Text(text) | Event::Code(text) | Event::Html(text) | Event::InlineHtml(text) => {
                if let Some((_, _, _, _, _, heading_text)) = active_heading.as_mut() {
                    heading_text.push_str(&text);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some((_, _, _, _, _, heading_text)) = active_heading.as_mut() {
                    heading_text.push(' ');
                }
            }
            _ => {}
        }
    }

    headings
}

fn find_quick_code_after_h1(
    body: &str,
    body_start_line: usize,
    line_index: &LineIndex,
    headings: &[HeadingInfo],
) -> Option<QuickCodeBlock> {
    let h1 = headings.iter().find(|heading| heading.depth == 1)?;
    let after_h1 = body.get(h1.end_offset..)?;
    let mut local_offset = 0usize;
    let mut lines = after_h1.split_inclusive('\n').peekable();

    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            local_offset += line.len();
            continue;
        }
        if trimmed.starts_with('#') {
            return None;
        }
        if !trimmed.starts_with("```quick") {
            return None;
        }

        let start_offset = h1.end_offset + local_offset;
        let (line_no, column) = line_index.line_column(start_offset, body_start_line);
        let mut content = String::new();
        let mut consumed_len = line.len();
        let mut end_offset = start_offset + line.len();
        let mut closed = false;

        for code_line in lines {
            let code_trimmed = code_line.trim_end_matches(['\r', '\n']);
            consumed_len += code_line.len();
            end_offset = start_offset + consumed_len;
            if code_trimmed == "```" {
                closed = true;
                break;
            }
            content.push_str(code_line);
        }

        if !closed {
            return None;
        }

        let (end_line, end_column) = line_index.line_column(end_offset, body_start_line);
        return Some(QuickCodeBlock {
            text: content,
            line: line_no,
            column,
            end_line,
            end_column,
        });
    }

    None
}

fn lint_meta_quick(
    file: &Path,
    frontmatter: &FrontmatterSplit<'_>,
    headings: &[HeadingInfo],
    quick_block: Option<&QuickCodeBlock>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(frontmatter) = frontmatter.frontmatter.as_ref() else {
        return;
    };
    if frontmatter.doc_type.as_deref() == Some("changelog") {
        return;
    }

    let line = headings
        .iter()
        .find(|heading| heading.depth == 1)
        .map(|heading| heading.line + 1)
        .unwrap_or(frontmatter_line_offset(frontmatter) + 1);

    let Some(quick_block) = quick_block else {
        push_diag(
            diagnostics,
            file,
            RuleId::MetaQuick,
            Severity::Error,
            line,
            1,
            "After the first H1, expected a fenced code block with language tag `quick` (```quick)."
                .to_string(),
            None,
            None,
        );
        return;
    };

    let words = count_words(&quick_block.text);
    if !(QUICK_MIN_WORDS..=QUICK_MAX_WORDS).contains(&words) {
        push_diag(
            diagnostics,
            file,
            RuleId::MetaQuick,
            Severity::Error,
            quick_block.line,
            quick_block.column,
            format!(
                "Quick Answer block must be {QUICK_MIN_WORDS}–{QUICK_MAX_WORDS} words; found {words}."
            ),
            Some(quick_block.end_line),
            Some(quick_block.end_column),
        );
    }
}

fn lint_struct_008(
    file: &Path,
    frontmatter_split: &FrontmatterSplit<'_>,
    body: &str,
    headings: &[HeadingInfo],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(frontmatter) = frontmatter_split.frontmatter.as_ref() else {
        return;
    };
    if frontmatter.doc_type.as_deref() == Some("changelog") {
        return;
    }

    let section_headings = headings
        .iter()
        .filter(|heading| heading.depth == 2 || heading.depth == 3)
        .collect::<Vec<_>>();
    let body_lines = body.split('\n').collect::<Vec<_>>();

    for (index, heading) in section_headings.iter().enumerate() {
        let content_start_line = heading.end_line + 1;
        let next_heading = section_headings[index + 1..].iter().find(|candidate| {
            candidate.start_offset > heading.end_offset && candidate.depth <= heading.depth
        });
        let content_end_line = next_heading
            .map(|candidate| candidate.line.saturating_sub(1))
            .unwrap_or(frontmatter_split.body_start_line + body_lines.len().saturating_sub(1));

        if content_start_line > content_end_line {
            push_diag(
                diagnostics,
                file,
                RuleId::Struct008,
                Severity::Error,
                heading.line,
                heading.column,
                "Section has no body; cannot satisfy STRUCT-008 word range.".to_string(),
                Some(heading.end_line),
                Some(heading.end_column),
            );
            continue;
        }

        let start_index = content_start_line.saturating_sub(frontmatter_split.body_start_line);
        let end_index = content_end_line.saturating_sub(frontmatter_split.body_start_line);
        let slice = body_lines
            .get(start_index..=end_index)
            .map(|lines| lines.join("\n"))
            .unwrap_or_default();
        let words = count_words(&slice);

        if !(SECTION_MIN_WORDS..=SECTION_MAX_WORDS).contains(&words) {
            push_diag(
                diagnostics,
                file,
                RuleId::Struct008,
                Severity::Error,
                heading.line,
                heading.column,
                format!(
                    "H{} section must be {SECTION_MIN_WORDS}–{SECTION_MAX_WORDS} words; found {words}.",
                    heading.depth
                ),
                Some(heading.end_line),
                Some(heading.end_column),
            );
        }
    }
}

fn lint_content_001(file: &Path, headings: &[HeadingInfo], diagnostics: &mut Vec<Diagnostic>) {
    let headings = headings
        .iter()
        .filter(|heading| heading.depth == 2 || heading.depth == 3)
        .collect::<Vec<_>>();
    if headings.is_empty() {
        return;
    }

    let question_count = headings
        .iter()
        .filter(|heading| is_question_heading(&heading.text))
        .count();
    let actual_ratio = question_count as f64 / headings.len() as f64;
    if actual_ratio + 1e-9 < QUESTION_RATIO_MIN {
        push_diag(
            diagnostics,
            file,
            RuleId::Content001,
            Severity::Error,
            headings[0].line,
            headings[0].column,
            format!(
                "At least {:.0}% of H2/H3 headings should be questions; got {:.1}% ({}/{}).",
                QUESTION_RATIO_MIN * 100.0,
                actual_ratio * 100.0,
                question_count,
                headings.len()
            ),
            None,
            None,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_diag(
    diagnostics: &mut Vec<Diagnostic>,
    file: &Path,
    rule_id: RuleId,
    severity: Severity,
    line: usize,
    column: usize,
    message: String,
    end_line: Option<usize>,
    end_column: Option<usize>,
) {
    diagnostics.push(Diagnostic {
        file: file.to_path_buf(),
        line,
        column,
        end_line,
        end_column,
        rule_id,
        severity,
        message,
        doc_url: None,
    });
}

fn count_words(text: &str) -> usize {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return 0;
    }
    trimmed.split_whitespace().count()
}

fn is_question_heading(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.ends_with('?') || question_heading_re().is_match(trimmed)
}

fn heading_level_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn frontmatter_line_offset(_frontmatter: &Frontmatter) -> usize {
    1
}

fn doc_id_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(DOC_ID_PATTERN).expect("doc_id regex"))
}

fn iso_date_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(ISO_DATE_PATTERN).expect("iso date regex"))
}

fn question_heading_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)^(how|what|when|where|why|who|which|can|should|does|is|are)\b")
            .expect("question heading regex")
    })
}

#[derive(Debug, Clone)]
struct LineIndex {
    line_starts: Vec<usize>,
}

impl LineIndex {
    fn new(input: &str) -> Self {
        let mut line_starts = vec![0];
        for (index, byte) in input.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(index + 1);
            }
        }
        Self { line_starts }
    }

    fn line_column(&self, offset: usize, first_line: usize) -> (usize, usize) {
        let line_index = match self.line_starts.binary_search(&offset) {
            Ok(index) => index,
            Err(index) => index.saturating_sub(1),
        };
        let line_start = self.line_starts.get(line_index).copied().unwrap_or(0);
        (
            first_line + line_index,
            offset.saturating_sub(line_start) + 1,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{format_diagnostics, MarkdownLinter, RuleId};
    use std::fs;
    use std::path::{Path, PathBuf};

    #[test]
    fn ts_reference_fixture_no_frontmatter_reports_meta001() {
        let file = fixture_path("no-frontmatter.md");
        let linter = MarkdownLinter::new_strict_geo();
        let result = linter.lint_file(&file).expect("lint");
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.rule_id == RuleId::Meta001));
    }

    #[test]
    fn ts_reference_fixture_compliant_passes() {
        let file = fixture_path("compliant.md");
        let linter = MarkdownLinter::new_strict_geo();
        let result = linter.lint_file(&file).expect("lint");
        let rendered = format_diagnostics(&result, None);
        assert!(
            result
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.severity != super::Severity::Error),
            "{rendered}"
        );
    }

    #[test]
    fn ts_reference_fixture_short_section_reports_struct008() {
        let file = fixture_path("short-section.md");
        let linter = MarkdownLinter::new_strict_geo();
        let result = linter.lint_file(&file).expect("lint");
        assert!(result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.rule_id == RuleId::Struct008));
    }

    #[test]
    fn unclosed_frontmatter_stops_body_parsing() {
        let linter = MarkdownLinter::new_strict_geo();
        let source = "---\ndoc_id: test.bad\ndoc_type: technical\n# fake heading in yaml\n";
        let result = linter.lint_source(Path::new("akta-docs/bad.md"), source);
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].rule_id, RuleId::Meta001);
    }

    #[test]
    fn quick_block_line_number_is_exact() {
        let linter = MarkdownLinter::new_strict_geo();
        let source = "\
---\n\
doc_id: test.quick\n\
doc_type: technical\n\
date: 2025-03-21\n\
---\n\
\n\
# Quick offset\n\
\n\
\n\
```quick\n\
too short\n\
```\n";
        let result = linter.lint_source(Path::new("akta-docs/quick.md"), source);
        let diagnostic = result
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.rule_id == RuleId::MetaQuick)
            .expect("meta quick diagnostic");
        assert_eq!(diagnostic.line, 10);
    }

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../sdks/akta-docs/typescript/tests/fixtures")
            .join(name)
    }

    #[test]
    fn fixture_directory_exists() {
        assert!(fs::metadata(fixture_path("compliant.md")).is_ok());
    }
}
