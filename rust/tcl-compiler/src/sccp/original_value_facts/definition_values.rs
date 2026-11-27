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

//! The value one definition takes: retained expression words, original substitutions, iteration bindings and normal stores.

use tcl_registry::value_transfer::NumericValue;

use super::super::{
    BuiltinFoldInputs, extract_foreach_elements, parse_literal_value,
    resolve_foreach_list_via_lattice,
};
use super::assignment_folds::{fold_assign_value, lattice_const_text, try_fold_cmd_subst};
use super::math_expressions::{env_from_uses, fold_math_expression};
use super::{MathFoldContext, RetainedExpressionFoldInputs, SolverPurpose};
use crate::analyses::{ConstValue, LatticeValue};
use crate::codegen_helpers::split_list_values;
use crate::ir::Statement;
use crate::ssa::{SsaSourceView, SsaStatement, Symbol, ValueKey};
use crate::tcl_expr_eval::{Env, FoldPolicy};
use std::collections::HashMap;

#[cfg(test)]
pub(in super::super) fn evaluate_fixture_def<'a, S: std::hash::BuildHasher>(
    stmt_ssa: &SsaStatement,
    values: &HashMap<ValueKey, LatticeValue, S>,
    ssa: impl Into<SsaSourceView<'a>>,
    policy: FoldPolicy,
    config: tcl_lexer::LexerConfig,
) -> LatticeValue {
    evaluate_fixture_def_with_folds(stmt_ssa, values, ssa, policy, None, config)
}

#[cfg(test)]
pub(in super::super) fn evaluate_fixture_def_with_folds<'a, S: std::hash::BuildHasher>(
    stmt_ssa: &SsaStatement,
    values: &HashMap<ValueKey, LatticeValue, S>,
    ssa: impl Into<SsaSourceView<'a>>,
    policy: FoldPolicy,
    folds: Option<BuiltinFoldInputs<'_>>,
    config: tcl_lexer::LexerConfig,
) -> LatticeValue {
    evaluate_def_with_math(
        stmt_ssa,
        values,
        ssa.into(),
        policy,
        folds,
        None,
        Some(config),
    )
}

pub(super) fn evaluate_def_with_math<S: std::hash::BuildHasher>(
    stmt_ssa: &SsaStatement,
    values: &HashMap<ValueKey, LatticeValue, S>,
    ssa: SsaSourceView<'_>,
    policy: FoldPolicy,
    folds: Option<BuiltinFoldInputs<'_>>,
    math: Option<MathFoldContext<'_>>,
    reference_config: Option<tcl_lexer::LexerConfig>,
) -> LatticeValue {
    match &stmt_ssa.statement {
        Statement::AssignConst { value, .. } => LatticeValue::Const(parse_literal_value(value)),
        Statement::AssignExpr { expr, .. } => {
            let env = env_from_uses(&stmt_ssa.uses, values, ssa);
            fold_math_expression(expr, &env, policy, math)
                .map_or(LatticeValue::Overdefined, |value| {
                    LatticeValue::Const(value.into_const())
                })
        }
        Statement::AssignValue { value, .. } => {
            // Fold when the RHS is either a plain literal
            // (no command substitution), a simple `$var` that
            // resolves to a lattice Const, or a `[cmd args...]`
            // that try_fold_cmd_subst (or, under `folds`, the registry
            // const-fold engine) recognises.
            fold_retained_assignment_expression(stmt_ssa, values, ssa, policy, folds, math)
                .unwrap_or_else(|| {
                    fold_assign_value(
                        value,
                        &stmt_ssa.uses,
                        values,
                        ssa,
                        policy,
                        folds,
                        reference_config,
                    )
                })
        }
        Statement::Call {
            command,
            args,
            defs,
            tokens,
            ..
        } if single_iteration_binding(command, args, defs, tokens.as_ref()) => {
            fold_iteration_value(
                &args[0],
                stmt_ssa,
                values,
                ssa,
                policy,
                folds,
                reference_config,
            )
        }
        Statement::Call {
            tokens: Some(tokens),
            ..
        } => fold_normal_store_value(tokens, stmt_ssa, values, ssa, policy, folds, math)
            .unwrap_or(LatticeValue::Overdefined),
        Statement::Incr { name, amount, .. } => {
            // Track `incr NAME ?AMOUNT?` through the lattice
            // when the current value of NAME is a single Const(Int)
            // and AMOUNT is either absent (defaults to 1), a decimal
            // integer literal, or a simple `$var` reference that
            // resolves to Const(Int) via `uses`.
            // A dynamic-key target (`incr a($i)`) never interns a symbol —
            // that miss is permanent, so it must widen: returning Unknown
            // would launder a fanned element's stale constant through
            // `join(prev, Unknown) = prev`.
            let Some(sym) = ssa.symbol(crate::naming::element_var_name(name)) else {
                return LatticeValue::Overdefined;
            };
            let ver = stmt_ssa.uses.get(&sym).copied().unwrap_or(0);
            let base = values
                .get(&(sym, ver))
                .cloned()
                .unwrap_or(LatticeValue::Unknown);
            let base_int = match &base {
                LatticeValue::Const(ConstValue::Number(NumericValue::Int(i))) => *i,
                LatticeValue::Unknown => return LatticeValue::Unknown,
                // Overdefined or a non-integer Const widens.
                _ => return LatticeValue::Overdefined,
            };
            let amt = match amount.as_deref() {
                None => 1,
                Some(text) => {
                    let trimmed = text.trim();
                    if let Ok(v) = trimmed.parse::<i64>() {
                        v
                    } else if let Some(amount) = resolve_simple_var_ref(
                        trimmed,
                        &stmt_ssa.uses,
                        values,
                        ssa,
                        reference_config,
                    ) {
                        match amount {
                            LatticeValue::Const(ConstValue::Number(NumericValue::Int(i))) => i,
                            LatticeValue::Unknown => return LatticeValue::Unknown,
                            _ => return LatticeValue::Overdefined,
                        }
                    } else {
                        return LatticeValue::Overdefined;
                    }
                }
            };
            base_int
                .checked_add(amt)
                .map_or(LatticeValue::Overdefined, |v| {
                    LatticeValue::Const(ConstValue::int(v))
                })
        }
        _ => LatticeValue::Overdefined,
    }
}

fn fold_retained_assignment_expression<S: std::hash::BuildHasher>(
    statement: &SsaStatement,
    values: &HashMap<ValueKey, LatticeValue, S>,
    source: SsaSourceView<'_>,
    policy: FoldPolicy,
    folds: Option<BuiltinFoldInputs<'_>>,
    math: Option<MathFoldContext<'_>>,
) -> Option<LatticeValue> {
    let inputs = folds?;
    let registry = inputs.registry;
    let metadata = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
        registry,
        inputs.source_metadata_input?,
    )?;
    let tokens = statement.statement.tokens()?;
    let invocation = crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
        registry,
        Some(metadata),
        tokens,
    )?;
    let (_, word) = invocation
        .stored_value_operand(&tokens.source_binding.as_ref()?.variable_context, registry)?;
    fold_retained_expression_word(
        word,
        tokens,
        statement,
        values,
        source,
        RetainedExpressionFoldInputs {
            source_metadata_input: inputs.source_metadata_input,
            registry,
            policy,
            math,
        },
    )
}

fn fold_retained_expression_word<S: std::hash::BuildHasher>(
    word: &tcl_source::words::WordExpr,
    tokens: &crate::ir::CommandTokens,
    statement: &SsaStatement,
    values: &HashMap<ValueKey, LatticeValue, S>,
    source: SsaSourceView<'_>,
    inputs: RetainedExpressionFoldInputs<'_>,
) -> Option<LatticeValue> {
    let (spelling, site) = word.sole_command_substitution()?;
    if let Some(value) =
        fold_original_substitution_value(tokens, site, &statement.uses, values, source, inputs)
    {
        return Some(LatticeValue::Const(parse_literal_value(&value)));
    }
    if inputs
        .math?
        .dependencies
        .is_some_and(|ledger| ledger.purpose == SolverPurpose::SemanticAnalysis)
        && let Some(value) = fold_prepared_invocation_expression(spelling, site, inputs)
    {
        return Some(value);
    }
    let input = inputs.source_metadata_input?;
    let context = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
        inputs.registry,
        input,
    )?;
    let mut parser = tcl_syntax::expr::parser::ExprParseContext::for_profile(input.unit_profile());
    parser.lexer_grammar = input.lexer_config().grammar_over(parser.lexer_grammar);
    let expressions = crate::word_subst::lifted_source_expressions_with_metadata_context(
        Some(tokens),
        inputs.registry,
        Some(context),
        input.lexer_config(),
        parser,
    );
    let mut matching = expressions
        .iter()
        .filter(|expression| expression.span == site.span);
    let expression = matching.next()?;
    if matching.next().is_some() {
        return None;
    }
    let context = inputs.math?;
    let context = MathFoldContext {
        bindings: context.bindings.at_base(expression.expression_base),
        ..context
    };
    let environment = env_from_uses(&statement.uses, values, source);
    fold_math_expression(
        &expression.expression,
        &environment,
        inputs.policy,
        Some(context),
    )
    .map_or(Some(LatticeValue::Overdefined), |value| {
        Some(LatticeValue::Const(value.into_const()))
    })
}

/// A selected original substitution and its complete child inventory.
/// Missing actual input or handler identity never falls back to written names.
pub(super) fn fold_original_substitution_value<
    S1: std::hash::BuildHasher,
    S2: std::hash::BuildHasher,
>(
    tokens: &crate::ir::CommandTokens,
    site: &tcl_source::words::SourceSite,
    uses: &HashMap<Symbol, crate::ssa::Version, S1>,
    values: &HashMap<ValueKey, LatticeValue, S2>,
    source: SsaSourceView<'_>,
    inputs: RetainedExpressionFoldInputs<'_>,
) -> Option<String> {
    let input = inputs.source_metadata_input?;
    let metadata = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
        inputs.registry,
        input,
    )?;
    let calls = crate::word_subst::checked_lifted_calls(tokens, input.lexer_config().nested())?;
    let mut selected = calls.iter().filter(|call| call.span == site.span);
    let call = selected.next()?;
    if selected.next().is_some() {
        return None;
    }
    let lookup = |name: &str| lattice_const_text(name, uses, values, source, inputs.policy);
    let trust = |_name: &str| false;
    crate::const_subst::ConstSubstCtx {
        registry: inputs.registry,
        resolution_namespace: "::",
        namespace_context: None,
        version: input.unit_profile().const_fold_version(),
        defining_class: None,
        trusts: &trust,
        lookup_var: &lookup,
        script_encoding: tcl_registry::value_transfer::ScriptEncoding::Auto,
    }
    .fold_retained_call_with_metadata_context(
        call,
        crate::const_subst::RetainedCalls::new(&calls),
        metadata,
    )
}

fn fold_prepared_invocation_expression(
    spelling: &str,
    site: &tcl_source::words::SourceSite,
    inputs: RetainedExpressionFoldInputs<'_>,
) -> Option<LatticeValue> {
    let grammar = inputs.policy.preparation_context()?.lexer_grammar;
    let invocation = crate::word_subst::nested_command_words(
        spelling,
        site,
        tcl_lexer::LexerConfig::from_grammar(grammar),
    )
    .ok()?;
    let offset = invocation.argv.first()?.start();
    let context = inputs.math?;
    let context = MathFoldContext {
        bindings: context.bindings.at_invocation(offset)?,
        ..context
    };
    let preparation = context
        .bindings
        .preparation_for_context(&inputs.policy.preparation_context()?)?;
    fold_math_expression(
        preparation.witness.tree(),
        &Env::default(),
        inputs.policy,
        Some(context),
    )
    .map(|value| LatticeValue::Const(value.into_const()))
}

fn fold_iteration_value<S: std::hash::BuildHasher>(
    argument: &str,
    statement: &SsaStatement,
    values: &HashMap<ValueKey, LatticeValue, S>,
    source: SsaSourceView<'_>,
    policy: FoldPolicy,
    folds: Option<BuiltinFoldInputs<'_>>,
    reference_config: Option<tcl_lexer::LexerConfig>,
) -> LatticeValue {
    // `foreach v LIST` / `lmap v LIST` folds the
    // iteration variable to the CONSTSET of elements when
    // LIST is a literal, resolves to a Const(String)
    // through the lattice, or is a command substitution
    // (`[list a b c]`, `[format …]`) that folds to a
    // constant list. Multi-variable and multi-list
    // foreaches are left as Overdefined.
    let elements = extract_foreach_elements(argument, policy.word_rules)
        .or_else(|| {
            resolve_foreach_list_via_lattice(
                argument,
                &statement.uses,
                values,
                source,
                policy.word_rules,
                reference_config,
            )
        })
        .or_else(|| {
            // `foreach v [list a b c]` — fold the command substitution
            // to a constant list string, then split into elements.
            let arg = argument.trim();
            if arg.starts_with('[')
                && arg.ends_with(']')
                && let Some(LatticeValue::Const(ConstValue::String(s))) = try_fold_cmd_subst(
                    arg,
                    &statement.uses,
                    values,
                    source,
                    policy,
                    folds,
                    reference_config,
                )
            {
                return Some(split_list_values(s.as_str(), policy.word_rules));
            }
            None
        });
    match elements {
        Some(items) if items.is_empty() => LatticeValue::Overdefined,
        Some(items) => {
            let consts: Vec<ConstValue> = items.iter().map(|s| parse_literal_value(s)).collect();
            if consts.len() == 1 {
                LatticeValue::Const(consts.into_iter().next().unwrap())
            } else {
                LatticeValue::constset(consts)
            }
        }
        None => LatticeValue::Overdefined,
    }
}

/// A normal handler's bounded store is separate from its compiler completion.
/// Frozen argv values take precedence; original variable reads retain their
/// exact temporal SSA dependency. This never licenses executing/eliding a call.
fn fold_normal_store_value<S: std::hash::BuildHasher>(
    tokens: &crate::ir::CommandTokens,
    statement: &SsaStatement,
    values: &HashMap<ValueKey, LatticeValue, S>,
    source: SsaSourceView<'_>,
    policy: FoldPolicy,
    folds: Option<BuiltinFoldInputs<'_>>,
    math: Option<MathFoldContext<'_>>,
) -> Option<LatticeValue> {
    let inputs = folds?;
    let registry = inputs.registry;
    let metadata = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
        registry,
        inputs.source_metadata_input?,
    )?;
    let binding = tokens.source_binding.as_ref()?;
    let invocation = crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
        registry,
        Some(metadata),
        tokens,
    )?;
    let (spelling, word) = invocation.stored_value_operand(&binding.variable_context, registry)?;
    if let Some(value) = fold_retained_expression_word(
        word,
        tokens,
        statement,
        values,
        source,
        RetainedExpressionFoldInputs {
            source_metadata_input: inputs.source_metadata_input,
            registry,
            policy,
            math,
        },
    ) {
        return Some(value);
    }
    if let Some(value) = invocation.stored_value_literal(&binding.variable_context, registry) {
        return Some(LatticeValue::Const(parse_literal_value(&value)));
    }
    if source.is_positioned() && word.sole_variable_substitution().is_some() {
        return Some(
            source
                .read_word(word)
                .and_then(|read| read.version.map(|version| (read.symbol, version)))
                .map_or(LatticeValue::Overdefined, |key| {
                    values.get(&key).cloned().unwrap_or(LatticeValue::Unknown)
                }),
        );
    }
    Some(fold_assign_value(
        spelling,
        &statement.uses,
        values,
        source,
        policy,
        folds,
        Some(inputs.source_metadata_input?.lexer_config()),
    ))
}

fn single_iteration_binding(
    command: &str,
    args: &[String],
    defs: &[String],
    tokens: Option<&crate::ir::CommandTokens>,
) -> bool {
    defs.len() == 1
        && args.len() == 1
        && tokens.map_or_else(
            // value-transfer-ok: irreducible — the tokenless legacy caller has
            // no retained invocation to ask; tokens consult the synthetic marker.
            // registry-axis-ok: command — tokenless legacy fold; the tokened path asks the registry; until the tokenless SCCP caller is retired
            || matches!(command, "foreach" | "lmap"),
            |tokens| {
                matches!(
                    tokens.synthetic,
                    Some(crate::ir::SyntheticMarker::IterationBindings(_))
                )
            },
        )
}

/// Resolve `$var` / `${var}` to a lattice value by looking up the
/// SSA version in `uses` and indexing `values`. Returns None when
/// the text isn't a simple var reference.
pub(super) fn source_read_key<S: std::hash::BuildHasher>(
    name: &str,
    uses: &HashMap<Symbol, crate::ssa::Version, S>,
    source: SsaSourceView<'_>,
) -> Option<ValueKey> {
    if source.is_positioned() {
        let read = source.read_spelling(name)?;
        Some((read.symbol, read.version?))
    } else {
        let symbol = source.symbol(name)?;
        Some((symbol, *uses.get(&symbol)?))
    }
}

pub(super) fn resolve_simple_var_ref<'a, S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    text: &str,
    uses: &HashMap<Symbol, crate::ssa::Version, S1>,
    values: &HashMap<ValueKey, LatticeValue, S2>,
    ssa: impl Into<SsaSourceView<'a>>,
    reference_config: Option<tcl_lexer::LexerConfig>,
) -> Option<LatticeValue> {
    let ssa = ssa.into();
    let name = tcl_syntax::naming::whole_scalar_variable_reference_name(text, reference_config?)?;
    let key = source_read_key(name, uses, ssa)?;
    Some(values.get(&key).cloned().unwrap_or(LatticeValue::Unknown))
}
