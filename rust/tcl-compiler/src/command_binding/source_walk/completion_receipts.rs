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

//! Completion receipts of a native handler and whether it leaves the command world closed.

use super::super::{
    BindingKind, MayBinding, ModuleCommandBindings, SourceCommandTarget, SourceExecutionContext,
    SourceNativeInvocation, SourceOutcomes, original_alias_creation, original_command_mutation,
    original_namespace_binding, original_namespace_deletion,
};
use super::outcomes::boxed_source_completion_receipt;
use std::collections::BTreeSet;
use tcl_registry::InvocationFacts;

pub(super) struct CapturedNativeCompletionReceipts {
    mutation: Option<Box<original_command_mutation::OriginalCommandMove>>,
    alias_creation: Option<Box<original_alias_creation::OriginalAliasCreation>>,
    namespace_binding: Option<Box<original_namespace_binding::OriginalNamespaceBinding>>,
    /// The same transfer under the release's own preload command, captured
    /// only when the strict receipt is unavailable. Diagnostic view only.
    diagnostic_namespace_binding: Option<Box<original_namespace_binding::OriginalNamespaceBinding>>,
    /// A class deletion, captured only when the strict receipt is
    /// unavailable. Diagnostic view only.
    diagnostic_deletion: Option<Box<original_command_mutation::OriginalCommandMove>>,
    namespace_deletion: Option<Box<original_namespace_deletion::OriginalNamespaceDeletion>>,
}

impl CapturedNativeCompletionReceipts {
    pub(in super::super) fn capture(
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        state: &ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) -> Self {
        let mutation = boxed_source_completion_receipt(|| {
            original_command_mutation::OriginalCommandMove::capture(native, facts, state, *context)
        });
        let alias_creation = boxed_source_completion_receipt(|| {
            original_alias_creation::OriginalAliasCreation::capture(native, facts, state, *context)
        });
        let namespace_binding = boxed_source_completion_receipt(|| {
            original_namespace_binding::OriginalNamespaceBinding::capture(
                native, facts, state, *context,
            )
        });
        let namespace_deletion = boxed_source_completion_receipt(|| {
            original_namespace_deletion::OriginalNamespaceDeletion::capture(
                native, facts, state, *context,
            )
        });
        let diagnostic_namespace_binding = if namespace_binding.is_none() {
            boxed_source_completion_receipt(|| {
                original_namespace_binding::OriginalNamespaceBinding::capture_with(
                    native,
                    facts,
                    state,
                    *context,
                    original_namespace_binding::ImportPreload::AbsentOrStock,
                )
            })
        } else {
            None
        };
        let diagnostic_deletion = if mutation.is_none() {
            boxed_source_completion_receipt(|| {
                original_command_mutation::OriginalCommandMove::capture_deletion_for_diagnostics(
                    native, facts, state, *context,
                )
            })
        } else {
            None
        };
        Self {
            mutation,
            alias_creation,
            namespace_binding,
            diagnostic_namespace_binding,
            diagnostic_deletion,
            namespace_deletion,
        }
    }

    /// Whether the diagnostic-only receipt matches the walked normal state.
    /// An abrupt alternative (the preload command may raise) does not matter
    /// here: advice after the step speaks only for the path that returned.
    pub(super) fn completed_for_diagnostics(
        &self,
        outcomes: &SourceOutcomes,
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        context: SourceExecutionContext<'_>,
    ) -> bool {
        let Some(normal) = outcomes.normal.as_ref() else {
            return false;
        };
        self.diagnostic_namespace_binding
            .as_ref()
            .is_some_and(|receipt| receipt.completed(native, normal, context))
            || self
                .diagnostic_deletion
                .as_ref()
                .is_some_and(|receipt| receipt.completed(native, facts, normal, context))
    }

    /// A declared deletion the walk recorded as exactly one alias slot going
    /// missing, with every other slot as it found it. Deleting an alias
    /// removes that one command and leaves its target alone.
    pub(super) fn deleted_one_alias_for_diagnostics(
        facts: &InvocationFacts,
        incoming: &ModuleCommandBindings,
        normal: &ModuleCommandBindings,
    ) -> bool {
        let Some(transitions) = facts.state_transitions.declared() else {
            return false;
        };
        let mut declared = transitions.command_bindings();
        if !matches!(
            declared.next(),
            Some(tcl_registry::CommandBindingTransition::Delete { .. })
        ) || declared.next().is_some()
            || normal.has_opaque_domain()
        {
            return false;
        }
        let missing = BTreeSet::from([MayBinding::Missing]);
        let mut deleted = false;
        for (key, before) in incoming.bindings.iter() {
            let Some(after) = normal.original_bindings_for_key(key) else {
                return false;
            };
            if &after == before {
                continue;
            }
            let mut targets = before.iter();
            let (Some(MayBinding::Target(target)), None) = (targets.next(), targets.next()) else {
                return false;
            };
            if target.kind != BindingKind::Alias || after != missing || deleted {
                return false;
            }
            deleted = true;
        }
        deleted
            && normal
                .bindings
                .iter()
                .all(|(key, _)| incoming.original_bindings_for_key(key).is_some())
    }

    /// A declared alias the walk recorded as exactly one new alias slot, with
    /// every other slot as it found it. Creating an alias runs nothing; its
    /// target runs at a later call, which is its own step.
    pub(super) fn created_one_alias_for_diagnostics(
        facts: &InvocationFacts,
        incoming: &ModuleCommandBindings,
        normal: &ModuleCommandBindings,
    ) -> bool {
        let Some(transitions) = facts.state_transitions.declared() else {
            return false;
        };
        let mut declared = transitions.command_bindings();
        if !matches!(
            declared.next(),
            Some(tcl_registry::CommandBindingTransition::Alias { .. })
        ) || declared.next().is_some()
            || normal.has_opaque_domain()
        {
            return false;
        }
        let missing = BTreeSet::from([MayBinding::Missing]);
        let mut created = false;
        for (key, after) in normal.bindings.iter() {
            let before = incoming.original_bindings_for_key(key);
            if before.as_ref() == Some(after) {
                continue;
            }
            let mut targets = after.iter();
            let (Some(MayBinding::Target(target)), None) = (targets.next(), targets.next()) else {
                return false;
            };
            if target.kind != BindingKind::Alias || before != Some(missing.clone()) || created {
                return false;
            }
            created = true;
        }
        created
            && incoming
                .bindings
                .iter()
                .all(|(key, _)| normal.original_bindings_for_key(key).is_some())
    }

    /// A definer the walk recorded as exactly one class command, with every
    /// other slot as it found it. The definition has no completion receipt
    /// (its script may still reach the caller frame), but the diagnostic view
    /// knows which command it published.
    pub(super) fn defined_one_class_for_diagnostics(
        facts: &InvocationFacts,
        incoming: &ModuleCommandBindings,
        normal: &ModuleCommandBindings,
    ) -> bool {
        let Some(transitions) = facts.state_transitions.declared() else {
            return false;
        };
        let mut defined = transitions.command_bindings();
        if !matches!(
            defined.next(),
            Some(tcl_registry::CommandBindingTransition::Define {
                kind: tcl_registry::CommandBindingDefinitionKind::Object,
                ..
            })
        ) || defined.next().is_some()
            || normal.has_opaque_domain()
        {
            return false;
        }
        let mut published = false;
        for (key, alternatives) in normal.bindings.iter() {
            if incoming.original_bindings_for_key(key).as_ref() == Some(alternatives) {
                continue;
            }
            let mut targets = alternatives.iter();
            let (Some(MayBinding::Target(target)), None) = (targets.next(), targets.next()) else {
                return false;
            };
            if target.kind != BindingKind::Class || published {
                return false;
            }
            published = true;
        }
        published
            && incoming
                .bindings
                .iter()
                .all(|(key, _)| normal.original_bindings_for_key(key).is_some())
    }

    /// Fresh same-interpreter aliases retain a prefix without invoking it.
    pub(super) const fn captured_alias_creation(&self) -> bool {
        self.alias_creation.is_some()
    }

    pub(in super::super) fn captured(&self) -> bool {
        self.mutation.is_some()
            || self.alias_creation.is_some()
            || self.namespace_binding.is_some()
            || self.namespace_deletion.is_some()
    }

    pub(in super::super) fn completed(
        &self,
        outcomes: &SourceOutcomes,
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        context: SourceExecutionContext<'_>,
    ) -> bool {
        self.mutation.as_ref().is_some_and(|receipt| {
            outcomes.abrupt.is_empty()
                && outcomes
                    .normal
                    .as_ref()
                    .is_some_and(|normal| receipt.completed(native, facts, normal, context))
        }) || self.alias_creation.as_ref().is_some_and(|receipt| {
            outcomes.abrupt.is_empty()
                && outcomes
                    .normal
                    .as_ref()
                    .is_some_and(|normal| receipt.completed(native, facts, normal, context))
        }) || self.namespace_binding.as_ref().is_some_and(|receipt| {
            outcomes.abrupt.is_empty()
                && outcomes
                    .normal
                    .as_ref()
                    .is_some_and(|normal| receipt.completed(native, normal, context))
        }) || self.namespace_deletion.as_ref().is_some_and(|receipt| {
            outcomes.abrupt.is_empty()
                && outcomes
                    .normal
                    .as_ref()
                    .is_some_and(|normal| receipt.completed(native, facts, normal, context))
        })
    }
}

/// An actual registration or selected executable prefix prevents the fresh
/// builtin shortcut. A possible callback is not a registration receipt:
/// `UNKNOWN` includes every kind, and ordinary variable accesses carry `TRACE`.
/// Their independent unresolved effects remain with the generic handler guards.
pub(in super::super) fn requires_reentry_guard(facts: &InvocationFacts) -> bool {
    use tcl_registry::{CallbackKinds, NamespaceTransition, StateTransition, TraceTransition};
    let kinds = facts.effects.callback().kinds;
    let precise_callback = !kinds.is_unknown()
        && (kinds.contains(CallbackKinds::COMMAND_PREFIX) || kinds.contains(CallbackKinds::HOST));
    let source_retention = facts.source_callback_retention()
        != tcl_registry::resolved_invocation::SourceCallbackRetention::Absent;
    let registration = facts
        .state_transitions
        .declared()
        .is_some_and(|transitions| {
            transitions.facts().iter().any(|fact| {
                matches!(
                    fact.transition,
                    StateTransition::Trace(TraceTransition::Add { .. })
                        | StateTransition::Namespace(NamespaceTransition::SetUnknown { .. })
                )
            })
        });
    precise_callback
        || source_retention
        || registration
        || facts
            .traits
            .intersects(tcl_registry::extension_default::TRACES)
}

/// Whether a handler without its own closed receipt still leaves the command
/// world exactly as the walk models it on normal return.
///
/// Only a command resolved to an unperturbed registry builtin qualifies, and
/// only when its selected descriptor is complete: every argument role is
/// known, the subcommand (if any) was selected, and it declares no
/// command-table mutation. A descriptor with declared transitions must
/// declare none at all (`list`, `string length`) and no re-entrant callback
/// (`lsort -command`), which could rebind commands before the invocation
/// returns. A builtin without declared transitions (`puts`) is the walk's
/// generic invocation; it qualifies only when its own traits claim no code
/// evaluation, dynamic barrier or trace, which every evaluator and every
/// extension-default command (`extension_default::EVALUATION`) carries.
/// Unknown commands, procedures, objects, aliases and incompletely described
/// commands keep the completed world withheld.
pub(super) fn command_world_closed_on_normal_return(
    target: &SourceCommandTarget,
    facts: &InvocationFacts,
) -> bool {
    use tcl_registry::OwnedSubcommandResolution;
    if creates_literal_ensemble(facts) {
        return target.registry_backed
            && target.kind == BindingKind::Builtin
            && target.prepended.is_empty()
            && !facts.effects.requires_world_barrier();
    }
    let complete_builtin = target.registry_backed
        && target.kind == BindingKind::Builtin
        && target.prepended.is_empty()
        && facts.arg_roles_complete
        && matches!(
            facts.subcommand,
            OwnedSubcommandResolution::NotApplicable
                | OwnedSubcommandResolution::Exact { .. }
                | OwnedSubcommandResolution::UniquePrefix { .. }
        )
        && !facts.effects.legacy().command_table_mutation;
    if !complete_builtin {
        return false;
    }
    match facts.state_transitions.declared() {
        // Linking a local to another variable cell (`global`, `upvar`,
        // `variable`) changes no command or namespace lookup.
        Some(transitions) => {
            transitions.facts().iter().all(|fact| {
                matches!(
                    fact.transition,
                    tcl_registry::StateTransition::VariableCellAlias(_)
                )
            }) && !facts.effects.requires_world_barrier()
        }
        None => undeclared_transitions_leave_command_world_closed(facts),
    }
}

/// A literal `namespace ensemble create` whose one installed command the
/// binding walk publishes exactly; the original world withdraws itself when
/// it cannot.
pub(super) fn creates_literal_ensemble(facts: &InvocationFacts) -> bool {
    facts
        .state_transitions
        .declared()
        .is_some_and(|transitions| {
            !transitions.facts().is_empty()
                && transitions.facts().iter().all(|fact| {
                    matches!(
                        fact.transition,
                        tcl_registry::StateTransition::Namespace(
                            tcl_registry::NamespaceTransition::Ensemble {
                                created: Some(_),
                                ..
                            }
                        )
                    )
                })
        })
}

/// A builtin without declared transitions is the walk's generic invocation:
/// it leaves the command world closed only when its traits claim no code
/// evaluation, dynamic barrier or trace.
#[must_use]
pub fn undeclared_transitions_leave_command_world_closed(facts: &InvocationFacts) -> bool {
    !facts.traits.intersects(
        tcl_registry::extension_default::EVALUATION.union(tcl_registry::extension_default::TRACES),
    )
}
