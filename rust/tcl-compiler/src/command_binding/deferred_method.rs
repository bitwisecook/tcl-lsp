// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Lexical receiver bodies of proved native class declarations.

use super::{
    Arc, CommandAllocationSite, DeferredSourceBody, ExecutedScriptSource, InvocationFacts,
    ModuleCommandBindings, SourceCommandBindings, SourceExecutionContext, SourceNativeInvocation,
    retained_script_operand, source_binding, source_effective_words,
};

/// Receiver selected by an original, validated method declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SourceMethodReceiver {
    /// A method declared on manufactured instances.
    Instance,
    /// A class-facing declaration on its own table or native classmethod delegate.
    Class,
}

/// Temporal method declaration inventory. Original source, frame and selected
/// visibility remain separate from current receiver execution and result proof.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceReceiverMethodEntry {
    name: String,
    name_source: crate::ir::SourceSite,
    receiver: SourceMethodReceiver,
    exported: bool,
    declaring_class: Option<super::SourceCommandTarget>,
    class_delegate: bool,
    delegate_allocation: Option<Arc<SourceClassDelegateAllocation>>,
    entry: super::SourceConstructorEntry,
}

/// One native delegate allocated by an original class factory, identified by
/// its owning class allocation rather than the generated namespace's bytes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SourceClassDelegateAllocation {
    owner: super::SourceCommandTarget,
    factory: super::SourceCommandTarget,
    worker: super::SourceCommandTarget,
    generation: u64,
}

/// Declared methods keyed by their original receiver table and frozen name.
pub type SourceReceiverMethodEntries =
    super::BTreeMap<(SourceMethodReceiver, String), SourceReceiverMethodEntry>;

impl SourceReceiverMethodEntry {
    /// Frozen declared method name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Exact original name-word site, including its source provenance.
    #[must_use]
    pub fn name_source(&self) -> &crate::ir::SourceSite {
        &self.name_source
    }
    /// Instance or class-object declaration receiver.
    #[must_use]
    pub fn receiver(&self) -> SourceMethodReceiver {
        self.receiver
    }
    /// Visibility at this retained dispatch generation. This does not prove
    /// that the receiver or dispatcher is still current.
    #[must_use]
    pub fn is_exported(&self) -> bool {
        self.exported
    }
    /// Original declaring class, separately from the current receiver class.
    /// Inherited entries retain this exact implementation allocation.
    #[must_use]
    pub fn declaring_class(&self) -> Option<&super::SourceCommandTarget> {
        self.declaring_class.as_ref()
    }
    /// Native delegate methods inherit through the original class allocation;
    /// an ordinary self method never acquires that inheritance policy.
    #[must_use]
    pub fn is_native_class_delegate(&self) -> bool {
        self.class_delegate
    }
    /// Actual source instance and declaration offset.
    #[must_use]
    pub fn declaration(&self) -> &CommandAllocationSite {
        self.entry.declaration()
    }
    /// Original body carrier of this method incarnation.
    #[must_use]
    pub fn body(&self) -> &ExecutedScriptSource {
        self.entry.body()
    }
    /// Native formal names and defaults retained at definition.
    #[must_use]
    pub fn formals(&self) -> &[(String, Option<String>)] {
        self.entry.formals()
    }
    /// Original receiver-method activation.
    #[must_use]
    pub fn frame(&self) -> &crate::var_resolve::VariableExecutionFrame {
        self.entry.frame()
    }
}

impl SourceCommandBindings {
    pub(super) fn retain_standalone_definition_references(
        &mut self,
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) {
        let Some((class, grammar)) = standalone_definition_class(native, facts, state, context)
        else {
            return;
        };
        let Some(origin) = state.current_source_origin.as_ref() else {
            return;
        };
        let phases = capture_standalone_definition_references(
            native, facts, grammar, &class, state, context,
        );
        self.record_standalone_definition_method_references(
            &class,
            CommandAllocationSite {
                source: Arc::clone(origin),
                offset: native.segment.span.start(),
            },
            phases,
        );
    }

    pub(super) fn register_bounded_methods(
        &mut self,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
        facts: &InvocationFacts,
    ) -> bool {
        let Some(grammar) = context
            .registry
            .get_for_surface(
                &native.target.command,
                state
                    .baseline
                    .dialect
                    .and_then(tcl_registry::InvocationDialect::authoring_query)
                    .map(|query| query.with_realm(context.realm)),
            )
            .and_then(|spec| spec.definition_body)
        else {
            return false;
        };
        if !state.default_construction_dependencies_hold(grammar) {
            return false;
        }
        let Some(argument) = facts.arg_roles.iter().find_map(|(index, role)| {
            (*role == tcl_registry::ArgRole::Body)
                .then_some(facts.argument_offset + usize::from(*index))
        }) else {
            return false;
        };
        let Some(definition) = retained_script_operand(
            argument,
            native.script_operands(),
            state,
            native.segment.span.start(),
            context.config,
        ) else {
            return false;
        };
        let Some(parameters) = state
            .baseline
            .dialect
            .and_then(tcl_registry::InvocationDialect::parameter_grammar)
        else {
            return false;
        };
        let map =
            tcl_lexer::SourceMap::from_image(&definition.text).with_base(definition.base(), 0, 0);
        let Some(segments) = crate::segmenter::segment_commands_image_with_offset_and_config(
            &definition.text,
            definition.base(),
            context.config,
        ) else {
            return false;
        };
        if !segments.iter().all(|segment| {
            closed_definition_member(grammar, &definition, &map, segment, state, context)
        }) {
            return false;
        }
        let mut entries = retained_class_entries(
            grammar,
            &definition,
            &map,
            &segments,
            state.baseline.dialect,
            parameters,
            context,
        );
        if !state.inherit_class_entries(grammar, &map, &segments, context, &mut entries) {
            return false;
        }
        let references = capture_definition_method_references(
            grammar,
            &definition,
            &map,
            &segments,
            state,
            DefinitionReferenceScope {
                parameters,
                receiver: SourceMethodReceiver::Instance,
            },
            context,
        );
        if let Some(created) = state.record_class_definition(facts, native.target, context, entries)
        {
            self.record_definition_method_references(&created, references);
        }
        self.register_original_method_bodies(
            grammar,
            &definition,
            state.baseline.dialect,
            parameters,
            context,
        );
        true
    }
    fn register_original_method_bodies(
        &mut self,
        grammar: &tcl_registry::definer::DefinitionBodyGrammar,
        definition: &ExecutedScriptSource,
        dialect: Option<tcl_registry::InvocationDialect>,
        parameters: tcl_dialect::ParameterGrammar,
        context: SourceExecutionContext<'_>,
    ) {
        let map =
            tcl_lexer::SourceMap::from_image(&definition.text).with_base(definition.base(), 0, 0);
        let Some(segments) = crate::segmenter::segment_commands_image_with_offset_and_config(
            &definition.text,
            definition.base(),
            context.config,
        ) else {
            return;
        };
        for segment in segments {
            if let Some(block) =
                class_definition_block(grammar, definition, &map, &segment, dialect, context)
            {
                self.register_original_method_bodies(grammar, &block, dialect, parameters, context);
            } else if let Some(body) = deferred_method_body(
                grammar, definition, &map, &segment, dialect, parameters, context,
            ) {
                self.deferred.insert(body.implementation_id(), body);
            }
        }
    }
}

fn standalone_definition_class<'a>(
    native: SourceNativeInvocation<'_>,
    facts: &InvocationFacts,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'a>,
) -> Option<(
    super::SourceCommandTarget,
    &'a tcl_registry::definer::DefinitionBodyGrammar,
)> {
    let mut configured = facts
        .state_transitions
        .declared()?
        .facts()
        .iter()
        .filter_map(|fact| match &fact.transition {
            tcl_registry::StateTransition::ObjectDispatch(
                tcl_registry::ObjectDispatchTransition::Configure {
                    target,
                    layer: tcl_registry::ObjectDispatchLayer::Class,
                },
            ) => target.literal(),
            _ => None,
        });
    let name = configured.next()?;
    if !native.target.registry_backed
        || !native.target.prepended.is_empty()
        || native.target.implementation_generation != 0
        || state.source_step_observed()
        || state.source_execution_observed(native.target.identity.as_ref())
    {
        return None;
    }
    let grammar = context
        .registry
        .get_for_surface(
            &native.target.command,
            state
                .baseline
                .dialect?
                .authoring_query()
                .map(|query| query.with_realm(context.realm)),
        )?
        .definition_body?;
    if configured.next().is_some()
        || !state.source_lookup_is_closed(name, &context.namespace_identity())
    {
        return None;
    }
    let binding = source_binding(state, name, &context.namespace_identity());
    let class = binding.proved_target()?;
    let identity = class.identity.as_ref()?;
    let raw = state
        .source_keys(name, &context.namespace_identity())
        .into_iter()
        .find_map(|slot| state.bindings.get(&slot))?;
    if raw.len() != 1
        || !matches!(raw.iter().next(), Some(super::MayBinding::Target(target))
        if target.kind == super::BindingKind::Class && target.terminal
            && target.prepended.is_empty() && target.token.as_ref() == Some(identity))
    {
        return None;
    }
    let definition = state.class_definitions.get(identity)?;
    (definition.dispatcher.is_none()
        && state.class_definition_dependencies_hold(definition)
        && !state.tainted_object_dispatch.contains("*")
        && !state.tainted_object_dispatch.contains(&class.command))
    .then(|| (class.clone(), grammar))
}

fn capture_standalone_definition_references(
    native: SourceNativeInvocation<'_>,
    facts: &InvocationFacts,
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    class: &super::SourceCommandTarget,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<Vec<super::definition_method_references::SourceDefinitionMethodReferencePhase>> {
    super::native_result::numeric_call_arguments(context, native.target)?;
    context.written_representations?;
    let entries = &state
        .class_definitions
        .get(class.identity.as_ref()?)?
        .receiver_method_entries;
    let arguments = native.invocation.arguments();
    if arguments.len() == 2 && facts.arg_roles.contains(&(1, tcl_registry::ArgRole::Body)) {
        let definition = retained_script_operand(
            1,
            native.script_operands(),
            state,
            native.segment.span.start(),
            context.config,
        )?;
        let map =
            tcl_lexer::SourceMap::from_image(&definition.text).with_base(definition.base(), 0, 0);
        let segments = crate::segmenter::segment_commands_image_with_offset_and_config(
            &definition.text,
            definition.base(),
            context.config,
        )?;
        let mut references = Vec::new();
        for segment in segments {
            let words = crate::ir::CommandTokens::from_segmented(&map, context.config, &segment);
            let values = source_effective_words(words.words(), state.baseline.dialect, None);
            let member = grammar.member(values.first()?.as_registry_word().literal()?)?;
            // Only visibility rows preserve this at-phase method inventory.
            // Retraction, slot options and arbitrary definition bodies need
            // independent effects before later operands can use that table.
            member.visibility_effect?;
            let worker = original_definition_member_target(
                grammar,
                member,
                tcl_registry::definer::DefinitionReceiver::Instance,
                state,
                context,
            )?;
            references.extend(capture_reference_operands(
                &DefinitionReferenceInvocation {
                    receiver: SourceMethodReceiver::Instance,
                    worker,
                    site: CommandAllocationSite {
                        source: Arc::clone(&definition.origin),
                        offset: segment.span.start(),
                    },
                    config: context.config,
                },
                &words,
                &values,
                entries,
            )?);
        }
        return Some(references);
    }
    let member = grammar.member(arguments.literal_at(1)?)?;
    member.visibility_effect?;
    let worker = original_definition_member_target(
        grammar,
        member,
        tcl_registry::definer::DefinitionReceiver::Instance,
        state,
        context,
    )?;
    let source = state.current_source_origin.as_ref()?;
    native
        .words
        .iter()
        .skip(3)
        .enumerate()
        .map(|(index, operand)| {
            let name = arguments.literal_at(index + 2)?;
            super::definition_method_references::SourceDefinitionMethodReferencePhase::capture(
                super::definition_method_references::DefinitionMethodReferenceCapture {
                    receiver: SourceMethodReceiver::Instance,
                    worker: worker.clone(),
                    invocation: CommandAllocationSite {
                        source: Arc::clone(source),
                        offset: native.segment.span.start(),
                    },
                    operand: operand.clone(),
                    name: name.to_owned(),
                    entry: entries
                        .get(&(SourceMethodReceiver::Instance, name.to_owned()))
                        .cloned(),
                    config: context.config,
                },
            )
        })
        .collect()
}

impl ModuleCommandBindings {
    /// A native classmethod body belongs to the original delegated receiver.
    /// This closes only that plain body entry, without creating a `TclOO` object
    /// receipt or granting authority to self-dispatch inside the body.
    pub(super) fn native_class_delegate_entry_is_current(
        &self,
        receiver: &super::SourceCommandTarget,
        entry: &SourceReceiverMethodEntry,
    ) -> bool {
        let Some(delegate) = entry.delegate_allocation.as_ref() else {
            return false;
        };
        entry.class_delegate
            && entry.receiver == SourceMethodReceiver::Class
            && entry.exported
            && !self.opaque_domain
            && !self.tainted_object_dispatch.contains("*")
            && !self.tainted_object_dispatch.contains(&receiver.command)
            && !self
                .tainted_object_dispatch
                .contains(&delegate.owner.command)
            && self.object_instances.class_delegate_generation == Some(delegate.generation)
            && entry.declaring_class.as_ref() == Some(&delegate.owner)
            && [
                &delegate.owner,
                &delegate.factory,
                &delegate.worker,
                receiver,
            ]
            .into_iter()
            .all(|target| target.prepended.is_empty() && self.retained_target_is_current(target))
            && receiver
                .identity
                .as_ref()
                .and_then(|identity| self.class_definitions.get(identity))
                .is_some_and(|definition| {
                    definition.dispatcher.is_none()
                        && definition.implementation_generation
                            == receiver.implementation_generation
                        && self.class_definition_dependencies_hold(definition)
                })
    }

    pub(super) fn class_definition_dependencies_hold(
        &self,
        definition: &super::ClassDefinitionReceipt,
    ) -> bool {
        !self.opaque_domain
            && definition.receiver_method_entries.values().all(|entry| {
                !entry.class_delegate
                    || entry.delegate_allocation.as_ref().is_some_and(|delegate| {
                        self.object_instances.class_delegate_generation == Some(delegate.generation)
                            && entry.declaring_class.as_ref() == Some(&delegate.owner)
                            && self.retained_target_is_current(&delegate.owner)
                    })
            })
            && definition.inherited_classes.iter().all(|base| {
                !self.tainted_object_dispatch.contains("*")
                    && !self.tainted_object_dispatch.contains(&base.command)
                    && self.retained_target_is_current(base)
                    && base
                        .identity
                        .as_ref()
                        .and_then(|identity| self.class_definitions.get(identity))
                        .is_some_and(|receipt| {
                            receipt.dispatcher.is_none()
                                && receipt.implementation_generation
                                    == base.implementation_generation
                        })
            })
    }

    pub(super) fn closed_inherited_member(
        &self,
        grammar: &tcl_registry::definer::DefinitionBodyGrammar,
        member: &tcl_registry::definer::MemberSpec,
        values: &[crate::registry_invocation::EffectiveInvocationWord],
        context: SourceExecutionContext<'_>,
    ) -> Option<(super::SourceCommandTarget, super::ClassDefinitionReceipt)> {
        let arguments = values
            .iter()
            .skip(1)
            .map(|word| word.as_registry_word().literal())
            .collect::<Option<Vec<_>>>()?;
        let (_, name) =
            grammar.single_native_inherited_class(member, &arguments, self.baseline.dialect)?;
        if !original_definition_member(
            grammar,
            member,
            tcl_registry::definer::DefinitionReceiver::Instance,
            self,
            context,
        ) || !self.source_lookup_is_closed(name, &context.namespace_identity())
        {
            return None;
        }
        let binding = source_binding(self, name, &context.namespace_identity());
        let base = binding.proved_target()?;
        let identity = base.identity.as_ref()?;
        // An inherited class is an actual object command, not an alias to one.
        let raw = self
            .source_keys(name, &context.namespace_identity())
            .into_iter()
            .find_map(|slot| self.bindings.get(&slot))?;
        if raw.len() != 1
            || !matches!(raw.iter().next(), Some(super::MayBinding::Target(target))
            if target.kind == super::BindingKind::Class && target.terminal
                && target.prepended.is_empty() && target.token.as_ref() == Some(identity))
            || !self
                .bounded_constructions
                .get(identity)
                .is_some_and(|construction| {
                    construction.implementation_generation == base.implementation_generation
                })
        {
            return None;
        }
        let definition = self.class_definitions.get(identity)?;
        if definition.dispatcher.is_some()
            || definition.constructor_entry.is_some()
            || definition.instance_methods.is_none()
            || !self.class_definition_dependencies_hold(definition)
            || self.tainted_object_dispatch.contains(&base.command)
        {
            return None;
        }
        Some((base.clone(), definition.clone()))
    }

    fn inherit_class_entries(
        &self,
        grammar: &tcl_registry::definer::DefinitionBodyGrammar,
        map: &tcl_lexer::SourceMap<'_>,
        segments: &[crate::segmenter::SegmentedCommand],
        context: SourceExecutionContext<'_>,
        entries: &mut RetainedClassEntries,
    ) -> bool {
        let mut inherited_once = false;
        for segment in segments {
            let words = crate::ir::CommandTokens::from_segmented(map, context.config, segment);
            let values = source_effective_words(words.words(), self.baseline.dialect, None);
            let Some(member) = values
                .first()
                .and_then(|word| word.as_registry_word().literal())
                .and_then(|head| grammar.member(head))
            else {
                continue;
            };
            let Some((base, inherited)) =
                self.closed_inherited_member(grammar, member, &values, context)
            else {
                continue;
            };
            if inherited_once {
                return false;
            }
            inherited_once = true;
            let arguments = values
                .iter()
                .skip(1)
                .map(|word| word.as_registry_word().literal())
                .collect::<Option<Vec<_>>>();
            let mixin = arguments
                .as_ref()
                .and_then(|arguments| {
                    grammar.single_native_inherited_class(member, arguments, self.baseline.dialect)
                })
                .is_some_and(|(kind, _)| {
                    kind == tcl_registry::definer::NativeInheritedClassKind::Mixin
                });
            // Overlapping mixin entries need a separate method-chain receipt.
            // Disjoint entries preserve the retained local activation unchanged.
            if mixin
                && inherited.receiver_method_entries.keys().any(|key| {
                    key.0 == SourceMethodReceiver::Instance
                        && entries.receiver_method_entries.contains_key(key)
                })
            {
                return false;
            }
            let dependencies = Arc::make_mut(&mut entries.inherited_classes);
            dependencies.push(base);
            dependencies.extend(inherited.inherited_classes.iter().cloned());
            if let (Some(local), Some(base)) =
                (&mut entries.instance_methods, inherited.instance_methods)
            {
                Arc::make_mut(local).extend(base.iter().cloned());
            }
            let local = Arc::make_mut(&mut entries.receiver_method_entries);
            for (key, entry) in inherited.receiver_method_entries.iter() {
                if key.0 == SourceMethodReceiver::Instance
                    || (!mixin && entry.is_native_class_delegate())
                {
                    local.entry(key.clone()).or_insert_with(|| entry.clone());
                }
            }
        }
        true
    }

    fn record_class_definition(
        &mut self,
        facts: &InvocationFacts,
        factory: &super::SourceCommandTarget,
        context: SourceExecutionContext<'_>,
        mut entries: RetainedClassEntries,
    ) -> Option<super::SourceCommandTarget> {
        let name = facts.state_transitions.declared().and_then(|transitions| {
            transitions
                .facts()
                .iter()
                .find_map(|fact| match &fact.transition {
                    tcl_registry::StateTransition::ObjectDispatch(
                        tcl_registry::ObjectDispatchTransition::Create {
                            target: tcl_registry::ObjectDispatchTarget::Named(subject),
                            kind: tcl_registry::ObjectDispatchKind::Class,
                            ..
                        },
                    ) => subject.literal(),
                    _ => None,
                })
        })?;
        let binding = source_binding(self, name, &context.namespace_identity());
        let created = binding.proved_target()?;
        let Some(identity) = &created.identity else {
            return None;
        };
        for entry in Arc::make_mut(&mut entries.receiver_method_entries).values_mut() {
            if entry.declaring_class.is_none() {
                entry.declaring_class = Some(created.clone());
                if entry.class_delegate {
                    let allocation = created.identity.as_ref()?.allocation.as_ref()?;
                    if allocation.incarnation == super::AllocationIncarnation::RepeatedFresh {
                        return None;
                    }
                    let grammar = context.registry.native_default_construction_grammar(
                        &factory.command,
                        self.baseline.dialect?,
                        context.realm,
                    )?;
                    let query = self
                        .baseline
                        .dialect
                        .and_then(tcl_registry::InvocationDialect::authoring_query)
                        .map(|query| query.with_realm(context.realm));
                    let member = grammar.native_classmethod_member(query)?;
                    let worker = original_definition_member_target(
                        grammar,
                        member,
                        tcl_registry::definer::DefinitionReceiver::Instance,
                        self,
                        context,
                    )?;
                    entry.delegate_allocation = Some(Arc::new(SourceClassDelegateAllocation {
                        owner: created.clone(),
                        factory: factory.clone(),
                        worker,
                        generation: self.object_instances.class_delegate_generation?,
                    }));
                }
            }
        }
        Arc::make_mut(&mut self.class_definitions).insert(
            identity.clone(),
            super::ClassDefinitionReceipt {
                factory: factory.command.clone(),
                implementation_generation: created.implementation_generation,
                dispatcher: None,
                instance_methods: entries.instance_methods,
                dispatcher_methods: Arc::default(),
                constructor_entry: entries.constructor_entry,
                destructor_entry: entries.destructor_entry,
                lifecycle_entries_closed: entries.lifecycle_entries_closed,
                receiver_method_entries: entries.receiver_method_entries,
                inherited_classes: entries.inherited_classes,
            },
        );
        Some(created.clone())
    }
}

struct RetainedClassEntries {
    instance_methods: Option<Arc<super::BTreeSet<String>>>,
    constructor_entry: Option<Arc<super::SourceConstructorEntry>>,
    destructor_entry: Option<Arc<super::SourceConstructorEntry>>,
    lifecycle_entries_closed: bool,
    receiver_method_entries: Arc<SourceReceiverMethodEntries>,
    inherited_classes: Arc<Vec<super::SourceCommandTarget>>,
}

#[derive(Clone, Copy)]
struct DefinitionReferenceScope {
    parameters: tcl_dialect::ParameterGrammar,
    receiver: SourceMethodReceiver,
}

fn capture_definition_method_references(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    definition: &ExecutedScriptSource,
    map: &tcl_lexer::SourceMap<'_>,
    segments: &[crate::segmenter::SegmentedCommand],
    state: &ModuleCommandBindings,
    scope: DefinitionReferenceScope,
    context: SourceExecutionContext<'_>,
) -> Option<Vec<super::definition_method_references::SourceDefinitionMethodReferencePhase>> {
    let mut entries = SourceReceiverMethodEntries::new();
    let mut references = Vec::new();
    for segment in segments {
        if let Some(block) = class_definition_block(
            grammar,
            definition,
            map,
            segment,
            state.baseline.dialect,
            context,
        ) {
            references.extend(capture_class_block_references(
                grammar,
                &block,
                state,
                scope.parameters,
                context,
            )?);
            continue;
        }
        let words = crate::ir::CommandTokens::from_segmented(map, context.config, segment);
        let values = source_effective_words(words.words(), state.baseline.dialect, None);
        let head = values.first()?.as_registry_word().literal()?;
        let member = grammar.member(head)?;
        if let Some((_, inherited)) =
            state.closed_inherited_member(grammar, member, &values, context)
        {
            for (key, entry) in inherited.receiver_method_entries.iter() {
                if key.0 == SourceMethodReceiver::Instance {
                    entries.entry(key.clone()).or_insert_with(|| entry.clone());
                }
            }
        }
        if member.all_args_ref == Some(tcl_registry::definer::MemberRefKind::Method) {
            let receiver = match scope.receiver {
                SourceMethodReceiver::Instance => {
                    tcl_registry::definer::DefinitionReceiver::Instance
                }
                SourceMethodReceiver::Class => tcl_registry::definer::DefinitionReceiver::Class,
            };
            let worker =
                original_definition_member_target(grammar, member, receiver, state, context)?;
            references.extend(capture_reference_operands(
                &DefinitionReferenceInvocation {
                    receiver: scope.receiver,
                    worker,
                    site: CommandAllocationSite {
                        source: Arc::clone(&definition.origin),
                        offset: segment.span.start(),
                    },
                    config: context.config,
                },
                &words,
                &values,
                &entries,
            )?);
        }
        apply_declared_visibility(
            grammar,
            map,
            segment,
            state.baseline.dialect,
            context,
            &mut entries,
        );
        if let Some(mut entry) = retained_method_entry(
            grammar,
            definition,
            map,
            segment,
            state.baseline.dialect,
            scope.parameters,
            context,
        ) {
            if scope.receiver == SourceMethodReceiver::Class {
                entry.receiver = SourceMethodReceiver::Class;
            }
            references.push(capture_method_declaration_reference(
                grammar, &words, &values, state, &entry, context,
            )?);
            entries.insert((entry.receiver, entry.name.clone()), entry);
        }
    }
    Some(references)
}

fn capture_method_declaration_reference(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    words: &crate::ir::CommandTokens,
    values: &[crate::registry_invocation::EffectiveInvocationWord],
    state: &ModuleCommandBindings,
    entry: &SourceReceiverMethodEntry,
    context: SourceExecutionContext<'_>,
) -> Option<super::definition_method_references::SourceDefinitionMethodReferencePhase> {
    use super::definition_method_references::{
        DefinitionMethodReferenceCapture, SourceDefinitionMethodReferencePhase,
    };
    let strings = values
        .iter()
        .map(|word| word.as_registry_word().literal())
        .collect::<Option<Vec<_>>>()?;
    let layout = grammar.receiver_method_layout(
        strings.first()?,
        &strings[1..],
        state
            .baseline
            .dialect?
            .authoring_query()
            .map(|query| query.with_realm(context.realm)),
    )?;
    let name_at = layout
        .member
        .arg_roles
        .iter()
        .find_map(|(index, role)| {
            (*role == tcl_registry::ArgRole::Name).then_some(usize::from(*index))
        })?
        .checked_add(layout.member_word)?
        .checked_add(1)?;
    let worker_receiver = if entry.receiver == SourceMethodReceiver::Class && !layout.class_delegate
    {
        tcl_registry::definer::DefinitionReceiver::Class
    } else {
        layout.worker_receiver
    };
    let worker =
        original_definition_member_target(grammar, layout.member, worker_receiver, state, context)?;
    SourceDefinitionMethodReferencePhase::capture(DefinitionMethodReferenceCapture {
        receiver: entry.receiver,
        worker,
        invocation: entry.declaration().clone(),
        operand: words.words().get(name_at)?.clone(),
        name: entry.name.clone(),
        entry: Some(entry.clone()),
        config: context.config,
    })
}

fn capture_class_block_references(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    block: &ExecutedScriptSource,
    state: &ModuleCommandBindings,
    parameters: tcl_dialect::ParameterGrammar,
    context: SourceExecutionContext<'_>,
) -> Option<Vec<super::definition_method_references::SourceDefinitionMethodReferencePhase>> {
    let map = tcl_lexer::SourceMap::from_image(&block.text).with_base(block.base(), 0, 0);
    let segments = crate::segmenter::segment_commands_image_with_offset_and_config(
        &block.text,
        block.base(),
        context.config,
    )?;
    capture_definition_method_references(
        grammar,
        block,
        &map,
        &segments,
        state,
        DefinitionReferenceScope {
            parameters,
            receiver: SourceMethodReceiver::Class,
        },
        context,
    )
}

struct DefinitionReferenceInvocation {
    receiver: SourceMethodReceiver,
    worker: super::SourceCommandTarget,
    site: CommandAllocationSite,
    config: tcl_lexer::LexerConfig,
}

fn capture_reference_operands(
    phase: &DefinitionReferenceInvocation,
    words: &crate::ir::CommandTokens,
    values: &[crate::registry_invocation::EffectiveInvocationWord],
    entries: &SourceReceiverMethodEntries,
) -> Option<Vec<super::definition_method_references::SourceDefinitionMethodReferencePhase>> {
    use super::definition_method_references::{
        DefinitionMethodReferenceCapture, SourceDefinitionMethodReferencePhase,
    };
    words
        .words()
        .iter()
        .zip(values)
        .skip(1)
        .map(|(operand, value)| {
            let name = value.as_registry_word().literal()?;
            SourceDefinitionMethodReferencePhase::capture(DefinitionMethodReferenceCapture {
                receiver: phase.receiver,
                worker: phase.worker.clone(),
                invocation: phase.site.clone(),
                operand: operand.clone(),
                name: name.to_owned(),
                entry: entries.get(&(phase.receiver, name.to_owned())).cloned(),
                config: phase.config,
            })
        })
        .collect()
}

fn retained_class_entries(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    definition: &ExecutedScriptSource,
    map: &tcl_lexer::SourceMap<'_>,
    segments: &[crate::segmenter::SegmentedCommand],
    dialect: Option<tcl_registry::InvocationDialect>,
    parameters: tcl_dialect::ParameterGrammar,
    context: SourceExecutionContext<'_>,
) -> RetainedClassEntries {
    let lifecycle = retained_lifecycle_entries(
        grammar, definition, map, segments, dialect, parameters, context,
    );
    let lifecycle_entries_closed = lifecycle.is_some();
    let lifecycle = lifecycle.unwrap_or_default();
    RetainedClassEntries {
        inherited_classes: Arc::default(),
        instance_methods: closed_instance_methods(grammar, map, segments, dialect, context),
        constructor_entry: lifecycle.constructor,
        destructor_entry: lifecycle.destructor,
        lifecycle_entries_closed,
        receiver_method_entries: retained_method_entries(
            grammar, definition, map, segments, dialect, parameters, context,
        ),
    }
}

fn retained_method_entries(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    definition: &ExecutedScriptSource,
    map: &tcl_lexer::SourceMap<'_>,
    segments: &[crate::segmenter::SegmentedCommand],
    dialect: Option<tcl_registry::InvocationDialect>,
    parameters: tcl_dialect::ParameterGrammar,
    context: SourceExecutionContext<'_>,
) -> Arc<SourceReceiverMethodEntries> {
    let mut entries = super::BTreeMap::new();
    for segment in segments {
        if let Some(block) =
            class_definition_block(grammar, definition, map, segment, dialect, context)
        {
            let block_map =
                tcl_lexer::SourceMap::from_image(&block.text).with_base(block.base(), 0, 0);
            let Some(inner) = crate::segmenter::segment_commands_image_with_offset_and_config(
                &block.text,
                block.base(),
                context.config,
            ) else {
                continue;
            };
            for ((_, name), mut entry) in retained_method_entries(
                grammar, &block, &block_map, &inner, dialect, parameters, context,
            )
            .as_ref()
            .clone()
            {
                entry.receiver = SourceMethodReceiver::Class;
                entries.insert((SourceMethodReceiver::Class, name), entry);
            }
            continue;
        }
        apply_declared_visibility(grammar, map, segment, dialect, context, &mut entries);
        if let Some(entry) = retained_method_entry(
            grammar, definition, map, segment, dialect, parameters, context,
        ) {
            entries.insert((entry.receiver, entry.name.clone()), entry);
        }
    }
    Arc::new(entries)
}

// Visibility changes only a method already installed at this definition
// phase. A later method declaration starts from its native default again.
fn apply_declared_visibility(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    map: &tcl_lexer::SourceMap<'_>,
    segment: &crate::segmenter::SegmentedCommand,
    dialect: Option<tcl_registry::InvocationDialect>,
    context: SourceExecutionContext<'_>,
    entries: &mut SourceReceiverMethodEntries,
) {
    let words = crate::ir::CommandTokens::from_segmented(map, context.config, segment);
    let values = source_effective_words(words.words(), dialect, None);
    let Some(effect) = values
        .first()
        .and_then(|word| word.as_registry_word().literal())
        .and_then(|head| grammar.member(head))
        .and_then(|member| member.visibility_effect)
    else {
        return;
    };
    for name in values
        .iter()
        .skip(1)
        .filter_map(|word| word.as_registry_word().literal())
    {
        if let Some(entry) = entries.get_mut(&(SourceMethodReceiver::Instance, name.to_owned())) {
            entry.exported = effect == tcl_registry::definer::MemberVisibility::Exported;
        }
    }
}

fn retained_method_entry(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    definition: &ExecutedScriptSource,
    map: &tcl_lexer::SourceMap<'_>,
    segment: &crate::segmenter::SegmentedCommand,
    dialect: Option<tcl_registry::InvocationDialect>,
    parameters: tcl_dialect::ParameterGrammar,
    context: SourceExecutionContext<'_>,
) -> Option<SourceReceiverMethodEntry> {
    let words = crate::ir::CommandTokens::from_segmented(map, context.config, segment);
    let values = source_effective_words(words.words(), dialect, None);
    let head = values.first()?.as_registry_word().literal()?;
    let arguments = values
        .iter()
        .skip(1)
        .map(|word| word.as_registry_word().literal())
        .collect::<Option<Vec<_>>>()?;
    let query = dialect
        .and_then(tcl_registry::InvocationDialect::authoring_query)
        .map(|query| query.with_realm(context.realm));
    let layout = grammar.receiver_method_layout(head, &arguments, query)?;
    let receiver = match layout.receiver {
        tcl_registry::definer::DefinitionReceiver::Instance => SourceMethodReceiver::Instance,
        tcl_registry::definer::DefinitionReceiver::Class => SourceMethodReceiver::Class,
    };
    let name_at = layout.member.arg_roles.iter().find_map(|(index, role)| {
        (*role == tcl_registry::ArgRole::Name)
            .then_some(usize::from(*index) + 1 + layout.member_word)
    })?;
    let name = values.get(name_at)?.as_registry_word().literal()?;
    let name_word = words.words().get(name_at)?;
    let body = deferred_method_body(
        grammar, definition, map, segment, dialect, parameters, context,
    )?;
    let (declaration, _, source) = body.executed_script.as_ref()?;
    Some(SourceReceiverMethodEntry {
        name: name.to_owned(),
        name_source: name_word.source().clone(),
        receiver,
        exported: grammar.member_default_exported(name),
        declaring_class: None,
        class_delegate: layout.class_delegate,
        delegate_allocation: None,
        entry: super::SourceConstructorEntry {
            declaration: declaration.clone(),
            body: source.clone(),
            formals: body
                .parameters
                .iter()
                .map(|formal| (formal.name.clone(), formal.default.clone()))
                .collect(),
            frame: crate::var_resolve::VariableExecutionFrame::ReceiverMethod {
                identity: super::source_activation_name(
                    body.source_origin.as_ref(),
                    &body.identity,
                    body.implementation_generation,
                ),
            },
        },
    })
}

#[derive(Default)]
struct RetainedLifecycleEntries {
    constructor: Option<Arc<super::SourceConstructorEntry>>,
    destructor: Option<Arc<super::SourceConstructorEntry>>,
}

fn retained_lifecycle_entries(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    definition: &ExecutedScriptSource,
    map: &tcl_lexer::SourceMap<'_>,
    segments: &[crate::segmenter::SegmentedCommand],
    dialect: Option<tcl_registry::InvocationDialect>,
    parameters: tcl_dialect::ParameterGrammar,
    context: SourceExecutionContext<'_>,
) -> Option<RetainedLifecycleEntries> {
    let mut selected = RetainedLifecycleEntries::default();
    for segment in segments {
        let words = crate::ir::CommandTokens::from_segmented(map, context.config, segment);
        let values = source_effective_words(words.words(), dialect, None);
        let head = values.first()?.as_registry_word().literal()?;
        let member = grammar.member(head)?;
        let arguments = values
            .iter()
            .skip(1)
            .map(|word| word.as_registry_word().literal())
            .collect::<Option<Vec<_>>>()?;
        if grammar
            .single_native_inherited_class(member, &arguments, dialect)
            .is_some()
        {
            continue;
        }
        if !grammar.preserves_local_constructor_entry(member) {
            return None;
        }
        let constructor = grammar.is_instance_constructor(member);
        if !constructor && !grammar.is_instance_destructor(member) {
            continue;
        }
        let body_at = member.arg_roles.iter().find_map(|(index, role)| {
            (*role == tcl_registry::ArgRole::Body).then_some(usize::from(*index) + 1)
        })?;
        let text = values.get(body_at)?.as_registry_word().literal()?;
        if grammar.native_lifecycle_body_disposition(member, text.as_bytes())
            == Some(tcl_registry::definer::NativeLifecycleBodyDisposition::Removed)
        {
            if constructor {
                selected.constructor = None;
            } else {
                selected.destructor = None;
            }
            continue;
        }
        let body = deferred_method_body(
            grammar, definition, map, segment, dialect, parameters, context,
        )?;
        let (_, _, source) = body.executed_script.as_ref()?;
        let declaration = CommandAllocationSite {
            source: Arc::clone(&definition.origin),
            offset: segment.span.start(),
        };
        let entry = Arc::new(super::SourceConstructorEntry {
            declaration,
            body: source.clone(),
            formals: body
                .parameters
                .iter()
                .map(|formal| (formal.name.clone(), formal.default.clone()))
                .collect(),
            frame: crate::var_resolve::VariableExecutionFrame::ReceiverMethod {
                identity: super::source_activation_name(
                    body.source_origin.as_ref(),
                    &body.identity,
                    body.implementation_generation,
                ),
            },
        });
        if constructor {
            selected.constructor = Some(entry);
        } else {
            selected.destructor = Some(entry);
        }
    }
    Some(selected)
}

// This is a complete name inventory only for the already-validated, closed
// instance-definition grammar. Visibility is deliberately not a call licence.
fn closed_instance_methods(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    map: &tcl_lexer::SourceMap<'_>,
    segments: &[crate::segmenter::SegmentedCommand],
    dialect: Option<tcl_registry::InvocationDialect>,
    context: SourceExecutionContext<'_>,
) -> Option<Arc<super::BTreeSet<String>>> {
    use tcl_registry::definer::{BuiltinMethodReceiver, MemberConstructionEffect};
    let mut names = grammar
        .builtin_object_methods
        .iter()
        .filter(|method| method.receiver == BuiltinMethodReceiver::AnyObject)
        .map(|method| method.name.to_owned())
        .collect::<super::BTreeSet<_>>();
    for segment in segments {
        let words = crate::ir::CommandTokens::from_segmented(map, context.config, segment);
        let values = source_effective_words(words.words(), dialect, None);
        let head = values.first()?.as_registry_word().literal()?;
        let member = grammar.member(head)?;
        let arguments = values
            .iter()
            .skip(1)
            .map(|word| word.as_registry_word().literal())
            .collect::<Option<Vec<_>>>()?;
        if grammar
            .single_native_inherited_class(member, &arguments, dialect)
            .is_some()
        {
            continue;
        }
        if grammar.construction_member_effect(member)
            == MemberConstructionEffect::DeferredInstanceMethod
            || grammar.native_classmethod_declaration(
                member,
                dialect
                    .and_then(tcl_registry::InvocationDialect::authoring_query)
                    .map(|query| query.with_realm(context.realm)),
            )
        {
            let name_at = member.arg_roles.iter().find_map(|(index, role)| {
                (*role == tcl_registry::ArgRole::Name).then_some(usize::from(*index) + 1)
            })?;
            let name = values.get(name_at)?.as_registry_word().literal()?;
            if grammar.unknown_dispatch_method == Some(name) {
                return None;
            }
            names.insert(name.to_owned());
        } else if member.visibility_effect.is_some() {
            // Native visibility metadata cannot add or remove method names.
        } else if member
            .arg_roles
            .iter()
            .any(|(_, role)| *role == tcl_registry::ArgRole::Name)
            || member.all_args_ref.is_some()
            || member.retraction.is_some()
            || member.visibility_effect.is_some()
            || member.slot.is_some()
        {
            return None;
        }
    }
    Some(Arc::new(names))
}

fn closed_definition_member(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    definition: &ExecutedScriptSource,
    map: &tcl_lexer::SourceMap<'_>,
    segment: &crate::segmenter::SegmentedCommand,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> bool {
    let words = crate::ir::CommandTokens::from_segmented(map, context.config, segment);
    let values = source_effective_words(words.words(), state.baseline.dialect, None);
    let Some(head) = values
        .first()
        .and_then(|word| word.as_registry_word().literal())
    else {
        return false;
    };
    let query = state
        .baseline
        .dialect
        .and_then(tcl_registry::InvocationDialect::authoring_query)
        .map(|query| query.with_realm(context.realm));
    let arguments = values
        .iter()
        .skip(1)
        .filter_map(|word| word.as_registry_word().literal())
        .collect::<Vec<_>>();
    if arguments.len() + 1 != values.len() {
        return false;
    }
    let Some(outer) = grammar.member(head) else {
        return false;
    };
    if grammar
        .single_native_inherited_class(outer, &arguments, state.baseline.dialect)
        .is_some()
    {
        return state
            .closed_inherited_member(grammar, outer, &values, context)
            .is_some();
    }
    if let Some(block) = class_definition_block(
        grammar,
        definition,
        map,
        segment,
        state.baseline.dialect,
        context,
    ) {
        if !original_definition_member(
            grammar,
            outer,
            tcl_registry::definer::DefinitionReceiver::Instance,
            state,
            context,
        ) {
            return false;
        }
        return closed_class_method_block(grammar, &block, state, context);
    }
    let layout = grammar.receiver_method_layout(head, &arguments, query);
    let (member, shift, receiver) = layout.map_or(
        (
            outer,
            0,
            tcl_registry::definer::DefinitionReceiver::Instance,
        ),
        |layout| (layout.member, layout.member_word, layout.worker_receiver),
    );
    if shift != 0
        && !original_definition_member(
            grammar,
            outer,
            tcl_registry::definer::DefinitionReceiver::Instance,
            state,
            context,
        )
    {
        return false;
    }
    if (grammar.manufacture_member_effect(member)
        != tcl_registry::definer::MemberManufactureDispatchEffect::PreservesNativeFactory
        && !layout.is_some_and(|layout| layout.class_delegate))
        || !original_definition_member(grammar, member, receiver, state, context)
    {
        return false;
    }
    valid_closed_member_body(grammar, member, &values[shift..], state.baseline.dialect)
}

fn valid_closed_member_body(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    member: &tcl_registry::definer::MemberSpec,
    values: &[crate::registry_invocation::EffectiveInvocationWord],
    dialect: Option<tcl_registry::InvocationDialect>,
) -> bool {
    // Class variable declarations are closed native definition metadata;
    // they select no body or constructor callback during declaration.
    if (member.all_args_var || member.visibility_effect.is_some())
        && grammar.preserves_local_constructor_entry(member)
    {
        return true;
    }
    if member
        .arg_roles
        .iter()
        .find_map(|(index, role)| {
            (*role == tcl_registry::ArgRole::Body).then(|| values.get(usize::from(*index) + 1))
        })
        .flatten()
        .and_then(|word| word.as_registry_word().literal())
        .is_some_and(|body| {
            grammar.native_lifecycle_body_disposition(member, body.as_bytes())
                == Some(tcl_registry::definer::NativeLifecycleBodyDisposition::Removed)
        })
    {
        let expected = member
            .arg_roles
            .iter()
            .map(|(index, _)| usize::from(*index) + 2)
            .max();
        return expected == Some(values.len());
    }
    valid_member_formals(member, values, dialect)
}

fn closed_class_method_block(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    block: &ExecutedScriptSource,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> bool {
    let block_map = tcl_lexer::SourceMap::from_image(&block.text).with_base(block.base(), 0, 0);
    crate::segmenter::segment_commands_image_with_offset_and_config(
        &block.text,
        block.base(),
        context.config,
    )
    .is_some_and(|segments| {
        segments
            .iter()
            .all(|inner| closed_class_method(grammar, &block_map, inner, state, context))
    })
}

fn valid_member_formals(
    member: &tcl_registry::definer::MemberSpec,
    values: &[crate::registry_invocation::EffectiveInvocationWord],
    dialect: Option<tcl_registry::InvocationDialect>,
) -> bool {
    let expected = member
        .arg_roles
        .iter()
        .map(|(index, _)| usize::from(*index) + 2)
        .max()
        .unwrap_or(1);
    if !member
        .arg_roles
        .iter()
        .any(|(_, role)| *role == tcl_registry::ArgRole::Body)
        || values.len() != expected
        || member.arg_roles.iter().any(|(index, role)| {
            *role == tcl_registry::ArgRole::ParamList
                && values
                    .get(usize::from(*index) + 1)
                    .and_then(|word| word.as_registry_word().literal())
                    .is_none_or(|text| {
                        dialect
                            .and_then(tcl_registry::InvocationDialect::parameter_grammar)
                            .is_none_or(|grammar| {
                                tcl_syntax::formal_params::parse_formal_parameters_in(text, grammar)
                                    .is_err()
                            })
                    })
        })
    {
        return false;
    }
    true
}

fn class_definition_block(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    definition: &ExecutedScriptSource,
    map: &tcl_lexer::SourceMap<'_>,
    segment: &crate::segmenter::SegmentedCommand,
    dialect: Option<tcl_registry::InvocationDialect>,
    context: SourceExecutionContext<'_>,
) -> Option<ExecutedScriptSource> {
    let words = crate::ir::CommandTokens::from_segmented(map, context.config, segment);
    let values = source_effective_words(words.words(), dialect, None);
    if values.len() != 2 {
        return None;
    }
    let head = values.first()?.as_registry_word().literal()?;
    if !grammar.is_class_receiver_wrapper(grammar.member(head)?) {
        return None;
    }
    let value = values.get(1)?.as_registry_word().literal()?;
    let site = CommandAllocationSite {
        source: Arc::clone(&definition.origin),
        offset: segment.span.start(),
    };
    Some(ExecutedScriptSource::from_word(
        site,
        0,
        words.words().get(1)?,
        value,
        context.config,
    ))
}

fn closed_class_method(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    map: &tcl_lexer::SourceMap<'_>,
    segment: &crate::segmenter::SegmentedCommand,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> bool {
    let words = crate::ir::CommandTokens::from_segmented(map, context.config, segment);
    let values = source_effective_words(words.words(), state.baseline.dialect, None);
    let Some(head) = values
        .first()
        .and_then(|word| word.as_registry_word().literal())
    else {
        return false;
    };
    let Some(member) = grammar.member(head) else {
        return false;
    };
    if !original_definition_member(
        grammar,
        member,
        tcl_registry::definer::DefinitionReceiver::Class,
        state,
        context,
    ) {
        return false;
    }
    let expected = member
        .arg_roles
        .iter()
        .map(|(at, _)| usize::from(*at) + 2)
        .max();
    values.len() == expected.unwrap_or(0)
        && values
            .iter()
            .all(|word| word.as_registry_word().literal().is_some())
        && member.arg_roles.iter().all(|(at, role)| {
            *role != tcl_registry::ArgRole::ParamList
                || state
                    .baseline
                    .dialect
                    .and_then(tcl_registry::InvocationDialect::parameter_grammar)
                    .is_some_and(|grammar| {
                        tcl_syntax::formal_params::parse_formal_parameters_in(
                            values[usize::from(*at) + 1]
                                .as_registry_word()
                                .literal()
                                .unwrap(),
                            grammar,
                        )
                        .is_ok()
                    })
        })
}

pub(super) fn original_definition_member(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    member: &tcl_registry::definer::MemberSpec,
    receiver: tcl_registry::definer::DefinitionReceiver,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> bool {
    original_definition_member_target(grammar, member, receiver, state, context).is_some()
}

fn original_definition_member_target(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    member: &tcl_registry::definer::MemberSpec,
    receiver: tcl_registry::definer::DefinitionReceiver,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<super::SourceCommandTarget> {
    let query = state
        .baseline
        .dialect
        .and_then(tcl_registry::InvocationDialect::authoring_query)
        .map(|query| query.with_realm(context.realm));
    let lookup = grammar.definition_member_lookup_for_receiver(member, receiver, query)?;
    let namespace_key = state.namespace_for_rooted_operand(lookup.namespace)?;
    let binding = source_binding(state, member.keyword, &namespace_key);
    binding
        .proved_target()
        .filter(|target| {
            target.registry_backed
                && target.prepended.is_empty()
                && target.command == lookup.implementation
                && target.implementation_generation == 0
                && !state.tainted_object_dispatch.contains("*")
                && !state.tainted_object_dispatch.contains(&target.command)
                && !state.source_step_observed()
                && !state.source_execution_observed(target.identity.as_ref())
        })
        .cloned()
}

fn deferred_method_body(
    grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    definition: &ExecutedScriptSource,
    map: &tcl_lexer::SourceMap<'_>,
    segment: &crate::segmenter::SegmentedCommand,
    dialect: Option<tcl_registry::InvocationDialect>,
    parameters: tcl_dialect::ParameterGrammar,
    context: SourceExecutionContext<'_>,
) -> Option<DeferredSourceBody> {
    let config = context.config;
    let words = crate::ir::CommandTokens::from_segmented(map, config, segment);
    let values = source_effective_words(words.words(), dialect, None);
    let head = values.first()?.as_registry_word().literal()?;
    let arguments = values
        .iter()
        .skip(1)
        .map(|word| word.as_registry_word().literal())
        .collect::<Option<Vec<_>>>()?;
    let query = dialect
        .and_then(tcl_registry::InvocationDialect::authoring_query)
        .map(|query| query.with_realm(context.realm));
    let layout = grammar.receiver_method_layout(head, &arguments, query);
    let (member, shift) = layout.map_or((grammar.member(head)?, 0), |layout| {
        (layout.member, layout.member_word)
    });
    let find = |role| {
        member.arg_roles.iter().find_map(|(index, declared)| {
            (*declared == role).then_some(usize::from(*index) + 1 + shift)
        })
    };
    let body_at = find(tcl_registry::ArgRole::Body)?;
    let value = values.get(body_at)?.as_registry_word().literal()?;
    if grammar.native_lifecycle_body_disposition(member, value.as_bytes())
        == Some(tcl_registry::definer::NativeLifecycleBodyDisposition::Removed)
    {
        return None;
    }
    let formals = if let Some(params_at) = find(tcl_registry::ArgRole::ParamList) {
        let text = values.get(params_at)?.as_registry_word().literal()?;
        tcl_syntax::formal_params::parse_formal_parameters_in(text, parameters).ok()?
    } else {
        Vec::new()
    };
    let word = words.words().get(body_at)?;
    let site = CommandAllocationSite {
        source: Arc::clone(&definition.origin),
        offset: segment.span.start(),
    };
    let source = ExecutedScriptSource::from_word(site.clone(), body_at - 1, word, value, config);
    let identity = site.variable_storage_identity();
    let body = DeferredSourceBody {
        realm: context.realm,
        identity,
        implementation_generation: 0,
        // Native member grammar seals a declaration scope here. Actual method
        // publication and receiver allocation retain independent owners.
        implementation_allocation: None,
        source_origin: Some(Arc::clone(&source.origin)),
        executed_script: Some((site, body_at - 1, source.clone())),
        executed_word: Some((Arc::clone(&definition.origin), word.source().span)),
        source: source.try_text().ok()?.to_owned(),
        offset: source.base(),
        namespace: "::".to_owned(),
        namespace_key: context.namespace_identity(),
        event: None,
        receiver_method: true,
        future_frame: None,
        parameters: formals,
        statics: None,
    };
    Some(body)
}

#[cfg(test)]
mod tests {
    use super::SourceCommandBindings;
    use crate::command_binding::SourceAnalysisOptions;
    use tcl_registry::CommandRegistry;

    #[test]
    fn visibility_metadata_keeps_original_entries_and_definition_order() {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        for (members, name, expected) in [
            (
                "method Upper {} {return ORIGINAL}; export Upper",
                "Upper",
                true,
            ),
            (
                "export Upper; method Upper {} {return ORIGINAL}",
                "Upper",
                false,
            ),
            (
                "method ping {} {return ORIGINAL}; unexport ping",
                "ping",
                false,
            ),
            (
                "unexport ping; method ping {} {return ORIGINAL}",
                "ping",
                true,
            ),
            (
                "method Upper {} {return FIRST}; export Upper; method Upper {} {return ORIGINAL}",
                "Upper",
                false,
            ),
            (
                "export absent; method ping {} {return ORIGINAL}",
                "ping",
                true,
            ),
        ] {
            let source = format!("oo::class create C {{{members}}}; C new");
            let bindings = SourceCommandBindings::analyse_with_options(
                &source,
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                &registry,
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
            let offset = u32::try_from(source.rfind("C new").unwrap()).unwrap();
            let binding = bindings.invocation_at_source("C", offset);
            let entry = binding
                .class_definition_method_entries()
                .and_then(|(_, entries)| {
                    entries.get(&(super::SourceMethodReceiver::Instance, name.to_owned()))
                })
                .expect("native visibility leaves the original method entry installed");
            assert_eq!(entry.is_exported(), expected, "{source}");
            assert_eq!(entry.body().text.try_text().unwrap(), "return ORIGINAL");
        }
    }

    #[test]
    fn native_classmethod_inheritance_retains_the_original_delegate_owner() {
        for dialect in ["tcl9.0", "tcl9.1"] {
            let profile = tcl_dialect::DialectProfile::find(dialect).unwrap();
            let registry = CommandRegistry::build_default().project_for_profile(profile);
            for (source, expected) in [
                (
                    "oo::class create Base {classmethod find {} {return ORIGINAL}}; oo::class create Child {superclass Base}; Child find",
                    true,
                ),
                (
                    "oo::class create Base {self method find {} {return ORIGINAL}}; oo::class create Child {superclass Base}; Child find",
                    false,
                ),
                (
                    "proc ::oo::define::classmethod args {error REPLACED}; oo::class create Base {classmethod find {} {return ORIGINAL}}; oo::class create Child {superclass Base}; Child find",
                    false,
                ),
                (
                    "oo::class create Base {classmethod find {} {return ORIGINAL}}; oo::class create Child {superclass Base}; oo::define Base classmethod find {} {return REPLACED}; Child find",
                    false,
                ),
                (
                    "oo::class create Base {classmethod find {} {return ORIGINAL}}; oo::class create Child {superclass Base}; unknownOperation; Child find",
                    false,
                ),
                (
                    "oo::class create Base {classmethod find {} {return ORIGINAL}}; oo::define ::oo::class constructor args {next {*}$args; oo::define Base classmethod find {} {return REPLACED}}; oo::class create Child {superclass Base}; Child find",
                    false,
                ),
            ] {
                let bindings = SourceCommandBindings::analyse_with_options(
                    source,
                    tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                    &registry,
                    SourceAnalysisOptions {
                        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(
                            profile,
                        )),
                        native_compilation:
                            tcl_registry::native_compilation::NativeCompilationContext {
                                mode:
                                    tcl_registry::native_compilation::NativeCompilationMode::Direct,
                                ..Default::default()
                            },
                        ..Default::default()
                    },
                );
                let binding = bindings.invocation_at_source(
                    "Child",
                    u32::try_from(source.rfind("Child find").unwrap()).unwrap(),
                );
                let entry = binding
                    .class_definition_method_entries()
                    .and_then(|(_, entries)| {
                        entries.get(&(super::SourceMethodReceiver::Class, "find".to_owned()))
                    });
                assert_eq!(entry.is_some(), expected, "{dialect}: {source}");
                if let Some(entry) = entry {
                    assert!(entry.is_native_class_delegate());
                    assert_eq!(entry.body().text.try_text().unwrap(), "return ORIGINAL");
                    assert_eq!(
                        entry
                            .declaring_class()
                            .unwrap()
                            .command
                            .trim_start_matches("::"),
                        "Base"
                    );
                    assert_eq!(
                        entry.declaration().offset,
                        u32::try_from(source.find("classmethod").unwrap()).unwrap()
                    );
                }
            }
        }
    }

    #[test]
    fn class_method_entry_requires_both_original_definition_receivers() {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        for (source, expected) in [
            (
                "oo::class create C {method ping {} {return INSTANCE}; self method ping {original} {return CLASS}}; C ping V",
                true,
            ),
            (
                "proc ::oo::define::self args {error REPLACED}; oo::class create C {self method ping {} {return CLASS}}; C ping",
                false,
            ),
            (
                "proc ::oo::objdefine::method args {error REPLACED}; oo::class create C {self method ping {} {return CLASS}}; C ping",
                false,
            ),
            (
                "oo::class create C {self method ping {} {return CLASS}}; C ping [oo::objdefine C method ping args {return NEW}]",
                false,
            ),
        ] {
            let bindings = SourceCommandBindings::analyse_with_options(
                source,
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                &registry,
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
            let offset = u32::try_from(source.rfind("C ping").unwrap()).unwrap();
            let binding = bindings.invocation_at_source("C", offset);
            let selected = binding
                .class_definition_method_entries()
                .and_then(|(_, entries)| {
                    entries.get(&(super::SourceMethodReceiver::Class, "ping".to_owned()))
                });
            assert_eq!(selected.is_some(), expected, "{source}: {binding:#?}");
            if let Some(entry) = selected {
                assert_eq!(entry.body().text.try_text().unwrap(), "return CLASS");
                assert_eq!(entry.formals(), &[("original".to_owned(), None)]);
                assert_eq!(
                    entry.declaration().offset,
                    u32::try_from(source.find("self method").unwrap()).unwrap()
                );
                assert_eq!(entry.receiver(), super::SourceMethodReceiver::Class);
            }
        }
    }

    #[test]
    fn class_block_method_entry_retains_the_original_inner_source() {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        for (source, expected) in [
            (
                "oo::class create C {self {method ping {original} {return CLASS}}}; C ping V",
                true,
            ),
            (
                "proc ::oo::define::self args {error REPLACED}; oo::class create C {self {method ping {original} {return CLASS}}}; C ping V",
                false,
            ),
            (
                "proc ::oo::objdefine::method args {error REPLACED}; oo::class create C {self {method ping {original} {return CLASS}}}; C ping V",
                false,
            ),
            (
                "oo::class create C {self {method ping {original} {return CLASS}}}; C ping [oo::objdefine C method ping args {return NEW}]",
                false,
            ),
            (
                "oo::class create C {self {method ping {original} {return FIRST}; method ping {original} {return CLASS}}}; C ping V",
                true,
            ),
            (
                "oo::class create C {self {method ping {original} {return CLASS}; superclass Other}}; C ping V",
                false,
            ),
        ] {
            let bindings = SourceCommandBindings::analyse_with_options(
                source,
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                &registry,
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
            let offset = u32::try_from(source.rfind("C ping").unwrap()).unwrap();
            let binding = bindings.invocation_at_source("C", offset);
            let selected = binding
                .class_definition_method_entries()
                .and_then(|(_, entries)| {
                    entries.get(&(super::SourceMethodReceiver::Class, "ping".to_owned()))
                });
            assert_eq!(selected.is_some(), expected, "{source}: {binding:#?}");
            if let Some(entry) = selected {
                assert_eq!(entry.body().text.try_text().unwrap(), "return CLASS");
                assert_eq!(entry.formals(), &[("original".to_owned(), None)]);
                assert_eq!(
                    entry.declaration().offset,
                    u32::try_from(source.rfind("method ping").unwrap()).unwrap()
                );
                assert_eq!(
                    entry.name_source().span.start(),
                    u32::try_from(source.rfind("method ping").unwrap() + 7).unwrap()
                );
                assert_eq!(entry.receiver(), super::SourceMethodReceiver::Class);
                assert!(matches!(
                    entry.frame(),
                    crate::var_resolve::VariableExecutionFrame::ReceiverMethod { .. }
                ));
            }
        }
    }

    #[test]
    fn method_source_inventory_is_temporal_and_retains_receiver_kind() {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        for (source, expected) in [
            (
                "oo::class create C {method ping {original} {return FIRST}}; C new",
                Some("return FIRST"),
            ),
            (
                "oo::class create C {method ping {} {return FIRST}}; oo::define C method ping {} {return SECOND}; C new",
                None,
            ),
            (
                "oo::class create C {method ping {} {return FIRST}}; C new [oo::define C method ping {} {return SECOND}]",
                None,
            ),
            (
                "oo::class create C {method ping {} {return FIRST}}; rename C {}; oo::class create C {method ping {} {return SECOND}}; C new",
                Some("return SECOND"),
            ),
        ] {
            let bindings = SourceCommandBindings::analyse_with_options(
                source,
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                &registry,
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
            let offset = u32::try_from(source.rfind("C new").unwrap()).unwrap();
            let binding = bindings.invocation_at_source("C", offset);
            let entries = binding.class_definition_method_entries();
            assert_eq!(
                entries.is_some(),
                expected.is_some(),
                "{source}: {binding:#?}"
            );
            if let Some((target, entries)) = entries {
                let entry = entries
                    .get(&(super::SourceMethodReceiver::Instance, "ping".to_owned()))
                    .unwrap();
                assert_eq!(target.command, "::C");
                assert_eq!(entry.body().text.try_text().unwrap(), expected.unwrap());
                assert_eq!(entry.name(), "ping");
                assert!(matches!(
                    entry.frame(),
                    crate::var_resolve::VariableExecutionFrame::ReceiverMethod { .. }
                ));
                assert!(
                    !entries.contains_key(&(super::SourceMethodReceiver::Class, "ping".to_owned()))
                );
                let declaration = source.rfind("method ping").unwrap();
                assert_eq!(
                    entry.declaration().offset,
                    u32::try_from(declaration).unwrap()
                );
            }
        }
    }

    #[test]
    fn constructor_entry_retains_original_formals_without_proving_the_result_class() {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        for (source, expected) in [
            (
                "oo::class create C {constructor {original} {error CALLBACK}}; C new value",
                true,
            ),
            (
                "oo::class create C {variable held; constructor {original} {error CALLBACK}}; C new value",
                true,
            ),
            (
                "oo::class create C {constructor {original} {}}; C new",
                false,
            ),
            (
                "oo::class create C {constructor {original} {}}; oo::define C constructor {replacement} {}; C new value",
                false,
            ),
            (
                "oo::class create C {constructor {original} {}}; C new [oo::define C constructor {replacement} {}]",
                false,
            ),
            (
                "oo::class create Base {constructor {base} {}}; oo::class create C {superclass Base; constructor {original} {}}; C new value",
                false,
            ),
        ] {
            let bindings = SourceCommandBindings::analyse_with_options(
                source,
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                &registry,
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
            let offset = u32::try_from(source.rfind("C new").unwrap()).unwrap();
            let binding = bindings.invocation_at_source("C", offset);
            let entry = binding.constructor_entry(&registry);
            assert_eq!(entry.is_some(), expected, "{source}: {binding:#?}");
            if let Some((target, entry, payload)) = entry {
                assert_eq!(target.command, "::C");
                assert_eq!(entry.formals(), &[("original".to_owned(), None)]);
                assert_eq!(entry.body().text.try_text().unwrap(), "error CALLBACK");
                assert!(matches!(
                    entry.frame(),
                    crate::var_resolve::VariableExecutionFrame::ReceiverMethod { .. }
                ));
                assert_eq!(payload, 1);
                assert!(binding.proved_construction_result(&registry).is_none());
            }
        }
    }

    #[test]
    fn class_definition_receipt_is_independent_of_constructor_execution() {
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        for (source, expected) in [
            ("oo::abstract create Base {}; Base new", true),
            (
                "oo::abstract create Base {}; rename Base Moved; Moved new",
                true,
            ),
            (
                "oo::class create C {constructor {} {error NO}}; C new",
                true,
            ),
            (
                "oo::abstract create Base {}; oo::objdefine Base export new; Base new",
                false,
            ),
            (
                "oo::abstract create Base {}; oo::objdefine Base method new {} {return YES}; Base new",
                false,
            ),
        ] {
            let bindings = SourceCommandBindings::analyse_with_options(
                source,
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                &registry,
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
            let offset = u32::try_from(source.rfind(';').unwrap() + 2).unwrap();
            let binding = bindings.invocation_at_source("", offset);
            assert_eq!(
                binding.proved_class_definition_factory().is_some(),
                expected,
                "source={source} binding={binding:#?}"
            );
        }
    }

    #[test]
    fn compiled_class_definition_retains_absolute_method_command_identity() {
        let source = "oo::class create C {method m {} {::return 1}}";
        let registry = CommandRegistry::build_default();
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::default(),
            &registry,
            SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(
                    tcl_dialect::TclVersion::V8_6,
                )),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                    frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                    catch_depth: Some(0),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let offset = u32::try_from(source.find("::return").unwrap()).unwrap();
        let binding = bindings.invocation_at_source("::return", offset);
        assert!(
            matches!(
                binding.variable_frame,
                crate::var_resolve::VariableExecutionFrame::ReceiverMethod { .. }
            ),
            "{binding:#?}"
        );
        assert!(
            binding
                .proved_handler_target()
                .is_some_and(|target| target.command == "::return"),
            "{binding:#?}"
        );
    }

    #[test]
    fn constructor_declaration_retains_its_formal_and_exact_read_source() {
        let source = "oo::class create C {constructor {s} {::set re {a+}; ::regexp $re $s}}";
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
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
        let offset = u32::try_from(source.find("::regexp").unwrap()).unwrap();
        let binding = bindings.invocation_at_source("::regexp", offset);
        assert!(binding.proved_handler_target().is_some(), "{binding:#?}");
        assert!(
            bindings
                .variable_accesses
                .values()
                .flatten()
                .any(|access| { access.original_spelling == "$re" })
        );
    }

    #[test]
    fn method_declaration_retains_local_frame_without_receiver_namespace() {
        let source = "oo::class create C {method m {} {global g; ::puts ok}}";
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::default(),
            &CommandRegistry::build_default(),
            SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(
                    tcl_dialect::TclVersion::V9_1,
                )),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..SourceAnalysisOptions::default()
            },
        );
        let global = bindings.invocation_at_source(
            "global",
            u32::try_from(source.find("global").unwrap()).unwrap(),
        );
        assert!(matches!(
            global.variable_frame,
            crate::var_resolve::VariableExecutionFrame::ReceiverMethod { .. }
        ));
        assert!(!global.variable_context.namespace_known);
        assert!(global.unknown);
        assert!(
            global.targets.iter().any(|target| target.registry_backed
                && target.command.trim_start_matches("::") == "global")
        );
        let offset = u32::try_from(source.find("global").unwrap()).unwrap();
        let config = tcl_lexer::LexerConfig::default();
        let map = tcl_lexer::SourceMap::new(source);
        let segment =
            crate::segmenter::segment_commands_with_offset_and_config("global g", offset, config)
                .into_iter()
                .next()
                .unwrap();
        let mut tokens = crate::ir::CommandTokens::from_segmented(&map, config, &segment);
        tokens.source_binding = Some(global);
        let registry = CommandRegistry::build_default();
        let footprint = crate::registry_invocation::possible_variable_alias_transitions(
            &registry, None, &tokens,
        )
        .unwrap();
        assert!(footprint.unknown_residual());
        assert!(footprint.aliases().any(|alias| matches!(
            alias.target,
            tcl_registry::VariableAliasTarget::Global { .. }
        ) && alias.local.literal() == Some("g")));
        assert!(
            crate::registry_invocation::normal_transfer_invocation(&registry, None, &tokens)
                .is_none()
        );
    }
}
