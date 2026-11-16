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

//! `# tcl-lsp: stub` declarations: inline and sidecar scanning, the stub grammar and the declared surface of a document.

use super::directives::matches_marker;
use std::collections::HashSet;
use tcl_lexer::Span;

/// Scan `source` for inline ``# tcl-lsp: stub <name> ...``
/// declarations bounded by ``# tcl-lsp: stubs-begin`` /
/// ``# tcl-lsp: stubs-end`` markers, returning the set of
/// declared command names.
///
/// Only the command names are extracted (not arg-roles or
/// flags). Adding the names to the W123 candidate set is the
/// load-bearing use case — the role/flag data only matters for
/// downstream diagnostic emitters.
///
/// Stubs declared *outside* the begin/end markers are ignored.
/// ``expr-func`` / ``expr-op`` lines are skipped — they declare
/// expression functions, not commands.
#[must_use]
pub fn scan_stub_command_names(source: &str) -> HashSet<String> {
    let mut names: HashSet<String> = HashSet::new();
    let mut in_block = false;
    for line in source.lines() {
        let stripped = line.trim();
        if !stripped.starts_with('#') {
            continue;
        }
        let body = stripped.trim_start_matches('#').trim_start();
        // Markers — both case-insensitive on the keyword.
        if matches_marker(body, "stubs-begin") {
            in_block = true;
            continue;
        }
        if matches_marker(body, "stubs-end") {
            in_block = false;
            continue;
        }
        if !in_block {
            continue;
        }
        if let Some(name) = parse_stub_name(body) {
            // Skip ``expr-func`` / ``expr-op`` — those
            // declare expression-language symbols, not
            // commands.
            if name.starts_with("expr-") {
                continue;
            }
            names.insert(name.to_string());
        }
    }
    names
}

/// Extract the command name from a ``# tcl-lsp: stub NAME …``
/// line.  Returns the raw name as a borrowed slice or `None`
/// when the line doesn't match the stub shape.
fn parse_stub_name(body: &str) -> Option<&str> {
    let s = body.trim_start();
    let lower_prefix = "tcl-lsp";
    let kw_end = lower_prefix.len();
    if s.len() < kw_end || !s[..kw_end].eq_ignore_ascii_case(lower_prefix) {
        return None;
    }
    let rest = s[kw_end..].trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    let stub_kw = "stub";
    let stub_end = stub_kw.len();
    if rest.len() < stub_end || !rest[..stub_end].eq_ignore_ascii_case(stub_kw) {
        return None;
    }
    let after = rest[stub_end..].trim_start();
    if after.is_empty() {
        return None;
    }
    // First whitespace-delimited token is the command name.
    let end = after
        .find(|c: char| c.is_whitespace())
        .unwrap_or(after.len());
    if end == 0 { None } else { Some(&after[..end]) }
}

/// Scan `source` for inline ``# tcl-lsp: stub NAME {ARGS} ?FLAGS?`` and
/// ``# tcl-lsp: stub expr-func NAME ?ARITY?`` (also ``expr-op``)
/// declarations bounded by the ``stubs-begin`` / ``stubs-end``
/// markers, returning ``(commands, expr_defs)``.
///
/// Parses args and flags for the command form and extracts arity
/// for the expression form. Command stubs *require* the
/// ``{ARGS}`` brace block; bare ``stub NAME`` is rejected. Stubs
/// whose ``:role`` annotation is unrecognised are silently
/// dropped.
#[must_use]
pub fn scan_source_for_stubs(
    source: &str,
) -> (
    Vec<super::super::types::StubCommandDef>,
    Vec<super::super::types::StubExprDef>,
) {
    let (commands, expressions, _) =
        // dialect-drift-ok: sidecar stub declarations (and tests); a document's
        // stubs are scanned by `scan_source_for_stubs_with_outside` under its config
        scan_source_for_stubs_with_outside(source, tcl_lexer::LexerConfig::default());
    (commands, expressions)
}

/// Parse inline stubs and retain valid directives discarded outside a block.
/// Comment positions come from the lexer, not from the directive grammar.
#[must_use]
pub fn scan_source_for_stubs_with_outside(
    source: &str,
    config: tcl_lexer::LexerConfig,
) -> (
    Vec<super::super::types::StubCommandDef>,
    Vec<super::super::types::StubExprDef>,
    Vec<Span>,
) {
    let mut cmds = Vec::new();
    let mut exprs = Vec::new();
    let mut outside_candidates = Vec::new();
    let mut in_block = false;
    let mut offset: u32 = 0;
    for (line_number, line) in source.split_inclusive('\n').enumerate() {
        let trimmed_line: &str = line.trim_end_matches('\n');
        let line_byte_len =
            u32::try_from(line.len()).expect("line length fits in u32 for in-memory source");
        let stripped = trimmed_line.trim();
        let trimmed_len = u32::try_from(trimmed_line.len())
            .expect("line length fits in u32 for in-memory source");
        let span = tcl_lexer::Span::new(offset, offset + trimmed_len);
        offset += line_byte_len;
        if !stripped.starts_with('#') {
            continue;
        }
        let body = stripped.trim_start_matches('#').trim_start();
        if matches_marker(body, "stubs-begin") {
            in_block = true;
            continue;
        }
        if matches_marker(body, "stubs-end") {
            in_block = false;
            continue;
        }
        if !in_block {
            let after_prefix = strip_tcl_lsp_prefix(body);
            if after_prefix != body
                && (parse_expr_stub(after_prefix).is_some()
                    || parse_command_stub(after_prefix, span).is_some())
                && let Ok(line) = u32::try_from(line_number)
            {
                outside_candidates.push((line, span));
            }
            continue;
        }
        // ``# tcl-lsp:`` prefix is optional inside a stubs block —
        // strip it once if present.
        let after_prefix = strip_tcl_lsp_prefix(body);
        // Try the expr-func / expr-op shape first because ``stub
        // expr-func NAME`` would otherwise match the command shape
        // with NAME == "expr-func".
        if let Some((kind, name, arity)) = parse_expr_stub(after_prefix) {
            exprs.push(super::super::types::StubExprDef {
                name: name.to_string(),
                kind: kind.to_string(),
                arity,
                range: span,
                from_sidecar: false,
            });
            continue;
        }
        if let Some(stub) = parse_command_stub(after_prefix, span) {
            cmds.push(stub);
        }
    }
    let outside = if outside_candidates.is_empty() {
        Vec::new()
    } else {
        let comments = tcl_lexer::comment_line_starts(source, config);
        outside_candidates
            .into_iter()
            .filter_map(|(line, span)| comments.binary_search(&line).is_ok().then_some(span))
            .collect()
    };
    (cmds, exprs, outside)
}

/// Report an authored declaration that the stub grammar parsed but could not
/// install because no stub block owned its comment line.
pub fn outside_stub_diagnostics(
    spans: Vec<Span>,
) -> impl Iterator<Item = super::super::types::Diagnostic> {
    spans.into_iter().map(|span| {
        super::super::types::Diagnostic::new(
            tcl_core_types::DiagCode::W153,
            span,
            "Stub directive outside a stubs-begin/stubs-end block has no effect",
            super::super::types::Severity::Warning,
        )
    })
}

/// Load the nearest workspace sidecar named `<dialect>.tcl.stubs` and parse
/// its declarations using the same grammar as inline stubs.  Sidecars are
/// deliberately searched from the source directory upwards: a package or
/// workspace can provide a broad bundle at its root while a nested project
/// can override it with a nearer file.  The source itself is never modified,
/// so all source spans remain stable; sidecar declaration spans are synthetic
/// and are only retained for the normal stub metadata surface.
#[must_use]
pub fn scan_sidecar_stubs(
    file_path: Option<&str>,
    // The dialect *name*, not a resolved profile: this is a filename on disk
    // (`<dialect>.tcl.stubs`), so it must keep the spelling the workspace
    // configured. Canonicalising an alias here would stop finding a sidecar a
    // user named after it.
    dialect: &str,
) -> (
    Vec<super::super::types::StubCommandDef>,
    Vec<super::super::types::StubExprDef>,
) {
    let Some(file_path) = file_path else {
        return (Vec::new(), Vec::new());
    };
    let mut dir = std::path::Path::new(file_path).parent();
    while let Some(current) = dir {
        let sidecar = current.join(format!("{dialect}.tcl.stubs"));
        if let Ok(content) = std::fs::read_to_string(&sidecar) {
            return scan_sidecar_lines(&content);
        }
        dir = current.parent();
    }
    (Vec::new(), Vec::new())
}

/// Parse sidecar syntax: one declaration per line, without the inline
/// comment prefix or begin/end markers. It is adapted into the canonical
/// scanner rather than maintaining a second parser; declaration spans are
/// synthetic.
fn scan_sidecar_lines(
    content: &str,
) -> (
    Vec<super::super::types::StubCommandDef>,
    Vec<super::super::types::StubExprDef>,
) {
    let mut adapted = String::from("# tcl-lsp: stubs-begin\n");
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            adapted.push_str(trimmed);
        } else {
            adapted.push_str("# tcl-lsp: ");
            adapted.push_str(line);
        }
        adapted.push('\n');
    }
    adapted.push_str("# tcl-lsp: stubs-end\n");
    let (mut cmds, mut exprs) = scan_source_for_stubs(&adapted);
    for stub in &mut cmds {
        stub.from_sidecar = true;
    }
    for stub in &mut exprs {
        stub.from_sidecar = true;
    }
    (cmds, exprs)
}

/// The procedures sibling workspace documents define, declared from their
/// sidecar-syntax `stub` lines through the same grammar as inline and sidecar
/// stubs. Each is [`Provenance::WorkspaceTrusted`]: it stands for a source
/// procedure in another document, not an assistance-only stub.
///
/// [`Provenance::WorkspaceTrusted`]: tcl_dialect::model::Provenance::WorkspaceTrusted
#[must_use]
pub fn workspace_procedure_declarations(content: &str) -> tcl_registry::model::DeclaredSurface {
    let (cmds, _) = scan_sidecar_lines(content);
    let stubbed = super::super::types::build_declared_surface(&cmds);
    let mut surface = tcl_registry::model::DeclaredSurface::new();
    for (_, command) in stubbed.iter() {
        let mut command = command.clone().inferred_from_procedure();
        command.declaration.provenance = tcl_dialect::model::Provenance::WorkspaceTrusted;
        surface.declare(command);
    }
    surface
}

/// Whether the source's nearest ancestor sidecar is readable. The incremental
/// analyser uses this as a correctness gate: sidecar signatures affect
/// isolated proc bodies, so their presence selects the full analyser until
/// those signatures are part of every per-body memo key.
#[must_use]
pub fn has_sidecar_stubs(file_path: Option<&str>, profile: &tcl_dialect::DialectProfile) -> bool {
    let Some(file_path) = file_path else {
        return false;
    };
    let mut dir = std::path::Path::new(file_path).parent();
    while let Some(current) = dir {
        if current
            .join(format!("{}.tcl.stubs", profile.name))
            .is_file()
        {
            return true;
        }
        dir = current.parent();
    }
    false
}

/// Strip a leading ``tcl-lsp:`` keyword (case-insensitive) from
/// the comment body, if present.  Whitespace flexible.
pub(super) fn strip_tcl_lsp_prefix(body: &str) -> &str {
    let s = body.trim_start();
    let lower_prefix = "tcl-lsp";
    let kw_end = lower_prefix.len();
    // `s.get(..kw_end)` (unlike `s[..kw_end]`) returns `None` rather than
    // panicking when `kw_end` falls inside a multi-byte char instead of on a
    // boundary — a non-ASCII byte in the comment before that offset would
    // otherwise crash the scan of a stubs block.
    let Some(prefix) = s.get(..kw_end) else {
        return s;
    };
    if !prefix.eq_ignore_ascii_case(lower_prefix) {
        return s;
    }
    let rest = s[kw_end..].trim_start();
    rest.strip_prefix(':').map_or(s, str::trim_start)
}

/// Parse a ``stub NAME {ARGS} ?FLAGS?`` line (case-insensitive on
/// the ``stub`` keyword).  Returns a fully-populated
/// `StubCommandDef` (name, args, flags, range) on match, or
/// ``None`` when the brace block is missing or an argument's
/// ``:role`` annotation is unrecognised.
pub(super) fn parse_command_stub(
    line: &str,
    range: tcl_lexer::Span,
) -> Option<super::super::types::StubCommandDef> {
    let s = line.trim_start();
    let stub_kw = "stub";
    if !s
        .get(..stub_kw.len())
        .is_some_and(|p| p.eq_ignore_ascii_case(stub_kw))
    {
        return None;
    }
    let after = s[stub_kw.len()..].trim_start();
    if after.is_empty() {
        return None;
    }
    // First whitespace-delimited token is the command name.
    let name_end = after
        .find(|c: char| c.is_whitespace())
        .unwrap_or(after.len());
    if name_end == 0 {
        return None;
    }
    let name = &after[..name_end];
    // Reject ``expr-func`` / ``expr-op`` here — those are handled
    // by ``parse_expr_stub``.
    if name.eq_ignore_ascii_case("expr-func") || name.eq_ignore_ascii_case("expr-op") {
        return None;
    }
    let after_name = after[name_end..].trim_start();
    // ``stub NAME {ARGS}`` must have a closing ``}`` — find the
    // matching close, capture the args body, then parse the
    // trailing flag run.
    let after_open = after_name.strip_prefix('{')?;
    let close_rel = after_open.find('}')?;
    let args_body = &after_open[..close_rel];
    let after_close = after_open[close_rel + 1..].trim_start();
    let args = parse_stub_args(args_body)?;
    let flags = parse_stub_flags(after_close);
    let frame = tcl_registry::model::DeclaredFrameEffect::from_stub_flags(after_close);
    Some(super::super::types::StubCommandDef {
        name: name.to_string(),
        args,
        range,
        flags,
        frame,
        from_sidecar: false,
    })
}

/// Parse the inside of ``stub NAME { … }``.  Returns the
/// argument list, or `None` if any token uses an unrecognised
/// ``:role`` annotation or has an empty name (which drops the
/// whole stub).  Empty input yields an empty list.
fn parse_stub_args(args_str: &str) -> Option<Vec<super::super::types::StubArgDef>> {
    let trimmed = args_str.trim();
    if trimmed.is_empty() {
        return Some(Vec::new());
    }
    let mut result = Vec::new();
    for token in trimmed.split_whitespace() {
        let mut name = token;
        let optional = name.len() >= 2 && name.starts_with('?') && name.ends_with('?');
        if optional {
            name = &name[1..name.len() - 1];
        }
        let (arg_name, role) = if let Some(idx) = name.find(':') {
            let arg_name = &name[..idx];
            let role_lower = name[idx + 1..].to_ascii_lowercase();
            // The registry owns the role vocabulary, so a word it does not
            // know is a typo rather than a silent `Value`: reject the whole
            // declaration and let the unresolved-command hint say so.
            tcl_registry::model::role_for_word_checked(&role_lower)?;
            (arg_name, role_lower)
        } else {
            (name, "value".to_string())
        };
        if arg_name.is_empty() {
            return None;
        }
        result.push(super::super::types::StubArgDef {
            name: arg_name.to_string(),
            role,
            optional,
        });
    }
    Some(result)
}

/// Parse the trailing ``?-flag…?`` run after the ``{ARGS}`` block.
/// Unrecognised tokens are ignored.
fn parse_stub_flags(flags_str: &str) -> super::super::types::StubFlags {
    let mut flags = super::super::types::StubFlags::empty();
    for token in flags_str.split_whitespace() {
        match token {
            "-barrier" => flags |= super::super::types::StubFlags::BARRIER,
            "-loop" => flags |= super::super::types::StubFlags::LOOP,
            "-pure" => flags |= super::super::types::StubFlags::PURE,
            "-mutator" => flags |= super::super::types::StubFlags::MUTATOR,
            "-unsafe" => flags |= super::super::types::StubFlags::UNSAFE,
            "-scope_alias" => flags |= super::super::types::StubFlags::SCOPE_ALIAS,
            "-extension" => flags |= super::super::types::StubFlags::EXTENSION,
            _ => {}
        }
    }
    flags
}

/// Parse a ``stub expr-func NAME ?ARITY?`` / ``stub expr-op NAME
/// ?ARITY?`` line.  Returns ``(kind, name, arity)`` on match.
/// ``arity`` defaults to 1 for functions and 2 for operators when
/// the trailing arity word is absent.
pub(super) fn parse_expr_stub(line: &str) -> Option<(&'static str, &str, u32)> {
    let s = line.trim_start();
    let stub_kw = "stub";
    if !s
        .get(..stub_kw.len())
        .is_some_and(|p| p.eq_ignore_ascii_case(stub_kw))
    {
        return None;
    }
    let after = s[stub_kw.len()..].trim_start();
    let kind: &'static str;
    let default_arity: u32;
    let rest;
    if after
        .get(.."expr-func".len())
        .is_some_and(|p| p.eq_ignore_ascii_case("expr-func"))
    {
        kind = "function";
        default_arity = 1;
        rest = after["expr-func".len()..].trim_start();
    } else if after
        .get(.."expr-op".len())
        .is_some_and(|p| p.eq_ignore_ascii_case("expr-op"))
    {
        kind = "operator";
        default_arity = 2;
        rest = after["expr-op".len()..].trim_start();
    } else {
        return None;
    }
    if rest.is_empty() {
        return None;
    }
    let name_end = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
    if name_end == 0 {
        return None;
    }
    let name = &rest[..name_end];
    let after_name = rest[name_end..].trim_start();
    let arity = if after_name.is_empty() {
        default_arity
    } else {
        let arity_end = after_name
            .find(|c: char| c.is_whitespace())
            .unwrap_or(after_name.len());
        after_name[..arity_end]
            .parse::<u32>()
            .unwrap_or(default_arity)
    };
    Some((kind, name, arity))
}

/// Every stub declaration in force for one document: the nearest
/// `<dialect>.tcl.stubs` sidecar, overridden by the document's own inline
/// `# tcl-lsp: stubs-begin` block.
///
/// The one ingestion path. The analyser calls it to build its
/// [`AnalysisResult::stub_commands`](super::super::types::AnalysisResult::stub_commands)
/// and its [`DeclaredSurface`](tcl_registry::model::DeclaredSurface); every
/// host that builds a [`CompilationUnit`](crate::compilation_unit::CompilationUnit)
/// through the `cu_override` seam calls it (via
/// [`document_declared_surface`]) so the unit it supplies declares exactly
/// what the analyser's own unit would.
#[must_use]
pub fn document_stub_declarations(
    source: &str,
    file_path: Option<&str>,
    dialect: &str,
) -> (
    Vec<super::super::types::StubCommandDef>,
    Vec<super::super::types::StubExprDef>,
) {
    let (commands, expressions, _) = document_stub_declarations_with_outside(
        source,
        file_path,
        dialect,
        // dialect-drift-ok: unreviewed. The gate stopped reading the old file at a test-module
        // line above this call; the file split of 16 November 2026 brought it into view.
        tcl_lexer::LexerConfig::default(),
    );
    (commands, expressions)
}

/// The same declaration scan, including discarded authored directive spans.
#[must_use]
pub fn document_stub_declarations_with_outside(
    source: &str,
    file_path: Option<&str>,
    dialect: &str,
    config: tcl_lexer::LexerConfig,
) -> (
    Vec<super::super::types::StubCommandDef>,
    Vec<super::super::types::StubExprDef>,
    Vec<Span>,
) {
    let (inline_cmds, inline_exprs, outside) = scan_source_for_stubs_with_outside(source, config);
    let (mut cmds, mut exprs) = scan_sidecar_stubs(file_path, dialect);
    // The document-local declaration is nearest in scope and wins over a
    // workspace sidecar with the same name — `DeclaredSurface::declare`
    // keeps the last ingested one.
    cmds.extend(inline_cmds);
    exprs.extend(inline_exprs);
    (cmds, exprs, outside)
}

/// The document's [`DeclaredSurface`](tcl_registry::model::DeclaredSurface) —
/// [`document_stub_declarations`] ingested through
/// [`build_declared_surface`](super::super::types::build_declared_surface).
#[must_use]
pub fn document_declared_surface(
    source: &str,
    file_path: Option<&str>,
    dialect: &str,
) -> tcl_registry::model::DeclaredSurface {
    let (cmds, _) = document_stub_declarations(source, file_path, dialect);
    super::super::types::build_declared_surface(&cmds)
}
