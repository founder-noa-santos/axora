//! Incremental AST extraction for LivingDocs drift detection.

use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tree_sitter::{InputEdit, Node, Parser, Point, Tree};

/// Result type for AST extraction.
pub type Result<T> = std::result::Result<T, AstError>;

/// AST extraction error.
#[derive(Debug, thiserror::Error)]
pub enum AstError {
    /// Unsupported source language.
    #[error("unsupported source language for {0}")]
    UnsupportedLanguage(String),

    /// Tree-sitter parser rejected the configured language.
    #[error("failed to set tree-sitter language for {0}")]
    LanguageSetup(&'static str),

    /// Parsing did not produce a syntax tree.
    #[error("failed to parse {0}")]
    ParseFailed(String),
}

/// Supported source languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceLanguage {
    /// TypeScript.
    TypeScript,
    /// TSX / React / Next.js component files.
    Tsx,
}

impl SourceLanguage {
    /// Detect language from file extension.
    pub fn detect(path: &Path) -> Result<Self> {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("ts") => Ok(Self::TypeScript),
            Some("tsx") | Some("jsx") | Some("js") => Ok(Self::Tsx),
            _ => Err(AstError::UnsupportedLanguage(path.display().to_string())),
        }
    }

    fn parser_name(self) -> &'static str {
        match self {
            Self::TypeScript => "typescript",
            Self::Tsx => "tsx",
        }
    }
}

/// Span inside a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AstSpan {
    /// 1-based start line.
    pub start_line: usize,
    /// 1-based end line.
    pub end_line: usize,
    /// 1-based start column.
    pub start_column: usize,
    /// 1-based end column.
    pub end_column: usize,
}

impl AstSpan {
    fn from_node(node: Node<'_>) -> Self {
        let start = node.start_position();
        let end = node.end_position();
        Self {
            start_line: start.row + 1,
            end_line: end.row + 1,
            start_column: start.column + 1,
            end_column: end.column + 1,
        }
    }
}

/// Machine-verifiable annotation tag extracted from doc comments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnotationTag {
    /// Tag key without the `@`.
    pub name: String,
    /// Tag value trimmed to a single line.
    pub value: String,
}

/// Supported code symbol kinds for drift checks.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AstSymbolKind {
    /// Function declaration or exported callable binding.
    Function,
    /// Class declaration.
    Class,
    /// Interface declaration.
    Interface,
    /// Type alias declaration.
    TypeAlias,
    /// Re-exported symbol exposed through a barrel or export clause.
    ReExport,
}

/// Reduced symbol signature used by drift detection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolSignature {
    /// Raw first-line signature.
    pub raw: String,
    /// Extracted parameter list.
    pub parameters: Vec<String>,
    /// Return type when declared.
    pub return_type: Option<String>,
}

/// Extracted symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AstSymbol {
    /// Absolute or repo-relative file path.
    pub file_path: PathBuf,
    /// Symbol name.
    pub name: String,
    /// Symbol kind.
    pub kind: AstSymbolKind,
    /// Whether symbol is exported.
    pub exported: bool,
    /// Location span.
    pub span: AstSpan,
    /// Signature details.
    pub signature: SymbolSignature,
    /// Leading documentation block.
    pub docs: Option<String>,
    /// Parsed annotation tags.
    pub annotations: Vec<AnnotationTag>,
    /// Business rule IDs extracted from annotations.
    pub rule_ids: Vec<String>,
    /// Stable structural hash for major-change comparisons.
    pub structural_hash: String,
}

/// Extracted file snapshot retained by the drift engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AstFileSnapshot {
    /// File path.
    pub file_path: PathBuf,
    /// Language used for parsing.
    pub language: SourceLanguage,
    /// Source hash.
    pub source_hash: String,
    /// File-level structural hash derived from symbols.
    pub module_hash: String,
    /// Exported and internal declarations of interest.
    pub symbols: Vec<AstSymbol>,
    /// Whether tree-sitter reported syntax errors.
    pub has_syntax_errors: bool,
}

#[derive(Debug)]
struct CachedAstFile {
    source_hash: String,
    retained_source: Option<String>,
    tree: Option<Tree>,
    snapshot: AstFileSnapshot,
    last_access_tick: u64,
    approx_tree_bytes: usize,
}

/// Live memory usage for retained incremental parser state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParserMemoryStats {
    /// Hard cap for retained parser state.
    pub budget_bytes: usize,
    /// Current retained bytes for cached trees and matching source text.
    pub retained_bytes: usize,
    /// Number of file snapshots tracked.
    pub cached_files: usize,
    /// Number of files with an in-memory tree retained for incremental parsing.
    pub retained_trees: usize,
}

/// Incremental parser with bounded tree retention.
pub struct IncrementalAstParser {
    parser: Parser,
    configured_language: Option<SourceLanguage>,
    cache: HashMap<PathBuf, CachedAstFile>,
    memory_budget_bytes: usize,
    retained_bytes: usize,
    tick: u64,
}

impl IncrementalAstParser {
    /// Create a parser with a strict tree-retention budget.
    pub fn new(memory_budget_bytes: usize) -> Self {
        Self {
            parser: Parser::new(),
            configured_language: None,
            cache: HashMap::new(),
            memory_budget_bytes,
            retained_bytes: 0,
            tick: 0,
        }
    }

    /// Adjust the retention budget and evict trees immediately if required.
    pub fn set_memory_budget(&mut self, memory_budget_bytes: usize) {
        self.memory_budget_bytes = memory_budget_bytes;
        self.enforce_budget();
    }

    /// Inspect current retained memory state.
    pub fn memory_stats(&self) -> ParserMemoryStats {
        ParserMemoryStats {
            budget_bytes: self.memory_budget_bytes,
            retained_bytes: self.retained_bytes,
            cached_files: self.cache.len(),
            retained_trees: self
                .cache
                .values()
                .filter(|entry| entry.tree.is_some())
                .count(),
        }
    }

    /// Drop all retained trees and their backing source text, keeping only extracted snapshots.
    pub fn release_all_trees(&mut self) {
        for entry in self.cache.values_mut() {
            entry.tree = None;
            entry.retained_source = None;
        }
        self.retained_bytes = 0;
    }

    /// Parse a changed file, reusing the previous tree when available.
    pub fn parse_changed_file(
        &mut self,
        file_path: &Path,
        source: &str,
    ) -> Result<AstFileSnapshot> {
        let language = SourceLanguage::detect(file_path)?;
        self.configure_language(language)?;

        let source_hash = hash_string(source);
        self.tick += 1;

        if let Some(entry) = self.cache.get_mut(file_path) {
            entry.last_access_tick = self.tick;
            if entry.source_hash == source_hash {
                return Ok(entry.snapshot.clone());
            }
        }

        let cached_previous = self
            .cache
            .get_mut(file_path)
            .map(|entry| (entry.tree.take(), entry.retained_source.take()));
        let parsed_tree = match cached_previous {
            Some((Some(mut tree), Some(previous_source))) => {
                tree.edit(&compute_incremental_edit(&previous_source, source));
                self.parser
                    .parse(source, Some(&tree))
                    .or_else(|| self.parser.parse(source, None))
                    .ok_or_else(|| AstError::ParseFailed(file_path.display().to_string()))?
            }
            Some((Some(_), None)) | Some((None, _)) | None => self
                .parser
                .parse(source, None)
                .ok_or_else(|| AstError::ParseFailed(file_path.display().to_string()))?,
        };

        let snapshot = extract_snapshot(file_path, language, source, &parsed_tree);
        let approx_tree_bytes = approximate_tree_bytes(source.len(), snapshot.symbols.len());

        self.install_cache_entry(
            file_path.to_path_buf(),
            CachedAstFile {
                source_hash,
                retained_source: Some(source.to_string()),
                tree: Some(parsed_tree),
                snapshot: snapshot.clone(),
                last_access_tick: self.tick,
                approx_tree_bytes,
            },
        );

        Ok(snapshot)
    }

    /// Drop the retained tree for a removed file.
    pub fn invalidate(&mut self, file_path: &Path) {
        if let Some(entry) = self.cache.remove(file_path) {
            self.retained_bytes = self.retained_bytes.saturating_sub(retained_cost(&entry));
        }
    }

    fn configure_language(&mut self, language: SourceLanguage) -> Result<()> {
        if self.configured_language == Some(language) {
            return Ok(());
        }

        let ts_language = match language {
            SourceLanguage::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            SourceLanguage::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
        };
        self.parser
            .set_language(&ts_language)
            .map_err(|_| AstError::LanguageSetup(language.parser_name()))?;
        self.configured_language = Some(language);
        Ok(())
    }

    fn install_cache_entry(&mut self, path: PathBuf, entry: CachedAstFile) {
        let path_for_lookup = path.clone();
        if let Some(previous) = self.cache.insert(path, entry) {
            self.retained_bytes = self.retained_bytes.saturating_sub(retained_cost(&previous));
        }
        let current = self
            .cache
            .get(path_for_lookup.as_path())
            .expect("inserted cache entry");
        self.retained_bytes = self.retained_bytes.saturating_add(retained_cost(current));

        self.enforce_budget();
    }

    fn enforce_budget(&mut self) {
        while self.retained_bytes > self.memory_budget_bytes {
            let Some(path) = self
                .cache
                .iter()
                .filter(|(_, entry)| entry.tree.is_some())
                .min_by_key(|(_, entry)| entry.last_access_tick)
                .map(|(path, _)| path.clone())
            else {
                break;
            };

            if let Some(entry) = self.cache.get_mut(&path) {
                let released_bytes = retained_cost(entry);
                if entry.tree.take().is_some() {
                    entry.retained_source = None;
                    self.retained_bytes = self
                        .retained_bytes
                        .saturating_sub(released_bytes);
                }
            }
        }
    }
}

fn extract_snapshot(
    file_path: &Path,
    language: SourceLanguage,
    source: &str,
    tree: &Tree,
) -> AstFileSnapshot {
    let root = tree.root_node();
    let mut symbols = Vec::new();
    extract_symbols(root, file_path, source, false, &mut symbols);
    apply_export_clause_symbols(root, file_path, source, &mut symbols);
    symbols.sort_by(|left, right| {
        left.span
            .start_line
            .cmp(&right.span.start_line)
            .then_with(|| left.name.cmp(&right.name))
    });

    let mut hasher = Hasher::new();
    for symbol in &symbols {
        hasher.update(symbol.structural_hash.as_bytes());
        hasher.update(symbol.name.as_bytes());
    }

    AstFileSnapshot {
        file_path: file_path.to_path_buf(),
        language,
        source_hash: hash_string(source),
        module_hash: hasher.finalize().to_hex().to_string(),
        symbols,
        has_syntax_errors: root.has_error(),
    }
}

fn extract_symbols(
    node: Node<'_>,
    file_path: &Path,
    source: &str,
    exported: bool,
    out: &mut Vec<AstSymbol>,
) {
    match node.kind() {
        "export_statement" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.is_named() {
                    if exported_default_symbol(child, node, file_path, source, out) {
                        continue;
                    }
                    extract_symbols(child, file_path, source, true, out);
                    if let Some(symbol) = make_default_export_alias_symbol(
                        child,
                        node,
                        file_path,
                        source,
                    ) {
                        out.push(symbol);
                    }
                }
            }
            return;
        }
        "function_declaration" => {
            if let Some(symbol) = make_function_symbol(node, file_path, source, exported) {
                out.push(symbol);
            }
        }
        "class_declaration" => {
            let class_name = child_text(node.child_by_field_name("name"), source);
            if let Some(symbol) = make_named_symbol(
                node,
                file_path,
                source,
                exported,
                AstSymbolKind::Class,
                class_name.clone(),
                extract_signature(node, source),
            ) {
                let extract_methods = symbol.exported || symbol.docs.is_some();
                out.push(symbol);
                if extract_methods {
                    out.extend(extract_class_method_symbols(
                        node,
                        file_path,
                        source,
                        exported,
                        class_name.as_deref().unwrap_or("anonymous"),
                    ));
                }
            }
        }
        "interface_declaration" => {
            if let Some(symbol) = make_named_symbol(
                node,
                file_path,
                source,
                exported,
                AstSymbolKind::Interface,
                child_text(node.child_by_field_name("name"), source),
                extract_signature(node, source),
            ) {
                out.push(symbol);
            }
        }
        "type_alias_declaration" => {
            if let Some(symbol) = make_named_symbol(
                node,
                file_path,
                source,
                exported,
                AstSymbolKind::TypeAlias,
                child_text(node.child_by_field_name("name"), source),
                extract_signature(node, source),
            ) {
                out.push(symbol);
            }
        }
        "lexical_declaration" => {
            out.extend(make_lexical_function_symbols(
                node, file_path, source, exported,
            ));
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.is_named() {
            extract_symbols(child, file_path, source, exported, out);
        }
    }
}

fn extract_class_method_symbols(
    class_node: Node<'_>,
    file_path: &Path,
    source: &str,
    exported: bool,
    class_name: &str,
) -> Vec<AstSymbol> {
    let mut symbols = Vec::new();
    let Some(body) = class_node.child_by_field_name("body") else {
        return symbols;
    };
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if child.kind() != "method_definition" && child.kind() != "method_signature" {
            continue;
        }
        let Some(method_name) = child
            .child_by_field_name("name")
            .and_then(|node| node.utf8_text(source.as_bytes()).ok())
            .map(str::trim)
            .map(str::to_string)
        else {
            continue;
        };
        let name = format!("{class_name}.{method_name}");
        if let Some(symbol) = make_named_symbol(
            child,
            file_path,
            source,
            exported,
            AstSymbolKind::Function,
            Some(name),
            extract_signature(child, source),
        ) {
            symbols.push(symbol);
        }
    }
    symbols
}

fn apply_export_clause_symbols(
    root: Node<'_>,
    file_path: &Path,
    source: &str,
    symbols: &mut Vec<AstSymbol>,
) {
    let mut export_statements = Vec::new();
    collect_nodes_by_kind(root, "export_statement", &mut export_statements);

    for export_statement in export_statements {
        let Ok(text) = export_statement.utf8_text(source.as_bytes()) else {
            continue;
        };

        if let Some(specifiers) = parse_export_clause_text(text) {
            for (local_name, exported_name) in specifiers {
                if local_name == exported_name {
                    mark_existing_symbol_exported(symbols, &local_name);
                    continue;
                }

                if let Some(existing) = symbols.iter().find(|symbol| symbol.name == local_name).cloned() {
                    symbols.push(clone_symbol_as_export(existing, exported_name));
                } else {
                    symbols.push(make_reexport_symbol(
                        export_statement,
                        file_path,
                        exported_name,
                        text.to_string(),
                    ));
                }
            }
            continue;
        }

        if let Some(alias) = parse_wildcard_export_alias(text) {
            symbols.push(make_reexport_symbol(
                export_statement,
                file_path,
                alias,
                text.to_string(),
            ));
        } else if is_plain_wildcard_export(text) {
            symbols.push(make_reexport_symbol(
                export_statement,
                file_path,
                "*".to_string(),
                text.to_string(),
            ));
        }
    }
}

fn collect_nodes_by_kind<'a>(node: Node<'a>, kind: &str, out: &mut Vec<Node<'a>>) {
    if node.kind() == kind {
        out.push(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.is_named() {
            collect_nodes_by_kind(child, kind, out);
        }
    }
}

fn parse_export_clause_text(text: &str) -> Option<Vec<(String, String)>> {
    let start = text.find('{')?;
    let end = text[start + 1..].find('}')? + start + 1;
    let contents = text.get(start + 1..end)?.trim();
    if contents.is_empty() {
        return None;
    }

    Some(
        split_top_level_commas(contents)
            .into_iter()
            .filter(|specifier| !specifier.is_empty())
            .map(|specifier| {
                let mut parts = specifier.split(" as ");
                let local = parts.next().unwrap_or_default().trim().to_string();
                let exported = parts
                    .next()
                    .map(str::trim)
                    .filter(|part| !part.is_empty())
                    .map(str::to_string)
                    .unwrap_or_else(|| local.clone());
                (local, exported)
            })
            .collect(),
    )
}

fn parse_wildcard_export_alias(text: &str) -> Option<String> {
    let alias_split = text.split(" as ").nth(1)?;
    let alias = alias_split
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .trim();
    if alias.is_empty() {
        None
    } else {
        Some(alias.to_string())
    }
}

fn is_plain_wildcard_export(text: &str) -> bool {
    text.trim_start().starts_with("export * from ")
}

fn mark_existing_symbol_exported(symbols: &mut Vec<AstSymbol>, symbol_name: &str) {
    for symbol in symbols.iter_mut().filter(|symbol| symbol.name == symbol_name) {
        if symbol.exported {
            continue;
        }
        symbol.exported = true;
        symbol.structural_hash = compute_symbol_hash(
            &symbol.kind,
            symbol.exported,
            &symbol.name,
            &symbol.signature,
            &symbol.rule_ids,
        );
    }
}

fn clone_symbol_as_export(mut symbol: AstSymbol, exported_name: String) -> AstSymbol {
    symbol.name = exported_name;
    symbol.exported = true;
    symbol.structural_hash = compute_symbol_hash(
        &symbol.kind,
        symbol.exported,
        &symbol.name,
        &symbol.signature,
        &symbol.rule_ids,
    );
    symbol
}

fn make_reexport_symbol(
    export_statement: Node<'_>,
    file_path: &Path,
    name: String,
    raw_signature: String,
) -> AstSymbol {
    let signature = SymbolSignature {
        raw: normalize_signature_text(raw_signature),
        parameters: Vec::new(),
        return_type: None,
    };
    let structural_hash =
        compute_symbol_hash(&AstSymbolKind::ReExport, true, &name, &signature, &[]);
    AstSymbol {
        file_path: file_path.to_path_buf(),
        name,
        kind: AstSymbolKind::ReExport,
        exported: true,
        span: AstSpan::from_node(export_statement),
        signature,
        docs: None,
        annotations: Vec::new(),
        rule_ids: Vec::new(),
        structural_hash,
    }
}

fn make_function_symbol(
    node: Node<'_>,
    file_path: &Path,
    source: &str,
    exported: bool,
) -> Option<AstSymbol> {
    let name = child_text(node.child_by_field_name("name"), source)?;
    let signature = extract_signature(node, source);
    make_named_symbol(
        node,
        file_path,
        source,
        exported,
        AstSymbolKind::Function,
        Some(name),
        signature,
    )
}

fn exported_default_symbol(
    child: Node<'_>,
    export_statement: Node<'_>,
    file_path: &Path,
    source: &str,
    out: &mut Vec<AstSymbol>,
) -> bool {
    if !export_statement_contains_default(export_statement, source) {
        return false;
    }
    if child.child_by_field_name("name").is_some() {
        return false;
    }

    let symbol = match child.kind() {
        "function_declaration" => make_named_symbol(
            child,
            file_path,
            source,
            true,
            AstSymbolKind::Function,
            Some("default".to_string()),
            extract_signature(child, source),
        ),
        "class_declaration" => make_named_symbol(
            child,
            file_path,
            source,
            true,
            AstSymbolKind::Class,
            Some("default".to_string()),
            extract_signature(child, source),
        ),
        _ => None,
    };
    if let Some(symbol) = symbol {
        out.push(symbol);
        return true;
    }
    false
}

fn make_default_export_alias_symbol(
    child: Node<'_>,
    export_statement: Node<'_>,
    file_path: &Path,
    source: &str,
) -> Option<AstSymbol> {
    if !export_statement_contains_default(export_statement, source)
        || child.child_by_field_name("name").is_none()
    {
        return None;
    }

    match child.kind() {
        "function_declaration" => make_named_symbol(
            child,
            file_path,
            source,
            true,
            AstSymbolKind::Function,
            Some("default".to_string()),
            extract_signature(child, source),
        ),
        "class_declaration" => make_named_symbol(
            child,
            file_path,
            source,
            true,
            AstSymbolKind::Class,
            Some("default".to_string()),
            extract_signature(child, source),
        ),
        _ => None,
    }
}

fn make_lexical_function_symbols(
    node: Node<'_>,
    file_path: &Path,
    source: &str,
    exported: bool,
) -> Vec<AstSymbol> {
    let mut symbols = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() != "variable_declarator" {
            continue;
        }

        let Some(name) = child_text(child.child_by_field_name("name"), source) else {
            continue;
        };
        let Some(value) = child.child_by_field_name("value") else {
            continue;
        };

        let kind = match value.kind() {
            "arrow_function" | "function" | "function_expression" => AstSymbolKind::Function,
            _ => continue,
        };

        let signature = extract_variable_signature(name.as_str(), value, source);
        if let Some(symbol) = make_named_symbol(
            child,
            file_path,
            source,
            exported,
            kind,
            Some(name),
            signature,
        ) {
            symbols.push(symbol);
        }
    }
    symbols
}

fn make_named_symbol(
    node: Node<'_>,
    file_path: &Path,
    source: &str,
    exported: bool,
    kind: AstSymbolKind,
    name: Option<String>,
    signature: SymbolSignature,
) -> Option<AstSymbol> {
    let name = name?;
    let signature = canonicalize_symbol_signature(&kind, exported, &name, signature);
    let docs = extract_leading_docs(source, node.start_position().row);
    let annotations = docs
        .as_deref()
        .map(parse_annotation_tags)
        .unwrap_or_default();
    let rule_ids = annotations
        .iter()
        .filter(|tag| tag.name.eq_ignore_ascii_case("ruleid"))
        .map(|tag| tag.value.clone())
        .collect::<Vec<_>>();
    let structural_hash = compute_symbol_hash(&kind, exported, &name, &signature, &rule_ids);

    Some(AstSymbol {
        file_path: file_path.to_path_buf(),
        name,
        kind,
        exported,
        span: AstSpan::from_node(node),
        signature,
        docs,
        annotations,
        rule_ids,
        structural_hash,
    })
}

fn canonicalize_symbol_signature(
    kind: &AstSymbolKind,
    exported: bool,
    name: &str,
    mut signature: SymbolSignature,
) -> SymbolSignature {
    if !exported || signature.raw.is_empty() || matches!(kind, AstSymbolKind::ReExport) {
        return signature;
    }

    if name == "default" && !signature.raw.starts_with("export default ") {
        signature.raw = format!("export default {}", signature.raw);
        return signature;
    }

    if !signature.raw.starts_with("export ") {
        signature.raw = format!("export {}", signature.raw);
    }
    signature
}

fn extract_signature(node: Node<'_>, source: &str) -> SymbolSignature {
    let raw = signature_prefix_text(node, source)
        .map(normalize_signature_text)
        .unwrap_or_default();
    let parameters = node
        .child_by_field_name("parameters")
        .map(|parameters| extract_parameter_names(parameters, source))
        .unwrap_or_default();
    let return_type = child_text(node.child_by_field_name("return_type"), source)
        .map(|text| text.trim_start_matches(':').trim().to_string())
        .filter(|value| !value.is_empty());

    SymbolSignature {
        raw,
        parameters,
        return_type,
    }
}

fn extract_variable_signature(name: &str, value: Node<'_>, source: &str) -> SymbolSignature {
    let raw = signature_prefix_text(value, source)
        .map(|prefix| normalize_signature_text(format!("const {name} = {prefix}")))
        .unwrap_or_else(|| format!("const {name}"));
    let parameters = value
        .child_by_field_name("parameters")
        .map(|parameters| extract_parameter_names(parameters, source))
        .unwrap_or_default();
    let return_type = child_text(value.child_by_field_name("return_type"), source)
        .map(|text| text.trim_start_matches(':').trim().to_string())
        .filter(|value| !value.is_empty());

    SymbolSignature {
        raw,
        parameters,
        return_type,
    }
}

fn extract_parameter_names(parameters: Node<'_>, source: &str) -> Vec<String> {
    let text = parameters
        .utf8_text(source.as_bytes())
        .unwrap_or_default()
        .trim()
        .trim_start_matches('(')
        .trim_end_matches(')')
        .to_string();

    split_top_level_commas(&text)
        .into_iter()
        .map(|segment| {
            segment
                .split(':')
                .next()
                .unwrap_or(segment.as_str())
                .trim()
                .trim_start_matches("...")
                .trim_start_matches('{')
                .trim_end_matches('}')
                .trim()
                .to_string()
        })
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn child_text(node: Option<Node<'_>>, source: &str) -> Option<String> {
    node.and_then(|node: Node<'_>| node.utf8_text(source.as_bytes()).ok())
        .map(str::trim)
        .map(ToString::to_string)
}

fn extract_leading_docs(source: &str, start_row_zero_based: usize) -> Option<String> {
    let lines: Vec<&str> = source.lines().collect();
    if start_row_zero_based == 0 || lines.is_empty() {
        return None;
    }

    let mut row = start_row_zero_based.saturating_sub(1);
    let mut collected = Vec::new();
    let mut seen_comment = false;

    loop {
        let line = lines.get(row).copied().unwrap_or_default().trim();
        if line.is_empty() {
            if seen_comment {
                break;
            }
        } else if line.starts_with("//")
            || line.starts_with("/*")
            || line.starts_with('*')
            || line.ends_with("*/")
        {
            seen_comment = true;
            collected.push(lines[row]);
        } else {
            break;
        }

        if row == 0 {
            break;
        }
        row -= 1;
    }

    if collected.is_empty() {
        None
    } else {
        collected.reverse();
        Some(collected.join("\n"))
    }
}

fn parse_annotation_tags(docs: &str) -> Vec<AnnotationTag> {
    docs.lines()
        .filter_map(|line| {
            let trimmed = line
                .trim()
                .trim_start_matches("///")
                .trim_start_matches("//")
                .trim_start_matches('/')
                .trim_start_matches('*')
                .trim_end_matches("*/")
                .trim();
            let tag = trimmed.strip_prefix('@')?;
            let mut parts = tag.splitn(2, char::is_whitespace);
            let name = parts.next()?.trim();
            let value = parts.next().unwrap_or_default().trim();
            Some(AnnotationTag {
                name: name.to_string(),
                value: value.to_string(),
            })
        })
        .collect()
}

fn compute_symbol_hash(
    kind: &AstSymbolKind,
    exported: bool,
    name: &str,
    signature: &SymbolSignature,
    rule_ids: &[String],
) -> String {
    let mut hasher = Hasher::new();
    hasher.update(format!("{kind:?}:{exported}:{name}").as_bytes());
    hasher.update(signature.raw.as_bytes());
    for parameter in &signature.parameters {
        hasher.update(parameter.as_bytes());
    }
    if let Some(return_type) = &signature.return_type {
        hasher.update(return_type.as_bytes());
    }
    for rule_id in rule_ids {
        hasher.update(rule_id.as_bytes());
    }
    hasher.finalize().to_hex().to_string()
}

fn hash_string(input: &str) -> String {
    blake3::hash(input.as_bytes()).to_hex().to_string()
}

fn approximate_tree_bytes(source_len: usize, symbol_count: usize) -> usize {
    let structural_overhead = symbol_count.saturating_mul(256);
    (source_len / 2)
        .saturating_add(structural_overhead)
        .max(4096)
}

fn retained_cost(entry: &CachedAstFile) -> usize {
    if entry.tree.is_none() {
        return 0;
    }
    entry.approx_tree_bytes + entry.retained_source.as_ref().map_or(0, String::len)
}

fn compute_incremental_edit(old_source: &str, new_source: &str) -> InputEdit {
    let old_bytes = old_source.as_bytes();
    let new_bytes = new_source.as_bytes();
    let mut prefix = 0usize;
    let prefix_limit = old_bytes.len().min(new_bytes.len());
    while prefix < prefix_limit && old_bytes[prefix] == new_bytes[prefix] {
        prefix += 1;
    }

    let mut suffix = 0usize;
    while suffix < old_bytes.len().saturating_sub(prefix)
        && suffix < new_bytes.len().saturating_sub(prefix)
        && old_bytes[old_bytes.len() - 1 - suffix] == new_bytes[new_bytes.len() - 1 - suffix]
    {
        suffix += 1;
    }

    let old_end_byte = old_bytes.len().saturating_sub(suffix);
    let new_end_byte = new_bytes.len().saturating_sub(suffix);
    InputEdit {
        start_byte: prefix,
        old_end_byte,
        new_end_byte,
        start_position: point_for_byte(old_source, prefix),
        old_end_position: point_for_byte(old_source, old_end_byte),
        new_end_position: point_for_byte(new_source, new_end_byte),
    }
}

fn signature_prefix_text(node: Node<'_>, source: &str) -> Option<String> {
    let end_byte = node
        .child_by_field_name("body")
        .map(|body| body.start_byte())
        .unwrap_or_else(|| node.end_byte());
    source
        .get(node.start_byte()..end_byte)
        .map(str::trim)
        .map(str::to_string)
}

fn normalize_signature_text(raw: impl AsRef<str>) -> String {
    raw.as_ref()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .trim_end_matches('{')
        .trim()
        .to_string()
}

fn split_top_level_commas(input: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut angles = 0usize;
    let mut parens = 0usize;
    let mut braces = 0usize;
    let mut brackets = 0usize;
    for (index, ch) in input.char_indices() {
        match ch {
            '<' => angles += 1,
            '>' => angles = angles.saturating_sub(1),
            '(' => parens += 1,
            ')' => parens = parens.saturating_sub(1),
            '{' => braces += 1,
            '}' => braces = braces.saturating_sub(1),
            '[' => brackets += 1,
            ']' => brackets = brackets.saturating_sub(1),
            ',' if angles == 0 && parens == 0 && braces == 0 && brackets == 0 => {
                parts.push(input[start..index].trim().to_string());
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(input[start..].trim().to_string());
    parts
}

fn export_statement_contains_default(node: Node<'_>, source: &str) -> bool {
    node.utf8_text(source.as_bytes())
        .ok()
        .map(|text| text.contains("default"))
        .unwrap_or(false)
}

fn point_for_byte(source: &str, byte_offset: usize) -> Point {
    let mut row = 0usize;
    let mut column = 0usize;
    for byte in source.as_bytes().iter().take(byte_offset) {
        if *byte == b'\n' {
            row += 1;
            column = 0;
        } else {
            column += 1;
        }
    }
    Point::new(row, column)
}

#[cfg(test)]
mod tests {
    use super::{compute_incremental_edit, AstSymbolKind, IncrementalAstParser};
    use std::path::Path;

    #[test]
    fn extracts_exported_typescript_symbols_and_rule_ids() {
        let mut parser = IncrementalAstParser::new(1024 * 1024);
        let source = r#"
/** @RuleID BR-001 */
export async function resolvePlan(accountId: string, force?: boolean): Promise<string> {
  return accountId;
}

/** @RuleID BR-002 */
export const hydrateQueue = async (jobId: string) => {
  return jobId;
};

export interface QueueState {
  size: number;
}

export type QueueMode = "fast" | "safe";
"#;

        let snapshot = parser
            .parse_changed_file(Path::new("apps/web/lib/plan.ts"), source)
            .expect("snapshot");

        assert_eq!(snapshot.symbols.len(), 4);
        assert_eq!(snapshot.symbols[0].kind, AstSymbolKind::Function);
        assert_eq!(snapshot.symbols[0].rule_ids, vec!["BR-001"]);
        assert_eq!(snapshot.symbols[1].rule_ids, vec!["BR-002"]);
    }

    #[test]
    fn parses_multiline_jsdoc_rule_ids() {
        let mut parser = IncrementalAstParser::new(1024 * 1024);
        let source = r#"
/**
 * @RuleID BR-777
 * @Owner team-docs
 */
export function resolvePlan(accountId: string): Promise<string> {
  return Promise.resolve(accountId);
}
"#;

        let snapshot = parser
            .parse_changed_file(Path::new("apps/web/lib/plan.ts"), source)
            .expect("snapshot");

        assert_eq!(snapshot.symbols[0].rule_ids, vec!["BR-777"]);
        assert_eq!(snapshot.symbols[0].annotations.len(), 2);
    }

    #[test]
    fn computes_bounded_incremental_edit_window() {
        let old_source = "export function resolvePlan(id: string) {\n  return id;\n}\n";
        let new_source =
            "export function resolvePlan(accountId: string) {\n  return accountId;\n}\n";
        let edit = compute_incremental_edit(old_source, new_source);

        assert!(edit.start_byte > 0);
        assert!(edit.old_end_byte < old_source.len());
        assert!(edit.new_end_byte < new_source.len());
    }

    #[test]
    fn captures_default_exports_reexports_and_route_handlers() {
        let mut parser = IncrementalAstParser::new(1024 * 1024);
        let source = r#"
export default async function Page() {
  return <div />;
}

async function routeHandler(request: Request): Promise<Response> {
  return new Response(String(request.url));
}

export { routeHandler as GET };
export * from "./shared";
"#;

        let snapshot = parser
            .parse_changed_file(Path::new("apps/web/app/api/route.tsx"), source)
            .expect("snapshot");

        assert!(snapshot.symbols.iter().any(|symbol| {
            symbol.name == "Page" && symbol.kind == AstSymbolKind::Function && symbol.exported
        }));
        assert!(snapshot.symbols.iter().any(|symbol| {
            symbol.name == "default"
                && symbol.kind == AstSymbolKind::Function
                && symbol.exported
        }));
        assert!(snapshot.symbols.iter().any(|symbol| {
            symbol.name == "GET" && symbol.kind == AstSymbolKind::Function && symbol.exported
        }));
        assert!(snapshot.symbols.iter().any(|symbol| {
            symbol.name == "*" && symbol.kind == AstSymbolKind::ReExport && symbol.exported
        }));
    }

    #[test]
    fn extracts_documented_class_methods_and_complex_signatures() {
        let mut parser = IncrementalAstParser::new(1024 * 1024);
        let source = r#"
/**
 * @RuleID BR-321
 */
export default class Planner<TState extends Record<string, string>> {
  resolvePlan(
    accountId: string,
    options: Array<Result<string, Error>>,
  ): Promise<Map<string, TState>> {
    return Promise.resolve(new Map());
  }
}
"#;

        let snapshot = parser
            .parse_changed_file(Path::new("apps/web/lib/planner.ts"), source)
            .expect("snapshot");

        let class_symbol = snapshot
            .symbols
            .iter()
            .find(|symbol| symbol.name == "Planner")
            .expect("class symbol");
        assert_eq!(class_symbol.rule_ids, vec!["BR-321"]);

        let method_symbol = snapshot
            .symbols
            .iter()
            .find(|symbol| symbol.name == "Planner.resolvePlan")
            .expect("method symbol");
        assert_eq!(method_symbol.kind, AstSymbolKind::Function);
        assert_eq!(method_symbol.signature.parameters, vec!["accountId", "options"]);
        assert_eq!(
            method_symbol.signature.return_type.as_deref(),
            Some("Promise<Map<string, TState>>")
        );

        assert!(snapshot.symbols.iter().any(|symbol| {
            symbol.name == "default"
                && symbol.kind == AstSymbolKind::Class
                && symbol.exported
        }));
    }

    #[test]
    fn evicts_retained_trees_when_budget_is_exceeded() {
        let mut parser = IncrementalAstParser::new(4 * 1024);
        let source = "export function resolvePlan(accountId: string) { return accountId; }\n";

        parser
            .parse_changed_file(Path::new("apps/web/lib/a.ts"), source)
            .expect("first snapshot");
        parser
            .parse_changed_file(Path::new("apps/web/lib/b.ts"), source)
            .expect("second snapshot");

        let stats = parser.memory_stats();
        assert_eq!(stats.cached_files, 2);
        assert!(stats.retained_trees <= 1);
        assert!(stats.retained_bytes <= stats.budget_bytes);
    }
}
