// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Store the actual frozen RHS representation before any write observer.

use super::{
    Arc, ModuleCommandBindings, SourceCommandTarget, SourceExecutionContext, SourceOutcomes,
};
use crate::{place::Place, registry_invocation::EffectiveInvocationWord};
use tcl_syntax::value::ValueRepresentation;

/// Representation of a frozen value, independently of whether its bytes are
/// known. A later possible shared-object coercion withdraws the receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FrozenSourceRepresentation {
    representation: ValueRepresentation,
    list_method_provider: Option<tcl_registry::native_result::NativeListMethodProvider>,
    numeric_shape: Option<crate::native_numeric::SourceNativeNumericShape>,
    stock_literal_object: Option<super::SourceStockLiteralObject>,
    validity: SourceRepresentationValidity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceRepresentationValidity {
    Frozen(u64),
    ImmediateCreated,
}

impl FrozenSourceRepresentation {
    pub(super) fn numeric_input_is_closed(
        self,
        variables: &crate::var_resolve::ResolveContext,
    ) -> bool {
        (self.numeric_shape.is_some() || self.stock_literal_object.is_some())
            && (self.is_current(variables) || self.is_immediate_created_result())
    }
    pub(super) fn capture_stock_literal(
        object: super::SourceStockLiteralObject,
        variables: &crate::var_resolve::ResolveContext,
    ) -> Option<Self> {
        Some(Self {
            representation: object.representation(),
            list_method_provider: None,
            numeric_shape: None,
            stock_literal_object: Some(object),
            validity: SourceRepresentationValidity::Frozen(variables.representation_epoch?),
        })
    }
    pub(super) fn capture(
        representation: ValueRepresentation,
        variables: &crate::var_resolve::ResolveContext,
    ) -> Option<Self> {
        matches!(
            representation,
            ValueRepresentation::List | ValueRepresentation::Dict
        )
        .then_some(())?;
        Some(Self {
            representation,
            list_method_provider: None,
            numeric_shape: None,
            stock_literal_object: None,
            validity: SourceRepresentationValidity::Frozen(variables.representation_epoch?),
        })
    }

    pub(super) fn callback_input_is_closed_for(
        self,
        variables: &crate::var_resolve::ResolveContext,
        method: tcl_registry::list_object_methods::NativeListMethod,
    ) -> bool {
        (self.is_current(variables) || self.is_immediate_created_result())
            && (self.stock_literal_object.is_some()
                || self.numeric_shape.is_some()
                || (method != tcl_registry::list_object_methods::NativeListMethod::StringAccess
                    && matches!(
                        self.representation,
                        ValueRepresentation::List | ValueRepresentation::Dict
                    ))
                || self
                    .list_method_provider
                    .is_some_and(|provider| provider.world_is_closed_for(method)))
    }

    pub(super) fn preserves_list_length_representation(self) -> bool {
        self.list_method_provider
            .is_some_and(tcl_registry::native_result::NativeListMethodProvider::length_is_read_only)
    }

    fn capture_list_method_provider(
        provider: tcl_registry::native_result::NativeListMethodProvider,
        variables: &crate::var_resolve::ResolveContext,
    ) -> Option<Self> {
        Some(Self {
            representation: ValueRepresentation::Unknown,
            list_method_provider: Some(provider),
            numeric_shape: None,
            stock_literal_object: None,
            validity: SourceRepresentationValidity::Frozen(variables.representation_epoch?),
        })
    }

    pub(super) fn is_current(self, variables: &crate::var_resolve::ResolveContext) -> bool {
        matches!(self.validity, SourceRepresentationValidity::Frozen(epoch)
            if variables.representation_epoch == Some(epoch))
    }

    pub(super) fn is_immediate_created_result(self) -> bool {
        self.validity == SourceRepresentationValidity::ImmediateCreated
    }

    fn stored_representation(self) -> crate::native_numeric::StoredNativeRepresentation {
        if let Some(shape) = self.numeric_shape {
            crate::native_numeric::StoredNativeRepresentation::NumericShape(shape)
        } else if let Some(provider) = self.list_method_provider {
            crate::native_numeric::StoredNativeRepresentation::ReadOnlyListMethodProvider(provider)
        } else if let Some(object) = self.stock_literal_object {
            crate::native_numeric::StoredNativeRepresentation::StockLiteralObject(object)
        } else {
            self.representation.into()
        }
    }
}

/// Current normalized intrep of the actually completed prepared operator.
/// Its unknown bytes do not supply a number or a native operand receipt.
pub(super) fn retain_expression_numeric_result(
    outcomes: &mut SourceOutcomes,
    shape: crate::native_numeric::SourceNativeNumericShape,
) {
    let Some(normal) = &outcomes.normal else {
        return;
    };
    outcomes.normal_representation = Some(Arc::new(FrozenSourceRepresentation {
        representation: ValueRepresentation::Unknown,
        list_method_provider: None,
        numeric_shape: Some(shape),
        stock_literal_object: None,
        validity: normal.source_variables.representation_epoch.map_or(
            SourceRepresentationValidity::ImmediateCreated,
            SourceRepresentationValidity::Frozen,
        ),
    }));
}

pub(super) fn scalar_math_result(
    state: &mut ModuleCommandBindings,
    protocol: tcl_registry::mathfunc::NativeScalarMathProtocol,
    input_closed: bool,
) -> SourceOutcomes {
    if !input_closed {
        // GetDouble can call an arbitrary operand object's string updater.
        // The stock result object remains independently numeric on OK.
        state.mark_opaque_binding_mutation();
    }
    Arc::make_mut(&mut state.source_variables)
        .invalidate_math_numeric_operand_representations(protocol.numeric_operand_policy());
    let mut outcomes = SourceOutcomes::invocation(state, protocol.completion_route());
    if let Some(dialect) = state.baseline.dialect {
        retain_expression_numeric_result(
            &mut outcomes,
            crate::native_numeric::SourceNativeNumericShape::from_result(
                dialect,
                protocol.result_production(),
            ),
        );
    }
    outcomes
}

/// Frozen original operands of an implicit expression math invocation.
#[derive(Clone, Copy)]
pub(super) struct SourceMathInput {
    pub(super) arity: usize,
    pub(super) closed: bool,
}

pub(super) fn scalar_math_input_is_closed(
    expression: &crate::expr_ast::ExprNode,
    outcomes: &SourceOutcomes,
) -> bool {
    let Some(state) = outcomes.normal.as_deref() else {
        return false;
    };
    outcomes
        .normal_representation
        .as_ref()
        .is_some_and(|receipt| receipt.numeric_input_is_closed(&state.source_variables))
        || outcomes
            .normal_value
            .as_ref()
            .and_then(|value| value.numeric.as_ref())
            .is_some_and(|receipt| {
                state.source_variables.representation_epoch == Some(receipt.epoch)
            })
        || super::literal_object_pool::SourceOrdinaryLiteralObject::capture_expression_literal(
            expression, state,
        )
        .is_some()
}

pub(super) fn unobserved_read_result(
    state: &ModuleCommandBindings,
    access: &Place,
    registry: &tcl_registry::CommandRegistry,
) -> SourceOutcomes {
    if state
        .source_variables
        .read_is_definitely_missing(access, registry)
    {
        return SourceOutcomes::invocation(
            state,
            tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                tcl_registry::completion::CompletionCode::Error,
            ),
        );
    }
    let mut outcomes = if let Some(value) =
        state.source_variables.literal_contents_at(access, registry)
    {
        let mut normal = SourceOutcomes::normal(state);
        normal.normal_value = Some(Arc::new(super::native_result::EvaluatedSourceValue {
            text: value.to_owned(),
            representation: state.source_variables.contents_representation_at(access),
            numeric: state
                .source_variables
                .contents_native_numeric_at(access, registry)
                .zip(state.source_variables.representation_epoch)
                .map(
                    |(object, epoch)| crate::native_numeric::FrozenSourceNumeric { object, epoch },
                ),
        }));
        normal
    } else if state.source_variables.read_produces_value(access, registry) {
        SourceOutcomes::normal(state)
    } else {
        SourceOutcomes::invocation(
            state,
            tcl_registry::completion_route::InvocationCompletionRoute::Unknown,
        )
    };
    if state.source_variables.read_produces_value(access, registry) {
        outcomes.normal_method_prefix = super::method_prefix::read_prefix(state, access, registry);
        outcomes.normal_representation = state
            .source_variables
            .contents_stock_literal_object_at(access, registry)
            .and_then(|object| {
                FrozenSourceRepresentation::capture_stock_literal(object, &state.source_variables)
            })
            .or_else(|| {
                FrozenSourceRepresentation::capture(
                    state.source_variables.contents_representation_at(access),
                    &state.source_variables,
                )
            })
            .or_else(|| {
                FrozenSourceRepresentation::capture_list_method_provider(
                    state
                        .source_variables
                        .contents_list_method_provider_at(access, registry)?,
                    &state.source_variables,
                )
            })
            .or_else(|| {
                Some(FrozenSourceRepresentation {
                    representation: ValueRepresentation::Unknown,
                    list_method_provider: None,
                    stock_literal_object: None,
                    numeric_shape: Some(
                        state
                            .source_variables
                            .contents_native_numeric_shape_at(access, registry)?,
                    ),
                    validity: SourceRepresentationValidity::Frozen(
                        state.source_variables.representation_epoch?,
                    ),
                })
            })
            .map(Arc::new);
    }
    // A scalar read with closed storage/materialisation cannot fail before
    // yielding its original value. Indexed reads retain their own index and
    // container obligations and do not obtain this certificate.
    if access.kind == crate::place::PlaceKind::Scalar
        && access.index.is_none()
        && state.source_variables.read_produces_value(access, registry)
        && state
            .source_variables
            .contents_integer_increment_conversion_at(access, registry)
            .is_some()
    {
        outcomes.retain_complete_normal_evaluation();
    }
    outcomes
}

/// Capture a conditional range shape before the handler changes representation
/// epochs. Original frozen object evidence must survive all later argv effects.
pub(super) struct CapturedRangeResult {
    representation: ValueRepresentation,
    value: Option<String>,
}

impl CapturedRangeResult {
    pub(super) const fn representation(&self) -> ValueRepresentation {
        self.representation
    }
}

pub(super) fn capture_range_result(
    native: super::SourceNativeInvocation<'_>,
    facts: &tcl_registry::InvocationFacts,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<Box<CapturedRangeResult>> {
    use tcl_registry::{
        native_result::NativeResultSelection, representation::OrdinaryContainerRepresentation,
    };
    super::native_result::numeric_call_arguments(context, native.target)?;
    let arguments = native.invocation.arguments();
    let selection = facts
        .native_result?
        .select(arguments, facts.argument_offset);
    let NativeResultSelection::ListRange { list_at, .. } = selection else {
        return None;
    };
    let representation = context
        .written_representations?
        .get(list_at.checked_add(1)?)?
        .as_ref()
        .filter(|receipt| receipt.is_current(&state.source_variables))
        .map(|receipt| receipt.representation)
        .or_else(|| {
            if !super::object_callbacks::original_operand_is_current(native, list_at, context) {
                return None;
            }
            let variables = &state.source_variables;
            let place =
                super::argument_reads::current_argument_place(native, list_at, state, context)?;
            variables
                .read_produces_value(place, context.registry)
                .then(|| variables.contents_representation_at(place))
        })?;
    let ordinary = match representation {
        ValueRepresentation::List => OrdinaryContainerRepresentation::List,
        ValueRepresentation::Dict => OrdinaryContainerRepresentation::Dictionary,
        _ => return None,
    };
    let dialect = arguments.dialect()?;
    let rules = tcl_syntax::word_rules::WordValueRules::from_grammar(&dialect.lexer_grammar);
    let length = tcl_syntax::list::split_list_bytes_in(
        arguments.literal_at(list_at)?.as_bytes(),
        rules.list,
        dialect.lexer_grammar.escapes,
    )
    .ok()?
    .len();
    Some(Box::new(CapturedRangeResult {
        representation: selection
            .ordinary_list_range_representation(arguments, ordinary, length)?,
        value: selection.ordinary_range_literal_result(arguments, ordinary, length),
    }))
}

/// Publish independently captured bytes only on the native normal result.
/// Unsupported serialization keeps the separate range-shape receipt.
pub(super) fn retain_range_result_bytes(
    outcomes: &mut SourceOutcomes,
    captured: Option<Box<CapturedRangeResult>>,
) {
    if outcomes.normal.is_none() {
        return;
    }
    let Some(captured) = captured else { return };
    let Some(text) = captured.value else { return };
    outcomes.normal_value = Some(Arc::new(super::native_result::EvaluatedSourceValue {
        text,
        representation: captured.representation,
        numeric: None,
    }));
}

/// A normal native result contract survives opaque operand effects. Its
/// result receipt grants no preservation of the mutable producer world.
pub(super) fn opaque_result_producer(
    native: super::SourceNativeInvocation<'_>,
    facts: &tcl_registry::InvocationFacts,
    state: &mut super::ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<SourceOutcomes> {
    let arguments = native.invocation.arguments();
    if facts.normal_list_method_provider(arguments).is_none()
        || facts
            .successful_handler_effects(arguments, state.source_variables.alias_frame())
            .is_some()
    {
        return None;
    }
    state.mark_opaque_binding_mutation();
    let mut outcomes = SourceOutcomes::invocation(
        state,
        tcl_registry::completion_route::InvocationCompletionRoute::Unknown,
    );
    retain_created_result(&mut outcomes, native, facts, context, None);
    Some(outcomes)
}

/// Selected native constructors establish their result representation. The
/// original List compiler operands distinguish construction from literal reuse.
pub(super) fn retain_created_result(
    outcomes: &mut SourceOutcomes,
    native: super::SourceNativeInvocation<'_>,
    facts: &tcl_registry::InvocationFacts,
    context: SourceExecutionContext<'_>,
    conditional: Option<ValueRepresentation>,
) {
    if outcomes.normal.is_none() {
        return;
    }
    let Some(contract) = facts.native_result else {
        return;
    };
    let Some(dialect) = native.invocation.arguments().dialect() else {
        return;
    };
    let selection = contract.select(native.invocation.arguments(), facts.argument_offset);
    let shapes = if native.target.prepended.is_empty() {
        native
            .words
            .iter()
            .skip(1)
            .map(super::compiled_invocation::word_shape)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let representation = if selection.list_construction(
        dialect,
        context.compilation,
        *native.compilation_selection,
        &shapes,
    ) == tcl_registry::native_result::ListResultConstruction::Fresh
    {
        ValueRepresentation::List
    } else {
        conditional
            .unwrap_or_else(|| selection.created_representation(dialect, context.compilation))
    };
    let list_method_provider = facts
        .normal_list_method_provider(native.invocation.arguments())
        .or_else(|| {
            (representation == ValueRepresentation::Unknown)
                .then(|| {
                    facts.normal_empty_list_root_provider(
                        native.invocation.arguments(),
                        *native.compilation_selection,
                    )
                })
                .flatten()
        });
    let numeric_shape = facts
        .normal_numeric_result_production(native.invocation.arguments())
        .map(|production| {
            crate::native_numeric::SourceNativeNumericShape::from_result(dialect, production)
        });
    if matches!(
        representation,
        ValueRepresentation::List | ValueRepresentation::Dict
    ) || list_method_provider.is_some()
        || numeric_shape.is_some()
    {
        let normal = outcomes.normal.as_mut().unwrap();
        if normal.source_variables.representation_epoch.is_none() {
            Arc::make_mut(&mut normal.source_variables).establish_created_representation_epoch();
        }
        outcomes.normal_representation = Some(Arc::new(FrozenSourceRepresentation {
            representation,
            list_method_provider,
            numeric_shape,
            stock_literal_object: None,
            validity: normal.source_variables.representation_epoch.map_or(
                SourceRepresentationValidity::ImmediateCreated,
                SourceRepresentationValidity::Frozen,
            ),
        }));
    }
}

pub(super) fn retain_store_representation(
    outcomes: &mut SourceOutcomes,
    facts: &tcl_registry::InvocationFacts,
    target: &SourceCommandTarget,
    arguments: tcl_registry::InvocationArguments<'_>,
    context: SourceExecutionContext<'_>,
) {
    let Some(state) = outcomes.normal.as_mut() else {
        return;
    };
    let Some(name) = arguments.literal_at(facts.argument_offset) else {
        return;
    };
    let receiver = crate::var_resolve::resolve_literal_access(
        name,
        &state.source_variables,
        false,
        context.registry,
        tcl_registry::TraceOperation::Write,
    );
    // An observed store publishes before its callbacks in the captured owner.
    // Installing afterwards would resurrect a representation they changed.
    if receiver.observed {
        return;
    }
    publish_numeric_store_representation(
        state,
        facts,
        arguments,
        &receiver,
        context.invocation_offset,
    );
    publish_store_representation(state, facts, target, arguments, context, &receiver);
}

pub(super) fn publish_store_representation(
    state: &mut ModuleCommandBindings,
    facts: &tcl_registry::InvocationFacts,
    target: &SourceCommandTarget,
    arguments: tcl_registry::InvocationArguments<'_>,
    context: SourceExecutionContext<'_>,
    receiver: &Place,
) {
    let Some(value) = frozen_store_representation(facts, target, arguments, context) else {
        return;
    };
    let variables = &state.source_variables;
    if (!value.is_immediate_created_result() && !value.is_current(variables))
        || receiver.dynamic
        || receiver.observed
        || variables.dynamic_bindings
        || variables.contents_presence(receiver) != crate::var_resolve::ContentsPresence::Defined
        || variables.store_would_error(receiver)
        || variables.contents_origin(receiver)
            != crate::var_resolve::ContentsOrigin::WrittenAt(context.invocation_offset)
        || state
            .current_source_origin
            .as_ref()
            .is_none_or(|source| !variables.contents_have_source(receiver, source))
        || context
            .written_values
            .and_then(|values| values.get(2))
            .and_then(Option::as_ref)
            .is_some_and(|known| {
                variables.literal_contents_at(receiver, context.registry)
                    != Some(known.text.as_str())
            })
    {
        return;
    }
    if let Some(key) = crate::var_resolve::canonical_binding_value_key(receiver) {
        Arc::make_mut(&mut state.source_variables)
            .value_representations
            .insert(key, value.stored_representation());
    }
}

pub(super) fn publish_numeric_store_representation(
    state: &mut ModuleCommandBindings,
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    receiver: &Place,
    source_offset: u32,
) {
    let Some(production) = facts.native_result.and_then(|contract| {
        contract.normal_numeric_store_production(arguments, facts.argument_offset)
    }) else {
        return;
    };
    let Some(dialect) = arguments.dialect() else {
        return;
    };
    let variables = &state.source_variables;
    if receiver.dynamic
        || receiver.observed
        || variables.contents_presence(receiver) != crate::var_resolve::ContentsPresence::Defined
        || variables.contents_origin(receiver)
            != crate::var_resolve::ContentsOrigin::WrittenAt(source_offset)
        || state
            .current_source_origin
            .as_ref()
            .is_none_or(|origin| !variables.contents_have_source(receiver, origin))
    {
        return;
    }
    if let Some(key) = crate::var_resolve::canonical_binding_value_key(receiver) {
        Arc::make_mut(&mut state.source_variables)
            .value_representations
            .insert(
                key,
                crate::native_numeric::StoredNativeRepresentation::NumericShape(
                    crate::native_numeric::SourceNativeNumericShape::from_selected(
                        dialect, production,
                    ),
                ),
            );
    }
}

/// Transfer a frozen actual object only through an ordinary Value formal's
/// current activation slot. Rest/default/reference and static binding remain
/// independent protocols.
pub(super) fn retain_value_formals(
    variables: &mut crate::var_resolve::ResolveContext,
    parameters: &[tcl_syntax::formal_params::FormalParameter],
    actual_count: usize,
    first_actual_word: usize,
    target: &SourceCommandTarget,
    context: SourceExecutionContext<'_>,
) {
    if !target.prepended.is_empty()
        || context.written_arguments.is_none_or(|words| {
            words.iter().any(|word| {
                matches!(
                    word,
                    EffectiveInvocationWord::Expanded | EffectiveInvocationWord::KnownExpansion(_)
                )
            })
        })
    {
        return;
    }
    let Some(receipts) = context
        .written_representations
        .and_then(|values| values.get(first_actual_word..))
    else {
        return;
    };
    if receipts.len() != actual_count {
        return;
    }
    let Some(grammar) = variables
        .invocation_dialect
        .and_then(tcl_registry::InvocationDialect::parameter_grammar)
    else {
        return;
    };
    let Ok(plan) =
        tcl_syntax::formal_params::bind_formal_arguments(parameters, actual_count, grammar)
    else {
        return;
    };
    for binding in plan {
        let tcl_syntax::formal_params::FormalArgumentBinding::Value {
            parameter,
            argument,
        } = binding
        else {
            continue;
        };
        let Some(receipt) = receipts[argument] else {
            continue;
        };
        if !receipt.is_current(variables) && !receipt.is_immediate_created_result() {
            continue;
        }
        let slot = crate::var_resolve::resolve_literal_access(
            &parameters[parameter].name,
            variables,
            false,
            context.registry,
            tcl_registry::TraceOperation::Read,
        );
        if slot.kind != crate::place::PlaceKind::Scalar
            || slot.index.is_some()
            || !variables.read_produces_value(&slot, context.registry)
            || !matches!(slot.cell.as_ref().map(|cell| &cell.owner),
                Some(crate::place::CellOwner::Activation(owner)) if variables.activation.as_ref() == Some(owner))
            || variables.contents_origin(&slot) != crate::var_resolve::ContentsOrigin::Incoming
        {
            continue;
        }
        if let Some(key) = crate::var_resolve::canonical_binding_value_key(&slot) {
            variables
                .value_representations
                .insert(key, receipt.stored_representation());
        }
    }
}

fn frozen_store_representation(
    facts: &tcl_registry::InvocationFacts,
    target: &SourceCommandTarget,
    arguments: tcl_registry::InvocationArguments<'_>,
    context: SourceExecutionContext<'_>,
) -> Option<FrozenSourceRepresentation> {
    if facts.operation
        != tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::Set,
        )
        || !target.prepended.is_empty()
        || arguments.exact_argv_len() != Some(2)
        || context.written_arguments.is_none_or(|words| {
            words.iter().any(|word| {
                matches!(
                    word,
                    EffectiveInvocationWord::Expanded | EffectiveInvocationWord::KnownExpansion(_)
                )
            })
        })
    {
        return None;
    }
    context.written_representations?.get(2).copied().flatten()
}

#[cfg(test)]
mod tests {
    use crate::command_binding::{SourceAnalysisOptions, SourceCommandBindings};
    use tcl_syntax::value::ValueRepresentation;

    fn analyse(source: &str) -> SourceCommandBindings {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
            SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
    }

    #[test]
    fn a_proved_unknown_byte_read_has_only_a_normal_substitution_route() {
        let source = "proc f {value} {set copy $value}";
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let bindings = analyse(source);
        let point = bindings.invocation_at_source(
            "set",
            u32::try_from(source.rfind("set copy").unwrap()).unwrap(),
        );
        let state = super::super::ModuleCommandBindings {
            source_variables: super::Arc::clone(&point.variable_context),
            ..Default::default()
        };
        let target = crate::var_resolve::resolve_literal_access(
            "value",
            &state.source_variables,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        assert!(
            state
                .source_variables
                .read_produces_value(&target, registry)
        );
        let outcomes = super::unobserved_read_result(&state, &target, registry);
        assert!(outcomes.normal.is_some());
        assert!(outcomes.abrupt.is_empty());
        assert!(outcomes.normal_value.is_none());
    }

    #[test]
    fn a_range_after_literal_indices_retains_its_actual_normal_list_result() {
        let source = "proc f {lst} {set constructor list; set lst [$constructor 7]; set x [lrange $lst 0 1]; expr {$x} + 1}";
        for dialect in ["tcl8.6", "tcl9.1"] {
            let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
            let profile = registry.profile().unwrap();
            let bindings = SourceCommandBindings::analyse_with_options(
                source,
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                registry,
                SourceAnalysisOptions {
                    invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                    native_compilation:
                        tcl_registry::native_compilation::NativeCompilationContext {
                            mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                            ..Default::default()
                        },
                    ..Default::default()
                },
            );
            let point = bindings.invocation_at_source(
                "expr",
                u32::try_from(source.rfind("expr").unwrap()).unwrap(),
            );
            let target = crate::var_resolve::resolve_literal_access(
                "x",
                &point.variable_context,
                false,
                registry,
                tcl_registry::TraceOperation::Read,
            );
            assert_eq!(
                point.variable_context.contents_representation_at(&target),
                ValueRepresentation::List,
                "{dialect}: {target:?}; range input={:?}",
                {
                    let range = bindings.invocation_at_source(
                        "lrange",
                        u32::try_from(source.find("lrange").unwrap()).unwrap(),
                    );
                    let list = crate::var_resolve::resolve_literal_access(
                        "lst",
                        &range.variable_context,
                        false,
                        registry,
                        tcl_registry::TraceOperation::Read,
                    );
                    (
                        range
                            .variable_context
                            .literal_contents_at(&list, registry)
                            .map(str::to_owned),
                        range.variable_context.contents_representation_at(&list),
                        range.variable_context.representation_epoch,
                        range.native_compilation_selection(),
                    )
                }
            );
        }
    }

    #[test]
    fn scalar_math_results_preserve_only_authenticated_numeric_categories() {
        for profile in ["tcl8.6", "tcl9.1"] {
            let environment = tcl_registry::model::ingress::static_context_for(profile);
            let registry = environment.commands();
            for (source, expected) in [
                (
                    "proc f {raw} {set raw [llength [list $raw]]; set d [expr {sqrt($raw)}]; set view $d}",
                    Some(tcl_registry::TclType::Double),
                ),
                (
                    "proc f {raw} {set d [expr {sqrt($raw)}]; set view $d}",
                    None,
                ),
                (
                    "proc f {} {set d [expr {sqrt(9)}]; set view $d}",
                    Some(tcl_registry::TclType::Double),
                ),
                (
                    "proc ::tcl::mathfunc::sqrt raw {return CHANGED}; proc f {raw} {set d [expr {sqrt($raw)}]; set view $d}",
                    None,
                ),
                (
                    "proc change {name key op} {upvar 1 $name receiver; set receiver CHANGED}; proc f {raw} {trace add variable d write change; set d [expr {sqrt(9)}]; set view $d}",
                    None,
                ),
            ] {
                let bindings = SourceCommandBindings::analyse_with_options(
                    source,
                    tcl_lexer::LexerConfig::for_profile(registry.profile()),
                    registry,
                    SourceAnalysisOptions {
                        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(
                            registry.profile().unwrap(),
                        )),
                        native_compilation:
                            crate::environment_ingress::authoring_native_compilation(),
                        ..SourceAnalysisOptions::default()
                    },
                );
                let offset = u32::try_from(source.rfind("$d").unwrap()).unwrap();
                let reads =
                    bindings.variable_accesses_in_span(tcl_lexer::Span::new(offset, offset + 2));
                assert_eq!(reads.len(), 1, "{profile}: {source}");
                for context in reads[0].context_alternatives() {
                    let place = reads[0].place_in_context(context, registry);
                    assert_eq!(
                        context.contents_native_numeric_category_at(&place, registry),
                        expected,
                        "{profile}: {source}"
                    );
                }
            }
        }
    }

    #[test]
    fn jim_math_weakens_only_the_actual_integer_operand_category() {
        let environment = tcl_registry::model::ingress::static_context_for("jim");
        let registry = environment.commands();
        let source = "proc f {raw} {incr raw; set d [expr {sqrt($raw)}]; set integer_view $raw; set double_view $d}";
        let entry = crate::environment_ingress::captured_native_entry(registry.profile().unwrap());
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::for_profile(registry.profile()),
            registry,
            SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(
                    registry.profile().unwrap(),
                )),
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                native_entry: Some(&entry),
                ..SourceAnalysisOptions::default()
            },
        );
        for (spelling, category) in [
            ("$raw", tcl_registry::TclType::Numeric),
            ("$d", tcl_registry::TclType::Double),
        ] {
            let offset = u32::try_from(source.rfind(spelling).unwrap()).unwrap();
            let reads = bindings.variable_accesses_in_span(tcl_lexer::Span::new(
                offset,
                offset + u32::try_from(spelling.len()).unwrap(),
            ));
            assert_eq!(reads.len(), 1);
            for context in reads[0].context_alternatives() {
                let place = reads[0].place_in_context(context, registry);
                assert_eq!(
                    context.contents_native_numeric_category_at(&place, registry),
                    Some(category),
                    "{spelling}"
                );
                assert!(
                    context
                        .contents_native_numeric_at(&place, registry)
                        .is_none()
                );
            }
        }
    }

    #[test]
    fn jim_math_without_a_retained_table_cannot_publish_a_category() {
        let registry = tcl_registry::model::ingress::static_context_for("jim").commands();
        let source = "proc f {raw} {incr raw; set d [expr {sqrt($raw)}]; set view $d}";
        let mut entry =
            crate::environment_ingress::captured_native_entry(registry.profile().unwrap());
        entry.math_functions = None;
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::for_profile(registry.profile()),
            registry,
            SourceAnalysisOptions {
                native_entry: Some(&entry),
                invocation_dialect: registry
                    .profile()
                    .map(tcl_registry::InvocationDialect::of_profile),
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..SourceAnalysisOptions::default()
            },
        );
        let offset = u32::try_from(source.rfind("$d").unwrap()).unwrap();
        let reads = bindings.variable_accesses_in_span(tcl_lexer::Span::new(offset, offset + 2));
        assert_eq!(reads.len(), 1);
        for context in reads[0].context_alternatives() {
            let place = reads[0].place_in_context(context, registry);
            assert_eq!(
                context.contents_native_numeric_category_at(&place, registry),
                None
            );
        }
    }

    #[test]
    fn scalar_math_operand_conversion_preserves_c_numeric_category() {
        let source = "proc f {raw} {incr raw; set d [expr {sqrt($raw)}]; set view $raw}";
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let bindings = analyse(source);
        let offset = u32::try_from(source.rfind("$raw").unwrap()).unwrap();
        let reads = bindings.variable_accesses_in_span(tcl_lexer::Span::new(offset, offset + 4));
        assert_eq!(reads.len(), 1);
        for context in reads[0].context_alternatives() {
            let place = reads[0].place_in_context(context, registry);
            assert_eq!(
                context.contents_native_numeric_category_at(&place, registry),
                Some(tcl_registry::TclType::Int)
            );
            let mut converted = context.as_ref().clone();
            converted.invalidate_math_numeric_operand_representations(
                tcl_registry::mathfunc::NativeMathNumericOperandPolicy::WeakenToNumeric,
            );
            assert_eq!(
                converted.contents_native_numeric_category_at(&place, registry),
                Some(tcl_registry::TclType::Numeric)
            );
            assert!(
                converted
                    .contents_native_numeric_at(&place, registry)
                    .is_none()
            );
        }
    }

    #[test]
    fn normal_procedure_results_preserve_only_current_representation_receipts() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        for (source, expected) in [
            (
                "proc make {raw} {list k $raw}; proc f {raw} {set x [make $raw]; set view $x}",
                ValueRepresentation::List,
            ),
            (
                "proc make {raw} {if {$raw} {return [list k v]}; list k $raw}; proc f {raw} {set x [make $raw]; set view $x}",
                ValueRepresentation::Unknown,
            ),
            (
                "proc observer {command code result op} {dict size $result}; proc make {raw} {list k $raw}; trace add execution make leave observer; proc f {raw} {set x [make $raw]; set view $x}",
                ValueRepresentation::Unknown,
            ),
        ] {
            let bindings = analyse(source);
            let offset = u32::try_from(source.rfind("$x").unwrap()).unwrap();
            let reads =
                bindings.variable_accesses_in_span(tcl_lexer::Span::new(offset, offset + 2));
            assert_eq!(reads.len(), 1, "{source}");
            for context in reads[0].context_alternatives() {
                let place = crate::var_resolve::resolve_substitution_access(
                    &reads[0].original_spelling,
                    context,
                    registry,
                    tcl_registry::TraceOperation::Read,
                );
                assert_eq!(
                    context.contents_representation_at(&place),
                    expected,
                    "{source}"
                );
            }
        }
    }

    #[test]
    fn range_bytes_are_independent_of_its_normal_list_shape() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        for (arguments, first, last, expected) in [
            ("a b c", "0", "end", Some("a b c")),
            ("a b c", "1", "end", Some("b c")),
            ("a b c", "-10", "100", Some("a b c")),
            ("{a b} c", "1", "end", Some("c")),
            ("{a b} c", "0", "0", Some("{a b}")),
            ("#value c", "0", "0", Some("{#value}")),
        ] {
            let source = format!(
                "proc f {{}} {{set constructor list; set lst [$constructor {arguments}]; set x [lrange $lst {first} {last}]; set view $x}}"
            );
            let bindings = analyse(&source);
            let offset = u32::try_from(source.rfind("$x").unwrap()).unwrap();
            let reads =
                bindings.variable_accesses_in_span(tcl_lexer::Span::new(offset, offset + 2));
            assert_eq!(reads.len(), 1, "{source}");
            for context in reads[0].context_alternatives() {
                let place = reads[0].place_in_context(context, registry);
                assert_eq!(
                    context.contents_representation_at(&place),
                    ValueRepresentation::List,
                    "{source}"
                );
                assert_eq!(
                    context.literal_contents_at(&place, registry),
                    expected,
                    "{source}"
                );
            }
        }
    }

    #[test]
    fn representation_receipts_require_a_known_unchanged_epoch() {
        let mut variables = crate::var_resolve::ResolveContext::default();
        let receipt =
            super::FrozenSourceRepresentation::capture(ValueRepresentation::Dict, &variables)
                .unwrap();
        assert!(receipt.is_current(&variables));
        variables.invalidate_shared_representations();
        assert!(!receipt.is_current(&variables));
        assert!(variables.value_representations.is_empty());
        variables.representation_epoch = None;
        assert!(
            super::FrozenSourceRepresentation::capture(ValueRepresentation::Dict, &variables)
                .is_none()
        );
    }

    #[test]
    fn a_constructor_after_an_uncertain_join_cannot_revive_an_older_receipt() {
        let mut left = crate::var_resolve::ResolveContext::default();
        let original =
            super::FrozenSourceRepresentation::capture(ValueRepresentation::Dict, &left).unwrap();
        let mut right = left.clone();
        right.invalidate_shared_representations();
        let newer =
            super::FrozenSourceRepresentation::capture(ValueRepresentation::List, &right).unwrap();
        left.join(&right);
        assert!(left.representation_epoch.is_none());
        let high_water = left.representation_epoch_high_water;
        left.invalidate_shared_representations();
        assert_eq!(left.representation_epoch_high_water, high_water);
        left.establish_created_representation_epoch();
        assert!(
            super::FrozenSourceRepresentation::capture(ValueRepresentation::Dict, &left).is_none()
        );
        assert!(!original.is_current(&left));
        assert!(!newer.is_current(&left));
        assert!(left.representation_epoch_high_water.is_none());
        let mut child = left.clone();
        child.invalidate_shared_representations();
        child.representation_epoch = None;
        let mut restored = crate::var_resolve::restore_execution_frame(&left, &child);
        restored.establish_created_representation_epoch();
        assert!(!original.is_current(&restored));
        restored.representation_epoch = None;
        restored.representation_epoch_high_water = Some(u64::MAX);
        restored.establish_created_representation_epoch();
        assert!(restored.representation_epoch.is_none());
        assert!(restored.representation_epoch_high_water.is_none());
    }

    #[test]
    fn unknown_outer_iteration_objects_keep_a_callback_residual() {
        // All six engine probes retain updater/free effects even on zero-trip
        // inputs: tcl-syntax/tests/data/native_list_methods/string-callbacks/README.md.
        let source = "proc f {items} {foreach x $items {set d [dict create k $x]; llength $d}}";
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let bindings = analyse(source);
        let point = bindings.invocation_at_source(
            "llength",
            u32::try_from(source.rfind("llength").unwrap()).unwrap(),
        );
        let target = crate::var_resolve::resolve_literal_access(
            "d",
            &point.variable_context,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        assert_eq!(
            point.variable_context.contents_representation_at(&target),
            ValueRepresentation::Unknown
        );
        assert!(
            !point
                .variable_context
                .read_produces_value(&target, registry)
        );
    }

    #[test]
    fn an_entered_loop_constructor_retains_unknown_bytes_with_its_actual_representation() {
        // Unknown outer objects can invoke updater/free hooks before iteration:
        // tcl-syntax/tests/data/native_list_methods/string-callbacks/README.md.
        // Keep the element bytes unknown, with an actual ordinary outer List.
        let source = "proc f {item} {set items [list $item]; foreach x $items {set d [dict create k $x]; llength $d}}";
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let bindings = analyse(source);
        let point = bindings.invocation_at_source(
            "llength",
            u32::try_from(source.rfind("llength").unwrap()).unwrap(),
        );
        let target = crate::var_resolve::resolve_literal_access(
            "d",
            &point.variable_context,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        assert!(
            point
                .variable_context
                .literal_contents_at(&target, registry)
                .is_none()
        );
        assert_eq!(
            point.variable_context.contents_representation_at(&target),
            ValueRepresentation::Dict,
            "target={target:?} presence={:?} origin={:?} kind={:?} representations={:?}",
            point.variable_context.contents_presence(&target),
            point.variable_context.contents_origin(&target),
            point.variable_context.root_contents_kind(&target),
            point.variable_context.value_representations,
        );
    }

    #[test]
    fn callback_container_conversion_reaches_the_retained_caller_alias() {
        let source = "proc observe args {upvar 1 dictionary destination; llength $destination}; proc f {} {set dictionary [dict create k 1]; upvar 0 dictionary view; trace add variable view write observe; set dictionary [dict create k 2]; llength $view}";
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let bindings = analyse(source);
        let point = bindings.invocation_at_source(
            "llength",
            u32::try_from(source.rfind("llength").unwrap()).unwrap(),
        );
        let target = crate::var_resolve::resolve_literal_access(
            "view",
            &point.variable_context,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        let callback_offset = u32::try_from(source.find("$destination").unwrap()).unwrap();
        let callback_reads = bindings
            .variable_accesses_in_span(tcl_lexer::Span::new(callback_offset, callback_offset + 12));
        let callback_proofs = callback_reads
            .iter()
            .flat_map(|read| {
                read.context_alternatives().iter().map(|context| {
                    let place = crate::var_resolve::resolve_substitution_access(
                        &read.original_spelling,
                        context,
                        registry,
                        tcl_registry::TraceOperation::Read,
                    );
                    (
                        place.clone(),
                        context.read_produces_value(&place, registry),
                        context
                            .literal_contents_at(&place, registry)
                            .map(str::to_owned),
                        context.contents_representation_at(&place),
                    )
                })
            })
            .collect::<Vec<_>>();
        assert_eq!(
            point.variable_context.contents_representation_at(&target),
            ValueRepresentation::List,
            "target={target:?} presence={:?} origin={:?} kind={:?} representations={:?} callback={callback_proofs:?}",
            point.variable_context.contents_presence(&target),
            point.variable_context.contents_origin(&target),
            point.variable_context.root_contents_kind(&target),
            point.variable_context.value_representations
        );
    }

    #[test]
    fn normal_ordinary_conversion_commits_only_the_captured_final_operand() {
        let source = "proc f {} {set d [dict create a 1]; set view $d; llength $d; puts $d}";
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let bindings = analyse(source);
        let point = bindings.invocation_at_source(
            "puts",
            u32::try_from(source.rfind("puts").unwrap()).unwrap(),
        );
        let direct = crate::var_resolve::resolve_literal_access(
            "d",
            &point.variable_context,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        let shared = crate::var_resolve::resolve_literal_access(
            "view",
            &point.variable_context,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        assert_eq!(
            point.variable_context.contents_representation_at(&direct),
            ValueRepresentation::List
        );
        assert_eq!(
            point.variable_context.contents_representation_at(&shared),
            ValueRepresentation::Unknown
        );
        let alternatives = point
            .variable_context
            .container_representation_alternatives_at(&shared)
            .unwrap();
        assert!(alternatives.contains(ValueRepresentation::List));
        assert!(alternatives.contains(ValueRepresentation::Dict));
    }

    #[test]
    fn normal_empty_list_conversion_preserves_a_closed_dictionary_alternative() {
        let source = "proc f {} {set d [dict create]; llength $d; puts $d}";
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let bindings = analyse(source);
        let point = bindings.invocation_at_source(
            "puts",
            u32::try_from(source.rfind("puts").unwrap()).unwrap(),
        );
        let place = crate::var_resolve::resolve_literal_access(
            "d",
            &point.variable_context,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        assert_eq!(
            point.variable_context.contents_representation_at(&place),
            ValueRepresentation::Unknown
        );
        let alternatives = point
            .variable_context
            .container_representation_alternatives_at(&place)
            .unwrap();
        assert!(alternatives.contains(ValueRepresentation::List));
        assert!(alternatives.contains(ValueRepresentation::Dict));
    }

    #[test]
    fn ordinary_list_length_completion_does_not_rejoin_a_pending_return_world() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        for (definition, expected) in [
            ("proc convert {} {llength $::d}", ValueRepresentation::List),
            (
                "rename llength saved; proc llength value {return -code return 0}; proc convert {} {llength $::d}",
                ValueRepresentation::Dict,
            ),
        ] {
            let source = format!("set d [dict create k 1]; {definition}; convert; puts $d");
            let bindings = analyse(&source);
            let point = bindings.invocation_at_source(
                "puts",
                u32::try_from(source.rfind("puts").unwrap()).unwrap(),
            );
            let receiver = crate::var_resolve::resolve_literal_access(
                "d",
                &point.variable_context,
                false,
                registry,
                tcl_registry::TraceOperation::Read,
            );
            assert_eq!(
                point.variable_context.contents_representation_at(&receiver),
                expected,
                "definition {definition:?}"
            );
        }
    }

    #[test]
    fn list_range_shape_requires_a_current_ordinary_nonempty_selection() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        for (input, first, last, expected) in [
            ("[$constructor 7]", "0", "1", ValueRepresentation::List),
            ("[$constructor 7]", "2", "3", ValueRepresentation::Unknown),
            ("$unknown", "0", "1", ValueRepresentation::Unknown),
        ] {
            let source = format!(
                "proc f {{unknown}} {{set constructor list; set x [lrange {input} {first} {last}]; puts $x}}"
            );
            let bindings = analyse(&source);
            let point = bindings.invocation_at_source(
                "puts",
                u32::try_from(source.rfind("puts").unwrap()).unwrap(),
            );
            let place = crate::var_resolve::resolve_literal_access(
                "x",
                &point.variable_context,
                false,
                registry,
                tcl_registry::TraceOperation::Read,
            );
            assert_eq!(
                point.variable_context.contents_representation_at(&place),
                expected,
                "{source}"
            );
        }
    }

    #[test]
    fn unknown_constructor_bytes_retain_the_actual_result_representation() {
        for constructor in ["dict create k $value", "list $value"] {
            let source = format!("proc f {{value}} {{set d [{constructor}]; llength $d}}");
            let bindings = analyse(&source);
            let point = bindings.invocation_at_source(
                "llength",
                u32::try_from(source.rfind("llength").unwrap()).unwrap(),
            );
            let target = crate::var_resolve::resolve_literal_access(
                "d",
                &point.variable_context,
                false,
                tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
                tcl_registry::TraceOperation::Read,
            );
            let expected = if constructor.starts_with("dict") {
                ValueRepresentation::Dict
            } else {
                ValueRepresentation::List
            };
            assert!(
                point
                    .variable_context
                    .literal_contents_at(
                        &target,
                        tcl_registry::model::ingress::static_context_for("tcl8.6").commands()
                    )
                    .is_none()
            );
            assert_eq!(
                point.variable_context.contents_representation_at(&target),
                expected,
                "{constructor}"
            );
        }
    }

    #[test]
    fn compiled_list_representation_requires_the_original_construction_recipe() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        for (constructor, expected) in [
            ("list $value b", ValueRepresentation::List),
            ("list a b", ValueRepresentation::Unknown),
        ] {
            let source = format!("proc f {{value}} {{set d [{constructor}]; llength $d}}");
            let bindings = analyse(&source);
            let point = bindings.invocation_at_source(
                "llength",
                u32::try_from(source.rfind("llength").unwrap()).unwrap(),
            );
            let target = crate::var_resolve::resolve_literal_access(
                "d",
                &point.variable_context,
                false,
                registry,
                tcl_registry::TraceOperation::Read,
            );
            assert_eq!(
                point.variable_context.contents_representation_at(&target),
                expected,
                "{constructor}: the native procedure owns its compiled body"
            );
        }
    }

    #[test]
    fn a_command_leave_coercion_cannot_publish_a_stale_rhs_representation() {
        let source = "set g [dict create k 1]; proc coerce {command code value op} {llength $::g}; trace add execution set leave coerce; set d [set ::g]; trace remove execution set leave coerce; llength $d";
        let bindings = analyse(source);
        let point = bindings.invocation_at_source(
            "llength",
            u32::try_from(source.rfind("llength").unwrap()).unwrap(),
        );
        let target = crate::var_resolve::resolve_literal_access(
            "d",
            &point.variable_context,
            false,
            tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
            tcl_registry::TraceOperation::Read,
        );
        assert_eq!(
            point.variable_context.contents_representation_at(&target),
            ValueRepresentation::Unknown
        );
    }

    #[test]
    fn a_frozen_constructor_representation_survives_only_unchanged_store_callbacks() {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        for (callback, expected, contents) in [
            ("", ValueRepresentation::Dict, "k 2"),
            (
                "upvar 1 d destination; llength $destination",
                ValueRepresentation::List,
                "k 2",
            ),
            (
                "upvar 1 d destination; set destination changed",
                ValueRepresentation::Unknown,
                "changed",
            ),
        ] {
            let source = format!(
                "proc observe args {{{callback}}}; set d [dict create k 1]; trace add variable d write observe; set d [dict create k 2]; llength $d"
            );
            let bindings = SourceCommandBindings::analyse_with_options(
                &source,
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                registry,
                SourceAnalysisOptions {
                    invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                    native_compilation:
                        tcl_registry::native_compilation::NativeCompilationContext {
                            mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                            ..Default::default()
                        },
                    ..Default::default()
                },
            );
            let point = bindings.invocation_at_source(
                "llength",
                u32::try_from(source.rfind("llength $d").unwrap()).unwrap(),
            );
            let target = crate::var_resolve::resolve_literal_access(
                "d",
                &point.variable_context,
                false,
                registry,
                tcl_registry::TraceOperation::Read,
            );
            assert_eq!(
                point.variable_context.contents_representation_at(&target),
                expected,
                "callback {callback:?}",
            );
            assert_eq!(
                point
                    .variable_context
                    .literal_contents_at(&target, registry),
                Some(contents),
                "callback {callback:?}",
            );
        }
    }
}
