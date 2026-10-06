// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original physical read/store and native conversion schedule for increments.

use crate::ir::{Script, Statement};
use tcl_registry::CommandRegistry;

/// A closed source edit, including any original literal normalization that
/// must remain executed. Type labels cannot construct this witness.
pub(crate) struct SourceIncrementRewrite {
    replacement: String,
}

impl SourceIncrementRewrite {
    pub(crate) fn replacement(self) -> String {
        self.replacement
    }
}

/// Conversion acceptance and already-normalised representation are different
/// obligations. A closed contents union keeps the original native normalisation
/// executed instead of granting erasure authority.
#[derive(Clone, Copy)]
enum OperandNormalisation {
    AlreadyInteger,
    RetainOriginal,
}

/// Prove the original single-read arithmetic and its captured setter address.
/// Unknown operands, callbacks, element addresses, native handlers or integer
/// overflow decline. Amount conversion remains separate from value equality.
pub(crate) fn assess_increment_rewrite(
    script: &Script,
    statement: &Statement,
    registry: &CommandRegistry,
) -> Option<SourceIncrementRewrite> {
    use crate::registry_invocation::{normal_transfer_invocation, resolved_tokens_invocation};
    use tcl_registry::{SemanticOperationId, hooks::LoweringHookId};
    let tokens = script.retained_source_tokens_for_statement(statement)?;
    let setter = resolved_tokens_invocation(registry, None, tokens)?;
    if setter.facts.operation != SemanticOperationId::StructuredLowering(LoweringHookId::Set)
        || !setter.effective.binding_prefix.is_empty()
    {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    if !binding.unobserved_native_dispatch() {
        return None;
    }
    let dialect = binding.variable_context.invocation_dialect?;
    let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
    let transfer = normal_transfer_invocation(registry, None, tokens)?;
    let name = transfer.argument_literal(0)?;
    if !super::optimiser::helpers::literals::is_safe_word(&name) {
        return None;
    }
    if let Statement::AssignExpr { name: lowered, .. } = statement
        && lowered != &name
    {
        return None;
    }
    let word = transfer.stored_value_word(&binding.variable_context, registry)?;
    let mut nested = crate::value_shapes::command_substitution_tokens(word, Some(tokens), config)?;
    if nested.len() != 1 {
        return None;
    }
    let expression = nested.remove(0);
    let selected = resolved_tokens_invocation(registry, None, &expression)?;
    if selected.facts.operation != SemanticOperationId::StructuredLowering(LoweringHookId::Expr)
        || !selected.effective.binding_prefix.is_empty()
        || selected.dialect != Some(dialect)
    {
        return None;
    }
    if !expression
        .source_binding
        .as_ref()?
        .unobserved_native_dispatch()
    {
        return None;
    }
    let preparation = expression
        .source_binding
        .as_ref()?
        .expression_preparation(&script.expression_preparations)?;
    let schedule = preparation.witness.increment_expression_schedule()?;
    let normalisation = prove_integer_read_store(
        tokens,
        preparation,
        schedule.operand(),
        schedule.amount(),
        dialect,
        registry,
    )?;
    prospective_increment(
        script,
        tokens,
        &expression,
        &name,
        &schedule,
        normalisation,
        registry,
    )
}

/// A single retained read and the original setter select the same physical
/// cell and contents origin; integer parsing uses its actual native policy.
fn prove_integer_read_store(
    tokens: &crate::ir::CommandTokens,
    preparation: &crate::command_binding::SourceExpressionPreparation,
    operand: &crate::expr_ast::ExprNode,
    amount: i64,
    dialect: tcl_registry::InvocationDialect,
    registry: &CommandRegistry,
) -> Option<OperandNormalisation> {
    let binding = tokens.source_binding.as_ref()?;
    let (_, source) = preparation.original_variable_source(operand)?;
    let mut accesses = tokens.variable_accesses.iter().filter(|access| {
        access.source == source
            && matches!(&access.owner, crate::command_binding::SourceVariableEvaluationOwner::NativeExpression { invocation, .. } if invocation == &preparation.invocation)
    });
    let read = accesses.next()?;
    if accesses.next().is_some()
        || read.context_residual() != crate::command_binding::SourceVariableReadResidual::Closed
    {
        return None;
    }
    let observations = binding.sole_rhs_read_store_observations()?;
    if observations.is_empty()
        || read.context_alternatives().is_empty()
        || !read.context_alternatives().iter().all(|context| {
            observations.iter().any(|observation| {
                observation.expression() == &preparation.invocation
                    && observation.operand() == &source
                    && observation.read_context() == context.as_ref()
                    && observation.read_place() == &read.place_in_context(context, registry)
            })
        })
    {
        return None;
    }
    let mut normalisation = OperandNormalisation::AlreadyInteger;
    for observation in observations {
        if observation.expression() != &preparation.invocation
            || observation.operand() != &source
            || !read
                .context_alternatives()
                .iter()
                .any(|context| observation.read_context() == context.as_ref())
        {
            return None;
        }
        if matches!(
            prove_observed_integer_conversion(observation, amount, dialect, registry)?,
            OperandNormalisation::RetainOriginal
        ) {
            normalisation = OperandNormalisation::RetainOriginal;
        }
    }
    Some(normalisation)
}

fn prove_observed_integer_conversion(
    observation: &crate::command_binding::SourceReadStoreObservation,
    amount: i64,
    dialect: tcl_registry::InvocationDialect,
    registry: &CommandRegistry,
) -> Option<OperandNormalisation> {
    let before = observation.read_context();
    let place = observation.read_place();
    let setter = observation.setter_context();
    let output = observation.setter_place();
    if place != output
        || place.observed
        || place.dynamic
        || place.index.is_some()
        || place.kind != crate::place::PlaceKind::Scalar
        || place
            .cell
            .as_ref()
            .is_none_or(|cell| cell.generation == crate::place::CellGeneration::Unknown)
        || before.invocation_dialect != Some(dialect)
        || !before.read_produces_value(place, registry)
        || setter.store_would_error(output)
        || before.read_contents_origin(place, registry)
            != setter.read_contents_origin(output, registry)
    {
        return None;
    }
    if before.contents_native_numeric_category_at(place, registry)
        == Some(tcl_registry::TclType::Int)
        && dialect.arithmetic() == Some(tcl_dialect::NativeArithmetic::TclBignum)
    {
        return Some(OperandNormalisation::AlreadyInteger);
    }
    if before
        .contents_integer_increment_conversion_at(place, registry)
        .is_some()
    {
        return Some(OperandNormalisation::RetainOriginal);
    }
    let tcl_syntax::number::Number::Int(value) = tcl_syntax::number::parse_whole_with(
        before.literal_contents_at(place, registry)?,
        tcl_syntax::number::ParseFlags::for_syntax(dialect.numbers),
    )?
    else {
        return None;
    };
    value.checked_add(amount)?;
    (before.contents_native_numeric_category_at(place, registry)
        == Some(tcl_registry::TclType::Int))
    .then_some(OperandNormalisation::AlreadyInteger)
}

/// Require the actual prospective native store and normalization handlers in
/// the retained original lookup world. Proposed words never acquire original
/// variable reads or compiler receipts by coincident source offsets.
fn prospective_increment(
    script: &Script,
    tokens: &crate::ir::CommandTokens,
    expression: &crate::ir::CommandTokens,
    name: &str,
    schedule: &tcl_registry::runtime_expr_validation::NativeIncrementExpressionSchedule<'_>,
    normalisation: OperandNormalisation,
    registry: &CommandRegistry,
) -> Option<SourceIncrementRewrite> {
    use tcl_registry::runtime_expr_validation::NativeIncrementAmountConversion as Conversion;
    let binding = tokens.source_binding.as_ref()?;
    let dialect = binding.variable_context.invocation_dialect?;
    let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
    let increment_binding = binding.lookup_command_word("incr");
    let increment = increment_binding.proved_handler_target()?;
    if !increment_binding.unobserved_native_dispatch()
        || !increment.registry_backed
        || !increment.prepended.is_empty()
    {
        return None;
    }
    let conversion = schedule.conversion();
    let replacement = match conversion {
        Conversion::Unit(1) => format!("incr {name}"),
        Conversion::Unit(amount) => format!("incr {name} {amount}"),
        Conversion::OriginalLiteral(literal) => format!("incr {name} [expr {{{literal}}}]"),
        Conversion::NegatedOriginalLiteral(literal) => {
            format!("incr {name} [expr {{- [expr {{{literal}}}]}}]")
        }
    };
    if !matches!(conversion, Conversion::Unit(_))
        || matches!(normalisation, OperandNormalisation::RetainOriginal)
    {
        let expected = expression
            .source_binding
            .as_ref()?
            .proved_handler_target()?;
        let proposed_expression = binding.lookup_command_word("expr");
        if !proposed_expression.unobserved_native_dispatch()
            || proposed_expression.proved_handler_target()? != expected
        {
            return None;
        }
    }
    let segments =
        crate::segmenter::segment_commands_with_offset_and_config(&replacement, 0, config);
    let [segment] = segments.as_slice() else {
        return None;
    };
    let mut proposed = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(&replacement),
        config,
        segment,
    );
    proposed.source_binding = Some(increment_binding);
    let native = crate::registry_invocation::normal_transfer_invocation(registry, None, &proposed)?;
    native.numeric_store_production()?;
    let outputs = native.mutation_places(&binding.variable_context, registry);
    let original = crate::registry_invocation::normal_transfer_invocation(registry, None, tokens)?;
    if outputs != original.mutation_places(&binding.variable_context, registry)
        || !binding
            .sole_rhs_read_store_observations()?
            .iter()
            .all(|observation| {
                native
                    .mutation_places(observation.setter_context(), registry)
                    .as_slice()
                    == std::slice::from_ref(observation.setter_place())
            })
        || !script.is_authored_source()
    {
        return None;
    }
    let replacement = match normalisation {
        OperandNormalisation::AlreadyInteger => replacement,
        OperandNormalisation::RetainOriginal => {
            let source = retained_operand_normalisation(script, expression, schedule)?;
            format!("expr {{{source}}}; {replacement}")
        }
    };
    Some(SourceIncrementRewrite { replacement })
}

/// Validate the newly retained unary recipe under the original actual parser
/// and compiler preparation. Handler/observer dependencies are checked by the
/// prospective invocation before this helper can construct source.
fn retained_operand_normalisation(
    script: &Script,
    expression: &crate::ir::CommandTokens,
    schedule: &tcl_registry::runtime_expr_validation::NativeIncrementExpressionSchedule<'_>,
) -> Option<String> {
    let text = schedule.operand_normalisation()?;
    let source = format!("+{text}");
    let preparation = expression
        .source_binding
        .as_ref()?
        .expression_preparation(&script.expression_preparations)?;
    let tcl_registry::runtime_expr_validation::ExpressionPreparationProof::Prepared(_) =
        tcl_registry::runtime_expr_validation::prepare_expression_witness(
            &source,
            preparation.witness.context(),
            None,
        )
    else {
        return None;
    };
    Some(source)
}
