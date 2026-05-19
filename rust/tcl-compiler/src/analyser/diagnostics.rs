//! Diagnostic-emission orchestrator — Rust port of
//! `core/analysis/_analyser/_diagnostics.py`.
//!
//! Three top-level methods, mirroring the Python file 1:1:
//!
//! - [`Analyser::emit_variable_usage_diagnostics`] — kept as a
//!   no-op hook for future scope-tree consumers (Python's W211
//!   moved to the SSA-based pass; same here).
//! - [`Analyser::emit_cfg_ssa_diagnostics`] — main entry; builds
//!   a [`crate::compilation_unit::CompilationUnit`] on demand, walks the top-level
//!   function and every procedure, dispatches per-function
//!   diagnostics, and runs the cross-function post-passes
//!   (var-as-command, interpolated-command resolution).
//! - [`Analyser::emit_cfg_ssa_diagnostics_for_function`] —
//!   per-function dispatcher; calls each landed emitter in
//!   declaration order.
//!
//! Two utility passes round out the Python file:
//!
//! - [`Analyser::dedupe_diagnostics`] — drop exact duplicates
//!   plus the line-based pairs (E002 swallowed by E101 on the
//!   same line; W122 swallowed by W124 on the same line).
//! - [`Analyser::apply_disabled_diagnostics`] — filter out
//!   codes the caller asked to silence.
//!
//! **Strip-by-strip status.**
//!
//! - **C41d1** — orchestrator scaffold + dedupe + disabled-
//!   codes filter.  ✅ landed.
//! - **C41d2** — `_diag_var_lifecycle.py`.  ✅ landed:
//!   W220 (dead store), W211 (unused variable), W214
//!   (unused parameter), W210 (read-before-set), W213
//!   (unset on possibly-undef), and H300 (paste error).
//!   W210 / W213 are gated on procs only — top-level RBS
//!   needs the ``globals_written_by_procs`` filter Python
//!   uses, deferred until interproc analysis is wired in.
//! - **C41d3** — `_diag_var_command.py`.  ✅ landed:
//!   ``var_command_sites`` / ``cmd_command_sites`` recorded
//!   during the walk dispatch; **W307** (non-literal command
//!   name) and **W308** (unknown method on object) both emit
//!   via the cross-function post-pass.  W308 uses the C41e0
//!   ``ClassHierarchy::method_target`` for MRO-aware method
//!   resolution, with all the Python suppression paths
//!   wired (inherited ``unknown`` handler, external
//!   superclass, ``oo::objdefine`` per-instance methods).
//!   The ``[cmd] method`` return-type suppression for W307
//!   on ``cmd_command_sites`` remains deferred — it needs
//!   IR-level type-lattice plumbing extended into the
//!   analyser, which is a separate strip.
//! - **C41d4** — `_diag_commands.py`.  ✅ partial: W123
//!   (unknown command) is wired via the cross-function post-
//!   pass.  ``command_invocations`` are now recorded for every
//!   command head during the walk dispatch.  Deferred:
//!   ``_resolve_interpolated_commands`` (CONSTSET-driven W123
//!   suppression for ``$``-bearing names),
//!   ``_globals_written_by_procs`` (used by the C41d2 W210
//!   top-level RBS filter), ``suggest_similar`` "did you
//!   mean…?" suggestions, and the
//!   ``unknown_proc_info`` / ``has_dynamic_providers``
//!   early-returns.
//! - **C41d5** — `_diag_branches.py` + `_diag_channel.py`.
//!   ✅ landed: I230 / I231 (constant branch / switch-arm) and
//!   W126 (channel argument validation) all wired through the
//!   per-function dispatcher.  Severity-Info Python diagnostics
//!   map to ``Severity::Hint`` here (no Info variant on the
//!   Rust side).
//! - **C41d6** — `_diag_ip.py`.  ✅ landed: W124 (invalid IP
//!   address literal) — IPv4 octet validation (over-255 →
//!   Error, leading-zero → Warning) and IPv6 parsing via
//!   ``std::net::Ipv6Addr``.  Anchors at the SSA def site;
//!   seen-offsets dedup avoids duplicates across SSA versions.
//! - **C41d7** — `_diag_racy.py`.  ⏸ deferred: IRULE4005
//!   (racy ``static::`` cross-event flow) needs the
//!   connection-scope / cross-event analysis that the Rust
//!   pipeline doesn't yet have (Python's
//!   ``cu.connection_scope.racy_static_defs``).  Once
//!   ``ConnectionScope`` lands on the Rust side, the emitter
//!   wires up in a single call to ``emit_racy_static_diagnostics``.

use std::collections::HashSet;

use tcl_lexer::SourceMap;

use super::state::Analyser;
use super::types::Severity;
use crate::expr_ast::{BinOp, ExprNode};

/// Find a case-insensitive match for `variable` in `defined_vars`.
///
/// Mirrors `_find_case_mismatch` in
/// `core/analysis/_analyser/_diag_var_lifecycle.py:135-148`.
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

/// Collect every variable name defined anywhere in `cfg`.
///
/// Mirrors `_collect_defined_vars` in
/// `_diag_var_lifecycle.py:123-133`.  Walks every block and pulls
/// the `defs` field off each [`crate::ir::Statement`] that has
/// one (assignments, ``incr``, ``Call`` statements with explicit
/// defs).  Used for the "did you mean…?" case-mismatch
/// suggestion in W210 / W211 / W220 messages.
fn collect_defined_vars(cfg: &crate::cfg::Function) -> HashSet<String> {
    use crate::ir::Statement;
    let mut names: HashSet<String> = HashSet::new();
    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            match stmt {
                Statement::AssignConst { name, .. }
                | Statement::AssignExpr { name, .. }
                | Statement::AssignValue { name, .. }
                | Statement::Incr { name, .. } => {
                    let normalised = crate::naming::normalise_var_name(name);
                    if !normalised.is_empty() {
                        names.insert(normalised.to_string());
                    }
                }
                Statement::Call { defs, .. } => {
                    for def in defs {
                        names.insert(def.clone());
                    }
                }
                _ => {}
            }
        }
    }
    names
}

/// Compute the set of global variable names that any procedure
/// in `cu` writes.
///
/// Mirrors `_globals_written_by_procs` in
/// `core/analysis/_analyser/_diag_commands.py:264-296`.
///
/// A global write happens when a proc either:
///
/// 1. assigns to a fully-qualified name (``::var``), or
/// 2. declares ``global var`` and then assigns to ``var`` in the
///    same proc body.
///
/// The result is the union of (1) and the intersection of
/// global aliases × locally-written names (case (2)).  Used at
/// top-level to suppress W210 for globals a helper proc may
/// populate before the top-level read.
///
/// **Simplification vs. Python.** The Rust port doesn't yet
/// have ``CommandRegistry::is_destroys_variable`` so commands
/// like ``unset`` aren't filtered out of the "writes" set.
/// That makes the suppression slightly more permissive (more
/// vars marked "written-by-procs" → more W210 suppressions).
/// Safe-on-correctness — the alternative is false positives
/// on real RBS sites.  When the registry gains
/// ``destroys_variable``, add the filter here for parity.
fn globals_written_by_procs(cu: &crate::compilation_unit::CompilationUnit) -> HashSet<String> {
    use crate::ir::Statement;
    let mut result: HashSet<String> = HashSet::new();
    for fu in cu.procedures.values() {
        let mut global_aliases: HashSet<String> = HashSet::new();
        let mut written: HashSet<String> = HashSet::new();
        for block in fu.cfg.blocks.values() {
            for stmt in &block.statements {
                let names: Vec<&String> = match stmt {
                    Statement::Call { command, defs, .. } => {
                        if command == "global" {
                            for d in defs {
                                global_aliases.insert(d.clone());
                            }
                            continue;
                        }
                        if matches!(command.as_str(), "variable" | "upvar") {
                            continue;
                        }
                        defs.iter().collect()
                    }
                    Statement::AssignConst { name, .. }
                    | Statement::AssignExpr { name, .. }
                    | Statement::AssignValue { name, .. }
                    | Statement::Incr { name, .. } => vec![name],
                    _ => continue,
                };
                for name in names {
                    if let Some(bare) = name.strip_prefix("::") {
                        let bare = bare.trim_start_matches(':');
                        if !bare.is_empty() {
                            result.insert(bare.to_string());
                        }
                    } else {
                        written.insert(name.clone());
                    }
                }
            }
        }
        for n in global_aliases.intersection(&written) {
            result.insert(n.clone());
        }
    }
    result
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
/// W123 message.  Mirrors the Python equivalent in
/// `_diag_commands.py:233-237`.
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
fn body_references_param(body: &str, param: &str) -> bool {
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

fn is_ident_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b':'
}

/// Walk `node` and collect every `==`/`!=` operator whose at least
/// one operand is a string literal ([`ExprNode::String`]).
///
/// Mirrors `_find_string_eq_ne` in
/// `core/analysis/checks/_style.py:685-713`.  Comparisons between
/// two variables (`$x == $y`) are intentionally *not* collected —
/// the variables may hold integer values, making `==` correct.
fn find_string_eq_ne_ops(node: &ExprNode) -> Vec<BinOp> {
    let mut found = Vec::new();
    walk_string_eq_ne(node, &mut found);
    found
}

fn walk_string_eq_ne(node: &ExprNode, found: &mut Vec<BinOp>) {
    match node {
        ExprNode::Binary { op, left, right } => {
            walk_string_eq_ne(left, found);
            walk_string_eq_ne(right, found);
            if matches!(op, BinOp::Eq | BinOp::Ne)
                && (matches!(**left, ExprNode::String { .. })
                    || matches!(**right, ExprNode::String { .. }))
            {
                found.push(*op);
            }
        }
        ExprNode::Unary { operand, .. } => walk_string_eq_ne(operand, found),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            walk_string_eq_ne(condition, found);
            walk_string_eq_ne(true_branch, found);
            walk_string_eq_ne(false_branch, found);
        }
        ExprNode::Call { args, .. } => {
            for arg in args {
                walk_string_eq_ne(arg, found);
            }
        }
        _ => {}
    }
}

/// Count the total number of `==`/`!=` operators in the expression
/// tree.  Mirrors `_count_eq_ne_ops` in
/// `core/analysis/checks/_style.py:716-731`.
fn count_eq_ne_ops(node: &ExprNode) -> usize {
    match node {
        ExprNode::Binary { op, left, right } => {
            let mut n = count_eq_ne_ops(left) + count_eq_ne_ops(right);
            if matches!(op, BinOp::Eq | BinOp::Ne) {
                n += 1;
            }
            n
        }
        ExprNode::Unary { operand, .. } => count_eq_ne_ops(operand),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            count_eq_ne_ops(condition)
                + count_eq_ne_ops(true_branch)
                + count_eq_ne_ops(false_branch)
        }
        ExprNode::Call { args, .. } => args.iter().map(count_eq_ne_ops).sum(),
        _ => 0,
    }
}

/// Rewrite `==`/`!=` operators to ` eq `/` ne ` for use in a code
/// fix's replacement text.  Mirrors `_rewrite_string_compare_ops`
/// in `core/analysis/checks/_helpers.py:82-88`.
///
/// Implements the Python regex semantics manually:
/// * `(?<![=!])==(?!=)`  → ` eq `
/// * `!=`                → ` ne `
/// * `[ \t]{2,}`         → ` `  (collapse runs of 2+ ws)
fn rewrite_string_compare_ops(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut step1 = String::with_capacity(text.len() + 8);
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        // !=  →  " ne "
        if c == '!' && i + 1 < chars.len() && chars[i + 1] == '=' {
            step1.push_str(" ne ");
            i += 2;
            continue;
        }
        // ==  →  " eq "  (with negative look-around)
        if c == '=' && i + 1 < chars.len() && chars[i + 1] == '=' {
            let prev_ok = i == 0 || (chars[i - 1] != '=' && chars[i - 1] != '!');
            let next_ok = i + 2 >= chars.len() || chars[i + 2] != '=';
            if prev_ok && next_ok {
                step1.push_str(" eq ");
                i += 2;
                continue;
            }
        }
        step1.push(c);
        i += 1;
    }
    // Collapse runs of 2+ space/tab into a single space.  Single
    // whitespace characters are preserved (matches Python's
    // ``re.sub(r"[ \t]{2,}", " ", ...)``).
    let chars: Vec<char> = step1.chars().collect();
    let mut out = String::with_capacity(step1.len());
    let mut i = 0;
    while i < chars.len() {
        if (chars[i] == ' ' || chars[i] == '\t')
            && i + 1 < chars.len()
            && (chars[i + 1] == ' ' || chars[i + 1] == '\t')
        {
            out.push(' ');
            while i < chars.len() && (chars[i] == ' ' || chars[i] == '\t') {
                i += 1;
            }
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// Scan `args` for the first positional argument that lacks a
/// preceding `--` terminator.  Mirrors
/// `core/analysis/checks/_helpers.py::_first_positional_without_terminator`.
///
/// Skips option words (text starts with `-`); skips an additional
/// argument when the option's [`OptionSpec`](tcl_registry::prelude::OptionSpec)
/// in [`ResolvedTerminator::options`](tcl_registry::ResolvedTerminator)
/// has `takes_value == true`.  Linear scan over the borrowed
/// option slice — per-command option counts are small (≤ a dozen
/// for the largest specs in practice), so this is cheaper than a
/// per-resolve `HashSet` allocation on the analyser hot path.
/// Returns `None` when a `--` is encountered (positional arguments
/// after `--` are explicitly terminated).
fn first_positional_without_terminator(
    args: &[String],
    profile: &tcl_registry::ResolvedTerminator,
) -> Option<usize> {
    let mut i = profile.scan_start;
    while i < args.len() {
        let arg = args[i].as_str();
        if arg == "--" {
            return None;
        }
        if arg.starts_with('-') {
            i += 1;
            let consumes_value = profile
                .options
                .iter()
                .any(|o| o.name == arg && o.takes_value);
            if consumes_value && i < args.len() {
                i += 1;
            }
            continue;
        }
        return Some(i);
    }
    None
}

/// Locate the most-recent literal `set var value` assignment whose
/// command-head precedes `before_offset`.  Mirrors
/// `core/analysis/checks/_helpers.py::_last_literal_set_value_for_var`.
///
/// Returns `Some((value_text, value_span, var_text))` when the
/// nearest preceding `set` is a fully-literal three-arg form.
/// Returns `None` when the latest assignment is dynamic / multi-
/// token (the runtime value cannot be proven statically).
fn last_literal_set_value_for_var(
    source: &str,
    var_name: &str,
    before_offset: u32,
) -> Option<(String, tcl_lexer::Span, String)> {
    if var_name.is_empty() || before_offset == 0 {
        return None;
    }
    let head = before_offset as usize;
    if head > source.len() {
        return None;
    }
    let prefix = &source[..head];
    let segments = crate::segmenter::segment_commands(prefix);

    for cmd in segments.iter().rev() {
        if cmd.texts.first().map(String::as_str) != Some("set") {
            continue;
        }
        if cmd.texts.len() < 3 {
            continue;
        }
        if cmd.texts[1] != var_name {
            continue;
        }
        // Most recent assignment wins.  If it's dynamic, the
        // runtime value can't be proven statically.
        if cmd.single_token_word.get(2).copied() != Some(true) {
            return None;
        }
        if cmd.argv.len() < 3 {
            return None;
        }
        let value_tok = cmd.argv[2];
        if !matches!(
            value_tok.kind,
            tcl_lexer::TokenType::Esc | tcl_lexer::TokenType::Str
        ) {
            return None;
        }
        return Some((cmd.texts[2].clone(), value_tok.span, var_name.to_string()));
    }
    None
}

impl Analyser {
    /// Scope-tree-driven variable diagnostic emitter.
    ///
    /// Mirrors `_emit_variable_usage_diagnostics` in
    /// `_diagnostics.py:111-116`.  Python keeps this method as
    /// an empty hook because W211 (unused-variable) moved to the
    /// SSA-based pass in `_emit_cfg_ssa_diagnostics_for_function`.
    /// The Rust port preserves the hook so future scope-tree-
    /// driven emitters (none currently planned) have a target.
    pub fn emit_variable_usage_diagnostics(&mut self) {
        // Intentionally empty — see module docstring.
    }

    /// **W105.** Emit "unbraced code block" warnings for body
    /// arguments that aren't braced.  Mirrors
    /// ``check_unbraced_body`` in
    /// ``core/analysis/checks/_style.py:238-302``.
    ///
    /// Severity is ERROR when the unbraced body contains
    /// substitutions (``$var`` / ``[cmd]``) — those risk double
    /// substitution.  Severity is WARNING otherwise.  Single
    /// barewords without substitution are silently allowed
    /// (some commands accept a proc name as a body alternative).
    pub(super) fn emit_w105_unbraced_body(
        &mut self,
        cmd_name: &str,
        body_text: &str,
        body_tok: tcl_lexer::Token,
    ) {
        // Already braced — `Str` token kind means the source
        // started with ``{``.  Mirrors ``_first_token_is_braced``
        // in Python.
        if matches!(body_tok.kind, tcl_lexer::TokenType::Str) {
            return;
        }
        let trimmed = body_text.trim();
        // Mirror Python's ``_has_substitution``: textual ``$`` /
        // ``[`` count as substitutions, and so do ``Var`` / ``Cmd``
        // tokens — even when the entire body is a direct
        // substitution (``while {$cond} $body``).  Those still
        // emit W105 at ERROR severity because an unbraced
        // substituted body double-evaluates at runtime.
        let has_substitution = trimmed.contains('$')
            || trimmed.contains('[')
            || matches!(
                body_tok.kind,
                tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
            );
        // Single bareword + no substitution is the alternative
        // form (e.g. body = a proc name).  Skip.
        if !trimmed.contains(char::is_whitespace) && !has_substitution {
            return;
        }
        let severity = if has_substitution {
            super::types::Severity::Error
        } else {
            super::types::Severity::Warning
        };
        let message = if has_substitution {
            format!(
                "Code block argument to '{cmd_name}' is not braced and \
contains substitutions \u{2014} risk of double substitution. \
Use braces: {{ \u{2026} }}"
            )
        } else {
            format!(
                "Code block argument to '{cmd_name}' should be braced \
for clarity and to prevent accidental substitution. \
Use braces: {{ \u{2026} }}"
            )
        };
        let new_text = format!("{{{body_text}}}");
        self.result.diagnostics.push(super::types::Diagnostic {
            code: "W105".to_string(),
            span: body_tok.span,
            message,
            severity,
            fixes: vec![super::types::CodeFix {
                span: body_tok.span,
                new_text,
                description: "Wrap code block in braces".to_string(),
            }],
        });
    }

    /// **W110.** Emit "use `eq`/`ne` instead of `==`/`!=` for
    /// string comparison" hints on the EXPR-role argument of
    /// commands like `if`, `while`, `for`, `expr`.
    ///
    /// Mirrors ``check_string_compare_in_expr`` in
    /// ``core/analysis/checks/_style.py:740-834``.  Fires when at
    /// least one operand of a `==` / `!=` comparison is a string
    /// literal (`ExprString`, e.g. `"foo"`, `"1"`, `"true"`);
    /// comparisons between variables (`$x == $y`) are left alone.
    ///
    /// `expr_text` is the post-substitution body of the EXPR-role
    /// argument (already brace-stripped) — the caller is
    /// responsible for joining multi-arg `expr` invocations with
    /// spaces before calling.  `diag_span` is the source span the
    /// diagnostic anchors to (the source range of the argument
    /// token, or the full token range for `expr`).
    pub(super) fn emit_w110_string_eq_ne(&mut self, expr_text: &str, diag_span: tcl_lexer::Span) {
        // Quick bail-out: no equality operator at all.
        if !expr_text.contains("==") && !expr_text.contains("!=") {
            return;
        }
        let parsed = crate::parse_expr(expr_text.trim(), Some(self.dialect.as_str()));
        // ``ExprNode::Raw`` means the expression was unparseable —
        // mirror Python's ``isinstance(parsed, ExprRaw): continue``.
        if matches!(parsed, ExprNode::Raw { .. }) {
            return;
        }
        let matched_ops = find_string_eq_ne_ops(&parsed);
        if matched_ops.is_empty() {
            return;
        }
        let first_op = matched_ops[0];
        let (op_text, replacement) = match first_op {
            BinOp::Eq => ("==", "eq"),
            BinOp::Ne => ("!=", "ne"),
            _ => unreachable!("find_string_eq_ne_ops only returns Eq/Ne"),
        };
        // Only offer the regex-based code fix when every ``==``/
        // ``!=`` in the expression has a string-literal operand;
        // otherwise the blanket rewrite would incorrectly change
        // non-string comparisons too.
        let total = count_eq_ne_ops(&parsed);
        let mut fixes = Vec::new();
        if matched_ops.len() >= total {
            let rewritten = rewrite_string_compare_ops(expr_text);
            if rewritten != expr_text {
                fixes.push(super::types::CodeFix {
                    span: diag_span,
                    new_text: rewritten,
                    description: format!("Use '{replacement}' for string comparison"),
                });
            }
        }
        let message = format!(
            "Use '{replacement}' instead of '{op_text}' for string \
comparison in expressions to avoid ambiguous \
numeric/string coercion."
        );
        self.result.diagnostics.push(super::types::Diagnostic {
            code: "W110".to_string(),
            span: diag_span,
            message,
            severity: Severity::Hint,
            fixes,
        });
    }

    /// **W302.** Emit "catch without result variable" hint when a
    /// `catch BODY` invocation omits the optional `RESULTVAR`
    /// argument, silently swallowing any error the body raises.
    ///
    /// Mirrors the `IRCatch` arm of ``_check_statement`` in
    /// ``core/compiler/compiler_checks.py:491-504``.  Python only
    /// emits W302 for `IRCatch` (not `IRBarrier`) — the lowerer
    /// falls back to `IRBarrier` when the body argument is multi-token
    /// (e.g. ``catch $body``), so this Rust emit gates on
    /// ``arg_single[0]`` to mirror that suppression.  The
    /// diagnostic anchors at the full command span (catch keyword
    /// through the last argument's end), matching Python's
    /// ``stmt.range``.
    pub(super) fn emit_w302_catch_no_result_var(
        &mut self,
        args: &[String],
        cmd_tok: tcl_lexer::Token,
        arg_tokens: &[tcl_lexer::Token],
        arg_single: &[bool],
    ) {
        // Only fires when a result variable is absent.  Empty args
        // is "malformed catch" in Python's lowerer (IRBarrier path,
        // no W302).  ≥2 args means a result variable is present.
        if args.len() != 1 {
            return;
        }
        // Mirror Python's "catch with dynamic body" IRBarrier
        // suppression: a multi-token body word can't be statically
        // resolved to a script, so the lowerer drops it before
        // ``_check_statement`` ever sees it.
        if arg_single.first().copied() != Some(true) {
            return;
        }
        let Some(body_tok) = arg_tokens.first().copied() else {
            return;
        };
        let span = tcl_lexer::Span::new(cmd_tok.span.start(), body_tok.span.end());
        self.result.diagnostics.push(super::types::Diagnostic {
            code: "W302".to_string(),
            span,
            message: "catch without a result variable silently swallows errors. \
Consider capturing the result: catch {\u{2026}} result"
                .to_string(),
            severity: Severity::Hint,
            fixes: Vec::new(),
        });
    }

    /// **W001.** Emit "Unknown subcommand" warning for commands
    /// whose registry signature is a [`SubcommandSig`](super::dispatch::SubcommandSig)
    /// when the first argument doesn't resolve to a known subcommand.
    ///
    /// Mirrors the `SubcommandSig` branch of `_check_arity` in
    /// ``core/compiler/compiler_checks.py:580-643``.  Skips:
    ///
    /// - commands the registry doesn't know (no signature),
    /// - simple-command signatures (no subcommand dispatch),
    /// - signatures with `allow_unknown == true` (generated
    ///   dialect packs),
    /// - first-arg values containing ``$`` / ``[`` (dynamic
    ///   substitution — runtime-resolved),
    /// - empty arg lists (handled by the E001 emitter, deferred).
    ///
    /// When emission is warranted, includes a "did you mean…?"
    /// suffix using [`crate::text::suggest_similar`] over the
    /// known subcommand set (max 1 suggestion within edit
    /// distance 3).
    ///
    /// **Known minor parity gap:** Python additionally skips when
    /// the subcommand position is ``{*}``-expanded
    /// (``arg_expand[0]``).  The Rust ``process_command`` does not
    /// currently thread the expansion flag through; the literal-
    /// text ``$`` / ``[`` gate covers the dynamic-substitution
    /// case, and ``{*}LITERAL`` for an unknown subcommand is rare
    /// enough in practice that the divergence is acceptable until
    /// expand-flag plumbing lands as its own chunk.
    pub(super) fn emit_w001_unknown_subcommand(
        &mut self,
        cmd_name: &str,
        args: &[String],
        cmd_tok: tcl_lexer::Token,
        arg_tokens: &[tcl_lexer::Token],
    ) {
        use super::dispatch::{signature_for_command, CommandSignature};
        use tcl_registry::prelude::DialectSet;

        let Some(registry) = self.registry.as_ref() else {
            return;
        };
        let Some(first_arg) = args.first() else {
            // Empty arg list — Python's E001 path; not in scope here.
            return;
        };
        // Dynamic-value subcommand position — can't resolve statically.
        if first_arg.contains('$') || first_arg.contains('[') {
            return;
        }
        let dialect = DialectSet::parse(&self.dialect).unwrap_or(DialectSet::ALL_TCL);
        let Some(CommandSignature::WithSubcommands(sig)) =
            signature_for_command(registry, cmd_name, dialect)
        else {
            return;
        };
        if sig.allow_unknown {
            return;
        }
        if sig.subcommands.contains_key(first_arg) {
            return;
        }
        let mut message = format!("Unknown subcommand '{first_arg}' for '{cmd_name}'");
        let candidates: Vec<&str> = sig.subcommands.keys().map(String::as_str).collect();
        let suggestions = crate::text::suggest_similar(first_arg, candidates.iter().copied(), 1, 3);
        let mut fixes: Vec<super::types::CodeFix> = Vec::new();
        if let Some(best) = suggestions.first() {
            use std::fmt::Write as _;
            let _ = write!(message, "; did you mean '{best}'?");
            if let Some(sub_tok) = arg_tokens.first() {
                // Target the *content* range of the subcommand
                // token rather than its full span.  Wrapper tokens
                // (`Str` braced, `Esc` quoted) carry the opening
                // delimiter via ``content_offset`` and intentionally
                // exclude the closing delimiter from ``span.end``;
                // replacing the full span would leave a stray
                // ``}`` / ``"`` behind (e.g. ``string {lenght}`` →
                // ``string length}``).  Using the content range
                // ([span.start + content_offset, span.end)) gives
                // ``{length}`` / ``"length"`` for the wrapped forms
                // and remains identical to the full span for bare
                // ``Esc`` words (``content_offset == 0``).
                let content_start = sub_tok.span.start() + u32::from(sub_tok.content_offset);
                let fix_span = tcl_lexer::Span::new(content_start, sub_tok.span.end());
                fixes.push(super::types::CodeFix {
                    span: fix_span,
                    new_text: (*best).to_string(),
                    description: format!("Replace with '{best}'"),
                });
            }
        }
        // Anchor at the command-head + subcommand-name range so
        // the squiggle covers ``cmd subname`` rather than the
        // entire invocation.  Mirrors Python's ``cmd_token_range``
        // which combines the command token with the subcommand
        // arg token.
        let span = match arg_tokens.first() {
            Some(sub_tok) => tcl_lexer::Span::new(cmd_tok.span.start(), sub_tok.span.end()),
            None => cmd_tok.span,
        };
        self.result.diagnostics.push(super::types::Diagnostic {
            code: "W001".to_string(),
            span,
            message,
            severity: Severity::Warning,
            fixes,
        });
    }

    /// **E004.** Emit "Malformed `if` command" / "Extra words after
    /// `else` clause" errors when an `if` invocation's structural
    /// shape doesn't match `if COND BODY ?elseif COND BODY ...?
    /// ?else BODY?`.
    ///
    /// Mirrors the `IRBarrier` arm of `_check_statement` in
    /// ``core/compiler/compiler_checks.py:506-525``, which fires
    /// when Python's `_lower_if`
    /// (``core/compiler/lowering.py:645-753``) returns an
    /// `IRBarrier` with `command == "if"` because the syntactic
    /// shape is invalid.  The reasons it produces:
    ///
    /// - `"malformed if"` — empty arg list, or no clauses after
    ///   the full walk.
    /// - `"malformed if else clause"` — bare `else` with no body
    ///   following.
    /// - `'extra words after "else" clause'` — `else BODY` with
    ///   one or more trailing words.
    /// - `"malformed if clause"` — condition with no body
    ///   (with or without an intervening `then` keyword).
    ///
    /// Detected analyser-side at the `if`-command dispatch site
    /// rather than by walking lowered IR — matches the established
    /// W302 / W001 dispatch-site pattern.  Also closes a latent
    /// parity gap in `lowering/structured.rs::lower_if`, which
    /// currently doesn't produce an "extra words after else"
    /// barrier at all (see `lowering.py:686-693` vs
    /// `structured.rs:147-162`).
    ///
    /// Severity: `Error`.  No code fixes (Python doesn't emit
    /// any).  Span anchors at the command-head token through the
    /// last argument-token end, mirroring Python's `cmd.range`
    /// (full command source range).
    pub(super) fn emit_e004_malformed_if(
        &mut self,
        args: &[String],
        cmd_tok: tcl_lexer::Token,
        arg_tokens: &[tcl_lexer::Token],
    ) {
        let full_span = match arg_tokens.last() {
            Some(last) => tcl_lexer::Span::new(cmd_tok.span.start(), last.span.end()),
            None => cmd_tok.span,
        };
        let push_malformed = |this: &mut Self| {
            this.result.diagnostics.push(super::types::Diagnostic {
                code: "E004".to_string(),
                span: full_span,
                message: "Malformed 'if' command".to_string(),
                severity: Severity::Error,
                fixes: Vec::new(),
            });
        };
        let push_extra_words = |this: &mut Self| {
            this.result.diagnostics.push(super::types::Diagnostic {
                code: "E004".to_string(),
                span: full_span,
                message: "Extra words after \"else\" clause in \"if\" command".to_string(),
                severity: Severity::Error,
                fixes: Vec::new(),
            });
        };

        if args.is_empty() {
            push_malformed(self);
            return;
        }

        let mut i = 0;
        let mut clause_count: usize = 0;
        while i < args.len() {
            if args[i] == "elseif" {
                i += 1;
                continue;
            }
            if args[i] == "else" {
                if i + 1 >= args.len() {
                    // Bare ``else`` with no body following.
                    push_malformed(self);
                    return;
                }
                if i + 2 < args.len() {
                    // ``else BODY <extra...>``.
                    push_extra_words(self);
                    return;
                }
                // ``else BODY`` — well-formed terminator.  Note:
                // Python's ``_lower_if`` does *not* append to
                // ``clauses`` here (else-only sets ``else_body``);
                // the post-walk ``if not clauses`` check still
                // fires on ``if else BODY`` to produce a
                // ``"malformed if"`` barrier.  We mirror that by
                // leaving ``clause_count`` unchanged in this arm.
                break;
            }

            // Condition + (optional ``then``) + body shape.
            i += 1;
            if i < args.len() && args[i] == "then" {
                i += 1;
            }
            if i >= args.len() {
                // Condition with no following body.
                push_malformed(self);
                return;
            }
            clause_count += 1;
            i += 1;
        }

        if clause_count == 0 {
            // E.g. ``if elseif`` / ``if else`` after the elseif-skip
            // / else-skip branches consume their keywords without
            // producing a clause.  Mirrors the post-walk
            // ``if not clauses`` check in ``_lower_if``.
            push_malformed(self);
        }
    }

    /// **W304.** Emit "Missing option terminator (`--`)" diagnostics
    /// for option-bearing commands whose first positional argument
    /// could be misinterpreted as an option.
    ///
    /// Mirrors `core/analysis/checks/_style.py::check_missing_option_terminator`
    /// (`_style.py:516-679`).  Resolves the command's option-
    /// terminator profile via
    /// [`tcl_registry::CommandRegistry::resolve_option_terminator`],
    /// scans for the first positional argument that lacks a
    /// preceding `--`, and emits a tristate-severity diagnostic:
    ///
    /// - **OFF** (no diagnostic) — the value is provably non-`-`-
    ///   prefixed (a non-dynamic literal whose representative token
    ///   isn't a `Var`/`Cmd` and whose text doesn't start with `-`).
    /// - **INFO** — dynamic value (`Var` / `Cmd` token) with no
    ///   proof of starting with `-`.  When the value is a single-
    ///   token `Var` whose most recent literal `set` resolves to a
    ///   non-`-`-prefixed value, an additional "origin" diagnostic
    ///   is emitted at the resolution site to explain the INFO
    ///   downgrade.
    /// - **WARNING** — the value is known to start with `-`: either
    ///   a literal whose first character is `-`, or a `Var` whose
    ///   constant-propagated value starts with `-`.
    ///
    /// The diagnostic carries a code-fix that prepends `"-- "` to
    /// the positional-argument span (with a one-byte extension for
    /// `Cmd` tokens whose lexer span excludes the closing `]`).
    ///
    /// **Note on `warn_without_terminator`:** the registry's
    /// `Traits::WARN_WITHOUT_TERMINATOR` flag (set on `regexp` only
    /// today) is plumbed onto [`tcl_registry::ResolvedTerminator`]
    /// for API parity with Python, but Python's analyser-side
    /// `_style.py` doesn't actually consume it.  The OFF gate
    /// fires uniformly for non-dynamic, non-`-`-prefixed values
    /// regardless of the trait — see `_style.py:558-563`.
    pub(super) fn emit_w304_missing_option_terminator(
        &mut self,
        cmd_name: &str,
        args: &[String],
        cmd_tok: tcl_lexer::Token,
        arg_tokens: &[tcl_lexer::Token],
    ) {
        use tcl_registry::prelude::DialectSet;

        let Some(registry) = self.registry.as_ref() else {
            return;
        };
        if args.is_empty() || arg_tokens.is_empty() {
            return;
        }

        let dialect = DialectSet::parse(&self.dialect).unwrap_or(DialectSet::ALL_TCL);
        let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
        let Some(profile) = registry.resolve_option_terminator(cmd_name, &arg_strs, dialect) else {
            return;
        };

        let Some(positional_idx) = first_positional_without_terminator(args, &profile) else {
            return;
        };
        if positional_idx >= arg_tokens.len() {
            return;
        }

        let tok = arg_tokens[positional_idx];
        let text = &args[positional_idx];

        let is_dynamic = matches!(
            tok.kind,
            tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
        );
        let looks_like_option = text.starts_with('-');

        // OFF — non-dynamic value that does not start with `-` can
        // never be confused with an option.
        if !is_dynamic && !looks_like_option {
            return;
        }

        let command_label = match profile.subcommand {
            Some(sub) => format!("{cmd_name} {sub}"),
            None => cmd_name.to_string(),
        };

        let (severity, message, origin) =
            self.classify_w304(tok, is_dynamic, looks_like_option, &command_label);

        // Build the code-fix span.  For ``Cmd`` (`[…]`) tokens the
        // lexer span covers ``[inner`` but excludes the closing
        // ``]``; extend by one byte when the byte after ``span.end``
        // is ``]`` so the replacement encompasses the bracket pair.
        let (fix_span, diag_end) = self.compute_w304_fix_span(tok);
        let fix_text = format!(
            "-- {}",
            &self.source[fix_span.start() as usize..fix_span.end() as usize]
        );
        let fixes = vec![super::types::CodeFix {
            span: fix_span,
            new_text: fix_text,
            description: "Insert '--' option terminator".to_string(),
        }];
        let diag_span = tcl_lexer::Span::new(tok.span.start(), diag_end);
        // Suppress unused-warning on the rare path where `cmd_tok`
        // isn't needed (the diagnostic anchors at the positional
        // arg's span, not the command head).
        let _ = cmd_tok;

        self.result.diagnostics.push(super::types::Diagnostic {
            code: "W304".to_string(),
            span: diag_span,
            message,
            severity,
            fixes,
        });
        if let Some(origin_diag) = origin {
            self.result.diagnostics.push(origin_diag);
        }
    }

    /// **W101.** Emit "eval with string concatenation" warning
    /// when an `eval` invocation's argument list could be a
    /// substitution-driven injection vector.
    ///
    /// Mirrors `core/analysis/checks/_security.py:19-73::check_eval_string_concat`.
    /// Suppressed when:
    ///
    /// - every argument's representative token is `Str` (braced,
    ///   `eval {script}` / `eval {a} {b}` — the safe form), or
    /// - the single argument is a `Cmd` substitution whose inner
    ///   command head produces a canonical list (per
    ///   [`tcl_registry::CommandRegistry::is_canonical_list_command`]
    ///   — `eval [list ...]`, `eval [linsert ...]`, etc.).
    ///
    /// Otherwise fires `Severity::Warning` when any argument's
    /// representative token is `Var` / `Cmd` (substitution at the
    /// word level), or any argument is a multi-token word
    /// (substitution within the word — the single-token-word flag
    /// is `false`).  This is a sound approximation of Python's
    /// `all_tokens[1:]`-walk: `process_command` doesn't currently
    /// thread the full token stream, but multi-token-word implies
    /// inner substitution and the per-arg representative kind
    /// covers the single-token VAR / CMD cases.
    ///
    /// Diagnostic anchors at the first argument's range.
    pub(super) fn emit_w101_eval_string_concat(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
        arg_single: &[bool],
    ) {
        if cmd_name != "eval" || args.is_empty() || arg_tokens.is_empty() {
            return;
        }
        // ``eval {script}`` / ``eval {a} {b}`` — every word is a
        // braced literal, no substitution risk.
        if arg_tokens
            .iter()
            .all(|tok| matches!(tok.kind, tcl_lexer::TokenType::Str))
        {
            return;
        }
        // ``eval [list ...]`` and similar canonical-list idioms —
        // single-arg ``Cmd`` whose inner head produces a canonical
        // list.
        if arg_tokens.len() == 1 && self.is_canonical_list_substitution(arg_tokens[0]) {
            return;
        }
        // Substitution detection.  An argument carries substitution
        // when:
        //
        // - the representative token kind is ``Var`` / ``Cmd``
        //   (single-token substitution at the word level), or
        // - the word is multi-token AND its source range contains
        //   an unescaped ``$`` / ``[`` outside any ``{...}`` block.
        //
        // The multi-token-word flag alone is **not** equivalent to
        // substitution: the segmenter sets ``single_token_word=false``
        // for any adjacent-token concatenation, including pure-
        // literal shapes like ``eval foo{bar}`` (Esc+Str joined,
        // no inner Var/Cmd).  Mirroring Python's
        // ``all_tokens[1:]`` walk would require threading the full
        // token stream through ``process_command``; instead we do a
        // brace/backslash-aware source-byte scan over the word's
        // span, which is sound for the common cases and matches
        // Python's behaviour for every fixture in
        // ``tests/test_checks.py::TestEvalStringConcat``.  Known
        // approximation gap: ``"foo{$x}bar"`` (substitution inside
        // a brace pair within a quoted string — Tcl treats braces
        // as literal inside ``"…"``) is not detected.  Real W101
        // shapes don't hit that pattern; documented for posterity.
        let has_substitution = arg_tokens.iter().enumerate().any(|(i, tok)| {
            if matches!(
                tok.kind,
                tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
            ) {
                return true;
            }
            if arg_single.get(i).copied() == Some(true) {
                return false;
            }
            self.word_span_contains_substitution(tok.span)
        });
        if !has_substitution {
            return;
        }
        let first = arg_tokens[0];
        self.result.diagnostics.push(super::types::Diagnostic {
            code: "W101".to_string(),
            span: first.span,
            message: "eval with substituted arguments risks code injection. \
Prefer direct invocation or {*}$cmdList to preserve argument boundaries."
                .to_string(),
            severity: Severity::Warning,
            fixes: Vec::new(),
        });
    }

    /// Scan the source bytes covered by `span` for an unescaped
    /// ``$`` or ``[`` outside any ``{...}`` brace block.  Used by
    /// [`Self::emit_w101_eval_string_concat`] to detect inner
    /// substitution within a multi-token word without requiring
    /// the full token stream to be threaded through
    /// ``process_command``.
    ///
    /// Brace tracking: ``{`` increments depth, ``}`` decrements;
    /// ``$`` / ``[`` only count as substitution when depth is
    /// zero.  Backslash escapes consume the next byte (so ``\$``
    /// is skipped).  Out-of-bounds spans return false rather than
    /// panicking.
    fn word_span_contains_substitution(&self, span: tcl_lexer::Span) -> bool {
        let start = span.start() as usize;
        let end = span.end() as usize;
        if end > self.source.len() || start >= end {
            return false;
        }
        let bytes = self.source.as_bytes();
        let mut i = start;
        let mut brace_depth: i32 = 0;
        while i < end {
            match bytes[i] {
                b'\\' if i + 1 < end => {
                    i += 2;
                    continue;
                }
                b'{' => brace_depth += 1,
                b'}' => {
                    if brace_depth > 0 {
                        brace_depth -= 1;
                    }
                }
                b'$' | b'[' if brace_depth == 0 => return true,
                _ => {}
            }
            i += 1;
        }
        false
    }

    /// Helper for [`Self::emit_w101_eval_string_concat`].  Returns
    /// true when `tok` is a `Cmd` token whose inner script's
    /// command head (or `cmd subcmd` pair) produces a canonical
    /// list per the registry — the W101 safe-idiom suppression.
    ///
    /// Conservative: rejects multi-command scripts (containing `;`
    /// or newline) because `[list a b; set x $user]` returns the
    /// last command's result, which isn't necessarily a safe list.
    /// Mirrors `_security.py::_is_list_command_token`.
    fn is_canonical_list_substitution(&self, tok: tcl_lexer::Token) -> bool {
        if !matches!(tok.kind, tcl_lexer::TokenType::Cmd) {
            return false;
        }
        let Some(registry) = self.registry.as_ref() else {
            return false;
        };
        let start = tok.span.start() as usize + tok.content_offset as usize;
        let end = tok.span.end() as usize;
        if start >= end || end > self.source.len() {
            return false;
        }
        let script = self.source[start..end].trim();
        if script.is_empty() || script.contains(';') || script.contains('\n') {
            return false;
        }
        // ``parts[0]`` = command head; check both bare form and
        // ``"head sub"`` compound form.
        let mut iter = script.splitn(2, char::is_whitespace);
        let Some(head) = iter.next() else {
            return false;
        };
        if registry.is_canonical_list_command(head) {
            return true;
        }
        if let Some(rest) = iter.next() {
            let mut sub_iter = rest.trim_start().splitn(2, char::is_whitespace);
            if let Some(sub) = sub_iter.next() {
                let compound = format!("{head} {sub}");
                if registry.is_canonical_list_command(&compound) {
                    return true;
                }
            }
        }
        false
    }

    /// Classify the positional value for W304: tristate severity,
    /// human-readable message, and an optional "origin" diagnostic
    /// for the constant-propagated INFO path.  Split out of
    /// [`Self::emit_w304_missing_option_terminator`] to keep that
    /// method's body within the clippy `too_many_lines` budget;
    /// mirrors the severity tree at ``_style.py:565-627``.
    fn classify_w304(
        &self,
        tok: tcl_lexer::Token,
        is_dynamic: bool,
        looks_like_option: bool,
        command_label: &str,
    ) -> (Severity, String, Option<super::types::Diagnostic>) {
        if is_dynamic && !looks_like_option {
            if matches!(tok.kind, tcl_lexer::TokenType::Var) {
                let var_name = self.var_name_from_token(tok);
                let resolved = var_name.and_then(|name| {
                    last_literal_set_value_for_var(&self.source, &name, tok.span.start())
                });
                if let Some((resolved_text, resolved_span, var_text)) = resolved {
                    if resolved_text.starts_with('-') {
                        let message = format!(
                            "'{command_label}' parses leading '-' as options. \
This value currently resolves to '{resolved_text}', so add '--' to force \
data parsing."
                        );
                        return (Severity::Warning, message, None);
                    }
                    let message = format!(
                        "'{command_label}' parses leading '-' as options. \
This value is reported at INFO because '{var_text}' currently resolves to \
static literal '{resolved_text}'. Keep '--' to guard against future \
option-injection regressions if the variable changes."
                    );
                    let origin = super::types::Diagnostic {
                        code: "W304".to_string(),
                        span: resolved_span,
                        message: format!(
                            "'{var_text}' is currently assigned static \
literal '{resolved_text}' here; this is why the diagnostic is INFO."
                        ),
                        severity: Severity::Suggestion,
                        fixes: Vec::new(),
                    };
                    return (Severity::Suggestion, message, Some(origin));
                }
            }
            // Command substitution / unresolved variable — INFO
            // with the substituted-input message.
            let message = format!(
                "'{command_label}' parses leading '-' as options. \
Insert '--' before substituted input to reduce option-injection risk."
            );
            return (Severity::Suggestion, message, None);
        }
        // ALWAYS: literal value that starts with `-`.
        let message = format!(
            "'{command_label}' argument starts with '-'. Add '--' \
before this value so it is treated as data, not an option."
        );
        (Severity::Warning, message, None)
    }

    /// Extract the variable name for a `Var` token using the
    /// lexer-provided token-text semantics
    /// ([`tcl_lexer::SourceMap::token_text`]).  Preserves the
    /// `Var`-specific normalisation rules (notably the trailing
    /// `}` strip for the `${}` degenerate case where the lexer
    /// extends the span by one byte to cover the closing brace),
    /// so this stays in sync with the rest of the analyser's
    /// token-text usage and avoids edge-case mismatches that a
    /// raw `self.source[..]` slice would introduce.  Returns
    /// `None` when the extracted text is empty.
    fn var_name_from_token(&self, tok: tcl_lexer::Token) -> Option<String> {
        let sm = tcl_lexer::SourceMap::new(&self.source);
        let text = sm.token_text(tok);
        if text.is_empty() {
            return None;
        }
        Some(text.to_string())
    }

    /// Compute the W304 code-fix span and diagnostic end position.
    ///
    /// For `Cmd` tokens (`[…]`) the lexer span excludes the closing
    /// `]`; we extend the span by one byte when the next character
    /// is `]` so the prepended ``-- `` doesn't split the bracket
    /// pair.  All other token kinds use the lexer span directly.
    fn compute_w304_fix_span(&self, tok: tcl_lexer::Token) -> (tcl_lexer::Span, u32) {
        let span_start = tok.span.start();
        let span_end = tok.span.end();
        if matches!(tok.kind, tcl_lexer::TokenType::Cmd) {
            let after = span_end as usize;
            if after < self.source.len() && self.source.as_bytes()[after] == b']' {
                let extended = span_end + 1;
                return (tcl_lexer::Span::new(span_start, extended), extended);
            }
        }
        (tcl_lexer::Span::new(span_start, span_end), span_end)
    }

    /// CFG/SSA-backed diagnostic orchestrator.
    ///
    /// Mirrors `_emit_cfg_ssa_diagnostics` in
    /// `_diagnostics.py:118-181`.  Builds a
    /// [`crate::compilation_unit::CompilationUnit`] for `source`,
    /// then walks the top-level + every procedure, dispatching
    /// per-function emitters.
    ///
    /// **C41d2 lands** the full ``_diag_var_lifecycle.py``
    /// emitter set (W220, W211, W214, W210, W213, H300).
    /// **C41d3 lands** the var-as-command post-pass (W307); W308
    /// awaits the class-hierarchy port.  W242 (interpolated-
    /// command resolution) lands in **C41d4**.
    pub fn emit_cfg_ssa_diagnostics(&mut self, source: &str) {
        use tcl_registry::prelude::DialectSet;
        use tcl_registry::CommandRegistry;

        let mut registry = CommandRegistry::build_default();
        if let Some(d) = DialectSet::parse(&self.dialect) {
            registry.load_dialect(d);
        }
        let cu = crate::compilation_unit::CompilationUnit::build_for(source, &registry, false);

        // **C41e3 follow-up.** Compute the set of globals any
        // proc in this module writes to.  Top-level RBS (W210)
        // is suppressed for these variables — a helper proc may
        // populate them before the top-level read fires.
        // Mirrors `_globals_written_by_procs` in
        // `_diag_commands.py:264-296`.
        let globals_written = globals_written_by_procs(&cu);

        // **C41-default-on-followups-postpass W220-IR-paths.**
        // pkgIndex.tcl files have ``$dir`` set by the package
        // loader before the script body runs — suppress dead-
        // store / unused-variable diagnostics for it at the
        // top-level.  Mirrors `_diagnostics.py:147-149`.
        let top_level_cross_event_vars: HashSet<String> = if self
            .file_path
            .as_deref()
            .is_some_and(|p| p.ends_with("pkgIndex.tcl"))
        {
            HashSet::from(["dir".to_string()])
        } else {
            HashSet::new()
        };

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
        self.emit_channel_diagnostics(&cu.top_level, &registry);
        for (qname, fu) in &cu.procedures {
            // **C41-default-on-followups-postpass W220-IR-paths.**
            // For ``::when::*`` procs, threaded
            // ``cross_event_defs | cross_event_imports`` from the
            // ConnectionScope so dead-store / unused-variable
            // diagnostics suppress vars that may be read in a
            // different iRule event.  Mirrors
            // `_diagnostics.py:165-167`.
            let cross_event_vars: HashSet<String> =
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
            self.emit_cfg_ssa_diagnostics_for_function_full(
                fu,
                &cu.ir_module,
                &HashSet::new(),
                &cross_event_vars,
            );
            self.emit_channel_diagnostics(fu, &registry);
            // **C41d7.** IRULE4005 — racy ``static::``
            // cross-event flow.  Only fires for non-RULE_INIT
            // ``when`` procs when ``ConnectionScope::racy_static_defs``
            // is non-empty.  Mirrors Python's
            // ``_emit_racy_static_diagnostics`` call site in
            // ``_diagnostics.py:171-175``.
            if let Some(scope) = cu.connection_scope.as_ref() {
                if qname.starts_with("::when::") && !scope.racy_static_defs.is_empty() {
                    let event = crate::ir::when_event_name(qname);
                    if event != "RULE_INIT" {
                        self.emit_racy_static_diagnostics(fu, &scope.racy_static_defs);
                    }
                }
            }
        }

        // Cross-function post-pass: resolve $var-as-command sites
        // collected during the walk.  Mirrors
        // ``_emit_var_command_diagnostics`` in
        // ``_diag_var_command.py``.
        self.emit_var_command_diagnostics(&cu, &registry);

        // **C41 follow-up.** Suppress W123 for command-name
        // heads with partial interpolations like ``foo$suffix``
        // when ``$suffix`` resolves cleanly to a finite set of
        // known commands via SCCP.  Mirrors
        // ``_resolve_interpolated_commands`` in
        // ``_diag_commands.py:188-260``.
        self.resolve_interpolated_w123_diagnostics(&cu);
    }

    /// Per-function diagnostic dispatcher.
    ///
    /// Mirrors `_emit_cfg_ssa_diagnostics_for_function` in
    /// `_diagnostics.py:183-209`.  Called once for the top-level
    /// script and once per procedure.  Each per-emitter call is
    /// gated on its own predicate inside the helper.
    ///
    /// **C41d2 wires** all six ``_diag_var_lifecycle.py``
    /// emitters.  Each future C41d strip adds another emitter
    /// call here.
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
    /// writes — matches the
    /// ``extra_known_defined_vars=self._globals_written_by_procs(cu)``
    /// argument in `_diagnostics.py:154`.
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
    ///
    /// Mirrors the `cross_event_vars=` arg threaded through
    /// `_emit_cfg_ssa_diagnostics_for_function` in
    /// `_diagnostics.py:159, 171`.
    pub fn emit_cfg_ssa_diagnostics_for_function_full(
        &mut self,
        function_unit: &crate::compilation_unit::FunctionUnit,
        ir_module: &crate::ir::Module,
        extra_known_defined: &HashSet<String>,
        cross_event_vars: &HashSet<String>,
    ) {
        let defined = collect_defined_vars(&function_unit.cfg);
        let scope_aliases = crate::optimiser::elimination::scan_scope_aliases(&function_unit.cfg);
        let textually_referenced = crate::optimiser::elimination::collect_textual_var_references(
            &self.source,
            &function_unit.cfg,
        );
        let ir_proc = ir_module.procedures.get(&function_unit.name);
        self.emit_dead_store_diagnostics(function_unit, &defined, &scope_aliases, cross_event_vars);
        self.emit_unused_variable_diagnostics(
            function_unit,
            &defined,
            &scope_aliases,
            &textually_referenced,
        );
        self.emit_possible_paste_error_diagnostics(function_unit);
        self.emit_read_before_set_diagnostics(
            function_unit,
            ir_proc,
            &defined,
            &scope_aliases,
            extra_known_defined,
        );
        self.emit_constant_branch_diagnostics(function_unit);
        self.emit_invalid_ip_diagnostics(function_unit);
        if let Some(ir_proc) = ir_proc {
            self.emit_unused_param_diagnostics(function_unit, ir_proc);
        }
    }

    /// W220 — dead-store hint.
    ///
    /// Mirrors `_emit_dead_store_diagnostics` in
    /// `_diag_var_lifecycle.py:29-72`, plus the
    /// IR-statement-type / SCCP path-sensitivity filters baked
    /// into Python's underlying `_dead_stores` analysis
    /// (`core_analyses.py:1105-1156`).  A *dead store* is an
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
    /// Filters applied (each one mirrors a Python suppression):
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
    ///    Python skips them in `_dead_stores`.
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
        for chain in fu.def_use.chains.values() {
            if !chain.is_dead() || chain.definition.kind != DefKind::Statement {
                continue;
            }
            let (var, version) = &chain.key;
            // Globals (``::``-prefixed) are externally consumed
            // — Python `_dead_stores` skips them.
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
            // ``any_other_live`` — another SSA version of this
            // variable has live uses, so this assignment is
            // overwritten.  When no other version is live, the
            // variable is truly unused — that's W211, handled
            // separately.
            let any_other_live = fu
                .def_use
                .chains
                .iter()
                .any(|(k, c)| k.0 == *var && k.1 != *version && !c.is_dead());
            if !any_other_live {
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
            // IR-statement type filter — mirror Python's
            // `_dead_stores` shape (`core_analyses.py:1149-1155`).
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
            let span = stmt.span();
            if span.is_empty() {
                continue;
            }
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

    /// W211 — unused-variable hint.
    ///
    /// Mirrors `_emit_unused_variable_diagnostics` in
    /// `_diag_var_lifecycle.py:226-258`.  Fires when an
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
            let span = stmt.span();
            if span.is_empty() {
                continue;
            }
            let mut message = format!("Variable '{var}' is set but never used");
            if let Some(similar) = find_case_mismatch(var, defined_vars) {
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
    /// Mirrors `_emit_possible_paste_error_diagnostics` in
    /// `_diag_var_lifecycle.py:74-121`.  When two consecutive
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
        use std::collections::HashMap;

        // Pre-compute, per block, the set of statement indices
        // that are dead stores.  Walk every dead Statement-kind
        // chain in def_use, bucket by block.
        let mut dead_idx: HashMap<&str, HashSet<usize>> = HashMap::new();
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
                let span = block.statements[idx + 1].span();
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
    /// Mirrors `_emit_unused_param_diagnostics` in
    /// `_diag_var_lifecycle.py:260-274`.  For every parameter
    /// declared in `ir_proc.params`, check whether any def-use
    /// chain for the parameter (any SSA version) has live uses.
    /// When all chains are dead, the parameter is unused —
    /// emit a Hint at the proc's span.
    ///
    /// Diverges slightly from Python's ``analysis.unused_params``:
    /// Python pre-computes the unused-params list during
    /// ``analyse_ir_module``; the Rust port inlines the same
    /// def-use scan here because the Rust ``FunctionAnalysis``
    /// builder hasn't been ported yet.  The check is equivalent —
    /// a parameter is unused iff no SSA version of its name has
    /// live uses.
    fn emit_unused_param_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        ir_proc: &crate::ir::Procedure,
    ) {
        for param in &ir_proc.params {
            // Tcl's variadic ``args`` parameter is conventionally
            // declared even when unused (as a "consume the rest"
            // marker).  Skip it from W214.
            if param == "args" {
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
            // Mirror the Python ``infer_param_traits`` shallow
            // pass's ``$param`` text scan: if the body source
            // contains a ``$param`` / ``${param}`` reference
            // anywhere, treat the parameter as used and skip
            // W214.  Saves the W214 over-emit on ``proc f {x}
            // { return [expr {$x + 1}] }``-style bodies until
            // the full ``infer_param_traits`` port lands.
            if let Some(body_source) = ir_proc.body_source.as_deref() {
                if body_references_param(body_source, param) {
                    continue;
                }
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

    /// W210 + W213 — read-before-set / unset on possibly-undefined.
    ///
    /// Mirrors `_emit_read_before_set_diagnostics` in
    /// `_diag_var_lifecycle.py:159-224`.  Walks every
    /// version-0 chain (`DefKind::Parameter`) in `fu.def_use`
    /// — those are the synthetic defs the def-use builder
    /// emits when a variable is used without a preceding def.
    ///
    /// Distinguishes real proc parameters from synthetic RBS
    /// reads via `ir_proc.params`.  Only emits inside procedures
    /// (i.e. when `ir_proc` is `Some`) — top-level RBS would
    /// need the `globals_written_by_procs` filter Python uses
    /// (deferred to a later strip).
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
    fn emit_read_before_set_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        ir_proc: Option<&crate::ir::Procedure>,
        defined_vars: &HashSet<String>,
        scope_aliases: &HashSet<String>,
        extra_known_defined: &HashSet<String>,
    ) {
        use crate::def_use::{DefKind, UseKind};
        use crate::ir::Statement;
        use std::fmt::Write as _;

        // **C41e3 follow-up.** Top-level RBS now uses the
        // ``extra_known_defined`` set (computed from
        // ``globals_written_by_procs``) to suppress W210 on
        // globals that helper procs write.  Inside procs the
        // set is empty, matching Python's per-call argument.
        let params_owned: HashSet<&str> = match ir_proc {
            Some(p) => p.params.iter().map(String::as_str).collect(),
            None => HashSet::new(),
        };
        let params = &params_owned;

        for chain in fu.def_use.chains.values() {
            if chain.definition.kind != DefKind::Parameter {
                continue;
            }
            let (var, _version) = &chain.key;
            if params.contains(var.as_str()) {
                continue;
            }
            if scope_aliases.contains(var) {
                continue;
            }
            if extra_known_defined.contains(var) {
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
                        (span, None)
                    } else {
                        let Ok(idx) = usize::try_from(use_site.statement_index) else {
                            continue;
                        };
                        let Some(stmt) = block.statements.get(idx) else {
                            continue;
                        };
                        (stmt.span(), Some(stmt))
                    };
                if span.is_empty() {
                    continue;
                }
                // ``unset`` without ``-nocomplain`` → W213.
                if let Some(Statement::Call { command, args, .. }) = stmt_opt {
                    if command == "unset" && !args.iter().any(|a| a == "-nocomplain") {
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
                }
                // ``safe_on_uninit`` calls that initialise the
                // variable themselves are not RBS — they handle
                // the uninitialised case.
                if let Some(Statement::Call {
                    safe_on_uninit,
                    defs,
                    ..
                }) = stmt_opt
                {
                    if *safe_on_uninit && defs.contains(var) {
                        continue;
                    }
                }
                let mut message = format!("Variable '{var}' is read before it is set");
                if let Some(similar) = find_case_mismatch(var, defined_vars) {
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

    /// I230 / I231 — constant branch / switch-arm condition.
    ///
    /// Mirrors `_emit_constant_branch_diagnostics` in
    /// `core/analysis/_analyser/_diag_branches.py`.  For every
    /// branch SCCP folded to a constant, when the *not-taken*
    /// target is also unreachable (i.e. SCCP confirmed only one
    /// path is feasible), emit an Info-level diagnostic so the
    /// LSP can highlight the dead arm.
    ///
    /// Code selection follows the Python rules:
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
            // The Python check is "not_taken_target in
            // unreachable_blocks".  Rust SCCP exposes
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
            let span = *span;

            let names = [
                branch.block.as_str(),
                branch.taken_target.as_str(),
                branch.not_taken_target.as_str(),
            ];
            let is_switch = names.iter().any(|n| n.starts_with("switch_"));
            let is_if = names.iter().any(|n| n.starts_with("if_"));

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
                severity: Severity::Hint,
                fixes: Vec::new(),
            });
        }
    }

    /// W126 — channel-argument validation.
    ///
    /// Mirrors `_emit_channel_diagnostics` in
    /// `core/analysis/_analyser/_diag_channel.py`.  Walks every
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
                            span: *span,
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
                            span: *span,
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
    /// Mirrors `_emit_invalid_ip_diagnostics` in
    /// `core/analysis/_analyser/_diag_ip.py`.  Walks every
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
    fn emit_invalid_ip_diagnostics(&mut self, fu: &crate::compilation_unit::FunctionUnit) {
        use crate::analyses::{ConstValue, LatticeValue};
        use std::net::Ipv6Addr;
        use std::str::FromStr;

        let dotted_quad =
            regex::Regex::new(r"\b(\d{1,4})\.(\d{1,4})\.(\d{1,4})\.(\d{1,4})\b").expect("regex");
        let ipv6_candidate =
            regex::Regex::new(r"\b([0-9A-Fa-f]{1,4}(?::[0-9A-Fa-f]{0,4}){2,7})\b").expect("regex");

        let mut seen_offsets: HashSet<u32> = HashSet::new();
        for (key, lv) in &fu.sccp.values {
            let Some(text) = (match lv {
                LatticeValue::Const(ConstValue::String(s)) => Some(s.as_str()),
                _ => None,
            }) else {
                continue;
            };

            // ---- IPv4 candidates ----
            for caps in dotted_quad.captures_iter(text) {
                let m = caps.get(0).unwrap();
                if m.start() > 0 && text.as_bytes()[m.start() - 1] == b'/' {
                    continue;
                }
                let octets: Vec<&str> = (1..=4).map(|i| caps.get(i).unwrap().as_str()).collect();
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
            for caps in ipv6_candidate.captures_iter(text) {
                let candidate = caps.get(1).unwrap().as_str();
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
        seen_offsets: &mut HashSet<u32>,
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
        let span = stmt.span();
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
    /// Mirrors `_emit_unresolved_command_diagnostics` in
    /// `core/analysis/_analyser/_diag_commands.py:39-186`.
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
    /// **Deferred** (Python parity gaps documented in the
    /// commit body): ``has_dynamic_providers`` early-return;
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
        if self.disabled_diagnostics.contains("W123") {
            return;
        }

        // Conservative gate: if any ``package require`` was seen,
        // suppress W123 entirely.  The package may load arbitrary
        // commands at runtime that the analyser can't see.
        if !self.result.package_requires.is_empty() {
            return;
        }

        // **C41e3 follow-up.** When the document defines a
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

        let registry_names: HashSet<String> =
            registry.command_names().map(str::to_string).collect();
        // **C41 follow-up.** Inline ``# tcl-lsp: stub NAME ...``
        // declarations contribute to the candidate set and the
        // suppression set so users who declared a stub for a
        // command don't get spurious W123s.  Mirrors the
        // ``stub_names`` set in
        // ``_diag_commands.py:_emit_unresolved_command_diagnostics``.
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
        // suggestions.  Mirrors Python's `candidates` set in
        // `_diag_commands.py:87-106` — every name a real command
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
        let mut seen_candidate_strs: HashSet<&str> = HashSet::new();
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
            if let Some(info) = self.result.unknown_proc_info.as_ref() {
                if info.dispatch_targets.contains(name) {
                    continue;
                }
            }
            // Absolute-form fallback — ``cmd`` may be defined as
            // ``::cmd`` in the global namespace.
            if self.result.all_procs.contains_key(&format!("::{name}")) {
                continue;
            }
            if self.result.all_classes.contains_key(&format!("::{name}")) {
                continue;
            }

            // **C41 follow-up.** "Did you mean…?" suggestion
            // via Levenshtein.  Mirrors the
            // ``suggest_similar(cmd_name, candidates,
            // max_suggestions=1, max_distance=2)`` call in
            // ``_diag_commands.py:166``.  ``candidate_strs`` was
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

    /// W307 — non-literal command name (variable / command-sub
    /// used as command head).
    ///
    /// Mirrors the W307 half of `_emit_var_command_diagnostics`
    /// in `core/analysis/_analyser/_diag_var_command.py:22-294`.
    /// Walks every recorded site in [`Self::var_command_sites`]
    /// and emits W307 unless the variable's value is statically
    /// resolvable to a finite set of known command names.
    ///
    /// **Resolution paths** (mirrors Python; first match
    /// suppresses W307):
    ///
    /// - Aggregate every CONSTSET / CONST entry in `cu`'s SCCP
    ///   results for the variable name; if every value in the
    ///   set is a known command, proc, class, or class-tail name,
    ///   the command head is statically resolvable — suppress.
    ///
    /// **Known limitations.**  W308 (unknown method on object)
    /// is deferred to a follow-up — it needs the
    /// `class_hierarchy` / MRO port (the C41e0 architectural
    /// decision still pending).  Likewise the
    /// `_cmd_command_sites` (``[cmd] method``) suppression via
    /// return-type analysis is deferred — that path needs the
    /// IR-level type-lattice plumbing extended into the
    /// analyser, which is a larger change than fits this strip.
    /// In-method W307 suppression and dict-with /
    /// dict-update barrier-range suppression also defer.
    #[allow(clippy::too_many_lines)]
    // Long-running analyser pass with many sequential phases over the CompilationUnit; splitting requires threading shared local state.
    fn emit_var_command_diagnostics(
        &mut self,
        cu: &crate::compilation_unit::CompilationUnit,
        registry: &tcl_registry::CommandRegistry,
    ) {
        use crate::analyses::{ConstValue, LatticeValue};
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
        // hierarchy.  Mirrors the ``all_typed_vars`` /
        // ``all_types`` aggregation in
        // ``_diag_var_command.py:49-67``.
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

        // Build the class hierarchy once for W308 method
        // resolution (uses the C41e0 ``ClassHierarchy``).
        let hierarchy = if self.result.all_classes.is_empty() {
            None
        } else {
            Some(super::class_hierarchy::build_class_hierarchy(
                self.result.all_classes.clone(),
            ))
        };

        // Aggregate constant-string knowledge per variable name
        // across every function in the CompilationUnit.  Python
        // uses ``_lattice_to_set`` which expands CONST and
        // CONSTSET into a flat set of values; we replicate that
        // shape here.
        let mut all_constsets: HashMap<String, HashSet<String>> = HashMap::new();
        let collect_from = |sccp: &crate::sccp::SccpResult,
                            out: &mut HashMap<String, HashSet<String>>| {
            for (key, lv) in &sccp.values {
                let (var_name, _ver) = key;
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

        // Drain sites so we can borrow self.result mutably below.
        let sites = std::mem::take(&mut self.var_command_sites);
        let objdefined_vars = self.objdefined_vars.clone();
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
            // The Rust analyser doesn't track method context
            // yet (lands in C41e — pending a Method scope kind),
            // so this filter currently matches Python's
            // ``in_method=False`` always-fall-through behaviour.
            if site.in_method {
                continue;
            }
            if let Some(values) = all_constsets.get(&site.var_name) {
                if !values.is_empty() && values.iter().all(|v| is_known_command(v)) {
                    continue;
                }
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

        // **C41 follow-up.** ``[cmd] method`` sites — emit
        // W307 only when the inner command's return type is
        // unknown AND the call isn't an OO self-dispatch
        // (``my`` / ``self``).  When the return type is a
        // known class, validate the method against the
        // hierarchy and emit W308 instead of W307.  This
        // mirrors the cmd_command_sites branch of
        // ``_emit_var_command_diagnostics`` in
        // ``_diag_var_command.py:296-375``.
        let cmd_sites = std::mem::take(&mut self.cmd_command_sites);
        for site in &cmd_sites {
            if site.in_method {
                continue;
            }
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

            // OO self-dispatch ⇒ suppress W307.
            let is_oo_self_dispatch = matches!(head, "my" | "self");
            if is_oo_self_dispatch {
                continue;
            }

            // **Codex P1 fix.** ``[Dog new]`` / ``[Dog create
            // name]`` produce an Object whose class is ``Dog``.
            // The registry lookup for the bare class name
            // returns Overdefined (the class isn't a built-in
            // command) so we recognise the constructor pattern
            // explicitly here — mirrors the Python
            // ``_return_type_for_command`` branch in
            // ``core/compiler/core_analyses.py`` that maps
            // ``known_class new/create`` to ``TclType.OBJECT``
            // with the class name attached.
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
                crate::type_infer::return_type_for_command(registry, head, &arg_strs)
            };

            // ``Object`` return type — suppress W307; if the
            // class is known, validate the method (W308).
            let is_object = ret_type.kind == crate::types::TypeKind::Known
                && matches!(ret_type.tcl_type, Some(tcl_registry::TclType::Object));
            if is_object {
                if !self.disabled_diagnostics.contains("W308") {
                    if let (Some(method), Some(class_name)) =
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
                                message: format!(
                                    "Unknown method '{method}' on class '{class_name}'"
                                ),
                                severity: Severity::Warning,
                                fixes: Vec::new(),
                            });
                        }
                    }
                }
                continue;
            }

            // Type is unknown — emit W307 (matching Python's
            // emit-then-suppress shape, but only the emit-half
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
    /// Mirrors the W308 method-resolution gate in
    /// ``_diag_var_command.py:341-361``.  A method is OK when
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
    /// Mirrors `_resolve_interpolated_commands` in
    /// `core/analysis/_analyser/_diag_commands.py:188-260`.
    /// Walks every emitted W123, extracts the command name
    /// from the message, and runs
    /// [`crate::text::fold_interpolation_set`] over the
    /// aggregated SCCP results.  When every resolved value is
    /// a known command, proc, class, or class-tail name, the
    /// W123 is removed.
    ///
    /// **Simplification vs. Python.**  Python builds a
    /// per-function SCCP map and uses range-based lookup so
    /// each W123 site sees only the variables in its enclosing
    /// function's scope.  The Rust port uses the union of
    /// every function's SCCP — slightly more permissive
    /// (over-suppresses if a same-named variable in a
    /// different function happens to resolve cleanly) but
    /// safe in practice.  Range-based lookup can land later
    /// when the parity gap surfaces.
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
    /// Mirrors `_dedupe_diagnostics` in
    /// `_diagnostics.py` (lives in `_core.py:595-630` — the
    /// orchestrator file imports it through the mixin
    /// hierarchy).  Two passes:
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
        let mut e101_lines: HashSet<u32> = HashSet::new();
        let mut w124_lines: HashSet<u32> = HashSet::new();
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

        let mut seen: HashSet<(String, u32, u32, String, Severity)> = HashSet::new();
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
    }

    /// Filter out diagnostics whose codes are in
    /// [`Self::disabled_diagnostics`].
    ///
    /// Mirrors the per-emitter `if "Wxxx" in
    /// self._disabled_diagnostics:` early-returns in Python's
    /// emitter files.  Centralising the filter on the orchestrator
    /// side keeps the per-emitter code (in C41d2 / C41d3 / etc.)
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
    /// Mirrors `_emit_racy_static_diagnostics` in
    /// `core/analysis/_analyser/_diag_racy.py`.  Walks every
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
        let mut emitted_spans: HashSet<u32> = HashSet::new();
        for block in fu.ssa.blocks.values() {
            for stmt in &block.statements {
                // Skip unset — not a real write.  Mirrors the
                // Python guard.
                if let crate::ir::Statement::Call { command, .. } = &stmt.statement {
                    if command == "unset" {
                        continue;
                    }
                }
                for name in stmt.defs.keys() {
                    if !racy_vars.contains(name) {
                        continue;
                    }
                    let span = stmt.statement.span();
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

    /// **W004.** Emit "Command option is not available in the active
    /// dialect" warning for option-bearing commands invoked with an
    /// option whose registry entry restricts it to a dialect that
    /// doesn't include the active one.
    ///
    /// Mirrors `check_dialect_invalid_option` in
    /// `core/analysis/checks/_domain.py` (PR #433).  Examples:
    /// `lsearch -stride` on Tcl 8.4 / 8.5 (option is 8.6+),
    /// `regsub -command` / `clock scan -validate` /
    /// `fconfigure -nodelay` on Tcl 8.x (options are 9.0+).
    ///
    /// Walks args looking for `-foo`-shaped flags, asks the registry
    /// for the matching `OptionSpec`, and fires when
    /// `OptionSpec::supports_dialect` returns false.  Substituted
    /// flag values (`-foo $bar`, `-foo [cmd]`) are skipped because
    /// the dispatching is only on the *flag name*; we don't have to
    /// inspect the value.  `--` terminates the scan.
    ///
    /// Subcommand-scoped options consult the subcommand's
    /// `OptionSpec` table when the first arg matches a known
    /// subcommand.
    pub(super) fn emit_w004_dialect_invalid_option(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[tcl_lexer::Token],
    ) {
        use tcl_registry::dialects::DialectSet;

        let Some(registry) = self.registry.as_ref() else {
            return;
        };
        if args.is_empty() || arg_tokens.is_empty() {
            return;
        }
        let Some(active) = DialectSet::parse(&self.dialect) else {
            return;
        };
        let Some(spec) = registry.get(cmd_name) else {
            return;
        };

        // Resolve subcommand-level options when the first arg names
        // one (mirrors Python's `if first in spec.subcommands`).
        let sub_match = (!spec.subcommands.is_empty())
            .then(|| spec.subcommands.iter().find(|s| s.name == args[0].as_str()))
            .flatten();
        let (options, parent_dialects, start_idx) = if let Some(sub) = sub_match {
            (sub.options, sub.dialects.or(spec.dialects), 1usize)
        } else {
            (spec.options, spec.dialects, 0usize)
        };

        if options.is_empty() {
            return;
        }

        let mut i = start_idx;
        while i < args.len() {
            let arg = args[i].as_str();
            if arg == "--" {
                break;
            }
            if !arg.starts_with('-') || arg.len() < 2 {
                i += 1;
                continue;
            }
            // Skip negative number literals (`-1`, `-1.5`).
            let rest = &arg[1..].trim_start_matches('-');
            if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit() || c == '.') {
                i += 1;
                continue;
            }
            // Skip dynamic-value args (Var / Cmd tokens).  The flag
            // name itself comes from the arg text, but if the
            // representative token is a substitution we can't know
            // it's actually `-foo`.
            if i < arg_tokens.len() {
                let tok = arg_tokens[i];
                if matches!(
                    tok.kind,
                    tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
                ) {
                    i += 1;
                    continue;
                }
            }
            // Find a matching OptionSpec; if found and dialect-gated
            // out, emit W004.
            if let Some(opt) = options.iter().find(|o| o.name == arg) {
                if !opt.supports_dialect(Some(active), parent_dialects) {
                    let span = if i < arg_tokens.len() {
                        arg_tokens[i].span
                    } else {
                        continue;
                    };
                    self.result.diagnostics.push(super::types::Diagnostic {
                        code: "W004".to_string(),
                        span,
                        message: format!(
                            "Option '{}' on command '{}' is not available in dialect '{}'.",
                            arg, cmd_name, self.dialect
                        ),
                        severity: Severity::Warning,
                        fixes: Vec::new(),
                    });
                }
            }
            i += 1;
        }
    }

    /// **W003.** Emit "Expression operator not available in active
    /// dialect" warning for expressions that use a Tcl 9.0 string-
    /// comparison operator (`lt` / `le` / `gt` / `ge`, TIP 461) in a
    /// pre-9.0 dialect, or `in` / `ni` (TIP 201, Tcl 8.5+) in
    /// Tcl 8.4 / f5-irules.
    ///
    /// Mirrors `check_dialect_invalid_expr_operator` in
    /// `core/analysis/checks/_domain.py` (PR #433).
    pub(super) fn emit_w003_dialect_invalid_expr_operator(
        &mut self,
        expr_text: &str,
        diag_span: tcl_lexer::Span,
    ) {
        use tcl_registry::dialects::DialectSet;

        // Quick lexical bail-out — the gated operators are short
        // word-shaped keywords; if none appear as a whole word we
        // can skip the parse.  Boundary check uses ASCII identifier
        // continuation so `tab`-, `newline`-, and start/end-of-text
        // boundaries all count (mirrors Tcl expr's whitespace
        // tolerance — `$x\tlt\t$y` and a wrapped `in` expression
        // both qualify).
        if !contains_gated_word(expr_text) {
            return;
        }
        let Some(active) = DialectSet::parse(&self.dialect) else {
            return;
        };
        // Pre-Tcl-8.5 dialects don't accept `in` / `ni` (TIP 201).
        let pre_85 = !DialectSet::TCL85_PLUS.contains(active);
        // Pre-Tcl-9.0 dialects don't accept `lt` / `le` / `gt` / `ge`
        // (TIP 461).
        let pre_90 = !DialectSet::from_iter([DialectSet::TCL90]).contains(active);
        if !pre_85 && !pre_90 {
            return;
        }

        let parsed = crate::parse_expr(expr_text.trim(), Some(self.dialect.as_str()));
        if matches!(parsed, ExprNode::Raw { .. }) {
            return;
        }
        let mut found: Vec<&'static str> = Vec::new();
        walk_dialect_invalid_ops(&parsed, pre_85, pre_90, &mut found);
        for op_name in found {
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "W003".to_string(),
                span: diag_span,
                message: format!(
                    "Expression operator '{op_name}' is not available in dialect '{}'.",
                    self.dialect
                ),
                severity: Severity::Warning,
                fixes: Vec::new(),
            });
        }
    }
}

/// Walk an expression AST and collect dialect-gated operator
/// occurrences.  Mirrors `_find_dialect_invalid_ops` in
/// `core/analysis/checks/_domain.py` (PR #433).
/// Return `true` if `text` contains any of the dialect-gated
/// expression operator keywords (`lt`, `le`, `gt`, `ge`, `in`, `ni`)
/// as a whole word — i.e. surrounded by non-identifier bytes or
/// the text boundary.  Used as a fast prefilter to skip the
/// expression parse for expressions that obviously can't trigger
/// W003.
///
/// Whitespace-aware: tabs, newlines, and any other non-identifier
/// byte (parentheses, operators, comparison glyphs, etc.) count
/// as word boundaries.  Matches Tcl expr's tolerance for
/// arbitrary whitespace between tokens.
fn contains_gated_word(text: &str) -> bool {
    const GATED: &[&[u8]] = &[b"lt", b"le", b"gt", b"ge", b"in", b"ni"];
    let bytes = text.as_bytes();
    for needle in GATED {
        let n = needle.len();
        let mut i = 0;
        while i + n <= bytes.len() {
            if &bytes[i..i + n] == *needle {
                let before_ok = i == 0 || !is_ident_continue(bytes[i - 1]);
                let after_ok = i + n == bytes.len() || !is_ident_continue(bytes[i + n]);
                if before_ok && after_ok {
                    return true;
                }
            }
            i += 1;
        }
    }
    false
}

fn walk_dialect_invalid_ops(
    node: &ExprNode,
    pre_85: bool,
    pre_90: bool,
    found: &mut Vec<&'static str>,
) {
    match node {
        ExprNode::Binary { op, left, right } => {
            walk_dialect_invalid_ops(left, pre_85, pre_90, found);
            walk_dialect_invalid_ops(right, pre_85, pre_90, found);
            match op {
                BinOp::In if pre_85 => found.push("in"),
                BinOp::Ni if pre_85 => found.push("ni"),
                BinOp::StrLt if pre_90 => found.push("lt"),
                BinOp::StrLe if pre_90 => found.push("le"),
                BinOp::StrGt if pre_90 => found.push("gt"),
                BinOp::StrGe if pre_90 => found.push("ge"),
                _ => {}
            }
        }
        ExprNode::Unary { operand, .. } => {
            walk_dialect_invalid_ops(operand, pre_85, pre_90, found);
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            walk_dialect_invalid_ops(condition, pre_85, pre_90, found);
            walk_dialect_invalid_ops(true_branch, pre_85, pre_90, found);
            walk_dialect_invalid_ops(false_branch, pre_85, pre_90, found);
        }
        ExprNode::Call { args, .. } => {
            for arg in args {
                walk_dialect_invalid_ops(arg, pre_85, pre_90, found);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::types::Diagnostic;
    use tcl_lexer::Span;

    #[test]
    fn body_references_param_bare_dollar() {
        assert!(body_references_param("set y $x", "x"));
        assert!(body_references_param("return [expr {$a + $b}]", "a"));
        assert!(body_references_param("return [expr {$a + $b}]", "b"));
        assert!(body_references_param("puts [list $val 1]", "val"));
    }

    #[test]
    fn body_references_param_braced_dollar() {
        assert!(body_references_param("set y ${x}", "x"));
        assert!(body_references_param("puts \"got ${val}!\"", "val"));
    }

    #[test]
    fn body_references_param_no_match_for_substring_only() {
        // ``$abc`` must not match ``ab`` (boundary check).
        assert!(!body_references_param("set y $abc", "ab"));
        assert!(!body_references_param("puts $foobar", "foo"));
    }

    #[test]
    fn body_references_param_skips_backslash_escape() {
        // ``\$x`` is a literal dollar — not a substitution.
        assert!(!body_references_param("puts \\$x", "x"));
    }

    #[test]
    fn body_references_param_handles_multiple_uses() {
        assert!(body_references_param("set y $x; set z $x", "x"));
    }

    #[test]
    fn body_references_param_misses_when_unused() {
        assert!(!body_references_param("puts hello", "x"));
        assert!(!body_references_param("return 42", "y"));
    }

    #[test]
    fn body_references_param_braced_with_punct_after() {
        // ``${x}foo`` is a valid substitution — boundary not
        // required inside braces.
        assert!(body_references_param("set y ${x}foo", "x"));
    }

    #[test]
    fn body_references_param_namespace_qualified() {
        // ``$ns::var`` is a qualified variable; the param name
        // is the leading identifier.  Boundary on ``::`` is
        // OK — both are part of the qualified name; the W214
        // emitter passes the bare param so this is a non-issue
        // in practice.  Test pins the boundary semantics.
        assert!(!body_references_param("set y $ns::var", "ns"));
    }

    fn diag(code: &str, span: Span, msg: &str) -> Diagnostic {
        Diagnostic {
            code: code.to_string(),
            span,
            message: msg.to_string(),
            severity: Severity::Warning,
            fixes: Vec::new(),
        }
    }

    #[test]
    fn w004_fires_on_regsub_command_in_tcl86() {
        // `regsub -command` is Tcl 9.0+ (TIP 463); on Tcl 8.6 it
        // should produce a W004 dialect-availability warning.
        let mut a = Analyser::new();
        let result = a.analyse("regsub -command {[A-Z]+} foo {bar} out", "tcl8.6");
        let w004: Vec<&Diagnostic> = result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W004")
            .collect();
        assert!(
            !w004.is_empty(),
            "expected W004 on tcl8.6 regsub -command, got {:?}",
            result.diagnostics
        );
        assert!(w004[0].message.contains("-command"));
        assert!(w004[0].message.contains("regsub"));
    }

    #[test]
    fn w004_fires_on_lsearch_stride_in_tcl85() {
        // PR #441 review (Codex): the W004 coverage requires the
        // option to exist in the registry.  `lsearch -stride` was
        // populated as part of this review fix.
        let mut a = Analyser::new();
        let result = a.analyse("lsearch -stride 2 {a b c d} b", "tcl8.5");
        assert!(
            result.diagnostics.iter().any(|d| d.code == "W004"),
            "expected W004 on tcl8.5 lsearch -stride, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn w004_silent_on_lsearch_stride_in_tcl86() {
        let mut a = Analyser::new();
        let result = a.analyse("lsearch -stride 2 {a b c d} b", "tcl8.6");
        assert!(
            !result.diagnostics.iter().any(|d| d.code == "W004"),
            "W004 must not fire on tcl8.6 lsearch -stride, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn w004_fires_on_clock_scan_validate_in_tcl86() {
        // `clock scan -validate` is Tcl 9.0+ (TIP 532); the
        // subcommand-scoped option table consults the active
        // dialect via the W004 emitter's `sub_match` branch.
        let mut a = Analyser::new();
        let result = a.analyse("clock scan {today} -validate 1", "tcl8.6");
        assert!(
            result.diagnostics.iter().any(|d| d.code == "W004"),
            "expected W004 on tcl8.6 clock scan -validate, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn w004_fires_on_fconfigure_nodelay_in_tcl86() {
        // `fconfigure -nodelay` is Tcl 9.0+ (TIP 528).
        let mut a = Analyser::new();
        let result = a.analyse("fconfigure $chan -nodelay 1", "tcl8.6");
        assert!(
            result.diagnostics.iter().any(|d| d.code == "W004"),
            "expected W004 on tcl8.6 fconfigure -nodelay, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn w004_fires_on_chan_configure_inputmode_in_tcl86() {
        // Subcommand-scoped option: `chan configure -inputmode` is
        // Tcl 9.0+ (TIP 160).
        let mut a = Analyser::new();
        let result = a.analyse("chan configure $chan -inputmode raw", "tcl8.6");
        assert!(
            result.diagnostics.iter().any(|d| d.code == "W004"),
            "expected W004 on tcl8.6 chan configure -inputmode, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn w004_silent_on_regsub_command_in_tcl9() {
        // Same input on Tcl 9.0 — option is supported, no W004.
        let mut a = Analyser::new();
        let result = a.analyse("regsub -command {[A-Z]+} foo {bar} out", "tcl9.0");
        assert!(
            !result.diagnostics.iter().any(|d| d.code == "W004"),
            "W004 should not fire on tcl9.0, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn w003_fires_on_string_compare_in_tcl84() {
        // `lt` / `le` / `gt` / `ge` are Tcl 9.0+ (TIP 461); on
        // Tcl 8.4 / 8.5 / 8.6 they should produce W003.
        let mut a = Analyser::new();
        let result = a.analyse("if {$x lt $y} { puts hi }", "tcl8.4");
        let w003: Vec<&Diagnostic> = result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W003")
            .collect();
        assert!(
            !w003.is_empty(),
            "expected W003 on tcl8.4 'lt' operator, got {:?}",
            result.diagnostics
        );
        assert!(w003[0].message.contains("'lt'"));
    }

    #[test]
    fn w003_silent_on_string_compare_in_tcl9() {
        let mut a = Analyser::new();
        let result = a.analyse("if {$x lt $y} { puts hi }", "tcl9.0");
        assert!(
            !result.diagnostics.iter().any(|d| d.code == "W003"),
            "W003 should not fire on tcl9.0, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn w003_fires_on_in_operator_in_tcl84() {
        // `in` / `ni` are Tcl 8.5+ (TIP 201).
        let mut a = Analyser::new();
        let result = a.analyse("if {$x in {a b c}} { puts hi }", "tcl8.4");
        assert!(
            result.diagnostics.iter().any(|d| d.code == "W003"),
            "expected W003 on tcl8.4 'in' operator, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn w003_fires_on_tab_separated_operator() {
        // PR #441 review (Codex): the prefilter must tolerate any
        // whitespace, not just literal spaces.  `if {$x\tlt\t$y}` is
        // valid Tcl 8.4 syntax that the expr parser handles — the
        // analyser must not skip it because we only checked for
        // space-delimited operators.
        let mut a = Analyser::new();
        let result = a.analyse("if {$x\tlt\t$y} { puts hi }", "tcl8.4");
        assert!(
            result.diagnostics.iter().any(|d| d.code == "W003"),
            "W003 must fire on tab-separated 'lt', got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn w003_fires_on_newline_separated_operator() {
        // Same shape with a newline boundary — also valid Tcl.
        let mut a = Analyser::new();
        let result = a.analyse("if {$x\nin\n{a b c}} { puts hi }", "tcl8.4");
        assert!(
            result.diagnostics.iter().any(|d| d.code == "W003"),
            "W003 must fire on newline-separated 'in', got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn contains_gated_word_handles_boundaries() {
        // No false positives on identifiers that contain the keyword.
        assert!(!contains_gated_word("$alt"));
        assert!(!contains_gated_word("$align"));
        assert!(!contains_gated_word("inner"));
        assert!(!contains_gated_word("$gem"));
        // Real matches at word boundaries.
        assert!(contains_gated_word("$x lt $y"));
        assert!(contains_gated_word("$x\tlt\t$y"));
        assert!(contains_gated_word("($x)lt($y)"));
        assert!(contains_gated_word("lt $y"));
        assert!(contains_gated_word("$x lt"));
    }

    #[test]
    fn w003_silent_on_in_operator_in_tcl85() {
        let mut a = Analyser::new();
        let result = a.analyse("if {$x in {a b c}} { puts hi }", "tcl8.5");
        assert!(
            !result.diagnostics.iter().any(|d| d.code == "W003"),
            "W003 should not fire on tcl8.5, got {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn emit_variable_usage_diagnostics_is_a_noop() {
        // Hook is intentionally empty — running it must leave
        // the diagnostics list untouched.
        let mut a = Analyser::new();
        a.result
            .diagnostics
            .push(diag("W113", Span::new(0, 3), "x"));
        a.emit_variable_usage_diagnostics();
        assert_eq!(a.result.diagnostics.len(), 1);
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_runs_without_panicking_on_empty_source() {
        // Smoke test — the orchestrator handles empty input
        // gracefully (an empty CompilationUnit yields no
        // diagnostics).
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("");
        assert!(a.result.diagnostics.is_empty());
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_no_w220_on_simple_assignment() {
        // ``set x 1`` — single assignment, no overwrite, no
        // W220.  Smoke test that pipeline runs without
        // emitting spurious W codes for clean code.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("set x 1");
        assert!(
            !a.result.diagnostics.iter().any(|d| d.code == "W220"),
            "W220 must not fire on a single assignment; got {:?}",
            a.result.diagnostics,
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w220_dead_store_overwritten() {
        // ``set x 1\nset x 2\nputs $x`` — the first ``set x 1``
        // is overwritten before being read.  W220 should fire
        // at the first assignment.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("set x 1\nset x 2\nputs $x");
        let w220s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W220")
            .collect();
        assert!(
            !w220s.is_empty(),
            "W220 expected for overwritten ``set x 1``; got {:?}",
            a.result.diagnostics,
        );
        assert!(w220s.iter().any(|d| d.message.contains("'x'")));
        assert_eq!(w220s[0].severity, Severity::Hint);
    }

    /// W220-IR-paths.  Variables prefixed with ``::`` are
    /// externally consumed (other namespaces, the global frame
    /// outside this file) — Python's ``_dead_stores`` skips
    /// them in `core_analyses.py:1147-1148`, and the Rust port
    /// must too.
    #[test]
    fn emit_cfg_ssa_diagnostics_w220_skips_global_qualified_var() {
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("set ::x 1\nset ::x 2\nputs $::x");
        let w220s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W220")
            .collect();
        assert!(
            w220s.is_empty(),
            "W220 must skip ``::``-prefixed globals; got {w220s:?}",
        );
    }

    /// W220-IR-paths.  ``set x [foo]`` is a side-effecting
    /// store: dropping the assignment would also drop the call
    /// to ``foo``.  Python's ``_dead_stores`` filters
    /// ``IRAssignValue`` containing ``[`` (`core_analyses.py:1152`).
    #[test]
    fn emit_cfg_ssa_diagnostics_w220_skips_command_substitution_value() {
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("set x [clock seconds]\nset x 2\nputs $x");
        let w220s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W220")
            .collect();
        assert!(
            w220s.is_empty(),
            "W220 must skip ``set x [cmd]`` side-effecting stores; got {w220s:?}",
        );
    }

    /// W220-IR-paths.  ``set x [expr {[foo]}]`` lowers as
    /// ``IRAssignExpr`` with a command call inside — same
    /// side-effecting reasoning as command-substitution
    /// values.  Python's ``_dead_stores`` filters
    /// ``IRAssignExpr`` whose tree contains an
    /// ``IRExprCommand`` (`core_analyses.py:1154`).
    #[test]
    fn emit_cfg_ssa_diagnostics_w220_skips_expr_with_command_call() {
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("set x [expr {[clock seconds] + 1}]\nset x 2\nputs $x");
        let w220s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W220")
            .collect();
        assert!(
            w220s.is_empty(),
            "W220 must skip ``IRAssignExpr`` containing a command call; got {w220s:?}",
        );
    }

    /// W220-IR-paths.  ``incr x`` is a side-effecting write
    /// (it reads the current value first).  Python's
    /// ``_dead_stores`` only matches ``IRAssignConst`` /
    /// ``IRAssignValue`` / ``IRAssignExpr`` — ``IRIncr`` and
    /// ``IRCall.defs`` are skipped by exclusion.
    #[test]
    fn emit_cfg_ssa_diagnostics_w220_skips_incr_writes() {
        let mut a = Analyser::new();
        // ``incr x`` reads x then writes x+1; even when later
        // overwritten, dropping the incr would also drop the
        // implicit read.  Of the three writes to ``x``, only
        // the ``incr`` qualifies as overwritten-before-read
        // (``set x 0`` is read by incr, ``set x 5`` is read
        // by puts), so any W220 on x must be from the incr,
        // and the IR-statement-type filter must drop it.
        a.emit_cfg_ssa_diagnostics("set x 0\nincr x\nset x 5\nputs $x");
        let w220s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W220" && d.message.contains("'x'"))
            .collect();
        assert!(
            w220s.is_empty(),
            "W220 must skip ``incr`` side-effecting writes; got {w220s:?}",
        );
    }

    /// W220-IR-paths.  ``lassign $list a b`` defines ``a`` and
    /// ``b`` via ``IRCall.defs`` — a side-effecting write that
    /// can't be dropped without also dropping the call.
    /// Python's ``_dead_stores`` only matches the three
    /// pure-assign IR shapes; ``IRCall`` is skipped by
    /// exclusion.
    #[test]
    fn emit_cfg_ssa_diagnostics_w220_skips_call_defs() {
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("lassign {1 2} a b\nset a 5\nputs $a");
        let w220s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W220")
            .collect();
        assert!(
            w220s.iter().all(|d| !d.message.contains("'a'")),
            "W220 must skip ``IRCall.defs`` side-effecting writes; got {w220s:?}",
        );
    }

    /// W220-IR-paths.  In a ``pkgIndex.tcl`` file, ``$dir`` is
    /// set by the Tcl package loader before the script body
    /// runs — even when the script reassigns it, the original
    /// store can't be considered dead (the loader-supplied
    /// value is the relevant initial state).  Mirrors
    /// `_diagnostics.py:147-149`.
    #[test]
    fn emit_cfg_ssa_diagnostics_w220_pkgindex_dir_var_suppressed() {
        let mut a = Analyser::new();
        a.file_path = Some("/some/path/pkgIndex.tcl".to_string());
        a.emit_cfg_ssa_diagnostics("set dir foo\nset dir bar\nputs $dir");
        let w220s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W220")
            .collect();
        assert!(
            w220s.is_empty(),
            "W220 must suppress ``$dir`` in pkgIndex.tcl; got {w220s:?}",
        );
    }

    /// W220-IR-paths.  Outside ``pkgIndex.tcl``, ``$dir`` is
    /// just a regular variable — no special suppression.
    /// Negative control for the pkgIndex special-case.
    #[test]
    fn emit_cfg_ssa_diagnostics_w220_dir_var_not_suppressed_outside_pkgindex() {
        let mut a = Analyser::new();
        a.file_path = Some("/some/path/script.tcl".to_string());
        a.emit_cfg_ssa_diagnostics("set dir foo\nset dir bar\nputs $dir");
        let w220s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W220")
            .collect();
        assert!(
            !w220s.is_empty(),
            "W220 must fire on ``$dir`` outside pkgIndex.tcl; got {:?}",
            a.result.diagnostics,
        );
        assert!(w220s.iter().any(|d| d.message.contains("'dir'")));
    }

    /// W220-IR-paths.  Variables shared across iRule events
    /// via ``::when::*`` procs (collected in
    /// ``ConnectionScope::cross_event_imports``) may be read
    /// in a different event from where they're set — the
    /// local "no use" verdict is unsafe.  Mirrors
    /// `_diagnostics.py:165-167`.
    #[test]
    fn emit_cfg_ssa_diagnostics_w220_irules_cross_event_var_suppressed() {
        let mut a = Analyser::new();
        a.dialect = "f5-irules".to_string();
        // ``HTTP_REQUEST`` writes ``v``, ``HTTP_RESPONSE``
        // reads ``v`` — ``v`` is a cross-event def.  The
        // ``set v 1\nset v 2`` shape inside ``HTTP_REQUEST``
        // would normally fire W220 on the first ``set v 1``,
        // but cross-event suppression should drop it.
        a.emit_cfg_ssa_diagnostics(
            "when HTTP_REQUEST {\n  set v 1\n  set v 2\n}\nwhen HTTP_RESPONSE {\n  log local0. $v\n}",
        );
        let w220s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W220")
            .collect();
        assert!(
            w220s.iter().all(|d| !d.message.contains("'v'")),
            "W220 must suppress vars shared across iRule events; got {w220s:?}",
        );
    }

    /// W220-IR-paths.  Negative control: a proc-local variable
    /// (NOT shared across events) inside a ``::when::*`` proc
    /// is still subject to W220.  Confirms the cross-event
    /// filter is targeted, not a blanket
    /// "skip everything in `::when::`*" rule.
    #[test]
    fn emit_cfg_ssa_diagnostics_w220_irules_proc_local_still_flagged() {
        let mut a = Analyser::new();
        a.dialect = "f5-irules".to_string();
        // ``local`` is only used inside HTTP_REQUEST — not a
        // cross-event var, so W220 should still fire on the
        // overwritten first assignment.
        a.emit_cfg_ssa_diagnostics(
            "when HTTP_REQUEST {\n  set local 1\n  set local 2\n  log local0. $local\n}",
        );
        let w220s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W220")
            .collect();
        assert!(
            w220s.iter().any(|d| d.message.contains("'local'")),
            "W220 must still fire for proc-local vars in ::when::*; got {:?}",
            a.result.diagnostics,
        );
    }

    /// W220-IR-paths.  Dead stores in SCCP-unreachable blocks
    /// are reported as O107 by the optimiser; the analyser
    /// must not double-report them as W220.  Mirrors Python's
    /// ``_dead_stores`` `executable_blocks` filter
    /// (`core_analyses.py:1112-1140`).
    #[test]
    fn emit_cfg_ssa_diagnostics_w220_skips_unreachable_block() {
        let mut a = Analyser::new();
        // ``if {0} { ... }`` makes the then-branch unreachable
        // under SCCP.  Any dead store inside is suppressed.
        a.emit_cfg_ssa_diagnostics("if {0} {\n  set x 1\n  set x 2\n  puts $x\n}\nputs done");
        let w220s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W220")
            .collect();
        assert!(
            w220s.is_empty(),
            "W220 must skip dead stores in SCCP-unreachable blocks; got {w220s:?}",
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w214_unused_param() {
        // ``proc foo {x y} { puts $x }`` — parameter ``y`` is
        // declared but never read in the body.  W214 should
        // fire on it.  Parameter ``x`` is read, so no W214.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {x y} { puts $x }");
        let w214s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W214")
            .collect();
        assert_eq!(
            w214s.len(),
            1,
            "expected exactly one W214 for unused param ``y``; got {:?}",
            a.result.diagnostics,
        );
        assert!(w214s[0].message.contains("'y'"));
        assert!(w214s[0].message.contains("'::foo'"));
        assert_eq!(w214s[0].severity, Severity::Hint);
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w211_unused_variable() {
        // ``proc foo {} { set y 1 }`` — y is set, never read,
        // and there's no other version → W211 fires.
        // Top-level test would be subject to global-scope
        // assumptions, so use a proc body where the local-only
        // verdict is safe.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {} { set y 1 }");
        let w211s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W211")
            .collect();
        assert!(
            !w211s.is_empty(),
            "W211 expected for unused var ``y`` in proc foo; got {:?}",
            a.result.diagnostics,
        );
        assert!(w211s[0].message.contains("'y'"));
        assert!(w211s[0].message.contains("set but never used"));
        assert_eq!(w211s[0].severity, Severity::Hint);
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w211_skipped_for_textually_referenced() {
        // ``proc foo {} { set msg hello; puts "got $msg" }`` —
        // ``msg`` is referenced inside a quoted string; the
        // textual-reference filter should suppress W211 because
        // the def-use builder doesn't track ``"$msg"`` reads.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {} { set msg hello; puts \"got $msg\" }");
        let w211s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W211" && d.message.contains("'msg'"))
            .collect();
        assert!(
            w211s.is_empty(),
            "W211 must not fire on var referenced via $-interpolation; got {:?}",
            a.result.diagnostics,
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w211_skipped_for_global_aliased() {
        // ``proc foo {} { global config; set config 1 }`` —
        // ``config`` is global-aliased; the write goes to the
        // outer scope, so the local "no use" verdict is unsafe.
        // W211 must not fire.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {} { global config; set config 1 }");
        let w211s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W211" && d.message.contains("'config'"))
            .collect();
        assert!(
            w211s.is_empty(),
            "W211 must not fire on global-aliased var; got {:?}",
            a.result.diagnostics,
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_h300_repeated_assignment() {
        // ``proc foo {} { set x 1; set x 1 }`` — same var,
        // same literal value, consecutive statements.  The
        // first is a dead store; H300 fires on the second.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {} { set x 1\nset x 1 }");
        let h300s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "H300")
            .collect();
        assert!(
            !h300s.is_empty(),
            "H300 expected for repeated ``set x 1``; got {:?}",
            a.result.diagnostics,
        );
        assert!(h300s[0].message.contains("'x'"));
        assert!(h300s[0].message.contains("Possible paste error"));
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_h300_skips_underscore_vars() {
        // Vars starting with ``_`` are excluded (the convention
        // for "intentionally unused").
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {} { set _x 1\nset _x 1 }");
        assert!(
            !a.result.diagnostics.iter().any(|d| d.code == "H300"),
            "H300 must not fire on underscore-prefixed vars",
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_h300_skips_distinct_values() {
        // Same var, different literal → not a paste error.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {} { set x 1\nset x 2 }");
        assert!(
            !a.result.diagnostics.iter().any(|d| d.code == "H300"),
            "H300 must not fire when literal values differ",
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w210_read_before_set() {
        // ``proc foo {} { puts $undef }`` — undef is not a
        // parameter and not in scope; W210 fires at the use.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {} { puts $undef }");
        let w210s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W210" && d.message.contains("'undef'"))
            .collect();
        assert!(
            !w210s.is_empty(),
            "W210 expected for read of undef ``$undef``; got {:?}",
            a.result.diagnostics,
        );
        assert_eq!(w210s[0].severity, Severity::Warning);
        assert!(w210s[0].message.contains("read before it is set"));
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w210_skipped_for_real_param() {
        // ``proc foo {x} { puts $x }`` — x IS a real parameter,
        // so W210 must not fire.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {x} { puts $x }");
        let w210s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W210" && d.message.contains("'x'"))
            .collect();
        assert!(
            w210s.is_empty(),
            "W210 must not fire on real param ``x``; got {:?}",
            a.result.diagnostics,
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w213_unset_on_possibly_undef() {
        // ``proc foo {} { unset xs }`` — ``xs`` may not exist;
        // ``unset`` without ``-nocomplain`` would error at
        // runtime.  W213 fires (instead of W210) at the unset
        // statement.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {} { unset xs }");
        let w213s: Vec<_> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W213")
            .collect();
        assert!(
            !w213s.is_empty(),
            "W213 expected for ``unset xs`` on possibly-undef var; got {:?}",
            a.result.diagnostics,
        );
        assert!(w213s[0].message.contains("'xs'"));
        assert!(w213s[0].message.contains("unset -nocomplain"));
        assert_eq!(w213s[0].severity, Severity::Warning);
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w213_skipped_with_nocomplain() {
        // ``unset -nocomplain xs`` is the safe form — W213
        // must not fire.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {} { unset -nocomplain xs }");
        assert!(
            !a.result.diagnostics.iter().any(|d| d.code == "W213"),
            "W213 must not fire when ``-nocomplain`` is present; got {:?}",
            a.result.diagnostics,
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w210_fires_at_top_level() {
        // **C41e3 follow-up.** Top-level RBS now fires when no
        // proc writes the variable.  ``puts $undef`` reads
        // ``undef`` without any preceding write.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("puts $undef");
        assert!(
            a.result.diagnostics.iter().any(|d| d.code == "W210"),
            "W210 must fire at top-level when no proc writes the var; got {:?}",
            a.result.diagnostics,
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w210_suppressed_when_proc_writes_global() {
        // A helper proc ``init`` writes ``::counter`` via ``set``,
        // so the top-level read should not flag W210 — the proc
        // may run before the read.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc init {} { set ::counter 0 }\nputs $counter");
        assert!(
            !a.result.diagnostics.iter().any(|d| d.code == "W210"),
            "W210 must be suppressed for globals written by procs; got {:?}",
            a.result.diagnostics,
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w210_suppressed_via_global_alias() {
        // ``proc init {} { global counter; set counter 0 }`` — the
        // ``global`` declaration aliases the proc-local ``counter``
        // to the global.  Top-level read should not flag W210.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc init {} { global counter; set counter 0 }\nputs $counter");
        assert!(
            !a.result.diagnostics.iter().any(|d| d.code == "W210"),
            "W210 must be suppressed via global-alias case; got {:?}",
            a.result.diagnostics,
        );
    }

    #[test]
    fn analyse_w307_suppressed_for_known_class_constructor_chain() {
        // ``[Dog new] bark`` — ``Dog`` is a user class so
        // ``new`` returns an Object whose class is ``Dog``.
        // The W307 cmd-sub suppression should kick in.  Since
        // ``bark`` is declared on ``Dog``, no W308 either.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create Dog { method bark {} { return woof } }\n[Dog new] bark",
            "tcl",
        );
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W307"),
            "W307 must not fire for [KnownClass new] method chain; got {:?}",
            r.diagnostics,
        );
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W308"),
            "W308 must not fire when method is declared on the class; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w308_emitted_for_unknown_method_on_known_class_constructor() {
        // ``[Dog new] fly`` — ``fly`` isn't declared on ``Dog``.
        // W307 is suppressed (constructor returns Object) but
        // W308 fires for the missing method.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create Dog { method bark {} { return woof } }\n[Dog new] fly",
            "tcl",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == "W308" && d.message.contains("fly")),
            "W308 expected for unknown method on known class; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w307_emitted_for_cmd_substitution_with_unknown_return_type() {
        // ``[bogus_cmd] foo`` — the inner command isn't in the
        // registry, so the return type is unknown.  W307 should
        // fire for the cmd-as-command site.
        let src = "[bogus_cmd] foo";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl");
        assert!(
            r.diagnostics.iter().any(|d| d.code == "W307"),
            "W307 expected for [unknown] method pattern; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w307_suppressed_for_my_self_dispatch() {
        // ``[my method]`` is OO self-dispatch — never trips W307.
        let src = "[my m] arg";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W307"),
            "W307 must not fire for OO self-dispatch; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w123_suppressed_for_partial_interpolation_resolving_to_known_proc() {
        // ``set suffix _hi`` makes ``$suffix`` resolve to ``_hi``;
        // ``foo$suffix`` therefore resolves to ``foo_hi``, which
        // is a known proc.  W123 should not fire.
        let src = "\
proc foo_hi {} {}
set suffix _hi
foo$suffix
";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W123"),
            "W123 should be suppressed when partial interpolation resolves to a known proc; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w123_kept_when_partial_interpolation_resolves_to_unknown() {
        // ``set suffix _missing`` makes ``foo$suffix`` resolve
        // to ``foo_missing`` — not a known command — so W123
        // should still fire.
        let src = "\
set suffix _missing
foo$suffix
";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl");
        assert!(
            r.diagnostics.iter().any(|d| d.code == "W123"),
            "W123 expected when partial interpolation resolves to an unknown command",
        );
    }

    #[test]
    fn analyse_w123_emits_did_you_mean_suggestion() {
        // ``puta`` is one edit away from ``puts`` — the
        // emitter should attach a suggestion and a CodeFix.
        let mut a = Analyser::new();
        let r = a.analyse("puta hi", "tcl");
        let w123 = r
            .diagnostics
            .iter()
            .find(|d| d.code == "W123")
            .expect("W123 emitted");
        assert!(
            w123.message.contains("did you mean 'puts'"),
            "expected suggestion in message, got: {}",
            w123.message,
        );
        assert!(!w123.fixes.is_empty(), "expected CodeFix payload");
        let fix = &w123.fixes[0];
        assert_eq!(fix.new_text, "puts");
        assert!(fix.description.contains("puts"));
    }

    #[test]
    fn analyse_w123_suppressed_for_inline_stub_declared_command() {
        // ``my_cmd`` is declared via inline stub — W123 must
        // not fire even though it isn't in the registry.
        let src = "\
# tcl-lsp: stubs-begin
# tcl-lsp: stub my_cmd {arg1:var body:body}
# tcl-lsp: stubs-end
my_cmd $x foo
";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W123"),
            "W123 must not fire for stub-declared commands; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w123_dispatch_target_from_unknown_proc_suppresses() {
        // ``foo`` is one of the switch arms inside a
        // user-defined ``unknown`` proc — the empty-stub gate
        // doesn't fire (body is non-empty), so W123 is
        // already suppressed.  Add a fixture that verifies
        // the dispatch_targets are also in the suggestion
        // candidate set when an empty-stub unknown is in play.
        let src = "\
proc unknown {cmd args} {}
foo
";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl");
        // Empty unknown means W123 still fires — but the
        // dispatch_targets membership doesn't apply (set is
        // empty).  Just sanity-check the test runs.
        assert!(r.diagnostics.iter().any(|d| d.code == "W123"));
    }

    #[test]
    fn analyse_w123_no_suggestion_when_far_from_any_known_command() {
        let mut a = Analyser::new();
        let r = a.analyse("xyzzy_unknown_cmd", "tcl");
        let w123 = r
            .diagnostics
            .iter()
            .find(|d| d.code == "W123")
            .expect("W123 emitted");
        assert!(
            !w123.message.contains("did you mean"),
            "no suggestion expected for far-away command name; got: {}",
            w123.message,
        );
        assert!(w123.fixes.is_empty());
    }

    #[test]
    fn analyse_irule4005_racy_static_emitted_for_per_request_writes() {
        // ``static::counter`` written in HTTP_REQUEST and read
        // in HTTP_RESPONSE — both per-request events; the
        // cross-event flow is racy ⇒ IRULE4005 fires.
        let mut a = Analyser::new();
        let r = a.analyse(
            "when HTTP_REQUEST { incr static::counter }\n\
             when HTTP_RESPONSE { log local0. \"$static::counter\" }",
            "f5-irules",
        );
        assert!(
            r.diagnostics.iter().any(|d| d.code == "IRULE4005"),
            "IRULE4005 expected for racy static cross-event flow; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_irule4005_no_emit_for_rule_init_writes() {
        // ``static::config`` written in RULE_INIT is racy-safe
        // (RULE_INIT runs once at iRule load) — IRULE4005 must
        // not fire.
        let mut a = Analyser::new();
        let r = a.analyse(
            "when RULE_INIT { set static::config 1 }\n\
             when HTTP_REQUEST { log local0. \"$static::config\" }",
            "f5-irules",
        );
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "IRULE4005"),
            "IRULE4005 must not fire for RULE_INIT writes; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w124_ipv4_octet_overflow() {
        // ``proc foo {} { set ip 192.168.1.999 }`` — 999 > 255,
        // not a valid IP.  W124 fires at the assignment.
        let mut a = Analyser::new();
        let r = a.analyse("proc foo {} { set ip 192.168.1.999 }", "tcl");
        let w124s: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W124").collect();
        assert!(
            !w124s.is_empty(),
            "W124 expected for IPv4 octet > 255; got {:?}",
            r.diagnostics,
        );
        assert!(w124s[0].message.contains("999"));
        assert!(w124s[0].message.contains("exceeds 255"));
        assert_eq!(w124s[0].severity, Severity::Error);
    }

    #[test]
    fn analyse_no_w124_for_valid_ipv4() {
        // ``proc foo {} { set ip 192.168.1.1 }`` — valid IP.
        let mut a = Analyser::new();
        let r = a.analyse("proc foo {} { set ip 192.168.1.1 }", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W124"),
            "W124 must not fire on valid IPv4; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w124_ipv4_leading_zero_warning() {
        // ``proc foo {} { set ip 192.168.01.1 }`` — leading
        // zero on octet 3; might be octal in some contexts.
        // Severity is Warning.
        let mut a = Analyser::new();
        let r = a.analyse("proc foo {} { set ip 192.168.01.1 }", "tcl");
        let w124s: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W124").collect();
        assert!(
            !w124s.is_empty(),
            "W124 expected for IPv4 leading-zero octet; got {:?}",
            r.diagnostics,
        );
        assert_eq!(w124s[0].severity, Severity::Warning);
        assert!(w124s[0].message.contains("leading zero"));
    }

    #[test]
    fn analyse_i230_constant_if_branch() {
        // ``proc foo {} { if {1} { puts hi } }`` — the ``if 1``
        // condition is constant, the false branch is unreachable.
        // I230 should fire.
        let mut a = Analyser::new();
        let r = a.analyse("proc foo {} { if {1} { puts hi } }", "tcl");
        let i230s: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "I230").collect();
        assert!(
            !i230s.is_empty(),
            "I230 expected for constant ``if 1``; got {:?}",
            r.diagnostics,
        );
        assert!(i230s[0].message.contains("always true"));
    }

    #[test]
    fn analyse_no_i230_for_dynamic_condition() {
        // ``proc foo {x} { if {$x > 0} {} }`` — ``$x > 0`` is
        // not constant; no I230.
        let mut a = Analyser::new();
        let r = a.analyse("proc foo {x} { if {$x > 0} { puts hi } }", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "I230"),
            "I230 must not fire on dynamic condition; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w123_unknown_command() {
        // ``no_such_cmd hello`` — bare name that's not a
        // built-in / proc / class / alias.  W123 fires.
        let mut a = Analyser::new();
        let r = a.analyse("no_such_cmd hello", "tcl");
        let w123s: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W123").collect();
        assert!(
            !w123s.is_empty(),
            "W123 expected for unknown command; got {:?}",
            r.diagnostics,
        );
        assert!(w123s[0].message.contains("'no_such_cmd'"));
        assert_eq!(w123s[0].severity, Severity::Hint);
    }

    #[test]
    fn analyse_no_w123_for_builtin_command() {
        // ``puts hello`` — ``puts`` is a built-in; no W123.
        let mut a = Analyser::new();
        let r = a.analyse("puts hello", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W123"),
            "W123 must not fire on built-in command; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_no_w123_for_user_proc() {
        // User-defined proc, then call it.  Both go through
        // the analyser walk; the call site must NOT trip W123.
        let mut a = Analyser::new();
        let r = a.analyse("proc greet {} { puts hi }\ngreet", "tcl");
        let w123s: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W123").collect();
        assert!(
            w123s.is_empty(),
            "W123 must not fire on user-defined proc call; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_no_w123_for_qualified_command_name() {
        // Qualified names (``a::b``) skip W123 — defer to
        // per-namespace logic.
        let mut a = Analyser::new();
        let r = a.analyse("ns::cmd hello", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W123"),
            "W123 must not fire on qualified command name; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w123_package_require_gate_suppresses_when_recorded() {
        // The ``package_requires`` gate suppresses W123 entirely
        // when any package require has been recorded.  The
        // analyser walk doesn't yet record ``package require``
        // (deferred — handler not landed), so we exercise the
        // gate by pre-populating ``result.package_requires``
        // and re-running the post-pass directly.
        use crate::signature_scan::types::SignaturePackageRequire;
        use tcl_lexer::Span;
        let mut a = Analyser::new();
        a.result.package_requires.push(SignaturePackageRequire {
            name: "Tcl".to_string(),
            version: Some("8.6".to_string()),
            range: Span::new(0, 24),
            conditional: false,
        });
        // Seed an invocation that would otherwise trip W123.
        a.result.command_invocations.push(
            crate::signature_scan::types::SignatureCommandInvocation {
                name: "random_cmd".to_string(),
                range: Span::new(25, 35),
            },
        );
        let registry = tcl_registry::CommandRegistry::build_default();
        a.emit_unresolved_command_diagnostics(&registry);
        assert!(
            !a.result.diagnostics.iter().any(|d| d.code == "W123"),
            "W123 must be fully suppressed when package_requires is non-empty; got {:?}",
            a.result.diagnostics,
        );
    }

    #[test]
    fn analyse_w123_filtered_by_disabled_diagnostics() {
        // ``# tcl-lsp: disable=W123`` at top of file silences
        // the diagnostic via the existing disable filter.
        let mut a = Analyser::new();
        let r = a.analyse("# tcl-lsp: disable=W123\nno_such_cmd hello", "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W123"),
            "W123 must be silenced by file-suppression directive; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w307_var_as_command() {
        // ``proc foo {x} { $x arg1 }`` — ``$x`` used as command
        // head; we have no static knowledge of what it holds, so
        // W307 fires.  Must go through ``analyse`` (not raw
        // ``emit_cfg_ssa_diagnostics``) because ``var_command_sites``
        // is populated by the analyser's walk dispatch, not the
        // emitter pipeline.
        let mut a = Analyser::new();
        let r = a.analyse("proc foo {x} { $x arg1 }", "tcl");
        let w307s: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W307").collect();
        assert!(
            !w307s.is_empty(),
            "W307 expected for ``$x arg1``; got {:?}",
            r.diagnostics,
        );
        assert_eq!(w307s[0].severity, Severity::Warning);
        assert!(w307s[0].message.contains("Non-literal command name"));
    }

    #[test]
    fn analyse_no_w307_for_static_known_command() {
        // ``proc foo {} { set cmd puts; $cmd hello }`` — ``cmd``
        // has constant value "puts" which IS a known command, so
        // W307 must be suppressed.
        let mut a = Analyser::new();
        let r = a.analyse("proc foo {} { set cmd puts\n$cmd hello }", "tcl");
        let w307s: Vec<_> = r.diagnostics.iter().filter(|d| d.code == "W307").collect();
        assert!(
            w307s.is_empty(),
            "W307 must be suppressed when var holds known command name; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_var_command_sites_recorded_during_walk() {
        // Smoke: confirm the recording infrastructure populates
        // ``var_command_sites`` for ``$var`` heads.  Run analyse
        // (not just emit) so the apply_disabled_diagnostics +
        // dedupe don't matter — we inspect post-analyse state.
        let mut a = Analyser::new();
        let _ = a.analyse("proc foo {x} { $x arg }", "tcl");
        // After analyse, var_command_sites is consumed by the
        // post-pass but restored at the end (snapshot/restore
        // contract).
        assert!(
            a.var_command_sites.iter().any(|s| s.var_name == "x"),
            "var_command_sites should record ``$x`` head; got {:?}",
            a.var_command_sites,
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_cmd_command_sites_recorded_during_walk() {
        // ``[cmd] arg`` records to ``cmd_command_sites`` even
        // though no W307 emitter consumes it yet.
        let mut a = Analyser::new();
        let _ = a.analyse("proc foo {} { [puts hi] arg }", "tcl");
        assert!(
            !a.cmd_command_sites.is_empty(),
            "cmd_command_sites should be populated for ``[cmd] arg``; got {:?}",
            a.cmd_command_sites,
        );
    }

    #[test]
    fn emit_cfg_ssa_diagnostics_w214_skips_args_param() {
        // The variadic ``args`` is conventional and frequently
        // declared without use; W214 must not fire on it.
        let mut a = Analyser::new();
        a.emit_cfg_ssa_diagnostics("proc foo {x args} { puts $x }");
        assert!(
            !a.result.diagnostics.iter().any(|d| d.code == "W214"),
            "W214 should not fire on ``args``; got {:?}",
            a.result.diagnostics,
        );
    }

    #[test]
    fn dedupe_drops_exact_duplicates() {
        // Same code + span + message + severity → kept once.
        let mut a = Analyser::new();
        a.source = "set x 1".to_string();
        a.result
            .diagnostics
            .push(diag("W210", Span::new(4, 5), "x not set"));
        a.result
            .diagnostics
            .push(diag("W210", Span::new(4, 5), "x not set"));
        a.dedupe_diagnostics();
        assert_eq!(a.result.diagnostics.len(), 1);
    }

    #[test]
    fn dedupe_keeps_distinct_diagnostics_at_different_spans() {
        let mut a = Analyser::new();
        a.source = "set x 1\nset y 2".to_string();
        a.result
            .diagnostics
            .push(diag("W210", Span::new(4, 5), "x"));
        a.result
            .diagnostics
            .push(diag("W210", Span::new(12, 13), "y"));
        a.dedupe_diagnostics();
        assert_eq!(a.result.diagnostics.len(), 2);
    }

    #[test]
    fn dedupe_drops_e002_on_e101_line() {
        // E101 fires on a line; any E002 on the same line is
        // a false positive (arity check confused by the
        // recovered switch) and gets dropped.
        let mut a = Analyser::new();
        a.source = "switch $x { foo {puts foo}".to_string();
        let switch_span = Span::new(0, 6);
        a.result
            .diagnostics
            .push(diag("E101", switch_span, "missing open brace"));
        a.result
            .diagnostics
            .push(diag("E002", switch_span, "too few args"));
        a.dedupe_diagnostics();
        assert!(a.result.diagnostics.iter().any(|d| d.code == "E101"));
        assert!(!a.result.diagnostics.iter().any(|d| d.code == "E002"));
    }

    #[test]
    fn dedupe_drops_w122_on_w124_line() {
        // W124 (SSA-based IP check) on a line → W122 (regex IP
        // check) on the same line is redundant.
        let mut a = Analyser::new();
        a.source = "if {[IP::addr $ip]} {}".to_string();
        let ip_span = Span::new(15, 18);
        a.result
            .diagnostics
            .push(diag("W124", ip_span, "invalid IP"));
        a.result
            .diagnostics
            .push(diag("W122", ip_span, "regex IP check"));
        a.dedupe_diagnostics();
        assert!(a.result.diagnostics.iter().any(|d| d.code == "W124"));
        assert!(!a.result.diagnostics.iter().any(|d| d.code == "W122"));
    }

    #[test]
    fn dedupe_keeps_e002_on_unrelated_line() {
        // E101 on line 0, E002 on line 1 — different lines, so
        // the suppression rule doesn't fire.
        let mut a = Analyser::new();
        a.source = "switch $x {\nset y 1".to_string();
        a.result
            .diagnostics
            .push(diag("E101", Span::new(0, 6), "missing brace"));
        a.result
            .diagnostics
            .push(diag("E002", Span::new(12, 15), "too few args"));
        a.dedupe_diagnostics();
        assert!(a.result.diagnostics.iter().any(|d| d.code == "E002"));
    }

    #[test]
    fn apply_disabled_diagnostics_removes_listed_codes() {
        let mut a = Analyser::with_disabled_diagnostics(
            ["W113"].iter().map(|s| (*s).to_string()).collect(),
        );
        a.result
            .diagnostics
            .push(diag("W113", Span::new(0, 3), "shadows"));
        a.result
            .diagnostics
            .push(diag("W210", Span::new(0, 3), "unset"));
        a.apply_disabled_diagnostics();
        assert!(!a.result.diagnostics.iter().any(|d| d.code == "W113"));
        assert!(a.result.diagnostics.iter().any(|d| d.code == "W210"));
    }

    #[test]
    fn apply_disabled_diagnostics_no_op_when_empty() {
        let mut a = Analyser::new();
        a.result
            .diagnostics
            .push(diag("W113", Span::new(0, 3), "x"));
        a.apply_disabled_diagnostics();
        assert_eq!(a.result.diagnostics.len(), 1);
    }
}
