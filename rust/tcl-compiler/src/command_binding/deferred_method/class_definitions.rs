// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Class definitions in the may-state: their dependencies, inherited entries and recording.

use super::super::{
    Arc, InvocationFacts, ModuleCommandBindings, SourceExecutionContext, SourceNativeInvocation,
    source_binding, source_effective_words,
};
use super::closed_members::{original_definition_member, original_definition_member_target};
use super::{
    RetainedClassEntries, SourceClassDelegateAllocation, SourceMethodReceiver,
    SourceReceiverMethodEntry,
};
use tcl_core_types::NameBytes;

impl ModuleCommandBindings {
    /// A native classmethod body belongs to the original delegated receiver.
    /// This closes only that plain body entry, without creating a `TclOO` object
    /// receipt or granting authority to self-dispatch inside the body.
    pub(in super::super) fn native_class_delegate_entry_is_current(
        &self,
        receiver: &super::super::SourceCommandTarget,
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

    /// An ordinary class-object method is selected from its own retained
    /// table. Native classmethod delegates keep their separate allocation and
    /// worker checks; an own entry does not acquire a delegate receipt.
    pub(in super::super) fn own_class_object_method_entry_is_current(
        &self,
        receiver: &super::super::SourceCommandTarget,
        entry: &SourceReceiverMethodEntry,
    ) -> bool {
        !entry.class_delegate
            && entry.receiver == SourceMethodReceiver::Class
            && entry.exported
            && receiver.kind == super::super::BindingKind::Class
            && receiver.prepended.is_empty()
            && entry.declaring_class.as_ref() == Some(receiver)
            && !self.opaque_domain
            && !self.tainted_object_dispatch.contains("*")
            && !self.tainted_object_dispatch.contains(&receiver.command)
            && self.retained_target_is_current(receiver)
            && receiver
                .identity
                .as_ref()
                .and_then(|identity| self.class_definitions.get(identity))
                .is_some_and(|definition| {
                    definition.dispatcher.is_none()
                        && definition.implementation_generation
                            == receiver.implementation_generation
                        && self.class_definition_dependencies_hold(definition)
                        && definition.receiver_method_entries.get(&(
                            SourceMethodReceiver::Class,
                            NameBytes::from(entry.original_name.bytes()),
                        )) == Some(entry)
                })
    }

    pub(in super::super) fn class_definition_dependencies_hold(
        &self,
        definition: &super::super::ClassDefinitionReceipt,
    ) -> bool {
        !self.opaque_domain
            && definition
                .custom_metaclass
                .as_ref()
                .is_none_or(|metaclass| {
                    !self.tainted_object_dispatch.contains("*")
                        && !self.tainted_object_dispatch.contains(&metaclass.command)
                        && self.retained_target_is_current(metaclass)
                })
            && definition
                .original_factory
                .as_ref()
                .is_none_or(|factory| factory.is_current(self))
            && definition.forward_method_entries.values().all(|entry| {
                self.baseline
                    .execution_name_policy
                    .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                    .is_some_and(|policy| entry.closed_for_policy(policy))
            })
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
                    && (definition.original_factory.as_ref().is_some_and(|factory| {
                        self.stock_inherited_class(factory.recipe().grammar(), base)
                            .is_some()
                    }) || base
                        .identity
                        .as_ref()
                        .and_then(|identity| self.class_definitions.get(identity))
                        .is_some_and(|receipt| {
                            receipt.dispatcher.is_none()
                                && receipt.implementation_generation
                                    == base.implementation_generation
                        }))
            })
    }

    pub(in super::super) fn closed_inherited_member(
        &self,
        grammar: &tcl_registry::definer::DefinitionBodyGrammar,
        member: &tcl_registry::definer::MemberSpec,
        values: &[crate::registry_invocation::EffectiveInvocationWord],
        context: SourceExecutionContext<'_>,
    ) -> Option<(
        super::super::SourceCommandTarget,
        super::super::ClassDefinitionReceipt,
    )> {
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
        if base.registry_backed {
            return self.stock_inherited_class(grammar, base);
        }
        let identity = base.identity.as_ref()?;
        // An inherited class is an actual object command, not an alias to one.
        let raw = self
            .source_keys(name, &context.namespace_identity())
            .into_iter()
            .find_map(|slot| self.bindings.get(&slot))?;
        if raw.len() != 1
            || !matches!(raw.iter().next(), Some(super::super::MayBinding::Target(target))
            if target.kind == super::super::BindingKind::Class && target.terminal
                && target.prepended.is_empty() && target.token.as_ref() == Some(identity))
        {
            return None;
        }
        let definition = self.class_definitions.get(identity)?;
        // naming.tcloo.original-forward-registration-prefix
        // docs/design/analysis/name-resolution-proofs/tcloo-original-forward-registration-prefix.md
        // The sealed declaration already owns the selected factory and local
        // workers. A compatibility body rescan cannot withdraw that receipt
        // merely because a retained forward has no script body. The Registry
        // grammar is a const descriptor; its selected family, rather than its
        // promoted address, identifies the inherited stock class system.
        // A user metaclass that only derives from a stock one keeps the stock
        // lifecycle open; inheriting it leaves the child's open the same way.
        if definition.dispatcher.is_some()
            || definition.implementation_generation != base.implementation_generation
            || (!definition.lifecycle_entries_closed
                && self.stock_metaclass_of(base, context).is_none())
            || !definition.original_factory.as_ref().is_some_and(|factory| {
                factory.recipe().grammar().family == grammar.family && factory.is_current(self)
            })
            || (definition.instance_methods.is_none()
                && self.stock_metaclass_of(base, context).is_none())
            || !self.class_definition_dependencies_hold(definition)
            || self.tainted_object_dispatch.contains(&base.command)
        {
            return None;
        }
        Some((base.clone(), definition.clone()))
    }

    /// The stock metaclass a user metaclass creates through, when the user
    /// class adds nothing that could change that creation: its only
    /// superclass is a registry metaclass (`superclass oo::class`) or another
    /// such user metaclass, and it has no constructor, destructor, dispatcher
    /// or class-side member. Then `Meta create NAME SCRIPT` runs the inherited
    /// `create` (tclsh 8.6.18, 9.0.4, 9.1.0: `info object class NAME` is
    /// `Meta`, and NAME is a class with SCRIPT's members), so the call is
    /// walked as that stock creation.
    pub(in super::super) fn stock_metaclass_creation_target(
        &self,
        target: &super::super::SourceCommandTarget,
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        context: SourceExecutionContext<'_>,
    ) -> Option<super::super::SourceCommandTarget> {
        let stock = self.stock_metaclass_of(target, context)?;
        let definition = self.class_definitions.get(target.identity.as_ref()?)?;
        let grammar = definition.original_factory.as_ref()?.recipe().grammar();
        let selector = effective.get(1)?.as_registry_word().literal()?;
        let manufacturer = grammar.manufacturer(selector)?;
        // The created class must be named by a written word, so the receipt
        // this call makes is found by that name afterwards.
        let name = usize::from(manufacturer.names_instance_at?).checked_add(1)?;
        effective.get(name)?.as_registry_word().literal()?;
        (manufacturer.visibility == tcl_registry::definer::MemberVisibility::Exported)
            .then_some(stock)
    }

    /// The registry metaclass `target` derives from without adding a
    /// lifecycle, dispatcher or class-side member anywhere on the chain. A
    /// receipt lists its whole inherited chain, so every user class in it
    /// must add nothing and every registry class must be that one metaclass.
    pub(in super::super) fn stock_metaclass_of(
        &self,
        target: &super::super::SourceCommandTarget,
        context: SourceExecutionContext<'_>,
    ) -> Option<super::super::SourceCommandTarget> {
        let mut pending = vec![target.clone()];
        let mut seen = Vec::new();
        let mut stock: Option<super::super::SourceCommandTarget> = None;
        while let Some(current) = pending.pop() {
            if current.kind != super::super::BindingKind::Class
                || current.registry_backed
                || !current.prepended.is_empty()
                || self.tainted_object_dispatch.contains("*")
                || self.tainted_object_dispatch.contains(&current.command)
            {
                return None;
            }
            let identity = current.identity.clone()?;
            if seen.contains(&identity) {
                continue;
            }
            seen.push(identity.clone());
            let definition = self.class_definitions.get(&identity)?;
            let grammar = definition.original_factory.as_ref()?.recipe().grammar();
            if definition.inherited_classes.is_empty()
                || definition.dispatcher.is_some()
                || definition.constructor_entry.is_some()
                || definition.destructor_entry.is_some()
                || definition.implementation_generation != current.implementation_generation
                // A metaclass's own instance methods and forwards are methods
                // of the classes it makes, so a creation through it adds them.
                || !definition.receiver_method_entries.is_empty()
                || !definition.forward_method_entries.is_empty()
                || !self.class_definition_dependencies_hold(definition)
            {
                return None;
            }
            // `create` on this class resolves through its own metaclass first.
            if let Some(metaclass) = &definition.custom_metaclass {
                pending.push(metaclass.clone());
            }
            for base in definition.inherited_classes.iter() {
                if !base.registry_backed {
                    pending.push(base.clone());
                    continue;
                }
                let (selected, _) = self.stock_inherited_class(grammar, base)?;
                let query = self
                    .baseline
                    .dialect
                    .and_then(tcl_registry::InvocationDialect::authoring_query)
                    .map(|query| query.with_realm(context.realm()));
                let spec = context.registry.get_for_surface(&selected.command, query)?;
                if !spec.traits.contains(tcl_registry::Traits::IS_OO_METACLASS)
                    || spec
                        .definition_body
                        .is_none_or(|stock| stock.family != grammar.family)
                    || stock
                        .as_ref()
                        .is_some_and(|previous| previous.command != selected.command)
                {
                    return None;
                }
                stock = Some(selected);
            }
        }
        stock
    }

    /// Stamp the class this stock creation named with the user metaclass the
    /// call went through. Only the named result is stamped: a definition
    /// script may make stock classes of its own (`oo::class create Inner`).
    /// `None` when that class cannot be found, so the caller can withdraw.
    pub(in super::super) fn mark_custom_metaclass_creation(
        &mut self,
        stock: &super::super::SourceCommandTarget,
        metaclass: &super::super::SourceCommandTarget,
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        context: SourceExecutionContext<'_>,
    ) -> Option<()> {
        let grammar = self
            .class_definitions
            .get(metaclass.identity.as_ref()?)?
            .original_factory
            .as_ref()?
            .recipe()
            .grammar();
        let selector = effective.get(1)?.as_registry_word().literal()?;
        let index = usize::from(grammar.manufacturer(selector)?.names_instance_at?);
        let name = effective
            .get(index.checked_add(1)?)?
            .as_registry_word()
            .literal()?;
        let created = source_binding(self, name, &context.namespace_identity())
            .proved_target()?
            .clone();
        let receipt =
            Arc::make_mut(&mut self.class_definitions).get_mut(created.identity.as_ref()?)?;
        if receipt.factory != stock.command || receipt.custom_metaclass.is_some() {
            return None;
        }
        receipt.custom_metaclass = Some(metaclass.clone());
        Some(())
    }

    /// A shipped base class (`oo::object`, `oo::class`) inherits members the
    /// source does not declare, so the child's instance method set stays open.
    fn stock_inherited_class(
        &self,
        grammar: &tcl_registry::definer::DefinitionBodyGrammar,
        base: &super::super::SourceCommandTarget,
    ) -> Option<(
        super::super::SourceCommandTarget,
        super::super::ClassDefinitionReceipt,
    )> {
        if !base.registry_backed
            || !base.prepended.is_empty()
            || base.implementation_generation != 0
            || !grammar
                .default_construction_lookup_dependencies()
                .is_some_and(|stock| {
                    stock.iter().any(|command| {
                        tcl_registry::command_descriptor_labels_match(&base.command, command)
                    })
                })
            || self.tainted_object_dispatch.contains("*")
            || self.tainted_object_dispatch.contains(&base.command)
            || !self.retained_target_is_current(base)
        {
            return None;
        }
        Some((
            base.clone(),
            super::super::ClassDefinitionReceipt {
                factory: base.command.clone(),
                original_factory: None,
                implementation_generation: base.implementation_generation,
                dispatcher: None,
                dispatcher_methods: Arc::default(),
                instance_methods: None,
                instance_variables: None,
                constructor_entry: None,
                constructor_provider: None,
                destructor_entry: None,
                destructor_provider: None,
                lifecycle_entries_closed: false,
                receiver_method_entries: Arc::default(),
                forward_method_entries: Arc::default(),
                inherited_classes: Arc::default(),
                single_superclass: false,
                custom_metaclass: None,
            },
        ))
    }

    pub(super) fn inherit_class_entries(
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
            if mixin
                && (!inherited.forward_method_entries.is_empty()
                    || !entries.forward_method_entries.is_empty())
            {
                return false;
            }
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
            entries.single_superclass = !mixin;
            dependencies.push(base);
            dependencies.extend(inherited.inherited_classes.iter().cloned());
            match (&mut entries.instance_methods, inherited.instance_methods) {
                (Some(local), Some(base)) => Arc::make_mut(local).extend(base.iter().cloned()),
                (local, None) => *local = None,
                (None, Some(_)) => {}
            }
            // Implicit variables belong to the selected implementation's
            // declaring class. Inheriting a method does not merge its local
            // resolver inventory into the child's own declaration table.
            entries.lifecycle_entries_closed &= inherited.lifecycle_entries_closed;
            if let Some(entry) = &inherited.constructor_entry
                && (mixin || entries.constructor_entry.is_none())
            {
                let Some(provider) = &inherited.constructor_provider else {
                    return false;
                };
                entries.constructor_entry = Some(Arc::clone(entry));
                entries.constructor_provider = Some(provider.clone());
            }
            if let Some(entry) = &inherited.destructor_entry
                && (mixin || entries.destructor_entry.is_none())
            {
                let Some(provider) = &inherited.destructor_provider else {
                    return false;
                };
                entries.destructor_entry = Some(Arc::clone(entry));
                entries.destructor_provider = Some(provider.clone());
            }
            let local = Arc::make_mut(&mut entries.receiver_method_entries);
            for (name, entry) in inherited.forward_method_entries.iter() {
                if !local.contains_key(&(SourceMethodReceiver::Instance, name.clone())) {
                    Arc::make_mut(&mut entries.forward_method_entries)
                        .entry(name.clone())
                        .or_insert_with(|| entry.clone());
                }
            }
            for (key, entry) in inherited.receiver_method_entries.iter() {
                if (key.0 == SourceMethodReceiver::Instance
                    && !entries.forward_method_entries.contains_key(&key.1))
                    || (!mixin && entry.is_native_class_delegate())
                {
                    local.entry(key.clone()).or_insert_with(|| entry.clone());
                }
            }
        }
        true
    }

    pub(super) fn record_class_definition(
        &mut self,
        facts: &InvocationFacts,
        native: SourceNativeInvocation<'_>,
        context: SourceExecutionContext<'_>,
        mut entries: RetainedClassEntries,
        original_factory: super::super::registered_class_factory::OriginalClassFactoryState,
    ) -> Option<super::super::SourceCommandTarget> {
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
                    ) => subject.argument_index(),
                    _ => None,
                })
        })?;
        let input = native
            .original_variable_operands
            .input(name, &self.source_variables)?;
        let binding = super::super::source_binding_from_original_input(
            self,
            input,
            &context.namespace_identity(),
        )?;
        let created = binding.proved_target()?;
        let factory = native.target;
        let Some(identity) = &created.identity else {
            return None;
        };
        for entry in Arc::make_mut(&mut entries.receiver_method_entries).values_mut() {
            if entry.declaring_class.is_none() {
                entry.declaring_class = Some(created.clone());
                if entry.class_delegate {
                    let allocation = created.identity.as_ref()?.allocation.as_ref()?;
                    if allocation.incarnation == super::super::AllocationIncarnation::RepeatedFresh
                    {
                        return None;
                    }
                    let grammar = original_factory.recipe().grammar();
                    let query = self
                        .baseline
                        .dialect
                        .and_then(tcl_registry::InvocationDialect::authoring_query)
                        .map(|query| query.with_realm(context.realm()));
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
            super::super::ClassDefinitionReceipt {
                factory: factory.command.clone(),
                original_factory: Some(original_factory),
                implementation_generation: created.implementation_generation,
                dispatcher: None,
                instance_methods: entries.instance_methods,
                instance_variables: entries.instance_variables,
                dispatcher_methods: Arc::default(),
                constructor_provider: entries.constructor_entry.as_ref().map(|_| {
                    entries
                        .constructor_provider
                        .unwrap_or_else(|| created.clone())
                }),
                constructor_entry: entries.constructor_entry,
                destructor_provider: entries.destructor_entry.as_ref().map(|_| {
                    entries
                        .destructor_provider
                        .unwrap_or_else(|| created.clone())
                }),
                destructor_entry: entries.destructor_entry,
                lifecycle_entries_closed: entries.lifecycle_entries_closed,
                receiver_method_entries: entries.receiver_method_entries,
                forward_method_entries: entries.forward_method_entries,
                inherited_classes: entries.inherited_classes,
                single_superclass: entries.single_superclass,
                custom_metaclass: None,
            },
        );
        Some(created.clone())
    }
    /// The instance methods a closed class configuration adds, when that is
    /// all it does: no other member, no redefinition, and no method the
    /// grammar dispatches every unmatched name to.
    fn configured_class_methods(
        &self,
        configuration: &super::super::closed_class_configuration::ClosedClassConfiguration,
        context: SourceExecutionContext<'_>,
    ) -> Option<RetainedClassEntries> {
        let definition = &configuration.body;
        let text = definition.text.try_text().ok()?;
        let declares_methods =
            super::super::module_bindings::definition_body_command_tokens(text, context.config)
                .all(|words| {
                    source_effective_words(
                        words.words(),
                        self.baseline.dialect,
                        Some((&self.source_variables, context.registry)),
                    )
                    .first()
                    .and_then(|word| word.as_registry_word().literal())
                    .and_then(|head| configuration.grammar.member(head))
                    .is_some_and(|member| {
                        configuration.grammar.construction_member_effect(member)
                        == tcl_registry::definer::MemberConstructionEffect::DeferredInstanceMethod
                    })
                });
        if !declares_methods {
            return None;
        }
        let map =
            tcl_lexer::SourceMap::from_image(&definition.text).with_base(definition.base(), 0, 0);
        let segments = crate::segmenter::segment_commands_image_with_offset_and_config(
            &definition.text,
            definition.base(),
            context.config,
        )?;
        let policy = self
            .baseline
            .execution_name_policy
            .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)?;
        let entries = super::retained_class_entries(
            configuration.grammar,
            definition,
            &map,
            &segments,
            self.baseline.dialect,
            super::MethodParameterSource {
                parameters: configuration.parameters,
                policy,
            },
            context,
        );
        let declares_only_methods = entries.constructor_entry.is_none()
            && entries.destructor_entry.is_none()
            && entries.forward_method_entries.is_empty()
            && entries.inherited_classes.is_empty()
            && entries
                .instance_variables
                .as_ref()
                .is_none_or(|variables| variables.declarations().is_empty())
            && entries
                .receiver_method_entries
                .iter()
                .all(|((receiver, _), entry)| {
                    *receiver == SourceMethodReceiver::Instance && !entry.class_delegate
                });
        // A redefinition replaces a body other classes may inherit or
        // instances may already have dispatched; only new methods are added.
        // A method the grammar dispatches every unmatched name to leaves the
        // class's method table no longer bounding its dispatch.
        let declares_unknown_dispatch =
            configuration
                .grammar
                .unknown_dispatch_method
                .is_some_and(|name| {
                    entries
                        .receiver_method_entries
                        .keys()
                        .any(|(_, declared)| declared.as_bytes() == name.as_bytes())
                });
        if !declares_only_methods
            || declares_unknown_dispatch
            || entries.receiver_method_entries.keys().any(|key| {
                configuration
                    .receipt
                    .receiver_method_entries
                    .contains_key(key)
            })
        {
            return None;
        }
        Some(entries)
    }

    /// The class definition a closed class configuration leaves: the receipt
    /// from before it, with the instance methods it declares added. A
    /// configuration that declares anything else (a variable, a lifecycle
    /// body, a forward, a relation, a class method) leaves the class with no
    /// retained definition, as an opaque configuration does. Whether the
    /// definition was kept.
    pub(in super::super) fn retain_configured_class_definition(
        &mut self,
        configuration: &super::super::closed_class_configuration::ClosedClassConfiguration,
        context: SourceExecutionContext<'_>,
    ) -> bool {
        let Some(identity) = configuration.class.identity.clone() else {
            return false;
        };
        let Some(entries) = self.configured_class_methods(configuration, context) else {
            return false;
        };
        // A method table that was not known stays unknown, and so does one
        // this configuration's members leave unknown.
        let mut receipt = configuration.receipt.clone();
        receipt.instance_methods = match (
            entries.instance_methods.as_ref(),
            configuration.receipt.instance_methods.as_ref(),
        ) {
            (Some(declared), Some(retained)) => {
                let mut methods = retained.as_ref().clone();
                methods.extend(declared.iter().cloned());
                Some(Arc::new(methods))
            }
            _ => None,
        };
        let receiver_entries = Arc::make_mut(&mut receipt.receiver_method_entries);
        for (key, entry) in entries.receiver_method_entries.iter() {
            let mut entry = entry.clone();
            entry.declaring_class = Some(configuration.class.clone());
            receiver_entries.insert(key.clone(), entry);
        }
        Arc::make_mut(&mut self.class_definitions).insert(identity.clone(), receipt);
        if let Some(bounded) = &configuration.bounded {
            Arc::make_mut(&mut self.bounded_constructions).insert(identity, bounded.clone());
        }
        // The definer's declared transition taints the configured class, or
        // every class when its operand is not an exact literal; this
        // configuration changed the one class above and nothing else.
        let tainted = Arc::make_mut(&mut self.tainted_object_dispatch);
        if !configuration.tainted {
            tainted.remove(&configuration.class.command);
        }
        if !configuration.tainted_all {
            tainted.remove("*");
        }
        true
    }
}
