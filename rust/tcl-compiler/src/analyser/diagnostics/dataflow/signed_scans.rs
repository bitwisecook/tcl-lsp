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

//! IRULE5008: a value `binary scan` or `scan` stores as a signed integer, compared with a constant outside the signed range.
//!
//! iRules run Tcl 8.4, which has neither the `u` suffix on `binary scan`
//! fields nor an unsigned `scan` conversion: `binary scan \x81 c b` stores
//! `-127`, and `scan ffffffff %x v` stores `-1`. A byte compared with `0x81`
//! therefore never matches. The check follows the scanned SSA version
//! (through phis whose every input is the same kind of scan) to an `==`,
//! `!=`, `<`, `<=`, `>`, `>=`, `eq` or `ne` against an integer literal.

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;
use crate::cfg::Terminator;
use crate::expr_ast::{BinOp, ExprNode};
use crate::ir::Statement;
use crate::ssa::{SsaSourceView, ValueKey};
use tcl_core_types::DiagCode;
use tcl_registry::ArgRole;

/// Where a signed integer came from.
#[derive(Clone, PartialEq, Eq)]
struct SignedOrigin {
    bits: u32,
    field: String,
    command: &'static str,
}

impl SignedOrigin {
    fn range(&self) -> (i64, i64) {
        let half = 1i64 << (self.bits - 1);
        (-half, half - 1)
    }
}

struct Comparison<'a> {
    op: BinOp,
    variable: &'a ExprNode,
    literal: i64,
    literal_text: &'a str,
    variable_left: bool,
}

impl Analyser {
    /// IRULE5008 over one function body.
    pub(in super::super) fn emit_signed_scan_comparisons(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
    ) {
        if !self.profile.is_irules() {
            return;
        }
        let context = self.analysis_context();
        let registry = context.commands();
        let signed = signed_scan_values(fu, registry, self.profile);
        if signed.is_empty() {
            return;
        }
        let numbers = crate::intervals::numbers_for_dialect(Some(self.profile));
        let mut findings: Vec<(tcl_lexer::Span, String)> = Vec::new();
        let mut check = |expr: &ExprNode,
                         view: SsaSourceView<'_>,
                         exit: Option<&ExitVersions>,
                         base: Option<u32>,
                         fallback| {
            for comparison in comparisons(expr, numbers) {
                let Some(read) = view.read_expression_variable(comparison.variable, base) else {
                    continue;
                };
                let version = read
                    .version
                    .or_else(|| exit.and_then(|exit| exit.get(&read.symbol).copied()));
                let Some(origin) = version.and_then(|v| signed.get(&(read.symbol, v))) else {
                    continue;
                };
                let Some(message) = message(&comparison, origin) else {
                    continue;
                };
                let span = match (base, comparison.variable) {
                    (Some(base), ExprNode::Var { start, end, .. }) => {
                        tcl_lexer::Span::new(base + start, base + end + 1)
                    }
                    _ => fallback,
                };
                findings.push((fu.abs_span(span), message));
            }
        };
        for_each_expression(fu, self.file_lexer_config(), registry, &mut check);
        findings.sort_by_key(|(span, _)| (span.start(), span.end()));
        findings.dedup();
        for (span, message) in findings {
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::Irule5008,
                    span,
                    message,
                    Severity::Warning,
                ));
        }
    }
}

type ExitVersions = HashMap<crate::ssa::Symbol, crate::ssa::Version>;

/// Every expression the body evaluates: lowered `expr`/`return` operands,
/// `[expr …]` substitutions inside other commands, and branch conditions,
/// each with its SSA view, the block's exit versions for a condition, its
/// source base and a fallback span.
fn for_each_expression(
    fu: &crate::compilation_unit::FunctionUnit,
    config: tcl_lexer::LexerConfig,
    registry: &tcl_registry::CommandRegistry,
    visit: &mut impl FnMut(
        &ExprNode,
        SsaSourceView<'_>,
        Option<&ExitVersions>,
        Option<u32>,
        tcl_lexer::Span,
    ),
) {
    for (&block, data) in &fu.ssa.blocks {
        for (index, statement) in data.statements.iter().enumerate() {
            let view = SsaSourceView::at_statement(&fu.ssa, block, index);
            match &statement.statement {
                Statement::AssignExpr {
                    expr,
                    expr_base,
                    span,
                    ..
                }
                | Statement::ExprEval {
                    expr,
                    expr_base,
                    span,
                    ..
                }
                | Statement::Return {
                    expr: Some(expr),
                    expr_base,
                    span,
                    ..
                } => visit(expr, view, None, *expr_base, *span),
                _ => {
                    for evaluation in crate::word_subst::entered_expression_evaluations(
                        view.source_tokens(),
                        config,
                        registry,
                    ) {
                        visit(
                            evaluation.expression(),
                            view,
                            None,
                            Some(evaluation.expression_base()),
                            evaluation.span(),
                        );
                    }
                }
            }
        }
        if let Some(Terminator::Branch {
            condition,
            span: Some(span),
            condition_base,
            ..
        }) = fu
            .cfg
            .blocks
            .get(&block)
            .and_then(|b| b.terminator.as_ref())
        {
            visit(
                condition,
                SsaSourceView::at_terminator(&fu.ssa, block),
                Some(&data.exit_versions),
                *condition_base,
                *span,
            );
        }
    }
}

/// Every SSA version that holds a signed integer straight from a scan.
fn signed_scan_values(
    fu: &crate::compilation_unit::FunctionUnit,
    registry: &tcl_registry::CommandRegistry,
    profile: &tcl_dialect::DialectProfile,
) -> HashMap<ValueKey, SignedOrigin> {
    let mut signed = HashMap::new();
    for data in fu.ssa.blocks.values() {
        for statement in &data.statements {
            let Statement::Call {
                command,
                canonical_command,
                args,
                ..
            } = &statement.statement
            else {
                continue;
            };
            let name = canonical_command.as_deref().unwrap_or(command);
            let words: Vec<&str> = args.iter().map(String::as_str).collect();
            for (variable, origin) in scanned_fields(registry, name, &words, profile) {
                let mut defined = statement
                    .defs
                    .iter()
                    .filter(|&(&symbol, _)| fu.ssa.var_name(symbol) == variable);
                if let (Some((&symbol, &version)), None) = (defined.next(), defined.next()) {
                    signed.insert((symbol, version), origin);
                }
            }
        }
    }
    extend_through_phis(fu, &mut signed);
    signed
}

/// A phi whose every input is the same signed scan holds that scan's value.
fn extend_through_phis(
    fu: &crate::compilation_unit::FunctionUnit,
    signed: &mut HashMap<ValueKey, SignedOrigin>,
) {
    loop {
        let mut added = false;
        for data in fu.ssa.blocks.values() {
            for phi in &data.phis {
                let key = (phi.name, phi.version);
                if signed.contains_key(&key) {
                    continue;
                }
                let mut origin: Option<&SignedOrigin> = None;
                let all = phi.incoming.values().all(|&version| {
                    let Some(input) = signed.get(&(phi.name, version)) else {
                        return false;
                    };
                    origin.is_none_or(|seen| seen == input) && {
                        origin = Some(input);
                        true
                    }
                });
                if all && let Some(origin) = origin.cloned() {
                    signed.insert(key, origin);
                    added = true;
                }
            }
        }
        if !added {
            return;
        }
    }
}

/// `(variable, origin)` for each signed integer a literal scan template stores.
fn scanned_fields(
    registry: &tcl_registry::CommandRegistry,
    command: &str,
    words: &[&str],
    profile: &tcl_dialect::DialectProfile,
) -> Vec<(String, SignedOrigin)> {
    let formats = registry.format_string_args(command, words);
    let [format] = formats.as_slice() else {
        return Vec::new();
    };
    let Some(template) = words
        .get(format.index)
        .filter(|t| !t.contains(['$', '[', '\\']))
    else {
        return Vec::new();
    };
    let variables: Vec<&str> = registry
        .arg_indices_for_role(command, words, ArgRole::VarWrite)
        .into_iter()
        .filter(|&index| index > format.index)
        .filter_map(|index| words.get(index).copied())
        .collect();
    if !format.scan {
        return Vec::new();
    }
    let widths: Vec<Option<(u32, String)>> = match format.kind {
        tcl_registry::patterns::FormatType::Binary => {
            tcl_cmd_core::binary::specifiers(template.as_bytes(), true)
                .iter()
                .filter(|field| tcl_cmd_core::binary::scan_field_assigns(field.letter))
                .map(|field| {
                    tcl_cmd_core::binary::signed_scan_width(field)
                        .map(|bits| (bits, template[field.letter_start..field.end].to_owned()))
                })
                .collect()
        }
        tcl_registry::patterns::FormatType::Sprintf => {
            let fields = tcl_syntax::scan::conversion_spans(template, profile);
            if fields.iter().any(|field| field.spec.xpg_index.is_some()) {
                return Vec::new();
            }
            fields
                .iter()
                .filter(|field| field.spec.assigns())
                .map(|field| {
                    field.spec.truncates_to_int32_c84().then(|| {
                        (
                            32,
                            template[field.percent.start..field.conversion.end].to_owned(),
                        )
                    })
                })
                .collect()
        }
        _ => return Vec::new(),
    };
    let command = match format.kind {
        tcl_registry::patterns::FormatType::Binary => "binary scan",
        _ => "scan",
    };
    widths
        .into_iter()
        .zip(variables)
        .filter_map(|(width, variable)| {
            let (bits, field) = width?;
            let plain = !variable.contains(['$', '[', '(', '\\', '{', '"']);
            plain.then(|| {
                (
                    variable.to_owned(),
                    SignedOrigin {
                        bits,
                        field,
                        command,
                    },
                )
            })
        })
        .collect()
}

fn comparisons(expr: &ExprNode, numbers: tcl_dialect::NumberSyntax) -> Vec<Comparison<'_>> {
    expr.evaluation_nodes()
        .into_iter()
        .filter_map(|node| {
            let ExprNode::Binary { op, left, right } = node else {
                return None;
            };
            if !matches!(
                op,
                BinOp::Eq
                    | BinOp::Ne
                    | BinOp::Lt
                    | BinOp::Le
                    | BinOp::Gt
                    | BinOp::Ge
                    | BinOp::StrEq
                    | BinOp::StrNe
            ) {
                return None;
            }
            let (variable, constant, variable_left) = match (left.as_ref(), right.as_ref()) {
                (var @ ExprNode::Var { .. }, lit @ ExprNode::Literal { .. }) => (var, lit, true),
                (lit @ ExprNode::Literal { .. }, var @ ExprNode::Var { .. }) => (var, lit, false),
                _ => return None,
            };
            let ExprNode::Literal { text, .. } = constant else {
                return None;
            };
            let value = crate::intervals::literal_int(constant, numbers)?;
            // A string comparison sees the literal's spelling, so only a
            // canonical decimal literal is the same number to `eq`.
            if matches!(op, BinOp::StrEq | BinOp::StrNe) && value.to_string() != text.trim() {
                return None;
            }
            Some(Comparison {
                op: *op,
                variable,
                literal: value,
                literal_text: text.trim(),
                variable_left,
            })
        })
        .collect()
}

fn message(comparison: &Comparison<'_>, origin: &SignedOrigin) -> Option<String> {
    let (lo, hi) = origin.range();
    let k = comparison.literal;
    if (lo..=hi).contains(&k) {
        return None;
    }
    // Normalise to `variable OP literal`.
    let op = if comparison.variable_left {
        comparison.op
    } else {
        match comparison.op {
            BinOp::Lt => BinOp::Gt,
            BinOp::Le => BinOp::Ge,
            BinOp::Gt => BinOp::Lt,
            BinOp::Ge => BinOp::Le,
            other => other,
        }
    };
    let always = match op {
        BinOp::Eq | BinOp::StrEq => false,
        BinOp::Ne | BinOp::StrNe => true,
        BinOp::Lt | BinOp::Le => k > hi,
        BinOp::Gt | BinOp::Ge => k < lo,
        _ => return None,
    };
    let ExprNode::Var { text: var, .. } = comparison.variable else {
        return None;
    };
    let bits = origin.bits;
    let mask = format!("0x{:x}", (1u64 << bits) - 1);
    let modulus = 1i64 << bits;
    let equality = matches!(op, BinOp::Eq | BinOp::Ne | BinOp::StrEq | BinOp::StrNe);
    let signed_equivalent = (equality && k > hi && k < modulus).then(|| k - modulus);
    let mut text = format!(
        "{var} holds a signed {bits}-bit value from `{} … {}` ({lo}..{hi}), so comparing it with {} is always {}. ",
        origin.command,
        origin.field,
        comparison.literal_text,
        if always { "true" } else { "false" },
    );
    let _ = match signed_equivalent {
        Some(signed) => write!(
            text,
            "Mask it to unsigned, `({var} & {mask}) {} {}`, or compare with the signed value {signed}.",
            op.as_str(),
            comparison.literal_text
        ),
        None => write!(
            text,
            "Mask it to unsigned with `({var} & {mask})` before comparing."
        ),
    };
    Some(text)
}

#[cfg(test)]
mod tests;
