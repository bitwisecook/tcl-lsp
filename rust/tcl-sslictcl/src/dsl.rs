// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Safe parser for the declarative `.sslictcl` vocabulary.
//!
//! Tcl supplies quoting, comments, line continuation, and nested braced
//! blocks.  This module walks the canonical CST and never invokes a Tcl
//! interpreter. Command substitution, variable substitution, and argument
//! expansion are rejected instead of being evaluated or guessed.
//!
//! [`load_with_diagnostics`] is the authoring-grade entry point: it recovers
//! past a bad declaration and reports every problem it finds, each with a
//! [`DiagCode`] and a byte range into the original document. [`load`] is the
//! thin single-error wrapper the batch consumers use.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use tcl_compiler::parsing::syntax::build::build_document;
use tcl_compiler::parsing::syntax::segment::segments_from_document;
use tcl_compiler::segmenter::SegmentedCommand;
use tcl_core_types::DiagCode;
use tcl_lexer::{LexerConfig, LineIndex, SourceMap, Span, TokenType, script_is_complete};

use crate::model::{
    CertificateDeclaration, Endpoint, HstsPolicy, ProtocolVersion, SslicModel, TlsValue,
};
use crate::testssl::import_testssl_json;

/// The `SslicTcl` vocabulary version this build implements.
pub const SUPPORTED_VOCABULARY: u32 = 1;

/// A successfully loaded document and its forwards-compatibility notices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DslDocument {
    /// Byte-for-byte original source. Unknown declarations can be re-emitted
    /// from this rather than from the normalized semantic projection.
    pub raw_source: String,
    /// Typed declarations.
    pub model: SslicModel,
    /// Unknown declarations retained in the model.
    pub notices: Vec<DslNotice>,
}

/// How loudly a [`DslDiagnostic`] should be presented.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DslSeverity {
    /// The declaration was skipped; the model does not contain it.
    Error,
    /// The declaration was kept, but something about it needs attention.
    Warning,
    /// The declaration was kept; this is context only.
    Hint,
}

/// One coded, ranged loader diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DslDiagnostic {
    /// Stable published code (`SSLIC1001` …).
    pub code: DiagCode,
    /// Presentation severity.
    pub severity: DslSeverity,
    /// Byte range in the **original** document, nested blocks included.
    pub range: Span,
    /// Human-readable explanation.
    pub message: String,
}

/// The outcome of loading one document: a model when one could be built, plus
/// every diagnostic found.
///
/// `document` is `Some` when the top-level statement stream segmented and the
/// document declared a usable `sslictcl VERSION` header; individual bad
/// declarations are skipped rather than abandoning the load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DslLoad {
    /// The recovered document, when one could be built.
    pub document: Option<DslDocument>,
    /// Every diagnostic, in source order of discovery.
    pub diagnostics: Vec<DslDiagnostic>,
}

/// A non-fatal unknown declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DslNotice {
    /// Published code.
    pub code: DiagCode,
    /// One-based source line.
    pub line: u32,
    /// Byte range in the original document.
    pub range: Span,
    /// Human-readable explanation.
    pub message: String,
}

/// A syntax or schema error that makes a declaration ambiguous or unsafe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DslError {
    /// Published code.
    pub code: DiagCode,
    /// One-based source line.
    pub line: u32,
    /// Byte range in the original document.
    pub range: Span,
    /// Human-readable explanation.
    pub message: String,
}

impl fmt::Display for DslError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {} [{}]", self.line, self.message, self.code)
    }
}

impl Error for DslError {}

/// Accumulates diagnostics while the walk recovers past bad declarations.
#[derive(Debug, Default)]
struct Sink {
    items: Vec<DslDiagnostic>,
}

impl Sink {
    fn push(&mut self, code: DiagCode, severity: DslSeverity, range: Span, message: String) {
        self.items.push(DslDiagnostic {
            code,
            severity,
            range,
            message,
        });
    }

    fn error(&mut self, code: DiagCode, range: Span, message: impl Into<String>) {
        self.push(code, DslSeverity::Error, range, message.into());
    }

    fn warning(&mut self, code: DiagCode, range: Span, message: impl Into<String>) {
        self.push(code, DslSeverity::Warning, range, message.into());
    }

    fn hint(&mut self, code: DiagCode, range: Span, message: impl Into<String>) {
        self.push(code, DslSeverity::Hint, range, message.into());
    }
}

#[derive(Debug, Clone)]
struct Word {
    text: String,
    braced: bool,
    literal: bool,
    /// Absolute span of the whole word, delimiters included.
    span: Span,
    /// Absolute offset of the word's first inner byte.
    content_start: u32,
}

#[derive(Debug, Clone)]
struct Stmt {
    words: Vec<Word>,
    /// Absolute span of the whole statement.
    span: Span,
}

impl Stmt {
    fn name(&self) -> &str {
        self.words.first().map_or("", |word| word.text.as_str())
    }

    fn word(&self, index: usize) -> &Word {
        &self.words[index]
    }

    /// Reject a statement whose word count is not `count` (`SSLIC1005`).
    fn require_words(&self, count: usize, sink: &mut Sink) -> bool {
        if self.words.len() == count {
            return true;
        }
        sink.error(
            DiagCode::Sslic1005,
            self.span,
            format!(
                "`{}` expects {count} word(s), got {}",
                self.name(),
                self.words.len()
            ),
        );
        false
    }

    /// Reject substitution and `{*}` expansion anywhere in the statement
    /// (`SSLIC1002`).
    fn require_literals(&self, sink: &mut Sink) -> bool {
        let Some(offender) = self.words.iter().find(|word| !word.literal) else {
            return true;
        };
        sink.error(
            DiagCode::Sslic1002,
            offender.span,
            format!(
                "`{}` must be declarative; substitutions and argument expansion are forbidden",
                self.name()
            ),
        );
        false
    }
}

/// The document under load: the original text plus its line index, so every
/// nested block statement can report an absolute range and a real line.
struct Document {
    lines: LineIndex,
}

impl Document {
    fn new(source: &str) -> Self {
        Self {
            lines: LineIndex::new(source),
        }
    }

    fn line_of(&self, range: Span) -> u32 {
        self.lines.line_at(range.start()) + 1
    }

    /// Segment `slice`, which starts at absolute offset `base`, into
    /// statements whose spans are absolute.
    fn segment(slice: &str, base: u32, sink: &mut Sink) -> Option<Vec<Stmt>> {
        let whole = Span::new(base, base + u32_len(slice));
        if !script_is_complete(slice) {
            sink.error(
                DiagCode::Sslic1001,
                whole,
                "incomplete SslicTcl declaration or unclosed delimiter",
            );
            return None;
        }
        let source_map = SourceMap::new(slice);
        let (document, warnings) = build_document(slice, LexerConfig::default());
        if let Some(warning) = warnings.first() {
            let start = base + warning.offset;
            sink.error(
                DiagCode::Sslic1001,
                Span::new(start.min(whole.end()), (start + 1).min(whole.end())),
                format!("invalid SslicTcl declaration: {}", warning.message),
            );
            return None;
        }
        Some(
            segments_from_document(document, &source_map)
                .into_iter()
                .map(|segment| statement_from_segment(&segment, base))
                .collect(),
        )
    }

    /// Segment a braced declaration body, reporting `SSLIC1006` when the body
    /// is not a braced literal.
    fn block(word: &Word, sink: &mut Sink) -> Option<Vec<Stmt>> {
        if !word.braced {
            sink.error(
                DiagCode::Sslic1006,
                word.span,
                "SslicTcl declaration body must be a braced literal",
            );
            return None;
        }
        Self::segment(&word.text, word.content_start, sink)
    }

    /// One `LIST` value: a braced Tcl list of literal words, or a bare word.
    fn literal_list(word: &Word, sink: &mut Sink) -> Option<Vec<String>> {
        if !word.literal {
            sink.error(
                DiagCode::Sslic1002,
                word.span,
                "list must not contain substitutions or argument expansion",
            );
            return None;
        }
        if word.text.trim().is_empty() {
            return Some(Vec::new());
        }
        if !word.braced {
            return Some(vec![word.text.clone()]);
        }
        let rows = Self::segment(&word.text, word.content_start, sink)?;
        if rows.len() != 1 {
            sink.error(
                DiagCode::Sslic1009,
                word.span,
                "list must be a single Tcl list, not multiple commands",
            );
            return None;
        }
        if !rows[0].require_literals(sink) {
            return None;
        }
        Some(rows[0].words.iter().map(|item| item.text.clone()).collect())
    }
}

fn u32_len(text: &str) -> u32 {
    u32::try_from(text.len()).unwrap_or(u32::MAX)
}

fn statement_from_segment(segment: &SegmentedCommand, base: u32) -> Stmt {
    let words = segment
        .texts
        .iter()
        .enumerate()
        .map(|(index, text)| {
            let token = segment.argv[index];
            let fragments = &segment.word_fragments[index];
            let expanded = segment
                .expand_word
                .as_ref()
                .and_then(|expanded| expanded.get(index))
                .copied()
                .unwrap_or(false);
            let literal = !expanded
                && fragments
                    .iter()
                    .all(|fragment| matches!(fragment.token.kind, TokenType::Esc | TokenType::Str));
            let start = fragments
                .iter()
                .map(|fragment| fragment.token.span.start())
                .min()
                .unwrap_or_else(|| token.span.start());
            let end = fragments
                .iter()
                .map(|fragment| fragment.token.span.end())
                .max()
                .unwrap_or_else(|| token.span.end());
            let braced = fragments.len() == 1 && token.kind == TokenType::Str && !token.in_quote;
            Word {
                text: text.clone(),
                braced,
                literal,
                span: Span::new(base + start, base + end),
                content_start: base + start + u32::from(token.content_offset),
            }
        })
        .collect();
    Stmt {
        words,
        span: Span::new(base + segment.span.start(), base + segment.span.end()),
    }
}

fn extension_value(stmt: &Stmt) -> TlsValue {
    if stmt.words.len() == 2 {
        TlsValue::Scalar(stmt.words[1].text.clone())
    } else {
        TlsValue::List(
            stmt.words
                .iter()
                .skip(1)
                .map(|word| word.text.clone())
                .collect(),
        )
    }
}

fn add_extension(map: &mut BTreeMap<String, Vec<TlsValue>>, stmt: &Stmt, sink: &mut Sink) {
    map.entry(stmt.name().to_owned())
        .or_default()
        .push(extension_value(stmt));
    sink.hint(
        DiagCode::Sslic1101,
        stmt.span,
        format!("unknown declaration `{}` preserved", stmt.name()),
    );
}

/// Load one `.sslictcl` source string without evaluating it, recovering past
/// every problem it can and reporting all of them.
#[must_use]
pub fn load_with_diagnostics(source: &str) -> DslLoad {
    let document = Document::new(source);
    let mut sink = Sink::default();
    let mut model = SslicModel::default();
    let mut saw_header = false;

    let Some(rows) = Document::segment(source, 0, &mut sink) else {
        return DslLoad {
            document: None,
            diagnostics: sink.items,
        };
    };

    for stmt in &rows {
        if stmt.name().is_empty() {
            continue;
        }
        if !stmt.require_literals(&mut sink) {
            continue;
        }
        match stmt.name() {
            "sslictcl" => parse_header(stmt, &mut model, &mut saw_header, &mut sink),
            "certificate" => parse_certificate(stmt, &mut model, &mut sink),
            "endpoint" => parse_endpoint(stmt, &mut model, &mut sink),
            "testssl-import" => parse_testssl_import(stmt, &mut model, &mut sink),
            _ => add_extension(&mut model.extensions, stmt, &mut sink),
        }
    }

    if !saw_header {
        sink.error(
            DiagCode::Sslic1003,
            Span::empty(0),
            "missing `sslictcl VERSION` declaration",
        );
        return DslLoad {
            document: None,
            diagnostics: sink.items,
        };
    }

    let notices = sink
        .items
        .iter()
        .filter(|item| item.severity != DslSeverity::Error)
        .map(|item| DslNotice {
            code: item.code,
            line: document.line_of(item.range),
            range: item.range,
            message: item.message.clone(),
        })
        .collect();
    DslLoad {
        document: Some(DslDocument {
            raw_source: source.to_owned(),
            model,
            notices,
        }),
        diagnostics: sink.items,
    }
}

/// Load one `.sslictcl` source string, failing on the first error.
///
/// The thin wrapper over [`load_with_diagnostics`] that batch consumers use;
/// it discards the recovered model when anything was rejected.
pub fn load(source: &str) -> Result<DslDocument, DslError> {
    let lines = LineIndex::new(source);
    let loaded = load_with_diagnostics(source);
    if let Some(first) = loaded
        .diagnostics
        .iter()
        .find(|item| item.severity == DslSeverity::Error)
    {
        return Err(DslError {
            code: first.code,
            line: lines.line_at(first.range.start()) + 1,
            range: first.range,
            message: first.message.clone(),
        });
    }
    loaded.document.ok_or_else(|| DslError {
        code: DiagCode::Sslic1003,
        line: 1,
        range: Span::empty(0),
        message: "missing `sslictcl VERSION` declaration".to_owned(),
    })
}

fn parse_header(stmt: &Stmt, model: &mut SslicModel, saw_header: &mut bool, sink: &mut Sink) {
    if !stmt.require_words(2, sink) {
        return;
    }
    if *saw_header {
        sink.error(
            DiagCode::Sslic1004,
            stmt.span,
            "duplicate `sslictcl` vocabulary declaration",
        );
        return;
    }
    let version_word = stmt.word(1);
    let Ok(vocabulary) = version_word.text.parse::<u32>() else {
        sink.error(
            DiagCode::Sslic1009,
            version_word.span,
            "SslicTcl vocabulary must be an unsigned integer",
        );
        return;
    };
    if vocabulary == 0 {
        sink.error(
            DiagCode::Sslic1009,
            version_word.span,
            "SslicTcl vocabulary must be at least 1",
        );
        return;
    }
    model.vocabulary = vocabulary;
    *saw_header = true;
    if vocabulary > SUPPORTED_VOCABULARY {
        sink.warning(
            DiagCode::Sslic1102,
            stmt.span,
            format!(
                "vocabulary {vocabulary} is newer than supported vocabulary \
                 {SUPPORTED_VOCABULARY}; unknown declarations are preserved"
            ),
        );
    }
}

fn parse_testssl_import(stmt: &Stmt, model: &mut SslicModel, sink: &mut Sink) {
    if !stmt.require_words(3, sink) {
        return;
    }
    let name = stmt.word(1).text.clone();
    let Some(fields) = Document::block(stmt.word(2), sink) else {
        return;
    };
    let mut schema = None;
    let mut raw_json = None;
    for field in &fields {
        if field.name().is_empty() {
            continue;
        }
        if !field.require_literals(sink) || !field.require_words(2, sink) {
            continue;
        }
        let value = field.word(1);
        match field.name() {
            "schema" => schema = Some(value.text.clone()),
            "raw-json-hex" => match decode_hex(&value.text) {
                Ok(bytes) => match String::from_utf8(bytes) {
                    Ok(text) => raw_json = Some(text),
                    Err(_) => sink.error(
                        DiagCode::Sslic1009,
                        value.span,
                        "`raw-json-hex` is not UTF-8 JSON",
                    ),
                },
                Err(message) => sink.error(DiagCode::Sslic1009, value.span, message),
            },
            other => sink.error(
                DiagCode::Sslic1007,
                field.span,
                format!("unknown `testssl-import` member `{other}`"),
            ),
        }
    }
    if schema.as_deref() != Some("1") {
        sink.error(
            DiagCode::Sslic1009,
            stmt.span,
            "`testssl-import` schema must be 1",
        );
        return;
    }
    let Some(source) = raw_json else {
        sink.error(
            DiagCode::Sslic1010,
            stmt.span,
            "`testssl-import` requires `raw-json-hex`",
        );
        return;
    };
    let imported = match import_testssl_json(&source) {
        Ok(imported) => imported,
        Err(error) => {
            sink.error(DiagCode::Sslic1009, stmt.span, error.to_string());
            return;
        }
    };
    if model.testssl_imports.contains_key(&name) {
        sink.error(
            DiagCode::Sslic1008,
            stmt.span,
            format!("duplicate testssl import `{name}`"),
        );
        return;
    }
    model.testssl_imports.insert(name, imported);
}

fn decode_hex(value: &str) -> Result<Vec<u8>, String> {
    if !value.len().is_multiple_of(2) {
        return Err("hexadecimal value must contain an even number of digits".to_owned());
    }
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let text = std::str::from_utf8(pair).unwrap_or_default();
            u8::from_str_radix(text, 16)
                .map_err(|_| "hexadecimal value contains a non-hex digit".to_owned())
        })
        .collect()
}

fn parse_certificate(stmt: &Stmt, model: &mut SslicModel, sink: &mut Sink) {
    if !stmt.require_words(3, sink) {
        return;
    }
    let name = stmt.word(1).text.clone();
    if model.certificates.contains_key(&name) {
        sink.error(
            DiagCode::Sslic1008,
            stmt.span,
            format!("duplicate certificate `{name}`"),
        );
        return;
    }
    let Some(fields) = Document::block(stmt.word(2), sink) else {
        return;
    };
    let mut declaration = CertificateDeclaration {
        name: name.clone(),
        material: String::new(),
        key: None,
        extensions: BTreeMap::new(),
    };
    for field in &fields {
        if field.name().is_empty() {
            continue;
        }
        if !field.require_literals(sink) {
            continue;
        }
        match field.name() {
            "pem" | "material" => {
                if field.require_words(2, sink) {
                    declaration.material.clone_from(&field.word(1).text);
                }
            }
            "key" => {
                if field.require_words(2, sink) {
                    declaration.key = Some(field.word(1).text.clone());
                }
            }
            _ => add_extension(&mut declaration.extensions, field, sink),
        }
    }
    if declaration.material.is_empty() {
        sink.error(
            DiagCode::Sslic1010,
            stmt.span,
            format!("certificate `{name}` has no `pem` or `material` member"),
        );
        return;
    }
    model.certificates.insert(name, declaration);
}

fn parse_endpoint(stmt: &Stmt, model: &mut SslicModel, sink: &mut Sink) {
    if !stmt.require_words(3, sink) {
        return;
    }
    let name = stmt.word(1).text.clone();
    if model.endpoints.contains_key(&name) {
        sink.error(
            DiagCode::Sslic1008,
            stmt.span,
            format!("duplicate endpoint `{name}`"),
        );
        return;
    }
    let Some(fields) = Document::block(stmt.word(2), sink) else {
        return;
    };
    let mut endpoint = Endpoint {
        name: name.clone(),
        ..Endpoint::default()
    };
    for field in &fields {
        if field.name().is_empty() {
            continue;
        }
        if !field.require_literals(sink) {
            continue;
        }
        match field.name() {
            "hostname" => {
                if field.require_words(2, sink) {
                    endpoint.hostname = Some(field.word(1).text.clone());
                }
            }
            "protocols" => {
                if field.require_words(2, sink) {
                    endpoint.protocols = protocol_list(field.word(1), sink);
                }
            }
            "ciphers" => {
                if field.require_words(2, sink)
                    && let Some(list) = Document::literal_list(field.word(1), sink)
                {
                    endpoint.ciphers = list;
                }
            }
            "groups" => {
                if field.require_words(2, sink)
                    && let Some(list) = Document::literal_list(field.word(1), sink)
                {
                    endpoint.groups = list;
                }
            }
            "signature-schemes" => {
                if field.require_words(2, sink)
                    && let Some(list) = Document::literal_list(field.word(1), sink)
                {
                    endpoint.signature_schemes = list;
                }
            }
            "certificate-chain" => {
                if field.require_words(2, sink)
                    && let Some(list) = Document::literal_list(field.word(1), sink)
                {
                    endpoint.certificate_chain = list;
                }
            }
            "hsts" => {
                if let Some(policy) = parse_hsts(field, sink) {
                    endpoint.hsts = Some(policy);
                }
            }
            _ => add_extension(&mut endpoint.extensions, field, sink),
        }
    }
    model.endpoints.insert(name, endpoint);
}

fn protocol_list(word: &Word, sink: &mut Sink) -> Vec<ProtocolVersion> {
    let Some(list) = Document::literal_list(word, sink) else {
        return Vec::new();
    };
    let mut protocols: Vec<ProtocolVersion> = list
        .iter()
        .filter_map(|value| match value.parse::<ProtocolVersion>() {
            Ok(protocol) => Some(protocol),
            Err(message) => {
                sink.error(DiagCode::Sslic1009, word.span, message);
                None
            }
        })
        .collect();
    protocols.sort_unstable();
    protocols.dedup();
    protocols
}

fn parse_hsts(stmt: &Stmt, sink: &mut Sink) -> Option<HstsPolicy> {
    if !stmt.require_words(2, sink) {
        return None;
    }
    let fields = Document::block(stmt.word(1), sink)?;
    let mut policy = HstsPolicy::default();
    for field in &fields {
        if field.name().is_empty() {
            continue;
        }
        if !field.require_literals(sink) || !field.require_words(2, sink) {
            continue;
        }
        let value = field.word(1);
        match field.name() {
            "enabled" => {
                if let Some(flag) = parse_bool(value, sink) {
                    policy.enabled = flag;
                }
            }
            "max-age" => {
                if let Some(seconds) = parse_unsigned(value, sink) {
                    policy.max_age = Some(seconds);
                }
            }
            "include-subdomains" => {
                if let Some(flag) = parse_bool(value, sink) {
                    policy.include_subdomains = flag;
                }
            }
            "preload" => {
                if let Some(flag) = parse_bool(value, sink) {
                    policy.preload = flag;
                }
            }
            other => sink.error(
                DiagCode::Sslic1007,
                field.span,
                format!("unknown `hsts` member `{other}`"),
            ),
        }
    }
    Some(policy)
}

fn parse_bool(word: &Word, sink: &mut Sink) -> Option<bool> {
    match word.text.to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => Some(true),
        "false" | "no" | "off" | "0" => Some(false),
        other => {
            sink.error(
                DiagCode::Sslic1009,
                word.span,
                format!("expected a boolean, got `{other}`"),
            );
            None
        }
    }
}

fn parse_unsigned(word: &Word, sink: &mut Sink) -> Option<u64> {
    let Ok(value) = word.text.parse::<u64>() else {
        sink.error(
            DiagCode::Sslic1009,
            word.span,
            format!("expected an unsigned integer, got `{}`", word.text),
        );
        return None;
    };
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r"
sslictcl 1
certificate leaf {
    pem {-----BEGIN CERTIFICATE-----
not-real
-----END CERTIFICATE-----}
    key leaf-key
    future-field {kept exactly}
}
endpoint /Common/example {
    hostname example.test
    protocols {tls1.2 tls1.3 tls1.2}
    ciphers {TLS_AES_128_GCM_SHA256 ECDHE-RSA-AES128-GCM-SHA256}
    certificate-chain {leaf intermediate}
    hsts {
        enabled true
        max-age 31536000
        include-subdomains yes
        preload false
    }
    adapter-source bigip
}
future-top value
";

    fn errors(loaded: &DslLoad) -> Vec<&DslDiagnostic> {
        loaded
            .diagnostics
            .iter()
            .filter(|item| item.severity == DslSeverity::Error)
            .collect()
    }

    #[test]
    fn loads_literal_document_and_preserves_unknowns() {
        let loaded = load(SAMPLE).expect("valid DSL");
        let endpoint = &loaded.model.endpoints["/Common/example"];
        assert_eq!(
            endpoint.protocols,
            vec![ProtocolVersion::Tls12, ProtocolVersion::Tls13]
        );
        assert_eq!(endpoint.certificate_chain, ["leaf", "intermediate"]);
        assert_eq!(
            endpoint.hsts.as_ref().and_then(|hsts| hsts.max_age),
            Some(31_536_000)
        );
        assert_eq!(loaded.notices.len(), 3);
        assert!(loaded.model.extensions.contains_key("future-top"));
    }

    #[test]
    fn rejects_all_substitution_forms() {
        for source in [
            "sslictcl 1\nendpoint x { hostname $host }",
            "sslictcl 1\nendpoint x { hostname [exec id] }",
            "sslictcl 1\nendpoint x { ciphers {*}$values }",
        ] {
            let error = load(source).expect_err("dynamic DSL must be rejected");
            assert_eq!(error.code, DiagCode::Sslic1002);
        }
    }

    #[test]
    fn requires_header_and_unique_names() {
        assert_eq!(load("endpoint x {}").unwrap_err().code, DiagCode::Sslic1003);
        assert_eq!(
            load("sslictcl 1\nendpoint x {}\nendpoint x {}")
                .unwrap_err()
                .code,
            DiagCode::Sslic1008
        );
        assert_eq!(
            load("sslictcl 1\nendpoint x {").unwrap_err().code,
            DiagCode::Sslic1001
        );
        assert_eq!(
            load("sslictcl 1\nsslictcl 1").unwrap_err().code,
            DiagCode::Sslic1004
        );
    }

    #[test]
    fn nested_member_ranges_are_absolute_document_offsets() {
        let source = "sslictcl 1\nendpoint x {\n    hsts {\n        enabled maybe\n    }\n}\n";
        let loaded = load_with_diagnostics(source);
        let found = errors(&loaded);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].code, DiagCode::Sslic1009);
        let start = source.find("maybe").unwrap();
        assert_eq!(found[0].range.start() as usize, start);
        assert_eq!(found[0].range.end() as usize, start + "maybe".len());
    }

    #[test]
    fn recovers_and_reports_every_independent_error() {
        let source = concat!(
            "sslictcl 1\n",
            "certificate a {\n    key only\n}\n",
            "endpoint good {\n    hostname good.test\n}\n",
            "endpoint bad {\n    hsts {\n        nonsense 1\n    }\n}\n",
            "endpoint good {\n}\n",
        );
        let loaded = load_with_diagnostics(source);
        let codes: Vec<DiagCode> = errors(&loaded).iter().map(|item| item.code).collect();
        assert_eq!(
            codes,
            vec![
                DiagCode::Sslic1010,
                DiagCode::Sslic1007,
                DiagCode::Sslic1008
            ]
        );
        let document = loaded.document.expect("recovered document");
        assert!(document.model.endpoints.contains_key("good"));
        assert!(document.model.endpoints.contains_key("bad"));
        assert!(!document.model.certificates.contains_key("a"));
    }

    #[test]
    fn newer_vocabulary_is_a_warning_not_an_error() {
        let loaded = load_with_diagnostics("sslictcl 2\n");
        assert!(errors(&loaded).is_empty());
        assert_eq!(loaded.diagnostics[0].code, DiagCode::Sslic1102);
        assert_eq!(loaded.diagnostics[0].severity, DslSeverity::Warning);
        assert_eq!(loaded.document.unwrap().model.vocabulary, 2);
    }

    #[test]
    fn wrong_word_count_and_unbraced_body_are_coded() {
        assert_eq!(
            load("sslictcl 1\nendpoint x").unwrap_err().code,
            DiagCode::Sslic1005
        );
        assert_eq!(
            load("sslictcl 1\nendpoint x body").unwrap_err().code,
            DiagCode::Sslic1006
        );
    }
}
