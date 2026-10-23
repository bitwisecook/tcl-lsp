// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Current object-method effects, independently of native handler selection.

use super::{
    Arc, CommandAllocationSite, ModuleCommandBindings, SourceCommandBindings,
    SourceExecutionContext, SourceInvocationBinding, SourceNativeInvocation, SourceOriginId,
};
use std::collections::BTreeMap;
use tcl_registry::{
    InvocationArguments, InvocationFacts,
    list_object_methods::{NativeListMethod, NativeListObjectProtocol},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ObjectCallbackEffects {
    Closed,
    Unknown,
}

pub(super) type ObjectCallbackInventory = BTreeMap<CommandAllocationSite, ObjectCallbackEffects>;

#[derive(Clone, Copy)]
pub(super) enum ObjectInputProtocol {
    Ordinary,
    ReadOnlyProvider,
    Unknown,
}

impl ObjectInputProtocol {
    pub(super) const fn preserves_representation(self) -> bool {
        matches!(self, Self::ReadOnlyProvider)
    }
}

impl SourceInvocationBinding {
    /// Every reached native object method at this original invocation has
    /// independently closed effects. This says nothing about compilation.
    pub(crate) fn object_callback_effects_closed(&self) -> bool {
        self.object_callback_effects == Some(ObjectCallbackEffects::Closed)
    }

    pub(super) fn join_object_callback_effects(&mut self, other: &Self) {
        if self.object_callback_effects != other.object_callback_effects {
            self.object_callback_effects = Some(ObjectCallbackEffects::Unknown);
        }
    }
}

impl SourceCommandBindings {
    pub(super) fn retain_math_object_callback_effects(
        &mut self,
        state: &ModuleCommandBindings,
        offset: u32,
        closed: bool,
    ) {
        let effects = if closed {
            ObjectCallbackEffects::Closed
        } else {
            ObjectCallbackEffects::Unknown
        };
        if let Some(source) = &state.current_source_origin {
            self.object_callback_effects
                .entry(CommandAllocationSite {
                    source: Arc::clone(source),
                    offset,
                })
                .and_modify(|previous| {
                    if *previous != effects {
                        *previous = ObjectCallbackEffects::Unknown;
                    }
                })
                .or_insert(effects);
        }
    }
    pub(super) fn attach_object_callback_effects(
        &self,
        mut binding: SourceInvocationBinding,
        origin: Option<&Arc<SourceOriginId>>,
        offset: u32,
    ) -> SourceInvocationBinding {
        binding.object_callback_effects = origin.and_then(|source| {
            self.object_callback_effects
                .get(&CommandAllocationSite {
                    source: Arc::clone(source),
                    offset,
                })
                .copied()
        });
        binding
    }

    pub(super) fn prepare_object_callbacks(
        &mut self,
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> ObjectInputProtocol {
        if let Some(protocol) = self.prepare_numeric_object_callbacks(native, facts, state, context)
        {
            return protocol;
        }
        if let Some(protocol) = facts.list_length_object_protocol(native.invocation.arguments()) {
            use tcl_registry::representation::ListLengthObjectProtocol;
            let protocol = match protocol {
                ListLengthObjectProtocol::Ordinary { argument }
                | ListLengthObjectProtocol::AbstractLength { argument } => current_input_protocol(
                    native,
                    argument,
                    NativeListMethod::Length,
                    state,
                    context,
                ),
                ListLengthObjectProtocol::Unknown => ObjectInputProtocol::Unknown,
            };
            self.record_object_callback_effects(
                state,
                context.invocation_offset,
                !matches!(protocol, ObjectInputProtocol::Unknown),
            );
            return protocol;
        }
        let numeric_indices = facts
            .representation_effect
            .ordinary_container_index_arguments(
                native.invocation.arguments(),
                facts.argument_offset,
            )
            .unwrap_or_default()
            .into_iter()
            .filter(|&index| {
                super::native_result::current_numeric_index(native, index, state, context).is_some()
            })
            .collect::<Vec<_>>();
        let protocol = match facts.native_list_method_requirements(
            native.invocation.arguments(),
            *native.compilation_selection,
            &numeric_indices,
        ) {
            None | Some(NativeListObjectProtocol::Ordinary) => {
                return ObjectInputProtocol::Ordinary;
            }
            Some(
                NativeListObjectProtocol::Unknown
                | NativeListObjectProtocol::Abstract {
                    unresolved: true, ..
                }
                | NativeListObjectProtocol::Conversion {
                    unresolved: true, ..
                },
            ) => ObjectInputProtocol::Unknown,
            Some(
                NativeListObjectProtocol::Abstract { requirements, .. }
                | NativeListObjectProtocol::Conversion { requirements, .. },
            ) => {
                let mut combined = ObjectInputProtocol::ReadOnlyProvider;
                for requirement in requirements {
                    match current_input_protocol(
                        native,
                        requirement.argument,
                        requirement.method,
                        state,
                        context,
                    ) {
                        ObjectInputProtocol::Unknown => {
                            combined = ObjectInputProtocol::Unknown;
                            break;
                        }
                        ObjectInputProtocol::Ordinary => combined = ObjectInputProtocol::Ordinary,
                        ObjectInputProtocol::ReadOnlyProvider => {}
                    }
                }
                combined
            }
        };
        self.record_object_callback_effects(
            state,
            context.invocation_offset,
            !matches!(protocol, ObjectInputProtocol::Unknown),
        );
        protocol
    }

    fn prepare_numeric_object_callbacks(
        &mut self,
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<ObjectInputProtocol> {
        if let Some(protocol) = facts.normal_scalar_math_protocol(native.invocation.arguments()) {
            let tcl_registry::mathfunc::NativeScalarMathInputRequirement::NumericOrClosedStringAccess { argument } = protocol.input_requirement();
            let closed = scalar_math_input_is_closed(native, argument, state, context);
            self.record_object_callback_effects(state, context.invocation_offset, closed);
            Arc::make_mut(&mut state.source_variables)
                .invalidate_math_numeric_operand_representations(protocol.numeric_operand_policy());
            return Some(if closed {
                ObjectInputProtocol::Ordinary
            } else {
                ObjectInputProtocol::Unknown
            });
        }
        if matches!(
            facts.representation_effect,
            tcl_registry::representation::RepresentationEffect::CoerceNumericValues { .. }
        ) {
            let closed = facts
                .numeric_object_conversion_arguments(native.invocation.arguments())
                .is_some_and(|arguments| {
                    arguments.into_iter().all(|argument| {
                        scalar_math_input_is_closed(native, argument, state, context)
                    })
                });
            self.record_object_callback_effects(state, context.invocation_offset, closed);
            return Some(if closed {
                ObjectInputProtocol::Ordinary
            } else {
                ObjectInputProtocol::Unknown
            });
        }
        None
    }

    pub(super) fn record_object_callback_effects(
        &mut self,
        state: &mut ModuleCommandBindings,
        offset: u32,
        closed: bool,
    ) {
        let effects = if closed {
            ObjectCallbackEffects::Closed
        } else {
            ObjectCallbackEffects::Unknown
        };
        if let Some(source) = &state.current_source_origin {
            self.object_callback_effects
                .entry(CommandAllocationSite {
                    source: Arc::clone(source),
                    offset,
                })
                .and_modify(|previous| {
                    if *previous != effects {
                        *previous = ObjectCallbackEffects::Unknown;
                    }
                })
                .or_insert(effects);
        }
        if effects == ObjectCallbackEffects::Unknown {
            // The captured handler still executes with its frozen argv. Only
            // its unenumerated object-method world is withdrawn here.
            state.mark_opaque_binding_mutation();
        }
    }
}

/// A closed stock list conversion can retire cache and epoch receipts while
/// retaining the builtin root-method class. This surrounds only preparation's
/// representation coercion; callback effects and physical stores never use it.
pub(super) fn coerce_closed_list_representations(
    native: SourceNativeInvocation<'_>,
    facts: &InvocationFacts,
    state: &mut ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
    protocol: ObjectInputProtocol,
) {
    use crate::native_numeric::StoredNativeRepresentation::ReadOnlyListMethodProvider;
    use tcl_registry::native_result::NativeListMethodProvider::EmptyListRoot;
    let list_operation = facts
        .list_length_object_protocol(native.invocation.arguments())
        .is_some()
        || facts
            .native_list_method_requirements(
                native.invocation.arguments(),
                *native.compilation_selection,
                &[],
            )
            .is_some();
    let roots = if list_operation && !matches!(protocol, ObjectInputProtocol::Unknown) {
        state
            .source_variables
            .value_representations
            .iter()
            .filter(|(_, value)| matches!(value, ReadOnlyListMethodProvider(EmptyListRoot)))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    super::native_result::coerce_native_representations(facts, native, state, context);
    // This preparation changes only intrep summaries: it performs no physical
    // write or user callback. Stronger disjoint cache evidence wins if retained.
    let variables = Arc::make_mut(&mut state.source_variables);
    for (key, value) in roots {
        variables.value_representations.entry(key).or_insert(value);
    }
}

fn current_input_protocol(
    native: SourceNativeInvocation<'_>,
    argument: usize,
    method: NativeListMethod,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> ObjectInputProtocol {
    let variables = &state.source_variables;
    let Some(representations) = context.written_representations else {
        return ObjectInputProtocol::Unknown;
    };
    let final_operand = native.invocation.arguments().exact_argv_len() == argument.checked_add(1);
    if let Some(receipt) = representations.get(argument + 1).copied().flatten()
        && (receipt.is_current(variables)
            || (final_operand && receipt.is_immediate_created_result()))
        && receipt.callback_input_is_closed_for(variables, method)
    {
        return if method == NativeListMethod::Length
            && receipt.preserves_list_length_representation()
        {
            ObjectInputProtocol::ReadOnlyProvider
        } else {
            ObjectInputProtocol::Ordinary
        };
    }
    if super::literal_object_pool::SourceOrdinaryLiteralObject::capture(
        native, argument, state, context,
    )
    .is_some()
    {
        return ObjectInputProtocol::Ordinary;
    }
    if !original_operand_is_current(native, argument, context) {
        return ObjectInputProtocol::Unknown;
    }
    let Some(place) =
        super::argument_reads::current_argument_place(native, argument, state, context)
    else {
        return ObjectInputProtocol::Unknown;
    };
    if !variables.read_produces_value(place, context.registry) {
        return ObjectInputProtocol::Unknown;
    }
    if variables
        .contents_stock_literal_object_at(place, context.registry)
        .is_some()
        || variables.contents_already_native_numeric_at(place, context.registry)
    {
        return ObjectInputProtocol::Ordinary;
    }
    if method == NativeListMethod::StringAccess
        && variables.contents_native_string_access_closed_at(place, context.registry)
    {
        return ObjectInputProtocol::Ordinary;
    }
    if method != NativeListMethod::StringAccess
        && variables
            .container_representation_alternatives_at(place)
            .is_some()
    {
        ObjectInputProtocol::Ordinary
    } else if variables
        .contents_list_method_provider_at(place, context.registry)
        .is_some_and(|provider| provider.world_is_closed_for(method))
    {
        if method == NativeListMethod::Length
            && variables
                .contents_list_method_provider_at(place, context.registry)
                .is_some_and(
                    tcl_registry::native_result::NativeListMethodProvider::length_is_read_only,
                )
        {
            ObjectInputProtocol::ReadOnlyProvider
        } else {
            ObjectInputProtocol::Ordinary
        }
    } else {
        ObjectInputProtocol::Unknown
    }
}

/// Close scalar math input methods from the actual captured operand class.
/// Native handler identity and successful numeric conversion remain separate.
pub(super) fn scalar_math_input_is_closed(
    native: SourceNativeInvocation<'_>,
    argument: usize,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> bool {
    super::native_result::current_numeric_index(native, argument, state, context).is_some()
        || super::argument_reads::current_argument_place(native, argument, state, context)
            .is_some_and(|place| {
                state
                    .source_variables
                    .contents_stock_literal_object_at(place, context.registry)
                    .is_some()
            })
        || (original_operand_is_current(native, argument, context)
            && context
                .written_representations
                .and_then(|receipts| receipts.get(argument + 1))
                .copied()
                .flatten()
                .is_some_and(|receipt| receipt.numeric_input_is_closed(&state.source_variables)))
        || super::literal_object_pool::SourceOrdinaryLiteralObject::capture(
            native, argument, state, context,
        )
        .is_some()
}

/// Later original literal operands perform no substitutions before dispatch.
/// This is current physical-read evidence, independent of frozen epoch receipts.
pub(super) fn original_operand_is_current(
    native: SourceNativeInvocation<'_>,
    argument: usize,
    context: SourceExecutionContext<'_>,
) -> bool {
    if context.written_representations.is_none()
        || super::native_result::numeric_call_arguments(context, native.target).is_none()
    {
        return false;
    }
    let Some(count) = native.invocation.arguments().exact_argv_len() else {
        return false;
    };
    argument < count
        && (argument + 1..count).all(|later| {
            native
                .script_operands()
                .written_word(later)
                .is_some_and(|word| {
                    matches!(
                        word,
                        crate::ir::WordExpr::Literal { .. }
                            | crate::ir::WordExpr::BracedLiteral { .. }
                    )
                })
        })
}

/// Refine effect-purpose facts for the same original invocation. Native opcode
/// and implementation identities remain intact when object effects are unknown.
pub(crate) fn refine_facts(
    facts: &mut InvocationFacts,
    arguments: InvocationArguments<'_>,
    binding: Option<&SourceInvocationBinding>,
) {
    let selection = binding.map_or(
        tcl_registry::native_compilation::NativeCompilationSelection::Unknown,
        SourceInvocationBinding::native_compilation_selection,
    );
    let closed = ((facts.normal_scalar_math_protocol(arguments).is_none()
        && facts.list_length_object_protocol(arguments).is_none()
        && facts.native_result
            != Some(tcl_registry::native_result::NativeResultContract::IncrementStore)
        && !matches!(
            facts.representation_effect,
            tcl_registry::representation::RepresentationEffect::CoerceNumericValues { .. }
        ))
        || binding.is_some_and(SourceInvocationBinding::object_callback_effects_closed))
        && match facts.native_list_method_requirements(arguments, selection, &[]) {
            None | Some(NativeListObjectProtocol::Ordinary) => true,
            Some(
                NativeListObjectProtocol::Abstract { .. }
                | NativeListObjectProtocol::Conversion { .. }
                | NativeListObjectProtocol::Unknown,
            ) => binding.is_some_and(SourceInvocationBinding::object_callback_effects_closed),
        };
    if !closed {
        facts
            .effects
            .add_callback(tcl_registry::world_effect::CallbackEffect {
                kinds: tcl_registry::world_effect::CallbackKinds::HOST,
                reentrancy: tcl_registry::world_effect::Reentrancy::AnyInterpreter,
            });
        facts.traits.remove(
            tcl_registry::Traits::PURE
                | tcl_registry::Traits::PURE_EVALUATION
                | tcl_registry::Traits::FRAMELESS_RUNTIME,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::super::{SourceAnalysisOptions, SourceCommandBindings};
    use super::*;

    fn analyse(
        source: &str,
        dialect: &str,
    ) -> (SourceCommandBindings, Arc<tcl_registry::CommandRegistry>) {
        let registry =
            Arc::clone(tcl_registry::model::ingress::static_context_for(dialect).commands());
        let profile = registry.profile().unwrap();
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
            &registry,
            SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        (bindings, registry)
    }

    #[test]
    fn scalar_math_hooks_preserve_implementation_but_withdraw_unknown_worlds() {
        for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for function in ["sqrt", "sin", "double", "abs"] {
                let source = format!(
                    "proc f {{input}} {{set keep SAFE; expr {{{function}($input)}}; set answer $keep}}"
                );
                let (bindings, _) = analyse(&source, dialect);
                let after = bindings.invocation_at_source(
                    "set",
                    u32::try_from(source.rfind("set answer").unwrap()).unwrap(),
                );
                assert!(
                    after.variable_context.dynamic_bindings,
                    "{dialect}: native {function} converts an unknown original object"
                );
                let source = format!(
                    "proc f {{}} {{set input [llength {{a b}}]; set keep SAFE; expr {{{function}($input)}}; set answer $keep}}"
                );
                let (bindings, _) = analyse(&source, dialect);
                let after = bindings.invocation_at_source(
                    "set",
                    u32::try_from(source.rfind("set answer").unwrap()).unwrap(),
                );
                assert!(
                    !after.variable_context.dynamic_bindings,
                    "{dialect}: native {function} receives a current Integer object"
                );
            }
        }
    }

    #[test]
    fn increment_hooks_require_the_completed_original_cell_object() {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            for body in ["incr input", "incr input $amount"] {
                let source =
                    format!("proc f {{input amount}} {{set keep SAFE; {body}; set answer $keep}}");
                let (bindings, _) = analyse(&source, dialect);
                let call = bindings.invocation_at_source(
                    "incr",
                    u32::try_from(source.find("incr input").unwrap()).unwrap(),
                );
                assert!(!call.object_callback_effects_closed(), "{dialect}: {body}");
                let after = bindings.invocation_at_source(
                    "set",
                    u32::try_from(source.rfind("set answer").unwrap()).unwrap(),
                );
                assert!(after.variable_context.dynamic_bindings, "{dialect}: {body}");
            }
            let source = "proc f {} {set input [llength {a b}]; set keep SAFE; incr input; set answer $keep}";
            let (bindings, _) = analyse(source, dialect);
            let call = bindings.invocation_at_source(
                "incr",
                u32::try_from(source.find("incr input").unwrap()).unwrap(),
            );
            assert!(call.object_callback_effects_closed(), "{dialect}");
            let after = bindings.invocation_at_source(
                "set",
                u32::try_from(source.rfind("set answer").unwrap()).unwrap(),
            );
            assert!(!after.variable_context.dynamic_bindings, "{dialect}");
        }
    }

    #[test]
    fn increment_input_class_is_captured_after_the_read_callback() {
        let source = "proc replace {name element operation} {upvar 1 $name slot; set slot [llength {a b}]}; proc f {input} {trace add variable input read replace; set keep SAFE; incr input; set answer $keep}";
        let (bindings, _) = analyse(source, "tcl8.6");
        let call = bindings.invocation_at_source(
            "incr",
            u32::try_from(source.find("incr input").unwrap()).unwrap(),
        );
        assert!(
            call.object_callback_effects_closed(),
            "the completed callback installed the actual Integer object"
        );
    }

    #[test]
    fn stock_conversion_effects_stay_open_on_unknown_zero_trip_inputs() {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let source = "proc f {items} {set keep SAFE; foreach item $items {}; set answer $keep}";
            let (bindings, _) = analyse(source, dialect);
            let foreach = bindings.invocation_at_source(
                "foreach",
                u32::try_from(source.find("foreach").unwrap()).unwrap(),
            );
            assert!(
                !foreach.object_callback_effects_closed(),
                "{dialect}: unknown custom scalar may mutate before zero trip"
            );
        }
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let source = "proc f {items} {set items [list $items]; foreach item $items {}}";
            let (bindings, _) = analyse(source, dialect);
            let foreach = bindings.invocation_at_source(
                "foreach",
                u32::try_from(source.find("foreach").unwrap()).unwrap(),
            );
            assert!(
                foreach.object_callback_effects_closed(),
                "{dialect}: current ordinary value list and original variable name"
            );
        }
    }

    #[test]
    fn indexed_and_iterated_methods_require_every_original_object_protocol() {
        for dialect in ["tcl9.0", "tcl9.1"] {
            for body in [
                "set input [lseq 3]; lindex $input 0",
                "set input [lseq 3]; lrange $input 0 1",
                "set input [lseq 3]; foreach element $input {}",
                "set input [dict create k 1]; lrange $input 0 1",
            ] {
                let source = format!("proc f {{}} {{set keep SAFE; {body}; set answer $keep}}");
                let (bindings, _) = analyse(&source, dialect);
                let after = bindings.invocation_at_source(
                    "set",
                    u32::try_from(source.rfind("set answer").unwrap()).unwrap(),
                );
                assert!(
                    !after.variable_context.dynamic_bindings,
                    "{dialect}: {body}"
                );
            }
            for body in [
                "lindex $input 0",
                "lrange $input 0 1",
                "foreach element $input {}",
                "set input [lseq 3]; lindex $input $indices",
                "set input [dict create k 1]; lindex $input 0 0",
            ] {
                let source =
                    format!("proc f {{input indices}} {{set keep SAFE; {body}; set answer $keep}}");
                let (bindings, _) = analyse(&source, dialect);
                let after = bindings.invocation_at_source(
                    "set",
                    u32::try_from(source.rfind("set answer").unwrap()).unwrap(),
                );
                assert!(after.variable_context.dynamic_bindings, "{dialect}: {body}");
            }
        }
    }

    #[test]
    fn unknown_list_inputs_withdraw_callback_worlds_on_every_engine() {
        let source = "proc f {input} {set keep SAFE; llength $input; set answer $keep}";
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let (bindings, registry) = analyse(source, dialect);
            let offset = u32::try_from(source.find("llength").unwrap()).unwrap();
            let call = bindings.invocation_at_source("llength", offset);
            let after = bindings.invocation_at_source(
                "set",
                u32::try_from(source.rfind("set answer").unwrap()).unwrap(),
            );
            assert!(after.variable_context.dynamic_bindings, "{dialect}");
            let words = [tcl_registry::InvocationWord::Dynamic];
            let arguments = InvocationArguments::structured(&words).with_dialect(
                tcl_registry::InvocationDialect::of_profile(registry.profile().unwrap()),
            );
            let invocation = tcl_registry::InvocationWords::structured(
                tcl_registry::InvocationWord::Literal("llength"),
                &words,
            )
            .with_dialect(arguments.dialect().unwrap());
            let resolution =
                tcl_registry::model::semantic::resolve_structured_invocation_in_context(
                    &registry, None, invocation,
                );
            let mut facts = resolution.resolved().unwrap().facts();
            refine_facts(&mut facts, arguments, Some(&call));
            assert!(facts.effects.requires_world_barrier(), "{dialect}");
        }
    }

    #[test]
    fn sequence_provider_result_does_not_preserve_unknown_or_replaced_handler_effects() {
        for dialect in ["tcl9.0", "tcl9.1"] {
            for setup in [
                "set input [lseq $count];",
                "rename lseq stock_sequence; proc lseq {args} {return 3}; set input [lseq 3];",
            ] {
                let source = format!(
                    "proc f {{count}} {{set keep SAFE; {setup} llength $input; set answer $keep}}"
                );
                let (bindings, _) = analyse(&source, dialect);
                let call = bindings.invocation_at_source(
                    "llength",
                    u32::try_from(source.find("llength").unwrap()).unwrap(),
                );
                assert!(
                    !call.object_callback_effects_closed(),
                    "{dialect}: {source}"
                );
                if setup.starts_with("set input") {
                    let after = bindings.invocation_at_source(
                        "set",
                        u32::try_from(source.rfind("set answer").unwrap()).unwrap(),
                    );
                    assert!(
                        after.variable_context.dynamic_bindings,
                        "{dialect}: {source}"
                    );
                }
            }
        }
    }

    #[test]
    fn ordinary_and_audited_provider_inputs_close_only_their_current_native_methods() {
        for (setup, closed) in [
            ("set input [dict create a b];", true),
            ("set input [lseq 3];", true),
            ("set input [lseq 3]; missing_mutator input;", false),
        ] {
            for dialect in ["tcl9.0", "tcl9.1"] {
                let source = format!(
                    "proc f {{}} {{{setup} set keep SAFE; llength $input; set answer $keep}}"
                );
                let (bindings, registry) = analyse(&source, dialect);
                let call = bindings.invocation_at_source(
                    "llength",
                    u32::try_from(source.find("llength").unwrap()).unwrap(),
                );
                let input = crate::var_resolve::resolve_literal_access(
                    "input",
                    &call.variable_context,
                    false,
                    &registry,
                    tcl_registry::TraceOperation::Read,
                );
                let creator = source.find("lseq").map(|offset| {
                    bindings.invocation_at_source("lseq", u32::try_from(offset).unwrap())
                });
                assert_eq!(
                    call.object_callback_effects_closed(),
                    closed,
                    "{dialect}: {source}; input={input:?}; dynamic={}, presence={:?}, origin={:?}, read_ok={}, repr={:?}, provider={:?}, epoch={:?}; creator_handler={:?}, creator_selection={:?}",
                    call.variable_context.dynamic_bindings,
                    call.variable_context.contents_presence(&input),
                    call.variable_context.contents_origin(&input),
                    call.variable_context.read_produces_value(&input, &registry),
                    call.variable_context.contents_representation_at(&input),
                    call.variable_context
                        .contents_list_method_provider_at(&input, &registry),
                    call.variable_context.representation_epoch,
                    creator
                        .as_ref()
                        .and_then(SourceInvocationBinding::proved_handler_target)
                        .map(|target| &target.command),
                    creator
                        .as_ref()
                        .map(SourceInvocationBinding::native_compilation_selection),
                );
                let after = bindings.invocation_at_source(
                    "set",
                    u32::try_from(source.rfind("set answer").unwrap()).unwrap(),
                );
                assert_eq!(
                    after.variable_context.literal_value("keep", &registry) == Some("SAFE"),
                    closed,
                    "{source}"
                );
                if setup == "set input [lseq 3];" {
                    let place = crate::var_resolve::resolve_literal_access(
                        "input",
                        &call.variable_context,
                        false,
                        &registry,
                        tcl_registry::TraceOperation::Read,
                    );
                    assert_eq!(
                        call.variable_context.contents_representation_at(&place),
                        tcl_syntax::value::ValueRepresentation::Unknown
                    );
                    assert!(
                        call.variable_context
                            .container_representation_alternatives_at(&place)
                            .is_none()
                    );
                }
            }
        }
    }

    #[test]
    fn object_effect_sidecars_intersect_and_never_grant_compiler_admission() {
        let mut known = SourceInvocationBinding {
            object_callback_effects: Some(ObjectCallbackEffects::Closed),
            ..Default::default()
        };
        let missing = SourceInvocationBinding::default();
        assert!(known.object_callback_effects_closed());
        assert_eq!(
            known.native_compilation_selection(),
            tcl_registry::native_compilation::NativeCompilationSelection::Unknown
        );
        known.join_object_callback_effects(&missing);
        assert!(!known.object_callback_effects_closed());
        known.join_object_callback_effects(&SourceInvocationBinding {
            object_callback_effects: Some(ObjectCallbackEffects::Closed),
            ..Default::default()
        });
        assert!(!known.object_callback_effects_closed());
    }
}
