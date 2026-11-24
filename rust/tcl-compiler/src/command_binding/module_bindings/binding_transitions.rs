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

//! One command-binding transition applied to the may-state and the table updates it performs.

use super::super::command_identity::{
    AllocationIncarnation, CommandAllocation, CommandAllocationSite, CommandIdentity, CommandToken,
};
use super::super::namespace_context::{CommandKeyQuery, SourceNamespaceKey};
use super::super::{
    BindingKind, MayBinding, ModuleCommandBindings, ResolvedCommandTarget, SourceCommandKey,
    command_publication, is_repeated_fresh_binding, namespace_slots, original_command_table,
};
use crate::alias::is_current_interpreter;
use std::collections::BTreeSet;
use std::sync::Arc;
use tcl_registry::{CommandBindingDefinitionKind, CommandBindingTransition, TransitionSubject};

impl ModuleCommandBindings {
    /// Replace sparse binding entries without detaching a shared state when
    /// every requested value is already present.
    pub(in super::super) fn replace_bindings(
        &mut self,
        bindings: impl IntoIterator<Item = (impl Into<SourceCommandKey>, BTreeSet<MayBinding>)>,
    ) -> bool {
        let replacements: Vec<_> = bindings
            .into_iter()
            .map(|(key, value)| (key.into(), value))
            .filter(|(key, value)| self.bindings.get(key) != Some(value))
            .collect();
        if replacements.is_empty() {
            return false;
        }
        // The actual object-private namespace is not a displayed command
        // name. A command-table mutation cannot prove that its injected
        // dispatcher survived merely from the class incarnation.
        Arc::make_mut(&mut self.object_instances).invalidate_receiver_dispatcher();
        Arc::make_mut(&mut self.bindings).extend(replacements);
        true
    }

    pub(in super::super) fn replace(
        &mut self,
        key: impl Into<SourceCommandKey>,
        bindings: BTreeSet<MayBinding>,
    ) {
        self.replace_bindings([(key, bindings)]);
    }

    /// Join one known-changed binding into the historical may-state. The
    /// ordinary lattice join handles arbitrary branches; a deterministic
    /// registry transition already identifies its exact key and should not
    /// rescan every unchanged binding merely to publish that one delta.
    pub(super) fn join_binding_from(
        &mut self,
        other: &Self,
        key: &(impl CommandKeyQuery + ?Sized),
    ) -> bool {
        let mut joined = self.binding_alternatives(key);
        joined.extend(other.binding_alternatives(key));
        self.replace_bindings([(key.command_key().into_owned(), joined)])
    }

    /// Record a rebound name, detaching the shared history only for a new one.
    pub(in super::super) fn retain_rebound_name(&mut self, key: &SourceCommandKey) {
        if !self.rebound_names.contains(key) {
            self.rebound_names.insert(key.clone());
        }
    }

    pub(in super::super) fn remove(&mut self, key: impl Into<SourceCommandKey>) {
        let key = key.into();
        self.retain_rebound_name(&key);
        let tokens = self
            .bindings
            .get(&key)
            .into_iter()
            .flatten()
            .filter_map(|binding| match binding {
                MayBinding::Target(target) => target.token.clone(),
                MayBinding::Imported(_) | MayBinding::Missing | MayBinding::Unknown => None,
            })
            .collect::<BTreeSet<_>>();
        for token in tokens {
            Arc::make_mut(&mut self.objects).remove(&token);
            let imported = self
                .bindings
                .iter()
                .filter_map(|(name, alternatives)| {
                    alternatives
                        .iter().any(|binding| matches!(binding, MayBinding::Imported(import) if import.origin == token))
                        .then_some(name.clone())
                })
                .collect::<Vec<_>>();
            for name in imported {
                let mut alternatives = self.bindings[&name].clone();
                alternatives.retain(|binding| !matches!(binding, MayBinding::Imported(import) if import.origin == token));
                alternatives.insert(MayBinding::Missing);
                self.replace(name, alternatives);
            }
        }
        self.replace(key, BTreeSet::from([MayBinding::Missing]));
    }

    fn allocate_command(
        &mut self,
        command: &SourceCommandKey,
        offset: u32,
    ) -> Option<CommandAllocation> {
        let site = CommandAllocationSite {
            source: Arc::clone(self.current_source_origin.as_ref()?),
            offset,
        };
        let count = Arc::make_mut(&mut self.allocation_counts)
            .entry((site.clone(), command.clone()))
            .or_default();
        *count = count.saturating_add(1).min(3);
        Some(CommandAllocation {
            slot: command.clone(),
            site,
            incarnation: match *count {
                1 => AllocationIncarnation::First,
                2 => AllocationIncarnation::Second,
                _ => AllocationIncarnation::RepeatedFresh,
            },
            command: crate::command_binding::ModuleCommandBindings::command_key_label(command)
                .unwrap_or_default(),
            namespace: command.holder().into_owned(),
        })
    }

    /// Install a command whose narrower identity this lattice does not track,
    /// created by the operation at `declaration`.
    pub(in super::super) fn install_created_command(
        &mut self,
        key: SourceCommandKey,
        declaration: u32,
    ) {
        let command = Self::command_key_label(&key).unwrap_or_default();
        self.install(
            key,
            MayBinding::Target(ResolvedCommandTarget {
                command,
                prepended: Vec::new(),
                registry_backed: false,
                kind: BindingKind::Command,
                implementation_generation: declaration,
                terminal: true,
                target_lookup: tcl_registry::AliasTargetLookup::Global,
                token: None,
                implementation_allocation: None,
            }),
        );
    }

    pub(in super::super) fn install(
        &mut self,
        key: impl Into<SourceCommandKey>,
        mut implementation: MayBinding,
    ) {
        let key = key.into();
        self.retain_rebound_name(&key);
        let label = crate::command_binding::ModuleCommandBindings::command_key_label(&key)
            .unwrap_or_default();
        if let MayBinding::Target(target) = &mut implementation
            && (!target.registry_backed || target.kind == BindingKind::Alias)
            && target.implementation_allocation.is_none()
        {
            target.implementation_allocation =
                self.allocate_command(&key, target.implementation_generation);
            target.token = Some(CommandIdentity {
                runtime: None,
                origin: label,
                declaration: target.implementation_generation,
                allocation: target.implementation_allocation.clone(),
            });
        }
        let prior = self.binding_alternatives(&key);
        let tokens = prior
            .iter()
            .filter_map(|binding| {
                if let MayBinding::Target(target) = binding {
                    target.token.clone()
                } else {
                    None
                }
            })
            .filter(|token| !self.command_token_replaced_on_install(token, &key))
            .collect::<BTreeSet<_>>();
        for token in &tokens {
            self.remove_command_observers_for_token(token);
        }
        let mut installed = BTreeSet::new();
        for token in tokens {
            let mut existing = implementation.clone();
            if let MayBinding::Target(target) = &mut existing {
                target.token = Some(token.clone());
            }
            let mut implementations = BTreeSet::from([existing.clone()]);
            if is_repeated_fresh_binding(&existing) {
                implementations.insert(MayBinding::Unknown);
            }
            Arc::make_mut(&mut self.objects).insert(token, implementations);
            installed.insert(existing);
        }
        if installed.is_empty()
            || prior.iter().any(
                |binding| !matches!(binding, MayBinding::Target(target) if target.token.is_some()),
            )
        {
            installed.insert(implementation);
        }
        if installed.iter().any(is_repeated_fresh_binding) {
            installed.insert(MayBinding::Unknown);
        }
        self.replace(key, installed);
    }

    pub(in super::super) fn unmodified_bindings(
        key: &(impl CommandKeyQuery + ?Sized),
        initial_registry_bindings: &BTreeSet<String>,
    ) -> BTreeSet<MayBinding> {
        let key = key.command_key();
        let Some(key) = key.authored_spelling() else {
            return BTreeSet::from([MayBinding::Missing]);
        };
        if initial_registry_bindings.contains(key) {
            BTreeSet::from([MayBinding::Target(ResolvedCommandTarget {
                command: key.to_owned(),
                prepended: Vec::new(),
                registry_backed: true,
                kind: BindingKind::Builtin,
                implementation_generation: 0,
                terminal: true,
                target_lookup: tcl_registry::AliasTargetLookup::Global,
                token: Some(CommandIdentity {
                    runtime: None,
                    origin: key.to_owned(),
                    declaration: 0,
                    allocation: None,
                }),
                implementation_allocation: None,
            })])
        } else {
            // A name first introduced by this module did not exist on the
            // pre-transition path. Treating it as an executable self-target
            // invents a user command and makes an exact alias spuriously
            // opaque when the before/after states are joined.
            BTreeSet::from([MayBinding::Missing])
        }
    }
}

/// After a constructor's `Define { kind: Instance }` installed its literal
/// name as an object command, mark that exact binding as an instance of the
/// constructor `factory`, so a call through it dispatches the factory's
/// declared instance methods.
pub(in super::super) fn retag_instance_definition(
    bindings: &mut ModuleCommandBindings,
    transition: &CommandBindingTransition,
    namespace: &crate::ir_helpers::ExecutionNamespace,
    declaration: u32,
    factory: &str,
) {
    let CommandBindingTransition::Define {
        name,
        kind: CommandBindingDefinitionKind::Instance,
    } = transition
    else {
        return;
    };
    let Some(name) = name.literal() else {
        return;
    };
    let Some(factory) = bindings
        .baseline
        .registry_owner
        .as_ref()
        .and_then(|owner| owner.registry().get(factory))
        .map(|spec| spec.name)
    else {
        return;
    };
    let Some(context) = namespace.for_head_context(name) else {
        return;
    };
    let Some(key) = bindings.publication_key_at(
        context.as_ref(),
        name,
        namespace_slots::PublicationPurpose::Command,
    ) else {
        return;
    };
    let alternatives = bindings.binding_alternatives(&key);
    let [MayBinding::Target(target)] = alternatives.iter().collect::<Vec<_>>()[..] else {
        return;
    };
    if target.kind != BindingKind::Class
        || target.registry_backed
        || target.implementation_generation != declaration
    {
        return;
    }
    let mut instance = target.clone();
    instance.kind = BindingKind::Instance { factory };
    bindings.replace(key, BTreeSet::from([MayBinding::Target(instance)]));
}

// Exhaustive interpreter of registry command-binding transitions.
#[allow(clippy::too_many_lines)]
pub(in super::super) fn apply_may_binding_transition(
    bindings: &mut ModuleCommandBindings,
    transition: &CommandBindingTransition,
    namespace: &crate::ir_helpers::ExecutionNamespace,
    precise_procedure: Option<&str>,
    declaration: u32,
    publication: Option<&command_publication::PreservingCommandPublication>,
    original_operands: Option<&original_command_table::OriginalCommandOperands>,
) {
    let preserves =
        publication.is_some_and(|receipt| receipt.preserves(transition, bindings, declaration));
    if !preserves
        && matches!(
            transition,
            CommandBindingTransition::Move { .. }
                | CommandBindingTransition::Delete { .. }
                | CommandBindingTransition::Alias { .. }
                | CommandBindingTransition::Define { .. }
        )
    {
        Arc::make_mut(&mut bindings.object_instances).invalidate_class_delegates(false);
    }
    if !preserves
        && matches!(
            transition,
            CommandBindingTransition::Define { .. }
                | CommandBindingTransition::Move { .. }
                | CommandBindingTransition::Delete { .. }
        )
    {
        Arc::make_mut(&mut bindings.object_instances).invalidate_dispatch(false);
    }
    if bindings.apply_captured_command_mutation(transition, declaration) {
        return;
    }

    if let Some(original) = original_operands {
        let current = match namespace {
            crate::ir_helpers::ExecutionNamespace::SourceContext(current) => Some(current.clone()),
            crate::ir_helpers::ExecutionNamespace::Exact(namespace) => {
                Some(SourceNamespaceKey::authored(namespace))
            }
            crate::ir_helpers::ExecutionNamespace::RuntimeSelected => None,
        };
        if current
            .as_ref()
            .and_then(|current| original.apply(bindings, transition, current, declaration))
            .is_none()
        {
            bindings.mark_unnameable_rebinding_subject();
        }
        return;
    }

    match transition {
        CommandBindingTransition::Alias {
            source_interpreter,
            alias,
            target_interpreter,
            target,
            arguments,
            target_lookup,
        } => {
            let Some(source_interpreter) = source_interpreter.literal() else {
                bindings.mark_unnameable_rebinding_subject();
                return;
            };
            if !matches!(source_interpreter, "" | "{}") {
                return;
            }
            let Some(alias) = alias.literal() else {
                bindings.mark_unnameable_rebinding_subject();
                return;
            };
            let current = bindings
                .native_root_namespace_key()
                .unwrap_or_else(|| SourceNamespaceKey::authored("::"));
            bindings.record_proc_rebound_candidates(
                alias,
                &crate::ir_helpers::ExecutionNamespace::SourceContext(current.clone()),
            );
            let Some(key) = bindings.publication_key_at(
                &current,
                alias,
                namespace_slots::PublicationPurpose::Alias,
            ) else {
                bindings.mark_unnameable_rebinding_subject();
                return;
            };
            bindings.rebound_names.insert(key.clone());
            let binding = if is_current_interpreter(target_interpreter) {
                target.literal().map(|target| {
                    let prepended = arguments
                        .iter()
                        .map(TransitionSubject::literal)
                        .map(|argument| {
                            argument.map_or(
                                crate::registry_invocation::EffectiveInvocationWord::Dynamic,
                                |value| {
                                    crate::registry_invocation::EffectiveInvocationWord::Literal(
                                        value.to_owned(),
                                    )
                                },
                            )
                        })
                        .collect();
                    MayBinding::Target(ResolvedCommandTarget {
                        command: target.to_owned(),
                        prepended,
                        registry_backed: true,
                        kind: BindingKind::Alias,
                        implementation_generation: declaration,
                        terminal: false,
                        target_lookup: *target_lookup,
                        token: None,
                        implementation_allocation: None,
                    })
                })
            } else {
                None
            };
            bindings.install(key, binding.unwrap_or(MayBinding::Unknown));
        }
        CommandBindingTransition::Move { from, to } => {
            let (Some(from), Some(to)) = (from.literal(), to.literal()) else {
                bindings.mark_unnameable_rebinding_subject();
                return;
            };
            bindings.record_proc_rebound_candidates(from, namespace);
            if !to.is_empty() {
                bindings.record_proc_rebound_candidates(to, namespace);
            }
            let Some(from_namespace) = namespace.for_head_context(from) else {
                bindings.mark_unnameable_rebinding_subject();
                return;
            };
            let Some(to_context) = namespace.for_head_context(to) else {
                bindings.mark_unnameable_rebinding_subject();
                return;
            };
            let Some(to_key) = bindings.publication_key_at(
                to_context.as_ref(),
                to,
                namespace_slots::PublicationPurpose::Rename,
            ) else {
                bindings.mark_unnameable_rebinding_subject();
                return;
            };
            let from_keys = bindings.source_keys(from, from_namespace.as_ref());
            if from_keys.len() != 1 {
                bindings.mark_unnameable_rebinding_subject();
            }
            let mut moved = BTreeSet::new();
            for from_key in from_keys {
                bindings.rebound_names.insert(from_key.clone());
                moved.extend(
                    bindings
                        .bindings
                        .get(&from_key)
                        .cloned()
                        .unwrap_or_else(|| {
                            ModuleCommandBindings::unmodified_bindings(
                                &from_key,
                                bindings.baseline.semantics.binding_names(),
                            )
                        }),
                );
                bindings.replace(from_key, BTreeSet::from([MayBinding::Missing]));
            }
            bindings.rebound_names.insert(to_key.clone());
            let mut moved = moved.into_iter().collect::<Vec<_>>();
            for binding in &mut moved {
                if let MayBinding::Target(target) = binding
                    && !target.registry_backed
                    && target.terminal
                {
                    if let Some(spelling) = bindings.callable_spelling_for_key(&to_key) {
                        target.command = spelling;
                    }
                    if let Some(token) = &target.token {
                        Arc::make_mut(&mut bindings.objects)
                            .insert(token.clone(), BTreeSet::from([binding.clone()]));
                    }
                }
            }
            bindings.replace(to_key, moved.into_iter().collect());
        }
        CommandBindingTransition::Delete { interpreter, name } => {
            let affects_current = match interpreter.as_ref().and_then(TransitionSubject::literal) {
                None if interpreter.is_some() => {
                    bindings.mark_unnameable_rebinding_subject();
                    return;
                }
                None | Some("" | "{}") => true,
                Some(_) => false,
            };
            if !affects_current {
                return;
            }
            let Some(name) = name.literal() else {
                bindings.mark_unnameable_rebinding_subject();
                return;
            };
            bindings.record_proc_rebound_candidates(name, namespace);
            // `interp alias {} NAME {}` names NAME in the source
            // interpreter's global namespace; bare `rename NAME {}` uses the
            // current command namespace.
            let deletion_namespace = if interpreter.is_some() {
                Some(std::borrow::Cow::Owned(
                    bindings
                        .native_root_namespace_key()
                        .unwrap_or_else(|| SourceNamespaceKey::authored("::")),
                ))
            } else {
                namespace.for_head_context(name)
            };
            let Some(deletion_namespace) = deletion_namespace else {
                bindings.mark_unnameable_rebinding_subject();
                return;
            };
            let keys = bindings.source_keys(name, deletion_namespace.as_ref());
            if keys.len() != 1 {
                bindings.mark_unnameable_rebinding_subject();
            }
            for key in keys {
                bindings.rebound_names.insert(key.clone());
                bindings.remove(key);
            }
        }
        CommandBindingTransition::Define { name, kind } => {
            if let Some(name) = name.literal() {
                let Some(context) = namespace.for_head_context(name) else {
                    bindings.mark_opaque_binding_mutation();
                    return;
                };
                let purpose = if *kind == CommandBindingDefinitionKind::Procedure {
                    namespace_slots::PublicationPurpose::Procedure
                } else {
                    namespace_slots::PublicationPurpose::Command
                };
                let Some(key) = bindings.publication_key_at(context.as_ref(), name, purpose) else {
                    bindings.mark_opaque_binding_mutation();
                    return;
                };
                let Some(label) =
                    crate::command_binding::ModuleCommandBindings::command_key_label(&key)
                else {
                    bindings.mark_opaque_binding_mutation();
                    return;
                };
                let binding = if *kind == CommandBindingDefinitionKind::Procedure {
                    if precise_procedure == Some(label.as_str())
                        && bindings.procedure_bodies.contains(&label)
                    {
                        MayBinding::Target(ResolvedCommandTarget {
                            command: label.clone(),
                            prepended: Vec::new(),
                            registry_backed: false,
                            kind: BindingKind::Proc,
                            implementation_generation: declaration,
                            terminal: true,
                            target_lookup: tcl_registry::AliasTargetLookup::Global,
                            token: Some(CommandToken {
                                runtime: None,
                                origin: label.clone(),
                                declaration,
                                allocation: None,
                            }),
                            implementation_allocation: None,
                        })
                    } else {
                        bindings.mark_opaque_binding_mutation();
                        MayBinding::Unknown
                    }
                } else {
                    MayBinding::Target(ResolvedCommandTarget {
                        command: label.clone(),
                        prepended: Vec::new(),
                        registry_backed: false,
                        kind: if matches!(
                            kind,
                            CommandBindingDefinitionKind::Object
                                | CommandBindingDefinitionKind::Instance
                        ) {
                            BindingKind::Class
                        } else {
                            BindingKind::Command
                        },
                        implementation_generation: declaration,
                        terminal: true,
                        target_lookup: tcl_registry::AliasTargetLookup::Global,
                        token: Some(CommandToken {
                            runtime: None,
                            origin: label.clone(),
                            declaration,
                            allocation: None,
                        }),
                        implementation_allocation: None,
                    })
                };
                bindings.install(key, binding);
            } else {
                bindings.mark_opaque_binding_mutation();
            }
        }
        CommandBindingTransition::Unknown { .. } => bindings.mark_unnameable_rebinding_subject(),
    }
}
