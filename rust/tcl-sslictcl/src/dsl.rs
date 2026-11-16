// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
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

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use tcl_compiler::parsing::syntax::build::build_document;
use tcl_compiler::parsing::syntax::segment::segments_from_document;
use tcl_compiler::segmenter::SegmentedCommand;
use tcl_core_types::DiagCode;
use tcl_lexer::{
    LexerConfig, LineIndex, SourceImage, SourceMap, Span, TokenType, script_image_is_complete,
};
use tcl_syntax::list::split_list;

use crate::estimate::{EstimateSeverity, Grade};
use crate::model::{
    CertificateDeclaration, ChainDeclaration, CipherFact, Endpoint, GradeRule, HstsPolicy, Policy,
    PolicyCheck, ProtocolFact, ProtocolVersion, SslicModel, TlsStatus, TlsValue,
    TrustAnchorDeclaration, TrustProgramDeclaration,
};
use crate::testssl::import_testssl_json;
use crate::trust::{ClientFamily, TrustPurpose};

/// The `SslicTcl` vocabulary version this build implements.
pub const SUPPORTED_VOCABULARY: u32 = 1;

/// Check identifier reserved for the `grade` rule.
///
/// A policy finding is identified by `(check_id, endpoint)` and the grade rule
/// emits its finding under `grade`, so a `check grade { … }` would collide with
/// it. Declaring one is `SSLIC1009`.
pub const RESERVED_CHECK_ID: &str = "grade";

/// A successfully loaded document and its forwards-compatibility notices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DslDocument {
    /// Byte-for-byte original source. Unknown declarations can be re-emitted
    /// from this rather than from the normalised semantic projection.
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
/// The lexing grammar for a whole `.sslictcl` file.
///
/// Derived from the `sslictcl` dialect profile through the owner every LSP
/// provider uses ([`LexerConfig::for_file_dialect`]), never from a copied
/// field list: the profile selects `GRAMMAR_TCL9X`, whose
/// `script_skips_leading_bom` is what makes a byte-order mark a file prologue
/// rather than part of the first word. `LexerConfig::default()` reads the mark
/// as content, so a BOM-prefixed document would segment as
/// `\u{FEFF}sslictcl` and report a missing header (`SSLIC1003`) for a
/// perfectly good file.
///
/// Nested body slices demote this with [`LexerConfig::nested`]: a mark inside
/// a braced body is data.
fn file_config() -> LexerConfig {
    LexerConfig::for_file_dialect("sslictcl")
}

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
    ///
    /// `config` is the lexing grammar, which the caller owns because the
    /// byte-order-mark rule differs by position: see [`file_config`].
    fn segment(slice: &str, base: u32, config: LexerConfig, sink: &mut Sink) -> Option<Vec<Stmt>> {
        let whole = Span::new(base, base + u32_len(slice));
        match script_image_is_complete(&SourceImage::document(slice), config) {
            Ok(true) => {}
            Ok(false) => {
                sink.error(
                    DiagCode::Sslic1001,
                    whole,
                    "incomplete SslicTcl declaration or unclosed delimiter",
                );
                return None;
            }
            Err(error) => {
                sink.error(DiagCode::Sslic1001, whole, error.to_string());
                return None;
            }
        }
        let source_map = SourceMap::new(slice);
        let (document, warnings) = build_document(slice, config);
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
        // A mark at the head of a nested body slice is data, not a file
        // prologue, so the body re-lex is demoted.
        Self::segment(&word.text, word.content_start, file_config().nested(), sink)
    }

    /// One `LIST` value: a braced Tcl list, or a single bare word.
    ///
    /// A braced word is verbatim by Tcl's own rules — nothing inside it is
    /// substituted — so the only grammar that applies is the list grammar, and
    /// that is owned by [`tcl_syntax::list`] (`Tcl_SplitList`), never
    /// re-derived here. Segmenting the body as a *command* instead would
    /// reject a perfectly literal `{[A-Z]*}` glob as command substitution and
    /// would split on `;` and newlines, which are not list separators.
    fn literal_list(word: &Word, sink: &mut Sink) -> Option<Vec<String>> {
        if word.braced {
            return match split_list(&word.text) {
                Ok(items) => Some(items.into_iter().map(Cow::into_owned).collect()),
                Err(error) => {
                    sink.error(
                        DiagCode::Sslic1009,
                        word.span,
                        error.full_message(&word.text),
                    );
                    None
                }
            };
        }
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
        Some(vec![word.text.clone()])
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
    let mut pending = Vec::new();
    let mut saw_header = false;

    let Some(rows) = Document::segment(source, 0, file_config(), &mut sink) else {
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
            "endpoint" => parse_endpoint(stmt, &mut model, &mut pending, &mut sink),
            "testssl-import" => parse_testssl_import(stmt, &mut model, &mut sink),
            "trust-program" => parse_trust_program(stmt, &mut model, &mut sink),
            "protocol" => parse_protocol_fact(stmt, &mut model, &mut sink),
            "cipher" => parse_cipher_fact(stmt, &mut model, &mut sink),
            "chain" => parse_chain(stmt, &mut model, &mut pending, &mut sink),
            "policy" => parse_policy(stmt, &mut model, &mut sink),
            _ => add_extension(&mut model.extensions, stmt, &mut sink),
        }
    }
    resolve_references(&mut model, &pending, &mut sink);

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

/// Mutable state one endpoint's member walk builds up.
struct EndpointBuild<'a> {
    endpoint: Endpoint,
    pending: &'a mut Vec<PendingRef>,
    /// Span of a `chain NAME` member, when one was declared.
    named_chain: Option<Span>,
    /// Span of a literal `certificate-chain LIST` member, when one was
    /// declared.
    literal_chain: Option<Span>,
}

fn parse_endpoint(
    stmt: &Stmt,
    model: &mut SslicModel,
    pending: &mut Vec<PendingRef>,
    sink: &mut Sink,
) {
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
    let mut build = EndpointBuild {
        endpoint: Endpoint {
            name: name.clone(),
            ..Endpoint::default()
        },
        pending,
        named_chain: None,
        literal_chain: None,
    };
    for field in &fields {
        if field.name().is_empty() || !field.require_literals(sink) {
            continue;
        }
        apply_endpoint_member(field, &mut build, sink);
    }
    if let (Some(named), Some(literal)) = (build.named_chain, build.literal_chain) {
        sink.error(
            DiagCode::Sslic1012,
            Span::new(
                named.start().min(literal.start()),
                named.end().max(literal.end()),
            ),
            format!(
                "endpoint `{name}` declares both `chain` and `certificate-chain`; they are \
                 mutually exclusive"
            ),
        );
    }
    model.endpoints.insert(name, build.endpoint);
}

fn apply_endpoint_member(field: &Stmt, build: &mut EndpointBuild<'_>, sink: &mut Sink) {
    if field.name() == "hsts" {
        if let Some(policy) = parse_hsts(field, sink) {
            build.endpoint.hsts = Some(policy);
        }
        return;
    }
    let known = matches!(
        field.name(),
        "hostname"
            | "protocols"
            | "ciphers"
            | "groups"
            | "signature-schemes"
            | "certificate-chain"
            | "chain"
            | "policy"
    );
    if !known {
        add_extension(&mut build.endpoint.extensions, field, sink);
        return;
    }
    if !field.require_words(2, sink) {
        return;
    }
    let value = field.word(1);
    match field.name() {
        "hostname" => build.endpoint.hostname = Some(value.text.clone()),
        "protocols" => build.endpoint.protocols = protocol_list(value, sink),
        "ciphers" => {
            if let Some(list) = Document::literal_list(value, sink) {
                build.endpoint.ciphers = list;
            }
        }
        "groups" => {
            if let Some(list) = Document::literal_list(value, sink) {
                build.endpoint.groups = list;
            }
        }
        "signature-schemes" => {
            if let Some(list) = Document::literal_list(value, sink) {
                build.endpoint.signature_schemes = list;
            }
        }
        "certificate-chain" => {
            if let Some(list) = Document::literal_list(value, sink) {
                build.literal_chain = Some(field.span);
                build.endpoint.certificate_chain = list;
            }
        }
        "chain" => {
            build.named_chain = Some(field.span);
            build.endpoint.chain = Some(value.text.clone());
            build.pending.push(PendingRef {
                kind: RefKind::EndpointChain,
                owner: build.endpoint.name.clone(),
                name: value.text.clone(),
                range: value.span,
            });
        }
        _ => {
            build.endpoint.policy = Some(value.text.clone());
            build.pending.push(PendingRef {
                kind: RefKind::EndpointPolicy,
                owner: build.endpoint.name.clone(),
                name: value.text.clone(),
                range: value.span,
            });
        }
    }
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

/// A name used before the declaration it refers to has necessarily been read.
/// Resolution is a post-pass, so declaration order is irrelevant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RefKind {
    EndpointChain,
    EndpointPolicy,
    ChainCertificate,
}

#[derive(Debug, Clone)]
struct PendingRef {
    kind: RefKind,
    owner: String,
    name: String,
    range: Span,
}

fn parse_chain(
    stmt: &Stmt,
    model: &mut SslicModel,
    pending: &mut Vec<PendingRef>,
    sink: &mut Sink,
) {
    if !stmt.require_words(3, sink) {
        return;
    }
    let name = stmt.word(1).text.clone();
    if model.chains.contains_key(&name) {
        sink.error(
            DiagCode::Sslic1008,
            stmt.span,
            format!("duplicate chain `{name}`"),
        );
        return;
    }
    let Some(fields) = Document::block(stmt.word(2), sink) else {
        return;
    };
    let mut declaration = ChainDeclaration {
        name: name.clone(),
        certificates: Vec::new(),
    };
    let mut saw_certificates = false;
    for field in &fields {
        if field.name().is_empty() {
            continue;
        }
        if !field.require_literals(sink) {
            continue;
        }
        match field.name() {
            "certificates" => {
                if field.require_words(2, sink)
                    && let Some(list) = Document::literal_list(field.word(1), sink)
                {
                    saw_certificates = true;
                    for entry in &list {
                        pending.push(PendingRef {
                            kind: RefKind::ChainCertificate,
                            owner: name.clone(),
                            name: entry.clone(),
                            range: field.word(1).span,
                        });
                    }
                    declaration.certificates = list;
                }
            }
            other => sink.error(
                DiagCode::Sslic1007,
                field.span,
                format!("unknown `chain` member `{other}`"),
            ),
        }
    }
    if !saw_certificates {
        sink.error(
            DiagCode::Sslic1010,
            stmt.span,
            format!("chain `{name}` has no `certificates` member"),
        );
        return;
    }
    model.chains.insert(name, declaration);
}

fn parse_trust_program(stmt: &Stmt, model: &mut SslicModel, sink: &mut Sink) {
    if !stmt.require_words(3, sink) {
        return;
    }
    let name = stmt.word(1).text.clone();
    if model.trust_programs.contains_key(&name) {
        sink.error(
            DiagCode::Sslic1008,
            stmt.span,
            format!("duplicate trust program `{name}`"),
        );
        return;
    }
    let Some(fields) = Document::block(stmt.word(2), sink) else {
        return;
    };
    let mut declaration = TrustProgramDeclaration {
        name: name.clone(),
        client: ClientFamily::Mozilla,
        version: String::new(),
        generated_at: String::new(),
        source_name: String::new(),
        source_url: String::new(),
        source_revision: String::new(),
        source_license: String::new(),
        anchors: BTreeMap::new(),
        extensions: BTreeMap::new(),
    };
    let mut saw_client = false;
    for field in &fields {
        if field.name().is_empty() {
            continue;
        }
        if !field.require_literals(sink) {
            continue;
        }
        if field.name() == "anchor" {
            parse_anchor(field, &mut declaration, sink);
            continue;
        }
        let text_member = matches!(
            field.name(),
            "client"
                | "version"
                | "generated-at"
                | "source-name"
                | "source-url"
                | "source-revision"
                | "source-license"
        );
        if !text_member {
            add_extension(&mut declaration.extensions, field, sink);
            continue;
        }
        if !field.require_words(2, sink) {
            continue;
        }
        let value = field.word(1);
        match field.name() {
            "client" => match value.text.parse::<ClientFamily>() {
                Ok(client) => {
                    declaration.client = client;
                    saw_client = true;
                }
                Err(message) => sink.error(DiagCode::Sslic1009, value.span, message),
            },
            "version" => declaration.version.clone_from(&value.text),
            "generated-at" => declaration.generated_at.clone_from(&value.text),
            "source-name" => declaration.source_name.clone_from(&value.text),
            "source-url" => declaration.source_url.clone_from(&value.text),
            "source-revision" => declaration.source_revision.clone_from(&value.text),
            _ => declaration.source_license.clone_from(&value.text),
        }
    }
    if !saw_client {
        sink.error(
            DiagCode::Sslic1010,
            stmt.span,
            format!("trust program `{name}` has no `client` member"),
        );
        return;
    }
    model.trust_programs.insert(name, declaration);
}

fn parse_anchor(stmt: &Stmt, program: &mut TrustProgramDeclaration, sink: &mut Sink) {
    if !stmt.require_words(3, sink) {
        return;
    }
    let key = stmt.word(1);
    let fingerprint = key.text.to_ascii_lowercase();
    if fingerprint.len() != 64 || !fingerprint.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        sink.error(
            DiagCode::Sslic1009,
            key.span,
            "anchor name must be 64 hexadecimal digits (a SHA-256 DER fingerprint)",
        );
        return;
    }
    if program.anchors.contains_key(&fingerprint) {
        sink.error(
            DiagCode::Sslic1008,
            stmt.span,
            format!("duplicate anchor `{fingerprint}`"),
        );
        return;
    }
    let Some(fields) = Document::block(stmt.word(2), sink) else {
        return;
    };
    let mut anchor = TrustAnchorDeclaration {
        fingerprint_sha256: fingerprint.clone(),
        ..TrustAnchorDeclaration::default()
    };
    for field in &fields {
        if field.name().is_empty() {
            continue;
        }
        if !field.require_literals(sink) || !field.require_words(2, sink) {
            continue;
        }
        let value = field.word(1);
        match field.name() {
            "subject" => anchor.subject.clone_from(&value.text),
            "der-base64" => anchor.der_base64 = Some(value.text.clone()),
            "purposes" => {
                if let Some(list) = Document::literal_list(value, sink) {
                    anchor.purposes = list
                        .iter()
                        .filter_map(|entry| match entry.parse::<TrustPurpose>() {
                            Ok(purpose) => Some(purpose),
                            Err(message) => {
                                sink.error(DiagCode::Sslic1009, value.span, message);
                                None
                            }
                        })
                        .collect();
                }
            }
            "trusted" => {
                if let Some(flag) = parse_bool(value, sink) {
                    anchor.trusted = flag;
                }
            }
            "distrust-after" => {
                if let Some(seconds) = parse_unsigned(value, sink) {
                    anchor.distrust_after = i64::try_from(seconds).ok();
                }
            }
            other => sink.error(
                DiagCode::Sslic1007,
                field.span,
                format!("unknown `anchor` member `{other}`"),
            ),
        }
    }
    program.anchors.insert(fingerprint, anchor);
}

fn parse_protocol_fact(stmt: &Stmt, model: &mut SslicModel, sink: &mut Sink) {
    if !stmt.require_words(3, sink) {
        return;
    }
    let key = stmt.word(1);
    let version = match key.text.parse::<ProtocolVersion>() {
        Ok(version) => version,
        Err(message) => {
            sink.error(DiagCode::Sslic1009, key.span, message);
            return;
        }
    };
    if model.facts.protocols.contains_key(&version) {
        sink.error(
            DiagCode::Sslic1008,
            stmt.span,
            format!("duplicate protocol fact `{version}`"),
        );
        return;
    }
    let Some(fields) = Document::block(stmt.word(2), sink) else {
        return;
    };
    let mut fact = ProtocolFact::default();
    for field in &fields {
        if field.name().is_empty() {
            continue;
        }
        if !field.require_literals(sink) || !field.require_words(2, sink) {
            continue;
        }
        let value = field.word(1);
        match field.name() {
            "status" => match value.text.parse::<TlsStatus>() {
                Ok(status) => fact.status = Some(status),
                Err(message) => sink.error(DiagCode::Sslic1009, value.span, message),
            },
            "score" => {
                if let Some(score) = parse_unsigned(value, sink) {
                    match u8::try_from(score) {
                        Ok(score) if score <= 100 => fact.score = Some(score),
                        _ => sink.error(
                            DiagCode::Sslic1009,
                            value.span,
                            "`score` must be between 0 and 100",
                        ),
                    }
                }
            }
            "reference" => fact.reference = Some(value.text.clone()),
            other => sink.error(
                DiagCode::Sslic1007,
                field.span,
                format!("unknown `protocol` member `{other}`"),
            ),
        }
    }
    model.facts.protocols.insert(version, fact);
}

fn parse_cipher_fact(stmt: &Stmt, model: &mut SslicModel, sink: &mut Sink) {
    if !stmt.require_words(3, sink) {
        return;
    }
    let name = stmt.word(1).text.clone();
    if model.facts.ciphers.contains_key(&name) {
        sink.error(
            DiagCode::Sslic1008,
            stmt.span,
            format!("duplicate cipher fact `{name}`"),
        );
        return;
    }
    let Some(fields) = Document::block(stmt.word(2), sink) else {
        return;
    };
    let mut fact = CipherFact::default();
    for field in &fields {
        if field.name().is_empty() {
            continue;
        }
        if !field.require_literals(sink) || !field.require_words(2, sink) {
            continue;
        }
        let value = field.word(1);
        match field.name() {
            "iana-name" => fact.iana_name = Some(value.text.clone()),
            "openssl-name" => fact.openssl_name = Some(value.text.clone()),
            "key-exchange" => fact.key_exchange = Some(value.text.clone()),
            "authentication" => fact.authentication = Some(value.text.clone()),
            "encryption" => fact.encryption = Some(value.text.clone()),
            "bits" => {
                if let Some(bits) = parse_unsigned(value, sink) {
                    match u16::try_from(bits) {
                        Ok(bits) => fact.bits = Some(bits),
                        Err(_) => sink.error(
                            DiagCode::Sslic1009,
                            value.span,
                            "`bits` is out of range for a symmetric strength",
                        ),
                    }
                }
            }
            "forward-secrecy" => fact.forward_secrecy = parse_bool(value, sink),
            "aead" => fact.aead = parse_bool(value, sink),
            "status" => match value.text.parse::<TlsStatus>() {
                Ok(status) => fact.status = Some(status),
                Err(message) => sink.error(DiagCode::Sslic1009, value.span, message),
            },
            "protocols" => fact.protocols = protocol_list(value, sink),
            other => sink.error(
                DiagCode::Sslic1007,
                field.span,
                format!("unknown `cipher` member `{other}`"),
            ),
        }
    }
    model.facts.ciphers.insert(name, fact);
}

fn parse_policy(stmt: &Stmt, model: &mut SslicModel, sink: &mut Sink) {
    if !stmt.require_words(3, sink) {
        return;
    }
    let name = stmt.word(1).text.clone();
    if model.policies.contains_key(&name) {
        sink.error(
            DiagCode::Sslic1008,
            stmt.span,
            format!("duplicate policy `{name}`"),
        );
        return;
    }
    let Some(fields) = Document::block(stmt.word(2), sink) else {
        return;
    };
    let mut policy = Policy {
        name: name.clone(),
        ..Policy::default()
    };
    for field in &fields {
        if field.name().is_empty() {
            continue;
        }
        if !field.require_literals(sink) {
            continue;
        }
        match field.name() {
            "check" => parse_policy_check(field, &mut policy, sink),
            "grade" => parse_grade_rule(field, &mut policy, sink),
            other => sink.error(
                DiagCode::Sslic1007,
                field.span,
                format!("unknown `policy` member `{other}`"),
            ),
        }
    }
    model.policies.insert(name, policy);
}

fn parse_policy_check(stmt: &Stmt, policy: &mut Policy, sink: &mut Sink) {
    if !stmt.require_words(3, sink) {
        return;
    }
    let id = stmt.word(1).text.clone();
    if id == RESERVED_CHECK_ID {
        sink.error(
            DiagCode::Sslic1009,
            stmt.word(1).span,
            format!(
                "`{RESERVED_CHECK_ID}` is reserved as a check identifier; it names the \
                 finding the `grade` rule produces"
            ),
        );
        return;
    }
    if policy.checks.contains_key(&id) {
        sink.error(
            DiagCode::Sslic1008,
            stmt.span,
            format!("duplicate check `{id}` in policy `{}`", policy.name),
        );
        return;
    }
    let Some(fields) = Document::block(stmt.word(2), sink) else {
        return;
    };
    let mut check = PolicyCheck {
        id: id.clone(),
        ..PolicyCheck::default()
    };
    for field in &fields {
        if field.name().is_empty() {
            continue;
        }
        if !field.require_literals(sink) || !field.require_words(2, sink) {
            continue;
        }
        let value = field.word(1);
        match field.name() {
            "severity" => match value.text.parse::<EstimateSeverity>() {
                Ok(severity) => check.severity = Some(severity),
                Err(message) => sink.error(DiagCode::Sslic1009, value.span, message),
            },
            "message" => check.message = Some(value.text.clone()),
            "require-protocols" => check.require_protocols = protocol_list(value, sink),
            "forbid-protocols" => check.forbid_protocols = protocol_list(value, sink),
            "forbid-ciphers" => {
                if let Some(list) = Document::literal_list(value, sink) {
                    check.forbid_ciphers = list;
                }
            }
            "require-forward-secrecy" => check.require_forward_secrecy = parse_bool(value, sink),
            "min-key-bits" => check.min_key_bits = parse_unsigned(value, sink),
            "require-hsts" => check.require_hsts = parse_bool(value, sink),
            "min-hsts-max-age" => check.min_hsts_max_age = parse_unsigned(value, sink),
            "predicate" => {
                if value.braced {
                    check.predicate = Some(value.text.clone());
                    sink.hint(
                        DiagCode::Sslic1103,
                        value.span,
                        format!(
                            "`predicate` on check `{id}` is retained verbatim and never \
                             evaluated in vocabulary 1"
                        ),
                    );
                } else {
                    sink.error(
                        DiagCode::Sslic1006,
                        value.span,
                        "`predicate` must be a braced literal script",
                    );
                }
            }
            other => sink.error(
                DiagCode::Sslic1007,
                field.span,
                format!("unknown `check` member `{other}`"),
            ),
        }
    }
    policy.checks.insert(id, check);
}

fn parse_grade_rule(stmt: &Stmt, policy: &mut Policy, sink: &mut Sink) {
    if !stmt.require_words(2, sink) {
        return;
    }
    let Some(fields) = Document::block(stmt.word(1), sink) else {
        return;
    };
    let mut minimum = None;
    for field in &fields {
        if field.name().is_empty() {
            continue;
        }
        if !field.require_literals(sink) || !field.require_words(2, sink) {
            continue;
        }
        let value = field.word(1);
        match field.name() {
            "minimum" => match value.text.parse::<Grade>() {
                Ok(grade) => minimum = Some(grade),
                Err(message) => sink.error(DiagCode::Sslic1009, value.span, message),
            },
            other => sink.error(
                DiagCode::Sslic1007,
                field.span,
                format!("unknown `grade` member `{other}`"),
            ),
        }
    }
    let Some(minimum) = minimum else {
        sink.error(
            DiagCode::Sslic1010,
            stmt.span,
            "`grade` has no `minimum` member",
        );
        return;
    };
    policy.grade = Some(GradeRule { minimum });
}

/// Resolve every cross-declaration name once the whole document has been read,
/// so declaration order never matters.
fn resolve_references(model: &mut SslicModel, pending: &[PendingRef], sink: &mut Sink) {
    for reference in pending {
        match reference.kind {
            RefKind::EndpointChain => {
                let Some(chain) = model.chains.get(&reference.name) else {
                    sink.error(
                        DiagCode::Sslic1011,
                        reference.range,
                        format!(
                            "endpoint `{}` references undeclared chain `{}`",
                            reference.owner, reference.name
                        ),
                    );
                    continue;
                };
                let certificates = chain.certificates.clone();
                if let Some(endpoint) = model.endpoints.get_mut(&reference.owner) {
                    endpoint.certificate_chain = certificates;
                }
            }
            RefKind::EndpointPolicy => {
                if !model.policies.contains_key(&reference.name) {
                    sink.error(
                        DiagCode::Sslic1011,
                        reference.range,
                        format!(
                            "endpoint `{}` references undeclared policy `{}`",
                            reference.owner, reference.name
                        ),
                    );
                }
            }
            RefKind::ChainCertificate => {
                if !model.certificates.contains_key(&reference.name) {
                    sink.error(
                        DiagCode::Sslic1011,
                        reference.range,
                        format!(
                            "chain `{}` references undeclared certificate `{}`",
                            reference.owner, reference.name
                        ),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_configured_document_tests;
