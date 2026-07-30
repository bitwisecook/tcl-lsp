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

//! Recursive IR and expression tree helpers for the CFG builder.
//!
//! These extract variable definitions from structured IR scripts and
//! expression trees. Used at `catch`/`try` merge points to
//! conservatively invalidate variables that may have been partially
//! modified before an exception, and at condition sites to track
//! definitions produced by command substitutions.

use tcl_lexer::{Lexer, SourceMap, TokenType};
use tcl_registry::{ArgRole, CommandRegistry, Traits};

use crate::depth_guard::{MAX_BRACKET_TEXT_DEPTH, MAX_EXPR_NODE_DEPTH};
use crate::expr_ast::ExprNode;
use crate::ir::{Script, Statement};
use crate::naming::normalise_var_name;

/// The nested script bodies a structured-control-flow statement contains —
/// every shape a flow-*insensitive* whole-body walk needs to recurse into
/// (`If`/`For`/`While`/`Foreach`/`Catch`/`Block`/`UpFrame`/`Switch`/`Try`).
/// `pub(crate)` so both [`crate::cfg_builder::global_write_info`] (per-proc
/// global-write summaries) and [`crate::var_observability`] (module-wide
/// trace-target summaries) share one recursive-descent shape instead of each
/// re-deriving it.
///
/// Distinct from [`collect_defs_from_script`]'s per-statement match: that
/// walk also extracts *defs* at each nesting level (iteration variables,
/// catch/try result vars) as part of the same pass, so it isn't a drop-in
/// replacement for a caller that only wants "which scripts nest inside this
/// statement".
#[must_use]
pub(crate) fn nested_bodies(stmt: &Statement) -> Vec<&Script> {
    match stmt {
        Statement::If {
            clauses, else_body, ..
        } => {
            let mut bodies: Vec<&Script> = clauses.iter().map(|c| &c.body).collect();
            if let Some(e) = else_body {
                bodies.push(e);
            }
            bodies
        }
        Statement::For {
            init, next, body, ..
        } => vec![init, next, body],
        Statement::While { body, .. }
        | Statement::Foreach { body, .. }
        | Statement::Catch { body, .. }
        | Statement::Block { body, .. }
        | Statement::UpFrame { body, .. } => vec![body],
        Statement::Switch {
            arms, default_body, ..
        } => {
            let mut bodies: Vec<&Script> = arms.iter().filter_map(|a| a.body.as_ref()).collect();
            if let Some(d) = default_body {
                bodies.push(d);
            }
            bodies
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            let mut bodies = vec![body];
            bodies.extend(handlers.iter().map(|h| &h.body));
            if let Some(f) = finally_body {
                bodies.push(f);
            }
            bodies
        }
        _ => Vec::new(),
    }
}

/// Depth cap for [`collect_defs_from_script`]'s recursion over nested
/// `if`/`for`/`while`/`foreach`/`catch`/`try`/`switch` bodies — issue #996.
/// Transitively bounded today via `MAX_LOWER_NEST_DEPTH` (every `Script`
/// feeding SSA construction was built by [`crate::lowering`], which already
/// caps its own construction at 256), capped here independently for
/// defence-in-depth and consistency with every other full-tree walker in
/// this crate.
const MAX_DEFS_COLLECT_DEPTH: tcl_core_types::RecursionLimit = tcl_core_types::RecursionLimit(256);

/// Collect all variable names defined anywhere inside a script (recursive).
///
/// Walks through structured IR nodes (`If`, `For`, `While`, `Foreach`,
/// `Catch`, `Try`, `Switch`) to find every assignment, call-def, and
/// iteration variable.
#[must_use]
pub fn defs_from_ir_script(script: &Script) -> Vec<String> {
    let mut defs = Vec::new();
    collect_defs_from_script(script, &mut defs, 0);
    defs
}

#[allow(clippy::too_many_lines)] // the depth-cap check adds a few lines over the threshold
fn collect_defs_from_script(script: &Script, defs: &mut Vec<String>, depth: u32) {
    if MAX_DEFS_COLLECT_DEPTH.exceeded(depth) {
        return;
    }
    for stmt in &script.statements {
        match stmt {
            Statement::AssignConst {
                name, name_braced, ..
            }
            | Statement::AssignExpr {
                name, name_braced, ..
            }
            | Statement::AssignValue {
                name, name_braced, ..
            }
            | Statement::Incr {
                name, name_braced, ..
            } => {
                // Element-qualified to match the SSA use scan: a collapsed
                // arm's `set b(k) 1; puts $b(k)` must cancel its own read.
                let n = crate::naming::element_var_name_braced(name, *name_braced);
                if !n.is_empty() {
                    defs.push(n.to_owned());
                }
            }

            Statement::Call {
                defs: call_defs, ..
            } if !call_defs.is_empty() => {
                defs.extend_from_slice(call_defs);
            }

            Statement::If {
                clauses, else_body, ..
            } => {
                for clause in clauses {
                    collect_defs_from_script(&clause.body, defs, depth + 1);
                }
                if let Some(eb) = else_body {
                    collect_defs_from_script(eb, defs, depth + 1);
                }
            }

            Statement::For { body, .. } | Statement::While { body, .. } => {
                collect_defs_from_script(body, defs, depth + 1);
            }

            Statement::Foreach {
                iterators, body, ..
            } => {
                for iter in iterators {
                    for vn in &iter.vars {
                        let n = crate::naming::element_var_name(vn);
                        if !n.is_empty() {
                            defs.push(n.to_owned());
                        }
                    }
                }
                collect_defs_from_script(body, defs, depth + 1);
            }

            Statement::Catch {
                body,
                result_var,
                options_var,
                ..
            } => {
                collect_defs_from_script(body, defs, depth + 1);
                if let Some(rv) = result_var {
                    defs.push(rv.clone());
                }
                if let Some(ov) = options_var {
                    defs.push(ov.clone());
                }
            }

            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                collect_defs_from_script(body, defs, depth + 1);
                for handler in handlers {
                    if let Some(vn) = &handler.var_name {
                        defs.push(vn.clone());
                    }
                    if let Some(ov) = &handler.options_var {
                        defs.push(ov.clone());
                    }
                    collect_defs_from_script(&handler.body, defs, depth + 1);
                }
                if let Some(fb) = finally_body {
                    collect_defs_from_script(fb, defs, depth + 1);
                }
            }

            Statement::Switch {
                arms, default_body, ..
            } => {
                for arm in arms {
                    if let Some(body) = &arm.body {
                        collect_defs_from_script(body, defs, depth + 1);
                    }
                }
                if let Some(db) = default_body {
                    collect_defs_from_script(db, defs, depth + 1);
                }
            }

            _ => {}
        }
    }
}

/// Extract variable names defined by command substitutions in an expression.
///
/// Walks the expression tree looking for `[cmd ...]` substitutions
/// where the command has `ArgRole::VarWrite` arguments (e.g. `set`,
/// `gets`, `regexp`, `scan`). Also scans `ArgRole::Body` arguments
/// for nested variable definitions so that patterns like
/// `[catch {set x [foo]}]` correctly report `x`.
#[must_use]
pub fn defs_from_expr(expr: &ExprNode, registry: &CommandRegistry) -> Vec<String> {
    let mut commands = Vec::new();
    collect_expr_commands(expr, &mut commands);

    let mut defs = Vec::new();
    for cmd_text in &commands {
        let text = cmd_text
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
            .unwrap_or(cmd_text);

        let words = tokenise_to_words(text);
        let Some((cmd_word, arg_words)) = words.split_first() else {
            continue;
        };
        let cmd_name = &cmd_word.text;
        let args: Vec<&str> = arg_words.iter().map(|w| w.text.as_str()).collect();

        // VarWrite positions.
        for idx in registry.arg_indices_for_role(cmd_name, &args, ArgRole::VarWrite) {
            if idx < args.len() {
                let name = normalise_var_name(args[idx]);
                if !name.is_empty() {
                    defs.push(name.to_owned());
                }
            }
        }

        // Body positions — scan for nested defs.
        for idx in registry.arg_indices_for_role(cmd_name, &args, ArgRole::Body) {
            if idx < args.len() {
                let body = args[idx]
                    .strip_prefix('{')
                    .and_then(|s| s.strip_suffix('}'))
                    .unwrap_or(args[idx]);
                defs.extend(defs_from_body_script(body, registry));
            }
        }
    }
    defs
}

/// Extract variable definitions from a body script by tokenising.
///
/// Scans the body text for commands with `ArgRole::VarWrite` arguments
/// (e.g. `set x 1` inside a `catch` body).
fn defs_from_body_script(body_text: &str, registry: &CommandRegistry) -> Vec<String> {
    let mut defs = Vec::new();
    for words in tokenise_command_words(body_text) {
        let Some((cmd_word, arg_words)) = words.split_first() else {
            continue;
        };
        let args: Vec<&str> = arg_words.iter().map(|w| w.text.as_str()).collect();
        for idx in registry.arg_indices_for_role(&cmd_word.text, &args, ArgRole::VarWrite) {
            if let Some(arg) = args.get(idx) {
                let name = normalise_var_name(arg);
                if !name.is_empty() {
                    defs.push(name.to_owned());
                }
            }
        }
    }
    defs
}

/// Out-variable names assigned by command substitutions appearing in
/// `condition` (an `if`/`while` condition expr).
///
/// A command that writes a variable named by one of its own arguments does
/// so *before* either branch runs, so a read of such a variable in the
/// guarded body is **not** read-before-set — the CFG records them as defs on
/// the synthetic `<cond>` statement so the def-use / W210 analysis sees the
/// write.  Which arguments those are is the registry's
/// [`ArgRole::VarWrite`] answer, never a name list here (issue #923 idx 49).
pub(crate) fn condition_command_out_vars(
    condition: &ExprNode,
    registry: &CommandRegistry,
) -> Vec<String> {
    let mut cmds = Vec::new();
    collect_expr_commands(condition, &mut cmds);
    let mut out = Vec::new();
    for cmd_text in &cmds {
        let trimmed = cmd_text.trim();
        let inner = trimmed
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
            .unwrap_or(trimmed);
        cmd_substitution_out_vars(&tokenise_to_words(inner), registry, &mut out, 0);
    }
    out
}

/// A word usable as a variable name: a *literal* identifier (so `{script}`,
/// `[sub]`, quoted words, and — via [`CommandWord::substituted`] — the
/// run-time-computed `$ref` are all rejected).
///
/// The substitution check has to come from the token stream, not the text:
/// the word's *content* spelling drops the `$`, so `set $n 1` would otherwise
/// read as a definition of a variable called `n` and silence a genuine
/// warning about it.
fn is_bare_var_word(word: &CommandWord) -> bool {
    !word.substituted
        && !word.text.is_empty()
        && word
            .text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b':')
}

fn push_out_var(word: &CommandWord, out: &mut Vec<String>) {
    if is_bare_var_word(word) {
        let normalised = normalise_var_name(&word.text);
        if !normalised.is_empty() && !out.iter().any(|v| v == normalised) {
            out.push(normalised.to_owned());
        }
    }
}

/// Out-vars written by the command named by `words[0]`, appended to `out`.
///
/// Registry-first: the write positions come from
/// [`CommandRegistry::arg_indices_for_role`]`(…, `[`ArgRole::VarWrite`]`)`,
/// which is the same machinery the analyser's out-var handling and W210's
/// suppression harvest already consult.  It answers for every command the
/// registry knows — `catch` / `scan` / `gets` / `regexp` as before, plus
/// `set` / `append` / `lappend` / `incr` / `lset` / `regsub` / `lassign` /
/// `binary scan` / `dict update` / `array set` / … — so
/// `if {[set idx [lsearch $l foo]] > -1} {puts $idx}` no longer looks
/// read-before-set (issue #923 idx 49; tclsh 9.0.4 / 8.6.14 both print the
/// index).
///
/// Bodies that run in the *caller's own* frame
/// ([`CommandRegistry::plain_body_arg_indices`] — `catch`'s script, and any
/// other `Plain` body) are descended into as well: their assignments are
/// (maybe) set once the substitution completes.  A `Structural` body
/// (`uplevel`, `namespace eval`, `proc`) writes somewhere else entirely and
/// is never harvested.
///
/// Destroyers ([`Traits::DESTROYS_VARIABLE`] — `unset`) are skipped: an
/// `unset` in a condition removes a variable rather than defining one, and
/// claiming it as a def would mask a genuine W213.
///
/// A **substituted command head** (`[$cmd length foo]`) is skipped for the
/// same reason: which command runs is run-time data, so nothing can be
/// claimed about what it writes.  The word's *content* spelling drops the
/// `$`, so without this check the head of `[$set length foo]` resolves as the
/// builtin `set` and the harvest invents a definition of `length` — enough to
/// swallow a genuine warning.  tclsh 9.0.4 / 8.6.14: `proc f {set} {if
/// {[$set length foo]} {puts $length}}; f string` errors `can't read
/// "length": no such variable`, so the read really is unsafe.
///
/// Suppress-only: over-collection avoids false warnings, under-collection
/// leaves the status quo — but a name harvested from the *wrong* command is
/// not over-collection, it is a wrong fact, so the skip is a correctness
/// requirement rather than a precision nicety.
fn cmd_substitution_out_vars(
    words: &[CommandWord],
    registry: &CommandRegistry,
    out: &mut Vec<String>,
    depth: u32,
) {
    // Native-stack safety net (issue #996): `catch {catch {catch …}}` nests
    // body text inside one word, so the bracket-text cap applies. Past it,
    // stop harvesting — under-collection is the safe direction here.
    if MAX_BRACKET_TEXT_DEPTH.exceeded(depth) {
        return;
    }
    let Some((cmd_word, arg_words)) = words.split_first() else {
        return;
    };
    if cmd_word.substituted {
        return;
    }
    let cmd = &cmd_word.text;
    if registry
        .get(cmd)
        .is_some_and(|s| s.traits.contains(Traits::DESTROYS_VARIABLE))
    {
        return;
    }
    let args: Vec<&str> = arg_words.iter().map(|w| w.text.as_str()).collect();
    for idx in registry.arg_indices_for_role(cmd, &args, ArgRole::VarWrite) {
        if let Some(w) = arg_words.get(idx) {
            push_out_var(w, out);
        }
    }
    for idx in registry.plain_body_arg_indices(cmd, &args) {
        // The word's *content* spelling already has the enclosing braces
        // stripped, so it is the body script text as written.  A body reached
        // through a substitution (`catch $script`) is unknown text and
        // contributes nothing.
        if let Some(body) = arg_words.get(idx).filter(|w| !w.substituted) {
            script_text_out_vars_at(&body.text, registry, out, depth);
        }
    }
}

/// Out-vars assigned by a run of **script text**.
///
/// Shared by [`cmd_substitution_out_vars`]'s `Plain`-body descent (a `catch`
/// script) and the analyser's read-before-set suppression for a
/// [`tcl_registry::Traits::SCRIPT_CONCATENATES_ARGS`] call whose words the
/// lowering left as an opaque barrier (`eval set l2 hello` — issue #1051).
/// Both need the same answer from the same text, so they ask once here.
///
/// Suppress-only: over-collection is safe (it only avoids false warnings),
/// under-collection merely leaves the status quo.
pub(crate) fn script_text_out_vars(body: &str, registry: &CommandRegistry, out: &mut Vec<String>) {
    script_text_out_vars_at(body, registry, out, 0);
}

fn script_text_out_vars_at(
    body: &str,
    registry: &CommandRegistry,
    out: &mut Vec<String>,
    depth: u32,
) {
    for words in tokenise_command_words(body) {
        cmd_substitution_out_vars(&words, registry, out, depth + 1);
    }
}

/// Return `true` if *expr* contains at least one command
/// substitution (`[cmd ...]`). Used by CFG lowering to decide
/// whether a branch condition needs a synthetic `<cond>` placeholder
/// for emission-time startCommand wrapping.
#[must_use]
pub fn expr_has_command(expr: &ExprNode) -> bool {
    // Public entry: the top of an expression tree is nesting depth 0 (issue
    // #996 — the recursion cap lives in [`expr_has_command_at`]).
    expr_has_command_at(expr, 0)
}

fn expr_has_command_at(expr: &ExprNode, depth: u32) -> bool {
    // Native-stack safety net (issue #996): past the cap, assume "yes, has a
    // command substitution" — the conservative direction, since callers use
    // this to decide whether a branch condition needs a synthetic `<cond>`
    // placeholder / startCommand wrapping; a false `true` only adds a
    // harmless wrapper, never drops a needed one.
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return true;
    }
    match expr {
        ExprNode::Command { .. } => true,
        ExprNode::Binary { left, right, .. } => {
            expr_has_command_at(left, depth + 1) || expr_has_command_at(right, depth + 1)
        }
        ExprNode::Unary { operand, .. } => expr_has_command_at(operand, depth + 1),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            expr_has_command_at(condition, depth + 1)
                || expr_has_command_at(true_branch, depth + 1)
                || expr_has_command_at(false_branch, depth + 1)
        }
        ExprNode::Call { args, .. } => args.iter().any(|a| expr_has_command_at(a, depth + 1)),
        ExprNode::Literal { .. }
        | ExprNode::String { .. }
        | ExprNode::Var { .. }
        | ExprNode::Raw { .. } => false,
    }
}

/// Recursively collect `ExprNode::Command` text from an expression tree.
///
/// Respects short-circuit evaluation: for `&&`/`||`, the RHS is
/// always included (conservative — we do not attempt compile-time
/// constant evaluation).
pub(crate) fn collect_expr_commands(expr: &ExprNode, out: &mut Vec<String>) {
    // Entry point: the top of an expression tree is nesting depth 0 (issue
    // #996 — the recursion cap lives in [`collect_expr_commands_at`]).
    collect_expr_commands_at(expr, out, 0);
}

fn collect_expr_commands_at(expr: &ExprNode, out: &mut Vec<String>, depth: u32) {
    // Native-stack safety net (issue #996): walks the `ExprNode` tree, one
    // native frame per level. Past the cap, stop descending — a collector
    // that returns the command texts gathered so far is the safe fallback
    // (substitutions buried deeper than the cap are not collected; never a
    // crash).
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return;
    }
    match expr {
        ExprNode::Command { text, .. } => {
            out.push(text.clone());
        }
        ExprNode::Binary { left, right, .. } => {
            collect_expr_commands_at(left, out, depth + 1);
            collect_expr_commands_at(right, out, depth + 1);
        }
        ExprNode::Unary { operand, .. } => {
            collect_expr_commands_at(operand, out, depth + 1);
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            collect_expr_commands_at(condition, out, depth + 1);
            collect_expr_commands_at(true_branch, out, depth + 1);
            collect_expr_commands_at(false_branch, out, depth + 1);
        }
        ExprNode::Call { args, .. } => {
            for arg in args {
                collect_expr_commands_at(arg, out, depth + 1);
            }
        }
        _ => {}
    }
}

/// Tokenise source text into a flat list of words (single command).
fn tokenise_to_words(source: &str) -> Vec<CommandWord> {
    tokenise_command_words(source)
        .into_iter()
        .next()
        .unwrap_or_default()
}

/// One word of a tokenised command, in both spellings its consumers need.
///
/// The lexer's `token_text` strips a token's `$` / `${` / `[` / `{` / `"`
/// prefix, which is what a name-harvesting caller wants (`{set x 1}` → the
/// script text) but which erases the distinction that decides whether a word
/// *is* a name: `set $x 1` and `set x 1` both reduce to `["set", "x", "1"]`.
/// Carrying both spellings plus the two quoting facts lets every consumer ask
/// its own question of one tokenisation.
#[derive(Debug, Clone)]
pub(crate) struct CommandWord {
    /// Concatenated token **content** — `$x` → `x`, `{s}` → `s`.
    pub text: String,
    /// Concatenated **verbatim** source spelling — `$x` stays `$x`.  A
    /// delimited token's closer is one past its span (the inner-end
    /// convention), so a braced word reads back as `{s` — enough to answer
    /// "does the name position substitute?", not enough to re-parse.
    pub raw: String,
    /// Some constituent token is a `$` / `[` substitution, so the word's
    /// value is not its source text.
    pub substituted: bool,
    /// The word is a single brace-quoted token (`{…}`): Tcl leaves its
    /// content literal, so a `$` inside is source text, not a substitution.
    pub braced_literal: bool,
}

/// Tokenise source text into a list of commands, each a list of words.
pub(crate) fn tokenise_command_words(source: &str) -> Vec<Vec<CommandWord>> {
    let sm = SourceMap::new(source);
    let Ok(tokens) = Lexer::new(source).tokenise_all() else {
        return Vec::new();
    };

    let mut commands = Vec::new();
    let mut words: Vec<CommandWord> = Vec::new();
    let mut prev_is_sep = true;

    for tok in &tokens {
        match tok.kind {
            TokenType::Eol | TokenType::Eof => {
                if !words.is_empty() {
                    commands.push(std::mem::take(&mut words));
                }
                prev_is_sep = true;
            }
            TokenType::Sep | TokenType::Comment => {
                prev_is_sep = true;
            }
            _ => {
                let substituted = matches!(tok.kind, TokenType::Var | TokenType::Cmd);
                match words.last_mut() {
                    Some(last) if !prev_is_sep => {
                        last.text.push_str(sm.token_text(*tok));
                        last.raw.push_str(sm.text(tok.span));
                        last.substituted |= substituted;
                        // A compound word (`{a}$b`) is no longer literal.
                        last.braced_literal = false;
                    }
                    _ => words.push(CommandWord {
                        text: sm.token_text(*tok).to_owned(),
                        raw: sm.text(tok.span).to_owned(),
                        substituted,
                        braced_literal: tok.kind == TokenType::Str,
                    }),
                }
                prev_is_sep = false;
            }
        }
    }
    if !words.is_empty() {
        commands.push(words);
    }
    commands
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{ForeachIterator, IfClause, Script};
    use tcl_lexer::Span;

    #[test]
    fn defs_from_empty_script() {
        let script = Script::new();
        assert!(defs_from_ir_script(&script).is_empty());
    }

    /// Regression coverage for issue #996: `expr_has_command` and
    /// `collect_expr_commands` each recurse once per `ExprNode` level with no
    /// depth cap before this fix. A tree built directly is unbounded (the
    /// Pratt parser caps its own output at 256) and empirically overflowed
    /// the native stack (SIGABRT) in the low thousands of levels on a 2 MiB
    /// thread. 3000 is past that crash range and past `MAX_EXPR_NODE_DEPTH`
    /// (256); the assertion is that each returns at all.
    #[test]
    fn deeply_nested_expr_command_walks_survive() {
        use crate::expr_ast::UnaryOp;
        let mut node = ExprNode::Command {
            text: "[x]".into(),
            start: 0,
            end: 3,
        };
        for _ in 0..3000 {
            node = ExprNode::Unary {
                op: UnaryOp::Not,
                operand: Box::new(node),
            };
        }
        let _ = expr_has_command(&node);
        let mut cmds = Vec::new();
        collect_expr_commands(&node, &mut cmds);
    }

    #[test]
    fn defs_from_assign() {
        let script = Script::from_statements(vec![Statement::AssignConst {
            span: Span::new(0, 7),
            name: "x".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        }]);
        assert_eq!(defs_from_ir_script(&script), vec!["x"]);
    }

    #[test]
    fn defs_from_call_defs() {
        let script = Script::from_statements(vec![Statement::Call {
            span: Span::new(0, 10),
            command: "set".into(),
            canonical_command: None,
            args: vec!["x".into(), "1".into()],
            defs: vec!["x".into()],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        }]);
        assert_eq!(defs_from_ir_script(&script), vec!["x"]);
    }

    #[test]
    fn defs_from_nested_if() {
        let script = Script::from_statements(vec![Statement::If {
            span: Span::new(0, 30),
            clauses: vec![IfClause {
                condition: ExprNode::Literal {
                    text: "1".into(),
                    start: 0,
                    end: 1,
                },
                condition_span: Span::new(3, 4),
                body: Script::from_statements(vec![Statement::AssignConst {
                    span: Span::new(6, 13),
                    name: "y".into(),
                    name_braced: false,
                    value: "2".into(),
                    value_span: None,
                }]),
                body_span: Span::new(5, 14),
                condition_base: None,
            }],
            else_body: None,
            else_span: None,
        }]);
        assert_eq!(defs_from_ir_script(&script), vec!["y"]);
    }

    /// Regression coverage for issue #996: `collect_defs_from_script`
    /// recurses once per nested `If`/`For`/`While`/`Foreach`/`Catch`/`Try`/
    /// `Switch` body, with no depth cap of its own before this fix.
    /// Transitively bounded to `MAX_LOWER_NEST_DEPTH` (256) by the lowering
    /// pass today, so this is defence-in-depth / consistency with every
    /// other full-tree walker in this crate, not a currently-reproducible
    /// crash. 2000 levels is comfortably past this new cap; the assertion
    /// is that `defs_from_ir_script` returns at all, not what it returns.
    #[test]
    fn deeply_nested_if_survives_defs_from_ir_script() {
        const DEPTH: usize = 2000;
        let leaf = Statement::AssignConst {
            span: Span::new(0, 0),
            name: "leaf".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        };
        let mut script = Script::from_statements(vec![leaf]);
        for _ in 0..DEPTH {
            script = Script::from_statements(vec![Statement::If {
                span: Span::new(0, 0),
                clauses: vec![IfClause {
                    condition: ExprNode::Literal {
                        text: "1".into(),
                        start: 0,
                        end: 1,
                    },
                    condition_span: Span::new(0, 0),
                    body: script,
                    body_span: Span::new(0, 0),
                    condition_base: None,
                }],
                else_body: None,
                else_span: None,
            }]);
        }

        // 2000 levels of nesting exceeds the new 256-deep cap, so the
        // innermost `leaf` def is never reached — the assertion is that
        // this returns at all (no stack overflow), not what it returns.
        let _ = defs_from_ir_script(&script);
    }

    #[test]
    fn defs_from_foreach() {
        let script = Script::from_statements(vec![Statement::Foreach {
            span: Span::new(0, 30),
            iterators: vec![ForeachIterator {
                vars: vec!["k".into(), "v".into()],
                list_arg: "$d".into(),
            }],
            body: Script::new(),
            body_span: Span::new(20, 25),
            is_lmap: false,
            raw_args: vec![],
            is_dict_iteration: false,
            is_array_iteration: false,
            raw_tokens: None,
        }]);
        assert_eq!(defs_from_ir_script(&script), vec!["k", "v"]);
    }

    #[test]
    fn defs_from_catch() {
        let script = Script::from_statements(vec![Statement::Catch {
            span: Span::new(0, 30),
            body: Script::from_statements(vec![Statement::AssignConst {
                span: Span::new(7, 14),
                name: "inner".into(),
                name_braced: false,
                value: "1".into(),
                value_span: None,
            }]),
            body_span: Span::new(6, 15),
            result_var: Some("result".into()),
            options_var: None,
            raw_args: vec![],
            tokens: None,
        }]);
        let d = defs_from_ir_script(&script);
        assert!(d.contains(&"inner".to_string()));
        assert!(d.contains(&"result".to_string()));
    }

    #[test]
    fn tokenise_simple_command() {
        let words = tokenise_to_words("set x 1");
        let texts: Vec<&str> = words.iter().map(|w| w.text.as_str()).collect();
        assert_eq!(texts, vec!["set", "x", "1"]);
        assert!(words.iter().all(|w| !w.substituted));
    }

    #[test]
    fn tokenise_keeps_both_spellings_and_the_substitution_flag() {
        let words = tokenise_to_words("set $x {a $b}");
        let texts: Vec<&str> = words.iter().map(|w| w.text.as_str()).collect();
        let raws: Vec<&str> = words.iter().map(|w| w.raw.as_str()).collect();
        // Content drops the `$`; raw keeps it. A delimited token's closer is
        // one past its span, so the braced word's raw spelling stops at `b`.
        assert_eq!(texts, vec!["set", "x", "a $b"]);
        assert_eq!(raws, vec!["set", "$x", "{a $b"]);
        assert_eq!(
            words.iter().map(|w| w.substituted).collect::<Vec<_>>(),
            vec![false, true, false]
        );
        assert!(words[2].braced_literal, "`{{a $b}}` is a literal word");
    }

    #[test]
    fn collect_expr_command_nodes() {
        let expr = ExprNode::Command {
            text: "[set x 1]".into(),
            start: 0,
            end: 9,
        };
        let mut cmds = Vec::new();
        collect_expr_commands(&expr, &mut cmds);
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0], "[set x 1]");
    }

    #[test]
    fn defs_from_expr_set_command() {
        let reg = CommandRegistry::build_default();
        let expr = ExprNode::Command {
            text: "[set x 1]".into(),
            start: 0,
            end: 9,
        };
        let d = defs_from_expr(&expr, &reg);
        assert!(
            d.contains(&"x".to_string()),
            "should find set's VarWrite; got {d:?}"
        );
    }

    /// Out-vars a condition-embedded `[cmd …]` contributes, as
    /// [`condition_command_out_vars`] sees them.
    fn cond_out_vars(cmd_text: &str) -> Vec<String> {
        let registry = tcl_registry::cache::registry_for_dialect("tcl8.6");
        let expr = ExprNode::Command {
            text: format!("[{cmd_text}]"),
            start: 0,
            end: 0,
        };
        condition_command_out_vars(&expr, registry)
    }

    /// Contract for issue #923 audit idx 49: the registry's
    /// `ArgRole::VarWrite` query must answer for every command the deleted
    /// `catch` / `scan` / `gets` / `regexp` name list covered.
    #[test]
    fn registry_query_covers_the_deleted_hardcoded_four() {
        for (cmd_text, expected) in [
            ("catch {error x} msg opts", vec!["msg", "opts"]),
            ("scan $t {%d %d} a b", vec!["a", "b"]),
            ("gets $fp line", vec!["line"]),
            ("regexp {(a)(b)} $s m p q", vec!["m", "p", "q"]),
            // The switch-skipping the hand-rolled `regexp` loop did by hand
            // is the registry resolver's job now.
            ("regexp -nocase -- $re $s m", vec!["m"]),
        ] {
            let got = cond_out_vars(cmd_text);
            for name in &expected {
                assert!(
                    got.iter().any(|v| v == name),
                    "`{cmd_text}` must still yield `{name}`; got {got:?}"
                );
            }
        }
    }

    /// …and it exceeds that list: commands the hardcoded four never covered,
    /// each verified against tclsh 9.0.4 / 8.6.14 first.
    #[test]
    fn registry_query_exceeds_the_deleted_hardcoded_four() {
        for (cmd_text, expected) in [
            // `proc bar {lst} {if {[set idx [lsearch $lst foo]] > -1} {puts $idx}}`
            // `bar {a foo b}` → 1
            ("set idx [lsearch $lst foo]", vec!["idx"]),
            // `proc loopy {} {set i 0; while {[incr i] < 3} {puts $i}}` → 1 2
            ("incr i", vec!["i"]),
            // `proc lass {lst} {lassign $lst a b; puts "$a-$b"}`
            // `lass {1 2}` → 1-2
            ("lassign $lst a b", vec!["a", "b"]),
            // `proc binscan {} {binary scan "AB" H2H2 hi lo; puts "$hi $lo"}`
            // → 41 42
            ("binary scan $d H2H2 hi lo", vec!["hi", "lo"]),
            // `regsub` writes its result variable.
            ("regsub $re $s $r out", vec!["out"]),
        ] {
            let got = cond_out_vars(cmd_text);
            for name in &expected {
                assert!(
                    got.iter().any(|v| v == name),
                    "`{cmd_text}` must yield `{name}`; got {got:?}"
                );
            }
        }
    }

    /// A `Plain` body (`catch`'s script) runs in the caller's own frame, so
    /// its writes are harvested; a destroyer is not a definition.
    #[test]
    fn condition_out_vars_descends_plain_bodies_but_skips_destroyers() {
        let got = cond_out_vars("catch {set inner 1} msg");
        assert!(got.iter().any(|v| v == "inner"), "got {got:?}");
        assert!(got.iter().any(|v| v == "msg"), "got {got:?}");

        // tclsh 9.0.4 / 8.6.14:
        // `proc t {} {if {[catch {unset gone}]} {puts $gone}}`
        // `catch {t} err` → 1, err → `can't read "gone": no such variable`
        let got = cond_out_vars("catch {unset gone}");
        assert!(
            !got.iter().any(|v| v == "gone"),
            "`unset` destroys rather than defines; got {got:?}"
        );
    }

    /// A dynamic target contributes no *name* — `push_out_var` only accepts a
    /// bare identifier, so `set $n 1` in a condition stays silent rather than
    /// inventing a def called `n`.
    #[test]
    fn condition_out_vars_ignores_a_dynamic_target() {
        let got = cond_out_vars("set $n 1");
        assert!(got.is_empty(), "got {got:?}");
    }

    /// PR #1076 review, P2: a command word that is itself a substitution names
    /// an unknown command, so nothing may be harvested from it — not even when
    /// its *content* spelling collides with a builtin.
    ///
    /// tclsh 9.0.4 / 8.6.14: `proc f {set} {if {[$set length foo]} {puts
    /// $length}}; f string` runs `string length foo` and then errors
    /// `can't read "length": no such variable`.
    #[test]
    fn condition_out_vars_ignores_a_substituted_command_head() {
        for cmd_text in ["$set length foo", "$catch {error x} msg", "[pick] $fp line"] {
            let got = cond_out_vars(cmd_text);
            assert!(
                got.is_empty(),
                "`{cmd_text}` has a computed head; nothing is knowable about \
what it writes, got {got:?}"
            );
        }
    }

    /// TP control for the pair above: the same words under a *literal* head
    /// keep harvesting, so the guard keys on substitution rather than on the
    /// spelling.
    #[test]
    fn condition_out_vars_still_harvests_a_literal_head() {
        assert!(
            cond_out_vars("set length foo")
                .iter()
                .any(|v| v == "length")
        );
        assert!(
            cond_out_vars("catch {error x} msg")
                .iter()
                .any(|v| v == "msg")
        );
        assert!(cond_out_vars("gets $fp line").iter().any(|v| v == "line"));
    }
}
