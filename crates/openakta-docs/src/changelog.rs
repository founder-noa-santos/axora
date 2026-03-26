//! TOON-style changelog payloads and deterministic Markdown append (Plan 5 / Plan 7).

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Core + extended migration taxonomy (strict serde snake_case).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationChangeKind {
    Added,
    Changed,
    Deprecated,
    Removed,
    Fixed,
    Security,
    Refactored,
    Rollback,
}

/// Scaffolding metadata for deterministic migration filenames (not required in every TOON payload).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationMetadata {
    /// ISO 8601 compact UTC: `YYYYMMDDHHMMSS` (14 digits), validated as a real calendar datetime.
    pub ts: String,
    pub doc_id: String,
    pub change_kind: MigrationChangeKind,
    pub slug: String,
}

impl MigrationMetadata {
    /// Validates timestamp and non-empty segments after sanitization.
    pub fn validate(&self) -> Result<(), ChangelogValidationError> {
        if !validate_ts_compact_iso8601_utc(&self.ts) {
            return Err(ChangelogValidationError::InvalidTimestamp);
        }
        let doc = sanitize_segment(&self.doc_id, MIGRATION_SEGMENT_MAX);
        let slug = sanitize_segment(&self.slug, MIGRATION_SEGMENT_MAX);
        if doc.is_empty() || slug.is_empty() {
            return Err(ChangelogValidationError::InvalidMigrationSegment);
        }
        Ok(())
    }

    /// `{timestamp}_{doc_id}_{type}_{slug}.md` with sanitized segments.
    pub fn migration_filename(&self) -> Result<String, AppendChangelogError> {
        self.validate()?;
        let doc = sanitize_segment(&self.doc_id, MIGRATION_SEGMENT_MAX);
        let slug = sanitize_segment(&self.slug, MIGRATION_SEGMENT_MAX);
        if doc.is_empty() || slug.is_empty() {
            return Err(AppendChangelogError::MissingMigrationFields);
        }
        Ok(format!(
            "{}_{}_{}_{}.md",
            self.ts,
            doc,
            migration_kind_snake_case(self.change_kind),
            slug
        ))
    }
}

/// Compact changelog payload (LLM emits JSON; Rust validates and applies).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToonChangelogPayload {
    /// Schema version.
    pub v: u8,
    /// ISO 8601 compact UTC: `YYYYMMDDHHMMSS` (14 digits).
    pub ts: String,
    #[serde(rename = "ty")]
    pub change_type: MigrationChangeKind,
    /// Short human description; single line (no newlines).
    pub d: String,
    /// Optional stable doc id segment for `10-changelog/` filenames (included in checksum when set).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    /// Optional slug segment for migration filenames (included in checksum when set).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    /// Exactly 16 lowercase hex chars: first 128 bits of SHA-256 over [`ToonChangelogPayload::canonical_json_without_checksum`].
    pub sha256_16: String,
}

#[derive(Debug, Error)]
pub enum ChangelogValidationError {
    #[error(
        "timestamp must be 14 ASCII digits (compact ISO 8601 UTC) and a valid calendar datetime"
    )]
    InvalidTimestamp,
    #[error("description must not contain newlines")]
    InvalidDescription,
    #[error("doc/slug migration segments invalid or empty after sanitization")]
    InvalidMigrationSegment,
}

#[derive(Debug, Error)]
pub enum AppendChangelogError {
    #[error("checksum mismatch on TOON payload")]
    ChecksumMismatch,
    #[error("invalid timestamp on TOON payload")]
    InvalidTimestamp,
    #[error("invalid TOON payload: {0}")]
    InvalidPayload(#[from] ChangelogValidationError),
    #[error("doc and slug are required for external migration files")]
    MissingMigrationFields,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum ToonPayloadParseError {
    #[error("yaml: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

/// Parse a TOON YAML document into a payload (checksum not verified; call [`ToonChangelogPayload::verify_checksum`]).
pub fn parse_toon_payload_yaml(s: &str) -> Result<ToonChangelogPayload, ToonPayloadParseError> {
    Ok(serde_yaml::from_str(s)?)
}

const MARKER: &str = "<!-- openakta:changelog -->";
const MIGRATION_SEGMENT_MAX: usize = 64;

impl ToonChangelogPayload {
    /// Build a payload and compute `sha256_16` deterministically. `doc` / `slug` are sanitized when `Some`.
    pub fn from_parts(
        v: u8,
        ts: String,
        change_type: MigrationChangeKind,
        d: String,
        doc: Option<String>,
        slug: Option<String>,
    ) -> Result<Self, ChangelogValidationError> {
        if !validate_ts_compact_iso8601_utc(&ts) {
            return Err(ChangelogValidationError::InvalidTimestamp);
        }
        if d.contains('\n') || d.contains('\r') {
            return Err(ChangelogValidationError::InvalidDescription);
        }
        let doc = doc.map(|s| sanitize_segment(&s, MIGRATION_SEGMENT_MAX));
        let slug = slug.map(|s| sanitize_segment(&s, MIGRATION_SEGMENT_MAX));
        if doc.as_ref().is_some_and(|s| s.is_empty()) || slug.as_ref().is_some_and(|s| s.is_empty())
        {
            return Err(ChangelogValidationError::InvalidMigrationSegment);
        }
        let mut p = Self {
            v,
            ts,
            change_type,
            d,
            doc,
            slug,
            sha256_16: String::new(),
        };
        p.sha256_16 = compute_sha256_16(&p.canonical_json_without_checksum());
        Ok(p)
    }

    /// Canonical JSON for checksum input (excludes `sha256_16`).
    pub fn canonical_json_without_checksum(&self) -> String {
        #[derive(Serialize)]
        struct Body<'a> {
            v: u8,
            ts: &'a str,
            #[serde(rename = "ty")]
            ty: MigrationChangeKind,
            d: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            doc: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            slug: Option<&'a str>,
        }
        let b = Body {
            v: self.v,
            ts: &self.ts,
            ty: self.change_type,
            d: &self.d,
            doc: self.doc.as_deref(),
            slug: self.slug.as_deref(),
        };
        serde_json::to_string(&b).expect("serialize TOON body")
    }

    pub fn verify_checksum(&self) -> bool {
        verify_sha256_16(&self.canonical_json_without_checksum(), &self.sha256_16)
    }

    pub fn validate_ts_compact(&self) -> bool {
        validate_ts_compact_iso8601_utc(&self.ts)
    }
}

/// First 16 **lowercase** hex characters of SHA-256 (128 bits), no allocation of full digest hex.
pub fn compute_sha256_16(canonical_body: &str) -> String {
    let digest = Sha256::digest(canonical_body.as_bytes());
    let mut s = String::with_capacity(16);
    for byte in digest.iter().take(8) {
        use core::fmt::Write as _;
        write!(&mut s, "{:02x}", byte).expect("capacity");
    }
    s
}

/// Verifies `sha256_16` is exactly 16 hex digits matching the first 128 bits of SHA-256(canonical_body).
pub fn verify_sha256_16(canonical_body: &str, sha256_16: &str) -> bool {
    if sha256_16.len() != 16 || !sha256_16.chars().all(|c| c.is_ascii_hexdigit()) {
        return false;
    }
    let digest = Sha256::digest(canonical_body.as_bytes());
    let mut expected = String::with_capacity(16);
    for byte in digest.iter().take(8) {
        use core::fmt::Write as _;
        write!(&mut expected, "{:02x}", byte).expect("capacity");
    }
    expected == sha256_16.to_ascii_lowercase()
}

/// `YYYYMMDDHHMMSS` (14 digits), UTC-naive wall time, must parse as a valid [`NaiveDateTime`].
fn validate_ts_compact_iso8601_utc(ts: &str) -> bool {
    if ts.len() != 14 || !ts.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    NaiveDateTime::parse_from_str(ts, "%Y%m%d%H%M%S").is_ok()
}

/// Deterministic filesystem-safe segment: lowercase, `[a-z0-9_-]`, collapsed dashes, max length.
pub fn sanitize_segment(s: &str, max: usize) -> String {
    let lower = s.to_lowercase();
    let mut out = String::with_capacity(lower.len());
    let mut prev_dash = false;
    for ch in lower.chars() {
        let mapped = match ch {
            'a'..='z' | '0'..='9' => Some(ch),
            '_' | '-' => Some(ch),
            ' ' | '/' | '\\' => Some('-'),
            _ => None,
        };
        if let Some(c) = mapped {
            if c == '-' && prev_dash {
                continue;
            }
            prev_dash = c == '-';
            out.push(c);
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.len() > max {
        out[..max].trim_end_matches('-').to_string()
    } else {
        out
    }
}

fn migration_kind_snake_case(k: MigrationChangeKind) -> &'static str {
    match k {
        MigrationChangeKind::Added => "added",
        MigrationChangeKind::Changed => "changed",
        MigrationChangeKind::Deprecated => "deprecated",
        MigrationChangeKind::Removed => "removed",
        MigrationChangeKind::Fixed => "fixed",
        MigrationChangeKind::Security => "security",
        MigrationChangeKind::Refactored => "refactored",
        MigrationChangeKind::Rollback => "rollback",
    }
}

/// `{timestamp}_{doc_id}_{type}_{slug}.md` — requires `doc` and `slug`; segments are sanitized for path safety.
pub fn migration_filename(payload: &ToonChangelogPayload) -> Result<String, AppendChangelogError> {
    let Some(ref doc) = payload.doc else {
        return Err(AppendChangelogError::MissingMigrationFields);
    };
    let Some(ref slug) = payload.slug else {
        return Err(AppendChangelogError::MissingMigrationFields);
    };
    let doc_s = sanitize_segment(doc, MIGRATION_SEGMENT_MAX);
    let slug_s = sanitize_segment(slug, MIGRATION_SEGMENT_MAX);
    if doc_s.is_empty() || slug_s.is_empty() {
        return Err(AppendChangelogError::MissingMigrationFields);
    }
    Ok(format!(
        "{}_{}_{}_{}.md",
        payload.ts,
        doc_s,
        migration_kind_snake_case(payload.change_type),
        slug_s
    ))
}

/// Workspace-relative path: `10-changelog/{migration_filename}`.
pub fn migration_relative_path(
    payload: &ToonChangelogPayload,
) -> Result<String, AppendChangelogError> {
    let name = migration_filename(payload)?;
    Ok(format!("10-changelog/{name}"))
}

/// Deprecated: use [`migration_relative_path`].
#[deprecated(note = "use migration_relative_path")]
pub fn external_changelog_relative_path(payload: &ToonChangelogPayload) -> String {
    migration_relative_path(payload)
        .unwrap_or_else(|_| format!("10-changelog/{}_{}.md", payload.ts, payload.sha256_16))
}

/// Writes under `docs_root/10-changelog/` using `{timestamp}_{doc_id}_{type}_{slug}.md`.
/// **Roll-forward:** if the file already exists, appends a new bullet line instead of truncating.
///
/// Returns the **absolute** path to the fragment file.
pub fn write_external_migration_file(
    docs_root: &Path,
    payload: &ToonChangelogPayload,
) -> Result<PathBuf, AppendChangelogError> {
    if !payload.validate_ts_compact() {
        return Err(AppendChangelogError::InvalidTimestamp);
    }
    if !payload.verify_checksum() {
        return Err(AppendChangelogError::ChecksumMismatch);
    }
    let line = changelog_entry_line(payload)?;
    let subdir = docs_root.join("10-changelog");
    fs::create_dir_all(&subdir)?;
    let rel = migration_relative_path(payload)?;
    let name = rel
        .strip_prefix("10-changelog/")
        .ok_or(AppendChangelogError::MissingMigrationFields)?;
    let path = subdir.join(name);

    if path.exists() {
        let mut f = OpenOptions::new().append(true).open(&path)?;
        writeln!(f, "{line}")?;
    } else {
        let body = format!("# Changelog fragment\n\n{line}\n");
        fs::write(&path, body)?;
    }
    Ok(path)
}

/// Deprecated: use [`write_external_migration_file`].
#[deprecated(note = "use write_external_migration_file")]
pub fn write_external_changelog_file(
    docs_root: &Path,
    payload: &ToonChangelogPayload,
) -> Result<PathBuf, AppendChangelogError> {
    write_external_migration_file(docs_root, payload)
}

/// One Markdown bullet line for this payload (validates checksum and timestamp).
pub fn changelog_entry_line(
    payload: &ToonChangelogPayload,
) -> Result<String, AppendChangelogError> {
    if !payload.validate_ts_compact() {
        return Err(AppendChangelogError::InvalidTimestamp);
    }
    if !payload.verify_checksum() {
        return Err(AppendChangelogError::ChecksumMismatch);
    }
    let ty = migration_kind_snake_case(payload.change_type);
    let extra = match (&payload.doc, &payload.slug) {
        (Some(d), Some(s)) if !d.is_empty() && !s.is_empty() => format!(" doc=`{d}` slug=`{s}`"),
        _ => String::new(),
    };
    Ok(format!(
        "- `{}` [{}] {} — `{}`{}",
        payload.ts, ty, payload.d, payload.sha256_16, extra
    ))
}

/// Injects one bullet line immediately after the first `MARKER` without rewriting other bytes.
/// Existing lines after the marker are preserved unchanged (roll-forward; prior entries not edited).
/// New entry is placed directly under the marker (newest-first within the section).
///
/// When `docs_root_for_external` is `Some`, writes/append to [`write_external_migration_file`] and returns `existing_md` unchanged.
pub fn append_changelog_entry(
    existing_md: &str,
    payload: &ToonChangelogPayload,
    docs_root_for_external: Option<&Path>,
) -> Result<String, AppendChangelogError> {
    if !payload.validate_ts_compact() {
        return Err(AppendChangelogError::InvalidTimestamp);
    }
    if !payload.verify_checksum() {
        return Err(AppendChangelogError::ChecksumMismatch);
    }

    if let Some(docs_root) = docs_root_for_external {
        write_external_migration_file(docs_root, payload)?;
        return Ok(existing_md.to_string());
    }

    let line = changelog_entry_line(payload)?;

    if let Some(idx) = existing_md.find(MARKER) {
        let after_marker = idx + MARKER.len();
        let mut out = String::with_capacity(existing_md.len() + line.len() + 8);
        out.push_str(&existing_md[..after_marker]);
        if !existing_md[after_marker..].starts_with('\n') {
            out.push('\n');
        }
        out.push_str(&line);
        out.push('\n');
        out.push_str(&existing_md[after_marker..]);
        Ok(out)
    } else {
        let mut out = existing_md.to_string();
        out.push('\n');
        out.push_str(MARKER);
        out.push('\n');
        out.push_str(&line);
        out.push('\n');
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn checksum_round_trip() {
        let p = ToonChangelogPayload::from_parts(
            1,
            "20250716143022".into(),
            MigrationChangeKind::Changed,
            "synced doc refs".into(),
            None,
            None,
        )
        .unwrap();
        assert!(p.verify_checksum());
        assert_eq!(p.sha256_16.len(), 16);
    }

    #[test]
    fn rejects_invalid_calendar_timestamp() {
        assert!(ToonChangelogPayload::from_parts(
            1,
            "20250230120000".into(),
            MigrationChangeKind::Changed,
            "x".into(),
            None,
            None,
        )
        .is_err());
    }

    #[test]
    fn verify_rejects_wrong_checksum_length() {
        assert!(!verify_sha256_16("{}", "abcd"));
    }

    #[test]
    fn verify_rejects_non_hex_checksum() {
        let body = r#"{"v":1,"ts":"20250101120000","ty":"changed","d":"x"}"#;
        assert!(!verify_sha256_16(body, "gggggggggggggggg"));
    }

    #[test]
    fn append_inserts_under_marker() {
        let p = ToonChangelogPayload::from_parts(
            1,
            "20250716143022".into(),
            MigrationChangeKind::Fixed,
            "api drift".into(),
            None,
            None,
        )
        .unwrap();
        let md = "# Title\n\n<!-- openakta:changelog -->\n\nold\n";
        let out = append_changelog_entry(md, &p, None).unwrap();
        assert!(out.contains("20250716143022"));
        assert!(out.contains("fixed"));
        assert!(out.contains("# Title"));
    }

    #[test]
    fn rejects_bad_checksum() {
        let mut p = ToonChangelogPayload::from_parts(
            1,
            "20250716143022".into(),
            MigrationChangeKind::Changed,
            "x".into(),
            None,
            None,
        )
        .unwrap();
        p.sha256_16 = "deadbeefdeadbeef".into();
        assert!(append_changelog_entry("x", &p, None).is_err());
    }

    #[test]
    fn external_changelog_writes_under_10_changelog() {
        let tmp = tempfile::tempdir().unwrap();
        let docs = tmp.path().join("akta-docs");
        let p = ToonChangelogPayload::from_parts(
            1,
            "20250716143022".into(),
            MigrationChangeKind::Changed,
            "ext test".into(),
            Some("api-docs".into()),
            Some("sync-refs".into()),
        )
        .unwrap();
        let abs = write_external_migration_file(&docs, &p).unwrap();
        assert!(abs.exists());
        assert!(abs.to_string_lossy().contains("10-changelog"));
        let rel = migration_relative_path(&p).unwrap();
        assert!(rel.starts_with("10-changelog/"));
        assert!(rel.contains("api-docs"));
        assert!(rel.contains("changed"));
        assert!(rel.contains("sync-refs"));
    }

    #[test]
    fn external_migration_append_does_not_truncate_existing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let docs = tmp.path().join("akta-docs");
        let sub = docs.join("10-changelog");
        fs::create_dir_all(&sub).unwrap();
        let p1 = ToonChangelogPayload::from_parts(
            1,
            "20250716143022".into(),
            MigrationChangeKind::Changed,
            "first".into(),
            Some("doc".into()),
            Some("s".into()),
        )
        .unwrap();
        let path = write_external_migration_file(&docs, &p1).unwrap();
        let first = fs::read_to_string(&path).unwrap();
        assert!(first.contains("first"));

        let p2 = ToonChangelogPayload::from_parts(
            1,
            "20250716143022".into(),
            MigrationChangeKind::Changed,
            "second".into(),
            Some("doc".into()),
            Some("s".into()),
        )
        .unwrap();
        write_external_migration_file(&docs, &p2).unwrap();
        let both = fs::read_to_string(&path).unwrap();
        assert!(
            both.contains("first"),
            "roll-forward must retain prior fragment content"
        );
        assert!(both.contains("second"));
    }

    #[test]
    fn append_with_external_docs_root_does_not_mutate_inline_and_writes_fragment_file() {
        let tmp = tempfile::tempdir().unwrap();
        let docs = tmp.path().join("akta-docs");
        let p = ToonChangelogPayload::from_parts(
            1,
            "20250716143022".into(),
            MigrationChangeKind::Security,
            "external append mode".into(),
            Some("rules".into()),
            Some("ext-mode".into()),
        )
        .unwrap();
        let md = "# Title\nno marker here\n";
        let out = append_changelog_entry(md, &p, Some(&docs)).unwrap();
        assert_eq!(
            out, md,
            "inline markdown must be unchanged in external mode"
        );
        let fragment = docs.join(migration_relative_path(&p).unwrap());
        assert!(fragment.is_file(), "expected fragment at {:?}", fragment);
        let text = fs::read_to_string(&fragment).unwrap();
        assert!(text.contains("external append mode"));
        assert!(text.contains("20250716143022"));
        assert!(text.contains("security"));
        assert!(text.contains(&p.sha256_16));
    }

    #[test]
    fn append_inline_inserts_marker_block_when_marker_absent() {
        let p = ToonChangelogPayload::from_parts(
            1,
            "20260101120000".into(),
            MigrationChangeKind::Refactored,
            "note".into(),
            None,
            None,
        )
        .unwrap();
        let md = "# Doc\n\nBody.\n";
        let out = append_changelog_entry(md, &p, None).unwrap();
        assert!(out.contains(super::MARKER));
        assert!(out.contains("# Doc"));
        assert!(out.contains("refactored"));
    }

    #[test]
    fn serde_json_rejects_unknown_fields_on_payload() {
        let bad = r#"{"v":1,"ts":"20250101120000","ty":"changed","d":"x","sha256_16":"0123456789abcdef","extra":true}"#;
        assert!(
            serde_json::from_str::<ToonChangelogPayload>(bad).is_err(),
            "deny_unknown_fields must reject extra keys"
        );
    }

    #[test]
    fn canonical_json_stable_for_checksum() {
        let p = ToonChangelogPayload::from_parts(
            1,
            "20250716143022".into(),
            MigrationChangeKind::Changed,
            "stable".into(),
            None,
            None,
        )
        .unwrap();
        let c1 = p.canonical_json_without_checksum();
        let c2 = p.canonical_json_without_checksum();
        assert_eq!(c1, c2);
        assert!(p.verify_checksum());
    }

    #[test]
    fn checksum_changes_when_doc_tampered() {
        let mut p = ToonChangelogPayload::from_parts(
            1,
            "20250716143022".into(),
            MigrationChangeKind::Added,
            "x".into(),
            Some("doc1".into()),
            Some("s".into()),
        )
        .unwrap();
        assert!(p.verify_checksum());
        p.doc = Some("doc2".into());
        assert!(!p.verify_checksum());
    }

    #[test]
    fn parse_yaml_round_trip() {
        let p = ToonChangelogPayload::from_parts(
            1,
            "20250101120000".into(),
            MigrationChangeKind::Deprecated,
            "yaml test".into(),
            None,
            None,
        )
        .unwrap();
        let yaml = serde_yaml::to_string(&p).unwrap();
        let q: ToonChangelogPayload = parse_toon_payload_yaml(&yaml).unwrap();
        assert_eq!(p, q);
        assert!(q.verify_checksum());
    }

    #[test]
    fn migration_metadata_filename() {
        let m = MigrationMetadata {
            ts: "20250716143022".into(),
            doc_id: "My Doc!".into(),
            change_kind: MigrationChangeKind::Fixed,
            slug: "bug-1".into(),
        };
        let name = m.migration_filename().unwrap();
        assert_eq!(name, "20250716143022_my-doc_fixed_bug-1.md");
    }
}
