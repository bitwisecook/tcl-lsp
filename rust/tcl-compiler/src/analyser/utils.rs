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

//! Pure analyser helpers.
//!
//! Pure free functions used by handlers and diagnostic emitters
//! across the analyser. `parse_param_list` is re-exported from
//! `signature_scan::params` so the analyser can keep its imports
//! flat.

use std::collections::{HashMap, HashSet};

use tcl_lexer::{SourceMap, Span, Token};
use tcl_registry::{CommandRegistry, Traits};

use crate::ir::Statement;

pub use crate::signature_scan::params::{
    param_name_spans, param_name_spans_for_token, parse_param_list,
};

/// Cap on how many leading lines the file-suppression scanner
/// inspects. Pathological all-comment files stop scanning past
/// this.
const FILE_DIRECTIVE_SCAN_LINES: usize = 100;

/// Extract file-wide diagnostic suppression from top-of-file
/// directives.
///
/// Scans leading comment / blank lines for
/// `# tcl-lsp: disable=CODE1,CODE2` (or `=*` for "all codes");
/// stops at the first line that is neither blank nor a `#`
/// comment. Multiple directives accumulate into the returned set.
///
/// Codes are split on commas and any whitespace. Empty tokens
/// after splitting are discarded. The keyword `tcl-lsp:disable=`
/// matches case-insensitively, but the codes themselves are not
/// lowercased.
#[must_use]
pub fn parse_file_suppression(source: &str) -> HashSet<String> {
    let mut codes: HashSet<String> = HashSet::new();
    for (idx, line) in source.lines().enumerate() {
        if idx >= FILE_DIRECTIVE_SCAN_LINES {
            break;
        }
        let stripped = line.trim();
        if stripped.is_empty() {
            continue;
        }
        if !stripped.starts_with('#') {
            break;
        }
        let Some(rest) = parse_disable_directive(line) else {
            continue;
        };
        for token in rest.split([',', ' ', '\t', '\r', '\n']) {
            let token = token.trim();
            if !token.is_empty() {
                codes.insert(token.to_string());
            }
        }
    }
    codes
}

/// Scan *source* for ``# noqa`` comment lines and record next-line
/// suppressions.
///
/// For each ``# noqa`` / ``# noqa: CODE,…`` comment on line *N*
/// (0-based), records the suppression codes against line *N+1*.
/// This covers two cases the command-attached
/// `apply_preceding_noqa` mechanism cannot reach:
///
/// * A ``# noqa`` comment at the tail of a brace body (orphaned —
///   no following command in that scope), where the diagnostic
///   fires on the immediately following ``} elseif …`` or similar
///   line.
/// * A ``# noqa`` comment placed immediately before another comment
///   line that itself generates a diagnostic (e.g. W115 on a
///   ``\\``-continued comment).
///
/// The scan is additive — callers are expected to merge the
/// returned map into ``result.suppressed_lines`` alongside the
/// existing command-level (`apply_preceding_noqa`) and file-level
/// (`parse_file_suppression`) passes.
///
/// Bare ``# noqa`` (no ``: CODE`` list) is recorded with the `"*"`
/// sentinel that the suppression filter treats as "every code".
#[must_use]
pub fn parse_noqa_line_suppressions(
    source: &str,
) -> std::collections::HashMap<i32, HashSet<String>> {
    let mut result: std::collections::HashMap<i32, HashSet<String>> =
        std::collections::HashMap::new();
    for (idx, line) in source.split('\n').enumerate() {
        let stripped = line.trim();
        if !stripped.starts_with('#') {
            continue;
        }
        let lower = stripped.to_ascii_lowercase();
        let Some(noqa_pos) = lower.find("noqa") else {
            continue;
        };
        let rest = stripped[noqa_pos + 4..].trim();
        let codes: HashSet<String> = if let Some(after_colon) = rest.strip_prefix(':') {
            let parsed: HashSet<String> = after_colon
                .split([',', ' ', '\t', '\r', '\n'])
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
            if parsed.is_empty() {
                std::iter::once("*".to_string()).collect()
            } else {
                parsed
            }
        } else {
            std::iter::once("*".to_string()).collect()
        };
        let next_line = i32::try_from(idx).unwrap_or(i32::MAX).saturating_add(1);
        result.entry(next_line).or_default().extend(codes);
    }
    result
}

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

/// Build the "known command" name universe consulted by unclosed-delimiter
/// recovery: the segmenter's scan-to-next heuristic
/// ([`crate::segmenter::segment_commands_with_recovery_and_config`]) and the
/// E100/E201/E202/E203 recovery diagnostics (`syntax_checks`) both use "does
/// the next line start with a known command?" as their strongest signal that
/// a missing delimiter belongs right before it.
///
/// Restricting "known" to the active dialect's [`CommandRegistry`]
/// under-recognises real code: almost every non-trivial Tcl file calls its
/// own `proc`s, so a break just before a call to one of them falls back to
/// the imprecise generic diagnostic (or, worse, the recovery point is never
/// found at all and the rest of the document goes unanalysed). This unions
/// the registry names with every proc / class / command-alias /
/// rename-target name the document itself defines — qualified and
/// unqualified tail — using the same fast, non-lowering scan
/// [`crate::signature_scan::extract_signatures`] already uses for
/// background-indexed files, so recovery recognises a document's own procs
/// as readily as a builtin.
///
/// `extra` is unioned in unconditionally alongside the registry — the same
/// user-declared / LSP-layer-resolved name set [`super::state::Analyser::extra_commands`]
/// already carries for the W123 unresolved-command check (`tclLsp.extraCommands`,
/// widened one layer up in `tcl-lsp-server` with workspace- and package-resolved
/// names on the recovery path — see `widen_recovery_extra_commands` there). It is
/// a plain in-memory set by the time it reaches here, so folding it in costs
/// nothing extra beyond the registry union already does.
///
/// Gated on [`tcl_lexer::script_is_complete`]: the in-file signature scan only
/// ever runs on a document with an unclosed delimiter, so a complete script
/// skips that extra scan entirely — zero added cost on the overwhelmingly
/// common well-formed edit.
#[must_use]
pub fn recovery_known_commands(
    source: &str,
    registry: &CommandRegistry,
    extra: &SharedNameSet,
) -> RecoveryKnownCommands {
    let mut names: HashSet<String> = registry.command_names().map(str::to_owned).collect();
    if !tcl_lexer::script_is_complete(source) {
        let sig = crate::signature_scan::extract_signatures(source, registry);
        for qname in sig
            .procs
            .keys()
            .chain(sig.classes.keys())
            .chain(sig.command_aliases.keys())
            .chain(sig.renames.keys())
        {
            insert_qualified_and_tail(&mut names, qname);
        }
    }
    RecoveryKnownCommands {
        local: names,
        extra: std::sync::Arc::clone(extra),
    }
}

/// The recovery known-command universe [`recovery_known_commands`] builds,
/// held as **two layers** rather than one merged set.
///
/// `local` is derived from the document being analysed (the active registry's
/// names plus the document's own proc / class / alias / rename names), so it is
/// rebuilt on every edit. `extra` is the caller-supplied name set
/// ([`super::state::Analyser::extra_commands`]), which on the LSP's
/// unclosed-delimiter recovery path is the *widened* set — every
/// workspace-indexed proc and class, plus every auto-loadable command name:
/// tens of thousands of entries on a large workspace, and unchanged between
/// keystrokes. Merging the two would deep-copy that set on every analysis of a
/// document with an open delimiter — which is the *normal* mid-typing state, so
/// it happened on every debounced run (issue #1154). Sharing it behind an `Arc`
/// makes the layering free.
///
/// Every consumer only ever asks "is this name known?" or enumerates the
/// universe once, both of which the two layers answer directly.
#[derive(Debug, Default, Clone)]
pub struct RecoveryKnownCommands {
    local: HashSet<String>,
    extra: SharedNameSet,
}

/// A command-name set shared between the analyser and its caller — see
/// [`RecoveryKnownCommands`] and [`super::state::Analyser::extra_commands`].
pub type SharedNameSet = std::sync::Arc<HashSet<String>>;

impl RecoveryKnownCommands {
    /// Whether `name` is a known command in either layer.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.local.contains(name) || self.extra.contains(name)
    }

    /// Every known name, `local` first. A name present in both layers is
    /// yielded twice; every consumer collects into a set, so that is harmless
    /// and avoids a de-duplication pass over the (large) shared layer.
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.local
            .iter()
            .chain(self.extra.iter())
            .map(String::as_str)
    }

    /// Drop the per-document layer and release the shared one — the
    /// between-runs reset ([`super::state::Analyser::clear_run_state`]).
    pub fn clear(&mut self) {
        self.local.clear();
        self.extra = std::sync::Arc::default();
    }
}

impl FromIterator<String> for RecoveryKnownCommands {
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
        Self {
            local: iter.into_iter().collect(),
            extra: std::sync::Arc::default(),
        }
    }
}

/// True when an earlier *unconditional* user `proc` named `qualified_name`
/// lexically precedes `call_start` — Tcl resolves a proc over a same-named
/// builtin/keyword at that call site, so the diagnostic that would fire
/// against the builtin/keyword must be suppressed. Shared by W002's inline
/// disabled-command shadow check and the EXPR-role emitters'
/// (W100/W110/W003/W114) control-flow-keyword shadow check — both are the
/// exact same position comparison against [`ProcDef::name_span`], just
/// triggered from different call sites. W002's *per-item* flush path needs
/// the fuller `UserResolutionFacts::resolves_to_user` instead (it also
/// covers aliases/renames/classes/ensembles and namespace-qualified procs),
/// so this simpler check isn't a fit there.
#[must_use]
pub fn proc_shadows_call<S: std::hash::BuildHasher>(
    all_procs: &HashMap<String, super::types::ProcDef, S>,
    qualified_name: &str,
    call_start: u32,
) -> bool {
    all_procs
        .get(qualified_name)
        .is_some_and(|def| def.name_span.start() < call_start)
}

/// Insert `qname` (e.g. `::ns::foo`) — as-is, with the leading `::`
/// stripped, and as its unqualified tail (`foo`) — into `names`. Mirrors the
/// tail-matching the W123 unresolved-command pass already uses for the same
/// proc/class/alias collections (`build_w123_known_names`), plus the
/// original fully-qualified form: `signature_scan::qualify` always prepends
/// `::`, and a recovery-point call can be written either way
/// (`::ns::foo` or `ns::foo`), so both must be recognised.
///
/// `pub` (not just `pub(super)`) because `tcl-lsp-server` reuses this exact
/// three-form insertion when widening the recovery known-command set with
/// workspace-indexed and package-resolved names one layer up — those names
/// carry the same `::`-qualified shape and need the same recognition rules,
/// so this is the single place that logic lives rather than a second,
/// independently-maintained copy.
pub fn insert_qualified_and_tail<S: std::hash::BuildHasher>(
    names: &mut HashSet<String, S>,
    qname: &str,
) {
    if !qname.is_empty() {
        names.insert(qname.to_string());
    }
    let absolute = qname.trim_start_matches("::");
    if !absolute.is_empty() {
        names.insert(absolute.to_string());
    }
    if let Some((_, tail)) = qname.rsplit_once("::")
        && !tail.is_empty()
    {
        names.insert(tail.to_string());
    }
}

/// Match a ``# tcl-lsp: <marker>`` line, where `marker` is
/// e.g. ``"stubs-begin"`` / ``"stubs-end"``.  Case-insensitive
/// on the keyword run; whitespace flexible.
fn matches_marker(body: &str, marker: &str) -> bool {
    let s = body.trim_start();
    let lower_prefix = "tcl-lsp";
    let kw_end = lower_prefix.len();
    // `str::get` returns `None` when `kw_end` is past the end *or* falls inside
    // a multi-byte char, so it subsumes the length check and never panics on
    // non-ASCII source (the marker run itself is pure ASCII).
    let Some(prefix) = s.get(..kw_end) else {
        return false;
    };
    if !prefix.eq_ignore_ascii_case(lower_prefix) {
        return false;
    }
    let rest = s[kw_end..].trim_start();
    let Some(rest) = rest.strip_prefix(':') else {
        return false;
    };
    let rest = rest.trim_start();
    let m_end = marker.len();
    let Some(head) = rest.get(..m_end) else {
        return false;
    };
    head.eq_ignore_ascii_case(marker)
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
    Vec<super::types::StubCommandDef>,
    Vec<super::types::StubExprDef>,
) {
    let mut cmds = Vec::new();
    let mut exprs = Vec::new();
    let mut in_block = false;
    let mut offset: u32 = 0;
    for line in source.split_inclusive('\n') {
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
            continue;
        }
        // ``# tcl-lsp:`` prefix is optional inside a stubs block —
        // strip it once if present.
        let after_prefix = strip_tcl_lsp_prefix(body);
        // Try the expr-func / expr-op shape first because ``stub
        // expr-func NAME`` would otherwise match the command shape
        // with NAME == "expr-func".
        if let Some((kind, name, arity)) = parse_expr_stub(after_prefix) {
            exprs.push(super::types::StubExprDef {
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
    (cmds, exprs)
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
    dialect: &str,
) -> (
    Vec<super::types::StubCommandDef>,
    Vec<super::types::StubExprDef>,
) {
    let Some(file_path) = file_path else {
        return (Vec::new(), Vec::new());
    };
    let mut dir = std::path::Path::new(file_path).parent();
    while let Some(current) = dir {
        let sidecar = current.join(format!("{dialect}.tcl.stubs"));
        if let Ok(content) = std::fs::read_to_string(&sidecar) {
            // Sidecar syntax is one declaration per line, without the inline
            // comment prefix or begin/end markers.  Adapt it into the
            // canonical scanner rather than maintaining a second parser.
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
            return (cmds, exprs);
        }
        dir = current.parent();
    }
    (Vec::new(), Vec::new())
}

/// Whether the source's nearest ancestor sidecar is readable. The incremental
/// analyser uses this as a correctness gate: sidecar signatures affect
/// isolated proc bodies, so their presence selects the full analyser until
/// those signatures are part of every per-body memo key.
#[must_use]
pub fn has_sidecar_stubs(file_path: Option<&str>, dialect: &str) -> bool {
    let Some(file_path) = file_path else {
        return false;
    };
    let mut dir = std::path::Path::new(file_path).parent();
    while let Some(current) = dir {
        if current.join(format!("{dialect}.tcl.stubs")).is_file() {
            return true;
        }
        dir = current.parent();
    }
    false
}

/// Strip a leading ``tcl-lsp:`` keyword (case-insensitive) from
/// the comment body, if present.  Whitespace flexible.
fn strip_tcl_lsp_prefix(body: &str) -> &str {
    let s = body.trim_start();
    let lower_prefix = "tcl-lsp";
    let kw_end = lower_prefix.len();
    // `s.get(..kw_end)` (unlike `s[..kw_end]`) returns `None` rather than
    // panicking when `kw_end` falls inside a multi-byte char instead of on a
    // boundary — a real case: a non-ASCII byte in the comment before that
    // offset used to crash the server while scanning a stubs block.
    let Some(prefix) = s.get(..kw_end) else {
        return s;
    };
    if !prefix.eq_ignore_ascii_case(lower_prefix) {
        return s;
    }
    let rest = s[kw_end..].trim_start();
    rest.strip_prefix(':').map_or(s, str::trim_start)
}

/// Valid argument-role annotations recognised after the ``:``
/// separator in a stub argument token.
const VALID_STUB_ROLES: &[&str] = &[
    "body", "expr", "var", "var_read", "name", "pattern", "channel", "value",
];

/// Parse a ``stub NAME {ARGS} ?FLAGS?`` line (case-insensitive on
/// the ``stub`` keyword).  Returns a fully-populated
/// `StubCommandDef` (name, args, flags, range) on match, or
/// ``None`` when the brace block is missing or an argument's
/// ``:role`` annotation is unrecognised.
fn parse_command_stub(line: &str, range: tcl_lexer::Span) -> Option<super::types::StubCommandDef> {
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
    Some(super::types::StubCommandDef {
        name: name.to_string(),
        args,
        range,
        flags,
        from_sidecar: false,
    })
}

/// Parse the inside of ``stub NAME { … }``.  Returns the
/// argument list, or `None` if any token uses an unrecognised
/// ``:role`` annotation or has an empty name (which drops the
/// whole stub).  Empty input yields an empty list.
fn parse_stub_args(args_str: &str) -> Option<Vec<super::types::StubArgDef>> {
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
            let role = &name[idx + 1..];
            let role_lower = role.to_ascii_lowercase();
            if !VALID_STUB_ROLES.contains(&role_lower.as_str()) {
                return None;
            }
            (arg_name, role_lower)
        } else {
            (name, "value".to_string())
        };
        if arg_name.is_empty() {
            return None;
        }
        result.push(super::types::StubArgDef {
            name: arg_name.to_string(),
            role,
            optional,
        });
    }
    Some(result)
}

/// Parse the trailing ``?-flag…?`` run after the ``{ARGS}`` block.
/// Unrecognised tokens are ignored.
fn parse_stub_flags(flags_str: &str) -> super::types::StubFlags {
    let mut flags = super::types::StubFlags::empty();
    for token in flags_str.split_whitespace() {
        match token {
            "-barrier" => flags |= super::types::StubFlags::BARRIER,
            "-loop" => flags |= super::types::StubFlags::LOOP,
            "-pure" => flags |= super::types::StubFlags::PURE,
            "-mutator" => flags |= super::types::StubFlags::MUTATOR,
            "-unsafe" => flags |= super::types::StubFlags::UNSAFE,
            "-scope_alias" => flags |= super::types::StubFlags::SCOPE_ALIAS,
            _ => {}
        }
    }
    flags
}

/// Parse a ``stub expr-func NAME ?ARITY?`` / ``stub expr-op NAME
/// ?ARITY?`` line.  Returns ``(kind, name, arity)`` on match.
/// ``arity`` defaults to 1 for functions and 2 for operators when
/// the trailing arity word is absent.
fn parse_expr_stub(line: &str) -> Option<(&'static str, &str, u32)> {
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

/// Decoration-character set for the body-docstring scanner
/// (``.-=*~#``).
const DECORATION_CHARS: &[char] = &['.', '-', '=', '*', '~', '#'];

/// Extract a leading comment block from a proc body — the
/// fallback that fires when there's no preceding-comment harvest
/// from the segmenter.
///
/// Lines containing only decoration characters
/// (``.-=*~#``) are skipped; remaining ``#``-prefixed lines have
/// the leading hash + whitespace stripped, then accumulated.
/// The first non-comment / non-blank line ends the block.
#[must_use]
pub fn extract_body_docstring(body: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    for raw in body.lines() {
        let stripped = raw.trim();
        if stripped.is_empty() {
            if !lines.is_empty() {
                break;
            }
            continue;
        }
        if !stripped.starts_with('#') {
            break;
        }
        let text = stripped.trim_start_matches('#').trim().to_string();
        // Skip pure-hash decoration lines (``####`` etc.).
        if text.is_empty() && stripped.chars().all(|c| c == '#') {
            continue;
        }
        // Skip lines made entirely of decoration characters.
        if !text.is_empty() && text.chars().all(|c| DECORATION_CHARS.contains(&c)) {
            continue;
        }
        lines.push(text);
    }
    lines.join("\n")
}

/// Per-line ``# noqa: CODE`` suppression scanner.
///
/// For each ``# noqa`` (with or without a ``: CODE`` list),
/// records the codes against the 0-based line range read from the
/// ``preceding_comment`` field of the segmented command. The
/// segmented-command dispatch loop calls this on each command to
/// attach the noqa codes to the *following* command's line range.
///
/// Matching is substring-based (``find("noqa")``) against the
/// comment body alone; a comment is only ever the source of a
/// noqa directive, so there's no risk of false-positiving on a
/// ``#`` inside a Tcl string — the segmenter's
/// ``preceding_comment`` field carries comment text only.  Bare
/// ``# noqa`` (no ``: CODE`` list) suppresses every code (`"*"`
/// sentinel); ``# noqa: A, B`` suppresses the named codes.
pub fn apply_preceding_noqa<S, T>(
    cmd: &crate::segmenter::SegmentedCommand,
    line_offsets: &[usize],
    suppressed_lines: &mut std::collections::HashMap<i32, std::collections::HashSet<String, T>, S>,
) where
    S: std::hash::BuildHasher,
    T: std::hash::BuildHasher + Default,
{
    let Some(comment) = cmd.preceding_comment.as_deref() else {
        return;
    };
    let lower = comment.to_ascii_lowercase();
    let Some(noqa_pos) = lower.find("noqa") else {
        return;
    };
    let rest = comment[noqa_pos + 4..].trim_start();
    let codes: std::collections::HashSet<String> = if let Some(after_colon) = rest.strip_prefix(':')
    {
        let parsed: std::collections::HashSet<String> = after_colon
            .split([',', ' ', '\t', '\r', '\n'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        if parsed.is_empty() {
            std::iter::once("*".to_string()).collect()
        } else {
            parsed
        }
    } else {
        std::iter::once("*".to_string()).collect()
    };
    // Attribute to every line spanned by the command.
    // ``SegmentedCommand.span`` is byte offsets; convert each via
    // the precomputed ``line_offsets`` index in ``O(log N)``
    // instead of a linear scan per call (the helper runs once per
    // segmented command).
    let span_start = cmd.span.start() as usize;
    let span_end = cmd.span.end() as usize;
    let start_line = super::state::line_at_offset(line_offsets, span_start);
    let end_line = super::state::line_at_offset(line_offsets, span_end);
    for line in start_line..=end_line {
        suppressed_lines
            .entry(line)
            .or_default()
            .extend(codes.iter().cloned());
    }
}

/// Match `# tcl-lsp: disable=…` (case-insensitive on the keyword),
/// returning the trailing CODE list as a borrowed slice. Returns
/// `None` if the line doesn't match the directive shape.
///
/// Uses [`str::get`] rather than byte-index slicing for the keyword
/// checks: a fixed ASCII-length prefix like `s[..kw_end]` panics when a
/// multi-byte UTF-8 character (an em dash, accented letter, …) straddles
/// that byte offset — `s.get(..kw_end)` instead returns `None` for a
/// non-boundary offset exactly as it does for a too-short `s`, so a
/// leading comment with early non-ASCII text is simply "not a directive"
/// rather than a panic.
fn parse_disable_directive(line: &str) -> Option<&str> {
    let mut s = line.trim_start();
    s = s.strip_prefix('#')?.trim_start();
    let lower_prefix = "tcl-lsp";
    let kw_end = lower_prefix.len();
    if !s.get(..kw_end)?.eq_ignore_ascii_case(lower_prefix) {
        return None;
    }
    s = s[kw_end..].trim_start();
    s = s.strip_prefix(':')?.trim_start();
    let disable_kw = "disable";
    let dis_end = disable_kw.len();
    if !s.get(..dis_end)?.eq_ignore_ascii_case(disable_kw) {
        return None;
    }
    s = s[dis_end..].trim_start();
    let s = s.strip_prefix('=')?.trim();
    Some(s)
}

/// Format a literal value for inclusion in a diagnostic message.
///
/// Replaces newlines with `\n` (literal two characters), and
/// truncates strings longer than 40 characters to `…[37 chars]…`.
#[must_use]
pub fn format_literal_for_message(value: &str) -> String {
    let display = value.replace('\n', "\\n");
    if display.chars().count() > 40 {
        // Slice at character (not byte) boundaries — Tcl source is
        // usually ASCII but multibyte diagnostic messages are
        // valid.
        let truncated: String = display.chars().take(37).collect();
        format!("{truncated}...")
    } else {
        display
    }
}

/// Return `(variable, static_value)` for assignments worth the
/// W214 paste-error heuristic check.
///
/// Only [`Statement::AssignConst`] and [`Statement::AssignValue`]
/// shapes qualify; `AssignValue` rejects values containing
/// `$` / `[` / `]` (interpolation / command substitution
/// disqualify the fingerprint).
#[must_use]
pub fn possible_paste_fingerprint(stmt: &Statement) -> Option<(String, String)> {
    match stmt {
        Statement::AssignConst { name, value, .. } => {
            Some((name.clone(), value.trim().to_string()))
        }
        Statement::AssignValue { name, value, .. } => {
            let value = value.trim();
            if value.is_empty() {
                return None;
            }
            if value.contains('$') || value.contains('[') || value.contains(']') {
                return None;
            }
            Some((name.clone(), value.to_string()))
        }
        _ => None,
    }
}

/// Return argv tokens widened to each Tcl word's full token span.
///
/// The segmenter already returns argv tokens with word-wide spans
/// (see `rust/tcl-compiler/src/segmenter.rs`), so this is the
/// identity function. The helper is kept as a transparent
/// passthrough so the indirection layer at the call sites stays
/// intact.
#[must_use]
pub fn argv_with_word_spans(argv: Vec<Token>, _all_tokens: &[Token]) -> Vec<Token> {
    argv
}

/// Full source span of one Tcl word, as defined by the lexer.
///
/// [`tcl_lexer::word_span`] is the authoritative delimiter policy: it handles
/// braces, brackets, quotes, empty words, escaped closers, and recovery
/// tokens without each analyser diagnostic reimplementing byte arithmetic.
#[must_use]
pub fn full_word_span(token: Token, source: &str) -> Span {
    tcl_lexer::word_span(&SourceMap::new(source), token)
}

/// The set of iRules commands that may only appear at the top
/// level of an event handler, computed fresh from the supplied
/// `registry`.
///
/// Reads the `IRULES_TOP_LEVEL_ONLY` trait from `registry` and
/// returns the matching command names.
///
/// **Not cached.** A previous version cached the first-call
/// result in a static `OnceLock`, which silently returned stale
/// data when a non-default `CommandRegistry` was passed on a
/// later call. The trait scan is `O(n)` over the registry's
/// command specs (~150 entries) and the registry itself is
/// already cached at the call sites that need this — so per-call
/// recomputation is cheap in practice and removes a correctness
/// trap. Callers that want caching should cache at their own
/// layer, keyed by whatever identity makes sense for them.
#[must_use]
pub fn irules_top_level_only(registry: &CommandRegistry) -> HashSet<String> {
    registry
        .commands_with_trait(Traits::IRULES_TOP_LEVEL_ONLY)
        .into_iter()
        .map(String::from)
        .collect()
}

/// The exact source window a [`Traits::SCRIPT_CONCATENATES_ARGS`] tail
/// evaluates — every content byte at its true offset, every structural
/// byte blanked to a space — or `None` when the script is not statically
/// knowable.
///
/// `Tcl_ConcatObj` (`generic/tclUtil.c`) is the one rule the whole eval
/// family shares: take each word's **string representation**, trim ASCII
/// whitespace from both ends of it, drop the ones that end up empty, and
/// join the rest with a single space. A braced word contributes its
/// *contents* — Tcl's word parsing has already removed the outer braces
/// before `Tcl_ConcatObj` ever sees the value — so braced and bare words
/// concatenate identically and a command may span word boundaries.
///
/// Joining the trimmed texts into a fresh string reproduces those bytes
/// but loses their *positions*: stripped delimiters, dropped words, and
/// collapsed whitespace shift every byte after the first divergence, so a
/// span recorded while walking the joined text can land on the wrong
/// source bytes (in `eval {} foo` the reconstructed `foo` starts at the
/// empty word's closing brace, and a rename would edit that delimiter
/// instead of the command). So this builds the equivalent script *in
/// place* instead: a buffer covering the words' whole span in `source`,
/// holding each word's content bytes at their real offsets and a space
/// everywhere else (delimiters, inter-word gaps). Script parsing treats
/// any whitespace run as one separator, so the window parses into exactly
/// the words the trimmed-and-joined text would — `Tcl_ConcatObj`'s trim,
/// drop, and single-space steps all degenerate to "more whitespace" —
/// while every span the walk records maps to the source bytes it
/// describes.
///
/// Pinned against tclsh8.6.14 and tclsh9.0.4:
///
/// ```text
/// eval set l2 hello        ≡ set l2 hello
/// eval {set l2} hello      ≡ set l2 hello   (braces gone, contents kept)
/// eval "set q" 7           ≡ set q 7        (quotes gone; a space inside
///                                            a quoted word separates —
///                                            the re-parse splits it just
///                                            as the join would)
/// eval {set l2} {$value}   ≡ set l2 $value  (braces blocked the outer
///                                            substitution, so the script
///                                            text is static; `eval`
///                                            itself resolves `$value`
///                                            when the script runs)
/// eval {} foo              ≡ foo            (an empty word is just more
///                                            whitespace)
/// ```
///
/// Returns `None` when any word is not statically-known script text
/// ([`super::diagnostics::helpers::word_is_static_script_text`]: a
/// substitution or a decode runs before concatenation, so the real script
/// is unknowable), when a word's text does not sit verbatim at its
/// token's content offset (bytes that cannot be mapped must not be
/// analysed), or when the window holds no content at all. Callers must
/// then consume the command without walking it rather than guess.
///
/// This is **not** the rule for `namespace inscope`, whose tail is appended
/// as list elements — see [`Traits::SCRIPT_APPENDS_LIST_ARGS`].
#[must_use]
pub fn concat_script_window(
    words: &[String],
    tokens: &[Token],
    source: &str,
) -> Option<(String, tcl_lexer::Span)> {
    let (first, last) = (tokens.first()?, tokens.last()?);
    if words.len() != tokens.len() {
        return None;
    }
    let start = first.span.start() as usize;
    let end = last.span.end() as usize;
    if end <= start || source.len() < end {
        return None;
    }
    let mut window = vec![b' '; end - start];
    for (word, tok) in words.iter().zip(tokens) {
        if !crate::analyser::diagnostics::helpers::word_is_static_script_text(word, tok.kind) {
            return None;
        }
        let content_start = tok.span.start() as usize + usize::from(tok.content_offset);
        // The word's text must sit verbatim at its content offset — any
        // divergence (an escape decode, a token shape this fn does not
        // model) makes the bytes unmappable, and unmappable bytes must
        // not be walked.
        if source.get(content_start..content_start + word.len()) != Some(word.as_str()) {
            return None;
        }
        let off = content_start.checked_sub(start)?;
        window
            .get_mut(off..off + word.len())?
            .copy_from_slice(word.as_bytes());
    }
    if window.iter().all(u8::is_ascii_whitespace) {
        return None;
    }
    let window = String::from_utf8(window).ok()?;
    Some((
        window,
        tcl_lexer::Span::new(u32::try_from(start).ok()?, u32::try_from(end).ok()?),
    ))
}

#[cfg(test)]
mod concat_script_window_tests {
    use super::concat_script_window;
    use tcl_lexer::{Span, Token, TokenType};

    /// Tail words of `source` from `from`, tokenised the way the analyser's
    /// argv sees them: each `(text, kind)` pair is located in `source` (in
    /// order), and a `Str`/quoted token's span covers its delimiters with
    /// `content_offset` pointing past the opener.
    fn tail(source: &str, from: usize, pairs: &[(&str, TokenType)]) -> (Vec<String>, Vec<Token>) {
        let mut texts = Vec::new();
        let mut toks = Vec::new();
        let mut at = from;
        for (text, kind) in pairs {
            let delim = usize::from(matches!(*kind, TokenType::Str));
            let content = if text.is_empty() {
                // Locate an empty content by its `{}` delimiter pair.
                source[at..].find("{}").expect("fixture") + at + 1
            } else {
                source[at..].find(text).expect("fixture") + at
            };
            let (s, e) = (content - delim, content + text.len() + delim);
            let mut tok = Token::new(
                *kind,
                Span::new(u32::try_from(s).unwrap(), u32::try_from(e).unwrap()),
            );
            tok.content_offset = u8::try_from(delim).unwrap();
            texts.push((*text).to_owned());
            toks.push(tok);
            at = e;
        }
        (texts, toks)
    }

    /// A braced word's contents and a bare word join into one script, and
    /// every content byte keeps its exact source offset — the window blanks
    /// the `{`/`}` delimiters and gap to spaces rather than shifting bytes.
    ///
    /// tclsh8.6.14 / tclsh9.0.4: `eval {set l2} hello` ≡ `set l2 hello`.
    #[test]
    fn braced_and_bare_words_window_in_place() {
        let src = "eval {set l2} hello";
        let (t, k) = tail(
            src,
            5,
            &[("set l2", TokenType::Str), ("hello", TokenType::Esc)],
        );
        let (win, span) = concat_script_window(&t, &k, src).expect("static");
        assert_eq!(win, " set l2  hello");
        assert_eq!((span.start(), span.end()), (5, 19));
        // The window is positionally exact: `hello` sits at its source offset.
        let off = usize::try_from(span.start()).unwrap();
        assert_eq!(off + win.find("hello").unwrap(), src.find("hello").unwrap());
    }

    /// A braced word whose contents carry `$`/`[` is still *statically known
    /// script text* — the braces blocked the outer substitution, and the
    /// eval-family command itself resolves it when the script runs.
    ///
    /// tclsh8.6.14 / tclsh9.0.4: `set value 5; eval {set l2} {$value};
    /// puts $l2` → `5`.
    #[test]
    fn a_braced_word_with_substitution_text_is_static() {
        let src = "eval {set l2} {$value}";
        let (t, k) = tail(
            src,
            5,
            &[("set l2", TokenType::Str), ("$value", TokenType::Str)],
        );
        let (win, _) = concat_script_window(&t, &k, src).expect("braced text is static");
        assert_eq!(win, " set l2   $value ");
    }

    /// Substitution runs before concatenation, so a dynamic word (or a bare
    /// word carrying `$`/`[`/escape/quote bytes) makes the real script
    /// unknowable — the helper declines rather than guessing.
    #[test]
    fn a_dynamic_or_decoded_word_anywhere_declines() {
        let src = "eval $cmd arg";
        let (t, k) = tail(src, 5, &[("$cmd", TokenType::Var), ("arg", TokenType::Esc)]);
        assert_eq!(concat_script_window(&t, &k, src), None);
        let src = "eval set $name";
        let (t, k) = tail(
            src,
            5,
            &[("set", TokenType::Esc), ("$name", TokenType::Var)],
        );
        assert_eq!(concat_script_window(&t, &k, src), None);
        let src = "eval puts [f]";
        let (t, k) = tail(src, 5, &[("puts", TokenType::Esc), ("[f]", TokenType::Cmd)]);
        assert_eq!(concat_script_window(&t, &k, src), None);
        // A backslash in a bare word decodes before the join — the decoded
        // text matches no source bytes, so the window declines.
        let src = "eval set\\ x 5";
        let (t, k) = tail(
            src,
            5,
            &[("set\\ x", TokenType::Esc), ("5", TokenType::Esc)],
        );
        assert_eq!(concat_script_window(&t, &k, src), None);
    }

    /// An empty braced word contributes nothing — its delimiters blank to
    /// whitespace and the remaining word keeps its exact offset. This is the
    /// shape where a naive join anchors the tail one byte early.
    ///
    /// tclsh8.6.14 / tclsh9.0.4: `eval {} foo` runs `foo`.
    #[test]
    fn an_empty_word_blanks_to_whitespace_and_offsets_hold() {
        let src = "eval {} foo";
        let (t, k) = tail(src, 5, &[("", TokenType::Str), ("foo", TokenType::Esc)]);
        let (win, span) = concat_script_window(&t, &k, src).expect("static");
        assert_eq!(win, "   foo");
        let off = usize::try_from(span.start()).unwrap();
        assert_eq!(off + win.find("foo").unwrap(), src.find("foo").unwrap());
    }

    /// A window with no content at all declines — there is no script to walk.
    #[test]
    fn an_all_whitespace_window_declines() {
        let src = "eval {} {  }";
        let (t, k) = tail(src, 5, &[("", TokenType::Str), ("  ", TokenType::Str)]);
        assert_eq!(concat_script_window(&t, &k, src), None);
    }

    /// A word whose text does not sit verbatim at its token's content offset
    /// cannot be mapped to source bytes, so the window declines.
    #[test]
    fn a_text_that_diverges_from_the_source_declines() {
        let src = "eval {set l2} hello";
        let (mut t, k) = tail(
            src,
            5,
            &[("set l2", TokenType::Str), ("hello", TokenType::Esc)],
        );
        t[1] = "other".to_owned();
        assert_eq!(concat_script_window(&t, &k, src), None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::Span;

    #[test]
    fn sidecar_stubs_load_nearest_dialect_bundle() {
        let root = std::env::temp_dir().join(format!(
            "tcl-lsp-sidecar-stubs-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let nested = root.join("src");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(
            root.join("tcl8.6.tcl.stubs"),
            "# package API\nstub db_eval {sql script:body} -barrier\n",
        )
        .unwrap();
        let file = nested.join("main.tcl");
        let (commands, exprs) = scan_sidecar_stubs(Some(file.to_str().unwrap()), "tcl8.6");
        assert_eq!(exprs.len(), 0);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].name, "db_eval");
        assert!(
            commands[0]
                .flags
                .contains(super::super::types::StubFlags::BARRIER)
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn parse_file_suppression_finds_codes() {
        let src = "# tcl-lsp: disable=W210,W211\nproc foo {} {}\n";
        let codes = parse_file_suppression(src);
        assert!(codes.contains("W210"));
        assert!(codes.contains("W211"));
        assert_eq!(codes.len(), 2);
    }

    #[test]
    fn parse_file_suppression_handles_whitespace_and_case() {
        let src = "  #  TCL-LSP : DISABLE = W210  W211  \n\n# more\nproc foo {} {}\n";
        let codes = parse_file_suppression(src);
        assert!(codes.contains("W210"));
        assert!(codes.contains("W211"));
    }

    #[test]
    fn parse_file_suppression_stops_at_first_non_comment() {
        let src = "# tcl-lsp: disable=W210\nproc foo {} {}\n# tcl-lsp: disable=W211\n";
        let codes = parse_file_suppression(src);
        assert!(codes.contains("W210"));
        assert!(!codes.contains("W211"));
    }

    #[test]
    fn parse_file_suppression_skips_blank_lines() {
        let src = "\n\n# tcl-lsp: disable=W210\n\nproc foo {} {}\n";
        let codes = parse_file_suppression(src);
        assert!(codes.contains("W210"));
    }

    #[test]
    fn parse_file_suppression_handles_wildcard() {
        let src = "# tcl-lsp: disable=*\nproc foo {} {}\n";
        let codes = parse_file_suppression(src);
        assert!(codes.contains("*"));
    }

    #[test]
    fn parse_file_suppression_no_directive_returns_empty() {
        let src = "# Just a comment\nproc foo {} {}\n";
        let codes = parse_file_suppression(src);
        assert!(codes.is_empty());
    }

    #[test]
    fn parse_file_suppression_multibyte_leading_comment_does_not_panic() {
        // Regression: the fixed-byte-offset keyword checks in
        // `parse_disable_directive` used to slice at `line[..kw_end]` /
        // `line[..dis_end]` unconditionally, which panics
        // (`byte index N is not a char boundary`) whenever a multi-byte
        // UTF-8 character — an em dash, an accented letter, any ordinary
        // non-ASCII text — straddles that offset. Neither of these is
        // adversarial input: both are plausible header-comment text.
        //
        // An em dash a few bytes into the leading comment straddles the
        // "tcl-lsp" keyword's 7-byte check.
        let em_dash_early = "# E001 \u{2014} missing dispatch word\nproc foo {} {}\n";
        assert!(parse_file_suppression(em_dash_early).is_empty());

        // An em dash inside the post-colon text straddles the "disable"
        // keyword's own 7-byte check.
        let em_dash_after_colon = "# tcl-lsp: abcdef\u{2014}ghi\nproc foo {} {}\n";
        assert!(parse_file_suppression(em_dash_after_colon).is_empty());
    }

    #[test]
    fn parse_noqa_line_suppressions_records_next_line_for_bare_noqa() {
        // Line 0: ``# noqa`` → suppression keyed on line 1 (the next).
        let src = "# noqa\nset x 1\n";
        let map = parse_noqa_line_suppressions(src);
        let codes = map.get(&1).expect("line 1 entry");
        assert!(codes.contains("*"));
    }

    #[test]
    fn parse_noqa_line_suppressions_records_specific_codes() {
        let src = "# noqa: W210, W211\nset x 1\n";
        let map = parse_noqa_line_suppressions(src);
        let codes = map.get(&1).expect("line 1 entry");
        assert!(codes.contains("W210"));
        assert!(codes.contains("W211"));
        assert!(!codes.contains("*"));
    }

    #[test]
    fn parse_noqa_line_suppressions_orphaned_at_brace_tail() {
        // A ``# noqa`` at the tail of a brace body has no following
        // command in that scope; it suppresses the line that follows
        // physically (often a ``} elseif {…}`` header).
        let src = "if {1} {\n    set x 1\n    # noqa\n} elseif {2} {\n    set y 2\n}\n";
        let map = parse_noqa_line_suppressions(src);
        // ``# noqa`` is on line 2 (0-based) — suppression keyed on
        // line 3 (the ``} elseif …`` header).
        let codes = map.get(&3).expect("line 3 entry");
        assert!(codes.contains("*"));
    }

    #[test]
    fn parse_noqa_line_suppressions_before_comment_line() {
        // ``# noqa`` immediately before another comment line that
        // generates a diagnostic (e.g. W115 on a backslash-continued
        // comment).
        let src = "# noqa: W115\n# trailing \\\nset x 1\n";
        let map = parse_noqa_line_suppressions(src);
        // ``# noqa`` is line 0 → suppresses line 1 (the W115-bearing
        // comment).  The diagnostic-bearing comment also has its own
        // entry registered for the ``# noqa`` shape — the helper
        // records ``# trailing \`` as carrying no noqa (no ``noqa``
        // substring), so only line 1 is in the map.
        let codes = map.get(&1).expect("line 1 entry");
        assert!(codes.contains("W115"));
    }

    #[test]
    fn parse_noqa_line_suppressions_unions_codes_when_multiple_noqa_target_same_line() {
        // Two consecutive noqa comments — the second's next-line is
        // line 2 (set x 1).  The first's next-line is line 1 (the
        // second noqa comment).  Both noqa comments carry their own
        // entries.  Verify line 2 sees only the second noqa's codes;
        // line 1 sees only the first noqa's codes.
        let src = "# noqa: W210\n# noqa: W211\nset x 1\n";
        let map = parse_noqa_line_suppressions(src);
        assert!(map.get(&1).expect("line 1").contains("W210"));
        let line2 = map.get(&2).expect("line 2");
        assert!(line2.contains("W211"));
        assert!(!line2.contains("W210"));
    }

    #[test]
    fn parse_noqa_line_suppressions_skips_non_comment_lines() {
        // The implementation skips any line that isn't ``#``-prefixed
        // after ``trim()`` (so comments only — code lines, blank
        // lines, and lines whose ``noqa`` substring sits inside a
        // Tcl string never seed the map).  Verify line 0 (``set x
        // "noqa"``) does not produce a line-1 entry: the only entry
        // comes from the ``# noqa`` on line 1, which seeds line 2.
        let src = "set x \"noqa\"\n# noqa\nset y 1\n";
        let map = parse_noqa_line_suppressions(src);
        assert!(!map.contains_key(&0));
        assert!(!map.contains_key(&1));
        // ``# noqa`` on line 1 → suppresses line 2.
        assert!(map.get(&2).expect("line 2 entry").contains("*"));
    }

    #[test]
    fn parse_noqa_line_suppressions_empty_for_no_directives() {
        let src = "set x 1\nset y 2\n";
        let map = parse_noqa_line_suppressions(src);
        assert!(map.is_empty());
    }

    #[test]
    fn parse_file_suppression_caps_at_scan_lines() {
        // 200 blank/comment lines, directive at line 150 — should
        // NOT be picked up because the cap is 100.
        let mut src = String::new();
        for _ in 0..150 {
            src.push_str("# noise\n");
        }
        src.push_str("# tcl-lsp: disable=W210\n");
        let codes = parse_file_suppression(&src);
        assert!(codes.is_empty());
    }

    #[test]
    fn format_literal_truncates_long_values() {
        let long = "a".repeat(50);
        let formatted = format_literal_for_message(&long);
        assert!(formatted.ends_with("..."));
        // 37 chars + 3 dots = 40 chars total.
        assert_eq!(formatted.chars().count(), 40);
    }

    #[test]
    fn format_literal_replaces_newlines() {
        assert_eq!(format_literal_for_message("a\nb"), "a\\nb");
    }

    #[test]
    fn format_literal_short_passes_through() {
        assert_eq!(format_literal_for_message("hello"), "hello");
    }

    #[test]
    fn possible_paste_fingerprint_assign_const() {
        let stmt = Statement::AssignConst {
            name: "x".to_string(),
            value: "  hello  ".to_string(),
            span: Span::new(0, 0),
            name_braced: false,
            value_span: None,
        };
        assert_eq!(
            possible_paste_fingerprint(&stmt),
            Some(("x".to_string(), "hello".to_string()))
        );
    }

    fn assign_value(value: &str) -> Statement {
        Statement::AssignValue {
            name: "x".to_string(),
            value: value.to_string(),
            span: Span::new(0, 0),
            name_braced: false,
            value_needs_backsubst: false,
            tokens: None,
        }
    }

    #[test]
    fn possible_paste_fingerprint_assign_value_with_dollar_rejected() {
        assert_eq!(possible_paste_fingerprint(&assign_value("$y")), None);
    }

    #[test]
    fn possible_paste_fingerprint_assign_value_with_bracket_rejected() {
        assert_eq!(possible_paste_fingerprint(&assign_value("[expr 1]")), None);
    }

    #[test]
    fn possible_paste_fingerprint_assign_value_pure_literal() {
        assert_eq!(
            possible_paste_fingerprint(&assign_value("  hello  ")),
            Some(("x".to_string(), "hello".to_string()))
        );
    }

    #[test]
    fn possible_paste_fingerprint_other_stmt_returns_none() {
        // A non-assign statement — return.
        let stmt = Statement::Return {
            value: None,
            expr: None,
            braced: false,
            span: Span::new(0, 0),
        };
        assert_eq!(possible_paste_fingerprint(&stmt), None);
    }

    #[test]
    fn argv_with_word_spans_is_identity() {
        // Rust's segmenter already widens; this helper is a
        // transparent passthrough.
        use tcl_lexer::TokenType;
        let tok = Token::new(TokenType::Esc, Span::new(0, 4));
        let argv = vec![tok];
        let result = argv_with_word_spans(argv.clone(), &[]);
        assert_eq!(result, argv);
    }

    #[test]
    fn irules_top_level_only_matches_registry() {
        // `proc` is the only command tagged with the
        // `IRULES_TOP_LEVEL_ONLY` trait, so the result is exactly
        // `{"proc"}`. Pinning the set surfaces registry-spec drift
        // here.
        let reg = CommandRegistry::build_default();
        let cmds = irules_top_level_only(&reg);
        assert!(
            cmds.contains("proc"),
            "expected `proc` in irules_top_level_only, got {cmds:?}",
        );
    }

    #[test]
    fn parse_param_list_reexport_works() {
        let params = parse_param_list("a b {c default}");
        assert_eq!(params.len(), 3);
        assert_eq!(params[0].name, "a");
        assert_eq!(params[2].name, "c");
        assert!(params[2].has_default);
    }

    #[test]
    fn scan_stub_command_names_extracts_inline_stubs() {
        let src = "\
# tcl-lsp: stubs-begin
# tcl-lsp: stub my_cmd {arg1:var body:body} -loop
# tcl-lsp: stub my_eval {script:body} -barrier
# tcl-lsp: stubs-end
proc foo {} {}
";
        let names = scan_stub_command_names(src);
        assert!(names.contains("my_cmd"));
        assert!(names.contains("my_eval"));
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn scan_stub_command_names_ignores_outside_block() {
        let src = "\
# tcl-lsp: stub orphan_cmd {x:var}
# tcl-lsp: stubs-begin
# tcl-lsp: stub inside {x:var}
# tcl-lsp: stubs-end
# tcl-lsp: stub also_orphan {x:var}
";
        let names = scan_stub_command_names(src);
        assert!(names.contains("inside"));
        assert!(!names.contains("orphan_cmd"));
        assert!(!names.contains("also_orphan"));
    }

    #[test]
    fn scan_stub_command_names_skips_expr_func_op_lines() {
        let src = "\
# tcl-lsp: stubs-begin
# tcl-lsp: stub expr-func sizeof 1
# tcl-lsp: stub expr-op contains 2
# tcl-lsp: stub regular_cmd {x:var}
# tcl-lsp: stubs-end
";
        let names = scan_stub_command_names(src);
        assert!(names.contains("regular_cmd"));
        assert!(!names.iter().any(|n| n.starts_with("expr-")));
    }

    #[test]
    fn scan_stub_command_names_handles_multiple_blocks() {
        let src = "\
# tcl-lsp: stubs-begin
# tcl-lsp: stub a {x:var}
# tcl-lsp: stubs-end
proc foo {} {}
# tcl-lsp: stubs-begin
# tcl-lsp: stub b {y:var}
# tcl-lsp: stubs-end
";
        let names = scan_stub_command_names(src);
        assert!(names.contains("a"));
        assert!(names.contains("b"));
    }

    #[test]
    fn scan_stub_command_names_empty_source() {
        let names = scan_stub_command_names("");
        assert!(names.is_empty());
    }

    // -- ``parse_command_stub`` + ``parse_expr_stub`` tests

    fn cmd_stub(line: &str) -> Option<super::super::types::StubCommandDef> {
        let body = line.trim_start_matches('#').trim_start();
        let after_prefix = strip_tcl_lsp_prefix(body);
        let line_len = u32::try_from(line.len()).expect("test fixture fits in u32");
        parse_command_stub(after_prefix, Span::new(0, line_len))
    }

    fn expr_stub(line: &str) -> Option<(&'static str, String, u32)> {
        let body = line.trim_start_matches('#').trim_start();
        let after_prefix = strip_tcl_lsp_prefix(body);
        parse_expr_stub(after_prefix).map(|(k, n, a)| (k, n.to_string(), a))
    }

    #[test]
    fn parse_command_stub_simple_command() {
        let stub = cmd_stub("# tcl-lsp: stub my_cmd {arg1 arg2}").unwrap();
        assert_eq!(stub.name, "my_cmd");
        assert_eq!(stub.args.len(), 2);
        assert_eq!(stub.args[0].name, "arg1");
        assert_eq!(stub.args[0].role, "value");
        assert_eq!(stub.args[1].name, "arg2");
    }

    #[test]
    fn parse_command_stub_with_roles() {
        let stub = cmd_stub(
            "# tcl-lsp: stub foreach_in_collection {varName:var collection body:body} -loop",
        )
        .unwrap();
        assert_eq!(stub.name, "foreach_in_collection");
        assert_eq!(stub.args[0].role, "var");
        assert_eq!(stub.args[1].role, "value");
        assert_eq!(stub.args[2].role, "body");
        assert!(stub.flags.contains(super::super::types::StubFlags::LOOP));
        assert!(!stub.flags.contains(super::super::types::StubFlags::BARRIER));
    }

    #[test]
    fn parse_command_stub_all_flags() {
        let stub = cmd_stub(
            "# tcl-lsp: stub dangerous {script:body} -barrier -unsafe -mutator -scope_alias",
        )
        .unwrap();
        assert!(stub.flags.contains(super::super::types::StubFlags::BARRIER));
        assert!(stub.flags.contains(super::super::types::StubFlags::UNSAFE));
        assert!(stub.flags.contains(super::super::types::StubFlags::MUTATOR));
        assert!(
            stub.flags
                .contains(super::super::types::StubFlags::SCOPE_ALIAS)
        );
        assert!(!stub.flags.contains(super::super::types::StubFlags::PURE));
        assert!(!stub.flags.contains(super::super::types::StubFlags::LOOP));
    }

    #[test]
    fn parse_command_stub_optional_args() {
        let stub = cmd_stub("# tcl-lsp: stub redirect {?-file? target body:body}").unwrap();
        assert!(stub.args[0].optional);
        assert_eq!(stub.args[0].name, "-file");
        assert!(!stub.args[1].optional);
    }

    #[test]
    fn parse_command_stub_bare_format() {
        let stub = cmd_stub("stub get_cells {pattern:pattern} -pure").unwrap();
        assert_eq!(stub.name, "get_cells");
        assert!(stub.flags.contains(super::super::types::StubFlags::PURE));
    }

    #[test]
    fn parse_command_stub_invalid_role_returns_none() {
        assert!(cmd_stub("# tcl-lsp: stub bad {arg:invalid_role}").is_none());
    }

    #[test]
    fn parse_command_stub_not_a_stub_returns_none() {
        assert!(cmd_stub("# just a regular comment").is_none());
    }

    #[test]
    fn multibyte_comment_before_keyword_length_does_not_panic() {
        // A non-ASCII byte (an em dash here) landing inside the
        // `tcl-lsp` / `stub` / `expr-func` / `expr-op` keyword's byte
        // length used to panic on a raw `s[..len]` slice instead of
        // just not matching. Each string below is crafted so the em
        // dash straddles the exact byte offset the check compares.
        assert!(cmd_stub("# st—b my_cmd {arg}").is_none());
        assert!(cmd_stub("# abcde—z not a stub").is_none());
        assert!(expr_stub("# st—b my_cmd {arg}").is_none());
        assert!(expr_stub("# tcl-lsp: stub expr-fun—c sizeof 1").is_none());
    }

    #[test]
    fn parse_command_stub_empty_args() {
        let stub = cmd_stub("# tcl-lsp: stub no_args {} -pure").unwrap();
        assert!(stub.args.is_empty());
        assert!(stub.flags.contains(super::super::types::StubFlags::PURE));
    }

    #[test]
    fn parse_command_stub_no_command_name_returns_none() {
        assert!(cmd_stub("# tcl-lsp: stub").is_none());
    }

    #[test]
    fn parse_command_stub_unclosed_optional_marker() {
        // A ``?arg`` without closing ``?`` is treated as a
        // regular argument name.
        let stub = cmd_stub("# tcl-lsp: stub cmd {?arg}").unwrap();
        assert!(!stub.args[0].optional);
        assert_eq!(stub.args[0].name, "?arg");
    }

    #[test]
    fn parse_command_stub_missing_brace_block_rejected() {
        // The ``{ARGS}`` block is required.
        assert!(cmd_stub("# tcl-lsp: stub bare_name").is_none());
    }

    #[test]
    fn parse_expr_stub_func() {
        let (kind, name, arity) = expr_stub("# tcl-lsp: stub expr-func sizeof 1").unwrap();
        assert_eq!(kind, "function");
        assert_eq!(name, "sizeof");
        assert_eq!(arity, 1);
    }

    #[test]
    fn parse_expr_stub_op() {
        let (kind, name, arity) = expr_stub("# tcl-lsp: stub expr-op contains 2").unwrap();
        assert_eq!(kind, "operator");
        assert_eq!(name, "contains");
        assert_eq!(arity, 2);
    }

    #[test]
    fn parse_expr_stub_default_func_arity() {
        let (_, _, arity) = expr_stub("stub expr-func myfunc").unwrap();
        assert_eq!(arity, 1);
    }

    #[test]
    fn parse_expr_stub_default_op_arity() {
        let (_, _, arity) = expr_stub("stub expr-op myop").unwrap();
        assert_eq!(arity, 2);
    }

    #[test]
    fn parse_expr_stub_not_expr_returns_none() {
        assert!(expr_stub("stub get_cells {pattern:pattern}").is_none());
    }

    #[test]
    fn scan_source_for_stubs_populates_args_and_flags() {
        let src = "\
# tcl-lsp: stubs-begin
# tcl-lsp: stub foreach_in_collection {varName:var collection body:body} -loop
# tcl-lsp: stub my_eval {script:body} -barrier
# tcl-lsp: stubs-end
proc foo {} {}
";
        let (cmds, exprs) = scan_source_for_stubs(src);
        assert!(exprs.is_empty());
        assert_eq!(cmds.len(), 2);
        let foreach = cmds
            .iter()
            .find(|c| c.name == "foreach_in_collection")
            .unwrap();
        assert!(foreach.flags.contains(super::super::types::StubFlags::LOOP));
        assert_eq!(foreach.args.len(), 3);
        assert_eq!(foreach.args[0].role, "var");
        assert_eq!(foreach.args[2].role, "body");
        let myeval = cmds.iter().find(|c| c.name == "my_eval").unwrap();
        assert!(
            myeval
                .flags
                .contains(super::super::types::StubFlags::BARRIER)
        );
        assert!(!myeval.flags.contains(super::super::types::StubFlags::LOOP));
    }

    #[test]
    fn scan_source_for_stubs_carries_expr_arity() {
        let src = "\
# tcl-lsp: stubs-begin
# tcl-lsp: stub expr-func sizeof 3
# tcl-lsp: stub expr-op contains
# tcl-lsp: stubs-end
";
        let (_, exprs) = scan_source_for_stubs(src);
        assert_eq!(exprs.len(), 2);
        let sizeof = exprs.iter().find(|e| e.name == "sizeof").unwrap();
        assert_eq!(sizeof.kind, "function");
        assert_eq!(sizeof.arity, 3);
        let contains = exprs.iter().find(|e| e.name == "contains").unwrap();
        assert_eq!(contains.kind, "operator");
        assert_eq!(contains.arity, 2);
    }

    #[test]
    fn scan_source_for_stubs_drops_invalid_role() {
        // The whole stub is dropped when any token uses an
        // unrecognised role.
        let src = "\
# tcl-lsp: stubs-begin
# tcl-lsp: stub good {arg:var}
# tcl-lsp: stub bad {arg:bogus}
# tcl-lsp: stubs-end
";
        let (cmds, _) = scan_source_for_stubs(src);
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].name, "good");
    }

    #[test]
    fn scan_source_for_stubs_handles_non_ascii_comments() {
        // Regression: a `#` comment opening with a multi-byte char used to
        // panic `matches_marker`'s byte slicing (`s[..7]` splitting inside the
        // char). Boundary-safe `str::get` must simply not match.
        let src = "\
# ═══ banner ═══
# tcl-lsp: stubs-begin
# tcl-lsp: stub good {arg:var}
# tcl-lsp: stubs-end
";
        let (cmds, _) = scan_source_for_stubs(src);
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].name, "good");
    }

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    #[test]
    fn recovery_known_commands_includes_registry_names() {
        let known =
            recovery_known_commands("set x [foo\n", &registry(), &std::sync::Arc::default());
        assert!(known.contains("set"));
        assert!(known.contains("puts"));
    }

    #[test]
    fn recovery_known_commands_includes_extra_even_for_a_complete_script() {
        // `extra` (the LSP-layer-widened `extraCommands` set) is unioned in
        // unconditionally, same as the registry — unlike the in-file
        // signature scan it is not gated on `script_is_complete`, since it
        // costs nothing extra to fold in a set the caller already built.
        let extra: std::sync::Arc<HashSet<String>> =
            std::sync::Arc::new(["workspace_helper".to_string()].into_iter().collect());
        let known = recovery_known_commands("set x 1\n", &registry(), &extra);
        assert!(known.contains("workspace_helper"));
    }

    /// Issue #1154: the caller-supplied set must be *shared*, not copied — on
    /// the LSP recovery path it holds every workspace-indexed proc and class,
    /// and the recovery branch runs on every keystroke inside an unterminated
    /// block.
    #[test]
    fn recovery_known_commands_shares_the_extra_set_rather_than_copying_it() {
        let extra: SharedNameSet =
            std::sync::Arc::new(["workspace_helper".to_string()].into_iter().collect());
        let before = std::sync::Arc::strong_count(&extra);
        let known = recovery_known_commands("set q {\nunclosed\n", &registry(), &extra);
        assert_eq!(
            std::sync::Arc::strong_count(&extra),
            before + 1,
            "the extra layer must be shared with the caller, not deep-copied",
        );
        assert!(known.contains("workspace_helper"));
        assert!(known.contains("set"), "the registry layer is still present");
        drop(known);
        assert_eq!(
            std::sync::Arc::strong_count(&extra),
            before,
            "dropping the universe must release the shared layer",
        );
    }

    #[test]
    fn recovery_known_commands_skips_the_scan_for_a_complete_script() {
        // The extra signature scan only runs on an incomplete script — a
        // well-formed document (however many procs it defines) never pays
        // for it, and its user-defined names are simply absent (recovery
        // never runs on complete input anyway).
        let known = recovery_known_commands(
            "proc my_helper {} {}\nmy_helper\n",
            &registry(),
            &std::sync::Arc::default(),
        );
        assert!(!known.contains("my_helper"));
        assert!(known.contains("proc")); // registry names are always present
    }

    #[test]
    fn recovery_known_commands_adds_user_proc_qualified_and_tail() {
        let src = "namespace eval myns {\n  proc helper {x} {return $x}\n}\n\nset q {\nunclosed\n";
        let known = recovery_known_commands(src, &registry(), &std::sync::Arc::default());
        assert!(
            known.contains("myns::helper"),
            "expected the qualified name: {known:?}"
        );
        assert!(
            known.contains("helper"),
            "expected the unqualified tail: {known:?}"
        );
    }

    #[test]
    fn recovery_known_commands_adds_the_leading_colon_colon_form_too() {
        // `signature_scan::qualify` always records `::myns::helper`; a
        // recovery-point call can equally be written that way (absolute
        // Tcl syntax), so the leading-`::` form must be recognised
        // alongside the stripped/tail forms asserted above.
        let src = "namespace eval myns {\n  proc helper {x} {return $x}\n}\n\nset q {\nunclosed\n";
        let known = recovery_known_commands(src, &registry(), &std::sync::Arc::default());
        assert!(
            known.contains("::myns::helper"),
            "expected the fully-qualified leading-:: form: {known:?}"
        );
    }

    #[test]
    fn recovery_known_commands_adds_user_class_and_alias() {
        let src =
            "oo::class create Widget {}\ninterp alias {} greet {} puts hi\n\nset q {\nunclosed\n";
        let known = recovery_known_commands(src, &registry(), &std::sync::Arc::default());
        assert!(known.contains("Widget"), "{known:?}");
        assert!(known.contains("greet"), "{known:?}");
    }

    #[test]
    fn recovery_known_commands_adds_rename_target() {
        let src = "rename puts my_puts\n\nset q {\nunclosed\n";
        let known = recovery_known_commands(src, &registry(), &std::sync::Arc::default());
        assert!(
            known.contains("my_puts"),
            "expected the rename target: {known:?}"
        );
    }

    #[test]
    fn recovery_known_commands_adds_namespaced_rename_target() {
        let src = "namespace eval myns {\n  rename puts loud_puts\n}\n\nset q {\nunclosed\n";
        let known = recovery_known_commands(src, &registry(), &std::sync::Arc::default());
        assert!(
            known.contains("myns::loud_puts"),
            "expected the qualified rename target: {known:?}"
        );
        assert!(
            known.contains("loud_puts"),
            "expected the unqualified tail: {known:?}"
        );
    }

    #[test]
    fn recovery_known_commands_rename_to_empty_deletes_not_defines() {
        // `rename OLD {}` deletes OLD; it must not introduce an empty-string
        // "command name".
        let src = "rename puts {}\n\nset q {\nunclosed\n";
        let known = recovery_known_commands(src, &registry(), &std::sync::Arc::default());
        assert!(!known.contains(""));
    }

    #[test]
    fn recovery_known_commands_does_not_invent_undefined_names() {
        let src = "set q {\nunclosed\n";
        let known = recovery_known_commands(src, &registry(), &std::sync::Arc::default());
        assert!(!known.contains("not_a_real_proc"));
        assert!(!known.contains("unclosed")); // plain data, not a definition
    }
}
