// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Registry-created instance source metadata retains its actual factory words.

use super::{
    AdviceCell, AdviceGraph, AdviceInvocation, AdviceInvocationContext,
    OriginalSourceCommandTransition, OriginalSourceCommandTransitionAdvice,
    OriginalSourceTransitionAdviceTape, SourceAdviceNameInput, SourceAdviceWord,
    SourceCommandTransitionObligation, original_words, registry_words,
};
use std::sync::Arc;
use tcl_core_types::{ByteCommandSlot, ByteNamespacePath};
use tcl_lexer::{ExecutablePart, LexerConfig, NativeWord, SourceImage};
use tcl_registry::handle_binding::{HandleClassSource, HandleName};
use tcl_registry::model::ContextRegistry;

/// A named factory's conditional source result, separate from an allocation.
/// The full selected constructor and original name survive moves and aliases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceRegisteredInstance {
    factory: Arc<OriginalSourceCommandTransitionAdvice>,
    name: SourceAdviceNameInput,
    slot: ByteCommandSlot,
    descriptor: &'static str,
    lineage: Vec<Arc<OriginalSourceCommandTransition>>,
}
impl OriginalSourceRegisteredInstance {
    /// Full selected original factory argv, context and applicability.
    #[must_use]
    pub fn factory(&self) -> &OriginalSourceCommandTransitionAdvice {
        &self.factory
    }
    /// Actual original naming operand, never a reconstructed class label.
    #[must_use]
    pub const fn name_input(&self) -> &SourceAdviceNameInput {
        &self.name
    }
    /// Conditional source command geometry, without a runtime slot grant.
    #[must_use]
    pub const fn source_slot(&self) -> &ByteCommandSlot {
        &self.slot
    }
    /// Complete original move and alias lineage for this source projection.
    #[must_use]
    pub fn lineage(&self) -> &[Arc<OriginalSourceCommandTransition>] {
        &self.lineage
    }
    /// Independently selected factory descriptor under the same full context.
    #[must_use]
    pub fn descriptor(
        &self,
        context: &ContextRegistry,
    ) -> Option<&'static tcl_registry::CommandSpec> {
        self.factory.matches_context(context).then_some(())?;
        let descriptor = context
            .context()
            .resolve_spec(context.commands(), self.descriptor)?;
        descriptor.object_class?;
        (descriptor.name == self.descriptor).then_some(descriptor)
    }
    pub(super) fn moved(
        &self,
        slot: ByteCommandSlot,
        step: &Arc<OriginalSourceCommandTransition>,
    ) -> Self {
        let mut moved = self.clone();
        moved.slot = slot;
        moved.lineage.push(Arc::clone(step));
        moved
    }
}

/// Original setter and factory substitution supplying a conditional handle.
/// This records authored layout; it grants no stored value or variable cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceRegisteredHandleBinding {
    setter: Arc<OriginalSourceCommandTransitionAdvice>,
    layout: tcl_registry::handle_binding::HandleBindingSpec,
    variable: SourceAdviceNameInput,
    operand: NativeWord,
    instance: Arc<OriginalSourceRegisteredInstance>,
}
impl OriginalSourceRegisteredHandleBinding {
    /// Actual whole original setter argv and selected source schema.
    #[must_use]
    pub fn setter(&self) -> &OriginalSourceCommandTransitionAdvice {
        &self.setter
    }
    /// Independently selected Registry handle-binding layout.
    #[must_use]
    pub const fn layout(&self) -> tcl_registry::handle_binding::HandleBindingSpec {
        self.layout
    }
    /// Actual original variable operand, separate from any cell identity.
    #[must_use]
    pub const fn variable_input(&self) -> &SourceAdviceNameInput {
        &self.variable
    }
    /// Whole substitution container, preserving grouping and channel.
    #[must_use]
    pub const fn original_operand(&self) -> &NativeWord {
        &self.operand
    }
    /// Actual selected factory and conditional result, without allocation.
    #[must_use]
    pub fn instance(&self) -> &OriginalSourceRegisteredInstance {
        &self.instance
    }
    /// Same full source, configuration, Registry and availability context.
    #[must_use]
    pub fn matches_source_context(
        &self,
        image: &SourceImage,
        config: LexerConfig,
        context: &ContextRegistry,
    ) -> bool {
        self.setter.matches_source(image, config)
            && self.setter.matches_context(context)
            && self.instance.factory.matches_source(image, config)
            && self.instance.factory.matches_context(context)
            && self.operand.image() == image
            && self.operand.config() == config
    }
}

/// Source-only instance method words joined to their exact original factory.
/// The receipt retains conditional applicability rather than an entered target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceRegisteredInstanceWords {
    site: super::super::CommandAllocationSite,
    original: Arc<[NativeWord]>,
    arguments: Vec<SourceAdviceWord>,
    instance: Arc<OriginalSourceRegisteredInstance>,
    handle: Option<Arc<OriginalSourceRegisteredHandleBinding>>,
    source_body: Option<Arc<crate::registry_invocation::OriginalSourceScriptBody>>,
    obligations: Vec<SourceCommandTransitionObligation>,
}
impl OriginalSourceRegisteredInstanceWords {
    /// Whole actual written invocation vector, including its receiver.
    #[must_use]
    pub fn original_words(&self) -> &[NativeWord] {
        &self.original
    }
    /// Original lexical occurrence, independent of runtime dispatch.
    #[must_use]
    pub const fn site(&self) -> &super::super::CommandAllocationSite {
        &self.site
    }
    /// Same exact factory source identity and current authored lineage.
    #[must_use]
    pub fn instance(&self) -> &OriginalSourceRegisteredInstance {
        &self.instance
    }
    /// Original handle setter when the receiver uses a variable substitution.
    #[must_use]
    pub fn handle_binding(&self) -> Option<&OriginalSourceRegisteredHandleBinding> {
        self.handle.as_deref()
    }
    /// Authentic selected source body when this occurrence is deferred.
    /// This preserves parent syntax, without an entered frame or namespace.
    #[must_use]
    pub fn source_body(&self) -> Option<&crate::registry_invocation::OriginalSourceScriptBody> {
        self.source_body.as_deref()
    }
    /// Every unresolved execution and source-factory premise remains explicit.
    #[must_use]
    pub fn obligations(&self) -> &[SourceCommandTransitionObligation] {
        &self.obligations
    }
    /// Effective method argv's retained original word at the selected ordinal.
    #[must_use]
    pub fn argument_word(&self, ordinal: usize) -> Option<&NativeWord> {
        Some(&self.arguments.get(ordinal)?.original)
    }
    /// Genuine original method operand policy, without promoting Logical units.
    #[must_use]
    pub fn argument_input(&self, ordinal: usize) -> Option<&SourceAdviceNameInput> {
        self.arguments.get(ordinal)?.input.as_ref()
    }
    /// Exact source-produced method/argument units when independently static.
    #[must_use]
    pub fn argument_bytes(&self, ordinal: usize) -> Option<&[u8]> {
        self.arguments.get(ordinal)?.value.as_deref()
    }
    /// Expanded source markers cannot supply a concrete argc.
    #[must_use]
    pub fn exact_argument_count(&self) -> Option<usize> {
        self.arguments
            .iter()
            .all(|word| !word.original.group().expand)
            .then_some(self.arguments.len())
    }
    /// Whole source image, full configuration and selected availability agree.
    #[must_use]
    pub fn matches_source_context(
        &self,
        image: &SourceImage,
        config: LexerConfig,
        context: &ContextRegistry,
    ) -> bool {
        self.site.source.source_image() == image
            && self
                .original
                .iter()
                .all(|word| word.image() == image && word.config() == config)
            && self.instance.factory.matches_source(image, config)
            && self.instance.factory.matches_context(context)
            && self.source_body.as_ref().is_none_or(|body| {
                body.matches_source(image, config) && body.matches_context(context)
            })
            && self
                .handle
                .as_ref()
                .is_none_or(|handle| handle.matches_source_context(image, config, context))
    }
    /// Borrow the method schema from the retained selected descriptor. No
    /// lookup of reporting class strings or inherited constructor effects occurs.
    #[must_use]
    pub fn with_source_schema<'r, T>(
        &'r self,
        context: &'r ContextRegistry,
        project: impl FnOnce(&tcl_registry::ResolvedInvocation<'r, '_>) -> T,
    ) -> Option<T> {
        let factory = self.instance.descriptor(context)?;
        let arguments = registry_words(&self.arguments);
        let floor = factory
            .required_package
            .and_then(|package| context.context().placement_floor(package));
        let floor = floor.map(tcl_dialect::model::Version::as_str);
        let selected = context
            .commands()
            .resolve_structured_instance_invocation_for_descriptor(
                factory,
                tcl_registry::InvocationWords::structured(
                    tcl_registry::InvocationWord::Dynamic,
                    &arguments,
                )
                .with_dialect(self.instance.factory.dialect()),
                floor,
                Some(context.context().authoring_query()),
            )?;
        Some(project(&selected))
    }
    /// Shared source keyword classification under the retained factory context.
    /// Opaque, NUL-containing or expanded selectors have no source projection.
    #[must_use]
    pub fn method_selection(
        &self,
        context: &ContextRegistry,
    ) -> Option<tcl_registry::abbrev::KeywordMatch<'static>> {
        let bytes = self.argument_bytes(0)?;
        if !bytes.is_ascii() || bytes.contains(&0) || self.argument_word(0)?.group().expand {
            return None;
        }
        let spelling = std::str::from_utf8(bytes).ok()?;
        let factory = self.instance.descriptor(context)?;
        let floor = factory
            .required_package
            .and_then(|package| context.context().placement_floor(package))
            .map(tcl_dialect::model::Version::as_str);
        context
            .commands()
            .instance_method_selection_for_descriptor_at(
                factory,
                spelling,
                floor,
                Some(context.context().authoring_query()),
            )
    }
    /// Authored visible method table under the same selected full context.
    #[must_use]
    pub fn methods(
        &self,
        context: &ContextRegistry,
    ) -> Option<Vec<&'static tcl_registry::SubCommand>> {
        let factory = self.instance.descriptor(context)?;
        let floor = factory
            .required_package
            .and_then(|package| context.context().placement_floor(package));
        let floor = floor.map(tcl_dialect::model::Version::as_str);
        Some(context.commands().instance_methods_for_descriptor_at(
            factory,
            floor,
            Some(context.context().authoring_query()),
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SourceHandleKey {
    Native(tcl_syntax::naming::NativeVariableRootGeometry),
    Logical(ByteCommandSlot),
}
impl AdviceGraph {
    /// Conditional Registry naming-layout candidate, not actual publication.
    /// Jim table comparison delegates the native counted owner; other source
    /// recipes retain the bounded ASCII authored geometry. Unknown factory
    /// publication cannot select a guessed native registration entry point.
    fn instance_key(&self, bytes: &[u8]) -> Option<ByteCommandSlot> {
        if bytes.is_empty() || bytes.contains(&0) {
            return None;
        }
        match self.policy.native() {
            Some(policy) if policy.recipe().is_jim084() => policy
                .recipe()
                .command_lookup_slot(super::root_context(policy), bytes)
                .ok(),
            _ => tcl_syntax::naming::authored_source_command_slot(
                self.logical_namespace
                    .as_ref()
                    .unwrap_or(&ByteNamespacePath::root()),
                bytes,
            ),
        }
    }
    pub(super) fn handle_key(&self, bytes: &[u8]) -> Option<SourceHandleKey> {
        if bytes.is_empty() || bytes.contains(&0) {
            return None;
        }
        if let Some(policy) = self.policy.native() {
            let projection = policy.recipe().combined_variable_input(bytes);
            if projection.element().is_some() {
                return None;
            }
            Some(SourceHandleKey::Native(
                policy
                    .recipe()
                    .variable_root_geometry(super::root_context(policy), bytes),
            ))
        } else {
            Some(SourceHandleKey::Logical(
                tcl_syntax::naming::authored_source_command_slot(
                    self.logical_namespace.as_ref()?,
                    bytes,
                )?,
            ))
        }
    }
    fn resolve_instance(
        &self,
        head: &SourceAdviceNameInput,
    ) -> Option<(Arc<OriginalSourceRegisteredInstance>, Vec<SourceAdviceWord>)> {
        let mut key = self.lookup_head_key(head.bytes())?;
        let mut seen = Vec::new();
        let mut prefix = Vec::new();
        let mut lineage = Vec::new();
        loop {
            if seen.contains(&key) {
                return None;
            }
            seen.push(key.clone());
            match self.cells.get(&key)? {
                AdviceCell::RegisteredInstance(instance) => {
                    let mut instance = instance.as_ref().clone();
                    instance.lineage.extend(lineage);
                    return Some((Arc::new(instance), prefix));
                }
                AdviceCell::Alias {
                    target,
                    prefix: captured,
                    lookup,
                    lineage: steps,
                } => {
                    let mut next = captured.clone();
                    next.extend(prefix);
                    prefix = next;
                    lineage.extend(steps.iter().cloned());
                    key = self.alias_target_key(target, *lookup)?;
                }
                _ => return None,
            }
        }
    }
}
impl OriginalSourceTransitionAdviceTape {
    pub(crate) fn registered_factory(
        &self,
        offset: u32,
    ) -> Option<&OriginalSourceRegisteredInstance> {
        self.registered_factories.get(&offset)?.as_deref()
    }
    pub(crate) fn registered_instance(
        &self,
        offset: u32,
    ) -> Option<&OriginalSourceRegisteredInstanceWords> {
        self.registered_instances.get(&offset)?.as_ref()
    }
    pub(crate) fn registered_handle(
        &self,
        offset: u32,
    ) -> Option<&OriginalSourceRegisteredHandleBinding> {
        self.registered_handles.get(&offset)?.as_deref()
    }
    fn retain_registered_instance(&mut self, receipt: OriginalSourceRegisteredInstanceWords) {
        self.registered_instances
            .entry(receipt.site.offset)
            .and_modify(|previous| {
                if previous.as_ref() != Some(&receipt) {
                    *previous = None;
                }
            })
            .or_insert(Some(receipt));
    }
}
impl AdviceInvocationContext<'_> {
    pub(super) fn before_operand_handle_binding(
        &self,
        graph: &AdviceGraph,
        native: &[NativeWord],
        written: &[SourceAdviceWord],
    ) -> Option<Arc<OriginalSourceRegisteredHandleBinding>> {
        self.with_before_operand_handle_schema(graph, native, written, |invocation, schema| {
            self.registered_handle_binding(graph, invocation, schema)
        })
    }
    pub(super) fn with_before_operand_handle_schema<T>(
        &self,
        graph: &AdviceGraph,
        native: &[NativeWord],
        written: &[SourceAdviceWord],
        capture: impl FnOnce(
            AdviceInvocation<'_>,
            &tcl_registry::ResolvedInvocation<'_, '_>,
        ) -> Option<T>,
    ) -> Option<T> {
        let head = written.first()?.input.as_ref()?;
        let (command, mut arguments, lineage) = graph.resolve(head)?;
        for (ordinal, word) in arguments.iter_mut().enumerate() {
            word.origin = crate::registry_invocation::InvocationWordOrigin::BindingPrefix(ordinal);
        }
        arguments.extend(written.iter().skip(1).cloned());
        let values = registry_words(&arguments);
        let resolution =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                self.context.commands(),
                Some(self.context.context()),
                tcl_registry::InvocationWords::structured(
                    tcl_registry::InvocationWord::Literal(&command),
                    &values,
                )
                .with_dialect(self.dialect),
                tcl_dialect::model::InvocationRealm::RuleLoader,
            );
        let schema = resolution.resolved()?;
        capture(
            AdviceInvocation {
                native,
                head,
                arguments: &arguments,
                lineage: &lineage,
            },
            &schema,
        )
    }
    pub(super) fn finish_registered_invocation(
        &self,
        graph: &mut AdviceGraph,
        native: &[NativeWord],
        written: &[SourceAdviceWord],
    ) {
        // Independently close only a selected callback-free naming footprint.
        // A readonly source receipt itself is never an effects or Normal grant.
        let mut capture = OriginalSourceTransitionAdviceTape::default();
        let quiet = self.retain_registered_invocation(&mut capture, graph, native, written)
            && capture
                .registered_instance(native[0].span().start())
                .is_some_and(|receipt| {
                    native.iter().all(|word| {
                        word.executable_parts()
                            .all_parts()
                            .all(|part| matches!(part.part, ExecutablePart::Text(_)))
                    }) && receipt.with_source_schema(self.context, |schema| {
                        use tcl_registry::world_effect::{
                            CallbackKinds, EffectAccessMode, WorldStateDomain,
                        };
                        let effects = schema.effect_footprint();
                        effects.callback().kinds == CallbackKinds::NONE
                            && effects.accesses().iter().all(|access| {
                                access.mode == EffectAccessMode::Read
                                    || !matches!(
                                        access.domain,
                                        WorldStateDomain::CommandBindings
                                            | WorldStateDomain::NamespaceLookup
                                            | WorldStateDomain::NamespaceUnknown
                                            | WorldStateDomain::VariableStore
                                            | WorldStateDomain::PackageState
                                            | WorldStateDomain::InterpreterPolicy
                                    )
                            })
                    }) == Some(true)
                });
        if !quiet {
            graph.widen(native);
        }
    }
    pub(super) fn registered_factory(
        &self,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> Option<Arc<OriginalSourceRegisteredInstance>> {
        // naming.source.original-registered-instance-words
        // docs/design/analysis/name-resolution-proofs/source-original-registered-instance-words.md
        let descriptor = schema.authored_source_descriptors().command;
        descriptor.object_class?;
        // Optional constructor control words need their own authored layout.
        // This bounded named-form receipt requires a mandatory naming operand.
        if descriptor.arity.min == 0
            || schema.facts().arity_accepts_frozen_arguments() != Some(true)
        {
            return None;
        }
        let ordinal = usize::from(descriptor.creates_instance_at?);
        let name = invocation.arguments.get(ordinal)?.input.as_ref()?.clone();
        if invocation.native.iter().any(|word| word.group().expand) {
            return None;
        }
        let slot = graph.instance_key(name.bytes())?;
        graph.namespaces.contains(&slot.namespace).then_some(())?;
        if graph
            .cells
            .get(&slot)
            .is_some_and(|cell| !matches!(cell, AdviceCell::Deleted))
        {
            return None;
        }
        let mut factory = self.schema_advice(invocation, graph, schema)?;
        factory
            .obligations
            .push(SourceCommandTransitionObligation::RegisteredFactoryApplicability);
        Some(Arc::new(OriginalSourceRegisteredInstance {
            factory: Arc::new(factory),
            name,
            slot,
            descriptor: descriptor.name,
            lineage: Vec::new(),
        }))
    }
    pub(super) fn registered_handle_binding(
        &self,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> Option<Arc<OriginalSourceRegisteredHandleBinding>> {
        let layout = *schema.authored_source_descriptors().command.binds_handle?;
        let HandleName::Word(variable) = layout.name_from else {
            return None;
        };
        let HandleClassSource::ConstructionValue(value) = layout.class_from else {
            return None;
        };
        if let Some(keyword) = layout.keyword
            && invocation
                .arguments
                .get(usize::from(keyword.at))?
                .value
                .as_deref()
                != Some(keyword.word.as_bytes())
        {
            return None;
        }
        if schema.facts().arity_accepts_frozen_arguments() != Some(true)
            || invocation.native.iter().any(|word| word.group().expand)
        {
            return None;
        }
        let variable = invocation
            .arguments
            .get(usize::from(variable))?
            .input
            .as_ref()?
            .clone();
        graph.handle_key(variable.bytes())?;
        let rhs = invocation.arguments.get(usize::from(value))?;
        // A captured alias prefix was evaluated when the alias was defined.
        // Only the actual current written RHS can supply this construction.
        if !matches!(
            rhs.origin,
            crate::registry_invocation::InvocationWordOrigin::Written(_)
        ) || !invocation.native.contains(&rhs.original)
        {
            return None;
        }
        let operand = &rhs.original;
        let substitution = super::original_single_command_substitution_words(operand)?;
        let mut written = original_words(&substitution.command().words, self.policy)?;
        let head = written.first()?.input.as_ref()?.clone();
        let (command, mut arguments, lineage) = graph.resolve(&head)?;
        for (ordinal, argument) in arguments.iter_mut().enumerate() {
            argument.origin =
                crate::registry_invocation::InvocationWordOrigin::BindingPrefix(ordinal);
        }
        arguments.extend(written.drain(1..));
        let values = registry_words(&arguments);
        let resolution =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                self.context.commands(),
                Some(self.context.context()),
                tcl_registry::InvocationWords::structured(
                    tcl_registry::InvocationWord::Literal(&command),
                    &values,
                )
                .with_dialect(self.dialect),
                tcl_dialect::model::InvocationRealm::RuleLoader,
            );
        let schema_factory = resolution.resolved()?;
        let instance = self.registered_factory(
            graph,
            AdviceInvocation {
                native: &substitution.command().words,
                head: &head,
                arguments: &arguments,
                lineage: &lineage,
            },
            &schema_factory,
        )?;
        let mut setter = self.schema_advice(invocation, graph, schema)?;
        setter
            .obligations
            .push(SourceCommandTransitionObligation::RegisteredHandleBindingApplicability);
        Some(Arc::new(OriginalSourceRegisteredHandleBinding {
            setter: Arc::new(setter),
            layout,
            variable,
            operand: operand.clone(),
            instance,
        }))
    }
    pub(super) fn retain_registered_invocation(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        native: &[NativeWord],
        written: &[SourceAdviceWord],
    ) -> bool {
        let Some(receiver) = written.first() else {
            return false;
        };
        let selected = if let Some(head) = &receiver.input {
            graph
                .resolve_instance(head)
                .map(|(instance, prefix)| (instance, prefix, None))
        } else {
            let arena = receiver.original.executable_parts();
            let [part] = arena.list(arena.root()) else {
                return false;
            };
            let ExecutablePart::Variable { name, index: None } = part.part else {
                return false;
            };
            let Some(key) = arena.bytes(name).and_then(|bytes| graph.handle_key(bytes)) else {
                return false;
            };
            graph
                .handles
                .iter()
                .find(|(owned, _)| owned == &key)
                .and_then(|(_, handle)| {
                    let AdviceCell::RegisteredInstance(current) =
                        graph.cells.get(handle.instance.source_slot())?
                    else {
                        return None;
                    };
                    (current.factory == handle.instance.factory).then(|| {
                        (
                            Arc::new(current.as_ref().clone()),
                            Vec::new(),
                            Some(Arc::clone(handle)),
                        )
                    })
                })
        };
        let Some((instance, mut arguments, handle)) = selected else {
            return false;
        };
        for (ordinal, argument) in arguments.iter_mut().enumerate() {
            argument.origin =
                crate::registry_invocation::InvocationWordOrigin::BindingPrefix(ordinal);
        }
        arguments.extend(written.iter().skip(1).cloned());
        let mut obligations = instance.factory.obligations.clone();
        if graph.procedure_body.is_some() {
            obligations.push(SourceCommandTransitionObligation::OriginalProcedureBodyApplicability);
        }
        if graph.future_procedure_source {
            obligations.push(
                SourceCommandTransitionObligation::FutureOriginalProcedureSourceApplicability,
            );
        }
        if handle.is_some() {
            obligations
                .push(SourceCommandTransitionObligation::RegisteredHandleBindingApplicability);
        }
        let current_callbacks = native.iter().any(|word| {
            word.executable_parts()
                .all_parts()
                .any(|part| !matches!(part.part, ExecutablePart::Text(_)))
        });
        if (current_callbacks || !graph.uncertain_operations.is_empty())
            && !obligations.contains(&SourceCommandTransitionObligation::UnknownEarlierMutation)
        {
            obligations.push(SourceCommandTransitionObligation::UnknownEarlierMutation);
        }
        let receipt = OriginalSourceRegisteredInstanceWords {
            site: super::super::CommandAllocationSite {
                source: Arc::clone(self.origin),
                offset: native.first().map_or(0, |word| word.span().start()),
            },
            original: Arc::from(native),
            arguments,
            instance,
            handle,
            source_body: graph.procedure_body.clone(),
            obligations,
        };
        self.retain_registered_callback_targets(tape, graph, &receipt);
        tape.retain_registered_instance(receipt);
        true
    }
    pub(super) fn retain_registered_body_owner(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        offset: u32,
        body: &Arc<crate::registry_invocation::OriginalSourceScriptBody>,
    ) {
        if let Some(Some(receipt)) = tape.registered_instances.get_mut(&offset)
            && body.matches_source(self.origin.source_image(), self.config)
            && body.matches_context(self.context)
        {
            receipt.source_body = Some(Arc::clone(body));
            receipt
                .obligations
                .push(SourceCommandTransitionObligation::DeferredLogicalBodyApplicability);
        }
    }
    pub(super) fn install_registered_results(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &mut AdviceGraph,
        factory: Option<Arc<OriginalSourceRegisteredInstance>>,
        handle: Option<Arc<OriginalSourceRegisteredHandleBinding>>,
    ) {
        if let Some(instance) = &factory
            && instance.factory.matches_context(self.context)
        {
            super::source_class::merge_receipt(
                &mut tape.registered_factories,
                instance.factory.site().offset,
                Some(Arc::clone(instance)),
            );
        }
        if let Some(instance) = factory
            && instance.factory.matches_context(self.context)
            && !graph
                .cells
                .get(&instance.slot)
                .is_some_and(|cell| !matches!(cell, AdviceCell::Deleted))
        {
            graph.cells.insert(
                instance.slot.clone(),
                AdviceCell::RegisteredInstance(instance),
            );
        }
        if let Some(handle) = handle
            && handle.setter.matches_context(self.context)
            && handle.instance.factory.matches_context(self.context)
            && !graph
                .cells
                .get(&handle.instance.slot)
                .is_some_and(|cell| !matches!(cell, AdviceCell::Deleted))
            && let Some(key) = graph.handle_key(handle.variable.bytes())
        {
            graph.handles.retain(|(owned, _)| owned != &key);
            graph.cells.insert(
                handle.instance.slot.clone(),
                AdviceCell::RegisteredInstance(Arc::clone(&handle.instance)),
            );
            graph.handles.push((key, Arc::clone(&handle)));
            let offset = handle.setter.site().offset;
            tape.registered_handles
                .entry(offset)
                .and_modify(|previous| {
                    if previous.as_deref() != Some(handle.as_ref()) {
                        *previous = None;
                    }
                })
                .or_insert(Some(handle));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::analyser::Analyser;
    use crate::registry_invocation::source_structure::{
        source_registered_handle_binding_at, source_registered_instance_words_at,
    };

    fn at(source: &str, marker: &str) -> u32 {
        u32::try_from(source.rfind(marker).unwrap()).unwrap()
    }

    #[test]
    fn original_registered_instance_keeps_factory_argv_alias_move_and_method_prefix() {
        // naming.source.original-registered-instance-words
        // docs/design/analysis/name-resolution-proofs/source-original-registered-instance-words.md
        let source = "rename ttk::treeview make; make .t; rename .t .held; interp alias {} dispatch {} .held; dispatch mov onlyone";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let words =
            source_registered_instance_words_at(source, &analysis, at(source, "dispatch mov"))
                .expect("selected actual source factory");
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        assert_eq!(words.instance().factory().command(), "ttk::treeview");
        assert_eq!(words.instance().name_input().bytes(), b".t");
        assert_eq!(words.instance().source_slot().simple.as_bytes(), b".held");
        assert_eq!(words.instance().lineage().len(), 2);
        assert_eq!(words.original_words().len(), 3);
        assert_eq!(words.argument_bytes(0), Some(&b"mov"[..]));
        assert_eq!(
            words.method_selection(&context).unwrap().unique(),
            Some("move")
        );
        assert_eq!(
            words.with_source_schema(&context, |schema| schema
                .facts()
                .arity_accepts_frozen_arguments()),
            Some(Some(false))
        );
        assert!(
            words.obligations().contains(
                &super::SourceCommandTransitionObligation::RegisteredFactoryApplicability
            )
        );
        assert!(
            source_registered_instance_words_at(
                &source.replace("onlyone", "other"),
                &analysis,
                at(source, "dispatch mov")
            )
            .is_none()
        );
        let mut changed = words.original_words().to_vec();
        changed.pop();
        assert!(
            analysis
                .retained_command_realm()
                .unwrap()
                .original_source_registered_instance_words(
                    analysis.resolved_input.as_ref().unwrap(),
                    &changed
                )
                .is_none()
        );
    }

    #[test]
    fn original_registered_handle_retains_whole_setter_and_factory_without_value_grant() {
        // naming.source.original-registered-instance-words
        // docs/design/analysis/name-resolution-proofs/source-original-registered-instance-words.md
        let source = "::set lb [listbox .l]; $lb curselection extra";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let handle = source_registered_handle_binding_at(source, &analysis, 0)
            .expect("whole original setter");
        assert_eq!(handle.variable_input().bytes(), b"lb");
        assert_eq!(handle.setter().original_words().len(), 3);
        assert_eq!(handle.instance().factory().original_words().len(), 2);
        assert_eq!(
            handle
                .original_operand()
                .executable_parts()
                .all_parts()
                .filter(|part| matches!(part.part, tcl_lexer::ExecutablePart::Command { .. }))
                .count(),
            1
        );
        let words = source_registered_instance_words_at(source, &analysis, at(source, "$lb"))
            .expect("same actual handle source binding");
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        assert_eq!(words.handle_binding(), Some(&handle));
        assert_eq!(
            words.with_source_schema(&context, |schema| schema
                .facts()
                .arity_accepts_frozen_arguments()),
            Some(Some(false))
        );
        assert!(words.obligations().contains(
            &super::SourceCommandTransitionObligation::RegisteredHandleBindingApplicability
        ));
    }

    #[test]
    fn original_registered_instance_refuses_known_replacement_deletion_and_unowned_factory() {
        // naming.source.original-registered-instance-words
        // docs/design/analysis/name-resolution-proofs/source-original-registered-instance-words.md
        for source in [
            "proc ttk::treeview {args} {}; ttk::treeview .t; .t bogus",
            "ttk::treeview .t; proc .t {args} {}; .t bogus",
            "ttk::treeview .t; rename .t {}; .t bogus",
            "ttk::treeview $name; .t bogus",
            "set lb [listbox .l]; set lb other; $lb bogus",
            "interp alias {} keep {} set lb [listbox .l]; keep; $lb bogus",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let offset = source.rfind(';').unwrap() + 2;
            assert!(
                source_registered_instance_words_at(
                    source,
                    &analysis,
                    u32::try_from(offset).unwrap()
                )
                .is_none(),
                "{source}"
            );
        }
        let source = "ttk::treeview .t; .t {*}$method";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let words =
            source_registered_instance_words_at(source, &analysis, at(source, ".t {*}")).unwrap();
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        assert!(words.method_selection(&context).is_none());
        assert!(words.exact_argument_count().is_none());
    }
    #[test]
    fn original_registered_instance_unknown_mutation_retains_only_conditional_factory_advice() {
        // naming.source.original-registered-instance-words
        // docs/design/analysis/name-resolution-proofs/source-original-registered-instance-words.md
        // The original factory remains source advice; no current widget or command is proved.
        let source = "ttk::treeview .t; unknown; .t bogus";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let words = source_registered_instance_words_at(source, &analysis, at(source, ".t bogus"))
            .expect("conditional original factory survives unknown source widening");
        assert_eq!(words.instance().factory().command(), "ttk::treeview");
        assert_eq!(words.instance().name_input().bytes(), b".t");
        assert_eq!(words.original_words().len(), 2);
        assert!(words.handle_binding().is_none());
        assert!(words.obligations().contains(
            &crate::command_binding::SourceCommandTransitionObligation::UnknownEarlierMutation
        ));
        assert!(words.obligations().contains(
            &crate::command_binding::SourceCommandTransitionObligation::UnavailableActualLookup
        ));
        let source = "set lb [listbox .l]; unknown; $lb bogus";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            source_registered_instance_words_at(source, &analysis, at(source, "$lb bogus"))
                .is_none()
        );
    }
    #[test]
    fn original_registered_factory_candidate_keys_delegate_jim_comparison_without_publication() {
        // naming.source.original-registered-instance-words
        // docs/design/analysis/name-resolution-proofs/source-original-registered-instance-words.md
        // This checks bounded candidate geometry, not a native factory or source receipt.
        for dialect_name in ["jim", "tcl8.6"] {
            let analysis = Analyser::new().analyse("", dialect_name);
            let input = analysis.resolved_input.as_ref().unwrap();
            let dialect = tcl_registry::InvocationDialect::of_profile(
                tcl_dialect::DialectProfile::find(dialect_name).unwrap(),
            );
            let policy =
                super::super::AdviceNamingPolicy::Native(dialect.authored_name_policy().unwrap());
            let graph = super::super::AdviceGraph::new(&input.context_registry(), &policy, dialect)
                .unwrap();
            assert!(graph.instance_key(b"").is_none());
            assert!(graph.instance_key(b"db\0tail").is_none());
            if dialect_name == "jim" {
                assert_eq!(
                    graph.instance_key(b"::::db").unwrap().simple.as_bytes(),
                    b"db"
                );
                assert_eq!(
                    graph
                        .instance_key("db α".as_bytes())
                        .unwrap()
                        .simple
                        .as_bytes(),
                    "db α".as_bytes()
                );
                assert_ne!(
                    graph.instance_key("é".as_bytes()),
                    graph.instance_key("e\u{301}".as_bytes())
                );
            } else {
                assert!(graph.instance_key("db α".as_bytes()).is_none());
                assert_eq!(graph.instance_key(b"db").unwrap().simple.as_bytes(), b"db");
            }
        }
    }
}
