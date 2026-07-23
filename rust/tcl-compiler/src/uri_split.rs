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

//! IRULE3103 — suggest `*::path` / `*::query` over `*::uri`.
//!
//! Detects patterns where a developer uses a `*::uri` getter and then
//! decomposes the result manually, when a purpose-built `*::path` or
//! `*::query` command would be clearer, cheaper, and eligible for the
//! `-normalized` flag.
//!
//! Detected patterns (all at the IR / CFG / SCCP level):
//!
//! * `split $uri "?"` / `split $uri "&"`
//! * `[HTTP::uri] starts_with "/path"` (path-like operand, no `?`/`&`)
//! * `[HTTP::uri] ends_with ".html"` (path-like suffix)
//! * `[HTTP::uri] contains "&key="` (query-like operand)
//! * `[HTTP::uri] matches_glob "/api/*"` (path-like pattern)
//! * `[HTTP::uri] matches_glob "*&key=*"` (query-like pattern)
//! * `string match "/api/*" $uri`
//! * `string first "?" $uri`
//!
//! The pass also generalises to **any** `*::uri` command that has
//! sibling `*::path` and/or `*::query` commands in the registry.

use std::collections::{HashMap, HashSet};
use tcl_core_types::DiagCode;

use tcl_lexer::Span;
use tcl_registry::CommandRegistry;

use crate::analyses::{ConstValue, LatticeValue};
use crate::cfg::{BlockId, Function as CfgFunction, Terminator};
use crate::depth_guard::MAX_EXPR_NODE_DEPTH;
use crate::expr_ast::{BinOp, ExprNode};
use crate::expr_parser::parse_expr;
use crate::ir::Statement;
use crate::naming::normalise_var_name;
use crate::ssa::{Phi, SsaFunction, SsaStatement, Symbol, ValueKey, Version};
use crate::taint::{TaintWarning, is_irules_dialect};
use crate::value_shapes::{is_pure_var_ref, parse_command_substitution};

/// Maximum depth for backward SSA tracing (prevents infinite loops on
/// pathological phi chains).
const MAX_TRACE_DEPTH: tcl_core_types::RecursionLimit = tcl_core_types::RecursionLimit(12);

/// Characters that indicate a query-string delimiter when literal.
const QUERY_CHARS: &[char] = &['?', '&'];

/// True for the bare built-in `split` and its fully-qualified `::split`
/// form; both denote the same Tcl built-in. Code may write either form
/// — the SSA / IR layer preserves the surface text — so detection
/// helpers must accept both to avoid false negatives like
/// `set parts [::split $uri "?"]`.
fn is_split_cmd(cmd: &str) -> bool {
    matches!(cmd, "split" | "::split")
}

/// True for the bare `string` ensemble and its fully-qualified
/// `::string` form. Same rationale as [`is_split_cmd`].
fn is_string_cmd(cmd: &str) -> bool {
    matches!(cmd, "string" | "::string")
}

// URI family discovery

/// Map `uri_cmd → (path_cmd?, query_cmd?)` derived from the registry.
pub type UriFamilies = HashMap<String, (Option<String>, Option<String>)>;

/// Build the [`UriFamilies`] map from a command registry.
///
/// Scans the command registry for `*::uri` commands that have a
/// sibling `*::path` and/or `*::query` registered for the iRules
/// dialect.
#[must_use]
pub fn uri_families(registry: &CommandRegistry) -> UriFamilies {
    let mut all_names: HashSet<String> = HashSet::new();
    for name in registry.command_names() {
        use tcl_registry::ProfileQueries;
        if let Some(spec) = registry.get(name)
            && tcl_dialect::DialectProfile::irules().is_available(spec)
        {
            all_names.insert(name.to_owned());
        }
    }

    let mut families: UriFamilies = HashMap::new();
    for uri in all_names.iter().filter(|n| n.ends_with("::uri")) {
        let Some(ns_end) = uri.rfind("::") else {
            continue;
        };
        let ns = &uri[..ns_end];
        let path_cmd = format!("{ns}::path");
        let query_cmd = format!("{ns}::query");
        let path = all_names.contains(&path_cmd).then_some(path_cmd);
        let query = all_names.contains(&query_cmd).then_some(query_cmd);
        if path.is_some() || query.is_some() {
            families.insert(uri.clone(), (path, query));
        }
    }
    families
}

/// Return `true` when `command` is a `*::uri` getter (zero-arg form).
fn is_uri_getter(families: &UriFamilies, command: &str, args: &[String]) -> bool {
    families.contains_key(command) && args.is_empty()
}

/// Return `(path_cmd, query_cmd)` for a `*::uri` command, both `None`
/// when the command is not in the family table.
fn uri_siblings<'a>(
    families: &'a UriFamilies,
    command: &str,
) -> (Option<&'a str>, Option<&'a str>) {
    families
        .get(command)
        .map_or((None, None), |(p, q)| (p.as_deref(), q.as_deref()))
}

/// Bundle of references threaded through the SSA / expression walkers.
///
/// Replaces the seven-argument signatures the helpers would otherwise
/// need. Lives just for the duration of one
/// [`find_uri_split_suggestions`] call.
#[derive(Clone, Copy)]
struct TraceCtx<'a> {
    cfg: &'a CfgFunction,
    ssa: &'a SsaFunction,
    families: &'a UriFamilies,
    def_sites: &'a HashMap<ValueKey, (BlockId, usize)>,
    phi_index: &'a HashMap<ValueKey, Phi>,
}

// Tcl quoting helper

/// Strip surrounding Tcl quoting (double-quotes or braces) from a word.
///
/// `parse_command_substitution` uses naive whitespace splitting, so
/// arguments arrive with their outer quoting intact.
fn strip_tcl_quotes(arg: &str) -> &str {
    let stripped = arg.trim();
    if stripped.len() >= 2 {
        if let Some(inner) = stripped.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
            return inner;
        }
        if let Some(inner) = stripped.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
            return inner;
        }
    }
    stripped
}

// Literal / SCCP resolution

/// Return the resolved literal string, or `None` if unknown.
fn resolve_literal<S: std::hash::BuildHasher>(
    arg: &str,
    sccp_values: Option<&HashMap<ValueKey, LatticeValue, S>>,
    uses: &HashMap<Symbol, Version>,
    ssa: &SsaFunction,
) -> Option<String> {
    let cleaned = strip_tcl_quotes(arg);
    if !cleaned.contains('$') && !cleaned.contains('[') {
        return Some(cleaned.to_owned());
    }
    let sccp = sccp_values?;
    if !is_pure_var_ref(cleaned) {
        return None;
    }
    let var_name = normalise_var_name(cleaned);
    if var_name.is_empty() {
        return None;
    }
    let sym = ssa.var_symbol(var_name)?;
    let ver = *uses.get(&sym).unwrap_or(&0);
    let lv = sccp.get(&(sym, ver))?;
    if let LatticeValue::Const(ConstValue::String(s)) = lv {
        Some(s.clone())
    } else {
        None
    }
}

// Pattern classification

/// Return `true` when `operand` is unambiguously a path pattern.
///
/// Path-like: starts with `/` and contains no `?` or `&` or `=`.
/// Also matches file extensions (e.g. `.html`, `.js`) or path
/// segments that contain no query-string characters.
fn is_path_like(operand: &str) -> bool {
    if operand.contains('?') || operand.contains('&') || operand.contains('=') {
        return false;
    }
    if operand.starts_with('/') {
        return true;
    }
    // Bare file extension or path-like token (no query chars).
    operand.starts_with('.') && !operand.contains('/')
}

/// Return `true` when `operand` is unambiguously a query pattern.
///
/// Query-like: contains `&` or `=` (like `key=val`), or starts with `?`.
fn is_query_like(operand: &str) -> bool {
    operand.contains('&') || operand.contains('=') || operand.starts_with('?')
}

/// Classify a glob pattern as `"path"` or `"query"` or `None`.
///
/// * `/api/*` (no `&` / `=`) → `"path"`
/// * `*&key=*` → `"query"`
///
/// Note: `?` is a single-character wildcard in glob syntax, so it is
/// **not** treated as a query-string delimiter here.
fn classify_glob_pattern(pattern: &str) -> Option<&'static str> {
    // Strip leading glob wildcard to inspect the meaningful prefix.
    let meaningful = pattern.trim_start_matches('*');
    if meaningful.starts_with('/') && !pattern.contains('&') && !pattern.contains('=') {
        return Some("path");
    }
    if pattern.contains('&') || pattern.contains('=') {
        return Some("query");
    }
    None
}

/// Classify a regex pattern as `"path"` or `"query"` or `None`.
///
/// Applies heuristics with common regex anchoring stripped (`^`, `\A`).
///
/// Note: bare `?` is a regex quantifier and is **not** treated as a
/// query delimiter.  Escaped `\?` is the literal form and *is* a
/// positive signal for query matching.
fn classify_regex_pattern(pattern: &str) -> Option<&'static str> {
    let stripped: &str = pattern
        .trim_start_matches('^')
        .strip_prefix("\\A")
        .unwrap_or_else(|| pattern.trim_start_matches('^'));
    if stripped.starts_with('/')
        && !stripped.contains('&')
        && !stripped.contains('=')
        && !pattern.contains("\\?")
    {
        return Some("path");
    }
    if pattern.contains("\\?") || pattern.contains("\\&") {
        return Some("query");
    }
    if stripped.contains('&') || stripped.contains('=') {
        return Some("query");
    }
    None
}

// Backward SSA tracing

/// Map `(var_name, version) → (block_id, statement_index)`.
fn build_def_site_map(ssa: &SsaFunction) -> HashMap<ValueKey, (BlockId, usize)> {
    let mut result: HashMap<ValueKey, (BlockId, usize)> = HashMap::new();
    for (bn, ssa_block) in &ssa.blocks {
        for (idx, s) in ssa_block.statements.iter().enumerate() {
            for (&var, &ver) in &s.defs {
                result.insert((var, ver), (*bn, idx));
            }
        }
    }
    result
}

/// Build a `(var_name, version) → Phi` lookup table.
fn build_phi_index(ssa: &SsaFunction) -> HashMap<ValueKey, Phi> {
    let mut index: HashMap<ValueKey, Phi> = HashMap::new();
    for ssa_block in ssa.blocks.values() {
        for phi in &ssa_block.phis {
            index.insert((phi.name, phi.version), phi.clone());
        }
    }
    index
}

/// Walk backward through SSA defs to find the originating `*::uri`
/// command. Returns the command name (e.g. `"HTTP::uri"`) if the value
/// provably flows from a single URI getter call with no intermediate
/// transformation, or `None` otherwise.
fn trace_to_uri_family(
    var_name: &str,
    version: Version,
    ctx: TraceCtx<'_>,
    depth: u32,
) -> Option<String> {
    if MAX_TRACE_DEPTH.exceeded(depth) {
        return None;
    }

    let sym = ctx.ssa.var_symbol(var_name)?;
    let key: ValueKey = (sym, version);
    let Some((block_id, idx)) = ctx.def_sites.get(&key).copied() else {
        // Check phi nodes via index (O(1) lookup).
        let phi = ctx.phi_index.get(&key)?;
        // All incoming edges must agree on the same origin.
        let mut origin: Option<String> = None;
        for inc_ver in phi.incoming.values() {
            if *inc_ver == 0 {
                // A version-0 incoming operand is the entry/live-in value
                // (a parameter, caller value, or uninitialised def) — NOT a
                // URI-getter result. Skipping it would attribute a merge like
                // `φ(x0_livein, x1_from_HTTP::uri)` wholly to `HTTP::uri` and
                // fire a spurious IRULE3103. The value is therefore not
                // *provably* a single URI family: bail (issue 150).
                return None;
            }
            let candidate = trace_to_uri_family(var_name, *inc_ver, ctx, depth + 1)?;
            match &origin {
                None => origin = Some(candidate),
                Some(existing) if existing != &candidate => return None,
                _ => {}
            }
        }
        return origin;
    };

    let block = ctx.cfg.blocks.get(&block_id)?;
    let ssa_block = ctx.ssa.blocks.get(&block_id)?;
    if idx >= block.statements.len() {
        return None;
    }
    let stmt = &block.statements[idx];
    let ssa_stmt = &ssa_block.statements[idx];

    match stmt {
        Statement::Call {
            command,
            args,
            defs,
            ..
        } if !defs.is_empty() => {
            if is_uri_getter(ctx.families, command, args) {
                Some(command.clone())
            } else {
                None
            }
        }
        Statement::AssignValue { value, .. } => {
            let value = value.trim();
            if let Some((cmd_name, cmd_args)) = parse_command_substitution(value) {
                if is_uri_getter(ctx.families, &cmd_name, &cmd_args) {
                    return Some(cmd_name);
                }
                return None;
            }
            if is_pure_var_ref(value) {
                let src_name = normalise_var_name(value);
                if !src_name.is_empty() {
                    let src_ver = ctx
                        .ssa
                        .var_symbol(src_name)
                        .and_then(|s| ssa_stmt.uses.get(&s))
                        .copied()
                        .unwrap_or(0);
                    if src_ver > 0 {
                        return trace_to_uri_family(src_name, src_ver, ctx, depth + 1);
                    }
                }
            }
            None
        }
        _ => None,
    }
}

/// Return the `*::uri` command name if `input_arg` traces back to one.
fn arg_traces_to_uri_family(
    input_arg: &str,
    ssa_stmt: &SsaStatement,
    ctx: TraceCtx<'_>,
) -> Option<String> {
    if is_pure_var_ref(input_arg) {
        let var_name = normalise_var_name(input_arg);
        if var_name.is_empty() {
            return None;
        }
        let ver = ctx
            .ssa
            .var_symbol(var_name)
            .and_then(|s| ssa_stmt.uses.get(&s))
            .copied()
            .unwrap_or(0);
        if ver == 0 {
            return None;
        }
        return trace_to_uri_family(var_name, ver, ctx, 0);
    }

    if input_arg.starts_with('[')
        && input_arg.ends_with(']')
        && let Some((cmd, args)) = parse_command_substitution(input_arg)
        && is_uri_getter(ctx.families, &cmd, &args)
    {
        return Some(cmd);
    }
    None
}

// Message builders

fn split_message(families: &UriFamilies, uri_cmd: &str, sep: &str) -> String {
    let (path_cmd, query_cmd) = uri_siblings(families, uri_cmd);
    let has_q = sep.contains('?');
    let has_amp = sep.contains('&');

    let join_path_query = || {
        let mut parts = Vec::new();
        if let Some(p) = path_cmd {
            parts.push(p);
        }
        if let Some(q) = query_cmd {
            parts.push(q);
        }
        parts.join(" and ")
    };

    if has_q && !has_amp {
        return format!(
            "Splitting {uri_cmd} on '?' to extract path or query; \
             use {} instead for clearer, more efficient URI decomposition.",
            join_path_query()
        );
    }
    if has_amp && !has_q {
        if let Some(q) = query_cmd {
            return format!(
                "Splitting {uri_cmd} on '&' to parse query parameters; \
                 use {q} to obtain the query string directly, then split on '&' if needed.",
            );
        }
        return format!(
            "Splitting {uri_cmd} on '&' to parse query parameters; \
             consider using the query-string accessor directly.",
        );
    }
    format!(
        "Splitting {uri_cmd} on '?' and '&' to decompose the request; \
         use {} for clearer, more efficient URI handling.",
        join_path_query()
    )
}

fn comparison_message(
    families: &UriFamilies,
    uri_cmd: &str,
    op_name: &str,
    component: &str,
) -> String {
    let (path_cmd, query_cmd) = uri_siblings(families, uri_cmd);
    if component == "path"
        && let Some(p) = path_cmd
    {
        return format!(
            "{uri_cmd} used with {op_name} on a path-like pattern; \
                 use {p} instead for clearer intent and to avoid query-string interference.",
        );
    }
    if component == "query"
        && let Some(q) = query_cmd
    {
        return format!(
            "{uri_cmd} used with {op_name} on a query-like pattern; \
                 use {q} instead for direct access to the query string.",
        );
    }
    let mut parts = Vec::new();
    if let Some(p) = path_cmd {
        parts.push(p);
    }
    if let Some(q) = query_cmd {
        parts.push(q);
    }
    format!(
        "{uri_cmd} used with {op_name}; consider using {} for more precise matching.",
        parts.join(" or ")
    )
}

fn string_first_message(families: &UriFamilies, uri_cmd: &str, needle: &str) -> String {
    let (path_cmd, query_cmd) = uri_siblings(families, uri_cmd);
    let mut parts = Vec::new();
    if let Some(p) = path_cmd {
        parts.push(p);
    }
    if let Some(q) = query_cmd {
        parts.push(q);
    }
    format!(
        "Searching {uri_cmd} for '{needle}' to decompose the URI; \
         use {} instead for direct access to path and query components.",
        parts.join(" and ")
    )
}

fn expr_hit_message(
    families: &UriFamilies,
    uri_cmd: &str,
    op_name: &str,
    component: &str,
) -> String {
    if op_name == "string first" {
        return string_first_message(families, uri_cmd, component);
    }
    if op_name == "split" {
        return split_message(families, uri_cmd, component);
    }
    comparison_message(families, uri_cmd, op_name, component)
}

// Expression-level detection

/// Default classification for operators that pick `"path"` first
/// (literal `/`-prefix or `.ext`) and otherwise `"query"`.
fn classify_path_first(operand: &str) -> Option<&'static str> {
    if is_path_like(operand) {
        Some("path")
    } else if is_query_like(operand) {
        Some("query")
    } else {
        None
    }
}

/// Return `"path"` or `"query"` for a literal operand, or `None`.
fn classify_operand_for_op(op: BinOp, operand: &str) -> Option<&'static str> {
    match op {
        BinOp::StartsWith | BinOp::Contains | BinOp::StrEq | BinOp::StrEquals | BinOp::Eq => {
            classify_path_first(operand)
        }
        BinOp::EndsWith => {
            // `ends_with` widens "no query chars" to "path" (a bare suffix
            // like ".html" without a leading `/` is still path-shaped),
            // but query indicators still win.
            if is_query_like(operand) {
                Some("query")
            } else if !operand.contains('?') && !operand.contains('&') && !operand.contains('=') {
                Some("path")
            } else {
                None
            }
        }
        BinOp::MatchesGlob => classify_glob_pattern(operand),
        BinOp::MatchesRegex => classify_regex_pattern(operand),
        _ => None,
    }
}

fn is_comparison_op(op: BinOp) -> bool {
    matches!(
        op,
        BinOp::StartsWith
            | BinOp::EndsWith
            | BinOp::Contains
            | BinOp::MatchesGlob
            | BinOp::MatchesRegex
            | BinOp::StrEq
            | BinOp::StrEquals
            | BinOp::Eq
    )
}

/// Return the unquoted literal text from an expression node, or `None`.
fn expr_literal_text(node: &ExprNode) -> Option<String> {
    match node {
        ExprNode::String { text, .. } => Some(strip_tcl_quotes(text).to_owned()),
        ExprNode::Literal { text, .. } => Some(text.clone()),
        _ => None,
    }
}

/// Return the `*::uri` command name if `node` traces back to one.
fn expr_traces_to_uri(
    node: &ExprNode,
    ssa_versions: &HashMap<Symbol, Version>,
    ctx: TraceCtx<'_>,
) -> Option<String> {
    match node {
        ExprNode::Command { text, .. } => {
            let (cmd, args) = parse_command_substitution(text)?;
            if is_uri_getter(ctx.families, &cmd, &args) {
                Some(cmd)
            } else {
                None
            }
        }
        ExprNode::Var { name, .. } => {
            let ver = ctx
                .ssa
                .var_symbol(name)
                .and_then(|s| ssa_versions.get(&s))
                .copied()
                .unwrap_or(0);
            if ver == 0 {
                return None;
            }
            trace_to_uri_family(name, ver, ctx, 0)
        }
        _ => None,
    }
}

/// A hit produced by the expression walker: `(uri_cmd, op_name, component)`.
type ExprHit = (String, String, String);

fn check_expr_binary(
    op: BinOp,
    left: &ExprNode,
    right: &ExprNode,
    ssa_versions: &HashMap<Symbol, Version>,
    ctx: TraceCtx<'_>,
) -> Option<ExprHit> {
    if !is_comparison_op(op) {
        return None;
    }

    // Pattern: <uri_expr> op <literal>
    if let Some(uri_cmd) = expr_traces_to_uri(left, ssa_versions, ctx)
        && let Some(lit) = expr_literal_text(right)
        && let Some(component) = classify_operand_for_op(op, &lit)
    {
        return Some((uri_cmd, op.as_str().to_owned(), component.to_owned()));
    }

    // Reversed operand order (uncommon but possible with eq/equals/==).
    if matches!(op, BinOp::StrEq | BinOp::StrEquals | BinOp::Eq)
        && let Some(uri_cmd) = expr_traces_to_uri(right, ssa_versions, ctx)
        && let Some(lit) = expr_literal_text(left)
        && let Some(component) = classify_operand_for_op(op, &lit)
    {
        return Some((uri_cmd, op.as_str().to_owned(), component.to_owned()));
    }
    None
}

/// Trace a single `[…] $var` argument through SSA to its `*::uri`
/// origin, given the live versions at the use site. Used by both
/// `check_expr_command`'s `string match` / `string first` / `split`
/// branches.
fn input_arg_uri(
    input_arg: &str,
    ssa_versions: &HashMap<Symbol, Version>,
    ctx: TraceCtx<'_>,
) -> Option<String> {
    let var_name = normalise_var_name(input_arg);
    if !is_pure_var_ref(input_arg) || var_name.is_empty() {
        return None;
    }
    let ver = ctx
        .ssa
        .var_symbol(var_name)
        .and_then(|s| ssa_versions.get(&s))
        .copied()
        .unwrap_or(0);
    if ver == 0 {
        return None;
    }
    trace_to_uri_family(var_name, ver, ctx, 0)
}

fn check_expr_command(
    text: &str,
    ssa_versions: &HashMap<Symbol, Version>,
    ctx: TraceCtx<'_>,
) -> Option<ExprHit> {
    let (cmd_name, cmd_args) = parse_command_substitution(text)?;

    if is_string_cmd(&cmd_name) && !cmd_args.is_empty() {
        let sub = cmd_args[0].as_str();
        if sub == "match" {
            let mut remaining: &[String] = &cmd_args[1..];
            if remaining.first().is_some_and(|w| w == "-nocase") {
                remaining = &remaining[1..];
            }
            if remaining.len() < 2 {
                return None;
            }
            let pattern = strip_tcl_quotes(&remaining[0]).to_owned();
            let input_arg = strip_tcl_quotes(&remaining[1]).to_owned();
            let uri_cmd = input_arg_uri(&input_arg, ssa_versions, ctx)?;
            let component = classify_glob_pattern(&pattern)?;
            return Some((uri_cmd, "string match".to_owned(), component.to_owned()));
        }
        if sub == "first" && cmd_args.len() >= 3 {
            let needle = strip_tcl_quotes(&cmd_args[1]).to_owned();
            let input_arg = strip_tcl_quotes(&cmd_args[2]).to_owned();
            let uri_cmd = input_arg_uri(&input_arg, ssa_versions, ctx)?;
            if needle.chars().any(|c| QUERY_CHARS.contains(&c)) {
                return Some((uri_cmd, "string first".to_owned(), needle));
            }
        }
    }

    if is_split_cmd(&cmd_name) && cmd_args.len() >= 2 {
        let sep = strip_tcl_quotes(&cmd_args[1]).to_owned();
        if sep.chars().any(|c| QUERY_CHARS.contains(&c)) {
            let input_arg = strip_tcl_quotes(&cmd_args[0]).to_owned();
            let uri_cmd = input_arg_uri(&input_arg, ssa_versions, ctx)?;
            return Some((uri_cmd, "split".to_owned(), sep));
        }
    }

    None
}

fn walk_expr(
    node: &ExprNode,
    ssa_versions: &HashMap<Symbol, Version>,
    ctx: TraceCtx<'_>,
    out: &mut Vec<ExprHit>,
    depth: u32,
) {
    // Native-stack safety net (issue #996): walks the `ExprNode` tree, one
    // native frame per level. Past the cap, stop descending — a collector
    // that returns the hits gathered so far is the safe fallback (IRULE31xx
    // hits buried deeper than the cap go unreported; never a crash).
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return;
    }
    match node {
        ExprNode::Binary { op, left, right } => {
            if let Some(hit) = check_expr_binary(*op, left, right, ssa_versions, ctx) {
                out.push(hit);
            }
            walk_expr(left, ssa_versions, ctx, out, depth + 1);
            walk_expr(right, ssa_versions, ctx, out, depth + 1);
        }
        ExprNode::Unary { operand, .. } => walk_expr(operand, ssa_versions, ctx, out, depth + 1),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            walk_expr(condition, ssa_versions, ctx, out, depth + 1);
            walk_expr(true_branch, ssa_versions, ctx, out, depth + 1);
            walk_expr(false_branch, ssa_versions, ctx, out, depth + 1);
        }
        ExprNode::Call { args, .. } => {
            for a in args {
                walk_expr(a, ssa_versions, ctx, out, depth + 1);
            }
        }
        ExprNode::Command { text, .. } => {
            // [string match …] / [string first …] / [split …] inside
            // expression conditions (e.g. `if { [string match …] }`).
            if let Some(hit) = check_expr_command(text, ssa_versions, ctx) {
                out.push(hit);
            }
        }
        ExprNode::Raw { text } => {
            // The `if`/`while`/`for` lowering currently parses condition
            // expressions with `dialect: None`, which means iRules-only
            // operators (`starts_with`, `ends_with`, `contains`,
            // `matches_glob`, `matches_regex`, `equals`) tokenise as
            // unknown words and the whole expression falls back to
            // [`ExprNode::Raw`]. Re-parse here under the iRules dialect
            // so IRULE3103 can still inspect the structured form.
            let reparsed = parse_expr(text, Some("f5-irules"));
            if !matches!(reparsed, ExprNode::Raw { .. }) {
                walk_expr(&reparsed, ssa_versions, ctx, out, depth + 1);
            }
        }
        ExprNode::Literal { .. } | ExprNode::String { .. } | ExprNode::Var { .. } => {}
    }
}

// Statement-level detection: split / string match / string first

/// Return `(input_arg, separator)` from a `split` call, or `None`.
fn extract_split_info<S: std::hash::BuildHasher>(
    stmt: &Statement,
    ssa_stmt: &SsaStatement,
    sccp_values: Option<&HashMap<ValueKey, LatticeValue, S>>,
    ssa: &SsaFunction,
) -> Option<(String, Option<String>)> {
    match stmt {
        Statement::Call { command, args, .. } if is_split_cmd(command) => {
            if args.len() < 2 {
                return None;
            }
            let sep = resolve_literal(&args[1], sccp_values, &ssa_stmt.uses, ssa);
            Some((args[0].trim().to_owned(), sep))
        }
        Statement::AssignValue { value, .. } => {
            let value = value.trim();
            let (cmd, args) = parse_command_substitution(value)?;
            if !is_split_cmd(&cmd) {
                return None;
            }
            if args.len() < 2 {
                return None;
            }
            let sep = resolve_literal(&args[1], sccp_values, &ssa_stmt.uses, ssa);
            Some((strip_tcl_quotes(&args[0]).to_owned(), sep))
        }
        _ => None,
    }
}

/// Detect `string match <pattern> $uri` or `string first <needle> $uri`.
/// Returns `(sub_command, pattern_or_needle, input_arg)` or `None`.
fn extract_string_match_info<S: std::hash::BuildHasher>(
    stmt: &Statement,
    ssa_stmt: &SsaStatement,
    sccp_values: Option<&HashMap<ValueKey, LatticeValue, S>>,
    ssa: &SsaFunction,
) -> Option<(&'static str, Option<String>, String)> {
    fn from_args<S: std::hash::BuildHasher>(
        cmd: &str,
        args: &[String],
        uses: &HashMap<Symbol, Version>,
        sccp_values: Option<&HashMap<ValueKey, LatticeValue, S>>,
        ssa: &SsaFunction,
    ) -> Option<(&'static str, Option<String>, String)> {
        if !is_string_cmd(cmd) || args.is_empty() {
            return None;
        }
        let sub = args[0].as_str();
        if sub == "match" {
            // string match ?-nocase? pattern string
            let mut remaining: &[String] = &args[1..];
            if remaining.first().is_some_and(|w| w == "-nocase") {
                remaining = &remaining[1..];
            }
            if remaining.len() < 2 {
                return None;
            }
            let pattern = resolve_literal(&remaining[0], sccp_values, uses, ssa);
            return Some((
                "string match",
                pattern,
                strip_tcl_quotes(&remaining[1]).to_owned(),
            ));
        }
        if sub == "first" {
            // string first needleString haystackString ?startIndex?
            if args.len() < 3 {
                return None;
            }
            let needle = resolve_literal(&args[1], sccp_values, uses, ssa);
            return Some((
                "string first",
                needle,
                strip_tcl_quotes(&args[2]).to_owned(),
            ));
        }
        None
    }

    match stmt {
        Statement::Call { command, args, .. } => {
            from_args(command, args, &ssa_stmt.uses, sccp_values, ssa)
        }
        Statement::AssignValue { value, .. } => {
            let value = value.trim();
            let (cmd, args) = parse_command_substitution(value)?;
            from_args(&cmd, &args, &ssa_stmt.uses, sccp_values, ssa)
        }
        _ => None,
    }
}

// Main entry point

/// Statement-level dispatch: emits warnings for `split` /
/// `string match` / `string first` / expression-eval calls and for
/// `set v [expr {…}]` assignments.
fn check_statement<S: std::hash::BuildHasher>(
    stmt: &Statement,
    ssa_stmt: &SsaStatement,
    sccp_values: Option<&HashMap<ValueKey, LatticeValue, S>>,
    ctx: TraceCtx<'_>,
    warnings: &mut Vec<TaintWarning>,
) {
    let stmt_span = stmt.span();

    // 1. split detection.
    if let Some((input_arg, Some(sep))) = extract_split_info(stmt, ssa_stmt, sccp_values, ctx.ssa)
        && sep.chars().any(|c| QUERY_CHARS.contains(&c))
        && let Some(uri_cmd) = arg_traces_to_uri_family(&input_arg, ssa_stmt, ctx)
    {
        warnings.push(TaintWarning {
            span: stmt_span,
            variable: String::new(),
            sink_command: "split".to_owned(),
            code: DiagCode::Irule3103,
            message: split_message(ctx.families, &uri_cmd, &sep),
            replacement: None,
            fixes: Vec::new(),
        });
        return;
    }

    // 2. string match / string first.
    if let Some((sub_cmd, Some(pattern), input_arg)) =
        extract_string_match_info(stmt, ssa_stmt, sccp_values, ctx.ssa)
        && let Some(uri_cmd) = arg_traces_to_uri_family(&input_arg, ssa_stmt, ctx)
    {
        if sub_cmd == "string match" {
            if let Some(component) = classify_glob_pattern(&pattern) {
                warnings.push(TaintWarning {
                    span: stmt_span,
                    variable: String::new(),
                    sink_command: "string match".to_owned(),
                    code: DiagCode::Irule3103,
                    message: comparison_message(ctx.families, &uri_cmd, "string match", component),
                    replacement: None,
                    fixes: Vec::new(),
                });
            }
        } else if sub_cmd == "string first" && pattern.chars().any(|c| QUERY_CHARS.contains(&c)) {
            warnings.push(TaintWarning {
                span: stmt_span,
                variable: String::new(),
                sink_command: "string first".to_owned(),
                code: DiagCode::Irule3103,
                message: string_first_message(ctx.families, &uri_cmd, &pattern),
                replacement: None,
                fixes: Vec::new(),
            });
        }
    }

    // 3. Expression-level checks in AssignExpr.
    if let Statement::AssignExpr { expr, .. } = stmt {
        let mut hits: Vec<ExprHit> = Vec::new();
        walk_expr(expr, &ssa_stmt.uses, ctx, &mut hits, 0);
        for (uri_cmd, op_name, component) in hits {
            warnings.push(TaintWarning {
                span: stmt_span,
                variable: String::new(),
                sink_command: op_name.clone(),
                code: DiagCode::Irule3103,
                message: expr_hit_message(ctx.families, &uri_cmd, &op_name, &component),
                replacement: None,
                fixes: Vec::new(),
            });
        }
    }
}

/// Branch-condition expression walk: `if { [HTTP::uri] starts_with … }`.
fn check_branch_terminator(
    block: &crate::cfg::Block,
    ssa_block: &crate::ssa::SsaBlock,
    ctx: TraceCtx<'_>,
    warnings: &mut Vec<TaintWarning>,
) {
    let Some(Terminator::Branch {
        condition,
        span: Some(span),
        ..
    }) = &block.terminator
    else {
        return;
    };

    let mut hits: Vec<ExprHit> = Vec::new();
    walk_expr(condition, &ssa_block.exit_versions, ctx, &mut hits, 0);
    for (uri_cmd, op_name, component) in hits {
        warnings.push(TaintWarning {
            span: *span,
            variable: String::new(),
            sink_command: op_name.clone(),
            code: DiagCode::Irule3103,
            message: expr_hit_message(ctx.families, &uri_cmd, &op_name, &component),
            replacement: None,
            fixes: Vec::new(),
        });
    }
}

/// Find IRULE3103 warnings: patterns where `*::uri` should be `*::path`
/// or `*::query`.
///
/// Dialect-gated: returns an empty vector unless `dialect` is
/// `"f5-irules"` / `"irules"`. Also no-ops when the registry has no
/// `*::uri` family (i.e. no `HTTP::path` / `HTTP::query` siblings).
#[must_use]
pub fn find_uri_split_suggestions<S, B>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    sccp_values: Option<&HashMap<ValueKey, LatticeValue, S>>,
    executable_blocks: &HashSet<BlockId, B>,
    registry: &CommandRegistry,
    dialect: Option<&str>,
) -> Vec<TaintWarning>
where
    S: std::hash::BuildHasher,
    B: std::hash::BuildHasher,
{
    if !is_irules_dialect(dialect) {
        return Vec::new();
    }
    let families = uri_families(registry);
    if families.is_empty() {
        return Vec::new();
    }

    let def_sites = build_def_site_map(ssa);
    let phi_index = build_phi_index(ssa);
    let ctx = TraceCtx {
        cfg,
        ssa,
        families: &families,
        def_sites: &def_sites,
        phi_index: &phi_index,
    };

    let mut warnings: Vec<TaintWarning> = Vec::new();
    for block_id in executable_blocks {
        let Some(block) = cfg.blocks.get(block_id) else {
            continue;
        };
        let Some(ssa_block) = ssa.blocks.get(block_id) else {
            continue;
        };

        for (idx, ssa_stmt) in ssa_block.statements.iter().enumerate() {
            if idx >= block.statements.len() {
                continue;
            }
            check_statement(
                &block.statements[idx],
                ssa_stmt,
                sccp_values,
                ctx,
                &mut warnings,
            );
        }

        check_branch_terminator(block, ssa_block, ctx, &mut warnings);
    }

    // Deduplicate warnings that share the same span, sink, and message.
    let mut seen: HashSet<(Span, String, String)> = HashSet::new();
    let mut deduped: Vec<TaintWarning> = Vec::with_capacity(warnings.len());
    for w in warnings {
        let key = (w.span, w.sink_command.clone(), w.message.clone());
        if seen.insert(key) {
            deduped.push(w);
        }
    }
    deduped
}

// Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;

    fn registry() -> CommandRegistry {
        let mut r = CommandRegistry::build_default();
        r.load_irules();
        r
    }

    /// Run the IRULE3103 pass over `source` under the iRules dialect.
    fn warnings_for(source: &str) -> Vec<TaintWarning> {
        warnings_for_dialect(source, Some("f5-irules"))
    }

    /// Regression coverage for issue #996: `walk_expr` recurses once per
    /// `ExprNode` level with no depth cap before this fix. A tree built
    /// directly is unbounded (the Pratt parser caps its own output at 256)
    /// and empirically overflowed the native stack (SIGABRT) in the low
    /// thousands of levels on a 2 MiB thread. 3000 is past that crash range
    /// and past `MAX_EXPR_NODE_DEPTH` (256); the assertion is that it returns
    /// at all.
    #[test]
    fn deeply_nested_walk_expr_survives() {
        let r = registry();
        let cu = CompilationUnit::build_for("set v 5", &r, false);
        let fu = cu.function("::top").unwrap();
        let families = uri_families(&r);
        let def_sites: HashMap<ValueKey, (BlockId, usize)> = HashMap::new();
        let phi_index: HashMap<ValueKey, Phi> = HashMap::new();
        let ctx = TraceCtx {
            cfg: &fu.cfg,
            ssa: &fu.ssa,
            families: &families,
            def_sites: &def_sites,
            phi_index: &phi_index,
        };
        let uses: HashMap<Symbol, u32> = HashMap::new();
        let mut node = ExprNode::Var {
            text: "$v".into(),
            name: "v".into(),
            start: 0,
            end: 2,
        };
        for _ in 0..3000 {
            node = ExprNode::Unary {
                op: crate::expr_ast::UnaryOp::Not,
                operand: Box::new(node),
            };
        }
        let mut hits = Vec::new();
        walk_expr(&node, &uses, ctx, &mut hits, 0);
    }

    fn warnings_for_dialect(source: &str, dialect: Option<&str>) -> Vec<TaintWarning> {
        let r = registry();
        let cu = CompilationUnit::build_for(source, &r, false);
        let mut out: Vec<TaintWarning> = Vec::new();
        for fu in cu.analysable_functions() {
            out.extend(find_uri_split_suggestions(
                &fu.cfg,
                &fu.ssa,
                Some(&fu.sccp.values),
                &fu.sccp.executable_blocks,
                &r,
                dialect,
            ));
        }
        out
    }

    // registry-derived URI families

    #[test]
    fn uri_families_includes_http() {
        let r = registry();
        let families = uri_families(&r);
        let http = families.get("HTTP::uri").expect("HTTP::uri family");
        assert_eq!(http.0.as_deref(), Some("HTTP::path"));
        assert_eq!(http.1.as_deref(), Some("HTTP::query"));
    }

    #[test]
    fn uri_families_empty_under_tcl_dialect() {
        // Plain Tcl registry — no iRules commands loaded.
        let r = CommandRegistry::build_default();
        assert!(uri_families(&r).is_empty());
    }

    // split detection

    #[test]
    fn split_uri_on_question_mark_fires() {
        let ws = warnings_for(
            r#"set uri [HTTP::uri]
set parts [split $uri "?"]"#,
        );
        assert_eq!(ws.len(), 1, "expected one IRULE3103, got {ws:?}");
        assert_eq!(ws[0].code, DiagCode::Irule3103);
        assert!(ws[0].message.contains("HTTP::path"));
        assert!(ws[0].message.contains("HTTP::query"));
    }

    /// Regression for the AssignValue-side namespace-qualified-split
    /// gap: `[::split …]` inside a `set`
    /// must fire IRULE3103 the same way `[split …]` does. The Call
    /// branch already accepted both forms; the `AssignValue` branch
    /// wrongly required the bare name.
    #[test]
    fn split_namespace_qualified_in_command_substitution_fires() {
        let ws = warnings_for(
            r#"set uri [HTTP::uri]
set parts [::split $uri "?"]"#,
        );
        assert_eq!(ws.len(), 1, "expected one IRULE3103, got {ws:?}");
        assert_eq!(ws[0].code, DiagCode::Irule3103);
    }

    /// Sibling regression: `[::string match …]` inside a `set` must
    /// also fire. Same gap shape as the split case.
    #[test]
    fn string_match_namespace_qualified_in_command_substitution_fires() {
        let ws = warnings_for(
            r#"set uri [HTTP::uri]
set m [::string match "/api/*" $uri]"#,
        );
        assert_eq!(ws.len(), 1, "expected one IRULE3103, got {ws:?}");
        assert_eq!(ws[0].code, DiagCode::Irule3103);
        assert!(ws[0].message.contains("HTTP::path"));
    }

    #[test]
    fn conditional_uri_merged_with_livein_does_not_fire() {
        // `uri` is `[HTTP::uri]` only on one branch; on the other it keeps its
        // parameter (live-in, version-0) value. The phi merge is therefore NOT
        // provably a single URI getter, so IRULE3103 must NOT fire — dropping
        // the version-0 operand would mis-attribute it to HTTP::uri (issue 150).
        let ws = warnings_for(
            r#"proc handle {uri flag} {
    if {$flag} {
        set uri [HTTP::uri]
    }
    set parts [split $uri "?"]
}"#,
        );
        assert!(
            ws.is_empty(),
            "no IRULE3103 for a live-in-merged value, got {ws:?}"
        );
    }

    #[test]
    fn split_uri_on_ampersand_fires() {
        let ws = warnings_for(
            r#"set uri [HTTP::uri]
set parts [split $uri "&"]"#,
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::query"));
    }

    #[test]
    fn split_uri_on_question_and_ampersand_fires() {
        let ws = warnings_for(
            r#"set uri [HTTP::uri]
set parts [split $uri "?&"]"#,
        );
        assert_eq!(ws.len(), 1);
        assert!(ws[0].message.contains("HTTP::path"));
        assert!(ws[0].message.contains("HTTP::query"));
    }

    #[test]
    fn split_inline_command_substitution_fires() {
        let ws = warnings_for(r#"set parts [split [HTTP::uri] "?"]"#);
        assert_eq!(ws.len(), 1);
    }

    #[test]
    fn split_non_uri_clean() {
        let ws = warnings_for(
            r#"set x "foo?bar"
set parts [split $x "?"]"#,
        );
        assert!(ws.is_empty(), "expected clean, got {ws:?}");
    }

    #[test]
    fn split_http_path_clean() {
        let ws = warnings_for(
            r#"set p [HTTP::path]
set parts [split $p "?"]"#,
        );
        assert!(ws.is_empty(), "expected clean, got {ws:?}");
    }

    #[test]
    fn split_uri_on_slash_clean() {
        let ws = warnings_for(
            r#"set uri [HTTP::uri]
set parts [split $uri "/"]"#,
        );
        assert!(ws.is_empty(), "expected clean, got {ws:?}");
    }

    #[test]
    fn split_through_copy_propagation_fires() {
        let ws = warnings_for(
            r#"set uri [HTTP::uri]
set copy $uri
set parts [split $copy "?"]"#,
        );
        assert_eq!(ws.len(), 1);
    }

    #[test]
    fn split_uri_setter_form_clean() {
        // `HTTP::uri /new` is the setter form, not a getter origin.
        let ws = warnings_for(
            r#"HTTP::uri "/new"
set parts [split [HTTP::uri /path] "?"]"#,
        );
        assert!(ws.is_empty(), "expected clean, got {ws:?}");
    }

    #[test]
    fn split_under_tcl_dialect_clean() {
        let ws = warnings_for_dialect(
            r#"set x "foo"
set parts [split $x "?"]"#,
            Some("tcl"),
        );
        assert!(ws.is_empty());
    }

    #[test]
    fn split_under_no_dialect_clean() {
        let ws = warnings_for_dialect(
            r#"set uri [HTTP::uri]
set parts [split $uri "?"]"#,
            None,
        );
        assert!(ws.is_empty());
    }

    // expression-operator detection

    #[test]
    fn starts_with_path_pattern_fires_inline() {
        let ws = warnings_for(r#"if { [HTTP::uri] starts_with "/api" } { log local0. x }"#);
        assert_eq!(ws.len(), 1, "got {ws:?}");
        assert!(ws[0].message.contains("HTTP::path"));
        assert!(ws[0].message.contains("starts_with"));
    }

    #[test]
    fn starts_with_path_pattern_fires_via_variable() {
        let ws = warnings_for(
            r#"set uri [HTTP::uri]
if { $uri starts_with "/api" } { log local0. x }"#,
        );
        assert_eq!(ws.len(), 1, "got {ws:?}");
        assert!(ws[0].message.contains("HTTP::path"));
    }

    #[test]
    fn ends_with_extension_fires() {
        let ws = warnings_for(r#"if { [HTTP::uri] ends_with ".html" } { log local0. x }"#);
        assert_eq!(ws.len(), 1, "got {ws:?}");
        assert!(ws[0].message.contains("HTTP::path"));
    }

    #[test]
    fn contains_query_param_fires() {
        let ws = warnings_for(r#"if { [HTTP::uri] contains "&key=" } { log local0. x }"#);
        assert_eq!(ws.len(), 1, "got {ws:?}");
        assert!(ws[0].message.contains("HTTP::query"));
    }

    #[test]
    fn contains_equals_query_fires() {
        let ws = warnings_for(r#"if { [HTTP::uri] contains "user=test" } { log local0. x }"#);
        assert_eq!(ws.len(), 1, "got {ws:?}");
        assert!(ws[0].message.contains("HTTP::query"));
    }

    #[test]
    fn matches_glob_path_fires() {
        let ws = warnings_for(r#"if { [HTTP::uri] matches_glob "/api/*" } { log local0. x }"#);
        assert_eq!(ws.len(), 1, "got {ws:?}");
        assert!(ws[0].message.contains("HTTP::path"));
    }

    #[test]
    fn matches_glob_query_fires() {
        let ws = warnings_for(r#"if { [HTTP::uri] matches_glob "*&key=*" } { log local0. x }"#);
        assert_eq!(ws.len(), 1, "got {ws:?}");
        assert!(ws[0].message.contains("HTTP::query"));
    }

    #[test]
    fn non_uri_expression_clean() {
        let ws = warnings_for(
            r#"set p [HTTP::path]
if { $p starts_with "/api" } { log local0. x }"#,
        );
        assert!(ws.is_empty(), "got {ws:?}");
    }

    #[test]
    fn ambiguous_operand_clean() {
        let ws = warnings_for(r#"if { [HTTP::uri] contains "something" } { log local0. x }"#);
        assert!(ws.is_empty(), "got {ws:?}");
    }

    // string match / string first

    #[test]
    fn string_match_path_pattern_fires() {
        let ws = warnings_for(
            r#"set uri [HTTP::uri]
set m [string match "/api/*" $uri]"#,
        );
        assert_eq!(ws.len(), 1, "got {ws:?}");
        assert!(ws[0].message.contains("HTTP::path"));
    }

    #[test]
    fn string_match_query_pattern_fires() {
        let ws = warnings_for(
            r#"set uri [HTTP::uri]
set m [string match "*&key=*" $uri]"#,
        );
        assert_eq!(ws.len(), 1, "got {ws:?}");
        assert!(ws[0].message.contains("HTTP::query"));
    }

    #[test]
    fn string_first_question_mark_fires() {
        let ws = warnings_for(
            r#"set uri [HTTP::uri]
set pos [string first "?" $uri]"#,
        );
        assert_eq!(ws.len(), 1, "got {ws:?}");
        let msg = &ws[0].message;
        assert!(
            msg.contains("HTTP::path") || msg.contains("HTTP::query"),
            "expected HTTP::path/HTTP::query in {msg:?}",
        );
    }

    #[test]
    fn string_match_non_uri_clean() {
        let ws = warnings_for(
            r#"set p [HTTP::path]
set m [string match "/api/*" $p]"#,
        );
        assert!(ws.is_empty(), "got {ws:?}");
    }

    #[test]
    fn string_match_ambiguous_pattern_clean() {
        let ws = warnings_for(
            r#"set uri [HTTP::uri]
set m [string match "*something*" $uri]"#,
        );
        assert!(ws.is_empty(), "got {ws:?}");
    }

    #[test]
    fn string_match_in_if_condition_fires() {
        let ws = warnings_for(
            r#"set uri [HTTP::uri]
if { [string match "/static/*" $uri] } { log local0. x }"#,
        );
        assert_eq!(ws.len(), 1, "got {ws:?}");
        assert!(ws[0].message.contains("HTTP::path"));
    }

    #[test]
    fn string_first_in_if_condition_fires() {
        let ws = warnings_for(
            r#"set uri [HTTP::uri]
if { [string first "?" $uri] >= 0 } { log local0. x }"#,
        );
        assert_eq!(ws.len(), 1, "got {ws:?}");
    }

    // glob / regex classifier edge cases

    #[test]
    fn glob_question_mark_is_wildcard_not_query() {
        let ws = warnings_for(
            r#"set uri [HTTP::uri]
set m [string match "/api/??" $uri]"#,
        );
        assert_eq!(ws.len(), 1, "got {ws:?}");
        assert!(ws[0].message.contains("HTTP::path"));
    }

    // pattern classifier unit tests

    #[test]
    fn classify_glob_pattern_recognises_path() {
        assert_eq!(classify_glob_pattern("/api/*"), Some("path"));
        assert_eq!(classify_glob_pattern("*/static/*"), Some("path"));
    }

    #[test]
    fn classify_glob_pattern_recognises_query() {
        assert_eq!(classify_glob_pattern("*key=*"), Some("query"));
        assert_eq!(classify_glob_pattern("*&foo*"), Some("query"));
    }

    #[test]
    fn classify_glob_pattern_question_mark_is_ambiguous() {
        // ? is a glob wildcard, not a query separator.
        assert_eq!(classify_glob_pattern("foo?bar"), None);
    }

    #[test]
    fn classify_regex_pattern_handles_anchors() {
        assert_eq!(classify_regex_pattern("^/api/"), Some("path"));
        assert_eq!(classify_regex_pattern(r"\\Akey="), Some("query"));
        assert_eq!(classify_regex_pattern(r"foo\?bar"), Some("query"));
    }

    #[test]
    fn is_path_like_basics() {
        assert!(is_path_like("/foo"));
        assert!(is_path_like(".html"));
        assert!(!is_path_like("?key=v"));
        assert!(!is_path_like("&key=v"));
        assert!(!is_path_like("k=v"));
    }

    #[test]
    fn is_query_like_basics() {
        assert!(is_query_like("?key=v"));
        assert!(is_query_like("&key=v"));
        assert!(is_query_like("k=v"));
        assert!(!is_query_like("/foo"));
    }

    // deduplication

    #[test]
    fn duplicate_hits_in_compound_condition_are_deduped() {
        // A boolean expression that mentions [HTTP::uri] twice with the
        // same operator/operand: only one warning should fire.
        let ws = warnings_for(
            r#"if { [HTTP::uri] starts_with "/api" || [HTTP::uri] starts_with "/api" } { log local0. x }"#,
        );
        assert_eq!(ws.len(), 1, "expected dedup, got {ws:?}");
    }
}
