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

/// Collect all variable names defined anywhere inside a script (recursive).
///
/// Walks through structured IR nodes (`If`, `For`, `While`, `Foreach`,
/// `Catch`, `Try`, `Switch`) to find every assignment, call-def, and
/// iteration variable. This is the Rust port of Python's
/// `_defs_from_ir_script` in `cfg.py`.
#[must_use]
pub fn defs_from_ir_script(script: &Script) -> Vec<String> {
    let mut defs = Vec::new();
    collect_defs_from_script(script, &mut defs);
    defs
}

fn collect_defs_from_script(script: &Script, defs: &mut Vec<String>) {
    for stmt in &script.statements {
        match stmt {
            Statement::AssignConst { name, .. }
            | Statement::AssignExpr { name, .. }
            | Statement::AssignValue { name, .. }
            | Statement::Incr { name, .. } => {
                let n = normalise_var_name(name);
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
                        let n = normalise_var_name(vn);
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

/// Return `true` if *expr* contains at least one command
/// substitution (`[cmd ...]`). Used by CFG lowering to decide
/// whether a branch condition needs a synthetic `<cond>` placeholder
/// for emission-time startCommand wrapping (C18 case 5).
#[must_use]
pub fn expr_has_command(expr: &ExprNode) -> bool {
    match expr {
        ExprNode::Command { .. } => true,
        ExprNode::Binary { left, right, .. } => {
            expr_has_command(left) || expr_has_command(right)
        }
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
/// constant evaluation in this port).
fn collect_expr_commands(expr: &ExprNode, out: &mut Vec<String>) {
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
            value: "1".into(),
        }]);
        assert_eq!(defs_from_ir_script(&script), vec!["x"]);
    }

    #[test]
    fn defs_from_call_defs() {
        let script = Script::from_statements(vec![Statement::Call {
            span: Span::new(0, 10),
            command: "set".into(),
            args: vec!["x".into(), "1".into()],
            defs: vec!["x".into()],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
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
                    value: "2".into(),
                }]),
                body_span: Span::new(5, 14),
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
                value: "1".into(),
            }]),
            body_span: Span::new(6, 15),
            result_var: Some("result".into()),
            options_var: None,
            raw_args: vec![],
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
