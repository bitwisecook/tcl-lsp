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

//! How rename, alias, binding and namespace transitions move a child's visible slots.

use super::super::Analyser;
use super::{SourceCommandOrigin, SourceInterpreterVisibilitySnapshot, SourceVisibleSlot};
use crate::signature_scan::scope::{SignatureNamespaceScope, SignatureSourceNameInput};
use tcl_core_types::ByteCommandSlot;
use tcl_lexer::{NativeWord, Token};
use tcl_registry::{StateTransition, TransitionSubject};
use tcl_syntax::naming::NativeNameContext;

impl SourceInterpreterVisibilitySnapshot {
    pub(super) fn mutation_operand(
        &self,
        native: &[NativeWord],
        subject: &TransitionSubject,
    ) -> Option<SignatureSourceNameInput> {
        self.original_input(native.get(subject.argument_index()?.checked_add(1)?)?)
    }
    fn withdraw_visible_slot(&mut self, slot: &ByteCommandSlot) {
        self.slots.remove(slot);
        if let Some(origin) = self.unavailable.get(slot) {
            self.slots
                .insert(slot.clone(), SourceVisibleSlot::Unavailable(origin.clone()));
        }
    }
    fn apply_move(
        &mut self,
        native: &[NativeWord],
        namespace: &SignatureNamespaceScope,
        from: &TransitionSubject,
        to: &TransitionSubject,
    ) -> Option<()> {
        let from = self.mutation_operand(native, from)?;
        let to = self.mutation_operand(native, to)?;
        let selected = self
            .policy
            .recipe()
            .rename_source_input(namespace.context()?, from.bytes())
            .ok()?;
        let slot = self.lookup_slot(selected.selected(), namespace)?;
        let Some(SourceVisibleSlot::Visible(origin)) = self.slots.get(&slot).cloned() else {
            return Some(());
        };
        let destination = if to.bytes().is_empty() {
            None
        } else {
            Some(
                self.policy
                    .recipe()
                    .rename_destination_slot(namespace.context()?, to.bytes())
                    .ok()?,
            )
        };
        if destination.as_ref().is_some_and(|destination| {
            matches!(
                self.slots.get(destination),
                Some(SourceVisibleSlot::Visible(_))
            )
        }) {
            return Some(());
        }
        self.withdraw_visible_slot(&slot);
        if let Some(destination) = destination {
            self.slots.insert(
                destination,
                SourceVisibleSlot::Visible(SourceCommandOrigin::Renamed {
                    original: Box::new(origin),
                    operands: native.to_vec(),
                }),
            );
        }
        Some(())
    }
    fn apply_binding_transition(
        &mut self,
        native: &[NativeWord],
        namespace: &SignatureNamespaceScope,
        transition: &tcl_registry::CommandBindingTransition,
    ) -> Option<()> {
        use tcl_registry::CommandBindingTransition as Binding;
        match transition {
            Binding::Define { name, kind } => {
                let input = self.mutation_operand(native, name)?;
                let slot = self
                    .policy
                    .recipe()
                    .command_publication_slot(namespace.context()?, input.bytes())
                    .ok()?;
                self.slots.insert(
                    slot,
                    SourceVisibleSlot::Visible(SourceCommandOrigin::Declaration {
                        name: input,
                        kind: *kind,
                        operands: native.to_vec(),
                    }),
                );
            }
            Binding::Move { from, to } => self.apply_move(native, namespace, from, to)?,
            Binding::Delete { interpreter, name } => {
                if let Some(interpreter) = interpreter
                    && !self
                        .mutation_operand(native, interpreter)?
                        .bytes()
                        .is_empty()
                {
                    return Some(());
                }
                let input = self.mutation_operand(native, name)?;
                let slot = self.lookup_slot(input.bytes(), namespace)?;
                self.withdraw_visible_slot(&slot);
            }
            Binding::Alias {
                source_interpreter,
                alias,
                target,
                ..
            } => {
                if !self
                    .mutation_operand(native, source_interpreter)?
                    .bytes()
                    .is_empty()
                {
                    return Some(());
                }
                let input = self.mutation_operand(native, alias)?;
                self.mutation_operand(native, target)?;
                let slot = self
                    .policy
                    .recipe()
                    .alias_publication_slot(namespace.context()?, input.bytes())
                    .ok()?;
                self.slots.insert(
                    slot,
                    SourceVisibleSlot::Visible(SourceCommandOrigin::Alias {
                        name: input,
                        operands: native.to_vec(),
                    }),
                );
            }
            Binding::Unknown { .. } => return None,
        }
        Some(())
    }
    fn apply_namespace_transition(
        &mut self,
        native: &[NativeWord],
        namespace: &SignatureNamespaceScope,
        transition: &tcl_registry::NamespaceTransition,
    ) -> Option<()> {
        use tcl_registry::{NamespaceTransition as Namespace, NamespaceTransitionTarget as Target};
        match transition {
            Namespace::Ensure { .. } | Namespace::Export { .. } => Some(()),
            Namespace::Delete { namespace: target } => {
                let path = match target {
                    Target::Current => match namespace {
                        SignatureNamespaceScope::C(path) => path.clone(),
                        _ => return None,
                    },
                    Target::Named(subject) => {
                        let input = self.mutation_operand(native, subject)?;
                        self.policy
                            .recipe()
                            .namespace_address_path(namespace.context()?, input.bytes())
                            .ok()?
                    }
                };
                if path.is_root() {
                    return None;
                }
                self.slots.retain(|slot, _| {
                    !slot.namespace.as_segments().starts_with(path.as_segments())
                });
                Some(())
            }
            Namespace::Import { .. }
            | Namespace::Forget { .. }
            | Namespace::SetPath { .. }
            | Namespace::SetUnknown { .. }
            | Namespace::Ensemble { .. } => None,
        }
    }
    pub(super) fn apply_command_transitions(
        &mut self,
        native: &[NativeWord],
        namespace: &SignatureNamespaceScope,
        transitions: &tcl_registry::StateTransitions,
    ) -> Option<()> {
        for fact in transitions.facts() {
            match &fact.transition {
                StateTransition::CommandBinding(transition) => {
                    self.apply_binding_transition(native, namespace, transition)?;
                }
                StateTransition::Namespace(transition) => {
                    self.apply_namespace_transition(native, namespace, transition)?;
                }
                _ => {}
            }
        }
        if transitions.widens(tcl_registry::StateTransitionDomain::CommandBindings)
            || transitions.widens(tcl_registry::StateTransitionDomain::CommandResolution)
        {
            return None;
        }
        Some(())
    }
}

impl Analyser {
    pub(in super::super) fn retain_interp_visibility_alias(
        &mut self,
        head: Token,
        arguments: &[Token],
    ) {
        let Some(invocation) = self.original_interpreter_invocation(
            head,
            arguments,
            tcl_registry::hooks::AnalyserHookId::InterpAlias,
        ) else {
            for state in self.interpreters.values_mut() {
                state.visibility = None;
                state.tainted = true;
            }
            return;
        };
        for transition in invocation.transitions.command_bindings() {
            if let tcl_registry::CommandBindingTransition::Alias {
                source_interpreter,
                alias,
                target,
                ..
            } = transition
            {
                let owned = (|| {
                    let path = invocation.input(source_interpreter)?;
                    let key = self.original_interp_state_key(&invocation, &path)?;
                    let alias = invocation.input(alias)?;
                    invocation.input(target)?;
                    let policy = alias.policy();
                    let slot = policy
                        .recipe()
                        .alias_publication_slot(NativeNameContext::root(), alias.bytes())
                        .ok()?;
                    Some((key, slot, alias, invocation.native.clone()))
                })();
                if let Some((key, slot, name, operands)) = owned {
                    if let Some(state) = self.interpreters.get_mut(&key)
                        && let Some(snapshot) = state.visibility.as_mut()
                    {
                        snapshot.slots.insert(
                            slot,
                            SourceVisibleSlot::Visible(SourceCommandOrigin::Alias {
                                name,
                                operands,
                            }),
                        );
                    }
                } else {
                    for state in self.interpreters.values_mut() {
                        state.visibility = None;
                        state.tainted = true;
                    }
                }
            } else {
                for state in self.interpreters.values_mut() {
                    state.visibility = None;
                    state.tainted = true;
                }
            }
        }
    }
}
