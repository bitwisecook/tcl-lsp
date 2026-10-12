// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original physical read/store and native conversion schedule for increments.

use crate::ir::{CommandTokens, Module, Script, Statement};
use crate::registry_invocation::InvocationMetadataContext;
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
    module: &Module,
) -> Option<SourceIncrementRewrite> {
    use crate::registry_invocation::{
        normal_transfer_invocation_with_metadata_context,
        resolved_tokens_invocation_with_metadata_context,
    };
    use tcl_registry::{SemanticOperationId, hooks::LoweringHookId};
    let tokens = script.retained_source_tokens_for_statement(statement)?;
    let metadata = original_increment_metadata(tokens, module, registry)?;
    let setter = resolved_tokens_invocation_with_metadata_context(registry, metadata, tokens)?;
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
    let config = binding.original_lexer_config_for_tokens(tokens)?;
    let transfer = normal_transfer_invocation_with_metadata_context(registry, metadata, tokens)?;
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
    let expression_metadata = original_increment_metadata(&expression, module, registry)?;
    let selected = resolved_tokens_invocation_with_metadata_context(
        registry,
        expression_metadata,
        &expression,
    )?;
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
        SelectionContext {
            registry,
            metadata,
            config,
        },
    )
}

struct SelectionContext<'a> {
    registry: &'a CommandRegistry,
    metadata: Option<InvocationMetadataContext<'a>>,
    config: tcl_lexer::LexerConfig,
}

/// The actual CU and original whole-vector owner retain availability and full
/// syntax independently of the native read/store proof. Only positively tagged
/// standalone modules can select their explicit compatibility metadata.
fn original_increment_metadata<'a>(
    tokens: &'a CommandTokens,
    module: &'a Module,
    registry: &CommandRegistry,
) -> Option<Option<InvocationMetadataContext<'a>>> {
    let owner = module.retained_source_bindings.as_deref()?;
    if !owner.matches_module(module, registry) {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let config = binding.original_lexer_config_for_tokens(tokens)?;
    if config.nested().normalized() != module.lexer_config.nested().normalized() {
        return None;
    }
    if module.source_entry.metadata_context.is_standalone()
        && module.source_metadata_input.is_none()
    {
        if !owner.owns_original_tokens(tokens) {
            return None;
        }
        return module
            .source_entry
            .metadata_context
            .metadata_context(registry);
    }
    Some(Some(binding.original_invocation_metadata_for_module(
        tokens, module, registry,
    )?))
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
    selection: SelectionContext<'_>,
) -> Option<SourceIncrementRewrite> {
    use tcl_registry::runtime_expr_validation::NativeIncrementAmountConversion as Conversion;
    let SelectionContext {
        registry,
        metadata,
        config,
    } = selection;
    let binding = tokens.source_binding.as_ref()?;
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
    let native = crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
        registry, metadata, &proposed,
    )?;
    native.numeric_store_production()?;
    let outputs = native.mutation_places(&binding.variable_context, registry);
    let original = crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
        registry, metadata, tokens,
    )?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};
    use std::sync::Arc;
    use tcl_lexer::LexerConfig;
    use tcl_registry::model::{ContextRegistry, ingress};

    fn logical_unit(
        source: &str,
        context: &Arc<ContextRegistry>,
        config: LexerConfig,
    ) -> CompilationUnit {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(context),
            config,
        );
        CompilationUnit::build_with_analysis_input(
            source,
            UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            &input,
        )
    }

    #[test]
    fn original_increment_metadata_keeps_actual_configuration_and_available_generation() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let current = ingress::resolve_environment("tcl9.1").default_context_registry();
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = LexerConfig {
            strict_quoting: true,
            expand_syntax: false,
            ..LexerConfig::for_file_grammar(profile.grammar)
        };
        let source = "set x [expr {$x+1}]";
        let unit = logical_unit(source, &current, config);
        let module = &unit.ir_module;
        let script = &module.top_level;
        let statement = script.statements.last().unwrap();
        let tokens = script
            .retained_source_tokens_for_statement(statement)
            .unwrap();
        let metadata = original_increment_metadata(tokens, module, current.commands())
            .unwrap()
            .unwrap();
        assert!(std::ptr::eq(metadata.context(), current.context()));
        assert_eq!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .original_lexer_config_for_tokens(tokens)
                .unwrap(),
            config
        );
        assert!(crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
            current.commands(), metadata, tokens,
        ).is_some());
        assert!(
            assess_increment_rewrite(script, statement, current.commands(), module).is_none(),
            "source selection supplies no Native read/store or integer conversion witness"
        );
        let older = Arc::new(
            ingress::resolve_environment("tcl8.4")
                .default_context_registry()
                .with_command_store(current.commands().snapshot().shared_registry()),
        );
        for (context, available) in [(&current, true), (&older, false)] {
            let unit = logical_unit(source, context, config);
            let script = &unit.ir_module.top_level;
            let tokens = script
                .retained_source_tokens_for_statement(script.statements.last().unwrap())
                .unwrap();
            let metadata = original_increment_metadata(tokens, &unit.ir_module, context.commands())
                .unwrap()
                .unwrap();
            assert_eq!(
                metadata
                    .context()
                    .resolve_spec_in_realm(
                        context.commands(),
                        "throw",
                        tcl_dialect::model::InvocationRealm::InterpreterRuntime
                    )
                    .is_some(),
                available
            );
        }
        let word = tokens.words().last().unwrap();
        let expression =
            crate::value_shapes::command_substitution_tokens(word, Some(tokens), config)
                .unwrap()
                .remove(0);
        let child = original_increment_metadata(&expression, module, current.commands())
            .unwrap()
            .unwrap();
        assert!(std::ptr::eq(child.context(), current.context()));
        assert_eq!(
            expression
                .source_binding
                .as_ref()
                .unwrap()
                .original_lexer_config_for_tokens(&expression)
                .unwrap()
                .nested()
                .normalized(),
            config.nested().normalized()
        );
    }

    #[test]
    fn original_increment_metadata_withdraws_supplied_owners_without_standalone_fallback() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let current = ingress::resolve_environment("tcl9.1").default_context_registry();
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = LexerConfig::for_file_grammar(profile.grammar);
        let unit = logical_unit("set x [expr {$x+1}]", &current, config);
        let script = &unit.ir_module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(script.statements.last().unwrap())
            .unwrap();
        assert!(original_increment_metadata(tokens, &unit.ir_module, current.commands()).is_some());
        for change in 0..5 {
            let mut module = unit.ir_module.clone();
            match change {
                0 => module.source_metadata_input = None,
                1 => module.lexer_config.strict_quoting = !config.strict_quoting,
                2 => module.retained_source_bindings = None,
                3 => {
                    module.source_entry.metadata_context =
                        crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable
                }
                _ => module.source = tcl_lexer::SourceImage::document("set x OTHER"),
            }
            assert!(original_increment_metadata(tokens, &module, current.commands()).is_none());
        }
        let foreign = CommandRegistry::build_default();
        assert!(original_increment_metadata(tokens, &unit.ir_module, &foreign).is_none());
        let mut missing = tokens.clone();
        missing.source_binding = None;
        assert!(
            original_increment_metadata(&missing, &unit.ir_module, current.commands()).is_none()
        );
        let registry = CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        let standalone = CompilationUnit::build_for("set x 0", &registry, false);
        assert!(
            standalone
                .ir_module
                .source_entry
                .metadata_context
                .is_standalone()
        );
        let script = &standalone.ir_module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(script.statements.last().unwrap())
            .unwrap();
        assert!(original_increment_metadata(tokens, &standalone.ir_module, &registry).is_some());
    }
}
