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

//! Jim's object-keyed records: command registration and inventory keys, and the namespace object each context retains.

use super::{
    JimNamespaceName, JimNamespaceObject, Namespace, Namespaces, NsId,
    OriginalJimCommandInventoryKey, GLOBAL, JIM_CONTEXT_PATH,
};
use crate::interp::Command;

impl Namespaces {
    pub(crate) fn has_original_jim_registration_key(
        &self,
        namespace: NsId,
        tail: &[u8],
        generation: u64,
    ) -> bool {
        self.arena
            .get(namespace)
            .and_then(|namespace| namespace.commands.entries.get(tail))
            .is_some_and(|slot| {
                slot.binding
                    .as_ref()
                    .is_some_and(|binding| binding.generation == generation)
                    && slot.original_jim_key.is_some()
            })
    }

    pub(crate) fn publish_original_jim_registration_key(
        &mut self,
        namespace: NsId,
        tail: &[u8],
        generation: u64,
        report: tcl_syntax::naming::NativeJimCommandTableKey,
        original: &std::rc::Rc<crate::obj::Owned>,
    ) -> bool {
        if self.variable_name_protocol != Some(tcl_syntax::naming::NativeNameProtocol::Jim084)
            || report.comparison_bytes() != tail
        {
            return false;
        }
        let Some(slot) = self
            .arena
            .get_mut(namespace)
            .and_then(|namespace| namespace.commands.entries.get_mut(tail))
        else {
            return false;
        };
        let Some(binding) = slot
            .binding
            .as_mut()
            .filter(|binding| binding.generation == generation)
        else {
            return false;
        };
        if slot.original_jim_key.is_some() {
            return true;
        }
        slot.original_jim_key = Some(std::rc::Rc::new(crate::obj::Owned::retain(
            original.as_ptr(),
        )));
        binding.jim_table_key = Some(report);
        true
    }

    /// Observe the actual found Jim hash key before retiring the lookup object's
    /// old primary. The allocation lease adds no native object reference.
    pub(crate) fn observe_original_jim_command_name(
        &mut self,
        generation: u64,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        use tcl_syntax::value::ValueError;
        if self.variable_name_protocol != Some(tcl_syntax::naming::NativeNameProtocol::Jim084) {
            return Err(ValueError::CommandProtocolUnavailable(
                "original Jim command name",
            ));
        }
        let (namespace, simple) = self.native_command_slot_at_node(generation).ok_or(
            ValueError::CommandProtocolUnavailable("original Jim command slot"),
        )?;
        let original = self.arena[namespace]
            .commands
            .entries
            .get(&simple)
            .and_then(|entry| entry.original_jim_key.as_ref())
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original Jim command hash key",
            ))?
            .as_ptr();
        crate::obj::check_native_liveness(original)?;
        let observation = crate::obj::JimCommandNameObservation::capture(original)?;
        self.native_command_nodes
            .get_mut(&generation)
            .expect("checked current command slot")
            .jim_command_name = Some(observation);
        Ok(())
    }

    /// The helper's info-commands List owns these exact current hash keys.
    #[cfg(test)]
    pub(crate) fn original_jim_command_enumeration_keys(
        &self,
        names: &[Vec<u8>],
    ) -> Result<Vec<crate::obj::NativeObjectLifetime>, tcl_syntax::value::ValueError> {
        use tcl_syntax::value::ValueError;
        if self.variable_name_protocol != Some(tcl_syntax::naming::NativeNameProtocol::Jim084) {
            return Err(ValueError::CommandProtocolUnavailable(
                "original Jim command enumeration issuer",
            ));
        }
        names
            .iter()
            .map(|name| {
                let slot = self.arena[GLOBAL].commands.entries.get(name).ok_or(
                    ValueError::CommandProtocolUnavailable("original Jim enumeration slot"),
                )?;
                let binding =
                    slot.binding
                        .as_ref()
                        .ok_or(ValueError::CommandProtocolUnavailable(
                            "original Jim enumeration binding",
                        ))?;
                if !self.original_publication_is_current(GLOBAL, name, binding.generation) {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "original Jim enumeration generation",
                    ));
                }
                let key = slot.original_jim_key.as_ref().ok_or(
                    ValueError::CommandProtocolUnavailable("original Jim enumeration hash key"),
                )?;
                crate::obj::check_native_liveness(key.as_ptr())?;
                Ok(crate::obj::NativeObjectLifetime::retain(key.as_ptr()))
            })
            .collect()
    }

    pub(crate) fn original_jim_command_inventory_keys(
        &self,
        kind: tcl_runtime_api::NativeJimCommandInventoryKind,
        visible: &impl Fn(&Command, u64) -> bool,
    ) -> Result<Vec<OriginalJimCommandInventoryKey>, tcl_syntax::value::ValueError> {
        use tcl_syntax::value::ValueError;
        if self.variable_name_protocol != Some(tcl_syntax::naming::NativeNameProtocol::Jim084) {
            return Err(ValueError::CommandProtocolUnavailable(
                "original Jim command inventory table",
            ));
        }
        self.arena[GLOBAL]
            .commands
            .entries
            .iter()
            .filter_map(|(slot, entry)| {
                entry.binding.as_ref().map(|binding| (slot, entry, binding))
            })
            .filter(|(_, _, binding)| {
                visible(&binding.command, binding.generation)
                    && match kind {
                        tcl_runtime_api::NativeJimCommandInventoryKind::Commands => true,
                        tcl_runtime_api::NativeJimCommandInventoryKind::Procs => {
                            matches!(&binding.command, Command::Proc(_))
                        }
                        // Jim's child alias registers without JIM_CMD_ISALIAS.
                        tcl_runtime_api::NativeJimCommandInventoryKind::Aliases => {
                            matches!(&binding.command, Command::Alias { .. })
                        }
                    }
            })
            .map(|(slot, entry, binding)| {
                if !self.original_publication_is_current(GLOBAL, slot, binding.generation) {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "original Jim inventory current binding",
                    ));
                }
                let key = entry.original_jim_key.as_ref().ok_or(
                    ValueError::CommandProtocolUnavailable("original Jim inventory actual key"),
                )?;
                let table_key = binding.jim_table_key.as_ref().ok_or(
                    ValueError::CommandProtocolUnavailable("original Jim inventory table geometry"),
                )?;
                if table_key.comparison_bytes() != slot.as_slice() {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "original Jim inventory key geometry",
                    ));
                }
                crate::obj::check_native_liveness(key.as_ptr())?;
                Ok(OriginalJimCommandInventoryKey {
                    generation: binding.generation,
                    report: table_key.report_bytes().to_vec(),
                    original: crate::obj::NativeObjectLifetime::retain(key.as_ptr()),
                })
            })
            .collect()
    }

    pub(crate) fn check_original_jim_command_inventory_key(
        &self,
        key: &OriginalJimCommandInventoryKey,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        use tcl_syntax::value::ValueError;
        let original = key.original()?;
        let (namespace, slot) = self.native_command_slot_at_node(key.generation).ok_or(
            ValueError::CommandProtocolUnavailable("retired original Jim inventory binding"),
        )?;
        let current = self.arena[namespace]
            .commands
            .entries
            .get(&slot)
            .filter(|_| namespace == GLOBAL)
            .and_then(|entry| entry.original_jim_key.as_ref());
        if !current.is_some_and(|current| current.as_ptr() == original) {
            return Err(ValueError::CommandProtocolUnavailable(
                "replaced original Jim inventory key",
            ));
        }
        Ok(())
    }

    /// Cold lookup observes one allocator/header slot. Selected readers check
    /// its current resident; exact key owners keep their own allocation birth.
    pub(crate) fn original_jim_command_name_observation(
        &self,
        generation: u64,
    ) -> Option<&crate::obj::JimCommandNameObservation> {
        self.native_command_nodes
            .get(&generation)?
            .jim_command_name
            .as_ref()
    }

    #[cfg(test)]
    pub(crate) fn original_jim_command_key(
        &self,
        ns: NsId,
        simple: &[u8],
    ) -> Option<*mut crate::obj::TclObj> {
        self.arena
            .get(ns)?
            .commands
            .entries
            .get(simple)?
            .original_jim_key
            .as_ref()
            .map(|key| key.as_ptr())
    }

    /// Retain the actual canonical Jim object for a new activation context.
    /// Jim helper contexts are counted objects, not C namespace-tree nodes.
    /// Equal names do not replace the supplied holder with another object.
    pub(crate) fn retain_jim_namespace(
        &mut self,
        holder: crate::obj::Owned,
        bytes: std::rc::Rc<[u8]>,
    ) -> NsId {
        self.retain_jim_namespace_with_name(holder, JimNamespaceName::Materialized(bytes))
    }

    /// Allocate an actual original-object context without claiming a name coordinate.
    pub(crate) fn retain_unmaterialized_jim_namespace(
        &mut self,
        holder: crate::obj::Owned,
    ) -> NsId {
        self.retain_jim_namespace_with_name(holder, JimNamespaceName::Unmaterialized)
    }

    fn retain_jim_namespace_with_name(
        &mut self,
        holder: crate::obj::Owned,
        name: JimNamespaceName,
    ) -> NsId {
        let token = self.arena.len();
        let mut namespace = Namespace::new(Vec::new(), None);
        namespace.jim_namespace =
            Some((JimNamespaceObject::Initial(std::rc::Rc::new(holder)), name));
        self.arena.push(namespace);
        self.activations.push(0);
        token
    }

    /// Adopt the original empty object as the genuine Jim top-frame namespace.
    pub(crate) fn adopt_jim_root_namespace(&mut self, holder: crate::obj::Owned) {
        self.arena[GLOBAL].jim_namespace = Some((
            JimNamespaceObject::Initial(std::rc::Rc::new(holder)),
            JimNamespaceName::Materialized(std::rc::Rc::from(&b""[..])),
        ));
    }

    /// The counted canonical name selected from the actual retained Jim object.
    pub(crate) fn jim_namespace_bytes(&self, token: NsId) -> Option<&[u8]> {
        let (holder, name) = self.arena.get(token)?.jim_namespace.as_ref()?;
        let JimNamespaceName::Materialized(bytes) = name else {
            return None;
        };
        let original = holder.owner()?;
        let resident = crate::obj::native_object_snapshot(original.as_ptr())
            .ok()?
            .resident?;
        (resident.as_ref() == bytes.as_ref()).then_some(bytes.as_ref())
    }

    /// Record only bytes obtained from this context's actual original object.
    pub(crate) fn record_jim_namespace_materialized(
        &mut self,
        token: NsId,
        original: *mut crate::obj::TclObj,
        bytes: std::rc::Rc<[u8]>,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        let context = self
            .arena
            .get_mut(token)
            .and_then(|namespace| namespace.jim_namespace.as_mut());
        let Some((holder, name)) = context else {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim namespace context owner",
            ));
        };
        if !holder
            .owner()
            .is_some_and(|holder| holder.as_ptr() == original)
            || !crate::obj::native_object_snapshot(original)?
                .resident
                .is_some_and(|resident| resident.as_ref() == bytes.as_ref())
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim namespace materialization owner",
            ));
        }
        *name = JimNamespaceName::Materialized(bytes);
        Ok(())
    }

    /// Retain the actual original Jim namespace object, without display reparsing.
    pub(crate) fn jim_namespace_object(&self, token: NsId) -> Option<crate::obj::Owned> {
        self.arena
            .get(token)?
            .jim_namespace
            .as_ref()
            .and_then(|(holder, _)| holder.owner())
            .map(|holder| crate::obj::Owned::retain(holder.as_ptr()))
    }

    /// Transfer a newly allocated native namespace reference to its actual
    /// frame or procedure. Published indexing retains only a weak lifetime.
    pub(crate) fn take_jim_namespace_owner(
        &mut self,
        token: NsId,
    ) -> Option<std::rc::Rc<crate::obj::Owned>> {
        let (slot, _) = self.arena.get_mut(token)?.jim_namespace.as_mut()?;
        let owner = slot.owner()?;
        match slot {
            JimNamespaceObject::Initial(_) => {
                *slot = JimNamespaceObject::Published(vec![std::rc::Rc::downgrade(&owner)]);
                Some(owner)
            }
            JimNamespaceObject::Published(owners) => {
                let owner = std::rc::Rc::new(crate::obj::Owned::retain(owner.as_ptr()));
                owners.retain(|owner| owner.strong_count() != 0);
                owners.insert(0, std::rc::Rc::downgrade(&owner));
                Some(owner)
            }
        }
    }

    /// Index an independently acquired actual frame role without retaining that role.
    pub(crate) fn remember_jim_namespace_owner(
        &mut self,
        token: NsId,
        owner: &std::rc::Rc<crate::obj::Owned>,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        let context = self
            .arena
            .get_mut(token)
            .and_then(|namespace| namespace.jim_namespace.as_mut());
        let Some((holder, _)) = context else {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim frame namespace context",
            ));
        };
        if !holder
            .owner()
            .is_some_and(|actual| actual.as_ptr() == owner.as_ptr())
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim frame namespace original",
            ));
        }
        match holder {
            JimNamespaceObject::Initial(_) => {
                return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "Jim frame namespace role transfer",
                ));
            }
            JimNamespaceObject::Published(owners) => {
                owners.retain(|actual| actual.strong_count() != 0);
                let birth = std::rc::Rc::downgrade(owner);
                if !owners
                    .first()
                    .is_some_and(|actual| std::rc::Weak::ptr_eq(actual, &birth))
                {
                    owners.insert(0, birth);
                }
            }
        }
        Ok(())
    }

    pub(crate) fn jim_namespace_original_owner(
        &self,
        token: NsId,
    ) -> Option<std::rc::Rc<crate::obj::Owned>> {
        self.arena.get(token)?.jim_namespace.as_ref()?.0.owner()
    }

    pub(super) fn jim_context(
        &self,
        token: NsId,
    ) -> Option<tcl_syntax::naming::NativeNameContext<'_>> {
        // Jim uses only the retained object. The constructed path is not an address.
        Some(tcl_syntax::naming::NativeNameContext::with_jim_namespace(
            &JIM_CONTEXT_PATH,
            self.jim_namespace_bytes(token)?,
        ))
    }
}
