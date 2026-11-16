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

//! `return` inside a spliced body: finding it, the one-shot loop that routes it and the stored result.

use tcl_compiler::expr_ast::ExprNode;
use tcl_compiler::ir::{CommandBindingSite, IfClause, Script, Statement, SwitchArm};
use tcl_lexer::Span;

// nested-return scanning

pub(super) fn has_nested_irreturn(script: &Script) -> bool {
    script
        .statements
        .iter()
        .any(|s| !matches!(s, Statement::Return { .. }) && scan_for_irreturn(s))
}

fn scan_for_irreturn(stmt: &Statement) -> bool {
    match stmt {
        Statement::Return { .. } => true,
        Statement::If {
            clauses, else_body, ..
        } => {
            clauses.iter().any(|c| has_any_irreturn(&c.body))
                || else_body.as_ref().is_some_and(has_any_irreturn)
        }
        Statement::For {
            init, next, body, ..
        } => has_any_irreturn(init) || has_any_irreturn(next) || has_any_irreturn(body),
        Statement::Block { body, .. }
        | Statement::UpFrame { body, .. }
        | Statement::While { body, .. }
        | Statement::Foreach { body, .. }
        | Statement::Catch { body, .. } => has_any_irreturn(body),
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            has_any_irreturn(body)
                || handlers.iter().any(|h| has_any_irreturn(&h.body))
                || finally_body.as_ref().is_some_and(has_any_irreturn)
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            arms.iter()
                .any(|a| a.body.as_ref().is_some_and(has_any_irreturn))
                || default_body.as_ref().is_some_and(has_any_irreturn)
        }
        _ => false,
    }
}

fn has_any_irreturn(script: &Script) -> bool {
    script
        .statements
        .iter()
        .any(|s| matches!(s, Statement::Return { .. }) || scan_for_irreturn(s))
}

/// A value stored for the wrapped body's result, as the statement it came from
/// said it: a literal, which nothing substitutes, or a word the emitter
/// substitutes. A braced word is the first, and storing it as the second runs
/// its backslash sequences and its `[…]`.
pub(super) enum Stored {
    Literal(String),
    Substituted(String),
}

impl Stored {
    /// The statement that stores this value in `name`.
    fn into_statement(self, span: Span, name: &str) -> Statement {
        match self {
            Self::Literal(value) => Statement::AssignConst {
                span,
                name: name.to_owned(),
                name_braced: false,
                value,
                value_span: None,
            },
            Self::Substituted(value) => Statement::AssignValue {
                span,
                name: name.to_owned(),
                name_braced: false,
                value_needs_backsubst: value.contains('\\'),
                value,
                tokens: None,
            },
        }
    }
}

/// Return the value the body's last statement contributes as Tcl's implicit
/// return value, as that statement stored it, or `None` when the shape is too
/// complex to capture safely.
pub(super) fn capture_implicit_return_value(stmt: &Statement) -> Option<Stored> {
    match stmt {
        Statement::AssignConst { value, .. } => Some(Stored::Literal(value.clone())),
        Statement::AssignValue { value, .. } => Some(Stored::Substituted(value.clone())),
        _ => None,
    }
}

// return-as-break wrap

/// Wrap `renamed_body` in a one-shot `while {1} { … }` loop that routes
/// `return`-as-break. Uses `while`
/// rather than `for` so empty init/next clauses don't emit placeholder
/// calls the WASM codegen would trap on.
pub(super) fn wrap_with_irreturn_loop(
    renamed_body: &Script,
    result_var: &str,
    span: Span,
    implicit_value: Option<Stored>,
) -> Vec<Statement> {
    let rewritten = substitute_irreturn(renamed_body, result_var);
    let mut body_with_break: Vec<Statement> = rewritten.statements;
    let command_binding_sites = rewritten.command_binding_sites.iter().cloned().collect();
    let procedure_binding_requirements = rewritten
        .procedure_binding_requirements
        .iter()
        .cloned()
        .collect();
    if let Some(stored) = implicit_value {
        body_with_break.push(stored.into_statement(span, result_var));
    }
    body_with_break.push(break_call(span));

    vec![
        Statement::AssignConst {
            span,
            name: result_var.to_owned(),
            name_braced: false,
            value: String::new(),
            value_span: None,
        },
        Statement::While {
            span,
            condition: ExprNode::Literal {
                text: "1".to_owned(),
                start: 0,
                end: 1,
            },
            condition_span: span,
            condition_base: None,
            body: Script::from_transformed_parts(
                body_with_break,
                command_binding_sites,
                procedure_binding_requirements,
            ),
            body_span: span,
            raw_args: Vec::new(),
            raw_tokens: None,
        },
    ]
}

/// A synthetic `break` call.
fn break_call(span: Span) -> Statement {
    Statement::Call {
        span,
        command: "break".to_owned(),
        canonical_command: Some("::break".to_owned()),
        args: Vec::new(),
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    }
}

/// Replace each [`Statement::Return`] reachable from `script` with
/// `set <result_var> <value>; break`. Recurses into transparent nested
/// bodies (`Block`, `if`, `switch`); does NOT recurse into loop / catch /
/// try / upframe bodies (the eligibility gate guarantees no return is
/// nested there).
fn substitute_irreturn(script: &Script, result_var: &str) -> Script {
    let mut out = Vec::with_capacity(script.statements.len());
    let mut command_binding_sites: Vec<CommandBindingSite> =
        script.command_binding_sites.iter().cloned().collect();
    let procedure_binding_requirements = script
        .procedure_binding_requirements
        .iter()
        .cloned()
        .collect();
    for stmt in &script.statements {
        if let Statement::Return {
            span,
            value,
            expr,
            braced,
            command_binding,
            ..
        } = stmt
        {
            if let Some(binding) = command_binding {
                command_binding_sites.push(CommandBindingSite {
                    span: *span,
                    binding: binding.clone(),
                    known_namespaces: None,
                    variable_frame: None,
                    variable_context: None,
                    existing_namespace_cells: None,
                    source_tokens: None,
                    shape: tcl_registry::release_compilation::InvocationShape::counted(
                        usize::from(value.is_some() || expr.is_some()),
                    ),
                });
            }
            let v = value.clone().unwrap_or_default();
            let stored = if *braced {
                Stored::Literal(v)
            } else {
                Stored::Substituted(v)
            };
            out.push(stored.into_statement(*span, result_var));
            out.push(break_call(*span));
        } else {
            out.push(substitute_irreturn_stmt(stmt, result_var));
        }
    }
    Script::from_transformed_parts(out, command_binding_sites, procedure_binding_requirements)
}

fn substitute_irreturn_stmt(stmt: &Statement, result_var: &str) -> Statement {
    match stmt {
        Statement::Block {
            span,
            body,
            namespace,
            tokens,
            error_context,
        } => Statement::Block {
            span: *span,
            body: substitute_irreturn(body, result_var),
            namespace: namespace.clone(),
            tokens: tokens.clone(),
            error_context: *error_context,
        },
        Statement::If {
            span,
            clauses,
            else_body,
            else_span,
        } => Statement::If {
            span: *span,
            clauses: clauses
                .iter()
                .map(|c| IfClause {
                    condition: c.condition.clone(),
                    condition_span: c.condition_span,
                    condition_base: c.condition_base,
                    body: substitute_irreturn(&c.body, result_var),
                    body_span: c.body_span,
                })
                .collect(),
            else_body: else_body
                .as_ref()
                .map(|b| substitute_irreturn(b, result_var)),
            else_span: *else_span,
        },
        Statement::Switch {
            span,
            subject,
            subject_braced,
            subject_span,
            arms,
            default_body,
            default_span,
            mode,
            nocase,
            raw_args,
            raw_arg_braced,
            raw_arg_quoted,
            command,
            patterns_braced,
        } => Statement::Switch {
            subject_braced: *subject_braced,
            span: *span,
            subject: subject.clone(),
            subject_span: *subject_span,
            arms: arms
                .iter()
                .map(|a| SwitchArm {
                    pattern: a.pattern.clone(),
                    pattern_braced: a.pattern_braced,
                    pattern_span: a.pattern_span,
                    body: a.body.as_ref().map(|b| substitute_irreturn(b, result_var)),
                    body_span: a.body_span,
                    fallthrough: a.fallthrough,
                })
                .collect(),
            default_body: default_body
                .as_ref()
                .map(|b| substitute_irreturn(b, result_var)),
            default_span: *default_span,
            mode: *mode,
            nocase: *nocase,
            raw_args: raw_args.clone(),
            raw_arg_braced: raw_arg_braced.clone(),
            raw_arg_quoted: raw_arg_quoted.clone(),
            command: command.clone(),
            patterns_braced: *patterns_braced,
        },
        other => other.clone(),
    }
}
