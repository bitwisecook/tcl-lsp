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

//! The cell and completion footprint of a lowered operation and of the operands it retains.

use super::{CellReference, LoweredFootprint};
use crate::expr_ast::ExprNode;
use crate::ir::Statement;
use tcl_core_types::Code as CompletionCode;

/// Project the exact cell and completion footprint of an already-lowered
/// operation from the statement the registry descriptor authorised.
///
/// Validation recomputes this from the retained statement and its exact
/// original source carrier. Missing name ownership keeps an unbounded target.
#[allow(clippy::too_many_lines)]
pub(super) fn lowered_operation_footprint(
    statement: &Statement,
    original_tokens: Option<&crate::ir::CommandTokens>,
) -> LoweredFootprint {
    match statement {
        Statement::AssignConst { name, .. } => {
            let write = lowered_source_name_reference(statement, original_tokens, name);
            let exact = write.name().is_some();
            LoweredFootprint {
                writes: vec![write],
                reads: Vec::new(),
                reads_unbounded: false,
                runs_commands: false,
                // A constant assignment to a statically named cell has no
                // operand that can fail; a computed target name can.
                completion: if exact {
                    vec![CompletionCode::Ok]
                } else {
                    vec![CompletionCode::Ok, CompletionCode::Error]
                },
            }
        }
        Statement::AssignValue { name, value, .. } => {
            let operand = operand_text_footprint(value);
            LoweredFootprint {
                writes: vec![lowered_source_name_reference(
                    statement,
                    original_tokens,
                    name,
                )],
                reads: Vec::new(),
                reads_unbounded: operand.reads_unbounded,
                runs_commands: operand.runs_commands,
                completion: vec![CompletionCode::Ok, CompletionCode::Error],
            }
        }
        Statement::AssignExpr {
            name,
            expr,
            expr_base,
            ..
        } => {
            let operand = expr_footprint(expr, *expr_base, original_tokens);
            LoweredFootprint {
                writes: vec![lowered_source_name_reference(
                    statement,
                    original_tokens,
                    name,
                )],
                reads: operand.reads,
                reads_unbounded: operand.reads_unbounded,
                runs_commands: operand.runs_commands,
                completion: vec![CompletionCode::Ok, CompletionCode::Error],
            }
        }
        Statement::Incr { name, amount, .. } => {
            let target = lowered_source_name_reference(statement, original_tokens, name);
            let operand = amount
                .as_deref()
                .map_or_else(OperandFootprint::exact, operand_text_footprint);
            LoweredFootprint {
                writes: vec![target.clone()],
                // `incr` reads its target before writing it.
                reads: vec![target],
                reads_unbounded: operand.reads_unbounded,
                runs_commands: operand.runs_commands,
                completion: vec![CompletionCode::Ok, CompletionCode::Error],
            }
        }
        Statement::ExprEval {
            expr, expr_base, ..
        } => {
            let operand = expr_footprint(expr, *expr_base, original_tokens);
            LoweredFootprint {
                writes: Vec::new(),
                reads: operand.reads,
                reads_unbounded: operand.reads_unbounded,
                runs_commands: operand.runs_commands,
                completion: vec![CompletionCode::Ok, CompletionCode::Error],
            }
        }
        Statement::Return {
            value,
            expr,
            expr_base,
            braced,
            ..
        } => {
            let operand = match (expr, value) {
                (Some(expr), _) => expr_footprint(expr, *expr_base, original_tokens),
                // A braced result word suppresses substitution entirely.
                (None, Some(_)) if *braced => OperandFootprint::exact(),
                (None, Some(value)) => operand_text_footprint(value),
                (None, None) => OperandFootprint::exact(),
            };
            let mut completion = Vec::new();
            if operand.runs_commands || operand.reads_unbounded {
                completion.push(CompletionCode::Error);
            }
            completion.push(CompletionCode::Return);
            LoweredFootprint {
                writes: Vec::new(),
                reads: operand.reads,
                reads_unbounded: operand.reads_unbounded,
                runs_commands: operand.runs_commands,
                completion,
            }
        }
        Statement::Call { .. }
        | Statement::Barrier { .. }
        | Statement::NativeCall { .. }
        | Statement::Block { .. }
        | Statement::UpFrame { .. }
        | Statement::If { .. }
        | Statement::For { .. }
        | Statement::While { .. }
        | Statement::Foreach { .. }
        | Statement::Catch { .. }
        | Statement::Try { .. }
        | Statement::Switch { .. } => LoweredFootprint::conservative(),
    }
}

fn lowered_source_name_reference(
    statement: &Statement,
    original_tokens: Option<&crate::ir::CommandTokens>,
    name: &str,
) -> CellReference {
    let original_name = || {
        let tokens = original_tokens?;
        (tokens.argv_texts.get(1).map(String::as_str) == Some(name)).then_some(())?;
        let (command, _) = tokens
            .source_binding
            .as_ref()?
            .original_recorded_command()?;
        (command.span == statement.span()).then_some(())?;
        crate::registry_invocation::original_literal_argument_data(tokens, 0)
    };
    original_name().map_or(CellReference::Computed, |name| {
        CellReference::from_name_data(&name)
    })
}

/// What evaluating one retained operand can read and run.
pub(super) struct OperandFootprint {
    pub(super) reads: Vec<CellReference>,
    pub(super) reads_unbounded: bool,
    pub(super) runs_commands: bool,
}

impl OperandFootprint {
    /// An operand that neither reads a cell nor runs a command.
    pub(super) const fn exact() -> Self {
        Self {
            reads: Vec::new(),
            reads_unbounded: false,
            runs_commands: false,
        }
    }
}

/// The footprint of a retained operand word held as exact text.
///
/// The text has already lost the token shape needed to enumerate its variable
/// references without re-running word substitution, so a substituted operand
/// reports unbounded reads rather than a guessed list. A bracket makes it a
/// command-running operand, which no cell footprint can bound.
fn operand_text_footprint(text: &str) -> OperandFootprint {
    OperandFootprint {
        reads: Vec::new(),
        reads_unbounded: text.contains('$') || text.contains('['),
        runs_commands: text.contains('['),
    }
}

/// The footprint of a parsed Tcl expression.
///
/// The walk stops at an unparsed `Raw` node rather than re-lexing its text:
/// a `Raw` node already makes the footprint unbounded, so enumerating the
/// variables inside it would change nothing — and this projection is
/// recomputed by validation from the retained expression without inventing
/// a grammar for an unparsed node.
pub(super) fn expr_footprint(
    expr: &ExprNode,
    expression_base: Option<u32>,
    original_tokens: Option<&crate::ir::CommandTokens>,
) -> OperandFootprint {
    let mut result = OperandFootprint::exact();
    let syntax = original_tokens.and_then(|tokens| {
        crate::registry_invocation::original_lowered_expression_syntax(
            expr,
            expression_base,
            tokens,
        )
    });
    for node in expr.evaluation_nodes() {
        match node {
            ExprNode::Var { .. } => {
                let Some((config, policy)) = syntax else {
                    result.reads.push(CellReference::Computed);
                    result.reads_unbounded = true;
                    continue;
                };
                let Ok(Some(reference)) = node.variable_reference(config) else {
                    result.reads.push(CellReference::Computed);
                    result.reads_unbounded = true;
                    continue;
                };
                result.reads.push(original_reference_cell(
                    reference.name,
                    reference.index.is_some(),
                    policy,
                ));
                if let Some(index) = reference.index {
                    original_substitution_reads(
                        index,
                        config,
                        policy,
                        tcl_lexer::word_parts::SubstFlags::default(),
                        true,
                        &mut result,
                    );
                }
            }
            ExprNode::String { text, .. } => original_string_reads(text, syntax, &mut result),
            ExprNode::CompiledWord {
                text,
                braced: false,
            } => {
                if let Some((config, policy)) = syntax {
                    if tcl_syntax::word_rules::compiled_word_substitution(text.as_bytes(), config)
                        == tcl_syntax::word_rules::CompiledWordSubstitution::LiteralValue
                    {
                        continue;
                    }
                    original_substitution_reads(
                        text.as_bytes(),
                        config,
                        policy,
                        tcl_lexer::word_parts::SubstFlags::compiled_word(),
                        false,
                        &mut result,
                    );
                } else {
                    result.reads_unbounded = true;
                    result.runs_commands = true;
                }
            }
            ExprNode::Command { .. } | ExprNode::Raw { .. } | ExprNode::Call { .. } => {
                result.reads_unbounded = true;
                result.runs_commands = true;
            }
            ExprNode::Literal { .. }
            | ExprNode::CompiledWord { braced: true, .. }
            | ExprNode::Binary { .. }
            | ExprNode::Unary { .. }
            | ExprNode::Ternary { .. } => {}
        }
    }
    result.reads.sort();
    result.reads.dedup();
    result.reads_unbounded |= result.reads.iter().any(|cell| cell.name().is_none());
    result
}

fn original_string_reads(
    text: &str,
    syntax: Option<(
        tcl_lexer::LexerConfig,
        Option<tcl_syntax::naming::NamePolicyProtocol>,
    )>,
    result: &mut OperandFootprint,
) {
    if crate::expr_ast::fixed_string_operand(text).is_some() {
        return;
    }
    if let Some(body) = crate::expr_ast::quoted_string_body(text) {
        if let Some((config, policy)) = syntax {
            original_substitution_reads(
                body.as_bytes(),
                config,
                policy,
                tcl_lexer::word_parts::SubstFlags::default(),
                false,
                result,
            );
        } else {
            result.reads_unbounded = true;
            result.runs_commands = true;
        }
    } else if tcl_syntax::word_rules::whole_braced_word(text).is_none() {
        result.reads_unbounded = true;
        result.runs_commands = true;
    }
}

fn original_reference_cell(
    name: &[u8],
    separate_index: bool,
    policy: Option<tcl_syntax::naming::NamePolicyProtocol>,
) -> CellReference {
    if name.contains(&0) {
        return CellReference::Computed;
    }
    let named = |root: &[u8], element| {
        std::str::from_utf8(root).map_or(CellReference::Computed, |name| CellReference::Named {
            name: name.to_owned(),
            element,
        })
    };
    match policy {
        Some(policy) if separate_index => {
            named(policy.recipe().variable_root_input(name).selected(), true)
        }
        Some(policy) => {
            let selected = policy.recipe().combined_variable_input(name);
            named(selected.root().selected(), selected.element().is_some())
        }
        None if !separate_index && tcl_syntax::naming::split_element_ref_bytes(name).is_none() => {
            named(name, false)
        }
        None => CellReference::Computed,
    }
}

fn original_substitution_reads(
    source: &[u8],
    config: tcl_lexer::LexerConfig,
    policy: Option<tcl_syntax::naming::NamePolicyProtocol>,
    flags: tcl_lexer::word_parts::SubstFlags,
    in_index: bool,
    footprint: &mut OperandFootprint,
) {
    use tcl_lexer::word_parts::{WordPart, decompose_spanned_checked};
    let Ok(parts) = decompose_spanned_checked(source, flags, config) else {
        footprint.reads_unbounded = true;
        footprint.runs_commands = true;
        return;
    };
    let mut pending: Vec<_> = parts
        .into_iter()
        .map(|part| (part.part, in_index))
        .collect();
    while let Some((part, in_index)) = pending.pop() {
        match part {
            WordPart::Text(_) => {}
            WordPart::Variable(reference) => {
                footprint.reads_unbounded |= in_index;
                footprint.reads.push(original_reference_cell(
                    reference.name,
                    reference.index.is_some(),
                    policy,
                ));
                if let Some(index) = reference.index {
                    pending.extend(index.into_iter().map(|part| (part, true)));
                }
            }
            WordPart::Command(_) | WordPart::Expression(_) | WordPart::ParseError(_) => {
                footprint.reads_unbounded = true;
                footprint.runs_commands = true;
            }
        }
    }
}
