// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Fresh stock factory transfers, separate from source declaration advice.

use super::{
    Arc, BTreeSet, BindingKind, CommandAllocationSite, MayBinding, ModuleCommandBindings,
    SourceCommandKey, SourceExecutionContext, SourceNativeInvocation,
};
use crate::signature_scan::scope::SignatureSourceNameInput;
use tcl_registry::{
    CommandBindingDefinitionKind, CommandBindingTransition, InvocationFacts, ObjectDispatchKind,
    ObjectDispatchTarget, ObjectDispatchTransition, ObjectPrivateNamespace, StateTransition,
};

/// Complete pre-operation intrinsic premises. The fresh public slot, stock
/// constructor, complete actual definition script and selected intrinsic
/// workers are independent of any later publication/metadata row.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct OriginalClassFactoryTransfer {
    site: CommandAllocationSite,
    words: Vec<crate::ir::WordExpr>,
    config: tcl_lexer::LexerConfig,
    factory: super::registered_class_factory::OriginalClassFactoryState,
    inputs: Vec<SignatureSourceNameInput>,
    name: SignatureSourceNameInput,
    destination: SourceCommandKey,
    body: super::ExecutedScriptSource,
}

impl OriginalClassFactoryTransfer {
    pub(super) fn capture(
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<Self> {
        if !quiet(state) || native.invocation.arguments().exact_argv_len() != Some(3) {
            return None;
        }
        let factory = super::registered_class_factory::OriginalClassFactoryState::capture(
            native, state, context,
        )?;
        #[cfg(test)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_FACTORY_CAPTURE offset={} constructor={:?} current={} quiet={}",
                native.segment.span.start(),
                factory.recipe().constructor(),
                factory.is_current(state),
                quiet(state)
            );
        }
        let inputs = (0..3)
            .map(|index| {
                native
                    .original_variable_operands
                    .input(index, &state.source_variables)
                    .cloned()
            })
            .collect::<Option<Vec<_>>>()?;
        // Static original operands retain actual scalar String producers. A
        // captured/evaluated value cannot acquire that storage/cleanup closure.
        if inputs
            .iter()
            .any(|input| input.original_word_key().is_none())
        {
            return None;
        }
        let policy = inputs.first()?.policy();
        if inputs.iter().any(|input| input.policy() != policy)
            || policy.recipe()
                != tcl_syntax::naming::NativeNameProtocol::C(factory.recipe().version())
        {
            return None;
        }
        let grammar = factory.recipe().grammar();
        let manufacturer = grammar.manufacturer(std::str::from_utf8(inputs[0].bytes()).ok()?)?;
        if manufacturer.names_instance_at != Some(1) || manufacturer.constructor_args_from != 2 {
            return None;
        }
        let (name, destination) = fresh_destination(native, facts, state, context)?;
        if name != inputs[1] {
            return None;
        }
        let body_argument = facts.arg_roles.iter().find_map(|(at, role)| {
            (*role == tcl_registry::ArgRole::Body)
                .then_some(facts.argument_offset + usize::from(*at))
        })?;
        if body_argument != 2 {
            return None;
        }
        let body = super::retained_script_operand(
            body_argument,
            native.script_operands(),
            state,
            native.segment.span.start(),
            context.config,
        )?;
        #[cfg(test)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_FACTORY_BODY offset={} closed={}",
                native.segment.span.start(),
                closed_declarations(grammar, &body, policy, state, context).is_some()
            );
        }
        closed_declarations(grammar, &body, policy, state, context)?;
        Some(Self {
            site: CommandAllocationSite {
                source: Arc::clone(state.current_source_origin.as_ref()?),
                offset: native.segment.span.start(),
            },
            words: native.words.to_vec(),
            config: context.config,
            factory,
            inputs,
            name,
            destination,
            body,
        })
    }

    pub(super) fn retain_completed_definition(
        &self,
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> bool {
        self.completed(native, facts, state, context)
            && state.retain_original_class_publication_definition(
                &self.site,
                &self.destination,
                &self.name,
            )
    }

    pub(super) fn completed(
        &self,
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> bool {
        #[cfg(test)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_FACTORY_POST offset={} quiet={} factory_current={} destination={:?} definition_count={}",
                native.segment.span.start(),
                quiet(state),
                self.factory.is_current(state),
                state.original_bindings_for_key(&self.destination),
                state.class_definitions.len()
            );
        }
        if !quiet(state)
            || self.site.offset != native.segment.span.start()
            || state.current_source_origin.as_ref() != Some(&self.site.source)
            || self.words != native.words
            || self.config != context.config
            || !self.factory.is_current(state)
            || self.inputs.iter().enumerate().any(|(index, input)| {
                native
                    .original_variable_operands
                    .input(index, &state.source_variables)
                    != Some(input)
            })
        {
            return false;
        }
        let Some(bindings) = state.original_bindings_for_key(&self.destination) else {
            return false;
        };
        let targets = bindings.iter().cloned().collect::<Vec<_>>();
        let [MayBinding::Target(target)] = targets.as_slice() else {
            return false;
        };
        let Some(identity) = target.token.as_ref() else {
            return false;
        };
        let Some(allocation) = target.implementation_allocation.as_ref() else {
            return false;
        };
        if target.kind != BindingKind::Class
            || !target.terminal
            || !target.prepended.is_empty()
            || target.registry_backed
            || identity.runtime.is_some()
            || allocation.site != self.site
            || identity.allocation.as_ref() != Some(allocation)
            || allocation.incarnation == super::AllocationIncarnation::RepeatedFresh
            || state
                .objects
                .get(identity)
                .is_some_and(|objects| objects != &bindings)
        {
            return false;
        }
        let Some(definition) = state.class_definitions.get(identity) else {
            return false;
        };
        if definition.original_factory.as_ref() != Some(&self.factory)
            || definition.implementation_generation != target.implementation_generation
        {
            return false;
        }
        let Some(slot) = state.original_byte_slot_for_key(&self.destination, self.name.policy())
        else {
            return false;
        };
        let Some(publication) = state.original_publication_at(&slot, self.name.policy()) else {
            return false;
        };
        publication.name_input() == &self.name
            && publication.declaration_site() == &self.site
            && publication.has_implementation_allocation(allocation)
            && super::retained_script_operand(
                2,
                native.script_operands(),
                state,
                native.segment.span.start(),
                context.config,
            )
            .as_ref()
                == Some(&self.body)
            && facts.traits.contains(tcl_registry::Traits::IS_OO_METACLASS)
    }
}

fn fresh_destination(
    native: SourceNativeInvocation<'_>,
    facts: &InvocationFacts,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<(SignatureSourceNameInput, SourceCommandKey)> {
    let transitions = facts.state_transitions.declared()?;
    let mut creates = transitions
        .facts()
        .iter()
        .filter_map(|fact| match &fact.transition {
            StateTransition::ObjectDispatch(ObjectDispatchTransition::Create {
                target: ObjectDispatchTarget::Named(name),
                private_namespace: ObjectPrivateNamespace::Fresh,
                kind: ObjectDispatchKind::Class,
            }) => Some(name),
            _ => None,
        });
    let subject = creates.next()?;
    if creates.next().is_some()
        || transitions.facts().len() != 2
        || !transitions.facts().iter().any(|fact| {
            matches!(&fact.transition,
        StateTransition::CommandBinding(CommandBindingTransition::Define {
            name, kind: CommandBindingDefinitionKind::Object,
        }) if name == subject)
        })
    {
        return None;
    }
    let name = super::original_command_table::original_operand(native, subject, state, context)?;
    let policy = name.policy();
    let current = context.namespace_identity();
    let scope = state.original_namespace_geometry(&current, policy)?;
    let slot = super::source_command_world::publication_slot(
        &scope,
        &name,
        CommandBindingDefinitionKind::Object,
    )?;
    if slot.simple.as_bytes().is_empty() {
        return None;
    }
    let destination = state.original_command_key_for_slot(&slot, policy)?;
    if !state.namespaces.contains(destination.holder().as_ref())
        || state
            .unknown_lookup_namespaces
            .contains(destination.holder().as_ref())
        || state.original_bindings_for_key(&destination)? != BTreeSet::from([MayBinding::Missing])
    {
        return None;
    }
    Some((name, destination))
}

fn closed_declarations(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    body: &super::ExecutedScriptSource,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<()> {
    let map = tcl_lexer::SourceMap::from_image(&body.text).with_base(body.base(), 0, 0);
    let commands = crate::segmenter::segment_commands_image_with_offset_and_config(
        &body.text,
        body.base(),
        context.config,
    )?;
    for command in &commands {
        #[cfg(test)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            let words = crate::ir::CommandTokens::from_segmented(&map, context.config, command);
            let values = super::source_effective_words(words.words(), state.baseline.dialect, None);
            let member = values
                .first()
                .and_then(|word| word.as_registry_word().literal())
                .and_then(|head| grammar.member(head));
            eprintln!(
                "ORIGINAL_FACTORY_MEMBER offset={} member={:?} partial={} closed={} target={:?}",
                command.span.start(),
                member.map(|member| member.keyword),
                command.is_partial,
                super::deferred_method::closed_definition_member(
                    grammar, body, &map, command, state, context
                ),
                member
                    .and_then(
                        |member| super::deferred_method::original_definition_member_target(
                            grammar,
                            member,
                            tcl_registry::definer::DefinitionReceiver::Instance,
                            state,
                            context
                        )
                    )
                    .map(|target| (target.command, target.kind))
            );
        }
        if command.is_partial
            || !super::deferred_method::closed_definition_member(
                grammar, body, &map, command, state, context,
            )
        {
            return None;
        }
        let words = crate::ir::CommandTokens::from_segmented(&map, context.config, command);
        let values = super::source_effective_words(words.words(), state.baseline.dialect, None);
        let member = grammar.member(values.first()?.as_registry_word().literal()?)?;
        // The common declaration closure validates stored native method
        // formals/body and current workers. Variable names additionally
        // need their own native declaration validator before Normal.
        if member.all_args_var {
            let arguments = values
                .get(1..)?
                .iter()
                .map(crate::registry_invocation::EffectiveInvocationWord::literal_bytes)
                .collect::<Option<Vec<_>>>()?;
            let mut inventory =
                super::receiver_variable_inventory::OriginalReceiverVariableInventory::empty(
                    grammar, policy,
                )?;
            let native = crate::registry_invocation::original_native_compiler_words(
                body.origin.source_image(),
                words.words(),
                command.span.start(),
                context.config,
            )?;
            let original = native
                .iter()
                .skip(1)
                .map(|word| {
                    crate::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
                        word,
                        tcl_syntax::word_rules::WordValueRules::from_config(&context.config),
                        policy,
                    )
                    .map(SignatureSourceNameInput::OriginalWord)
                })
                .collect::<Option<Vec<_>>>()?;
            if arguments.len() != original.len() {
                return None;
            }
            inventory.apply(member.slot, original)?;
        }
    }
    Some(())
}

fn quiet(state: &ModuleCommandBindings) -> bool {
    !state.has_opaque_domain()
        && state.baseline.native_entry.is_none()
        && !state.baseline.unknown_entry
        && !state.source_step_observed()
        && !state.source_execution_observed(None)
        && state.command_observers.registrations.is_empty()
        && state.command_observers.pending_mutations.is_empty()
}

#[cfg(test)]
mod tests {
    fn analyse(
        source: &str,
        version: tcl_dialect::TclVersion,
    ) -> super::super::SourceCommandBindings {
        let dialect = tcl_registry::InvocationDialect::for_version(version);
        let registry =
            tcl_registry::model::ingress::static_context_for(version.dialect_name()).commands();
        super::super::SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
            registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(dialect),
                ..Default::default()
            },
        )
    }

    #[test]
    fn original_stock_class_factory_normal_requires_fresh_valid_intrinsic_definition() {
        // Implementation contract: naming.tcloo.original-stock-class-factory-normal-transfer
        // docs/design/analysis/name-resolution-proofs/tcloo-original-stock-class-factory-normal-transfer.md
        for version in [
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            for source in [
                "oo::class create C {}",
                r"oo::class create C\uD800 {method p\uD801 {} {return YES}}",
                "oo::class create C {constructor {x} {return}; destructor {return}; variable value}",
            ] {
                assert!(
                    analyse(source, version)
                        .original_completed_command_world()
                        .is_some(),
                    "{version:?}: {source}"
                );
            }
            for source in [
                "proc occupied {} {}; oo::class create occupied {}",
                "oo::class create ::Missing::C {}",
                "oo::class create C {method p {a(x)} {}}",
                "oo::class create C {variable a(b)}",
                "oo::class create C {arbitrary}",
            ] {
                assert!(
                    analyse(source, version)
                        .original_completed_command_world()
                        .is_none(),
                    "{version:?}: {source}"
                );
            }
        }
        for version in [tcl_dialect::TclVersion::V9_0, tcl_dialect::TclVersion::V9_1] {
            assert!(analyse("oo::configurable create C {property readable -kind readable writable -kind writable}",version)
                .original_completed_command_world().is_some());
        }
    }
}
