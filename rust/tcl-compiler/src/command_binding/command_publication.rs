// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Exact fresh native publications preserve independently unchanged object tables.

use super::{
    Arc, CommandAllocation, CommandAllocationSite, ModuleCommandBindings, SourceCommandTarget,
    SourceExecutionContext, SourceNativeInvocation,
};

pub(super) struct PreservingCommandPublication {
    factory: SourceCommandTarget,
    allocation: CommandAllocation,
    slot: super::SourceCommandKey,
    lookup_namespace: super::SourceNamespaceKey,
    factory_head: String,
    name: String,
    kind: tcl_registry::CommandBindingDefinitionKind,
}

impl PreservingCommandPublication {
    pub(super) fn capture(
        native: SourceNativeInvocation<'_>,
        facts: &tcl_registry::InvocationFacts,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<Self> {
        if (state.baseline.native_entry.is_some() && context.namespace_key.is_none())
            || !native.target.registry_backed
            || native.target.implementation_generation != 0
            || !native.target.prepended.is_empty()
            || state.has_opaque_domain()
            || !state.source_variables.namespace_known
            || state.source_step_observed()
            || state.source_execution_observed(None)
        {
            return None;
        }
        let (name, slot, kind) = selected_publication(native, facts, state, context)?;
        let holder = slot.holder();
        let command = crate::command_binding::ModuleCommandBindings::command_key_label(&slot)?;
        let lookup_namespace = context.namespace_identity();
        let factory_words =
            super::source_effective_words(native.words, state.baseline.dialect, None);
        let factory_head = factory_words
            .first()?
            .as_registry_word()
            .literal()?
            .to_owned();
        if name.is_empty()
            || !state.namespaces.contains(holder.as_ref())
            || state.unknown_lookup_namespaces.contains(holder.as_ref())
            || state.binding_alternatives(&slot)
                != std::collections::BTreeSet::from([super::MayBinding::Missing])
        {
            return None;
        }
        for argument in 0..native.invocation.arguments().exact_argv_len()? {
            super::literal_object_pool::SourceOrdinaryLiteralObject::capture(
                native, argument, state, context,
            )?;
        }
        let site = CommandAllocationSite {
            source: Arc::clone(state.current_source_origin.as_ref()?),
            offset: native.segment.span.start(),
        };
        let incarnation = match state
            .allocation_counts
            .get(&(site.clone(), slot.clone()))
            .copied()
            .unwrap_or(0)
        {
            0 => super::AllocationIncarnation::First,
            1 => super::AllocationIncarnation::Second,
            _ => return None,
        };
        Some(Self {
            factory: native.target.clone(),
            slot: slot.clone(),
            lookup_namespace,
            factory_head,
            name,
            kind,
            allocation: CommandAllocation {
                site,
                incarnation,
                namespace: holder.into_owned(),
                command,
            },
        })
    }

    pub(super) fn preserves(
        &self,
        transition: &tcl_registry::CommandBindingTransition,
        state: &ModuleCommandBindings,
        declaration: u32,
    ) -> bool {
        matches!(transition, tcl_registry::CommandBindingTransition::Define {
            name, kind,
        } if *kind == self.kind && name.literal() == Some(self.name.as_str()))
            && state
                .allocation_counts
                .get(&(self.allocation.site.clone(), self.slot.clone()))
                .copied()
                .unwrap_or(0)
                == match self.allocation.incarnation {
                    super::AllocationIncarnation::First => 0,
                    super::AllocationIncarnation::Second => 1,
                    super::AllocationIncarnation::RepeatedFresh => return false,
                }
            && self.allocation.site.offset == declaration
            && state.current_source_origin.as_ref() == Some(&self.allocation.site.source)
            && super::source_binding(state, &self.factory_head, &self.lookup_namespace)
                .proved_target()
                == Some(&self.factory)
    }
}

/// Conditional absence for a future fresh source allocation. This is never a
/// physical command token, object receipt or permission to execute a handler.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct SourceFreshProcedureTraceAbsence {
    entry: Arc<tcl_runtime_api::NativeCompilationEntry>,
    factory: SourceCommandTarget,
    allocation: CommandAllocation,
    slot: super::SourceCommandKey,
}

impl SourceFreshProcedureTraceAbsence {
    pub(super) fn capture(
        publication: &PreservingCommandPublication,
        native: SourceNativeInvocation<'_>,
        facts: &tcl_registry::InvocationFacts,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<Self> {
        let entry = state.baseline.native_entry.as_ref()?;
        let profile = context.registry.profile()?;
        if publication.kind != tcl_registry::CommandBindingDefinitionKind::Procedure
            || publication.allocation.incarnation != super::AllocationIncarnation::First
            || !entry.closed
            || entry.profile != profile.cache_key()
            || entry.invocation_policy.as_ref() != Some(&entry.profile)
            || entry.execution_point != profile.core_point
            || state.baseline.dialect != Some(tcl_registry::InvocationDialect::of_profile(profile))
            || state.baseline.source_point_entry.is_some()
            || state.baseline.unknown_entry
            || !state.command_observers.is_quiet()
            || !matches!(
                entry.source_string_protocol,
                Some(tcl_syntax::native_string::NativeStringProtocol::C(_))
            )
            || super::original_procedure_publication_key(native, state, context, facts).as_ref()
                != Some(&publication.slot)
            || state.original_initial_bindings_for_key(&publication.slot)?
                != std::collections::BTreeSet::from([super::MayBinding::Missing])
        {
            return None;
        }
        entry
            .empty_literal_world
            .as_ref()?
            .is_current_for(entry.interpreter, entry.epoch)
            .then_some(())?;
        let receipt = Self {
            entry: Arc::clone(entry),
            factory: native.target.clone(),
            allocation: publication.allocation.clone(),
            slot: publication.slot.clone(),
        };
        receipt.factory_is_current(state).then_some(receipt)
    }

    fn factory_is_current(&self, state: &ModuleCommandBindings) -> bool {
        let Some(runtime) = self.factory.identity.as_ref().and_then(|id| id.runtime) else {
            return false;
        };
        if runtime.interpreter != self.entry.interpreter
            || !state.retained_target_is_current(&self.factory)
            || state.source_execution_observed(self.factory.identity.as_ref())
        {
            return false;
        }
        let mut rows = self
            .entry
            .commands
            .iter()
            .filter(|row| row.token == runtime.token);
        let Some(row) = rows.next() else {
            return false;
        };
        rows.next().is_none()
            && !row.has_execution_trace
            && self.factory.runtime_implementation_generation == Some(row.implementation_generation)
            && matches!(&row.implementation,
                tcl_runtime_api::native_compilation::NativeCommandImplementation::Registry {
                    identity, ..
                } if tcl_registry::command_descriptor_labels_match(identity, &self.factory.command))
    }

    pub(super) fn retain_after_definition(
        self,
        native: SourceNativeInvocation<'_>,
        facts: &tcl_registry::InvocationFacts,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) {
        if !super::closed_procedure_definition(native, facts, state, context)
            || !self.current_entry(state)
        {
            return;
        }
        let Some(bindings) = state.bindings.get(&self.slot) else {
            return;
        };
        if bindings.len() != 1 {
            return;
        }
        let Some(super::MayBinding::Target(target)) = bindings.first() else {
            return;
        };
        if !target.terminal
            || target.registry_backed
            || target.kind != super::BindingKind::Proc
            || !target.prepended.is_empty()
            || target.implementation_generation != self.allocation.site.offset
            || target.implementation_allocation.as_ref() != Some(&self.allocation)
            || target.token.as_ref().is_none_or(|token| {
                token.runtime.is_some() || token.allocation.as_ref() != Some(&self.allocation)
            })
        {
            return;
        }
        Arc::make_mut(&mut state.command_observers)
            .fresh_procedures
            .insert(self.allocation.clone(), self);
    }

    fn current_entry(&self, state: &ModuleCommandBindings) -> bool {
        state.baseline.native_entry.as_deref() == Some(self.entry.as_ref())
            && !state.has_opaque_domain()
            && state.baseline.source_point_entry.is_none()
            && state.command_observers.is_quiet()
            && state
                .ordinary_literal_pool
                .as_ref()
                .is_some_and(|pool| pool.effects_current())
            && self
                .entry
                .empty_literal_world
                .as_ref()
                .is_some_and(|world| world.is_current_for(self.entry.interpreter, self.entry.epoch))
            && self.factory_is_current(state)
    }

    pub(super) fn admits(
        &self,
        state: &ModuleCommandBindings,
        target: &SourceCommandTarget,
    ) -> bool {
        if !self.current_entry(state)
            || target.registry_backed
            || target.kind != super::BindingKind::Proc
            || !target.prepended.is_empty()
            || target.implementation_generation != self.allocation.site.offset
            || target.implementation_allocation.as_ref() != Some(&self.allocation)
            || target.identity.as_ref().is_none_or(|token| {
                token.runtime.is_some() || token.allocation.as_ref() != Some(&self.allocation)
            })
            || state.source_execution_observed(target.identity.as_ref())
        {
            return false;
        }
        let Some(bindings) = state.bindings.get(&self.slot) else {
            return false;
        };
        bindings.len() == 1
            && bindings.iter().any(|binding| {
                matches!(binding,
            super::MayBinding::Target(current) if current.terminal
                && current.token == target.identity
                && current.command == target.command
                && current.implementation_generation == target.implementation_generation
                && current.implementation_allocation == target.implementation_allocation)
            })
    }
}

fn selected_publication(
    native: SourceNativeInvocation<'_>,
    facts: &tcl_registry::InvocationFacts,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<(
    String,
    super::SourceCommandKey,
    tcl_registry::CommandBindingDefinitionKind,
)> {
    let dialect = state.baseline.dialect?;
    if let Some(grammar) = context.registry.native_default_construction_grammar(
        &native.target.command,
        dialect,
        context.realm,
    ) {
        return selected_class_publication(native, facts, state, context, grammar);
    }
    selected_procedure_publication(native, facts, state, context)
}

fn selected_class_publication(
    native: SourceNativeInvocation<'_>,
    facts: &tcl_registry::InvocationFacts,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
) -> Option<(
    String,
    super::SourceCommandKey,
    tcl_registry::CommandBindingDefinitionKind,
)> {
    use tcl_registry::{CommandBindingDefinitionKind, StateTransition};
    let transitions = facts.state_transitions.declared()?;
    if !state.default_construction_dependencies_hold(grammar)
        || transitions.command_bindings().count() != 1
    {
        return None;
    }
    let name = transitions
        .facts()
        .iter()
        .find_map(|fact| match &fact.transition {
            StateTransition::ObjectDispatch(tcl_registry::ObjectDispatchTransition::Create {
                target: tcl_registry::ObjectDispatchTarget::Named(name),
                private_namespace: tcl_registry::ObjectPrivateNamespace::Fresh,
                kind: tcl_registry::ObjectDispatchKind::Class,
            }) => name.literal(),
            _ => None,
        })?;
    closed_definition_bodies(native, facts, state, context, grammar)?;
    Some((
        name.to_owned(),
        state.publication_key_at(
            &context.namespace_identity(),
            name,
            super::namespace_slots::PublicationPurpose::Command,
        )?,
        CommandBindingDefinitionKind::Object,
    ))
}

fn selected_procedure_publication(
    native: SourceNativeInvocation<'_>,
    facts: &tcl_registry::InvocationFacts,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<(
    String,
    super::SourceCommandKey,
    tcl_registry::CommandBindingDefinitionKind,
)> {
    use tcl_registry::{CommandBindingDefinitionKind, StateTransition};
    let dialect = state.baseline.dialect?;
    let transitions = facts.state_transitions.declared()?;
    let spec = context.registry.get_for_surface(
        &native.target.command,
        Some(dialect.authoring_query()?.with_realm(context.realm)),
    )?;
    if dialect.family() != Some(tcl_dialect::model::Family::Tcl)
        || dialect.tcl_version.is_none()
        || tcl_registry::registry::spec_pack_of(spec) != Some("tcl")
        || spec.procedure_definition
            != Some(tcl_registry::native_procedure::NativeProcedureDefinitionSpec::Core)
        || facts.operation
            != tcl_registry::SemanticOperationId::StructuredLowering(
                tcl_registry::hooks::LoweringHookId::Proc,
            )
        || super::native_procedure_parameters(facts, native.invocation.arguments()).is_none()
    {
        return None;
    }
    let Some(tcl_registry::native_procedure::NativeProcedureDefinitionSelection::Valid(definition)) =
        facts.procedure_definition
    else {
        return None;
    };
    if definition.statics_at.is_some() || native.invocation.arguments().exact_argv_len() != Some(3)
    {
        return None;
    }
    let [fact] = transitions.facts() else {
        return None;
    };
    let StateTransition::CommandBinding(tcl_registry::CommandBindingTransition::Define {
        name,
        kind: CommandBindingDefinitionKind::Procedure,
    }) = &fact.transition
    else {
        return None;
    };
    let name = name.literal()?;
    if native.invocation.arguments().literal_at(definition.name_at) != Some(name) {
        return None;
    }
    Some((
        name.to_owned(),
        state.publication_key_at(
            &context.namespace_identity(),
            name,
            super::namespace_slots::PublicationPurpose::Procedure,
        )?,
        CommandBindingDefinitionKind::Procedure,
    ))
}

fn closed_definition_bodies(
    native: SourceNativeInvocation<'_>,
    facts: &tcl_registry::InvocationFacts,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
) -> Option<()> {
    for (at, role) in &facts.arg_roles {
        if *role != tcl_registry::ArgRole::Body {
            continue;
        }
        let body = native
            .invocation
            .arguments()
            .literal_at(facts.argument_offset.checked_add(usize::from(*at))?)?;
        if !state.definition_has_only_deferred_members(body, grammar, context) {
            return None;
        }
        for words in super::definition_body_command_tokens(body, context.config) {
            for word in words.words() {
                super::literal_object_pool::SourceOrdinaryLiteralObject::capture_word(word, state)?;
            }
        }
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::super::{SourceAnalysisOptions, SourceCommandBindings, SourceVariableAccess};

    #[test]
    fn fresh_procedure_publication_preserves_only_unaffected_receiver_tables() {
        let owner = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let profile = owner.commands().profile().expect("actual C8.6 fixture");
        for (publication, preserved) in [
            ("proc pass {value} {return $value}", true),
            ("proc C args {return CUSTOM}", false),
            ("proc proc args {return CUSTOM}; proc pass {} {}", false),
            ("proc ::oo::define::method args {return CUSTOM}", false),
            (
                "oo::define ::oo::class constructor args {next {*}$args}",
                false,
            ),
            (
                "proc observe args {oo::define C method valid {} {return CHANGED}}; trace add execution proc leave observe; proc pass {} {}",
                false,
            ),
            ("set body [unknownOperation]; proc pass {} $body", false),
        ] {
            let source = format!(
                "oo::class create C {{method valid {{}} {{}}}}; set original [C new]; {publication}; $original valid"
            );
            let bindings = SourceCommandBindings::analyse_with_options(
                &source,
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                owner.commands(),
                SourceAnalysisOptions {
                    invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                    native_compilation: crate::environment_ingress::authoring_native_compilation(),
                    ..Default::default()
                },
            );
            let offset = u32::try_from(source.rfind("$original valid").unwrap()).unwrap();
            let proof = bindings
                .variable_accesses
                .get(&offset)
                .and_then(|reads| reads.first())
                .and_then(SourceVariableAccess::proved_object_instance);
            assert_eq!(proof.is_some(), preserved, "{source}");
            if let Some(proof) = proof {
                assert_eq!(proof.class_target().command, "::C");
                assert_eq!(
                    proof.allocation().site.offset,
                    u32::try_from(source.find("C new").unwrap()).unwrap()
                );
            }
        }
    }
}
