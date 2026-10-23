// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual ensemble roots; configuration clones contain no native child roles.

use crate::Value;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use tcl_cmd_core::ensemble::EnsembleObjectRole;
use tcl_core_types::NameBytes;

#[derive(Clone, Default)]
pub(crate) struct NativeEnsembleObjects {
    pub(crate) map: Option<EnsembleObjectRole<NativeEnsembleRoot>>,
    pub(crate) subcommands: Option<EnsembleObjectRole<NativeEnsembleRoot>>,
    pub(crate) parameters: Option<EnsembleObjectRole<NativeEnsembleRoot>>,
    pub(crate) unknown: Option<EnsembleObjectRole<NativeEnsembleRoot>>,
    pub(crate) table: Rc<NativePrefixTable>,
}

pub(crate) struct NativeEnsembleRoot {
    original: crate::value::WeakNativeObject,
    owner: RefCell<Option<Value>>,
}
impl NativeEnsembleRoot {
    pub(crate) fn pending(original: &Value) -> Self {
        Self {
            original: original.downgrade_native_object(),
            owner: RefCell::new(None),
        }
    }
    pub(crate) fn owned(owner: Value) -> Self {
        Self {
            original: owner.downgrade_native_object(),
            owner: RefCell::new(Some(owner)),
        }
    }
    pub(crate) fn inspect<R>(&self, query: impl FnOnce(&Value) -> R) -> Option<R> {
        self.owner.borrow().as_ref().map(query)
    }
    fn activate(&self) {
        if self.owner.borrow().is_none() {
            *self.owner.borrow_mut() = self.original.upgrade();
        }
    }
}
impl NativeEnsembleObjects {
    pub(crate) fn activate(&self) {
        for role in [
            &self.map,
            &self.subcommands,
            &self.parameters,
            &self.unknown,
        ]
        .into_iter()
        .flatten()
        {
            let _ = role.inspect(NativeEnsembleRoot::activate);
        }
    }
}

#[derive(Default)]
pub(crate) struct NativePrefixTable {
    dirty: Cell<bool>,
    epoch: Cell<Option<u64>>,
    entries: RefCell<Option<Vec<(NameBytes, Value, bool)>>>,
}

impl NativePrefixTable {
    pub(crate) fn needs_build(&self, epoch: u64) -> bool {
        self.dirty.get() || self.epoch.get() != Some(epoch) || self.entries.borrow().is_none()
    }
    pub(crate) fn replace(&self, entries: Vec<(NameBytes, Value, bool)>, epoch: u64) {
        let old = self.entries.replace(Some(entries));
        self.dirty.set(false);
        self.epoch.set(Some(epoch));
        drop(old);
    }
    pub(crate) fn prefix(&self, name: &NameBytes) -> Option<(Value, bool)> {
        self.entries
            .borrow()
            .as_ref()?
            .iter()
            .find(|(key, _, _)| key == name)
            .map(|(_, root, mapped)| (root.clone(), *mapped))
    }
    fn retire(&self) {
        let old = self.entries.borrow_mut().take();
        drop(old);
    }
}

impl super::EnsembleDef {
    /// Borrow a current original target for passive compilation-entry inspection.
    /// No object conversion, lookup, or native child reference is introduced.
    pub(crate) fn with_original_map_target<R>(
        &self,
        member: &[u8],
        position: usize,
        query: impl FnOnce(&Value) -> R,
    ) -> Option<R> {
        self.originals
            .map
            .as_ref()?
            .inspect(|root| {
                root.inspect(|root| {
                    root.with_cached_dictionary_member(member, |prefix| {
                        let (backing, _) = prefix?.cached_list_representation()?;
                        let value = backing.get(position)?;
                        Some(query(value))
                    })
                    .flatten()
                })
                .flatten()
            })
            .flatten()
    }
}

pub(crate) fn retire_configuration(old: &super::EnsembleDef, new: Option<&super::EnsembleDef>) {
    for (old_role, new_role) in [
        (
            &old.originals.map,
            new.and_then(|c| c.originals.map.as_ref()),
        ),
        (
            &old.originals.subcommands,
            new.and_then(|c| c.originals.subcommands.as_ref()),
        ),
        (
            &old.originals.parameters,
            new.and_then(|c| c.originals.parameters.as_ref()),
        ),
        (
            &old.originals.unknown,
            new.and_then(|c| c.originals.unknown.as_ref()),
        ),
    ] {
        if let Some(role) = old_role
            && new_role.is_none_or(|next| !role.same_role(next))
        {
            role.retire();
        }
    }
    if new.is_some() {
        old.originals.table.dirty.set(true);
    } else {
        old.originals.table.retire();
    }
}
