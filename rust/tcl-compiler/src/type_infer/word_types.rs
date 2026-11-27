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

//! The type a value word stores: nested command results, prepared expressions, class constructions and variable reads.

use super::container_elements::return_elements_lattice;
use super::definition_types::{DefTyping, evaluate_type_def};
use super::literals::literal_type;
use super::{WordTypingCtx, constructor_object_type, type_version};
use crate::expr_ast::ExprNode;
use crate::types::{TypeKind, TypeLattice, type_join};
use tcl_registry::TclType;

/// Successful result representation is independent of permission to emit a
/// native opcode. Nested operand types retain their exact read provenance.
fn normal_result_type<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    tokens: &crate::ir::CommandTokens,
) -> TypeLattice {
    if let Some(target) = tokens
        .source_binding
        .as_ref()
        .and_then(crate::command_binding::SourceInvocationBinding::proved_handler_target)
        && target.kind == crate::command_binding::BindingKind::Proc
        && let Some(allocation) = &target.implementation_allocation
        && let Some(result) = ctx
            .normal_results
            .and_then(|results| results.get(allocation))
    {
        return result.clone();
    }
    if let Some(result) = prepared_expression_result_type(ctx, tokens) {
        return result;
    }
    let Some(invocation) =
        crate::registry_invocation::normal_representation_invocation_with_metadata_context(
            ctx.registry,
            ctx.context,
            tokens,
        )
    else {
        return TypeLattice::overdefined();
    };
    if let Some(elements) = invocation.result_elements()
        && let Some(words) = invocation
            .effective_words()
            .words
            .get(1 + invocation.argument_offset()..)
    {
        let arguments = words
            .iter()
            .map(tcl_source::words::WordExpr::legacy_text)
            .collect::<Vec<_>>();
        let references = arguments.iter().map(String::as_str).collect::<Vec<_>>();
        if let Some(result) =
            return_elements_lattice(&ctx.with_tokens(tokens), elements, &references, Some(words))
        {
            return result;
        }
    }
    invocation
        .result_representation_type()
        .map_or_else(TypeLattice::overdefined, TypeLattice::of)
}

/// Conditional representation of an authenticated original expression.
/// Operand normalisation and current authored pool evidence are separate from
/// arithmetic category, physical object evidence and executable admission.
fn prepared_expression_result_type<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    tokens: &crate::ir::CommandTokens,
) -> Option<TypeLattice> {
    if let Some(advice) =
        crate::registry_invocation::original_expression_operand_advice(ctx.registry, tokens)
    {
        let variables =
            ctx.expression_variable_types(&advice.expression, Some(advice.expression_base));
        let receipt = tokens
            .source_binding
            .as_ref()?
            .conditional_expression_evaluation(ctx.registry, tokens)?;
        let math = crate::math_function_binding::ExpressionMathBindings::for_origin(
            ctx.math,
            Some(receipt.source()),
            Some(advice.expression_base),
        );
        return receipt
            .normal_result_representation(
                |node| {
                    let ExprNode::Var {
                        text, start, end, ..
                    } = node
                    else {
                        return None;
                    };
                    variables.get(&(text.clone(), *start, *end))?.tcl_type()
                },
                // Only a call whose dispatch reached a registry function
                // with no alias prefix has that function's result class.
                |node| {
                    let ExprNode::Call {
                        function, start, ..
                    } = node
                    else {
                        return None;
                    };
                    let call = math.resolved_call_for_value_analysis(function, *start)?;
                    call.prepended
                        .is_empty()
                        .then(|| tcl_syntax::expr::mathfunc::result_class(call.function))
                },
            )
            .map(TypeLattice::of);
    }
    let preparation = tokens
        .source_binding
        .as_ref()?
        .expression_preparation(ctx.preparations)?;
    let production = preparation.witness.normal_numeric_result_production()?;
    Some(TypeLattice::of(production.result_type()))
}

/// A lowered expression retains its original sole child invocation. Analysis
/// IR topology alone does not establish a current expression result intrep.
pub(super) fn lowered_expression_result_type<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    tokens: Option<&crate::ir::CommandTokens>,
    expression: &ExprNode,
    base: Option<u32>,
) -> TypeLattice {
    let Some(tokens) = tokens else {
        return TypeLattice::overdefined();
    };
    let config = ctx.config;
    let Some(calls) = crate::word_subst::checked_lifted_calls(tokens, config) else {
        return TypeLattice::overdefined();
    };
    let mut result = None;
    for call in calls {
        let Some(child) = call.tokens else {
            continue;
        };
        let Some(advice) =
            crate::registry_invocation::original_expression_operand_advice(ctx.registry, &child)
        else {
            continue;
        };
        if advice.expression != *expression || Some(advice.expression_base) != base {
            continue;
        }
        if result.is_some() {
            return TypeLattice::overdefined();
        }
        result = prepared_expression_result_type(ctx, &child);
    }
    result.unwrap_or_else(TypeLattice::overdefined)
}

/// Infer the intrep a `set`-style value *word* stores.
///
/// The shared body of the [`Statement::AssignValue`] typing and the
/// value-passthrough typing of a canonically-`set` [`Statement::Call`] (an
/// aliased or renamed `set`).  A pure `$var` reference inherits the source
/// version's type, a `[cmd …]` command substitution takes the command's
/// declared return type (or an object-collection retrieval), an
/// interpolated / otherwise-complex word is `String`, and a bare literal is
/// classified by its Tcl intrep.
pub(super) fn value_word_type<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    value: &str,
    retained: Option<&tcl_source::words::WordExpr>,
) -> TypeLattice {
    use tcl_source::words::WordExpr;
    use tcl_source::words::WordPart;
    let config = ctx.config;
    let parsed = retained
        .is_none()
        .then(|| crate::value_shapes::value_word_with_config(value, config))
        .flatten();
    let Some(word) = retained.or(parsed.as_ref()) else {
        return TypeLattice::overdefined();
    };
    let variable = match word {
        WordExpr::Variable { spelling, .. } => Some(spelling.as_str()),
        WordExpr::Template { parts, .. } => match parts.as_slice() {
            [WordPart::Variable { spelling, .. }] => Some(spelling.as_str()),
            _ => None,
        },
        _ => None,
    };
    if let Some(spelling) = variable {
        if ctx.source.is_positioned() {
            return retained_contents_type(ctx, word);
        }
        let Some(reference) = tcl_lexer::word_parts::whole_var_ref(spelling.as_bytes(), config)
            .ok()
            .flatten()
        else {
            return TypeLattice::overdefined();
        };
        let Ok(name) = core::str::from_utf8(reference.name) else {
            return TypeLattice::overdefined();
        };
        return ctx
            .variable_type(name)
            .unwrap_or_else(TypeLattice::overdefined);
    }
    if word.sole_command_substitution().is_some() {
        return command_word_result_type(ctx, word);
    }
    match word {
        WordExpr::Literal { text, .. } | WordExpr::BracedLiteral { text, .. } => {
            literal_type(text, ctx.numbers)
        }
        WordExpr::Template { .. } => TypeLattice::of(TclType::String),
        WordExpr::Expand { .. }
        | WordExpr::Opaque { .. }
        | WordExpr::Variable { .. }
        | WordExpr::CommandSubstitution { .. } => TypeLattice::overdefined(),
    }
}

/// A `[Class new …]` / `[Class create …]` word whose head names a class this
/// unit declares, typed as that class's object. This is a candidate type from
/// the written words, as a top-level constructor call's result is; it is not
/// the construction proof, which only `proved_construction_result` gives. A
/// registry command and a head bound to a builtin are never candidates.
fn source_class_construction_candidate<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    tokens: &crate::ir::CommandTokens,
) -> TypeLattice {
    fn literal(word: Option<&tcl_source::words::WordExpr>) -> Option<&str> {
        match word? {
            tcl_source::words::WordExpr::Literal { text, .. }
            | tcl_source::words::WordExpr::BracedLiteral { text, .. } => Some(text.as_str()),
            _ => None,
        }
    }
    let words = tokens.words();
    let (Some(head), Some(first)) = (literal(words.first()), literal(words.get(1))) else {
        return TypeLattice::overdefined();
    };
    if ctx.known_classes.is_empty()
        || ctx.registry.get(head).is_some()
        || tokens
            .source_binding
            .as_ref()
            .and_then(crate::command_binding::SourceInvocationBinding::proved_target)
            .is_some_and(|target| target.registry_backed)
    {
        return TypeLattice::overdefined();
    }
    constructor_object_type(
        ctx.registry,
        head,
        &[first],
        ctx.known_classes,
        ctx.namespace,
    )
}

/// Type the original sole nested command using the same source metadata.
fn command_word_result_type<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    word: &tcl_source::words::WordExpr,
) -> TypeLattice {
    let config = ctx.config;
    let Some(mut commands) =
        crate::value_shapes::command_substitution_tokens(word, ctx.tokens, config)
    else {
        return TypeLattice::overdefined();
    };
    // Earlier commands may replace the last implementation or return
    // abruptly. Their composed result is not a nominal string value.
    if commands.len() != 1 {
        return TypeLattice::overdefined();
    }
    let tokens = commands.remove(0);
    if let Some(class_name) = tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.proved_construction_result(ctx.registry))
    {
        return TypeLattice::object_of(class_name);
    }
    let candidate = source_class_construction_candidate(ctx, &tokens);
    if candidate.class_name().is_some() {
        return candidate;
    }
    let Some(invocation) =
        crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
            ctx.registry,
            ctx.context,
            &tokens,
        )
    else {
        return normal_result_type(ctx, &tokens);
    };
    if let Some(result) = prepared_expression_result_type(ctx, &tokens) {
        return result;
    }
    let arguments = invocation
        .arguments
        .iter()
        .map(Option::as_deref)
        .collect::<Option<Vec<_>>>();
    if let Some(fact) = invocation.facts.return_elements
        && let Some(arguments) = arguments.as_ref()
    {
        let elements = arguments
            .get(invocation.facts.argument_offset..)
            .unwrap_or(&[]);
        if let Some(value) = return_elements_lattice(
            &ctx.with_tokens(&tokens),
            fact,
            elements,
            invocation
                .effective
                .words
                .get(1 + invocation.facts.argument_offset..),
        ) {
            return value;
        }
    }
    invocation
        .facts
        .return_type
        .map_or_else(TypeLattice::overdefined, TypeLattice::of)
}

/// Join the actual stores reaching a read whose memory contents do not have
/// one scalar SSA version. The shared read owner proves root/lifetime overlap;
/// this adapter only supplies the value lattice of those retained statements.
pub(super) fn retained_contents_type<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    word: &tcl_source::words::WordExpr,
) -> TypeLattice {
    if let Some(read) = ctx.source.read_word(word)
        && let Some(version) = read.version
    {
        let version = type_version(ctx.ssa, read.symbol, version, ctx.types);
        return ctx
            .types
            .get(&(read.symbol, version))
            .cloned()
            .unwrap_or_else(TypeLattice::overdefined);
    }
    let Some(contents) = ctx.source.read_word_contents(word, ctx.registry) else {
        return TypeLattice::overdefined();
    };
    type_of_read_contents(ctx, contents)
}

pub(super) fn type_of_read_contents<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    contents: crate::ssa::SsaReadContents,
) -> TypeLattice {
    if contents.unknown_residual || ctx.ssa.cell_key(contents.reference.symbol).is_none() {
        return TypeLattice::overdefined();
    }
    if let Some(version) = contents.reference.version {
        let version = type_version(ctx.ssa, contents.reference.symbol, version, ctx.types);
        return ctx
            .types
            .get(&(contents.reference.symbol, version))
            .cloned()
            .unwrap_or_else(TypeLattice::overdefined);
    }
    let mut result = if contents.includes_incoming {
        TypeLattice::overdefined()
    } else {
        TypeLattice::unknown()
    };
    for (block, index) in contents.writes {
        if ctx.active_writes.contains(&(block, index)) {
            return TypeLattice::overdefined();
        }
        let Some(statement) = ctx
            .ssa
            .blocks
            .get(&block)
            .and_then(|block| block.statements.get(index))
        else {
            return TypeLattice::overdefined();
        };
        let mut active_writes = ctx.active_writes.to_vec();
        active_writes.push((block, index));
        let write_ctx = WordTypingCtx {
            preparations: ctx.preparations,
            math: ctx.math,
            tokens: statement.statement.tokens(),
            source: crate::ssa::SsaSourceView::at_statement(ctx.ssa, block, index),
            uses: &statement.uses,
            defs: Some(&statement.defs),
            context: ctx.context,
            config: ctx.config,
            types: ctx.types,
            values: ctx.values,
            registry: ctx.registry,
            known_classes: ctx.known_classes,
            namespace: ctx.namespace,
            ssa: ctx.ssa,
            numbers: ctx.numbers,
            active_writes: &active_writes,
            normal_results: ctx.normal_results,
        };
        let value = match evaluate_type_def(&statement.statement, &write_ctx) {
            DefTyping::Uniform(value) => value,
            DefTyping::PerDef(values) => ctx
                .ssa
                .cell_key(contents.reference.symbol)
                .and_then(|cell| values.get(cell))
                .cloned()
                .unwrap_or_else(TypeLattice::overdefined),
        };
        result = type_join(&result, &value);
    }
    if result.kind() == TypeKind::Unknown {
        TypeLattice::overdefined()
    } else {
        result
    }
}
