// SPDX-License-Identifier: AGPL-3.0-or-later
//! Ownership extracted from one reached storage mutation, without new holds.

use super::{
    native_element_entry::NativeElementEntries, native_variable_names::NativeHashKey, Cell,
    RetiredArrayDefault, Var,
};
use crate::obj::{self, Owned};

enum RetiredPart {
    Value(Var),
    Default(RetiredArrayDefault),
    Key(Owned),
    Elements(NativeElementEntries),
    HashKey(NativeHashKey),
    Cell(Cell),
    Searches(tcl_core_types::NativeArraySearchChain<Owned>),
}

/// Only references already owned by the selected cell/table leave in this
/// carrier. Its caller must release cell and enclosing storage borrows before
/// retirement. Once selected, every owner is released even after first failure.
#[derive(Default)]
pub(crate) struct RetiredVariableParts(Vec<RetiredPart>);

impl RetiredVariableParts {
    pub(super) fn value(&mut self, old: Option<Var>) {
        if let Some(old) = old {
            self.0.push(RetiredPart::Value(old));
        }
    }
    pub(super) fn array_default(&mut self, old: Option<RetiredArrayDefault>) {
        if let Some(old) = old {
            self.0.push(RetiredPart::Default(old));
        }
    }
    pub(super) fn key(&mut self, old: Option<Owned>) {
        if let Some(old) = old {
            self.0.push(RetiredPart::Key(old));
        }
    }
    pub(super) fn elements(&mut self, old: NativeElementEntries) {
        self.0.push(RetiredPart::Elements(old));
    }
    pub(super) fn hash_key(&mut self, old: Option<NativeHashKey>) {
        if let Some(old) = old {
            self.0.push(RetiredPart::HashKey(old));
        }
    }
    pub(super) fn cell(&mut self, old: Cell) {
        self.0.push(RetiredPart::Cell(old));
    }

    pub(super) fn searches(&mut self, old: &mut tcl_core_types::NativeArraySearchChain<Owned>) {
        self.0.push(RetiredPart::Searches(std::mem::take(old)));
    }
    pub(super) fn extend(&mut self, mut old: Self) {
        self.0.append(&mut old.0);
    }

    pub(crate) fn finish<E>(mut self, current: impl Fn() -> Result<(), E>) -> Result<(), E> {
        let mut first = None;
        for old in std::mem::take(&mut self.0) {
            match old {
                RetiredPart::Value(Var::Array(values)) => {
                    for (_, value) in values {
                        // SAFETY: the removed member transfers its stored +1.
                        unsafe { obj::decr_ref_count(value) };
                        if let Err(error) = current() {
                            first.get_or_insert(error);
                        }
                    }
                }
                RetiredPart::Value(value) => {
                    value.release();
                    if let Err(error) = current() {
                        first.get_or_insert(error);
                    }
                }
                RetiredPart::Elements(elements) => {
                    if let Err(error) = elements.retire_with_check(&current) {
                        first.get_or_insert(error);
                    }
                }
                RetiredPart::Default(default) => {
                    drop(default);
                    if let Err(error) = current() {
                        first.get_or_insert(error);
                    }
                }
                RetiredPart::Key(key) => {
                    drop(key);
                    if let Err(error) = current() {
                        first.get_or_insert(error);
                    }
                }
                RetiredPart::HashKey(key) => {
                    drop(key);
                    if let Err(error) = current() {
                        first.get_or_insert(error);
                    }
                }
                RetiredPart::Searches(mut searches) => {
                    for handle in searches.take_handles() {
                        drop(handle);
                        if let Err(error) = current() {
                            first.get_or_insert(error);
                        }
                    }
                }
                RetiredPart::Cell(cell) => {
                    drop(cell);
                    if let Err(error) = current() {
                        first.get_or_insert(error);
                    }
                }
            }
        }
        if let Err(error) = current() {
            first.get_or_insert(error);
        }
        first.map_or(Ok(()), Err)
    }
}

impl Drop for RetiredVariableParts {
    fn drop(&mut self) {
        // Fallback is mandatory ownership cleanup, with no operation admission.
        for old in std::mem::take(&mut self.0) {
            match old {
                RetiredPart::Value(value) => value.release(),
                RetiredPart::Default(old) => drop(old),
                RetiredPart::Key(old) => drop(old),
                RetiredPart::Elements(old) => drop(old),
                RetiredPart::HashKey(old) => drop(old),
                RetiredPart::Cell(old) => drop(old),
                RetiredPart::Searches(old) => drop(old),
            }
        }
    }
}
