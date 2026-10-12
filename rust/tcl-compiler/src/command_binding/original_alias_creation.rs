// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Independent normal transfer for fresh, quiet same-interpreter aliases.

use super::{
    Arc, BTreeSet, BindingKind, CommandAllocationSite, MayBinding, ModuleCommandBindings,
    ResolvedCommandTarget, SourceCommandKey, SourceExecutionContext, SourceNativeInvocation,
};
use crate::signature_scan::scope::SignatureSourceNameInput;
use tcl_registry::{CommandBindingTransition, InvocationFacts, StateTransition};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct OriginalAliasCreation {
    site: CommandAllocationSite,
    words: Vec<crate::ir::WordExpr>,
    config: tcl_lexer::LexerConfig,
    handler: super::SourceCommandTarget,
    inputs: Vec<SignatureSourceNameInput>,
    alias: SignatureSourceNameInput,
    target: SignatureSourceNameInput,
    arguments: Vec<SignatureSourceNameInput>,
    destination: SourceCommandKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AliasNode {
    Proposed,
    Existing(SourceCommandKey, Box<ResolvedCommandTarget>),
}

impl OriginalAliasCreation {
    pub(super) fn capture(
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<Self> {
        #[cfg(test)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_ALIAS_CAPTURE offset={} quiet={} handler={:?} kind={:?} current={} argc={:?} roles_complete={} effects={:?} transitions={:?}",
                native.segment.span.start(),
                quiet(state),
                facts.successful_handler,
                native.target.kind,
                state.retained_target_is_current(native.target),
                native.invocation.arguments().exact_argv_len(),
                facts.arg_roles_complete,
                facts.effects,
                facts.state_transitions
            );
        }
        if !quiet(state)
            || facts.analyser_hook != Some(tcl_registry::hooks::AnalyserHookId::InterpAlias)
            || facts.successful_handler != Some(
                tcl_registry::native_compilation::SuccessfulHandlerSpec::CommandBindingTransition,
            )
            || !native.target.registry_backed
            || native.target.kind != BindingKind::Builtin
            || native.target.implementation_generation != 0
            || native.target.runtime_implementation_generation.is_some()
            || !native.target.prepended.is_empty()
            || !state.retained_target_is_current(native.target)
            || facts.effects.accesses().iter().any(|access| {
                access.mode != tcl_registry::world_effect::EffectAccessMode::Read
            })
        {
            return None;
        }
        facts.successful_handler_effects(
            native.invocation.arguments(),
            tcl_registry::VariableAliasFrame::Unknown,
        )?;
        let count = native.invocation.arguments().exact_argv_len()?;
        let inputs = (0..count)
            .map(|index| {
                native
                    .original_variable_operands
                    .input(index, &state.source_variables)
                    .cloned()
            })
            .collect::<Option<Vec<_>>>()?;
        #[cfg(test)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_ALIAS_INPUTS offset={} inputs={:?}",
                native.segment.span.start(),
                inputs
                    .iter()
                    .map(super::super::signature_scan::name_value::SignatureSourceNameInput::bytes)
                    .collect::<Vec<_>>()
            );
        }
        let policy = inputs.first()?.policy();
        let tcl_syntax::naming::NativeNameProtocol::C(version) = policy.recipe() else {
            return None;
        };
        let dialect = tcl_registry::InvocationDialect::for_version(version);
        let spec = context.registry.get_for_surface(
            native.target.registry_identity()?,
            Some(dialect.authoring_query()?.with_realm(context.realm)),
        )?;
        if tcl_registry::registry::spec_pack_of(spec) != Some("tcl")
            || inputs.iter().any(|input| input.policy() != policy)
        {
            return None;
        }
        let receipt = Self::capture_receipt(native, facts, state, context, inputs)?;
        #[cfg(test)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_ALIAS_PRE offset={} destination={:?} chain_closed={}",
                native.segment.span.start(),
                receipt.destination,
                receipt.alias_chain_is_closed(state)
            );
        }
        if receipt.alias_chain_is_closed(state) {
            Some(receipt)
        } else {
            None
        }
    }

    fn capture_receipt(
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
        inputs: Vec<SignatureSourceNameInput>,
    ) -> Option<Self> {
        let transitions = facts.state_transitions.declared()?;
        let [fact] = transitions.facts() else {
            return None;
        };
        let StateTransition::CommandBinding(CommandBindingTransition::Alias {
            source_interpreter,
            alias,
            target_interpreter,
            target,
            arguments,
            target_lookup,
        }) = &fact.transition
        else {
            return None;
        };
        if *target_lookup != tcl_registry::AliasTargetLookup::Global {
            return None;
        }
        let original = |subject: &tcl_registry::TransitionSubject| {
            super::original_command_table::original_operand(native, subject, state, context)
        };
        if !original(source_interpreter)?.bytes().is_empty()
            || !original(target_interpreter)?.bytes().is_empty()
        {
            return None;
        }
        let alias = original(alias)?;
        let target = original(target)?;
        let arguments = arguments.iter().map(original).collect::<Option<Vec<_>>>()?;
        let root = state.source_root_namespace_key()?;
        let destination = state.original_publication_key(
            &root,
            &alias,
            super::namespace_slots::PublicationPurpose::Alias,
        )?;
        if !state.namespaces.contains(destination.holder().as_ref())
            || state
                .unknown_lookup_namespaces
                .contains(destination.holder().as_ref())
            || state.original_bindings_for_key(&destination)?
                != BTreeSet::from([MayBinding::Missing])
        {
            return None;
        }
        let receipt = Self {
            site: CommandAllocationSite {
                source: Arc::clone(state.current_source_origin.as_ref()?),
                offset: native.segment.span.start(),
            },
            words: native.words.to_vec(),
            config: context.config,
            handler: native.target.clone(),
            inputs,
            alias,
            target,
            arguments,
            destination,
        };
        Some(receipt)
    }

    fn alias_chain_is_closed(&self, state: &ModuleCommandBindings) -> bool {
        tcl_syntax::naming::alias_chain_loops(AliasNode::Proposed, |node| {
            let input = match node {
                AliasNode::Proposed => &self.target,
                AliasNode::Existing(key, target) => {
                    let slot = state
                        .original_byte_slot_for_key(key, self.alias.policy())
                        .ok_or(())?;
                    let publication = state
                        .original_publication_at(&slot, self.alias.policy())
                        .ok_or(())?;
                    let allocation = target.implementation_allocation.as_ref().ok_or(())?;
                    if !publication.has_implementation_allocation(allocation)
                        || target
                            .token
                            .as_ref()
                            .and_then(|identity| identity.allocation.as_ref())
                            != Some(allocation)
                        || state
                            .objects
                            .get(target.token.as_ref().ok_or(())?)
                            .is_some_and(|implementations| {
                                implementations
                                    != &BTreeSet::from([MayBinding::Target(
                                        target.as_ref().clone(),
                                    )])
                            })
                    {
                        return Err(());
                    }
                    let alias = publication.alias_target().ok_or(())?;
                    if alias.lookup() != tcl_registry::AliasTargetLookup::Global {
                        return Err(());
                    }
                    alias.name_input()
                }
            };
            if input.policy() != self.alias.policy() || !input.is_current(&state.source_variables) {
                return Err(());
            }
            let root = state.source_root_namespace_key().ok_or(())?;
            let paths = state
                .original_alias_loop_paths_for_input(&root, input)
                .ok_or(())?;
            let destination = state
                .original_byte_slot_for_key(&self.destination, self.alias.policy())
                .ok_or(())?;
            let mut unanimous = None;
            for path in paths {
                let mut selected = None;
                for (slot, key) in path {
                    // The fresh reservation will occupy this cell before the
                    // native loop gate performs its actual target lookup.
                    if slot == destination {
                        selected = Some(AliasNode::Proposed);
                        break;
                    }
                    let Some(key) = key else {
                        continue;
                    };
                    let bindings = state.original_bindings_for_key(&key).ok_or(())?;
                    if bindings == BTreeSet::from([MayBinding::Missing]) {
                        continue;
                    }
                    let targets = bindings.iter().cloned().collect::<Vec<_>>();
                    let [MayBinding::Target(target)] = targets.as_slice() else {
                        return Err(());
                    };
                    if target.kind == BindingKind::Alias && !target.terminal {
                        selected = Some(AliasNode::Existing(key, Box::new(target.clone())));
                    } else if !target.terminal {
                        return Err(());
                    }
                    break;
                }
                if unanimous.as_ref().is_some_and(|prior| prior != &selected) {
                    return Err(());
                }
                unanimous = Some(selected);
            }
            unanimous.ok_or(())
        }) == Ok(false)
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
                "ORIGINAL_ALIAS_POST offset={} quiet={} current={} binding={:?} inputs_current={}",
                native.segment.span.start(),
                quiet(state),
                state.retained_target_is_current(&self.handler),
                state.original_bindings_for_key(&self.destination),
                self.inputs
                    .iter()
                    .all(|input| input.is_current(&state.source_variables))
            );
        }
        if !quiet(state)
            || self.config != context.config
            || self.words != native.words
            || self.handler != *native.target
            || !state.retained_target_is_current(&self.handler)
            || self.site.offset != native.segment.span.start()
            || state.current_source_origin.as_ref() != Some(&self.site.source)
            || facts.analyser_hook != Some(tcl_registry::hooks::AnalyserHookId::InterpAlias)
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
        let Some(allocation) = target.implementation_allocation.as_ref() else {
            return false;
        };
        let Some(identity) = target.token.as_ref() else {
            return false;
        };
        if target.kind != BindingKind::Alias
            || target.terminal
            || target.registry_backed
            || allocation.site != self.site
            || allocation.incarnation == super::AllocationIncarnation::RepeatedFresh
            || identity.runtime.is_some()
            || identity.allocation.as_ref() != Some(allocation)
            || state
                .objects
                .get(identity)
                .is_some_and(|implementations| implementations != &bindings)
        {
            return false;
        }
        let Some(slot) = state.original_byte_slot_for_key(&self.destination, self.alias.policy())
        else {
            return false;
        };
        let Some(publication) = state.original_publication_at(&slot, self.alias.policy()) else {
            return false;
        };
        publication.name_input() == &self.alias
            && publication.declaration_site() == &self.site
            && publication.has_implementation_allocation(allocation)
            && publication.alias_target().is_some_and(|alias| {
                alias.lookup() == tcl_registry::AliasTargetLookup::Global
                    && alias.name_input() == &self.target
                    && alias.arguments() == self.arguments
            })
            && self.alias_chain_is_closed(state)
    }
}

fn quiet(state: &ModuleCommandBindings) -> bool {
    !state.has_opaque_domain()
        && state.baseline.native_entry.is_none()
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
        let registry =
            tcl_registry::model::ingress::static_context_for(version.dialect_name()).commands();
        let dialect = tcl_registry::InvocationDialect::for_version(version);
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
    fn original_alias_creation_closes_only_fresh_quiet_actual_transfers() {
        // Implementation contract: naming.alias.original-fresh-creation-transfer
        // docs/design/analysis/name-resolution-proofs/original-fresh-alias-creation-transfer.md
        for version in tcl_dialect::TclVersion::ALL {
            for source in [
                "interp alias {} forward {} future",
                "interp alias {} forward {} ::Missing::future",
                "interp alias {} first {} set value; interp alias {} second {} first",
                "proc P {prefix value} {}; interp alias {} forward {} P HELD",
                "interp alias {} opaque\\uD800 {} future\\uD801 PREFIX",
            ] {
                let bindings = analyse(source, version);
                let world = bindings.original_completed_command_world().expect(source);
                assert!(world.declarations().any(|entry| entry.kind()
                    == super::super::source_command_world::OriginalCommandPublicationKind::Alias));
            }
            for source in [
                "interp alias {} loop {} loop",
                "interp alias {} loop {} ::loop",
                "interp alias {} first {} second; interp alias {} second {} first",
                "proc occupied {} {}; interp alias {} occupied {} set value",
                "interp alias child forward {} set value",
                "interp alias {} forward child set value",
                "interp alias {} forward {} set value; interp alias {} forward {} set other",
            ] {
                assert!(
                    analyse(source, version)
                        .original_completed_command_world()
                        .is_none(),
                    "{}: {source}",
                    version.dialect_name()
                );
            }
        }
    }
}
