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

//! Coordinated physical namespace activations, independent of native lookup authority.

use super::{NameBytes, NamespaceResolutionStacks, NsId};
use std::rc::{Rc, Weak};

/// One stack activation. Its birth moves with hiding, frame selection and flow transport.
pub(super) struct NamespaceResolutionActivation {
    pub(super) name: NameBytes,
    pub(super) namespace: NsId,
    birth: Rc<()>,
}

#[derive(Default)]
pub(super) struct NamespaceResolutionTail {
    names: Vec<NameBytes>,
    namespaces: Vec<NsId>,
    births: Vec<Rc<()>>,
}
impl NamespaceResolutionTail {
    pub(super) fn current_namespace(&self) -> Option<NsId> {
        self.namespaces.last().copied()
    }
}

/// Weak metadata is a cleanup identity, not a namespace or callable role.
#[derive(Clone)]
pub(super) struct NamespaceActivationObservation {
    pub(super) position: usize,
    pub(super) namespace: NsId,
    birth: Weak<()>,
    parent: Option<Weak<()>>,
}
impl NamespaceActivationObservation {
    pub(super) fn original_present(&self, stack: &NamespaceResolutionStacks) -> bool {
        stack.ns_id_stack.get(self.position) == Some(&self.namespace)
            && stack
                .activation_births
                .get(self.position)
                .is_some_and(|birth| Weak::ptr_eq(&self.birth, &Rc::downgrade(birth)))
    }
    pub(super) fn caller_restored(&self, stack: &NamespaceResolutionStacks) -> bool {
        stack.ns_stack.len() == self.position
            && stack.ns_id_stack.len() == self.position
            && stack.activation_births.len() == self.position
            && self
                .parent
                .as_ref()
                .zip(stack.activation_births.last())
                .is_some_and(|(old, current)| Weak::ptr_eq(old, &Rc::downgrade(current)))
    }
}
impl NamespaceResolutionStacks {
    pub(super) fn push_activation(&mut self, name: NameBytes, namespace: NsId) {
        self.restore_activation(NamespaceResolutionActivation {
            name,
            namespace,
            birth: Rc::new(()),
        });
    }
    pub(super) fn restore_activation(&mut self, entry: NamespaceResolutionActivation) {
        self.ns_stack.push(entry.name);
        self.ns_id_stack.push(entry.namespace);
        self.activation_births.push(entry.birth);
    }
    pub(super) fn pop_activation(&mut self) -> Option<NamespaceResolutionActivation> {
        (self.ns_stack.len() > 1).then(|| NamespaceResolutionActivation {
            name: self.ns_stack.pop().expect("non-global namespace name"),
            namespace: self.ns_id_stack.pop().expect("non-global namespace token"),
            birth: self
                .activation_births
                .pop()
                .expect("non-global namespace birth"),
        })
    }
    pub(super) fn observe_activation(&self) -> Option<NamespaceActivationObservation> {
        let position = self.ns_stack.len().checked_sub(1)?;
        if position == 0
            || self.ns_id_stack.len() != position + 1
            || self.activation_births.len() != position + 1
        {
            return None;
        }
        Some(NamespaceActivationObservation {
            position,
            namespace: self.ns_id_stack[position],
            birth: Rc::downgrade(&self.activation_births[position]),
            parent: self.activation_births.get(position - 1).map(Rc::downgrade),
        })
    }
    pub(super) fn take_original_activation(
        &mut self,
        original: &NamespaceActivationObservation,
    ) -> Option<NamespaceResolutionActivation> {
        original
            .original_present(self)
            .then(|| NamespaceResolutionActivation {
                name: self.ns_stack.remove(original.position),
                namespace: self.ns_id_stack.remove(original.position),
                birth: self.activation_births.remove(original.position),
            })
    }
    pub(super) fn take_tail(&mut self, cut: usize) -> NamespaceResolutionTail {
        NamespaceResolutionTail {
            names: self.ns_stack.split_off(cut),
            namespaces: self.ns_id_stack.split_off(cut),
            births: self.activation_births.split_off(cut),
        }
    }
    pub(super) fn append_tail(&mut self, mut tail: NamespaceResolutionTail) {
        self.ns_stack.append(&mut tail.names);
        self.ns_id_stack.append(&mut tail.namespaces);
        self.activation_births.append(&mut tail.births);
    }
    pub(super) fn truncate_tail(&mut self, len: usize) {
        self.ns_stack.truncate(len);
        self.ns_id_stack.truncate(len);
        self.activation_births.truncate(len);
    }
}
