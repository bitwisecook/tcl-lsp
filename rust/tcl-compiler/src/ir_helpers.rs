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
use tcl_registry::{ArgRole, CommandRegistry};

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

/// Collect all variable names defined anywhere inside a script (recursive).
///
/// Walks through structured IR nodes (`If`, `For`, `While`, `Foreach`,
/// `Catch`, `Try`, `Switch`) to find every assignment, call-def, and
/// iteration variable.
#[must_use]
pub fn defs_from_ir_script(script: &Script) -> Vec<String> {
    let mut defs = Vec::new();
    collect_defs_from_script(script, &mut defs);
    defs
}

fn collect_defs_from_script(script: &Script, defs: &mut Vec<String>) {
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
                    collect_defs_from_script(&clause.body, defs);
                }
                if let Some(eb) = else_body {
                    collect_defs_from_script(eb, defs);
                }
            }

            Statement::For { body, .. } | Statement::While { body, .. } => {
                collect_defs_from_script(body, defs);
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
                collect_defs_from_script(body, defs);
            }

            Statement::Catch {
                body,
                result_var,
                options_var,
                ..
            } => {
                collect_defs_from_script(body, defs);
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
                collect_defs_from_script(body, defs);
                for handler in handlers {
                    if let Some(vn) = &handler.var_name {
                        defs.push(vn.clone());
                    }
                    if let Some(ov) = &handler.options_var {
                        defs.push(ov.clone());
                    }
                    collect_defs_from_script(&handler.body, defs);
                }
                if let Some(fb) = finally_body {
                    collect_defs_from_script(fb, defs);
                }
            }

            Statement::Switch {
                arms, default_body, ..
            } => {
                for arm in arms {
                    if let Some(body) = &arm.body {
                        collect_defs_from_script(body, defs);
                    }
                }
                if let Some(db) = default_body {
                    collect_defs_from_script(db, defs);
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
        if words.is_empty() {
            continue;
        }
        let cmd_name = &words[0];
        let args: Vec<&str> = words[1..].iter().map(String::as_str).collect();

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
    let words_list = tokenise_to_command_words(body_text);

    for words in &words_list {
        if words.is_empty() {
            continue;
        }
        let cmd_name = &words[0];
        let args: Vec<&str> = words[1..].iter().map(String::as_str).collect();
        for idx in registry.arg_indices_for_role(cmd_name, &args, ArgRole::VarWrite) {
            if idx < args.len() {
                let name = normalise_var_name(args[idx]);
                if !name.is_empty() {
                    defs.push(name.to_owned());
                }
            }
        }
    }
    defs
}

/// Out-variable names assigned by `catch` / `regexp` / `scan` command
/// substitutions appearing in `condition` (an `if`/`while` condition expr).
/// These builtins write result variables as a side effect, so a read of
/// such a variable in the guarded body is **not** read-before-set — the
/// CFG records them as defs on the synthetic `<cond>` statement so the
/// def-use / W210 analysis sees the write.
pub(crate) fn condition_command_out_vars(condition: &ExprNode) -> Vec<String> {
    let mut cmds = Vec::new();
    collect_expr_commands(condition, &mut cmds);
    let mut out = Vec::new();
    for cmd_text in &cmds {
        let trimmed = cmd_text.trim();
        let inner = trimmed
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
            .unwrap_or(trimmed);
        cmd_substitution_out_vars(&tokenise_to_words(inner), &mut out);
    }
    out
}

/// A word usable as a variable name: identifier characters only (so
/// `{script}`, `$ref`, `[sub]` and quoted words are rejected).
fn is_bare_var_word(word: &str) -> bool {
    !word.is_empty()
        && word
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b':')
}

fn push_out_var(word: &str, out: &mut Vec<String>) {
    if is_bare_var_word(word) {
        let normalised = normalise_var_name(word);
        if !normalised.is_empty() && !out.iter().any(|v| v == normalised) {
            out.push(normalised.to_owned());
        }
    }
}

/// Out-vars written by the builtin named by `words[0]`, appended to `out`.
fn cmd_substitution_out_vars(words: &[String], out: &mut Vec<String>) {
    match words.first().map(String::as_str) {
        // `catch SCRIPT ?resultVar? ?optionsVar?`
        Some("catch") => {
            if let Some(w) = words.get(2) {
                push_out_var(w, out);
            }
            if let Some(w) = words.get(3) {
                push_out_var(w, out);
            }
            // The SCRIPT body (words[1]) runs in the current scope, so variables
            // it assigns are (maybe) set once the catch completes — e.g.
            // `if {![catch {set x 1}]} { puts $x }` (tclsh prints 1, so the read
            // is safe). Recover the body's writes too.
            if let Some(body) = words.get(1) {
                catch_body_out_vars(body, out);
            }
        }
        // `scan STRING FORMAT ?varName ...?`
        Some("scan") => {
            for w in words.iter().skip(3) {
                push_out_var(w, out);
            }
        }
        // `gets channelId ?varName?` writes the line into `varName` in the
        // current scope (e.g. `while {[gets $fp line] >= 0} {…}`).
        Some("gets") => {
            if let Some(w) = words.get(2) {
                push_out_var(w, out);
            }
        }
        // `regexp ?switches? EXP STRING ?matchVar subVar ...?`
        Some("regexp") => {
            let mut i = 1;
            while i < words.len() && words[i].starts_with('-') {
                if words[i] == "--" {
                    i += 1;
                    break;
                }
                // `-start` consumes a value; every other regexp switch is a
                // valueless flag.
                if words[i] == "-start" {
                    i += 1;
                }
                i += 1;
            }
            // Skip EXP and STRING; the remaining words are out-vars.
            for w in words.iter().skip(i + 2) {
                push_out_var(w, out);
            }
        }
        _ => {}
    }
}

/// Out-vars assigned by the *body* of a `catch {SCRIPT}` — direct
/// `set`/`append`/`lappend`/`incr` targets plus nested command-substitution
/// writers (`gets`/`scan`/`regexp`/`catch`). Suppress-only: keeps a read of a
/// catch-body-assigned variable *after* the catch from looking
/// read-before-set. Over-collection here is safe (it only avoids false
/// warnings); a body whose error precedes the assignment is the conservative
/// stance read-before-set already takes for command substitutions.
fn catch_body_out_vars(body_word: &str, out: &mut Vec<String>) {
    // The body is usually a single braced word; strip the braces to recover the
    // script text. A non-braced body (e.g. a bare command) is scanned as-is.
    let body = body_word
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .unwrap_or(body_word);
    // First-arg-writer membership is the registry's
    // `writes_first_arg_variable` query (cached default registry — the set
    // is core Tcl in every dialect); over-collection stays safe per the
    // suppress-only contract above.
    let registry = tcl_registry::cache::registry_for_dialect("tcl8.6");
    for words in tokenise_to_command_words(body) {
        match words.first() {
            // Direct assignment commands write their first argument.
            Some(cmd) if registry.writes_first_arg_variable(cmd) => {
                if let Some(w) = words.get(1) {
                    push_out_var(w, out);
                }
            }
            // Nested command-sub writers (`gets`/`scan`/`regexp`/`catch`).
            _ => cmd_substitution_out_vars(&words, out),
        }
    }
}

/// Return `true` if *expr* contains at least one command
/// substitution (`[cmd ...]`). Used by CFG lowering to decide
/// whether a branch condition needs a synthetic `<cond>` placeholder
/// for emission-time startCommand wrapping.
#[must_use]
pub fn expr_has_command(expr: &ExprNode) -> bool {
    match expr {
        ExprNode::Command { .. } => true,
        ExprNode::Binary { left, right, .. } => expr_has_command(left) || expr_has_command(right),
        ExprNode::Unary { operand, .. } => expr_has_command(operand),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            expr_has_command(condition)
                || expr_has_command(true_branch)
                || expr_has_command(false_branch)
        }
        ExprNode::Call { args, .. } => args.iter().any(expr_has_command),
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
    match expr {
        ExprNode::Command { text, .. } => {
            out.push(text.clone());
        }
        ExprNode::Binary { left, right, .. } => {
            collect_expr_commands(left, out);
            collect_expr_commands(right, out);
        }
        ExprNode::Unary { operand, .. } => {
            collect_expr_commands(operand, out);
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            collect_expr_commands(condition, out);
            collect_expr_commands(true_branch, out);
            collect_expr_commands(false_branch, out);
        }
        ExprNode::Call { args, .. } => {
            for arg in args {
                collect_expr_commands(arg, out);
            }
        }
        _ => {}
    }
}

/// Tokenise source text into a flat list of words (single command).
fn tokenise_to_words(source: &str) -> Vec<String> {
    let sm = SourceMap::new(source);
    let lexer = Lexer::new(source);
    let Ok(tokens) = lexer.tokenise_all() else {
        return Vec::new();
    };

    let mut words = Vec::new();
    let mut prev_is_sep = true;
    for tok in &tokens {
        match tok.kind {
            TokenType::Sep | TokenType::Eol | TokenType::Eof | TokenType::Comment => {
                prev_is_sep = true;
            }
            _ => {
                let text = sm.token_text(*tok);
                if prev_is_sep {
                    words.push(text.to_owned());
                } else if let Some(last) = words.last_mut() {
                    last.push_str(text);
                } else {
                    words.push(text.to_owned());
                }
                prev_is_sep = false;
            }
        }
    }
    words
}

/// Tokenise source text into a list of commands, each a list of words.
fn tokenise_to_command_words(source: &str) -> Vec<Vec<String>> {
    let sm = SourceMap::new(source);
    let lexer = Lexer::new(source);
    let Ok(tokens) = lexer.tokenise_all() else {
        return Vec::new();
    };

    let mut commands = Vec::new();
    let mut words: Vec<String> = Vec::new();
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
                let text = sm.token_text(*tok);
                if prev_is_sep {
                    words.push(text.to_owned());
                } else if let Some(last) = words.last_mut() {
                    last.push_str(text);
                } else {
                    words.push(text.to_owned());
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

    #[test]
    fn defs_from_assign() {
        let script = Script::from_statements(vec![Statement::AssignConst {
            span: Span::new(0, 7),
            name: "x".into(),
            name_braced: false,
            value: "1".into(),
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
                }]),
                body_span: Span::new(5, 14),
                condition_base: None,
            }],
            else_body: None,
            else_span: None,
        }]);
        assert_eq!(defs_from_ir_script(&script), vec!["y"]);
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
        assert_eq!(words, vec!["set", "x", "1"]);
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
}
