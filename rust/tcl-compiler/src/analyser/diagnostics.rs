//! Diagnostic-emission orchestrator.
//!
//! Three top-level methods:
//!
//! - [`Analyser::emit_variable_usage_diagnostics`] — a
//!   no-op hook for scope-tree consumers (W211 is emitted by the
//!   SSA-based pass instead).
//! - [`Analyser::emit_cfg_ssa_diagnostics`] — main entry; builds
//!   a [`crate::compilation_unit::CompilationUnit`] on demand, walks the top-level
//!   function and every procedure, dispatches per-function
//!   diagnostics, and runs the cross-function post-passes
//!   (var-as-command, interpolated-command resolution).
//! - [`Analyser::emit_cfg_ssa_diagnostics_for_function`] —
//!   per-function dispatcher; calls each emitter in
//!   declaration order.
//!
//! Two utility passes round things out:
//!
//! - [`Analyser::dedupe_diagnostics`] — drop exact duplicates
//!   plus the line-based pairs (E002 swallowed by E101 on the
//!   same line; W122 swallowed by W124 on the same line).
//! - [`Analyser::apply_disabled_diagnostics`] — filter out
//!   codes the caller asked to silence.
//!
//! The per-function dispatcher wires up the following emitters:
//!
//! - Variable lifecycle: W220 (dead store), W211 (unused
//!   variable), W214 (unused parameter), W210 (read-before-set),
//!   W213 (unset on possibly-undef), and H300 (paste error).
//!   W210 / W213 are gated on procs only.
//! - Var-as-command: **W307** (non-literal command name) and
//!   **W308** (unknown method on object) both emit via the
//!   cross-function post-pass. W308 uses
//!   ``ClassHierarchy::method_target`` for MRO-aware method
//!   resolution, with all the suppression paths wired (inherited
//!   ``unknown`` handler, external superclass, ``oo::objdefine``
//!   per-instance methods).
//! - Unknown commands: **W123** is wired via the cross-function
//!   post-pass; ``command_invocations`` are recorded for every
//!   command head during the walk dispatch.
//! - Branches and channels: I230 / I231 (constant branch /
//!   switch-arm) and W126 (channel argument validation) all wired
//!   through the per-function dispatcher. Info-severity diagnostics
//!   map to ``Severity::Hint`` (there is no Info variant here).
//! - IP literals: W124 (invalid IP address literal) — IPv4 octet
//!   validation (over-255 → Error, leading-zero → Warning) and
//!   IPv6 parsing via ``std::net::Ipv6Addr``. Anchors at the SSA
//!   def site; seen-offsets dedup avoids duplicates across SSA
//!   versions.

use std::collections::{HashMap, HashSet};

use rustc_hash::{FxHashMap, FxHashSet};
use tcl_lexer::SourceMap;

use helpers::{
    UndefSuppression, block_dominated_by, build_phi_undef_index, build_undef_suppression,
    collect_defined_vars, collect_existence_guards, globals_written_by_procs, is_ident_continue,
    is_word_byte, phi_can_undef, source_slice,
};

use super::state::Analyser;
use super::types::Severity;
use crate::expr_ast::{ExprNode, UnaryOp};

// Re-export the sibling analyser modules the family submodules reference by
// relative path (`super::types::Diagnostic`, `super::utils::…`, …) so those
// references resolve from `analyser::diagnostics::<family>`.
pub(super) use super::{confusables_table, dispatch, handlers, state, types, utils};

// `find_dotted_quads` is shared between the subnet-mask (usage) and
// invalid-IP (dataflow) checks; re-export it from the diagnostics subtree so
// the in-root caller and the `tests` submodule both resolve it.
pub(in crate::analyser::diagnostics) use helpers::find_dotted_quads;

// Re-export the family helpers exercised by this module's unit tests so the
// `tests` submodule reaches them through its `use super::*`.
#[cfg(test)]
pub(in crate::analyser::diagnostics) use security::has_redos_shape;
#[cfg(test)]
pub(in crate::analyser::diagnostics) use usage::{
    first_nested_expr, is_benign_unicode, is_safe_literal, is_safe_literal_expr,
    is_valid_subnet_mask, looks_like_subnet_mask, name_arg_indices, nearest_valid_mask,
};
#[cfg(test)]
pub(in crate::analyser::diagnostics) use validity::contains_gated_word;

mod helpers;
mod security;
mod usage;
mod validity;

/// Collect the bracketed text of every `[…]` command-substitution node in
/// an `expr` AST (recursing operands but stopping at the substitution
/// boundary). Used to recover
/// variable reads hidden inside `if`/`while` conditions and `expr` values.
fn collect_expr_command_texts(node: &ExprNode, out: &mut Vec<String>) {
    match node {
        ExprNode::Command { text, .. } => out.push(text.clone()),
        ExprNode::Binary { left, right, .. } => {
            collect_expr_command_texts(left, out);
            collect_expr_command_texts(right, out);
        }
        ExprNode::Unary { operand, .. } => collect_expr_command_texts(operand, out),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            collect_expr_command_texts(condition, out);
            collect_expr_command_texts(true_branch, out);
            collect_expr_command_texts(false_branch, out);
        }
        ExprNode::Call { args, .. } => {
            for arg in args {
                collect_expr_command_texts(arg, out);
            }
        }
        ExprNode::Literal { .. }
        | ExprNode::String { .. }
        | ExprNode::Var { .. }
        | ExprNode::Raw { .. } => {}
    }
}

/// Parse a namespaced-ensemble dispatch head `${prefix}::tail` from the source
/// slice at `span`, returning `(prefix_var_name, tail)`.  Returns `None` when
/// the head isn't this shape.
///
/// Only the **braced** form composes a command path.  A bare `$prefix::tail`
/// is lexed by Tcl as a *single* variable named `prefix::tail` (the runtime
/// reads that variable — it is not `$prefix` followed by a literal `::tail`),
/// so it must NOT be treated as ensemble dispatch.  This only matters after
/// a `${…}` closing brace — the bare VAR token already swallows the `::tail`,
/// so the character after it is never `::`.
fn parse_namespaced_ensemble(source: &str, span: tcl_lexer::Span) -> Option<(String, String)> {
    let start = span.start() as usize;
    let end = (span.end() as usize).min(source.len());
    if start >= end {
        return None;
    }
    let head = &source[start..end];
    let braced = head.strip_prefix("${")?;
    let close = braced.find('}')?;
    let (prefix, after) = (&braced[..close], &braced[close + 1..]);
    let tail = after.strip_prefix("::")?;
    // Both prefix and tail must be non-empty; a `${arr(key)}` array element is
    // not an ensemble prefix.
    if prefix.is_empty() || tail.is_empty() || prefix.contains('(') {
        return None;
    }
    Some((prefix.to_string(), tail.to_string()))
}

/// Harvest `array set arr {k1 v1 k2 v2 …}` literal element values into the
/// constset map keyed by `arr(key)`, so the W307 callback-array suppression
/// can check the *actual* value of `$arr(-command)` against the known-command
/// set.  Without this, the dash-prefixed / callback-suffixed array-key
/// heuristic fires even when SCCP-equivalent literal evidence proves the value
/// is (or isn't) a command.
fn harvest_array_set_constants(
    cu: &crate::compilation_unit::CompilationUnit,
    out: &mut HashMap<String, HashSet<String>>,
) {
    use crate::ir::Statement;
    let units = std::iter::once(&cu.top_level).chain(cu.procedures.values());
    for fu in units {
        for block in fu.cfg.blocks.values() {
            for stmt in &block.statements {
                let (Statement::Call { command, args, .. }
                | Statement::Barrier { command, args, .. }) = stmt
                else {
                    continue;
                };
                let is_array =
                    command == "array" || stmt.canonical_command_or_source() == "::array";
                if !is_array || args.first().map(String::as_str) != Some("set") || args.len() < 3 {
                    continue;
                }
                let arr_name = &args[1];
                let items = crate::tcl_expr_eval::split_tcl_list(&args[2]);
                if !items.len().is_multiple_of(2) {
                    continue;
                }
                for pair in items.chunks_exact(2) {
                    let elem_name = format!("{arr_name}({})", pair[0]);
                    out.entry(elem_name).or_default().insert(pair[1].clone());
                }
            }
        }
    }
}

/// Harvest `dict with d { … }` unpacked variable values: when `d` is a known
/// literal dict (via SCCP CONST at param entry — usually from call-site
/// constant propagation), the body sees each dict key as a local variable
/// bound to its value.  Register those bindings so a `$cmd hi` dispatch inside
/// the body checks `cmd`'s value against the known-command set.
fn harvest_dict_with_constants(
    cu: &crate::compilation_unit::CompilationUnit,
    out: &mut HashMap<String, HashSet<String>>,
) {
    use crate::ir::Statement;
    let units = std::iter::once(&cu.top_level).chain(cu.procedures.values());
    for fu in units {
        for block in fu.cfg.blocks.values() {
            for stmt in &block.statements {
                let (Statement::Barrier { command, args, .. }
                | Statement::Call { command, args, .. }) = stmt
                else {
                    continue;
                };
                let is_dict = command == "dict" || stmt.canonical_command_or_source() == "::dict";
                if !is_dict || args.first().map(String::as_str) != Some("with") {
                    continue;
                }
                let Some(dict_var) = args.get(1) else {
                    continue;
                };
                let dvar = crate::naming::normalise_var_name(dict_var);
                // The call-site-propagated literal lands at the param entry (v0).
                let Some(crate::analyses::LatticeValue::Const(
                    crate::analyses::ConstValue::String(dict_text),
                )) = fu.sccp.values.get(&(dvar.to_string(), 0))
                else {
                    continue;
                };
                let items = crate::tcl_expr_eval::split_tcl_list(dict_text);
                if !items.len().is_multiple_of(2) {
                    continue;
                }
                for pair in items.chunks_exact(2) {
                    out.entry(pair[0].clone())
                        .or_default()
                        .insert(pair[1].clone());
                }
            }
        }
    }
}

/// Sentinel scope key for the W307 dispatcher-suppression maps covering
/// statements outside any proc body.
const W307_TOP_SCOPE: &str = "::top";

/// The variable named by a single `$var` / `${var}` substitution, or `None`.
///
/// The text must be exactly one bare or braced variable reference whose name
/// is made of word / namespace characters.  Anything else (literals, command
/// subs, composite words) yields `None`.
fn extract_dollar_var(value: &str) -> Option<String> {
    let v = value.trim();
    let rest = v.strip_prefix('$')?;
    let is_name = |s: &str| {
        !s.is_empty()
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b':')
    };
    if let Some(inner) = rest.strip_prefix('{').and_then(|r| r.strip_suffix('}')) {
        // Braced `${name}` — reject nested braces.
        if !inner.contains('{') && is_name(inner) {
            return Some(inner.to_string());
        }
        return None;
    }
    is_name(rest).then(|| rest.to_string())
}

/// The variable returned by a proc's **last** `return $var`, or `None`.
///
/// Walks every block's statements and terminator (returns can lower to either
/// an `IRReturn` statement or a `Return` terminator) and keeps the last whose
/// value is a single `$var`.  Used by the object-returning-proc inference:
/// a proc returning `$X` where `X` was assigned from a factory is itself an
/// object factory.
fn last_return_var_of(cfg: &crate::cfg::Function) -> Option<String> {
    use crate::cfg::Terminator;
    use crate::ir::Statement;
    let mut last = None;
    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            if let Statement::Return { value: Some(v), .. } = stmt
                && let Some(name) = extract_dollar_var(v)
            {
                last = Some(name);
            }
        }
        if let Some(Terminator::Return { value: Some(v), .. }) = &block.terminator
            && let Some(name) = extract_dollar_var(v)
        {
            last = Some(name);
        }
    }
    last
}

/// Every return value (statement + terminator) a proc body can produce, as raw
/// text.  Seeds the object-returning-proc inference.
fn return_values_of(cfg: &crate::cfg::Function) -> Vec<String> {
    use crate::cfg::Terminator;
    use crate::ir::Statement;
    let mut out = Vec::new();
    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            if let Statement::Return { value: Some(v), .. } = stmt {
                out.push(v.clone());
            }
        }
        if let Some(Terminator::Return { value: Some(v), .. }) = &block.terminator {
            out.push(v.clone());
        }
    }
    out
}

// IP / ReDoS leaf scanners (regex-free)

/// Find every IPv6 *candidate* substring — `\b[hex]{1,4}(:[hex]{0,4}){2,7}\b`
/// — in `text` (the caller validates each via `Ipv6Addr::from_str`).
/// Replaces the regex; each candidate begins at a word boundary, has a
/// 1-4 hex-digit first group, 2-7 following `:`-groups (each 0-4 hex),
/// and ends on a hex digit at a trailing word boundary.
fn find_ipv6_candidates(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let boundary_before = i == 0 || !is_word_byte(bytes[i - 1]);
        if boundary_before
            && bytes[i].is_ascii_hexdigit()
            && let Some(end) = match_ipv6_candidate(bytes, i)
        {
            out.push(&text[i..end]);
            i = end;
            continue;
        }
        i += 1;
    }
    out
}

/// Read up to `max` contiguous hex-digit bytes from `start`, returning
/// the count.
fn hex_run_len(bytes: &[u8], start: usize, max: usize) -> usize {
    let mut k = 0;
    while k < max && start + k < bytes.len() && bytes[start + k].is_ascii_hexdigit() {
        k += 1;
    }
    k
}

/// Match an IPv6 candidate starting at `start`, returning the end offset
/// of the longest `hex(:hex?){2,7}` run that ends on a hex digit and is
/// followed by a word boundary, or `None`.
fn match_ipv6_candidate(bytes: &[u8], start: usize) -> Option<usize> {
    let first = hex_run_len(bytes, start, 4);
    if first == 0 {
        return None;
    }
    let mut pos = start + first;
    let mut groups = 0usize;
    let mut best: Option<usize> = None;
    while groups < 7 && bytes.get(pos) == Some(&b':') {
        let after_colon = pos + 1;
        let h = hex_run_len(bytes, after_colon, 4);
        pos = after_colon + h;
        groups += 1;
        // A valid `\b`-terminated end: ≥2 groups, ends on a hex digit,
        // and is followed by a non-word byte (or end of input).
        if groups >= 2 && h >= 1 && (pos >= bytes.len() || !is_word_byte(bytes[pos])) {
            best = Some(pos);
        }
    }
    best
}

/// Find a defined variable that differs from `variable` only in case.
/// Returns the lexicographically smallest other-cased variant —
/// deterministic across runs.
fn find_case_mismatch<'a>(variable: &str, defined_vars: &'a HashSet<String>) -> Option<&'a str> {
    let lower = variable.to_lowercase();
    let mut matches: Vec<&str> = defined_vars
        .iter()
        .filter(|n| n.as_str() != variable && n.to_lowercase() == lower)
        .map(String::as_str)
        .collect();
    matches.sort_unstable();
    matches.into_iter().next()
}

/// Variables this statement queries *only for
/// existence* (`info exists X` / `array exists X`, whether a bare call
/// or a `[...]` command substitution inside an assignment / argument).
/// Such a reference is not a value read, so it must not raise W210.
fn existence_query_vars(stmt: &crate::ir::Statement) -> Vec<String> {
    use crate::expr_ast::existence_query_in_text;
    use crate::ir::Statement;
    let mut out = Vec::new();
    // Bare-call form: `info exists X` / `array exists X`.
    if let Statement::Call { command, args, .. } = stmt
        && matches!(command.as_str(), "info" | "array")
        && args.first().map(String::as_str) == Some("exists")
        && let Some(v) = args.get(1)
    {
        out.push(v.clone());
    }
    // Command-substitution form: `set y [info exists X]`,
    // `puts [array exists X]`, etc.
    let texts: &[String] = match stmt {
        Statement::AssignValue { value, .. } => std::slice::from_ref(value),
        Statement::Call { args, .. } => args,
        _ => &[],
    };
    for t in texts {
        if let Some(v) = existence_query_in_text(t.trim()) {
            out.push(v);
        }
    }
    out
}

/// True when a read of `var` at `use_block` is exempt
/// from W210 because it is the existence-query word itself, or because
/// it sits in a region guarded by an enclosing `[info exists var]`.
fn existence_exempt(
    stmt_opt: Option<&crate::ir::Statement>,
    var: &str,
    exists_guards: &[(String, String)],
    ssa: &crate::ssa::SsaFunction,
    use_block: &str,
) -> bool {
    if let Some(stmt) = stmt_opt
        && existence_query_vars(stmt).iter().any(|q| q == var)
    {
        return true;
    }
    exists_guards
        .iter()
        .any(|(gv, gblk)| gv == var && block_dominated_by(ssa, use_block, gblk))
}

/// True when a read of `var` at this use-site statement is in fact a safe
/// self-initialisation, not a read-before-set: a `safe_on_uninit` call (e.g.
/// `lappend`/`dict set`/`append`) that defines `var`, or an `incr` of its own
/// target (which initialises an unset var to 0 in Tcl 8.5+).
fn use_site_safe_initialises(stmt: Option<&crate::ir::Statement>, var: &str) -> bool {
    use crate::ir::Statement;
    match stmt {
        Some(Statement::Call {
            safe_on_uninit,
            defs,
            ..
        }) => *safe_on_uninit && defs.iter().any(|d| d == var),
        Some(Statement::Incr { name, .. }) => crate::naming::normalise_var_name(name) == var,
        _ => false,
    }
}

/// The namespace of a fully-qualified name: everything up to the last `::`,
/// or `::` for a top-level name.
fn namespace_of(qualified_name: &str) -> String {
    match qualified_name.rsplit_once("::") {
        Some((ns, _)) if !ns.is_empty() => ns.to_string(),
        _ => "::".to_string(),
    }
}

/// Implicit / interpreter-provided variables that are always defined and
/// must never raise a read-before-set.
fn is_implicit_var(name: &str) -> bool {
    matches!(
        name,
        "argc"
            | "argv"
            | "argv0"
            | "auto_path"
            | "env"
            | "errorCode"
            | "errorInfo"
            | "errorResult"
            | "tcl_interactive"
            | "tcl_library"
            | "tcl_patchLevel"
            | "tcl_pkgPath"
            | "tcl_platform"
            | "tcl_precision"
            | "tcl_rcFileName"
            | "tcl_version"
            | "tcl_wordchars"
            | "tcl_nonwordchars"
            | "static"
    )
}

/// Tcl ARE metacharacters: a pattern free of these reduces to a literal
/// substring search.
const TCL_REGEX_METACHARS: &str = r"\^$.|?*+()[]{}";

/// `regexp` switches that don't change match-vs-no-match for a pure-literal
/// pattern.
fn is_regexp_literal_safe_switch(opt: &str) -> bool {
    matches!(
        opt,
        "-indices" | "-inline" | "-all" | "-line" | "-lineanchor" | "-linestop" | "-start" | "--"
    )
    // `-expanded` is handled separately (whitespace/comment-gated) by the
    // caller, so it is intentionally not listed here.
}

/// True iff `regexp PATTERN INPUT` provably returns 0.  Sound only when
/// `pat` is a pure-literal pattern (no ARE metacharacters), reducing the
/// match to substring search.  Unknown / unsafe switches bail (return
/// `false` = cannot prove no-match).
fn regexp_literal_no_match(pat: &str, inp: &str, options: &[String]) -> bool {
    if pat.chars().any(|c| TCL_REGEX_METACHARS.contains(c)) {
        return false;
    }
    let mut nocase = false;
    let mut expanded = false;
    for opt in options {
        if !opt.starts_with('-') {
            continue; // an option value (e.g. after `-start`)
        }
        if opt == "-nocase" {
            nocase = true;
            continue;
        }
        if opt == "-expanded" {
            expanded = true;
            continue;
        }
        if is_regexp_literal_safe_switch(opt) {
            continue;
        }
        return false; // unknown / unsafe switch
    }
    // `-expanded` makes Tcl ignore unescaped whitespace and `#`-comments in
    // the pattern, so a pattern containing either is NOT a plain substring
    // (`regexp -expanded {a b} {ab}` matches).  Bail in that case so the
    // no-match proof stays sound — a whitespace/comment-free literal is
    // still safe.
    if expanded && pat.chars().any(|c| c.is_whitespace() || c == '#') {
        return false;
    }
    if nocase {
        !inp.to_lowercase().contains(&pat.to_lowercase())
    } else {
        !inp.contains(pat)
    }
}

/// `Some(true)` when a `regexp` / `scan` call (`is_regexp` selects the arg
/// order) with literal pattern + input provably can't match; `Some(false)`
/// when it might match; `None` when the args can't be statically resolved
/// (dynamic substitution, too few args).
fn regexp_scan_no_match(is_regexp: bool, args: &[String]) -> Option<bool> {
    let value_opts: &[&str] = if is_regexp { &["-start"] } else { &[] };
    let pos = skip_options(args, value_opts);
    if pos + 1 >= args.len() {
        return None;
    }
    let a = &args[pos];
    let b = &args[pos + 1];
    // `regexp ?opts? PATTERN STRING …`; `scan STRING FORMAT …`.
    let (pat, inp) = if is_regexp { (a, b) } else { (b, a) };
    // Dynamic substitution markers — runtime value unknown.
    if pat.contains(['$', '[']) || inp.contains(['$', '[']) {
        return None;
    }
    if is_regexp {
        let opts: Vec<String> = args[..pos].to_vec();
        Some(regexp_literal_no_match(pat, inp, &opts))
    } else {
        Some(crate::scan_predicate::scan_provably_no_match(pat, inp))
    }
}

/// Index of the first non-option argument in `args`, skipping `-option`
/// flags and the values of options in `value_opts`.
fn skip_options(args: &[String], value_opts: &[&str]) -> usize {
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == "--" {
            i += 1;
            break;
        }
        if a.starts_with('-') {
            i += 1;
            if value_opts.contains(&a.as_str()) && i < args.len() {
                i += 1;
            }
            continue;
        }
        break;
    }
    i
}

/// External OO base classes that aren't in the per-document
/// ``ClassDef`` index but are recognised as legitimate
/// superclasses for W308 / W308-related gates.
const OO_BASE: [&str; 2] = ["oo::object", "oo::class"];

/// Extract the first single-quoted word from a diagnostic
/// message string, or `None` if the message has no quoted run.
///
/// Used by [`Analyser::resolve_interpolated_w123_diagnostics`]
/// to recover the command name from a "Unknown command 'NAME'"
/// W123 message.
fn extract_quoted_word(message: &str) -> Option<String> {
    let start = message.find('\'')?;
    let rest = &message[start + 1..];
    let end = rest.find('\'')?;
    Some(rest[..end].to_string())
}

/// Return ``true`` when ``body`` contains a ``$param`` /
/// ``${param}`` substitution.  Used as a fallback by the W214
/// (unused-parameter) emitter to suppress the warning when the
/// parameter is read inside a ``[expr {...}]`` / ``[cmd ...]``
/// substitution that the IR lowerer doesn't track as a use.
///
/// Conservative — false negatives are fine (W214 still fires
/// when the param genuinely isn't referenced), but false
/// positives would cause the over-emit this guard exists to
/// prevent.  The bare-name match enforces a non-identifier
/// boundary on each side so ``$abc`` doesn't match ``$ab``,
/// and skips the variable when it follows a ``\\`` escape.
/// True when the proc body textually references the parameter `$param` /
/// `${param}`, scanning command-by-command so a `namespace eval` body — which
/// runs in the *namespace* frame, not the caller's — does **not** falsely
/// recover a read of the caller's parameter.  Other bodies (`eval`, `if`,
/// loops) run in the caller frame, so their `$param` reads still count.
fn body_references_param(body: &str, param: &str) -> bool {
    if param.is_empty() {
        return false;
    }
    let cmds = crate::segmenter::segment_commands_with_offset_and_config(
        body,
        0,
        tcl_lexer::LexerConfig::default(),
    );
    for cmd in &cmds {
        // `namespace eval NS BODY` — the trailing body word evaluates in NS's
        // frame, so exclude it; the NS-name word (e.g. `namespace eval $x …`)
        // is still substituted in the caller frame and is scanned.
        let is_ns_eval = cmd.texts.first().map(String::as_str) == Some("namespace")
            && cmd.texts.get(1).map(String::as_str) == Some("eval");
        let skip_last = is_ns_eval && cmd.texts.len() >= 4;
        let last_idx = cmd.texts.len().saturating_sub(1);
        for (i, word) in cmd.texts.iter().enumerate() {
            if skip_last && i == last_idx {
                continue;
            }
            if word_references_param(word, param) {
                return true;
            }
        }
    }
    false
}

/// True when a single word textually references `$param` / `${param}`.  Flat
/// byte scan with identifier-boundary and `\$` escape handling.
fn word_references_param(body: &str, param: &str) -> bool {
    if param.is_empty() {
        return false;
    }
    let bytes = body.as_bytes();
    let plen = param.len();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c != b'$' {
            i += 1;
            continue;
        }
        // Skip backslash-escaped ``\$``.
        if i > 0 && bytes[i - 1] == b'\\' {
            i += 1;
            continue;
        }
        // ``${name}`` form.
        if i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            let start = i + 2;
            if start + plen <= bytes.len()
                && &bytes[start..start + plen] == param.as_bytes()
                && start + plen < bytes.len()
                && bytes[start + plen] == b'}'
            {
                return true;
            }
        } else {
            // ``$name`` form — bare identifier match.
            let start = i + 1;
            if start + plen <= bytes.len() && &bytes[start..start + plen] == param.as_bytes() {
                let after = start + plen;
                let next_ok = after >= bytes.len() || !is_ident_continue(bytes[after]);
                if next_ok {
                    return true;
                }
            }
        }
        i += 1;
    }
    false
}

impl Analyser {
    /// Scope-tree-driven variable diagnostic emitter.
    ///
    /// An empty hook: W211 (unused-variable) is emitted by the
    /// SSA-based pass in `emit_cfg_ssa_diagnostics_for_function`.
    /// The hook is preserved so future scope-tree-driven emitters
    /// (none currently planned) have a target.
    pub fn emit_variable_usage_diagnostics(&mut self) {
        // Intentionally empty — see module docstring.
    }

    /// **W128.** Flag a call to a command that was
    /// renamed or deleted earlier in the same file — it falls through to
    /// the `unknown` handler.
    ///
    /// Backed by the flow-sensitive command-binding lattice
    /// ([`crate::command_binding`]).  The lattice is seeded with every
    /// module procedure (canonically qualified) as `Proc` so a proc
    /// defined inside a `namespace eval` block — whose top-level CFG never
    /// sees the full qname — is still known, matching the optimiser's
    /// gating view.  A call fires W128 only when its resolved binding is
    /// `Opaque` *and* its name was actually perturbed somewhere in this
    /// file (`rebound_names`); a merely-undefined external command (always
    /// opaque, never rebound) does not.  A dynamic mutation collapses the
    /// lattice to the wildcard ⊤, under which every binding resolves to
    /// `Unknown` (not `Opaque`), so W128 conservatively goes quiet.
    pub(super) fn emit_w128_renamed_command(
        &mut self,
        cu: &crate::compilation_unit::CompilationUnit,
        registry: &tcl_registry::CommandRegistry,
    ) {
        use crate::command_binding::{Binding, BindingKind, analyse_command_binding};
        use crate::ir::Statement;
        use crate::naming::normalise_qualified_name as nqn;

        let cfg = &cu.top_level.cfg;
        let seed: Vec<(String, Binding)> = cu
            .ir_module
            .procedures
            .keys()
            .map(|q| {
                (
                    q.clone(),
                    Binding {
                        kind: BindingKind::Proc,
                        target: Some(q.clone()),
                    },
                )
            })
            .collect();
        let binding = analyse_command_binding(cfg, registry, &seed);
        let rebound = binding.rebound_names();
        if rebound.is_empty() {
            return;
        }
        // Reverse-postorder for deterministic diagnostic ordering.
        for block_name in cfg.reverse_postorder() {
            let Some(block) = cfg.blocks.get(&block_name) else {
                continue;
            };
            for (idx, stmt) in block.statements.iter().enumerate() {
                let Statement::Call { command, span, .. } = stmt else {
                    continue;
                };
                // The mutating commands themselves are not flagged.
                if command.is_empty() || matches!(command.as_str(), "rename" | "interp" | "proc") {
                    continue;
                }
                if binding.binding_at(&block_name, idx, command).kind != BindingKind::Opaque {
                    continue;
                }
                if !rebound.contains(&nqn(command)) {
                    continue; // never bound here → an ordinary external command
                }
                self.result.diagnostics.push(super::types::Diagnostic {
                    code: "W128".to_string(),
                    span: *span,
                    message: format!(
                        "Command '{command}' was renamed or deleted earlier in this \
file; this call falls through to the 'unknown' handler."
                    ),
                    severity: Severity::Warning,
                    fixes: Vec::new(),
                });
            }
        }
    }

    /// CFG/SSA-backed diagnostic orchestrator.
    ///
    /// Builds a
    /// [`crate::compilation_unit::CompilationUnit`] for `source`,
    /// then walks the top-level + every procedure, dispatching
    /// per-function emitters.
    pub fn emit_cfg_ssa_diagnostics(&mut self, source: &str) {
        use tcl_registry::CommandRegistry;
        use tcl_registry::prelude::DialectSet;

        let mut registry = CommandRegistry::build_default();
        if let Some(d) = DialectSet::parse(&self.dialect) {
            registry.load_dialect(d);
        }
        // Seed each proc's SCCP with caller-side parameter constants so a
        // branch on a param every caller passes the same literal folds (the
        // `if {$x}` body is provably taken under uniform `q 1` callers, so a
        // var set only there is not read-before-set).
        // Incremental seam: when the per-item path has supplied a unit whose
        // per-function lattices were memoised, consume it instead of
        // rebuilding the whole-file unit.  Equal by construction to the
        // freshly-built unit.
        if let Some(cu) = self.cu_override.take() {
            self.emit_cfg_ssa_diagnostics_with_cu(&cu, &registry);
            return;
        }
        // Own the dialect so the firewall closure below doesn't hold an
        // immutable borrow of `self` (via `self.dialect`) while it also needs
        // `&mut self` for the emission.
        let dialect_owned: Option<String> =
            (!self.dialect.is_empty()).then(|| self.dialect.clone());
        // AN-H1: firewall the lowering→CFG→SSA→interprocedural build (and the
        // emission that consumes it). A panic on adversarial input is contained
        // to "no CFG/SSA diagnostics for this document" instead of crashing the
        // whole document's diagnostics — the same conservative containment the
        // `unknown`-proc lowering path uses (`oo.rs`). (Deep-nesting stack
        // overflow is separately bounded by the lowering depth guards;
        // `catch_unwind` cannot contain a SIGABRT.)
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let dialect_opt = dialect_owned.as_deref();
            let cu = crate::compilation_unit::CompilationUnit::build_for(source, &registry, false)
                .with_interprocedural(&registry, dialect_opt);
            self.emit_cfg_ssa_diagnostics_with_cu(&cu, &registry);
        }));
    }

    /// Emit the CFG/SSA-derived diagnostics from an already-built
    /// [`crate::compilation_unit::CompilationUnit`].
    ///
    /// Split out of [`Self::emit_cfg_ssa_diagnostics`] so the incremental
    /// per-item path can supply a `CompilationUnit` whose per-function
    /// lattices were memoised, instead of rebuilding the whole-file unit on
    /// every edit.  Behaviour is identical: the whole-file entry point builds
    /// the unit exactly as before and delegates here, and every cross-function
    /// pass below reads the supplied unit unchanged.
    pub fn emit_cfg_ssa_diagnostics_with_cu(
        &mut self,
        cu: &crate::compilation_unit::CompilationUnit,
        registry: &tcl_registry::CommandRegistry,
    ) {
        // **W128.** Flag calls to commands renamed or
        // deleted earlier in the file via the flow-sensitive
        // command-binding lattice.  Independent of the CFG/SSA dead-store
        // machinery below, so run it up front against the same `cu`.
        self.emit_w128_renamed_command(cu, registry);

        // Compute the set of globals any
        // proc in this module writes to.  Top-level RBS (W210)
        // is suppressed for these variables — a helper proc may
        // populate them before the top-level read fires.
        let globals_written = globals_written_by_procs(cu);

        // **W220 call-by-name suppression.** Build the
        // interprocedural proc-index once so a caller-local passed *by
        // name* to a proc that consumes it via `upvar` (`set tag "";
        // asnPeekTag data tag type dummy`) is not flagged as a dead
        // store.  `collect_call_by_name_reads` then yields the suppressed
        // names per function, merged into the dead-store `cross_event_vars`.
        let cbn_proc_index = {
            let ia = crate::interprocedural::build_interprocedural_analysis(
                &cu.ir_module,
                registry,
                Some(self.dialect.as_str()),
            );
            crate::interprocedural::build_proc_index_from_summaries(&ia)
        };

        // pkgIndex.tcl files have ``$dir`` set by the package
        // loader before the script body runs — suppress dead-
        // store / unused-variable diagnostics for it at the
        // top-level.
        let mut top_level_cross_event_vars: HashSet<String> = if self
            .file_path
            .as_deref()
            .is_some_and(|p| p.ends_with("pkgIndex.tcl"))
        {
            HashSet::from(["dir".to_string()])
        } else {
            HashSet::new()
        };
        top_level_cross_event_vars.extend(crate::interprocedural::collect_call_by_name_reads(
            &cu.top_level.cfg,
            &cbn_proc_index,
        ));

        // Top-level first, then procedures in insertion order —
        // matches the iteration order of
        // ``CompilationUnit::functions``.
        // Iterate top-level explicitly so we can pass the IR
        // module through.
        self.emit_cfg_ssa_diagnostics_for_function_full(
            &cu.top_level,
            &cu.ir_module,
            &globals_written,
            &top_level_cross_event_vars,
        );
        self.emit_channel_diagnostics(&cu.top_level, registry);
        for (qname, fu) in &cu.procedures {
            // For ``::when::*`` procs, threaded
            // ``cross_event_defs | cross_event_imports`` from the
            // ConnectionScope so dead-store / unused-variable
            // diagnostics suppress vars that may be read in a
            // different iRule event.
            let mut cross_event_vars: HashSet<String> =
                if let Some(scope) = cu.connection_scope.as_ref() {
                    if qname.starts_with("::when::") {
                        scope
                            .cross_event_defs
                            .iter()
                            .chain(scope.cross_event_imports.iter())
                            .cloned()
                            .collect()
                    } else {
                        HashSet::new()
                    }
                } else {
                    HashSet::new()
                };
            // Suppress dead-store on caller-locals this
            // proc passes by name to an upvar callee.
            cross_event_vars.extend(crate::interprocedural::collect_call_by_name_reads(
                &fu.cfg,
                &cbn_proc_index,
            ));
            self.emit_cfg_ssa_diagnostics_for_function_full(
                fu,
                &cu.ir_module,
                &HashSet::new(),
                &cross_event_vars,
            );
            self.emit_channel_diagnostics(fu, registry);
            // IRULE4005 — racy ``static::``
            // cross-event flow.  Only fires for non-RULE_INIT
            // ``when`` procs when ``ConnectionScope::racy_static_defs``
            // is non-empty.
            if let Some(scope) = cu.connection_scope.as_ref()
                && qname.starts_with("::when::")
                && !scope.racy_static_defs.is_empty()
            {
                let event = crate::ir::when_event_name(qname);
                if event != "RULE_INIT" {
                    self.emit_racy_static_diagnostics(fu, &scope.racy_static_defs);
                }
            }
        }

        // Cross-function post-pass: resolve $var-as-command sites
        // collected during the walk.
        self.emit_var_command_diagnostics(cu, registry);

        // Suppress W123 for command-name
        // heads with partial interpolations like ``foo$suffix``
        // when ``$suffix`` resolves cleanly to a finite set of
        // known commands via SCCP.
        self.resolve_interpolated_w123_diagnostics(cu);
    }

    /// Per-function diagnostic dispatcher.
    ///
    /// Called once for the top-level
    /// script and once per procedure.  Each per-emitter call is
    /// gated on its own predicate inside the helper.
    pub fn emit_cfg_ssa_diagnostics_for_function(
        &mut self,
        function_unit: &crate::compilation_unit::FunctionUnit,
        ir_module: &crate::ir::Module,
    ) {
        self.emit_cfg_ssa_diagnostics_for_function_full(
            function_unit,
            ir_module,
            &HashSet::new(),
            &HashSet::new(),
        );
    }

    /// Per-function diagnostic dispatcher with an extra
    /// "known-defined" set passed through to RBS suppression.
    ///
    /// Same as [`Self::emit_cfg_ssa_diagnostics_for_function`]
    /// but accepts an additional set of variable names that
    /// should be treated as already-defined for the W210
    /// (read-before-set) emitter.  Used at the top-level to
    /// suppress RBS for variables that any proc in the module
    /// writes.
    pub fn emit_cfg_ssa_diagnostics_for_function_with_extra(
        &mut self,
        function_unit: &crate::compilation_unit::FunctionUnit,
        ir_module: &crate::ir::Module,
        extra_known_defined: &HashSet<String>,
    ) {
        self.emit_cfg_ssa_diagnostics_for_function_full(
            function_unit,
            ir_module,
            extra_known_defined,
            &HashSet::new(),
        );
    }

    /// Per-function diagnostic dispatcher with the full
    /// suppression context.
    ///
    /// Adds `cross_event_vars` on top of
    /// [`Self::emit_cfg_ssa_diagnostics_for_function_with_extra`].
    /// Used by the W220 IR-paths port to suppress dead-store
    /// diagnostics for variables a `::when::*` proc may carry
    /// across iRule events (`cu.connection_scope.cross_event_defs
    /// | cross_event_imports`) and for `pkgIndex.tcl` `$dir`,
    /// which the package loader assigns before the script body
    /// runs.
    pub fn emit_cfg_ssa_diagnostics_for_function_full(
        &mut self,
        function_unit: &crate::compilation_unit::FunctionUnit,
        ir_module: &crate::ir::Module,
        extra_known_defined: &HashSet<String>,
        cross_event_vars: &HashSet<String>,
    ) {
        let defined = collect_defined_vars(&function_unit.cfg);
        let scope_aliases = crate::optimiser::elimination::scan_scope_aliases(&function_unit.cfg);
        let mut textually_referenced =
            crate::optimiser::elimination::collect_textual_var_references(
                &self.source,
                &function_unit.cfg,
                function_unit.base_offset,
            );
        // A var read in another iRule event, or consumed *by name* via a
        // call-by-name upvar callee, is "used" — suppress
        // the unused-variable (W211) hint too, not just the dead store
        // (W220).
        textually_referenced.extend(cross_event_vars.iter().cloned());
        // A read-modify-write command's target buried in a substitution
        // (`lappend r [incr i $j]` reads `i`) keeps a feeding `set i 0` alive —
        // recover those name-level reads so they suppress the dead-store /
        // unused-variable hints.
        if let Some(registry) = self.registry.as_ref() {
            textually_referenced.extend(crate::optimiser::elimination::collect_rmw_hidden_reads(
                function_unit,
                registry,
            ));
        }
        let ir_proc = ir_module.procedures.get(&function_unit.name);
        self.emit_dead_store_diagnostics(function_unit, &defined, &scope_aliases, cross_event_vars);
        self.emit_unused_variable_diagnostics(
            function_unit,
            &defined,
            &scope_aliases,
            &textually_referenced,
        );
        self.emit_possible_paste_error_diagnostics(function_unit);
        // Shared read-before-set context: the SCCP-executable block set and
        // the name-level suppression (`dict with` keys, qualified-`variable`
        // alias tails, dict vars), threaded through both the version-0
        // statement/branch emitter and the `Terminator::Return` pass.
        let considered: HashSet<String> = if function_unit.sccp.executable_blocks.is_empty() {
            function_unit.ssa.blocks.keys().cloned().collect()
        } else {
            function_unit.sccp.executable_blocks.clone()
        };
        let supp = build_undef_suppression(function_unit, &considered);
        let exists_guards = collect_existence_guards(function_unit);
        let rbs_params: HashSet<&str> = ir_proc
            .map(|p| p.params.iter().map(String::as_str).collect())
            .unwrap_or_default();
        self.emit_read_before_set_diagnostics(
            function_unit,
            ir_proc,
            &defined,
            &scope_aliases,
            extra_known_defined,
            &supp,
        );
        // Phi-from-undef on `return $v` reads (the def-use builder records
        // statement + branch-condition uses but NOT `Terminator::Return`
        // values).
        self.emit_return_phi_undef_w210(
            function_unit,
            &rbs_params,
            &exists_guards,
            &scope_aliases,
            extra_known_defined,
            &defined,
            &considered,
            &supp,
        );
        // W210 on reads of a provably-no-match regexp / scan output var.
        self.emit_provably_unset_w210(function_unit, &considered, &defined);
        self.emit_constant_branch_diagnostics(function_unit);
        self.emit_existence_constant_branch_diagnostics(function_unit, ir_proc);
        self.emit_invalid_ip_diagnostics(function_unit);
        self.emit_w233_divide_by_zero(function_unit);
        self.emit_interval_bounds_diagnostics(function_unit);
        if let Some(ir_proc) = ir_proc {
            self.emit_unused_param_diagnostics(function_unit, ir_proc);
        }
    }

    /// Statements whose dead-store **W220** hint should be **suppressed**
    /// because their array-element / dict-path def place is observed by some
    /// read in the function.
    ///
    /// Name-level SSA folds `a(k)` / `a(j)` / `$a` to the base name `a`, so a
    /// later `set a(j) 2` looks like it overwrites `set a(k) 1` before any read
    /// — a false dead store when `a(k)` is in fact read.  Delegates to the
    /// shared [`crate::place_bridge::element_writes_observed_by_reads`] (also
    /// used by the optimiser's O109), which resolves each element write to a
    /// [`Place`](crate::place::Place) and consults the over-approximating
    /// [`overlap`](crate::place::overlap).  Scalars keep the precise name-level
    /// verdict (they don't fold), so a genuine `set x 1; set x 2; puts $x` dead
    /// store still fires.  Empty when no registry is bound (e.g. the bare
    /// `emit_cfg_ssa_diagnostics` test path).
    fn place_suppressed_dead_stores(
        &self,
        fu: &crate::compilation_unit::FunctionUnit,
    ) -> std::collections::HashSet<(String, i32)> {
        self.registry.as_ref().map_or_else(Default::default, |reg| {
            crate::place_bridge::element_writes_observed_by_reads(&fu.cfg, &fu.name, reg)
        })
    }

    /// Variable names read inside positions the version-precise SSA `used`
    /// set can't see — `[…]` command substitutions in command arguments,
    /// `expr` values, and `if`/`while`/`for` branch conditions. A write to
    /// such a name is not a dead store even when its SSA version looks
    /// unused.
    fn substitution_hidden_reads(
        &self,
        fu: &crate::compilation_unit::FunctionUnit,
    ) -> FxHashSet<String> {
        self.registry
            .as_ref()
            .map_or_else(FxHashSet::default, |reg| {
                Self::substitution_hidden_reads_of(fu, reg)
            })
    }

    /// `self`-free core of [`Self::substitution_hidden_reads`] so the explorer's
    /// liveness dead-store pass (which has no `Analyser`) can reuse it.
    pub(crate) fn substitution_hidden_reads_of(
        fu: &crate::compilation_unit::FunctionUnit,
        registry: &tcl_registry::CommandRegistry,
    ) -> FxHashSet<String> {
        use crate::var_refs::{VarReferenceScanner, VarScanOptions};
        let mut out = FxHashSet::default();
        // Command-argument + AssignValue substitutions (deep RMW scan minus
        // shallow), already factored out for the optimiser's elimination pass.
        out.extend(crate::optimiser::elimination::collect_rmw_hidden_reads(
            fu, registry,
        ));
        // Branch conditions and expr-valued statements carry their `[…]` in an
        // `ExprNode`, not a word. Walk the AST for `Command` nodes (bracketed
        // substitution text) and scan each — their inner reads are invisible to
        // the version-precise `used` set, so they keep every write alive. A
        // bare `$x` in `if {$x}` is already a version-precise condition use, so
        // it is not collected here.
        let mut deep = VarReferenceScanner::new(VarScanOptions {
            include_var_read_roles: true,
            recurse_cmd_substitutions: true,
            include_reads_before_write: true,
        });
        let mut cmd_texts: Vec<String> = Vec::new();
        for block in fu.cfg.blocks.values() {
            for stmt in &block.statements {
                if let crate::ir::Statement::AssignExpr { expr, .. }
                | crate::ir::Statement::ExprEval { expr, .. } = stmt
                {
                    collect_expr_command_texts(expr, &mut cmd_texts);
                }
            }
            if let Some(crate::cfg::Terminator::Branch { condition, .. }) = &block.terminator {
                collect_expr_command_texts(condition, &mut cmd_texts);
            }
        }
        for text in &cmd_texts {
            out.extend(deep.scan_word(text, registry));
        }
        out
    }

    /// W220 — dead-store hint.
    ///
    /// A *dead store* is an
    /// assignment whose value is overwritten before being read —
    /// some other SSA version of the same variable is live, so
    /// this version's value never reaches a user.
    ///
    /// Walks every dead [`Statement`](crate::ir::Statement) chain
    /// in `fu.def_use`, checks that another version of the same
    /// variable has live uses, and emits a Hint at the dead
    /// statement's span.  When the variable's name has a
    /// case-insensitive twin among `defined_vars`, the message
    /// includes a "did you mean…?" suggestion.
    ///
    /// Filters applied:
    ///
    /// 1. **SCCP-unreachable blocks** — definitions in blocks
    ///    SCCP proved unreachable are reported as O107 by the
    ///    optimiser and intentionally suppressed here so we
    ///    don't double-up on dead-code calls.
    /// 2. **Scope aliases** (`global` / `upvar`) — writes are
    ///    visible in another scope; the local "no use" verdict
    ///    is unsafe.
    /// 3. **Cross-event vars** — for `pkgIndex.tcl` `$dir` and
    ///    iRules `::when::*` cross-event defs/imports, a write
    ///    in one event may be read in another at runtime.
    /// 4. **Globals (`::`-prefixed)** — externally consumed.
    /// 5. **Side-effecting stores** — only `AssignConst`,
    ///    `AssignValue` without `[`, and `AssignExpr` without a
    ///    command call are considered.  `Call.defs`, `Incr`, and
    ///    other side-effecting writes shouldn't be flagged
    ///    because removing the assignment would also drop the
    ///    side effect.
    fn emit_dead_store_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        defined_vars: &HashSet<String>,
        scope_aliases: &HashSet<String>,
        cross_event_vars: &HashSet<String>,
    ) {
        use crate::def_use::DefKind;
        use crate::ir::Statement;
        use crate::ir_helpers::expr_has_command;
        use std::fmt::Write as _;
        let hidden_reads = self.substitution_hidden_reads(fu);
        // Array-element / dict-path writes the
        // name-level SSA mis-folds but that a read actually observes.
        let place_suppressed = self.place_suppressed_dead_stores(fu);
        for chain in fu.def_use.chains.values() {
            if !chain.is_dead() || chain.definition.kind != DefKind::Statement {
                continue;
            }
            let (var, _version) = &chain.key;
            // A name read inside a command substitution / expr / branch
            // condition the version-precise `used` set can't see keeps every
            // write of it alive (`set i 0` before `[incr i $j]`). Suppress at
            // name level.
            if hidden_reads.contains(var) {
                continue;
            }
            // Globals (``::``-prefixed) are externally consumed.
            if var.starts_with("::") {
                continue;
            }
            // Scope-aliased vars (introduced via ``global`` or
            // ``upvar``) write through to a different scope — the
            // local "no use" verdict is unsafe.
            if scope_aliases.contains(var) {
                continue;
            }
            // Cross-event vars (iRules ``::when::*`` defs/imports
            // or ``pkgIndex.tcl`` ``$dir``) may be read in
            // another event/scope at runtime.
            if cross_event_vars.contains(var) {
                continue;
            }
            // Suppress dead stores in SCCP-unreachable blocks —
            // O107 already reports the whole block as dead, and
            // re-flagging individual stores inside it adds noise.
            if !fu.sccp.executable_blocks.contains(&chain.definition.block) {
                continue;
            }
            // A dead assignment is W220 whether or not the variable is also
            // unused overall: the assignment-level dead store (W220) and,
            // when the variable is never read at all, the variable-level
            // unused hint (W211) are distinct diagnostics with distinct
            // fixes (drop this assignment vs. drop the variable).  Fires
            // on any dead store regardless of other live versions.
            let Some(block) = fu.cfg.blocks.get(&chain.definition.block) else {
                continue;
            };
            let Ok(idx) = usize::try_from(chain.definition.statement_index) else {
                continue;
            };
            let Some(stmt) = block.statements.get(idx) else {
                continue;
            };
            // IR-statement type filter.
            // Only pure assignments are reportable; side-effecting
            // writes (``Call``, ``Incr``, command-substitution
            // values, expressions invoking commands) are skipped
            // because dropping them would also drop the side
            // effect.
            match stmt {
                Statement::AssignConst { .. } => {}
                Statement::AssignValue { value, .. } => {
                    if value.contains('[') {
                        continue;
                    }
                }
                Statement::AssignExpr { expr, .. } => {
                    if expr_has_command(expr) {
                        continue;
                    }
                }
                _ => continue,
            }
            // Suppress when this element write is observed by a read the
            // name-level SSA can't see (place-model overlap).
            if place_suppressed.contains(&(
                chain.definition.block.clone(),
                chain.definition.statement_index,
            )) {
                continue;
            }
            let cmd_span = fu.abs_span(stmt.span());
            if cmd_span.is_empty() {
                continue;
            }
            // Anchor at the variable name (the assignment target), not the
            // command-start column.
            let span = self.narrow_to_assigned_name(cmd_span).unwrap_or(cmd_span);
            let mut message = format!("Assignment to '{var}' is never read");
            if let Some(similar) = find_case_mismatch(var, defined_vars) {
                let _ = write!(message, "; did you mean '{similar}'?");
            }
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "W220".to_string(),
                span,
                message,
                severity: Severity::Hint,
                fixes: Vec::new(),
            });
        }
    }

    /// Narrow a whole-command span to its assignment-target token (the
    /// second word, `argv[1]`), returning that token's absolute span — or
    /// `None` when it can't be located, so callers fall back to the command
    /// span.  W211 / W220 anchor at the variable-name column, not the command
    /// start.  Re-lexes the command's own source slice (token-based) and
    /// takes the first non-separator word after the command name.
    fn narrow_to_assigned_name(&self, stmt_span: tcl_lexer::Span) -> Option<tcl_lexer::Span> {
        let base = stmt_span.start();
        let slice = source_slice(&self.source, stmt_span)?;
        let toks = tcl_lexer::Lexer::with_source_map(
            tcl_lexer::SourceMap::new(&slice),
            self.lexer_config(),
        )
        .tokenise_all()
        .ok()?;
        let name = toks
            .iter()
            .filter(|t| {
                !matches!(
                    t.kind,
                    tcl_lexer::TokenType::Sep
                        | tcl_lexer::TokenType::Eol
                        | tcl_lexer::TokenType::Comment
                )
            })
            .nth(1)?;
        Some(tcl_lexer::Span::new(
            name.span.start() + base,
            name.span.end() + base,
        ))
    }

    /// Narrow a whole-command span to the `$var` read token for *var*,
    /// returning that token's absolute span — or `None` when no matching
    /// top-level `Var` token is found (e.g. the read is nested inside a
    /// quoted/compound word, where the caller falls back to the command
    /// span).  W210 anchors at the variable read, not the command-start
    /// column.
    fn narrow_to_read_var(&self, stmt_span: tcl_lexer::Span, var: &str) -> Option<tcl_lexer::Span> {
        // De-sigil + drop any array-index suffix so `$a(k)` / `${a}` / `$a`
        // all compare equal to the chain's scalar/element base name.
        fn base(text: &str) -> &str {
            let inner = text.strip_prefix("${").map_or_else(
                || text.strip_prefix('$').unwrap_or(text),
                |i| i.strip_suffix('}').unwrap_or(i),
            );
            inner.split('(').next().unwrap_or(inner)
        }
        let target = base(var);
        let start = stmt_span.start();
        let slice = source_slice(&self.source, stmt_span)?;
        let sm = tcl_lexer::SourceMap::new(&slice);
        let toks = tcl_lexer::Lexer::with_source_map(
            tcl_lexer::SourceMap::new(&slice),
            self.lexer_config(),
        )
        .tokenise_all()
        .ok()?;
        toks.iter()
            .find(|t| t.kind == tcl_lexer::TokenType::Var && base(sm.token_text(**t)) == target)
            .map(|t| tcl_lexer::Span::new(t.span.start() + start, t.span.end() + start))
    }

    /// W211 — unused-variable hint.
    ///
    /// Fires when an
    /// assignment's variable has no live uses **and** no other
    /// SSA version is live (so the variable is entirely unused
    /// — distinct from W220's overwritten-before-read case).
    ///
    /// Three filters apply:
    ///
    /// 1. **Scope aliases** (``global`` / ``upvar``) — writes
    ///    are visible in the aliased scope, so a "no local use"
    ///    verdict is unsafe.
    /// 2. **Textual references** — variable names that appear
    ///    inside a ``"$x"`` string interpolation or a
    ///    ``Return`` value are kept live; the def-use builder
    ///    doesn't track those reads.
    /// 3. **Empty spans** — synthetic IR statements with no
    ///    user-visible source text.
    ///
    /// "Did you mean…?" suggestions use case-insensitive
    /// matching against the function's defined-variable set.
    fn emit_unused_variable_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        defined_vars: &HashSet<String>,
        scope_aliases: &HashSet<String>,
        textually_referenced: &HashSet<String>,
    ) {
        use crate::def_use::DefKind;
        use std::fmt::Write as _;
        // W211 is a per-variable verdict ("the variable is set but never
        // used"), not per-assignment: a variable set several times and never
        // read fires once, at its earliest definition. Collect the earliest
        // reportable span per variable, then emit a single W211 per unused
        // variable.
        let mut earliest: std::collections::HashMap<String, tcl_lexer::Span> =
            std::collections::HashMap::new();
        for chain in fu.def_use.chains.values() {
            if !chain.is_dead() || chain.definition.kind != DefKind::Statement {
                continue;
            }
            let (var, version) = &chain.key;
            if scope_aliases.contains(var) {
                continue;
            }
            if textually_referenced.contains(var) {
                continue;
            }
            // Only emit when no other SSA version of this var is
            // live — the W220 path handles overwritten cases.
            let any_other_live = fu
                .def_use
                .chains
                .iter()
                .any(|(k, c)| k.0 == *var && k.1 != *version && !c.is_dead());
            if any_other_live {
                continue;
            }
            let Some(block) = fu.cfg.blocks.get(&chain.definition.block) else {
                continue;
            };
            let Ok(idx) = usize::try_from(chain.definition.statement_index) else {
                continue;
            };
            let Some(stmt) = block.statements.get(idx) else {
                continue;
            };
            // Only pure assignments are reportable as "set but never used".
            // A variable written by a command (`scan` / `binary scan` /
            // `regexp -> capture`, etc.) or a barrier is a command output the
            // user may legitimately ignore; `IRCall` / `IRBarrier` defs are
            // skipped.
            if matches!(
                stmt,
                crate::ir::Statement::Call { .. } | crate::ir::Statement::Barrier { .. }
            ) {
                continue;
            }
            // Approach B: CFG span is relative to the unit's `base_offset`.
            let cmd_span = fu.abs_span(stmt.span());
            if cmd_span.is_empty() {
                continue;
            }
            // Anchor at the variable name (the assignment target), not the
            // command-start column.
            let span = self.narrow_to_assigned_name(cmd_span).unwrap_or(cmd_span);
            earliest
                .entry(var.clone())
                .and_modify(|s| {
                    if span.start() < s.start() {
                        *s = span;
                    }
                })
                .or_insert(span);
        }
        let mut entries: Vec<(String, tcl_lexer::Span)> = earliest.into_iter().collect();
        entries.sort_by_key(|(_, span)| span.start());
        for (var, span) in entries {
            let mut message = format!("Variable '{var}' is set but never used");
            if let Some(similar) = find_case_mismatch(&var, defined_vars) {
                let _ = write!(message, "; did you mean '{similar}'?");
            }
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "W211".to_string(),
                span,
                message,
                severity: Severity::Hint,
                fixes: Vec::new(),
            });
        }
    }

    /// H300 — possible paste error (duplicate dead-store with
    /// identical literal).
    ///
    /// When two consecutive
    /// statements in the same block are both dead stores AND
    /// share the same paste-fingerprint
    /// (same variable name + same trimmed literal value), emit
    /// a Hint at the *second* statement's span — the duplicate
    /// is the one that's almost certainly a paste error.
    ///
    /// Variables whose names start with ``_`` are excluded from
    /// the heuristic on the assumption that the leading
    /// underscore signals the user has flagged them as
    /// intentional.
    fn emit_possible_paste_error_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
    ) {
        use crate::def_use::DefKind;

        // Pre-compute, per block, the set of statement indices
        // that are dead stores.  Walk every dead Statement-kind
        // chain in def_use, bucket by block.
        let mut dead_idx: FxHashMap<&str, FxHashSet<usize>> = FxHashMap::default();
        for chain in fu.def_use.chains.values() {
            if !chain.is_dead() || chain.definition.kind != DefKind::Statement {
                continue;
            }
            let Ok(idx) = usize::try_from(chain.definition.statement_index) else {
                continue;
            };
            dead_idx
                .entry(chain.definition.block.as_str())
                .or_default()
                .insert(idx);
        }

        for (block_name, block) in &fu.cfg.blocks {
            let Some(dead_indices) = dead_idx.get(block_name.as_str()) else {
                continue;
            };
            // Walk consecutive pairs (idx, idx + 1).  Only the
            // first must be dead — the second's
            // dead-status is irrelevant; what matters is whether
            // the value being assigned matches.
            for idx in 0..block.statements.len().saturating_sub(1) {
                if !dead_indices.contains(&idx) {
                    continue;
                }
                let Some(first) = super::utils::possible_paste_fingerprint(&block.statements[idx])
                else {
                    continue;
                };
                let Some(second) =
                    super::utils::possible_paste_fingerprint(&block.statements[idx + 1])
                else {
                    continue;
                };
                if first != second {
                    continue;
                }
                let (var_name, literal) = first;
                if var_name.starts_with('_') {
                    continue;
                }
                let span = fu.abs_span(block.statements[idx + 1].span());
                if span.is_empty() {
                    continue;
                }
                let pretty = super::utils::format_literal_for_message(&literal);
                let message = format!(
                    "Possible paste error: repeated assignment to '{var_name}' \
                     with static value '{pretty}'; \
                     did you mean to assign a different variable?"
                );
                self.result.diagnostics.push(super::types::Diagnostic {
                    code: "H300".to_string(),
                    span,
                    message,
                    severity: Severity::Hint,
                    fixes: Vec::new(),
                });
            }
        }
    }

    /// W214 — unused-parameter hint.
    ///
    /// For every parameter
    /// declared in `ir_proc.params`, check whether any def-use
    /// chain for the parameter (any SSA version) has live uses.
    /// When all chains are dead, the parameter is unused —
    /// emit a Hint at the proc's span.
    fn emit_unused_param_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        ir_proc: &crate::ir::Procedure,
    ) {
        // Empty-body procs (``proc foo {a b} {}``) are signature
        // placeholders — stubs declaring an API whose implementation
        // lives elsewhere.  Every parameter is necessarily "unused"
        // since there is no body to use it, so flagging is pure noise.
        if ir_proc.body.statements.is_empty() {
            return;
        }
        let mut unused: Vec<String> = Vec::new();
        for param in &ir_proc.params {
            // Tcl's variadic ``args`` parameter is conventionally
            // declared even when unused (as a "consume the rest"
            // marker).  Skip it from W214.
            if param == "args" {
                continue;
            }
            // Positional keyword markers: a param whose name is itself a
            // quoted literal (snit-style ``{"as" ""}``) is a syntactic
            // placeholder consumed by being PRESENT in the call form, not
            // read as a variable.  Flagging it is noise.  Conservative:
            // only suppress params whose name starts AND ends with ``"``.
            if param.len() >= 2 && param.starts_with('"') && param.ends_with('"') {
                continue;
            }
            let any_live = fu
                .def_use
                .chains
                .iter()
                .any(|(k, c)| k.0 == *param && !c.is_dead());
            if any_live {
                continue;
            }
            // Fallback: the def-use builder doesn't track variable
            // references inside ``[expr {...}]`` command
            // substitutions or arbitrary nested ``[cmd ...]``
            // bodies that don't lower into a structured IR.
            // If the body source contains a ``$param`` /
            // ``${param}`` reference anywhere, treat the parameter
            // as used and skip W214.  Saves the W214 over-emit on
            // ``proc f {x} { return [expr {$x + 1}] }``-style bodies.
            if let Some(body_source) = ir_proc.body_source.as_deref()
                && body_references_param(body_source, param)
            {
                continue;
            }
            unused.push(param.clone());
        }
        if unused.is_empty() {
            return;
        }
        // Dispatch-protocol suppression: when ≥3 peer procs in this
        // namespace share this proc's leading-param signature AND an
        // arity-compatible variable-command dispatcher exists, the leading
        // params are an external contract, not genuinely unused.  Computed
        // only when there is something to report.
        let ns = namespace_of(&ir_proc.qualified_name);
        let leading: Vec<String> = ir_proc
            .params
            .iter()
            .take_while(|p| *p != "args")
            .cloned()
            .collect();
        let protocol_params: HashSet<String> = if !leading.is_empty()
            && self
                .dispatch_protocol_signatures()
                .contains(&(ns, leading.clone()))
        {
            leading.into_iter().collect()
        } else {
            HashSet::new()
        };
        for param in unused {
            if protocol_params.contains(&param) {
                continue;
            }
            let message = format!(
                "Parameter '{param}' of proc '{name}' is unused",
                name = ir_proc.qualified_name,
            );
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "W214".to_string(),
                span: ir_proc.span,
                message,
                severity: Severity::Hint,
                fixes: Vec::new(),
            });
        }
    }

    /// Identify `(namespace, leading-param-list)` pairs that look like a
    /// **dispatch protocol** — ≥3 peer procs in the same namespace sharing a
    /// leading-param signature dictated by an arity-compatible
    /// variable-command dispatcher.
    fn dispatch_protocol_signatures(&self) -> HashSet<(String, Vec<String>)> {
        // Group user procs by (namespace, leading-param-tuple stopping at `args`).
        let mut groups: FxHashMap<(String, Vec<String>), usize> = FxHashMap::default();
        for (qname, pdef) in &self.result.all_procs {
            let leading: Vec<String> = pdef
                .params
                .iter()
                .take_while(|p| p.name != "args")
                .map(|p| p.name.clone())
                .collect();
            if leading.is_empty() {
                continue;
            }
            *groups.entry((namespace_of(qname), leading)).or_insert(0) += 1;
        }
        let peer_protos: HashSet<(String, Vec<String>)> = groups
            .into_iter()
            .filter(|(_, n)| *n >= 3)
            .map(|(k, _)| k)
            .collect();
        if peer_protos.is_empty() {
            return HashSet::new();
        }
        // Dispatcher evidence: map each dispatcher namespace → the argument
        // counts observed at its variable-command sites.
        let mut dispatcher_ns_argc: FxHashMap<String, FxHashSet<usize>> = FxHashMap::default();
        for site in &self.var_command_sites {
            let off = site.cmd_span.start();
            let dns = self
                .result
                .all_procs
                .iter()
                .find(|(_, p)| p.body_span.start() <= off && off <= p.body_span.end())
                .map_or_else(|| "::".to_string(), |(q, _)| namespace_of(q));
            dispatcher_ns_argc.entry(dns).or_default().insert(site.argc);
        }
        peer_protos
            .into_iter()
            .filter(|(ns_key, params)| {
                let min_argc = params.len();
                dispatcher_ns_argc.iter().any(|(dns, argcs)| {
                    (dns == ns_key || dns.starts_with(&format!("{ns_key}::")))
                        && argcs.iter().any(|&a| a >= min_argc)
                })
            })
            .collect()
    }

    /// W210 + W213 — read-before-set / unset on possibly-undefined.
    ///
    /// Walks every
    /// version-0 chain (`DefKind::Parameter`) in `fu.def_use`
    /// — those are the synthetic defs the def-use builder
    /// emits when a variable is used without a preceding def.
    ///
    /// Distinguishes real proc parameters from synthetic RBS
    /// reads via `ir_proc.params`.  Only emits inside procedures
    /// (i.e. when `ir_proc` is `Some`) — top-level RBS needs the
    /// `globals_written_by_procs` filter.
    ///
    /// Per use site:
    ///
    /// - **Phi-incoming uses** are skipped — they sit at block
    ///   boundaries and don't anchor on a real statement.
    /// - **`unset` without `-nocomplain`** emits W213 (the more
    ///   specific code) instead of W210.  W213 message tells
    ///   the user to add `-nocomplain` rather than initialise
    ///   the variable.
    /// - **`safe_on_uninit` calls** that initialise the variable
    ///   themselves (it's in their `defs`) are skipped —
    ///   commands like `lappend` / `incr` / `dict set` safely
    ///   initialise an uninitialised variable.
    /// - Everything else emits W210 with the canonical
    ///   "read before set" message + optional "did you mean…?"
    ///   suggestion.
    #[allow(clippy::too_many_lines)]
    fn emit_read_before_set_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        ir_proc: Option<&crate::ir::Procedure>,
        defined_vars: &HashSet<String>,
        scope_aliases: &HashSet<String>,
        extra_known_defined: &HashSet<String>,
        supp: &UndefSuppression,
    ) {
        use crate::def_use::{DefKind, UseKind};
        use crate::ir::Statement;
        use std::fmt::Write as _;

        // Top-level RBS uses the ``extra_known_defined`` set
        // (computed from ``globals_written_by_procs``) to suppress
        // W210 on globals that helper procs write.  Inside procs the
        // set is empty.
        let params_owned: HashSet<&str> = match ir_proc {
            Some(p) => p.params.iter().map(String::as_str).collect(),
            None => HashSet::new(),
        };
        let params = &params_owned;

        // Collect `[info exists X]` / `[array exists X]`
        // guards: `(var, guard_block)` where reads of `var` in any
        // block dominated by `guard_block` are guarded (X is known to
        // exist there).  Positive guards the true arm; `![info exists
        // X]` guards the false arm.
        let exists_guards = collect_existence_guards(fu);

        // W210 fires **once per variable**, at the earliest read-before-set.
        // The def-use walk below
        // visits *every* version-0 use, so record the earliest passing span
        // per variable here and emit after the walk (W213, a distinct code,
        // stays inline).
        let mut w210_min: std::collections::HashMap<String, tcl_lexer::Span> =
            std::collections::HashMap::new();

        for chain in fu.def_use.chains.values() {
            // Version-0 synthetic defs are the undef origin; an
            // `unset`-killed real version, and a phi version that can reach
            // an undef origin (one-branch `set` / try-handler merge), are
            // undef at their reads too — all flow through the same
            // suppression + emission logic below.
            if chain.definition.kind != DefKind::Parameter
                && !supp.killed.contains(&chain.key)
                && !supp.can_undef.contains(&chain.key)
            {
                continue;
            }
            let (var, _version) = &chain.key;
            if params.contains(var.as_str()) {
                continue;
            }
            // A dynamic-target upvar local is possibly-unset, so its
            // scope-alias status must not suppress the read-before-set (an
            // unconditional `$local` read still fires; an `[info exists local]`
            // guard suppresses it per-use below).
            if scope_aliases.contains(var) && !supp.dynamic_upvar_locals.contains(var) {
                continue;
            }
            if extra_known_defined.contains(var) {
                continue;
            }
            // `dict with`/`dict update` unpacking + qualified-`variable`
            // alias tails suppress version-0 reads of the unpacked / aliased
            // names (the `puts $a` inside `dict with d {…}` is not RBS).
            // Interproc constant propagation resolves an empty caller dict to
            // CONST("") (keys = ∅, not unknown), so the blanket variant fires
            // on a genuine missing-key read while still suppressing an
            // unknown-shape (mixed-caller / no-caller) dict.
            if supp.suppresses(var) {
                continue;
            }
            for use_site in &chain.uses {
                if matches!(use_site.kind, UseKind::PhiIncoming) {
                    continue;
                }
                let Some(block) = fu.cfg.blocks.get(&use_site.block) else {
                    continue;
                };
                let (span, stmt_opt): (tcl_lexer::Span, Option<&Statement>) =
                    if use_site.statement_index == -1 {
                        let Some(span) = block
                            .terminator
                            .as_ref()
                            .and_then(crate::cfg::Terminator::span)
                        else {
                            continue;
                        };
                        (fu.abs_span(span), None)
                    } else {
                        let Ok(idx) = usize::try_from(use_site.statement_index) else {
                            continue;
                        };
                        let Some(stmt) = block.statements.get(idx) else {
                            continue;
                        };
                        (fu.abs_span(stmt.span()), Some(stmt))
                    };
                if span.is_empty() {
                    continue;
                }
                // A read-modify-write command (`lappend` / `append`) that
                // auto-creates its target is not a read-before-set: it both
                // reads and defines the variable, creating it from an empty
                // default when absent. `unset` also carries
                // `reads_own_defs` but is destructive, not auto-creating — its
                // missing-variable case is exactly the W213 handled just below,
                // so it must not be skipped here.
                if let Some(Statement::Call {
                    reads_own_defs: true,
                    command,
                    defs,
                    ..
                }) = stmt_opt
                    && command != "unset"
                    && defs.iter().any(|d| d == var)
                {
                    continue;
                }
                // Skip the existence-query word itself and
                // reads narrowed by an enclosing `[info exists X]` guard.
                if existence_exempt(stmt_opt, var, &exists_guards, &fu.ssa, &use_site.block) {
                    continue;
                }
                // ``unset`` without ``-nocomplain`` → W213.
                if let Some(Statement::Call { command, args, .. }) = stmt_opt
                    && command == "unset"
                    && !args.iter().any(|a| a == "-nocomplain")
                {
                    let message = format!(
                        "Variable '{var}' may not exist; \
                             use 'unset -nocomplain' to suppress the error",
                    );
                    self.result.diagnostics.push(super::types::Diagnostic {
                        code: "W213".to_string(),
                        span,
                        message,
                        severity: Severity::Warning,
                        fixes: Vec::new(),
                    });
                    continue;
                }
                // A use site that itself safely initialises the variable
                // (`safe_on_uninit` calls like `lappend`/`dict set`, or an
                // `incr` of its own target) is not read-before-set.
                if use_site_safe_initialises(stmt_opt, var) {
                    continue;
                }
                // Anchor at the `$var` read token; fall back to the command
                // span when the read is nested inside a quoted/compound word.
                let read_span = self.narrow_to_read_var(span, var).unwrap_or(span);
                w210_min
                    .entry(var.clone())
                    .and_modify(|s| {
                        if read_span.start() < s.start() {
                            *s = read_span;
                        }
                    })
                    .or_insert(read_span);
            }
        }

        let mut entries: Vec<(String, tcl_lexer::Span)> = w210_min.into_iter().collect();
        entries.sort_by_key(|(_, s)| s.start());
        for (var, span) in entries {
            let mut message = format!("Variable '{var}' is read before it is set");
            if let Some(similar) = find_case_mismatch(&var, defined_vars) {
                let _ = write!(message, "; did you mean '{similar}'?");
            }
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "W210".to_string(),
                span,
                message,
                severity: Severity::Warning,
                fixes: Vec::new(),
            });
        }
    }

    /// W210 on `return $v` reads where `v`'s reaching version can be
    /// undefined on some executable path (phi-from-undef / `unset`-killed).
    /// Companion to [`Self::emit_read_before_set_diagnostics`]; see its
    /// trailing call site for why the def-use-chain pass cannot catch
    /// these (return values are terminator reads, not recorded uses).
    #[allow(clippy::too_many_arguments)]
    fn emit_return_phi_undef_w210(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        params: &HashSet<&str>,
        exists_guards: &[(String, String)],
        scope_aliases: &HashSet<String>,
        extra_known_defined: &HashSet<String>,
        defined_vars: &HashSet<String>,
        considered: &HashSet<String>,
        supp: &UndefSuppression,
    ) {
        use crate::var_refs::{VarReferenceScanner, VarScanOptions};
        use std::fmt::Write as _;

        let Some(registry) = self.registry.as_ref() else {
            return;
        };

        let (phi_def, phi_block, killed) = build_phi_undef_index(&fu.ssa, considered);

        let mut scanner = VarReferenceScanner::new(VarScanOptions {
            include_var_read_roles: false,
            recurse_cmd_substitutions: true,
            include_reads_before_write: false,
        });

        let mut reported: FxHashSet<String> = FxHashSet::default();
        // Deterministic block order for stable diagnostics.
        let mut block_names: Vec<&String> = considered.iter().collect();
        block_names.sort();

        for bn in block_names {
            let Some(cfg_block) = fu.cfg.blocks.get(bn) else {
                continue;
            };
            let Some(crate::cfg::Terminator::Return { value, expr, .. }) = &cfg_block.terminator
            else {
                continue;
            };
            let Some(span) = cfg_block
                .terminator
                .as_ref()
                .and_then(crate::cfg::Terminator::span)
                .map(|s| fu.abs_span(s))
            else {
                continue;
            };
            if span.is_empty() {
                continue;
            }
            let Some(ssa_block) = fu.ssa.blocks.get(bn) else {
                continue;
            };

            // Collect the variable names read by the return value (word
            // substitutions + nested `[...]`) and any parsed expr.
            let mut reads: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
            if let Some(v) = value {
                reads.extend(scanner.scan_script(v, registry));
            }
            if let Some(e) = expr {
                reads.extend(crate::var_refs::vars_in_expr(e));
            }

            for name in reads {
                if reported.contains(&name) {
                    continue;
                }
                let ver = ssa_block.exit_versions.get(&name).copied().unwrap_or(0);
                // Version-0 return reads are now recorded in def_use, so the
                // version-0 (`DefKind::Parameter`) emitter handles them with
                // the full suppression set — this pass only covers the
                // phi-from-undef / `unset`-killed (version > 0) cases, which
                // def-use can't express.  Skipping ver 0 avoids double-firing.
                if ver == 0 {
                    continue;
                }
                let mut seen = FxHashSet::default();
                if !phi_can_undef(
                    &name,
                    ver,
                    &phi_def,
                    &phi_block,
                    &killed,
                    considered,
                    &fu.sccp.executable_edges,
                    exists_guards,
                    &fu.ssa,
                    &mut seen,
                ) {
                    continue;
                }
                if params.contains(name.as_str())
                    || scope_aliases.contains(&name)
                    || extra_known_defined.contains(&name)
                    || is_implicit_var(&name)
                    || name.contains("::")
                    || supp.suppresses(&name)
                {
                    continue;
                }
                // A dominating existence guard proves the var exists here.
                if exists_guards
                    .iter()
                    .any(|(gv, gblk)| *gv == name && block_dominated_by(&fu.ssa, bn, gblk))
                {
                    continue;
                }
                reported.insert(name.clone());
                let mut message = format!("Variable '{name}' is read before it is set");
                if let Some(similar) = find_case_mismatch(&name, defined_vars) {
                    let _ = write!(message, "; did you mean '{similar}'?");
                }
                self.result.diagnostics.push(super::types::Diagnostic {
                    code: "W210".to_string(),
                    span,
                    message,
                    severity: Severity::Warning,
                    fixes: Vec::new(),
                });
            }
        }
    }

    /// **W210 (provably-unset regexp / scan output).** A `regexp` / `scan`
    /// with literal pattern + input that can be statically proven not to
    /// match leaves its output variables unset, so a later read of one is a
    /// real read-before-set.  Handles both the top-level call form and the
    /// call embedded in an `if` / `while` condition (firing only on the
    /// no-match branch).
    fn emit_provably_unset_w210(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        considered: &HashSet<String>,
        defined_vars: &HashSet<String>,
    ) {
        use crate::ir::Statement;
        use std::fmt::Write as _;

        // var name -> (def_block, def_stmt_idx); idx == -1 means "from the
        // start of the block" (the embedded-condition no-match target).
        let mut provably_unset: std::collections::HashMap<String, (String, i32)> =
            std::collections::HashMap::new();

        for bn in considered {
            let Some(block) = fu.cfg.blocks.get(bn) else {
                continue;
            };
            // Top-level regexp / scan calls.
            for (idx, stmt) in block.statements.iter().enumerate() {
                let Statement::Call {
                    command,
                    canonical_command,
                    args,
                    defs,
                    ..
                } = stmt
                else {
                    continue;
                };
                let canon = canonical_command.as_deref().unwrap_or(command);
                let is_regexp = canon == "::regexp" || command == "regexp";
                let is_scan = canon == "::scan" || command == "scan";
                if (!is_regexp && !is_scan) || defs.is_empty() {
                    continue;
                }
                if let Some(no_match) = regexp_scan_no_match(is_regexp, args)
                    && no_match
                {
                    for d in defs {
                        provably_unset.entry(d.clone()).or_insert_with(|| {
                            (bn.clone(), i32::try_from(idx).unwrap_or(i32::MAX))
                        });
                    }
                }
            }
            // regexp / scan embedded in the branch condition.
            if let Some(crate::cfg::Terminator::Branch {
                condition,
                true_target,
                false_target,
                ..
            }) = &block.terminator
            {
                Self::collect_embedded_provably_unset(
                    condition,
                    true_target,
                    false_target,
                    &mut provably_unset,
                );
            }
        }

        if provably_unset.is_empty() {
            return;
        }

        // Fire on every executable use after the def (same block) or in a
        // block dominated by the def block.
        let mut reported: FxHashSet<String> = FxHashSet::default();
        let mut block_names: Vec<&String> = considered.iter().collect();
        block_names.sort();
        for bn in block_names {
            let Some(ssa_block) = fu.ssa.blocks.get(bn) else {
                continue;
            };
            for (idx, s) in ssa_block.statements.iter().enumerate() {
                for name in s.uses.keys() {
                    if reported.contains(name) {
                        continue;
                    }
                    let Some((def_block, def_idx)) = provably_unset.get(name) else {
                        continue;
                    };
                    let in_def_block_after =
                        bn == def_block && i32::try_from(idx).unwrap_or(i32::MAX) > *def_idx;
                    let dominated = bn != def_block && block_dominated_by(&fu.ssa, bn, def_block);
                    if !(in_def_block_after || dominated) {
                        continue;
                    }
                    let span = match fu.cfg.blocks.get(bn).and_then(|b| b.statements.get(idx)) {
                        Some(st) if !st.span().is_empty() => fu.abs_span(st.span()),
                        _ => continue,
                    };
                    reported.insert(name.clone());
                    let mut message = format!("Variable '{name}' is read before it is set");
                    if let Some(similar) = find_case_mismatch(name, defined_vars) {
                        let _ = write!(message, "; did you mean '{similar}'?");
                    }
                    self.result.diagnostics.push(super::types::Diagnostic {
                        code: "W210".to_string(),
                        span,
                        message,
                        severity: Severity::Warning,
                        fixes: Vec::new(),
                    });
                }
            }
        }
    }

    /// Walk a branch `condition` for an embedded `[regexp …]` / `[scan …]`
    /// command substitution that provably can't match, recording its output
    /// variables as provably-unset on the no-match branch target (only when
    /// the condition is exactly `[cmd]` → false target, or `![cmd]` → true
    /// target; more complex shapes are skipped).
    fn collect_embedded_provably_unset(
        condition: &ExprNode,
        true_target: &str,
        false_target: &str,
        provably_unset: &mut std::collections::HashMap<String, (String, i32)>,
    ) {
        let (cmd_node, no_match_target) = match condition {
            ExprNode::Command { .. } => (condition, false_target),
            ExprNode::Unary {
                op: UnaryOp::Not | UnaryOp::WordNot,
                operand,
            } if matches!(operand.as_ref(), ExprNode::Command { .. }) => {
                (operand.as_ref(), true_target)
            }
            _ => return,
        };
        let ExprNode::Command { text, .. } = cmd_node else {
            return;
        };
        // Strip the surrounding `[` … `]` and segment the interior.
        let inner = text
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
            .unwrap_or(text);
        let segs = crate::segmenter::segment_commands(inner);
        let Some(seg) = segs.first() else {
            return;
        };
        let Some(cmd) = seg.texts.first() else {
            return;
        };
        let bare = cmd
            .trim_start_matches(':')
            .rsplit("::")
            .next()
            .unwrap_or(cmd);
        let is_regexp = bare == "regexp";
        let is_scan = bare == "scan";
        if !is_regexp && !is_scan {
            return;
        }
        let args: Vec<String> = seg.texts[1..].to_vec();
        let pos = skip_options(&args, if is_regexp { &["-start"] } else { &[] });
        if pos + 2 > args.len() {
            return;
        }
        let out_vars = &args[(pos + 2).min(args.len())..];
        if out_vars.is_empty() {
            return;
        }
        if regexp_scan_no_match(is_regexp, &args) != Some(true) {
            return;
        }
        for v in out_vars {
            let name = crate::naming::normalise_var_name(v);
            if !name.is_empty() {
                provably_unset
                    .entry(name.to_string())
                    .or_insert_with(|| (no_match_target.to_string(), -1));
            }
        }
    }

    /// I230 / I231 — constant branch / switch-arm condition.
    ///
    /// For every
    /// branch SCCP folded to a constant, when the *not-taken*
    /// target is also unreachable (i.e. SCCP confirmed only one
    /// path is feasible), emit an Info-level diagnostic so the
    /// LSP can highlight the dead arm.
    ///
    /// Code selection:
    /// - Block name starts with ``switch_`` → I231 (switch-arm).
    /// - Block name starts with ``if_`` → I230 (constant if).
    /// - Otherwise → I230 with the generic
    ///   ``"Branch condition '...' is constant"`` message.
    ///
    /// Severity is mapped to ``Hint`` because the Rust
    /// [`Severity`] enum has no ``Info`` variant — ``Hint`` is
    /// the closest non-actionable level.
    fn emit_constant_branch_diagnostics(&mut self, fu: &crate::compilation_unit::FunctionUnit) {
        for branch in &fu.sccp.constant_branches {
            // A branch is dead when the not-taken target is
            // unreachable.  SCCP exposes
            // ``executable_blocks`` (the complement); a block
            // is unreachable iff it's in ``cfg.blocks`` but
            // NOT in ``executable_blocks``.
            if fu.sccp.executable_blocks.contains(&branch.not_taken_target) {
                continue;
            }
            // Locate the branch's terminator span.
            let Some(block) = fu.cfg.blocks.get(&branch.block) else {
                continue;
            };
            let Some(crate::cfg::Terminator::Branch {
                span: Some(span), ..
            }) = &block.terminator
            else {
                continue;
            };
            let span = fu.abs_span(*span);

            let names = [
                branch.block.as_str(),
                branch.taken_target.as_str(),
                branch.not_taken_target.as_str(),
            ];
            let is_switch = names.iter().any(|n| n.starts_with("switch_"));
            let is_if = names.iter().any(|n| n.starts_with("if_"));
            let is_loop = names.iter().any(|n| {
                n.starts_with("while_") || n.starts_with("for_") || n.starts_with("foreach_")
            });
            // Suppress the idiomatic infinite loop `while 1 { … }`:
            // a constant-TRUE loop condition is intentional, not a bug (a
            // constant-FALSE loop still flags its unreachable body).
            if is_loop && branch.value {
                continue;
            }

            let (code, message) = if is_switch {
                let code = "I231";
                let msg = if branch.value {
                    format!(
                        "Switch condition '{}' is always true here; \
                         subsequent switch arms are unreachable",
                        branch.condition,
                    )
                } else {
                    format!(
                        "Switch arm condition '{}' is always false; \
                         this arm is unreachable",
                        branch.condition,
                    )
                };
                (code, msg)
            } else if is_if {
                let msg = if branch.value {
                    format!(
                        "Condition '{}' is always true; \
                         the alternate branch is unreachable",
                        branch.condition,
                    )
                } else {
                    format!(
                        "Condition '{}' is always false; \
                         the alternate branch is unreachable",
                        branch.condition,
                    )
                };
                ("I230", msg)
            } else {
                let msg = format!(
                    "Branch condition '{}' is constant; one branch is unreachable",
                    branch.condition,
                );
                ("I230", msg)
            };

            self.result.diagnostics.push(super::types::Diagnostic {
                code: code.to_string(),
                span,
                message,
                // I230/I231 are observational (LSP `Information`);
                // they previously collapsed to `Hint`.
                severity: Severity::Info,
                fixes: Vec::new(),
            });
        }
    }

    /// I230 — fold `[info exists X]` / `[array exists X]` conditions.
    ///
    /// SCCP can't fold these (the predicate lowers to an
    /// opaque `ExprNode::Command`, and SCCP has no parameter/existence
    /// facts), so the fold is computed by
    /// [`crate::sccp::existence_constant_branches`] using
    /// `ir_proc.params` — the same helper whose result
    /// `FunctionUnit::build` appends to `sccp.constant_branches` for the
    /// optimiser's O101 fold / DCE.  Emitting the I230 here (rather than
    /// via [`Self::emit_constant_branch_diagnostics`]) is deliberate:
    /// that emitter gates on the not-taken arm being unreachable in
    /// `executable_blocks`, which these post-pass folds don't update, so
    /// it skips them and there is no double emission.
    fn emit_existence_constant_branch_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        ir_proc: Option<&crate::ir::Procedure>,
    ) {
        let params: HashSet<&str> = match ir_proc {
            Some(p) => p.params.iter().map(String::as_str).collect(),
            None => HashSet::new(),
        };
        for cb in crate::sccp::existence_constant_branches(&fu.cfg, &params) {
            let Some(span) = cb.span.map(|s| fu.abs_span(s)) else {
                continue;
            };
            let message = if cb.value {
                format!(
                    "Condition '{}' is always true; the alternate branch is unreachable",
                    cb.condition,
                )
            } else {
                format!(
                    "Condition '{}' is always false; the alternate branch is unreachable",
                    cb.condition,
                )
            };
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "I230".to_string(),
                span,
                message,
                // I230 is observational (LSP `Information`).
                severity: Severity::Info,
                fixes: Vec::new(),
            });
        }
    }

    /// W126 — channel-argument validation.
    ///
    /// Walks every
    /// SSA-annotated `Call` statement for commands that declare
    /// `ArgRole::Channel` arguments; for each channel-position
    /// argument, checks the SSA type lattice to determine whether
    /// the value is genuinely a channel.  Two failure modes:
    ///
    /// - **`$var` reference** with `TypeKind::Known` and a non-
    ///   `TclType::Channel` type — emits "passed as channel … has
    ///   type X, not CHANNEL".
    /// - **String literal** that isn't `stdin` / `stdout` /
    ///   `stderr` and contains no substitutions — emits
    ///   "String literal 'X' used as channel argument".
    ///
    /// The standard channels (`stdin`, `stdout`, `stderr`) are
    /// always accepted.  Unknown / overdefined types skip the
    /// check (could be anything).
    fn emit_channel_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        registry: &tcl_registry::CommandRegistry,
    ) {
        use crate::ir::Statement;
        use crate::types::TypeKind;
        use tcl_registry::ArgRole;

        const STANDARD_CHANNELS: &[&str] = &["stdout", "stderr", "stdin"];

        for block in fu.ssa.blocks.values() {
            for ssa_stmt in &block.statements {
                let Statement::Call {
                    command,
                    args,
                    span,
                    ..
                } = &ssa_stmt.statement
                else {
                    continue;
                };
                let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
                let channel_indices =
                    registry.arg_indices_for_role(command, &arg_refs, ArgRole::Channel);
                if channel_indices.is_empty() {
                    continue;
                }
                for idx in channel_indices {
                    if idx >= args.len() {
                        continue;
                    }
                    let arg_text = &args[idx];
                    // Extract bare var name from ``$var`` / ``${var}``.
                    let var_name: Option<&str> =
                        if arg_text.starts_with("${") && arg_text.ends_with('}') {
                            Some(&arg_text[2..arg_text.len() - 1])
                        } else if let Some(rest) = arg_text.strip_prefix('$') {
                            Some(rest)
                        } else {
                            None
                        };

                    if let Some(name) = var_name {
                        let Some(&version) = ssa_stmt.uses.get(name) else {
                            continue;
                        };
                        let key: crate::ssa::ValueKey = (name.to_string(), version);
                        let Some(var_type) = fu.types.get(&key) else {
                            continue;
                        };
                        if var_type.kind != TypeKind::Known {
                            continue;
                        }
                        let Some(tcl_type) = var_type.tcl_type else {
                            continue;
                        };
                        if matches!(tcl_type, tcl_registry::TclType::Channel) {
                            continue;
                        }
                        let type_label = format!("{tcl_type:?}").to_uppercase();
                        let message = format!(
                            "Variable '${name}' passed as channel to '{command}' \
                             has type {type_label}, not CHANNEL.",
                        );
                        self.result.diagnostics.push(super::types::Diagnostic {
                            code: "W126".to_string(),
                            span: fu.abs_span(*span),
                            message,
                            severity: Severity::Warning,
                            fixes: Vec::new(),
                        });
                    } else {
                        // Literal — strip surrounding braces / quotes.
                        let literal = arg_text
                            .trim_matches('"')
                            .trim_start_matches('{')
                            .trim_end_matches('}');
                        if STANDARD_CHANNELS.contains(&literal) {
                            continue;
                        }
                        // Only warn for clearly-not-substituted literals.
                        if arg_text.contains('$') || arg_text.contains('[') {
                            continue;
                        }
                        let message = format!(
                            "String literal '{literal}' used as channel argument to \
                             '{command}' — expected a channel from open/socket/chan create.",
                        );
                        self.result.diagnostics.push(super::types::Diagnostic {
                            code: "W126".to_string(),
                            span: fu.abs_span(*span),
                            message,
                            severity: Severity::Warning,
                            fixes: Vec::new(),
                        });
                    }
                }
            }
        }
    }

    /// W124 — invalid IP address literal.
    ///
    /// Walks every
    /// SSA-tracked constant string in the function's SCCP
    /// values; regex-searches for IPv4 dotted-quad and IPv6
    /// candidates and validates each.
    ///
    /// **Validation:**
    /// - **IPv4** — each octet must be 0..255; leading-zero
    ///   octets emit a Warning (interpreted as octal in some
    ///   contexts); over-255 octets emit an Error.  Patterns
    ///   preceded by ``/`` (CIDR / version-number context) are
    ///   skipped.
    /// - **IPv6** — parsed via [`std::net::Ipv6Addr`]; failure
    ///   emits an Error.
    ///
    /// Diagnostic anchors at the SSA def site (the assignment
    /// statement's span); seen-offsets dedup avoids duplicate
    /// emissions when multiple SSA versions share a def.
    /// **W233.** Division / modulo by a provably-zero divisor — raises
    /// "divide by zero" at runtime.  Delegates to the canonical
    /// interval-bounds analysis [`crate::interval_bounds::find_divide_by_zero`]
    /// (the single source of truth, shared with the interval-bounds index
    /// checks): a `/` or `%` whose divisor's interval — guard-narrowed at the
    /// use site and seeded from the SCCP lattice — is exactly `[0, 0]`, on the
    /// always-evaluated spine of an executable expression.
    ///
    /// (Verified against tclsh 8.4–9.0: integer `1/0` and `5%0` raise "divide
    /// by zero"; float division such as `1.0/0` yields `Inf` and does not
    /// error. The interval domain is integer, matching that boundary for the
    /// common cases.)
    fn emit_w233_divide_by_zero(&mut self, fu: &crate::compilation_unit::FunctionUnit) {
        // The block set SCCP proved reachable; fall back to every SSA block
        // when SCCP produced nothing (e.g. a trivial function) so the check
        // still runs — matching the previous emitter's reachability fallback.
        let executable: HashSet<String> = if fu.sccp.executable_blocks.is_empty() {
            fu.ssa.blocks.keys().cloned().collect()
        } else {
            fu.sccp.executable_blocks.clone()
        };
        for finding in crate::interval_bounds::find_divide_by_zero(
            &fu.cfg,
            &fu.ssa,
            &fu.sccp.values,
            &executable,
        ) {
            let span = fu.abs_span(finding.span);
            if span.is_empty() {
                continue;
            }
            let verb = if finding.op == "/" {
                "Division"
            } else {
                "Modulo"
            };
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "W233".to_string(),
                span,
                message: format!(
                    "{verb} by a provably-zero divisor — raises 'divide by zero' at runtime."
                ),
                severity: Severity::Warning,
                fixes: Vec::new(),
            });
        }
    }

    /// **W230 / W231 / W232 (dynamic).** Interval-driven out-of-range index
    /// detection for a `$var` index whose [`crate::intervals`] range — guard-
    /// narrowed at the use site — proves the access is wholly out of range
    /// against a statically-established container length.  Complements the
    /// syntactic bounds checks (literal index + literal container only); the
    /// two never double-fire because the syntactic checks back off on any
    /// `$var` index.  Restricted to SCCP-reachable blocks so a dynamic index
    /// in dead code does not warn.
    fn emit_interval_bounds_diagnostics(&mut self, fu: &crate::compilation_unit::FunctionUnit) {
        let executable: HashSet<String> = if fu.sccp.executable_blocks.is_empty() {
            fu.ssa.blocks.keys().cloned().collect()
        } else {
            fu.sccp.executable_blocks.iter().cloned().collect()
        };
        let findings = crate::interval_bounds::find_interval_bounds(
            &fu.cfg,
            &fu.ssa,
            &fu.sccp.values,
            &executable,
        );
        for f in findings {
            if f.span.is_empty() {
                continue;
            }
            let bound = if f.reason == "negative" {
                "below 0".to_string()
            } else {
                format!("past the end ({})", f.length)
            };
            let rng = if f.reason == "negative" {
                "negative".to_string()
            } else if f.index_interval.lo == f.index_interval.hi {
                format!("is {}", f.index_interval.lo.map_or(0, |l| l))
            } else {
                let lo = f
                    .index_interval
                    .lo
                    .map_or("-inf".to_string(), |l| l.to_string());
                let hi = f
                    .index_interval
                    .hi
                    .map_or("+inf".to_string(), |h| h.to_string());
                format!("is in [{lo}, {hi}]")
            };
            let outcome = if f.code == "W231" {
                "raises 'index out of range' at runtime"
            } else {
                "silently returns the empty string"
            };
            self.result.diagnostics.push(super::types::Diagnostic {
                code: f.code,
                span: fu.abs_span(f.span),
                message: format!(
                    "{}: index ${} {rng}, {bound} \u{2014} {outcome}.",
                    f.command, f.index_var
                ),
                severity: Severity::Warning,
                fixes: Vec::new(),
            });
        }
    }

    fn emit_invalid_ip_diagnostics(&mut self, fu: &crate::compilation_unit::FunctionUnit) {
        use crate::analyses::{ConstValue, LatticeValue};
        use std::net::Ipv6Addr;
        use std::str::FromStr;

        let mut seen_offsets: FxHashSet<u32> = FxHashSet::default();
        for (key, lv) in &fu.sccp.values {
            let Some(text) = (match lv {
                LatticeValue::Const(ConstValue::String(s)) => Some(s.as_str()),
                _ => None,
            }) else {
                continue;
            };

            // ---- IPv4 candidates ----
            for quad in find_dotted_quads(text, 4) {
                let bytes = text.as_bytes();
                if quad.start > 0 && bytes[quad.start - 1] == b'/' {
                    continue;
                }
                // Skip OID-like patterns: the matched quad is a slice of a
                // longer dotted-digit chain (LDAP/SNMP OIDs like
                // ``1.3.6.1.4.1.4203.1.11.3``).  Detect a ``digit.<quad>``
                // before or a ``<quad>.digit`` after.
                let before_dot_digit = quad.start >= 2
                    && bytes[quad.start - 1] == b'.'
                    && bytes[quad.start - 2].is_ascii_digit();
                let after_dot_digit = quad.end + 1 < bytes.len()
                    && bytes[quad.end] == b'.'
                    && bytes[quad.end + 1].is_ascii_digit();
                if before_dot_digit || after_dot_digit {
                    continue;
                }
                let octets = quad.octets;
                let mut diag: Option<(String, Severity)> = None;
                for (i, octet) in octets.iter().enumerate() {
                    let v: u32 = octet.parse().unwrap_or(0);
                    if v > 255 {
                        diag = Some((
                            format!(
                                "IPv4 octet {} ({}) exceeds 255 — this is not a valid IP address.",
                                i + 1,
                                octet,
                            ),
                            Severity::Error,
                        ));
                        break;
                    }
                    if octet.len() > 1
                        && octet.starts_with('0')
                        && octet.bytes().all(|b| (b'0'..=b'7').contains(&b))
                    {
                        diag = Some((
                            format!(
                                "IPv4 octet {} ({}) has a leading zero — may be interpreted as octal in some contexts.",
                                i + 1,
                                octet,
                            ),
                            Severity::Warning,
                        ));
                        break;
                    }
                }
                if let Some((msg, sev)) = diag {
                    self.emit_ip_diag_at_def(fu, key, &msg, sev, &mut seen_offsets);
                    break;
                }
            }

            // ---- IPv6 candidates ----
            for candidate in find_ipv6_candidates(text) {
                if Ipv6Addr::from_str(candidate).is_err() {
                    let msg = format!("Invalid IPv6 address '{candidate}'.");
                    self.emit_ip_diag_at_def(fu, key, &msg, Severity::Error, &mut seen_offsets);
                    break;
                }
            }
        }
    }

    /// Helper for [`Self::emit_invalid_ip_diagnostics`].
    fn emit_ip_diag_at_def(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        key: &crate::ssa::ValueKey,
        message: &str,
        severity: Severity,
        seen_offsets: &mut FxHashSet<u32>,
    ) {
        let (var_name, version) = key;
        let Some(chain) = fu.def_use.chain_for(var_name, *version) else {
            return;
        };
        let Some(block) = fu.cfg.blocks.get(&chain.definition.block) else {
            return;
        };
        let Ok(idx) = usize::try_from(chain.definition.statement_index) else {
            return;
        };
        let Some(stmt) = block.statements.get(idx) else {
            return;
        };
        let span = fu.abs_span(stmt.span());
        if span.is_empty() {
            return;
        }
        if !seen_offsets.insert(span.start()) {
            return;
        }
        self.result.diagnostics.push(super::types::Diagnostic {
            code: "W124".to_string(),
            span,
            message: message.to_string(),
            severity,
            fixes: Vec::new(),
        });
    }

    /// W123 — unknown / unresolved command head.
    ///
    /// Walks every command invocation recorded during the
    /// analyser walk and emits W123 ("Unknown command 'X'")
    /// when no matching definition is in scope.
    ///
    /// Resolution paths checked in order — first match
    /// suppresses W123:
    ///
    /// - `cmd_name in registry_names` (built-in command).
    /// - `cmd_name` contains `::` (qualified — defer to
    ///   per-namespace logic, conservative skip).
    /// - `cmd_name` starts with `$` / `[` (interpolated /
    ///   substituted head — handled by W307 / W308).
    /// - User-defined proc tail or absolute name.
    /// - User-defined class tail or absolute name.
    /// - Command alias tail.
    /// - Ensemble namespace tail.
    ///
    /// Idempotency: ``self.unresolved_commands_emitted`` guards
    /// against double-emission when ``analyse`` is called twice
    /// or the chunked entry runs both passes.
    ///
    /// **Not yet implemented:** ``has_dynamic_providers`` early-return;
    /// the CONSTSET-driven interpolation suppression for
    /// ``$``-bearing command names.
    // Long-running analyser pass with many sequential phases over the CompilationUnit; splitting requires threading shared local state.
    #[allow(clippy::too_many_lines)]
    pub fn emit_unresolved_command_diagnostics(
        &mut self,
        registry: &tcl_registry::CommandRegistry,
    ) {
        if self.unresolved_commands_emitted {
            return;
        }
        self.unresolved_commands_emitted = true;
        // The W123 *diagnostic* honours `disabled_diagnostics`, but the
        // unresolved-command *call sites* are recorded regardless (below), so a
        // cross-file consumer can run its arity check independently of the W123
        // toggle.  The knowability gates that follow (dynamic `package require` /
        // `unknown` proc) still suppress both, since resolution is then unknown.
        let emit_w123 = !self.disabled_diagnostics.contains("W123");

        // Conservative gate: if any ``package require`` was seen,
        // suppress W123 entirely.  The package may load arbitrary
        // commands at runtime that the analyser can't see.
        if !self.result.package_requires.is_empty() {
            return;
        }

        // When the document defines a
        // user-level ``unknown`` proc with a *dynamic* dispatch
        // shape — chains the original handler, case-folds,
        // uses pattern (glob / regexp) dispatch, calls
        // ``exec``, or calls ``auto_load`` — the analyser can't
        // statically prove which commands are resolvable, so
        // suppress W123 entirely.  For the *non-dynamic* shape
        // (only explicit ``dispatch_targets`` listed), W123
        // still fires below; the per-invocation loop checks
        // ``dispatch_targets`` membership and lets unrelated
        // commands surface their warnings.  Empty-stub
        // ``unknown`` (``proc unknown {cmd args} {}``) resolves
        // nothing so we never hit this gate.
        if let Some(info) = self.result.unknown_proc_info.as_ref() {
            let is_dynamic = info.chains_original
                || info.case_insensitive
                || info.has_pattern_dispatch
                || info.has_exec
                || info.has_auto_load;
            if is_dynamic {
                return;
            }
        }

        // Only commands
        // *enabled in the active dialect* count as "known" for W123.  The
        // registry's `command_names()` returns every loaded spec —
        // including base tcl commands like `exec`/`glob` that `build_default`
        // loads but the active dialect (e.g. f5-irules) disables — so filter
        // by dialect support.  Without this, `exec`/`glob` under f5-irules
        // would draw W002 (disabled) but not the W123 (unknown-in-dialect)
        // that should also fire.  Base commands valid everywhere (`set`/`if`,
        // `dialects: None`) still pass `get_for_dialect`, so they are not
        // spuriously flagged.
        let active_dialect = tcl_registry::prelude::DialectSet::parse(&self.dialect)
            .unwrap_or(tcl_registry::prelude::DialectSet::ALL_TCL);
        let registry_names: HashSet<String> = registry
            .command_names()
            .filter(|name| registry.get_for_dialect(name, active_dialect).is_some())
            .map(str::to_string)
            .collect();
        // Inline ``# tcl-lsp: stub NAME ...``
        // declarations contribute to the candidate set and the
        // suppression set so users who declared a stub for a
        // command don't get spurious W123s.
        let stub_names: HashSet<String> = super::utils::scan_stub_command_names(&self.source);
        let proc_tail_names: HashSet<String> = self
            .result
            .all_procs
            .keys()
            .filter_map(|qn| qn.rsplit_once("::").map(|(_, t)| t.to_string()))
            .filter(|s| !s.is_empty())
            .collect();
        let class_tail_names: HashSet<String> = self
            .result
            .all_classes
            .keys()
            .filter_map(|qn| qn.rsplit_once("::").map(|(_, t)| t.to_string()))
            .filter(|s| !s.is_empty())
            .collect();
        let alias_names: HashSet<String> = self
            .result
            .command_aliases
            .keys()
            .filter_map(|qn| qn.rsplit_once("::").map(|(_, t)| t.to_string()))
            .filter(|s| !s.is_empty())
            .collect();
        let ensemble_cmds: HashSet<String> = self
            .ensemble_namespaces
            .iter()
            .filter_map(|ns| ns.rsplit_once("::").map(|(_, t)| t.to_string()))
            .filter(|s| !s.is_empty())
            .collect();

        // Build the candidate set for "did you mean…?"
        // suggestions — every name a real command
        // could resolve to (including unknown-proc dispatch
        // targets and inline-stub declarations).
        let mut candidates: Vec<String> = Vec::new();
        candidates.extend(registry_names.iter().cloned());
        candidates.extend(proc_tail_names.iter().cloned());
        candidates.extend(class_tail_names.iter().cloned());
        candidates.extend(alias_names.iter().cloned());
        candidates.extend(ensemble_cmds.iter().cloned());
        candidates.extend(stub_names.iter().cloned());
        if let Some(info) = self.result.unknown_proc_info.as_ref() {
            for t in &info.dispatch_targets {
                candidates.push(t.clone());
            }
        }

        // Pre-compute the deduplicated ``Vec<&str>`` over the
        // candidate set once, instead of rebuilding it per
        // unresolved invocation.  ``candidates`` may carry
        // duplicates because each contributor (registry / proc
        // tails / class tails / aliases / ensemble cmds /
        // stubs / unknown-proc dispatch_targets) is unioned
        // independently — dedupe via a ``HashSet`` filter
        // while preserving stable iteration order.
        let mut seen_candidate_strs: FxHashSet<&str> = FxHashSet::default();
        let candidate_strs: Vec<&str> = candidates
            .iter()
            .map(String::as_str)
            .filter(|candidate| seen_candidate_strs.insert(*candidate))
            .collect();

        // Drain so the iteration loop can mutate
        // ``self.result.diagnostics`` freely; restore at the end
        // (matches the snapshot/restore round-trip contract).
        let invocations = std::mem::take(&mut self.result.command_invocations);
        for inv in &invocations {
            let name = &inv.name;
            if registry_names.contains(name) {
                continue;
            }
            if name.contains("::") {
                continue;
            }
            if name.starts_with('$') || name.starts_with('[') {
                continue;
            }
            if proc_tail_names.contains(name) {
                continue;
            }
            if class_tail_names.contains(name) {
                continue;
            }
            if alias_names.contains(name) {
                continue;
            }
            if ensemble_cmds.contains(name) {
                continue;
            }
            if stub_names.contains(name) {
                continue;
            }
            if let Some(info) = self.result.unknown_proc_info.as_ref()
                && info.dispatch_targets.contains(name)
            {
                continue;
            }
            // Absolute-form fallback — ``cmd`` may be defined as
            // ``::cmd`` in the global namespace.
            if self.result.all_procs.contains_key(&format!("::{name}")) {
                continue;
            }
            if self.result.all_classes.contains_key(&format!("::{name}")) {
                continue;
            }

            // Unresolved.  Record the call site so a cross-file consumer can run
            // its arity check independently of the W123 toggle, then emit the W123
            // diagnostic unless it is disabled.
            self.result
                .unresolved_command_sites
                .push((inv.range, name.clone()));
            if !emit_w123 {
                continue;
            }

            // "Did you mean…?" suggestion
            // via Levenshtein (max 1 suggestion, max distance 2).
            // ``candidate_strs`` was
            // deduplicated above so every name in it is unique;
            // copying the slice per invocation is cheap (Vec of
            // ``&str`` references).
            let suggestions =
                crate::text::suggest_similar(name, candidate_strs.iter().copied(), 1, 2);
            let mut message = format!("Unknown command '{name}'");
            let mut fixes: Vec<super::types::CodeFix> = Vec::new();
            if let Some(best) = suggestions.first() {
                use std::fmt::Write as _;
                let _ = write!(message, "; did you mean '{best}'?");
                fixes.push(super::types::CodeFix {
                    span: inv.range,
                    new_text: (*best).to_string(),
                    description: format!("Replace with '{best}'"),
                });
            }
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "W123".to_string(),
                span: inv.range,
                message,
                severity: Severity::Hint,
                fixes,
            });
        }
        self.result.command_invocations = invocations;
    }

    /// W120 — command used without a corresponding
    /// `package require`.
    ///
    /// For every command
    /// invocation whose registry spec carries a
    /// `required_package`, emit W120 (once per command name)
    /// unless that package is already imported (a
    /// `package require` / `package provide` in this file).
    /// Attaches a `CodeFix` that inserts
    /// `package require <pkg>` after the last existing
    /// `package require`, or at the top of the file.
    ///
    /// Gated off entirely when:
    /// * the dialect has no `package` command (iRules);
    /// * the file loads packages dynamically
    ///   (`has_dynamic_providers`) — the runtime set of
    ///   commands is then unknowable;
    /// * W120 is in `disabled_diagnostics`.
    pub fn emit_missing_package_require_diagnostics(
        &mut self,
        registry: &tcl_registry::CommandRegistry,
    ) {
        if self.disabled_diagnostics.contains("W120") {
            return;
        }
        // Dialects without a `package` command (e.g. iRules)
        // can't `package require`, so W120 never applies.
        if registry.get("package").is_none() {
            return;
        }
        // Dynamic providers ⇒ unknowable command set ⇒ no W120.
        if self.result.has_dynamic_providers {
            return;
        }

        // Packages already available in this file: every
        // `package require` name plus every `package provide`
        // name (a file that provides a package needn't require
        // it).
        let mut imported: FxHashSet<&str> = FxHashSet::default();
        for pr in &self.result.package_requires {
            imported.insert(pr.name.as_str());
        }
        for pp in &self.result.package_provides {
            imported.insert(pp.name.as_str());
        }

        // Insertion point for the code fix: just after the last
        // `package require` line, else the top of the file.
        let insert_offset = self.package_require_insert_offset();

        // Emit once per command name, anchored at its **source-earliest**
        // invocation.  Selecting by position (rather than the first in
        // `command_invocations` iteration order) makes the result independent of
        // *how* the walk was driven — the whole-file DFS and the per-item
        // shell+graft order record invocations in different orders, but both
        // pick the same anchor here (the per-item path's `command_invocations`
        // is only sorted by `canonicalize_result_order`, which runs after this
        // emitter).  This keeps the result walk-strategy-independent, as the
        // tail already enforces for other order-sensitive collections.
        let mut best: HashMap<&str, &crate::signature_scan::types::SignatureCommandInvocation> =
            HashMap::new();
        for inv in &self.result.command_invocations {
            let Some(spec) = registry.get(&inv.name) else {
                continue;
            };
            if spec.required_package.is_none() {
                continue;
            }
            best.entry(inv.name.as_str())
                .and_modify(|cur| {
                    if (inv.range.start(), inv.range.end()) < (cur.range.start(), cur.range.end()) {
                        *cur = inv;
                    }
                })
                .or_insert(inv);
        }
        let mut new_diags: Vec<super::types::Diagnostic> = Vec::new();
        for inv in best.values() {
            let spec = registry
                .get(&inv.name)
                .expect("invocation selected only when registry-known");
            let pkg = spec
                .required_package
                .expect("invocation selected only when it requires a package");
            if imported.contains(pkg) {
                continue;
            }
            let fix = super::types::CodeFix {
                span: tcl_lexer::Span::new(insert_offset, insert_offset),
                new_text: format!("package require {pkg}\n"),
                description: format!("Add 'package require {pkg}'"),
            };
            new_diags.push(super::types::Diagnostic {
                code: "W120".to_string(),
                span: inv.range,
                message: format!("\"{}\" requires `package require {pkg}`", inv.name),
                severity: Severity::Warning,
                fixes: vec![fix],
            });
        }
        self.result.diagnostics.extend(new_diags);
    }

    /// Byte offset at which a `package require <pkg>` line
    /// should be inserted: just past the newline after the
    /// last existing `package require`, else `0` (top of
    /// file).
    fn package_require_insert_offset(&self) -> u32 {
        let Some(last) = self
            .result
            .package_requires
            .iter()
            .max_by_key(|p| p.range.end())
        else {
            return 0;
        };
        let bytes = self.source.as_bytes();
        let mut off = last.range.end() as usize;
        while off < bytes.len() && bytes[off] != b'\n' {
            off += 1;
        }
        if off < bytes.len() {
            off += 1; // past the newline
        }
        u32::try_from(off).unwrap_or(0)
    }

    /// True when `my <method>` / `self <method>` dispatched at `site_offset`
    /// resolves to a method in the enclosing class whose body is a simple
    /// `return <literal>` — i.e. it returns a plain string, not an object
    /// handle.  The enclosing class is the one whose `body_span` contains the
    /// dispatch offset; the method is looked up in its `methods` /
    /// `class_methods`.  A literal return is `return <word>` on a single line
    /// with no command substitution (`[`) or variable interpolation (`$`) in
    /// the returned word.
    fn oo_self_method_returns_literal(&self, site_offset: u32, method_name: &str) -> bool {
        for class_def in self.result.all_classes.values() {
            let body = class_def.body_span;
            if !(body.start() <= site_offset && site_offset <= body.end()) {
                continue;
            }
            let Some(md) = class_def
                .methods
                .get(method_name)
                .or_else(|| class_def.class_methods.get(method_name))
            else {
                // Enclosing class found but no such method — stay conservative
                // (treat as object-returning).
                return false;
            };
            let start = md.body_span.start() as usize;
            let end = (md.body_span.end() as usize).min(self.source.len());
            if start >= end {
                return false;
            }
            let mut bt = self.source[start..end].trim();
            // Strip one layer of surrounding braces.
            if let Some(inner) = bt.strip_prefix('{') {
                bt = inner.trim_end();
                bt = bt.strip_suffix('}').unwrap_or(bt).trim();
            }
            // Simple `return <literal>` — single statement, no substitutions.
            if bt.contains('\n') || bt.contains(';') {
                return false;
            }
            let Some(ret_arg) = bt.strip_prefix("return ") else {
                return false;
            };
            let ret_arg = ret_arg.trim();
            return !ret_arg.is_empty() && !ret_arg.contains('[') && !ret_arg.contains('$');
        }
        false
    }

    /// Harvest `set x [Cls new]` / `set x [Cls create name]` where `Cls` is a
    /// known `TclOO` class: `x` then holds an Object of class `Cls`, so a later
    /// `$x method` dispatch resolves through the W308 method check instead of
    /// firing W307.  The type lattice doesn't model the constructor return
    /// type for a var assignment yet (the cmd-site path recognises the
    /// bare-class `new`/`create` pattern directly), so mirror that recognition
    /// here for the var-assignment shape.
    fn harvest_constructor_object_types(
        &self,
        cu: &crate::compilation_unit::CompilationUnit,
        out: &mut HashMap<String, HashSet<String>>,
    ) {
        use crate::ir::Statement;
        let units = std::iter::once(&cu.top_level).chain(cu.procedures.values());
        for fu in units {
            for block in fu.cfg.blocks.values() {
                for stmt in &block.statements {
                    let Statement::AssignValue { name, value, .. } = stmt else {
                        continue;
                    };
                    let Some((head, args)) =
                        crate::value_shapes::parse_command_substitution(value.trim())
                    else {
                        continue;
                    };
                    if !args.first().is_some_and(|s| s == "new" || s == "create") {
                        continue;
                    }
                    let class_qn = self.canonicalise_class_name(&head);
                    if self.result.all_classes.contains_key(&class_qn)
                        || self.result.all_classes.contains_key(&head)
                    {
                        out.entry(name.clone()).or_default().insert(class_qn);
                    }
                }
            }
        }
    }

    /// Per-proc `(body_start, body_end, factory_local_vars)` ranges — the
    /// variables that hold an *object factory* result, so a `$var method`
    /// dispatch on them suppresses W307 (an object handle is a designed,
    /// non-literal command target, not a static error).
    ///
    /// A factory local is a `set X [head …]`
    /// where `head` is object-returning: a known `TclOO` class command, a
    /// namespaced factory (the documented tcllib `::ns::cmd` convention, minus
    /// known user procs and registry commands with a non-OBJECT return type),
    /// or another proc proven object-returning by the fixpoint.  This tracks
    /// *no* class identity — it only suppresses W307 (it never enables W308).
    #[allow(clippy::too_many_lines)]
    // A single fixpoint algorithm (classify heads → collect factory locals →
    // seed → propagate → extend → materialise ranges) whose phases share local
    // state; splitting it would only scatter that state behind extra args.
    fn compute_factory_object_ranges(
        &self,
        cu: &crate::compilation_unit::CompilationUnit,
        registry: &tcl_registry::CommandRegistry,
    ) -> Vec<(u32, u32, HashSet<String>)> {
        use crate::ir::Statement;

        let class_qnames: HashSet<&String> = self.result.all_classes.keys().collect();
        let class_tails: HashSet<&str> = class_qnames
            .iter()
            .filter_map(|qn| qn.rsplit_once("::").map(|(_, t)| t))
            .filter(|t| !t.is_empty())
            .collect();
        let is_user_proc = |head: &str| {
            self.result.all_procs.contains_key(head)
                || self.result.all_procs.contains_key(&format!("::{head}"))
        };
        // A command head whose value-returning invocation yields an object
        // handle (excluding user procs, which the fixpoint classifies).
        let is_object_returning_head = |head: &str| -> bool {
            if class_tails.contains(head) || class_qnames.contains(&format!("::{head}")) {
                return true;
            }
            if head.contains("::") {
                let qualified = if head.starts_with("::") {
                    head.to_string()
                } else {
                    format!("::{head}")
                };
                // A known user proc defers to the fixpoint (returns false here).
                if self.result.all_procs.contains_key(&qualified) {
                    return false;
                }
                // A registered command with an explicit non-OBJECT return type
                // (http::*, clock::*, …) is not a factory.
                if let Some(spec) = registry.get(head).or_else(|| registry.get(&qualified))
                    && let Some(rt) = spec.return_type
                    && rt != tcl_registry::TclType::Object
                {
                    return false;
                }
                // Unregistered `::pkg::cmd` — treat as a factory (the tcllib
                // convention; documented heuristic).
                return true;
            }
            false
        };

        // All analysable units: top level + procedures (not methods, which are
        // `in_method`-suppressed for W307 anyway).
        let units: Vec<(&str, &crate::compilation_unit::FunctionUnit)> =
            std::iter::once(("::top", &cu.top_level))
                .chain(cu.procedures.iter().map(|(q, fu)| (q.as_str(), fu)))
                .collect();

        // Per-proc: factory-local vars (non-user-proc factory heads), the last
        // returned var, and the `{var -> rhs command head}` assignment map.
        let mut factory_locals: FxHashMap<String, HashSet<String>> = FxHashMap::default();
        let mut return_var: FxHashMap<String, Option<String>> = FxHashMap::default();
        let mut assigns: FxHashMap<String, FxHashMap<String, String>> = FxHashMap::default();
        let mut object_returning: FxHashSet<String> = FxHashSet::default();
        for (qname, fu) in &units {
            let mut names = HashSet::new();
            let mut amap = FxHashMap::default();
            for block in fu.cfg.blocks.values() {
                for stmt in &block.statements {
                    let Statement::AssignValue { name, value, .. } = stmt else {
                        continue;
                    };
                    let Some((head, _)) =
                        crate::value_shapes::parse_command_substitution(value.trim())
                    else {
                        continue;
                    };
                    amap.insert(name.clone(), head.clone());
                    if is_object_returning_head(&head) && !is_user_proc(&head) {
                        names.insert(name.clone());
                    }
                }
            }
            factory_locals.insert((*qname).to_string(), names);
            assigns.insert((*qname).to_string(), amap);
            return_var.insert((*qname).to_string(), last_return_var_of(&fu.cfg));
            // Seed: a proc whose every return value is a namespaced
            // object-returning cmd-sub is itself object-returning (G4: ALL
            // returns must qualify, so a string-returning branch disqualifies).
            let rvs = return_values_of(&fu.cfg);
            if !rvs.is_empty()
                && rvs.iter().all(|rv| {
                    crate::value_shapes::parse_command_substitution(rv.trim())
                        .is_some_and(|(head, _)| is_object_returning_head(&head))
                })
            {
                object_returning.insert((*qname).to_string());
            }
        }
        // A proc returning one of its own factory locals is object-returning.
        for (qname, rv) in &return_var {
            if let Some(rv) = rv
                && factory_locals.get(qname).is_some_and(|s| s.contains(rv))
            {
                object_returning.insert(qname.clone());
            }
        }

        // Bare-name → qualified-name index for resolving relative call heads.
        let mut bare_to_qnames: FxHashMap<&str, Vec<&str>> = FxHashMap::default();
        for qname in cu.ir_module.procedures.keys() {
            let bare = qname.rsplit_once("::").map_or(qname.as_str(), |(_, t)| t);
            bare_to_qnames.entry(bare).or_default().push(qname.as_str());
        }
        let resolve_candidates = |head: &str| -> Vec<String> {
            let mut c = vec![head.to_string(), format!("::{head}")];
            if let Some(qs) = bare_to_qnames.get(head) {
                c.extend(qs.iter().map(|s| (*s).to_string()));
            }
            c
        };

        // Fixpoint: a proc whose returned var is assigned `[other]` where
        // `other` is a proven object-returning user proc is itself one.
        let mut changed = true;
        while changed {
            changed = false;
            for (qname, rv) in &return_var {
                let Some(rv) = rv else { continue };
                if object_returning.contains(qname) {
                    continue;
                }
                let Some(rhs) = assigns.get(qname).and_then(|m| m.get(rv)) else {
                    continue;
                };
                if resolve_candidates(rhs)
                    .iter()
                    .any(|c| object_returning.contains(c))
                {
                    object_returning.insert(qname.clone());
                    changed = true;
                }
            }
        }
        // Extend factory locals: `set X [user_proc]` where the proc is now
        // proven object-returning makes `X` a factory local too.
        for (qname, amap) in &assigns {
            let mut add = FxHashSet::default();
            for (var, head) in amap {
                if factory_locals.get(qname).is_some_and(|s| s.contains(var)) {
                    continue;
                }
                if resolve_candidates(head)
                    .iter()
                    .any(|c| object_returning.contains(c))
                {
                    add.insert(var.clone());
                }
            }
            factory_locals.entry(qname.clone()).or_default().extend(add);
        }

        // Materialise ranges (top level spans the whole source).
        let mut ranges = Vec::new();
        for (qname, names) in factory_locals {
            if names.is_empty() {
                continue;
            }
            if qname == "::top" {
                ranges.push((0, u32::MAX, names));
            } else if let Some(p) = cu.ir_module.procedures.get(&qname) {
                ranges.push((p.span.start(), p.span.end(), names));
            }
        }
        ranges
    }

    /// W307 — non-literal command name (variable / command-sub
    /// used as command head) and W308 (unknown method on object).
    ///
    /// Walks every recorded
    /// site in [`Self::var_command_sites`] / [`Self::cmd_command_sites`] and
    /// emits W307 unless the command head is statically resolvable to a finite
    /// set of known command names, an OBJECT of a known class (→ W308 method
    /// check), or a positive OO-dispatch signal (`$self`, `my`/`self`
    /// self-dispatch, namespaced ensemble, callback-array, dict-with unpack).
    #[allow(clippy::too_many_lines)]
    // Long-running analyser pass with many sequential phases over the CompilationUnit; splitting requires threading shared local state.
    fn emit_var_command_diagnostics(
        &mut self,
        cu: &crate::compilation_unit::CompilationUnit,
        registry: &tcl_registry::CommandRegistry,
    ) {
        use crate::types::TypeKind;
        use std::collections::HashMap;

        if self.var_command_sites.is_empty() && self.cmd_command_sites.is_empty() {
            return;
        }
        // Aggregate type-lattice knowledge per variable name
        // across every FunctionUnit.  For each var with a
        // ``TclType::Object`` lattice entry that has a
        // ``class_name``, record the class qualified name so
        // W308 can validate the method against the class
        // hierarchy.
        let mut all_object_types: HashMap<String, HashSet<String>> = HashMap::new();
        let collect_object_types =
            |types: &HashMap<crate::ssa::ValueKey, crate::types::TypeLattice>,
             out: &mut HashMap<String, HashSet<String>>| {
                for ((var_name, _ver), tl) in types {
                    if tl.kind != TypeKind::Known {
                        continue;
                    }
                    if !matches!(tl.tcl_type, Some(tcl_registry::TclType::Object)) {
                        continue;
                    }
                    let Some(class_name) = &tl.class_name else {
                        continue;
                    };
                    out.entry(var_name.clone())
                        .or_default()
                        .insert(class_name.clone());
                }
            };
        collect_object_types(&cu.top_level.types, &mut all_object_types);
        for fu in cu.procedures.values() {
            collect_object_types(&fu.types, &mut all_object_types);
        }
        self.harvest_constructor_object_types(cu, &mut all_object_types);

        // Build the class hierarchy once for W308 method
        // resolution (uses the ``ClassHierarchy``).
        let hierarchy = if self.result.all_classes.is_empty() {
            None
        } else {
            Some(super::class_hierarchy::build_class_hierarchy(
                self.result.all_classes.clone(),
            ))
        };

        // Aggregate constant-string knowledge per variable name
        // across every function in the CompilationUnit.  CONST and
        // CONSTSET are expanded into a flat set of values.
        let mut all_constsets: HashMap<String, HashSet<String>> = HashMap::new();
        let collect_from = |sccp: &crate::sccp::SccpResult,
                            out: &mut HashMap<String, HashSet<String>>| {
            for (key, lv) in &sccp.values {
                let (var_name, _ver) = key;
                let Some(values) = lattice_command_values(lv) else {
                    continue;
                };
                let entry = out.entry(var_name.clone()).or_default();
                for v in values {
                    entry.insert(v);
                }
            }
        };
        collect_from(&cu.top_level.sccp, &mut all_constsets);
        for fu in cu.procedures.values() {
            collect_from(&fu.sccp, &mut all_constsets);
        }

        harvest_array_set_constants(cu, &mut all_constsets);
        harvest_dict_with_constants(cu, &mut all_constsets);

        // Build the "known commands" universe — registry +
        // user-defined procs + class tail names.
        let known_cmds: HashSet<String> = registry.command_names().map(str::to_string).collect();
        let known_procs: HashSet<String> = self.result.all_procs.keys().cloned().collect();
        let known_proc_bare: HashSet<String> = known_procs
            .iter()
            .filter_map(|qn| qn.rsplit_once("::").map(|(_, tail)| tail.to_string()))
            .filter(|s| !s.is_empty())
            .collect();
        let known_class_tails: HashSet<String> = self
            .result
            .all_classes
            .keys()
            .filter_map(|qn| qn.rsplit_once("::").map(|(_, tail)| tail.to_string()))
            .filter(|s| !s.is_empty())
            .collect();

        let is_known_command = |v: &str| {
            known_cmds.contains(v)
                || known_procs.contains(v)
                || known_proc_bare.contains(v)
                || known_procs.contains(&format!("::{v}"))
                || known_class_tails.contains(v)
                || self.result.all_classes.contains_key(&format!("::{v}"))
        };

        // Per-SSA-version refinement: map each
        // function to its source range + FunctionUnit so the W307
        // suppression can read the value at the dispatch's *exact* SSA
        // use-version instead of the merged set.  ``::top`` covers the
        // whole source; a proc's narrower range wins where it contains
        // the offset.  Methods are ``in_method``-suppressed, so
        // they are left out.
        let mut func_ranges: Vec<(String, u32, u32)> = vec![("::top".to_string(), 0, u32::MAX)];
        let mut fu_by_qname: HashMap<String, &crate::compilation_unit::FunctionUnit> =
            HashMap::new();
        fu_by_qname.insert("::top".to_string(), &cu.top_level);
        for (qname, fu) in &cu.procedures {
            fu_by_qname.insert(qname.clone(), fu);
            if let Some(ir_proc) = cu.ir_module.procedures.get(qname) {
                func_ranges.push((qname.clone(), ir_proc.span.start(), ir_proc.span.end()));
            }
        }

        // Drain sites so we can borrow self.result mutably below.
        let sites = std::mem::take(&mut self.var_command_sites);
        let objdefined_vars = self.objdefined_vars.clone();
        // Object-factory locals: vars holding a factory result (`set x [Class
        // new]` / `set x [::ns::factory]` / `set x [object_returning_proc]`).
        // A `$x method` dispatch on one suppresses W307 (designed object usage).
        let factory_object_ranges = self.compute_factory_object_ranges(cu, registry);
        let is_factory_local = |var: &str, off: u32| -> bool {
            factory_object_ranges
                .iter()
                .any(|(s, e, names)| *s <= off && off <= *e && names.contains(var))
        };
        // Snit / OO instance-variable dispatch: `$mytree get` where `mytree` is
        // a class instance variable and the dispatch sits inside the class body
        // (including non-method helper `proc`s that `upvar` it). An instance var
        // holds a component / sub-object, so dispatching on it is designed usage
        // — suppress W307.  `snit_var_ranges` is built from every
        // `ClassDef`'s body span + declared `variables`.
        let snit_var_ranges: Vec<(u32, u32, &Vec<String>)> = self
            .result
            .all_classes
            .values()
            .filter(|cd| !cd.variables.is_empty())
            .map(|cd| (cd.body_span.start(), cd.body_span.end(), &cd.variables))
            .collect();
        let is_snit_member = |var: &str, off: u32| -> bool {
            snit_var_ranges
                .iter()
                .any(|(s, e, vars)| *s <= off && off <= *e && vars.iter().any(|v| v == var))
        };

        // **Proc-parameter / multi-dispatch object-dispatch suppression.**
        // A dispatch on a proc
        // *parameter* — `proc walk {tree} { $tree visit }` — is object
        // dispatch the user has documented as the proc's API contract, not a
        // static error.  A non-parameter local dispatched ≥2 times in the same
        // scope is likewise evidenced object usage (a single dispatch could be
        // a typo; repeated use is clearly designed).  Build, per enclosing
        // proc body, its parameter set and the per-var dispatch count, plus a
        // taint carve-out: a *tainted* var is never suppressed (dispatching a
        // user-controlled command name is an injection risk regardless of how
        // many times it appears).  `::top` is the sentinel for statements
        // outside any proc body.
        let mut proc_body_ranges: Vec<(u32, u32, String, HashSet<String>)> = self
            .result
            .all_procs
            .iter()
            .map(|(qname, pdef)| {
                let params: HashSet<String> = pdef.params.iter().map(|p| p.name.clone()).collect();
                (
                    pdef.body_span.start(),
                    pdef.body_span.end(),
                    qname.clone(),
                    params,
                )
            })
            .collect();
        // Innermost-enclosing wins: scan largest-start-first for a range that
        // contains the offset (procs don't nest, but `namespace eval` bodies
        // can wrap several, so this stays robust).  Returns the index into
        // `proc_body_ranges`, or `None` for the `::top` sentinel scope.
        proc_body_ranges.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
        let enclosing_idx = |off: u32| -> Option<usize> {
            proc_body_ranges
                .iter()
                .enumerate()
                .rev()
                .find(|(_, (s, e, _, _))| *s <= off && off <= *e)
                .map(|(i, _)| i)
        };
        let scope_qname = |idx: Option<usize>| -> &str {
            idx.map_or(W307_TOP_SCOPE, |i| proc_body_ranges[i].2.as_str())
        };
        let mut dispatch_counts: FxHashMap<(String, String), usize> = FxHashMap::default();
        for site in &sites {
            let qname = scope_qname(enclosing_idx(site.cmd_span.start()));
            *dispatch_counts
                .entry((qname.to_owned(), site.var_name.clone()))
                .or_insert(0) += 1;
        }
        // Per-scope tainted var names — any tainted SSA version of a name
        // disqualifies it from dispatcher-suppression.  Keyed by qname, with
        // `::top` for the top-level scope.
        let tainted_names_of = |fu: &crate::compilation_unit::FunctionUnit| -> HashSet<String> {
            fu.taints
                .iter()
                .filter(|(_, tl)| tl.is_tainted())
                .map(|((var, _ver), _)| var.clone())
                .collect()
        };
        let mut tainted_by_scope: FxHashMap<String, HashSet<String>> = FxHashMap::default();
        let top_tainted = tainted_names_of(&cu.top_level);
        if !top_tainted.is_empty() {
            tainted_by_scope.insert(W307_TOP_SCOPE.to_owned(), top_tainted);
        }
        for (qname, fu) in &cu.procedures {
            let names = tainted_names_of(fu);
            if !names.is_empty() {
                tainted_by_scope.insert(qname.clone(), names);
            }
        }

        for site in &sites {
            // **W308 path.**  Variable known to hold an Object
            // — validate the method name against the class
            // hierarchy.  When the method isn't found and the
            // class doesn't have an external superclass that
            // could carry it, emit W308.
            if let Some(class_names) = all_object_types.get(&site.var_name) {
                if let (Some(method_name), Some(hierarchy)) = (&site.method_name, &hierarchy) {
                    let mut found = false;
                    let mut has_local_class = false;
                    for cls in class_names {
                        if hierarchy.method_target(cls, method_name).is_some() {
                            found = true;
                            break;
                        }
                        if let Some(cd) = self.result.all_classes.get(cls) {
                            has_local_class = true;
                            if cd.methods.contains_key(method_name)
                                || cd.class_methods.contains_key(method_name)
                                || matches!(
                                    method_name.as_str(),
                                    "new" | "create" | "destroy" | "configure" | "cget"
                                )
                                || cd.methods.contains_key("unknown")
                            {
                                found = true;
                                break;
                            }
                        }
                    }
                    // Inherited ``unknown`` handler via MRO.
                    if !found && has_local_class {
                        for cls in class_names {
                            if hierarchy.method_target(cls, "unknown").is_some() {
                                found = true;
                                break;
                            }
                        }
                    }
                    // External superclass: a method might come
                    // from a class outside the current index.
                    if !found && has_local_class {
                        const OO_BASE: &[&str] = &["oo::object", "oo::class"];
                        'cls_loop: for cls in class_names {
                            if let Some(cd) = self.result.all_classes.get(cls) {
                                for s in &cd.superclasses {
                                    if !self.result.all_classes.contains_key(s)
                                        && !OO_BASE.contains(&s.as_str())
                                    {
                                        found = true;
                                        break 'cls_loop;
                                    }
                                }
                            }
                        }
                    }
                    // ``oo::objdefine`` adds per-instance
                    // methods we can't see at the class level.
                    if !found && objdefined_vars.contains(&site.var_name) {
                        found = true;
                    }
                    if !found && has_local_class && !self.disabled_diagnostics.contains("W308") {
                        let mut classes_sorted: Vec<&str> =
                            class_names.iter().map(String::as_str).collect();
                        classes_sorted.sort_unstable();
                        let cls_display = classes_sorted.join(", ");
                        let message =
                            format!("Unknown method '{method_name}' on class '{cls_display}'");
                        self.result.diagnostics.push(super::types::Diagnostic {
                            code: "W308".to_string(),
                            span: site.cmd_span,
                            message,
                            severity: Severity::Warning,
                            fixes: Vec::new(),
                        });
                    }
                }
                // W307 path doesn't fire when the var is a
                // known Object — the method-name check is the
                // load-bearing piece.
                continue;
            }

            // **W307 path.**  Variable not a known Object.
            // ``in_method`` short-circuits W307 because OO
            // methods routinely use ``$obj method`` patterns.
            // The analyser doesn't track method context
            // yet (pending a Method scope kind),
            // so this filter currently always falls through.
            if site.in_method {
                continue;
            }
            // Prefer the value at the dispatch's exact SSA use-version;
            // fall back to the merged constset when no precise version
            // is found. This drops the merged-set false positive on a
            // variable reassigned from a non-command to a known command
            // before the dispatch (`set c x; set c puts; $c ...`).
            let precise = w307_precise_cmd_values(
                &func_ranges,
                &fu_by_qname,
                site.cmd_span.start(),
                &site.var_name,
            );
            let effective = precise
                .as_ref()
                .or_else(|| all_constsets.get(&site.var_name));
            if let Some(values) = effective
                && !values.is_empty()
                && values.iter().all(|v| is_known_command(v))
            {
                continue;
            }
            // Proc-parameter / multi-dispatch object-dispatch suppression: a
            // dispatch on a parameter of the enclosing proc (any count), or on
            // a non-parameter local dispatched ≥2 times in the same scope, is
            // evidenced object usage — suppress unless the var is tainted.
            let idx = enclosing_idx(site.cmd_span.start());
            let encl_qname = scope_qname(idx);
            let is_param = idx.is_some_and(|i| proc_body_ranges[i].3.contains(&site.var_name));
            let dispatch_count = dispatch_counts
                .get(&(encl_qname.to_owned(), site.var_name.clone()))
                .copied()
                .unwrap_or(0);
            let dispatcher_suppressed = is_param || dispatch_count >= 2;
            let tainted = tainted_by_scope
                .get(encl_qname)
                .is_some_and(|s| s.contains(&site.var_name));
            if dispatcher_suppressed && !tainted {
                continue;
            }
            // Namespaced-ensemble dispatch: `${ns}::tail` / `$ns::tail` where
            // `ns` holds a namespace prefix and `::tail` composes a qualified
            // command path (tcllib's logger / dns / irc modules use this).
            // When the prefix is an SCCP const and *every* composed name
            // `<value>::tail` resolves to a known command/proc/class, the
            // dispatch is statically resolvable — suppress.  A composition
            // that resolves to nothing (unknown proc) still fires.
            if let Some((prefix, tail)) = parse_namespaced_ensemble(&self.source, site.cmd_span)
                && let Some(values) = all_constsets.get(&prefix)
                && !values.is_empty()
                && values
                    .iter()
                    .all(|v| is_known_command(&format!("{v}::{tail}")))
            {
                continue;
            }
            // Object-factory provenance: `$var` holds a factory result in this
            // scope — a designed object handle, so the dispatch is not a static
            // error. W307 exemption.
            if is_factory_local(&site.var_name, site.cmd_span.start()) {
                continue;
            }
            // Class instance-variable dispatch inside the class body (component
            // / sub-object) — W307 exemption.
            if is_snit_member(&site.var_name, site.cmd_span.start()) {
                continue;
            }
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "W307".to_string(),
                span: site.cmd_span,
                message: "Non-literal command name — cannot statically analyze".to_string(),
                severity: Severity::Warning,
                fixes: Vec::new(),
            });
        }
        // Restore the sites list — snapshot/restore expects it
        // to round-trip independently of emission.
        self.var_command_sites = sites;

        // ``[cmd] method`` sites — emit
        // W307 only when the inner command's return type is
        // unknown AND the call isn't an OO self-dispatch
        // (``my`` / ``self``).  When the return type is a
        // known class, validate the method against the
        // hierarchy and emit W308 instead of W307.
        let cmd_sites = std::mem::take(&mut self.cmd_command_sites);
        for site in &cmd_sites {
            // No blanket `in_method` suppression: an in-method `[cmd] method`
            // dispatch must earn its silence from a positive signal (a known
            // OBJECT return type, or `my`/`self` self-dispatch resolving to a
            // method that returns an object).
            //
            // Parse the command-substitution text into
            // ``head ?args...``.  ``cmd_text`` is what the
            // analyser captured from
            // ``SourceMap::token_text``; the leading ``[`` /
            // trailing ``]`` are stripped already because
            // ``content_offset`` skipped them.
            let inner = site.cmd_text.trim();
            let inner = inner
                .strip_prefix('[')
                .map_or(inner, str::trim)
                .strip_suffix(']')
                .map_or(inner, str::trim);
            let mut parts = inner.split_whitespace();
            let Some(head) = parts.next() else {
                continue;
            };
            let arg_strs: Vec<&str> = parts.collect();

            // OO self-dispatch (`my <method>` / `self <method>`): by default
            // the return is treated as an object handle (suppress).  But when
            // the dispatched method resolves in the enclosing class and its
            // body is a simple `return <literal>`, the result is a plain
            // string, not an object — so the *outer* dispatch fires W307.
            if matches!(head, "my" | "self") {
                let returns_literal = arg_strs.first().is_some_and(|method| {
                    self.oo_self_method_returns_literal(site.cmd_span.start(), method)
                });
                if returns_literal {
                    self.result.diagnostics.push(super::types::Diagnostic {
                        code: "W307".to_string(),
                        span: site.cmd_span,
                        message: "Non-literal command name — cannot statically analyze".to_string(),
                        severity: Severity::Warning,
                        fixes: Vec::new(),
                    });
                }
                continue;
            }

            // ``[Dog new]`` / ``[Dog create
            // name]`` produce an Object whose class is ``Dog``.
            // The registry lookup for the bare class name
            // returns Overdefined (the class isn't a built-in
            // command) so we recognise the constructor pattern
            // explicitly here — ``known_class new/create`` maps to
            // ``TclType.OBJECT`` with the class name attached.
            let class_qn = self.canonicalise_class_name(head);
            let head_is_known_class = self.result.all_classes.contains_key(&class_qn)
                || self.result.all_classes.contains_key(head);
            let is_constructor_call = head_is_known_class
                && arg_strs
                    .first()
                    .is_some_and(|sub| matches!(*sub, "new" | "create"));

            // Look up the return type via the registry.  When
            // the head is a user proc / class, fall back to
            // ``Overdefined`` (matches the registry behaviour
            // for unknown commands).
            let ret_type = if is_constructor_call {
                crate::types::TypeLattice {
                    kind: crate::types::TypeKind::Known,
                    tcl_type: Some(tcl_registry::TclType::Object),
                    from_type: None,
                    class_name: Some(class_qn.clone()),
                }
            } else {
                // The constructor case is already handled inline above using the
                // analyser's authoritative class set, so the registry fallback
                // only needs to recognise registered built-ins here — pass an
                // empty class set / root namespace.
                crate::type_infer::return_type_for_command(
                    registry,
                    head,
                    &arg_strs,
                    &std::collections::HashSet::new(),
                    "::",
                )
            };

            // ``Object`` return type — suppress W307; if the
            // class is known, validate the method (W308).
            let is_object = ret_type.kind == crate::types::TypeKind::Known
                && matches!(ret_type.tcl_type, Some(tcl_registry::TclType::Object));
            if is_object {
                if !self.disabled_diagnostics.contains("W308")
                    && let (Some(method), Some(class_name)) =
                        (site.method_name.as_ref(), ret_type.class_name.as_ref())
                {
                    let cls_qn = self.canonicalise_class_name(class_name);
                    let cd = self.result.all_classes.get(&cls_qn).cloned();
                    let method_ok = self.validate_method_on_class(
                        &cls_qn,
                        method,
                        cd.as_ref(),
                        hierarchy.as_ref(),
                    );
                    if !method_ok {
                        self.result.diagnostics.push(super::types::Diagnostic {
                            code: "W308".to_string(),
                            span: site.cmd_span,
                            message: format!("Unknown method '{method}' on class '{class_name}'"),
                            severity: Severity::Warning,
                            fixes: Vec::new(),
                        });
                    }
                }
                continue;
            }

            // Type is unknown — emit W307 (only the emit-half
            // for the residual unknown-type case).
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "W307".to_string(),
                span: site.cmd_span,
                message: "Non-literal command name — cannot statically analyze".to_string(),
                severity: Severity::Warning,
                fixes: Vec::new(),
            });
        }
        self.cmd_command_sites = cmd_sites;
    }

    /// Resolve a possibly-bare class name to its fully-qualified
    /// form keyed in `result.all_classes`.
    fn canonicalise_class_name(&self, name: &str) -> String {
        if name.starts_with("::") {
            return name.to_string();
        }
        let qualified = format!("::{name}");
        if self.result.all_classes.contains_key(&qualified) {
            qualified
        } else {
            name.to_string()
        }
    }

    /// Decide whether `method` is callable on `class_name`,
    /// consulting the class hierarchy + the class's local
    /// method tables.
    ///
    /// A method is OK when
    /// the class's MRO produces a concrete provider, or the
    /// class is external (no local `ClassDef`), or the method
    /// is one of the OO standard hooks (``new`` / ``create`` /
    /// ``destroy`` / ``configure`` / ``cget``), or the class
    /// declares an ``unknown`` method, or the class extends an
    /// external superclass we can't introspect.
    fn validate_method_on_class(
        &self,
        class_name: &str,
        method: &str,
        cd: Option<&super::types::ClassDef>,
        hierarchy: Option<&super::class_hierarchy::ClassHierarchy>,
    ) -> bool {
        if hierarchy.is_some_and(|h| h.method_target(class_name, method).is_some()) {
            return true;
        }
        let Some(cd) = cd else {
            // External class — can't validate.
            return true;
        };
        if cd.methods.contains_key(method) || cd.class_methods.contains_key(method) {
            return true;
        }
        if matches!(method, "new" | "create" | "destroy" | "configure" | "cget") {
            return true;
        }
        if cd.methods.contains_key("unknown") {
            return true;
        }
        if hierarchy.is_some_and(|h| h.method_target(class_name, "unknown").is_some()) {
            return true;
        }
        // External superclass ⇒ skip W308.
        if !cd.superclasses.is_empty() {
            for s in &cd.superclasses {
                if !self.result.all_classes.contains_key(s) && !OO_BASE.contains(&s.as_str()) {
                    return true;
                }
            }
        }
        false
    }

    /// Suppress W123 diagnostics whose command-name contains a
    /// `$` interpolation that resolves cleanly via SCCP.
    ///
    /// Walks every emitted W123, extracts the command name
    /// from the message, and runs
    /// [`crate::text::fold_interpolation_set`] over the
    /// aggregated SCCP results.  When every resolved value is
    /// a known command, proc, class, or class-tail name, the
    /// W123 is removed.
    ///
    /// **Simplification.**  This uses the union of
    /// every function's SCCP — slightly more permissive
    /// (over-suppresses if a same-named variable in a
    /// different function happens to resolve cleanly) but
    /// safe in practice.  Range-based per-function lookup
    /// could be added later.
    fn resolve_interpolated_w123_diagnostics(
        &mut self,
        cu: &crate::compilation_unit::CompilationUnit,
    ) {
        use crate::analyses::{ConstValue, LatticeValue};
        use std::collections::HashMap;

        // Bail early when no W123 carries a ``$`` — the common
        // case for non-iRules code.
        let has_interpolated = self
            .result
            .diagnostics
            .iter()
            .any(|d| d.code == "W123" && d.message.contains('$'));
        if !has_interpolated {
            return;
        }

        // Aggregate SCCP-resolved string sets per variable name
        // across every function in the CU.  Same shape as
        // ``emit_var_command_diagnostics``.
        let mut all_constsets: HashMap<String, HashSet<String>> = HashMap::new();
        let collect_from = |sccp: &crate::sccp::SccpResult,
                            out: &mut HashMap<String, HashSet<String>>| {
            for ((var_name, _ver), lv) in &sccp.values {
                let values: Option<Vec<String>> = match lv {
                    LatticeValue::Const(ConstValue::String(s)) => Some(vec![s.clone()]),
                    LatticeValue::ConstSet(set) => set
                        .iter()
                        .map(|cv| match cv {
                            ConstValue::String(s) => Some(s.clone()),
                            _ => None,
                        })
                        .collect::<Option<Vec<_>>>(),
                    _ => None,
                };
                let Some(values) = values else { continue };
                let entry = out.entry(var_name.clone()).or_default();
                for v in values {
                    entry.insert(v);
                }
            }
        };
        collect_from(&cu.top_level.sccp, &mut all_constsets);
        for fu in cu.procedures.values() {
            collect_from(&fu.sccp, &mut all_constsets);
        }

        // Build the universe of names that count as "known
        // commands" for the resolution check.  Same set the
        // emitter used to skip suggestions in the first pass.
        let registry = tcl_registry::CommandRegistry::build_default();
        let known_cmds: HashSet<String> = registry.command_names().map(str::to_string).collect();
        let known_proc_tails: HashSet<String> = self
            .result
            .all_procs
            .keys()
            .filter_map(|qn| qn.rsplit_once("::").map(|(_, t)| t.to_string()))
            .filter(|s| !s.is_empty())
            .collect();

        // Walk W123 diagnostics and remove those whose
        // interpolated command name resolves cleanly.
        let drained = std::mem::take(&mut self.result.diagnostics);
        let mut kept: Vec<super::types::Diagnostic> = Vec::with_capacity(drained.len());
        for d in drained {
            if d.code != "W123" {
                kept.push(d);
                continue;
            }
            let Some(cmd_name) = extract_quoted_word(&d.message) else {
                kept.push(d);
                continue;
            };
            if !cmd_name.contains('$') {
                kept.push(d);
                continue;
            }
            let Some(resolved) = crate::text::fold_interpolation_set(&cmd_name, &all_constsets)
            else {
                kept.push(d);
                continue;
            };
            // All resolved candidates must be known commands.
            let all_known = resolved.iter().all(|name| {
                known_cmds.contains(name)
                    || known_proc_tails.contains(name)
                    || self.result.all_procs.contains_key(&format!("::{name}"))
                    || self.result.all_procs.contains_key(name)
            });
            if all_known {
                // Suppress this W123 — the interpolated head
                // statically resolves to a known command set.
                continue;
            }
            kept.push(d);
        }
        self.result.diagnostics = kept;
    }

    /// Drop exact-duplicate diagnostics + line-based suppression
    /// pairs.
    ///
    /// Two passes:
    ///
    /// 1. Compute the set of source lines on which `E101`
    ///    (missing-open-brace) and `W124` (SSA-based IP check)
    ///    fired.  These are sentinels for the related
    ///    redundant-message codes.
    /// 2. Walk diagnostics in source order, deduplicating by
    ///    `(code, span, message, severity)` and dropping:
    ///    - `E002` on a line where `E101` fired (the recovered
    ///      switch makes the arity message a false positive).
    ///    - `W122` on a line where `W124` fired (the SSA check
    ///      is more precise).
    ///
    /// Lines come from the [`SourceMap`] over `self.source`.
    pub fn dedupe_diagnostics(&mut self) {
        let sm = SourceMap::new(&self.source);
        let mut e101_lines: FxHashSet<u32> = FxHashSet::default();
        let mut w124_lines: FxHashSet<u32> = FxHashSet::default();
        for d in &self.result.diagnostics {
            let line = sm.range_positions(d.span).0.line;
            match d.code.as_str() {
                "E101" => {
                    e101_lines.insert(line);
                }
                "W124" => {
                    w124_lines.insert(line);
                }
                _ => {}
            }
        }

        let mut seen: FxHashSet<(String, u32, u32, String, Severity)> = FxHashSet::default();
        let drained = std::mem::take(&mut self.result.diagnostics);
        let mut deduped = Vec::with_capacity(drained.len());
        for d in drained {
            let key = (
                d.code.clone(),
                d.span.start(),
                d.span.end(),
                d.message.clone(),
                d.severity,
            );
            if seen.contains(&key) {
                continue;
            }
            let line = sm.range_positions(d.span).0.line;
            if d.code == "E002" && e101_lines.contains(&line) {
                continue;
            }
            if d.code == "W122" && w124_lines.contains(&line) {
                continue;
            }
            seen.insert(key);
            deduped.push(d);
        }
        self.result.diagnostics = deduped;

        // Canonical, deterministic order. The post-walk emitters
        // (`emit_variable_usage_diagnostics` etc.) iterate the scope tree's
        // `HashMap`s, whose per-instance iteration order is non-deterministic —
        // so emission order varied run-to-run and, critically, between
        // `analyse` and `analyse_commands` (the per-item incremental path).
        // That non-determinism meant the multiset always
        // matched; only the `Vec` order differed. Sorting by source position
        // here makes the output deterministic and path-independent — required
        // for `incremental == fresh`, and a saner source-ordered contract for
        // the LSP. Dedupe above guarantees `(code, start, end, message,
        // severity)` is unique, so this key is a total order (no ties).
        self.result.diagnostics.sort_by(|a, b| {
            a.span
                .start()
                .cmp(&b.span.start())
                .then(a.span.end().cmp(&b.span.end()))
                .then_with(|| a.code.cmp(&b.code))
                .then_with(|| a.severity.as_str().cmp(b.severity.as_str()))
                .then_with(|| a.message.cmp(&b.message))
        });
    }

    /// Filter out diagnostics whose codes are in
    /// [`Self::disabled_diagnostics`].
    ///
    /// Centralising the filter on the orchestrator
    /// side keeps the per-emitter code
    /// from having to thread the check at every emit site —
    /// emitters can push freely and the orchestrator drops the
    /// silenced codes at the end.
    ///
    /// Idempotent on an empty filter set (no allocations).
    pub fn apply_disabled_diagnostics(&mut self) {
        if self.disabled_diagnostics.is_empty() {
            return;
        }
        // Borrow-checker dance: `retain` closure can't capture
        // `&self.disabled_diagnostics` while ``self.result`` is
        // mut-borrowed; clone the set into a local first.  The
        // disabled set is small (LSP-config-scale) so the clone
        // cost is negligible vs. the rest of the diagnostics
        // pipeline.
        let disabled = self.disabled_diagnostics.clone();
        self.result
            .diagnostics
            .retain(|d| !disabled.contains(&d.code));
    }

    /// IRULE4005 — racy ``static::`` cross-event flow.
    ///
    /// Walks every
    /// SSA statement in `fu` and emits IRULE4005 for any
    /// non-``unset`` def of a name in `racy_vars`.
    /// `racy_vars` comes from
    /// [`crate::connection_scope::ConnectionScope::racy_static_defs`]
    /// — built once per `CompilationUnit` and shared by every
    /// ``::when::*`` proc except `RULE_INIT`.
    fn emit_racy_static_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        racy_vars: &HashSet<String>,
    ) {
        if self.disabled_diagnostics.contains("IRULE4005") {
            return;
        }
        let mut emitted_spans: FxHashSet<u32> = FxHashSet::default();
        for block in fu.ssa.blocks.values() {
            for stmt in &block.statements {
                // Skip unset — not a real write.
                if let crate::ir::Statement::Call { command, .. } = &stmt.statement
                    && command == "unset"
                {
                    continue;
                }
                for name in stmt.defs.keys() {
                    if !racy_vars.contains(name) {
                        continue;
                    }
                    let span = fu.abs_span(stmt.statement.span());
                    if span.is_empty() || !emitted_spans.insert(span.start()) {
                        continue;
                    }
                    let message = format!(
                        "Potential race: '{name}' is written outside RULE_INIT and read in \
                         another event. static:: variables persist across all connections on \
                         the same virtual server; concurrent writes can produce unpredictable \
                         results."
                    );
                    self.result.diagnostics.push(super::types::Diagnostic {
                        code: "IRULE4005".to_string(),
                        span,
                        message,
                        severity: Severity::Warning,
                        fixes: Vec::new(),
                    });
                }
            }
        }
    }
}

/// Expand a CONST / CONSTSET lattice value into the flat set of its
/// string values, or `None` for any non-string-constant lattice state.
fn lattice_command_values(lv: &crate::analyses::LatticeValue) -> Option<Vec<String>> {
    use crate::analyses::{ConstValue, LatticeValue};
    match lv {
        LatticeValue::Const(ConstValue::String(s)) => Some(vec![s.clone()]),
        LatticeValue::ConstSet(set) => set
            .iter()
            .map(|cv| match cv {
                ConstValue::String(s) => Some(s.clone()),
                _ => None,
            })
            .collect::<Option<Vec<_>>>(),
        _ => None,
    }
}

/// The SCCP value set of `var_name` at the SSA use-version that reaches
/// the dispatch statement at `offset` (W307 per-SSA-version refinement).
///
/// The merged `all_constsets` map unions every version of a variable,
/// so `set c notacommand; set c parse; $c x` wrongly keeps
/// `notacommand` in the set even though only the `parse` version
/// reaches the dispatch. Reading the value at the use site's exact
/// version removes that false positive.
///
/// Purely additive: returns a set only when a CFG statement containing
/// `offset` that *uses* `var_name` is found and its version has a
/// concrete CONST / CONSTSET value — otherwise `None`, and the caller
/// falls back to the merged-set logic. Never broadens a fire into a
/// suppression unsoundly — the value is the exact one flowing into the
/// dispatch.
fn w307_precise_cmd_values(
    func_ranges: &[(String, u32, u32)],
    fu_by_qname: &std::collections::HashMap<String, &crate::compilation_unit::FunctionUnit>,
    offset: u32,
    var_name: &str,
) -> Option<HashSet<String>> {
    // Narrowest function range containing `offset`.
    let mut best: Option<(u32, &str)> = None;
    for (qname, start, end) in func_ranges {
        if *start <= offset && offset <= *end {
            let width = end - start;
            if best.is_none_or(|(bw, _)| width < bw) {
                best = Some((width, qname.as_str()));
            }
        }
    }
    let fu = fu_by_qname.get(best?.1)?;

    // Narrowest CFG statement containing `offset` that uses `var_name`,
    // reading its SSA use-version (CFG / SSA blocks are parallel-indexed).
    let mut best_width: Option<u32> = None;
    let mut best_version: Option<u32> = None;
    for (block_name, block) in &fu.cfg.blocks {
        let Some(ssa_block) = fu.ssa.blocks.get(block_name) else {
            continue;
        };
        for (idx, stmt) in block.statements.iter().enumerate() {
            let span = fu.abs_span(stmt.span());
            if !(span.start() <= offset && offset <= span.end()) {
                continue;
            }
            let Some(ssa_stmt) = ssa_block.statements.get(idx) else {
                continue;
            };
            let Some(version) = ssa_stmt.uses.get(var_name) else {
                continue;
            };
            let width = span.end() - span.start();
            if best_width.is_none_or(|bw| width < bw) {
                best_width = Some(width);
                best_version = Some(*version);
            }
        }
    }
    let version = best_version?;
    let lv = fu.sccp.values.get(&(var_name.to_string(), version))?;
    Some(lattice_command_values(lv)?.into_iter().collect())
}

#[cfg(test)]
mod tests;
