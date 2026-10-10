// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original source-class construction and conditional receiver words.

use super::registered_instance::SourceHandleKey;
use super::{
    AdviceCell, AdviceGraph, AdviceInvocation, AdviceInvocationContext, AdviceNamingPolicy,
    AdvicePublicationPurpose, OriginalSourceClassDeclaration, OriginalSourceCommandTransition,
    OriginalSourceCommandTransitionAdvice, OriginalSourceConstructorCall,
    OriginalSourceTransitionAdviceTape, SourceAdviceNameInput, SourceAdviceWord,
    SourceCommandTransitionObligation,
};
use crate::registry_invocation::InvocationWordOrigin;
use crate::registry_invocation::OriginalSourceScriptBody;
use std::sync::Arc;
use tcl_core_types::ByteCommandSlot;
use tcl_lexer::{ExecutablePart, LexerConfig, NativeWord, SourceImage};
use tcl_registry::{
    handle_binding::{HandleBindingSpec, HandleClassSource, HandleName},
    model::ContextRegistry,
};

/// Genuine original constructor syntax and its conditional named result.
/// Anonymous construction has no manufactured command name or byte slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceClassInstance {
    constructor: Arc<OriginalSourceConstructorCall>,
    name: Option<SourceAdviceNameInput>,
    slot: Option<ByteCommandSlot>,
    lineage: Vec<Arc<OriginalSourceCommandTransition>>,
}
impl OriginalSourceClassInstance {
    /// Complete authentic constructor call and selected class declaration.
    #[must_use]
    pub fn constructor(&self) -> &OriginalSourceConstructorCall {
        &self.constructor
    }
    /// Same canonical original source class; reporting names are not identities.
    #[must_use]
    pub fn class_declaration(&self) -> &OriginalSourceClassDeclaration {
        self.constructor.class_declaration()
    }
    /// Genuine constructor naming operand, absent for anonymous construction.
    #[must_use]
    pub const fn name_input(&self) -> Option<&SourceAdviceNameInput> {
        self.name.as_ref()
    }
    /// Conditional publication geometry only; no allocated object is supplied.
    #[must_use]
    pub const fn source_slot(&self) -> Option<&ByteCommandSlot> {
        self.slot.as_ref()
    }
    /// Original source moves and aliases, independently of runtime success.
    #[must_use]
    pub fn lineage(&self) -> &[Arc<OriginalSourceCommandTransition>] {
        &self.lineage
    }
    pub(super) fn moved(
        &self,
        slot: ByteCommandSlot,
        step: &Arc<OriginalSourceCommandTransition>,
    ) -> Self {
        let mut moved = self.clone();
        moved.slot = Some(slot);
        moved.lineage.push(Arc::clone(step));
        moved
    }
}

/// Exact selected setter and whole constructor substitution, without a cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceClassHandleBinding {
    setter: Arc<OriginalSourceCommandTransitionAdvice>,
    layout: HandleBindingSpec,
    variable: SourceAdviceNameInput,
    operand: NativeWord,
    instance: Arc<OriginalSourceClassInstance>,
}
impl OriginalSourceClassHandleBinding {
    /// Whole original setter argv and source applicability.
    #[must_use]
    pub fn setter(&self) -> &OriginalSourceCommandTransitionAdvice {
        &self.setter
    }
    /// Genuine selected setter layout, without an evaluated result.
    #[must_use]
    pub const fn layout(&self) -> HandleBindingSpec {
        self.layout
    }
    /// Original variable source operand, without variable-cell identity.
    #[must_use]
    pub const fn variable_input(&self) -> &SourceAdviceNameInput {
        &self.variable
    }
    /// Whole original single-command substitution container.
    #[must_use]
    pub const fn original_operand(&self) -> &NativeWord {
        &self.operand
    }
    /// Original constructed source value, never a stored object handle.
    #[must_use]
    pub fn instance(&self) -> &OriginalSourceClassInstance {
        &self.instance
    }
    /// Same full immutable source and selected Registry context.
    #[must_use]
    pub fn matches_source_context(
        &self,
        image: &SourceImage,
        config: LexerConfig,
        context: &ContextRegistry,
    ) -> bool {
        self.setter.matches_source(image, config)
            && self.setter.matches_context(context)
            && self.operand.image() == image
            && self.operand.config() == config
            && self
                .instance
                .constructor
                .matches_source_context(image, config, context)
    }
}

/// Whole later source receiver call joined to genuine constructor/setter words.
/// This retains conditional source advice, never dispatch or object lifetime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceClassInstanceWords {
    site: super::super::CommandAllocationSite,
    original: Arc<[NativeWord]>,
    arguments: Vec<SourceAdviceWord>,
    instance: Arc<OriginalSourceClassInstance>,
    handle: Option<Arc<OriginalSourceClassHandleBinding>>,
    source_body: Option<Arc<OriginalSourceScriptBody>>,
    obligations: Vec<SourceCommandTransitionObligation>,
}
impl OriginalSourceClassInstanceWords {
    /// Actual source occurrence, without an invented native invocation point.
    #[must_use]
    pub const fn site(&self) -> &super::super::CommandAllocationSite {
        &self.site
    }
    /// Whole written receiver vector, including its head and expansion syntax.
    #[must_use]
    pub fn original_words(&self) -> &[NativeWord] {
        &self.original
    }
    /// Genuine original class construction and conditional naming lineage.
    #[must_use]
    pub fn instance(&self) -> &OriginalSourceClassInstance {
        &self.instance
    }
    /// Authenticated original setter when the source head is a variable read.
    #[must_use]
    pub fn handle_binding(&self) -> Option<&OriginalSourceClassHandleBinding> {
        self.handle.as_deref()
    }
    /// Selected enclosing source body, without entered-frame permission.
    #[must_use]
    pub fn source_body(&self) -> Option<&OriginalSourceScriptBody> {
        self.source_body.as_deref()
    }
    /// All unresolved source and execution applicability premises.
    #[must_use]
    pub fn obligations(&self) -> &[SourceCommandTransitionObligation] {
        &self.obligations
    }
    /// Shared original operand facets; unknown expansion length stays unknown.
    #[must_use]
    pub fn arguments(&self) -> Vec<crate::registry_invocation::EffectiveInvocationWord> {
        self.arguments
            .iter()
            .map(|word| {
                if word.original.group().expand {
                    crate::registry_invocation::EffectiveInvocationWord::Expanded
                } else {
                    word.value.as_deref().map_or(
                        crate::registry_invocation::EffectiveInvocationWord::Dynamic,
                        crate::registry_invocation::EffectiveInvocationWord::from_bytes,
                    )
                }
            })
            .collect()
    }
    /// Whole actual argument producer, including captured alias prefixes.
    #[must_use]
    pub fn argument_word(&self, ordinal: usize) -> Option<&NativeWord> {
        Some(&self.arguments.get(ordinal)?.original)
    }
    /// Genuine source naming input, without converting Logical to Native.
    #[must_use]
    pub fn argument_input(&self, ordinal: usize) -> Option<&SourceAdviceNameInput> {
        self.arguments.get(ordinal)?.input.as_ref()
    }
    /// Full source, configuration and Registry all belong to the original input.
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
            && self
                .instance
                .constructor
                .matches_source_context(image, config, context)
            && self
                .handle
                .as_ref()
                .is_none_or(|handle| handle.matches_source_context(image, config, context))
            && self.source_body.as_ref().is_none_or(|body| {
                body.matches_source(image, config) && body.matches_context(context)
            })
    }
}

pub(super) struct PendingSourceClassHandle {
    key: SourceHandleKey,
    binding: Option<Arc<OriginalSourceClassHandleBinding>>,
}
impl OriginalSourceTransitionAdviceTape {
    pub(crate) fn class_instance_words(
        &self,
        offset: u32,
    ) -> Option<&OriginalSourceClassInstanceWords> {
        self.class_instance_calls.get(&offset)?.as_ref()
    }
}
impl AdviceGraph {
    fn source_instance_class_is_current(&self, instance: &OriginalSourceClassInstance) -> bool {
        self.cells.values().any(|cell| {
            matches!(cell, AdviceCell::SourceClass(class)
            if class.factory() == instance.class_declaration().factory())
        })
    }
    fn source_class_instance(
        &self,
        input: &SourceAdviceNameInput,
    ) -> Option<(Arc<OriginalSourceClassInstance>, Vec<SourceAdviceWord>)> {
        let mut key = self.lookup_head_key(input.bytes())?;
        let mut seen = Vec::new();
        let mut arguments = Vec::new();
        let mut lineage = Vec::new();
        loop {
            if seen.contains(&key) {
                return None;
            }
            seen.push(key.clone());
            match self.cells.get(&key)? {
                AdviceCell::ClassInstance(instance) => {
                    self.source_instance_class_is_current(instance)
                        .then_some(())?;
                    let mut selected = instance.as_ref().clone();
                    selected.lineage.extend(lineage);
                    return Some((Arc::new(selected), arguments));
                }
                AdviceCell::Alias {
                    target,
                    prefix,
                    lookup,
                    lineage: steps,
                } => {
                    let mut composed = prefix.clone();
                    composed.extend(arguments);
                    arguments = composed;
                    lineage.extend(steps.iter().cloned());
                    key = self.alias_target_key(target, *lookup)?;
                }
                _ => return None,
            }
        }
    }
    fn class_handle_instance(
        &self,
        word: &SourceAdviceWord,
    ) -> Option<Arc<OriginalSourceClassHandleBinding>> {
        let arena = word.original.executable_parts();
        let [part] = arena.list(arena.root()) else {
            return None;
        };
        let ExecutablePart::Variable { name, index: None } = part.part else {
            return None;
        };
        let key = self.handle_key(arena.bytes(name)?)?;
        let (_, binding) = self.class_handles.iter().find(|(owned, _)| owned == &key)?;
        if let Some(slot) = binding.instance.source_slot() {
            let AdviceCell::ClassInstance(current) = self.cells.get(slot)? else {
                return None;
            };
            (current.constructor == binding.instance.constructor).then_some(())?;
        }
        self.source_instance_class_is_current(&binding.instance)
            .then_some(())?;
        Some(Arc::clone(binding))
    }
}
impl AdviceInvocationContext<'_> {
    pub(super) fn class_instance_from_call(
        &self,
        call: &OriginalSourceConstructorCall,
        graph: &AdviceGraph,
    ) -> Option<Arc<OriginalSourceClassInstance>> {
        let grammar = call.class_declaration().grammar(self.context)?;
        let arguments = call.arguments();
        let first = std::str::from_utf8(arguments.first()?.literal_bytes()?).ok()?;
        let name_at = if let Some(method) = grammar.manufacturer(first) {
            method.names_instance_at.map(usize::from)
        } else {
            Some(grammar.conditional_construction_name_at(first, std::iter::empty::<&str>())?)
        };
        let (name, slot) = if let Some(name_at) = name_at {
            let name = call.argument_input(name_at)?.clone();
            if name.bytes().is_empty() || name.bytes().contains(&0) {
                return None;
            }
            let slot = if grammar.family == tcl_registry::definer::DefinerFamily::TclOo {
                match self.policy {
                    AdviceNamingPolicy::Native(policy) => policy
                        .recipe()
                        .oo_object_publication_slot(graph.native_source_context()?, name.bytes())
                        .ok()?,
                    AdviceNamingPolicy::Logical(_) => {
                        graph.publication_key(name.bytes(), AdvicePublicationPurpose::Define)?
                    }
                }
            } else {
                graph.publication_key(name.bytes(), AdvicePublicationPurpose::Define)?
            };
            graph.namespaces.contains(&slot.namespace).then_some(())?;
            if let Some(AdviceCell::ClassInstance(instance)) = graph.cells.get(&slot)
                && instance.constructor.as_ref() == call
            {
                return Some(Arc::clone(instance));
            }
            if graph
                .cells
                .get(&slot)
                .is_some_and(|cell| !matches!(cell, AdviceCell::Deleted))
            {
                return None;
            }
            (Some(name), Some(slot))
        } else {
            (None, None)
        };
        Some(Arc::new(OriginalSourceClassInstance {
            constructor: Arc::new(call.clone()),
            name,
            slot,
            lineage: Vec::new(),
        }))
    }
    pub(super) fn install_source_class_instance(
        graph: &mut AdviceGraph,
        instance: Option<Arc<OriginalSourceClassInstance>>,
    ) {
        if let Some(instance) = instance
            && let Some(slot) = instance.source_slot().cloned()
        {
            graph
                .cells
                .insert(slot, AdviceCell::ClassInstance(instance));
        }
    }
    pub(super) fn before_operand_class_handle(
        &self,
        tape: &OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        native: &[NativeWord],
        written: &[SourceAdviceWord],
    ) -> Option<PendingSourceClassHandle> {
        self.with_before_operand_handle_schema(graph, native, written, |invocation, schema| {
            self.source_class_handle(tape, graph, invocation, schema)
        })
    }
    fn source_class_handle(
        &self,
        tape: &OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> Option<PendingSourceClassHandle> {
        let layout = *schema.authored_source_descriptors().command.binds_handle?;
        let HandleName::Word(variable_at) = layout.name_from else {
            return None;
        };
        let HandleClassSource::ConstructionValue(value_at) = layout.class_from else {
            return None;
        };
        if schema.facts().arity_accepts_frozen_arguments() != Some(true)
            || invocation.native.iter().any(|word| word.group().expand)
        {
            return None;
        }
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
        let variable = invocation
            .arguments
            .get(usize::from(variable_at))?
            .input
            .as_ref()?
            .clone();
        let key = graph.handle_key(variable.bytes())?;
        let binding = self.source_class_handle_value(
            tape,
            graph,
            invocation,
            schema,
            (layout, variable, usize::from(value_at)),
        );
        Some(PendingSourceClassHandle { key, binding })
    }
    fn source_class_handle_value(
        &self,
        tape: &OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        operands: (HandleBindingSpec, SourceAdviceNameInput, usize),
    ) -> Option<Arc<OriginalSourceClassHandleBinding>> {
        let (layout, variable, value_at) = operands;
        let rhs = invocation.arguments.get(value_at)?;
        if !matches!(rhs.origin, InvocationWordOrigin::Written(_))
            || !invocation.native.contains(&rhs.original)
        {
            return None;
        }
        let substitution = super::original_single_command_substitution_words(&rhs.original)?;
        let call = tape.constructor_call(substitution.command().words.first()?.span().start())?;
        (call.original_words() == substitution.command().words).then_some(())?;
        let instance = self.class_instance_from_call(call, graph)?;
        let mut setter = self.schema_advice(invocation, graph, schema)?;
        setter
            .obligations
            .push(SourceCommandTransitionObligation::RegisteredHandleBindingApplicability);
        Some(Arc::new(OriginalSourceClassHandleBinding {
            setter: Arc::new(setter),
            layout,
            variable,
            operand: rhs.original.clone(),
            instance,
        }))
    }
    pub(super) fn retain_source_class_handle_body(
        &self,
        update: Option<PendingSourceClassHandle>,
        body: &Arc<OriginalSourceScriptBody>,
        obligation: SourceCommandTransitionObligation,
    ) -> Option<PendingSourceClassHandle> {
        body.matches_source(self.origin.source_image(), self.config)
            .then_some(())?;
        body.matches_context(self.context).then_some(())?;
        let mut update = update?;
        if let Some(binding) = &mut update.binding {
            let binding = Arc::make_mut(binding);
            let setter = Arc::make_mut(&mut binding.setter);
            setter.logical_body = Some(Arc::clone(body));
            setter.obligations.push(obligation);
        }
        Some(update)
    }
    pub(super) fn install_source_class_handle(
        graph: &mut AdviceGraph,
        update: Option<PendingSourceClassHandle>,
    ) {
        if let Some(update) = update {
            graph.class_handles.retain(|(key, _)| key != &update.key);
            if let Some(binding) = update.binding {
                Self::install_source_class_instance(graph, Some(Arc::clone(&binding.instance)));
                graph.class_handles.push((update.key, binding));
            }
        }
    }
    pub(super) fn retain_class_instance_invocation(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        native: &[NativeWord],
        written: &[SourceAdviceWord],
    ) -> bool {
        let Some(head) = written.first() else {
            return false;
        };
        let selection = if let Some(input) = &head.input {
            graph
                .source_class_instance(input)
                .map(|(instance, prefix)| (instance, prefix, None))
        } else {
            graph
                .class_handle_instance(head)
                .map(|binding| (Arc::clone(&binding.instance), Vec::new(), Some(binding)))
        };
        let Some((instance, mut arguments, handle)) = selection else {
            return false;
        };
        for (ordinal, word) in arguments.iter_mut().enumerate() {
            word.origin = InvocationWordOrigin::BindingPrefix(ordinal);
        }
        arguments.extend(written.iter().skip(1).cloned());
        let mut obligations = instance.constructor.obligations().to_vec();
        if handle.is_some() {
            obligations
                .push(SourceCommandTransitionObligation::RegisteredHandleBindingApplicability);
        }
        if graph.procedure_body.is_some() {
            obligations.push(SourceCommandTransitionObligation::OriginalProcedureBodyApplicability);
        }
        if graph.future_procedure_source {
            obligations.push(
                SourceCommandTransitionObligation::FutureOriginalProcedureSourceApplicability,
            );
        }
        if (!graph.uncertain_operations.is_empty()
            || native.iter().any(|word| {
                word.executable_parts()
                    .all_parts()
                    .any(|part| !matches!(part.part, ExecutablePart::Text(_)))
            }))
            && !obligations.contains(&SourceCommandTransitionObligation::UnknownEarlierMutation)
        {
            obligations.push(SourceCommandTransitionObligation::UnknownEarlierMutation);
        }
        let receipt = OriginalSourceClassInstanceWords {
            site: super::super::CommandAllocationSite {
                source: Arc::clone(self.origin),
                offset: native[0].span().start(),
            },
            original: Arc::from(native),
            arguments,
            instance,
            handle,
            source_body: graph.procedure_body.clone(),
            obligations,
        };
        super::source_class::merge_receipt(
            &mut tape.class_instance_calls,
            receipt.site.offset,
            Some(receipt),
        );
        true
    }
    pub(super) fn retain_class_instance_body_owner(
        tape: &mut OriginalSourceTransitionAdviceTape,
        offset: u32,
        body: &Arc<OriginalSourceScriptBody>,
        obligation: SourceCommandTransitionObligation,
    ) {
        if let Some(Some(receipt)) = tape.class_instance_calls.get_mut(&offset) {
            if receipt
                .source_body
                .as_ref()
                .is_some_and(|previous| previous != body)
            {
                tape.class_instance_calls.insert(offset, None);
                return;
            }
            receipt.source_body = Some(Arc::clone(body));
            if !receipt.obligations.contains(&obligation) {
                receipt.obligations.push(obligation);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SourceCommandTransitionObligation;
    use crate::analyser::{Analyser, AnalysisResult};
    use crate::registry_invocation::source_structure::source_class_instance_words_at;

    fn receiver(
        source: &str,
        analysis: &AnalysisResult,
        marker: &str,
    ) -> Option<super::OriginalSourceClassInstanceWords> {
        let offset = u32::try_from(source.find(marker).unwrap()).unwrap();
        source_class_instance_words_at(source, analysis, offset)
    }

    #[test]
    fn original_class_instance_receiver_keeps_named_moves_alias_prefix_and_whole_vector() {
        // naming.source.original-class-instance-receiver
        // docs/design/analysis/name-resolution-proofs/source-original-class-instance-receiver.md
        let source = "oo::class create C {method ping {x} {}}; C create obj; rename obj moved; interp alias {} call {} moved ping; call value";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let words = receiver(source, &analysis, "call value").expect("original named receiver");
        assert_eq!(words.original_words().len(), 2);
        assert_eq!(
            words.arguments()[0].literal_bytes(),
            Some(b"ping".as_slice())
        );
        assert_eq!(
            words.arguments()[1].literal_bytes(),
            Some(b"value".as_slice())
        );
        assert_eq!(
            words.argument_input(0).unwrap().original_word(),
            words.argument_word(0)
        );
        assert!(words.argument_word(0).unwrap().span().start() < words.site().offset);
        assert_eq!(words.instance().name_input().unwrap().bytes(), b"obj");
        assert_eq!(
            words.instance().source_slot().unwrap().simple.as_bytes(),
            b"moved"
        );
        assert_eq!(words.instance().lineage().len(), 2);
        assert!(words.handle_binding().is_none());
        assert!(
            words
                .obligations()
                .contains(&SourceCommandTransitionObligation::UnknownEarlierMutation)
        );
        assert!(receiver(&source.replace("value", "other"), &analysis, "call other").is_none());
        assert_eq!(
            words
                .instance()
                .class_declaration()
                .source_class(&analysis)
                .unwrap()
                .name_input()
                .bytes(),
            b"C"
        );
    }

    #[test]
    fn original_class_instance_receiver_keeps_anonymous_exact_setter_without_slot_or_cell() {
        // naming.source.original-class-instance-receiver
        // docs/design/analysis/name-resolution-proofs/source-original-class-instance-receiver.md
        for source in [
            "oo::class create C {}; set o [C new]; $o ping value",
            "oo::class create C {}; interp alias {} make {} C new; set o [make]; $o ping value",
            "oo::class create C {}; rename set store; store o [C new]; $o ping value",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let words = receiver(source, &analysis, "$o ping value").expect(source);
            assert!(words.instance().name_input().is_none());
            assert!(words.instance().source_slot().is_none());
            let setter = words.handle_binding().expect("genuine source setter");
            assert_eq!(setter.variable_input().bytes(), b"o");
            assert_eq!(setter.setter().original_words().len(), 3);
            assert!(
                super::super::original_single_command_substitution_words(setter.original_operand())
                    .is_some()
            );
            assert_eq!(
                words.arguments()[0].literal_bytes(),
                Some(b"ping".as_slice())
            );
            assert!(words.obligations().contains(
                &SourceCommandTransitionObligation::RegisteredHandleBindingApplicability
            ));
        }
    }

    #[test]
    fn original_class_instance_receiver_named_handle_refuses_known_slot_moves_and_terminal_mutations()
     {
        // naming.source.original-class-instance-receiver
        // docs/design/analysis/name-resolution-proofs/source-original-class-instance-receiver.md
        let source = "oo::class create C {}; set o [C create obj]; $o ping";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let words = receiver(source, &analysis, "$o ping")
            .expect("same constructor source at original named slot");
        assert_eq!(
            words.instance().source_slot().unwrap().simple.as_bytes(),
            b"obj"
        );
        for source in [
            "oo::class create C {}; set o [C create obj]; rename obj moved; $o ping",
            "oo::class create C {}; C create obj; rename obj {}; obj ping",
            "oo::class create C {}; C create obj; proc obj args {}; obj ping",
            "oo::class create C {}; C create obj; rename C {}; obj ping",
            "oo::class create C {}; set o [C create obj]; rename obj {}; $o ping",
            "oo::class create C {}; set o [C new]; set o replacement; $o ping",
            "oo::class create C {}; set o [C new]; rename C {}; $o ping",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let marker = if source.contains("$o ping") {
                "$o ping"
            } else {
                "obj ping"
            };
            assert!(receiver(source, &analysis, marker).is_none(), "{source}");
        }
    }

    #[test]
    fn original_class_instance_receiver_refuses_nonwhole_unknown_or_overridden_construction() {
        // naming.source.original-class-instance-receiver
        // docs/design/analysis/name-resolution-proofs/source-original-class-instance-receiver.md
        for source in [
            "oo::class create C {}; set o prefix[C new]; $o ping",
            "oo::class create C {}; set o [C new; puts other]; $o ping",
            "oo::class create C {}; set o [C {*}$args]; $o ping",
            "oo::class create C {self method new args {}}; set o [C new]; $o ping",
            "oo::class create C {self unexport new}; set o [C new]; $o ping",
            "oo::class create C {}; proc set args {}; set o [C new]; $o ping",
            "oo::class create C {}; set o [C new]; prefix$o ping",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let marker = if source.contains("prefix$o") {
                "prefix$o ping"
            } else {
                "$o ping"
            };
            assert!(receiver(source, &analysis, marker).is_none(), "{source}");
        }
    }

    #[test]
    fn original_class_instance_receiver_keeps_future_body_without_borrowing_parent_handles() {
        // naming.source.original-class-instance-receiver
        // docs/design/analysis/name-resolution-proofs/source-original-class-instance-receiver.md
        let source = "proc p {} {set o [C new]; $o ping}; oo::class create C {}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let words = receiver(source, &analysis, "$o ping").expect("genuine future source body");
        assert!(words.source_body().is_some());
        assert!(
            words
                .obligations()
                .contains(&SourceCommandTransitionObligation::OriginalProcedureBodyApplicability)
        );
        assert!(words.obligations().contains(
            &SourceCommandTransitionObligation::FutureOriginalProcedureSourceApplicability
        ));
        for source in [
            "oo::class create C {}; set o [C new]; proc p {} {$o ping}; p",
            "oo::class create C {}; proc p {} {set o [C new]}; p; $o ping",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            assert!(receiver(source, &analysis, "$o ping").is_none(), "{source}");
        }
    }

    #[test]
    fn original_class_instance_receiver_provider_layout_preserves_type_method_refusals() {
        // naming.source.original-class-instance-receiver
        // docs/design/analysis/name-resolution-proofs/source-original-class-instance-receiver.md
        for source in [
            "snit::type Type {method ping {} {}}; Type obj; obj ping",
            "snit::type Type {method ping {} {}}; Type create obj; obj ping",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let words = receiver(source, &analysis, "obj ping").expect(source);
            assert_eq!(words.instance().name_input().unwrap().bytes(), b"obj");
            assert_eq!(
                words
                    .instance()
                    .class_declaration()
                    .source_class(&analysis)
                    .unwrap()
                    .name_input()
                    .bytes(),
                b"Type"
            );
        }
        let source = "snit::type Type {typemethod obj {} {}}; Type obj; obj ping";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        assert!(receiver(source, &analysis, "obj ping").is_none());
    }
    #[test]
    fn original_logical_class_instance_body_retains_source_axis_without_native_manufacturer_grant()
    {
        // naming.source.original-class-instance-receiver
        // docs/design/analysis/name-resolution-proofs/source-original-class-instance-receiver.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        for (source, obligation) in [
            (
                "oo::class create C {}; proc p {} {set o [C new]; $o ping}",
                SourceCommandTransitionObligation::DeferredLogicalBodyApplicability,
            ),
            (
                "oo::class create C {}; if {1} {set o [C new]; $o ping}",
                SourceCommandTransitionObligation::ConditionalLogicalBodyApplicability,
            ),
        ] {
            let input = crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                tcl_registry::model::ingress::context_for_profile(profile),
                tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
            );
            let analysis = Analyser::new()
                .with_resolved_input(input)
                .analyse(source, profile.name);
            let input = analysis.resolved_input.as_ref().unwrap();
            let ctor_at = u32::try_from(source.find("[C new").unwrap() + 1).unwrap();
            let constructor =
                crate::registry_invocation::source_structure::source_constructor_call_at(
                    source, &analysis, ctor_at,
                )
                .unwrap();
            let body = constructor.original_source_body().unwrap();
            let offset = u32::try_from(source.find("$o ping").unwrap()).unwrap();
            let plan = tcl_lexer::native_script_words_in(
                tcl_lexer::SourceImage::document(source),
                body.content_span(),
                input.lexer_config(),
            )
            .unwrap();
            let words = plan
                .commands
                .iter()
                .find(|command| command.words[0].span().start() == offset)
                .unwrap();
            let receipt = analysis
                .retained_command_realm()
                .unwrap()
                .original_source_class_instance_words(input, &words.words)
                .expect("source axis retains genuine Logical setter/body");
            assert!(receipt.obligations().contains(&obligation));
            assert!(receipt.source_body().is_some());
            assert!(
                receipt
                    .handle_binding()
                    .unwrap()
                    .variable_input()
                    .native_input()
                    .is_none()
            );
            assert!(
                receipt
                    .instance()
                    .constructor()
                    .constructor_shape(&analysis)
                    .is_none()
            );
            assert!(source_class_instance_words_at(source, &analysis, offset).is_none());
        }
    }
    #[test]
    fn original_native_instance_and_handle_names_delegate_purpose_name_units() {
        // naming.source.original-class-instance-receiver
        // docs/design/analysis/name-resolution-proofs/source-original-class-instance-receiver.md
        // Source receipt coverage; no Unicode object creation or stored-value observation.
        let source =
            "oo::class create é {}; é create {obj α}; {obj α} ping; set hβ [é new]; ${hβ} ping";
        for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let analysis = Analyser::new().analyse(source, dialect);
            let named = receiver(source, &analysis, "{obj α} ping").unwrap();
            assert_eq!(
                named.instance().name_input().unwrap().bytes(),
                "obj α".as_bytes()
            );
            assert_eq!(
                named.instance().source_slot().unwrap().simple.as_bytes(),
                "obj α".as_bytes()
            );
            let held = receiver(source, &analysis, "${hβ} ping").unwrap();
            assert_eq!(
                held.handle_binding().unwrap().variable_input().bytes(),
                "hβ".as_bytes()
            );
            assert!(held.instance().name_input().is_none());
            assert!(held.instance().source_slot().is_none());
        }
    }
}
