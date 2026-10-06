// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual ensemble object roles; cloned configurations are metadata queries.

use crate::obj::{Owned, TclObj};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use tcl_cmd_core::ensemble::EnsembleObjectRole;

#[derive(Clone, Default)]
pub(crate) struct NativeEnsembleObjects {
    pub(crate) map: Option<EnsembleObjectRole<NativeEnsembleRoot>>,
    pub(crate) subcommands: Option<EnsembleObjectRole<NativeEnsembleRoot>>,
    pub(crate) parameters: Option<EnsembleObjectRole<NativeEnsembleRoot>>,
    pub(crate) unknown: Option<EnsembleObjectRole<NativeEnsembleRoot>>,
    pub(crate) table: Rc<NativePrefixTable>,
}

pub(crate) struct NativeEnsembleRoot {
    original: *mut TclObj,
    owner: RefCell<Option<Owned>>,
}
impl NativeEnsembleRoot {
    pub(crate) fn pending(original: *mut TclObj) -> Self {
        Self {
            original,
            owner: RefCell::new(None),
        }
    }
    pub(crate) fn owned(owner: Owned) -> Self {
        Self {
            original: owner.as_ptr(),
            owner: RefCell::new(Some(owner)),
        }
    }
    fn activate(&self) {
        if self.owner.borrow().is_none() {
            *self.owner.borrow_mut() = Some(Owned::retain(self.original));
        }
    }
    fn pointer(&self) -> Option<*mut TclObj> {
        self.owner.borrow().as_ref().map(Owned::as_ptr)
    }
}

impl std::fmt::Debug for NativeEnsembleObjects {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeEnsembleObjects")
            .finish_non_exhaustive()
    }
}

type NativePrefixEntries = Vec<(Vec<u8>, Owned, bool)>;

#[derive(Default)]
pub(crate) struct NativePrefixTable {
    dirty: Cell<bool>,
    epoch: Cell<Option<u64>>,
    entries: RefCell<Option<NativePrefixEntries>>,
}

impl NativePrefixTable {
    pub(crate) fn needs_build(&self, epoch: u64) -> bool {
        self.dirty.get() || self.epoch.get() != Some(epoch) || self.entries.borrow().is_none()
    }
    pub(crate) fn replace(&self, entries: NativePrefixEntries, epoch: u64) {
        let old = self.entries.replace(Some(entries));
        self.dirty.set(false);
        self.epoch.set(Some(epoch));
        drop(old);
    }
    pub(crate) fn prefix(&self, name: &[u8]) -> Option<(*mut TclObj, bool)> {
        self.entries
            .borrow()
            .as_ref()?
            .iter()
            .find(|(key, _, _)| key == name)
            .map(|(_, root, mapped)| (root.as_ptr(), *mapped))
    }
    fn retire(&self) {
        let old = self.entries.borrow_mut().take();
        drop(old);
    }
}

impl NativeEnsembleObjects {
    pub(crate) fn pointer(
        role: &Option<EnsembleObjectRole<NativeEnsembleRoot>>,
    ) -> Option<*mut TclObj> {
        role.as_ref()?
            .inspect(NativeEnsembleRoot::pointer)
            .flatten()
    }
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

pub(crate) fn retire_configuration(
    old: &super::EnsembleConfig,
    new: Option<&super::EnsembleConfig>,
) {
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
        if let Some(role) = old_role {
            if new_role.is_none_or(|next| !role.same_role(next)) {
                role.retire();
            }
        }
    }
    if new.is_some() {
        old.originals.table.dirty.set(true);
    } else {
        old.originals.table.retire();
    }
}
