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

//! The by-name coordinator API: `set`, `get`, `unset`, array defaults, constants and `is_array`, with their frame-addressed variants.

#[cfg(test)]
use super::read_retained;
#[cfg(test)]
use super::separate_element;
#[cfg(test)]
use super::store_retained;
#[cfg(test)]
use super::unset_retained;
use super::{classify, follow_links, resolve, resolve_at, table, table_mut, Place, Resolved};
use crate::frame::{FrameStack, VarError, VarHome};
use crate::namespace::{Namespaces, NsId};
use crate::obj::TclObj;

// the public coordinator API (mirrors the old FrameStack surface)

/// Select a definition-time static source. Reference capture retains the raw
/// alias slot, while copy capture follows its currently selected destination.
pub(crate) fn capture_static_source(
    frames: &FrameStack,
    ns: &Namespaces,
    current_ns: NsId,
    name: &[u8],
    reference: bool,
) -> Option<crate::frame::RetainedVariableCell> {
    let Resolved::Place(raw) = classify(frames, ns, current_ns, name) else {
        return None;
    };
    if reference {
        return table(frames, ns, raw.home)?.capture_cell(&raw.name);
    }
    let resolved = follow_links(
        frames,
        ns,
        Place {
            home: raw.home,
            name: raw.name.clone(),
            elem: raw.elem.clone(),
            array_cell: raw.array_cell.clone(),
            scalar_entry: raw.scalar_entry.clone(),
        },
    )
    .ok()?;
    if resolved.elem.is_some() || !table(frames, ns, resolved.home)?.is_set(&resolved.name) {
        return None;
    }
    table(frames, ns, resolved.home)?.capture_cell(&resolved.name)
}

pub(crate) enum CapturedByteStore {
    Retained(crate::frame::VariableReceiver),
    Generic {
        home: VarHome,
        root: crate::frame::CapturedJimLinkRoot,
    },
}

impl CapturedByteStore {
    pub(crate) fn store(
        &mut self,
        value: *mut TclObj,
        currency: &crate::interp::native_operation_currency::NativeOperationCurrency,
    ) -> Result<(), crate::frame::JimLinkStoreError> {
        match self {
            Self::Retained(receiver) => {
                currency.ensure_current_or_refuse().map_err(|error| {
                    crate::frame::JimLinkStoreError::Command(
                        tcl_cmd_core::CmdError::from_execution_refusal(error),
                    )
                })?;
                let reached = receiver.store(value);
                currency.ensure_current_or_refuse().map_err(|error| {
                    crate::frame::JimLinkStoreError::Command(
                        tcl_cmd_core::CmdError::from_execution_refusal(error),
                    )
                })?;
                receiver.check_current()?;
                reached.map_err(Into::into)
            }
            Self::Generic { root, .. } => root.store(value, currency),
        }
    }
    pub(crate) fn check(
        &self,
        frames: &FrameStack,
        ns: &Namespaces,
        currency: &crate::interp::native_operation_currency::NativeOperationCurrency,
    ) -> Result<(), crate::frame::JimLinkStoreError> {
        currency.ensure_current_or_refuse().map_err(|error| {
            crate::frame::JimLinkStoreError::Command(
                tcl_cmd_core::CmdError::from_execution_refusal(error),
            )
        })?;
        match self {
            Self::Retained(receiver) => receiver.check_current().map_err(Into::into),
            Self::Generic { home, root } => {
                root.check(currency)?;
                if !table(frames, ns, *home).is_some_and(|table| root.matches_table(table)) {
                    return Err(crate::frame::JimLinkStoreError::Command(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "captured byte variable table",
                        )
                        .into(),
                    ));
                }
                Ok(())
            }
        }
    }
}

/// `set name value` — write through links to wherever `name` resolves. The cell
/// takes a **+1** on `obj`. A qualified write into a missing namespace errors.
#[cfg(test)]
pub(crate) fn set(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    current_ns: NsId,
    name: &[u8],
    obj: *mut TclObj,
) -> Result<(), VarError> {
    let place = match resolve(frames, ns, current_ns, name) {
        Resolved::Place(p) => p,
        Resolved::Error(error) => return Err(error),
    };
    if let Some(result) = store_retained(&place, obj) {
        return result;
    }
    let t = table_mut(frames, ns, place.home);
    match place.elem {
        Some(elem) => place.array_cell.as_ref().map_or_else(
            || t.store_elem(&place.name, &elem, obj),
            |cell| cell.store(&elem, obj),
        ),
        None => t.store_scalar(&place.name, obj),
    }
}

/// `set name(key) value`. Errors if `name` is a scalar / its ns is missing.
#[cfg(test)]
pub(crate) fn set_elem(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    current_ns: NsId,
    name: &[u8],
    key: &[u8],
    obj: *mut TclObj,
) -> Result<(), VarError> {
    let key = separate_element(ns, name, key)?;
    let place = match resolve(frames, ns, current_ns, name) {
        Resolved::Place(p) => p,
        Resolved::Error(error) => return Err(error),
    };
    if place.elem.is_some() {
        // `a(b)(c)` — the resolved place is already an element.
        return Err(VarError::IsScalar);
    }
    let place = Place {
        elem: Some(key.clone()),
        ..place
    };
    if let Some(result) = store_retained(&place, obj) {
        return result;
    }
    table_mut(frames, ns, place.home).store_elem(&place.name, &key, obj)
}

/// `set name` — borrowed value, or `None` (unset / missing namespace).
#[cfg(test)]
pub(crate) fn get(
    frames: &FrameStack,
    ns: &Namespaces,
    current_ns: NsId,
    name: &[u8],
) -> Option<*mut TclObj> {
    match resolve(frames, ns, current_ns, name) {
        Resolved::Place(p) => {
            if let Some(result) = read_retained(&p) {
                return result.ok().flatten();
            }
            let t = table(frames, ns, p.home)?;
            match &p.elem {
                Some(elem) => p
                    .array_cell
                    .as_ref()
                    .map_or_else(|| t.load_elem(&p.name, elem), |cell| cell.read(elem)),
                None => t.load_scalar(&p.name),
            }
        }
        Resolved::Error(_) => None,
    }
}

// frame-addressed access (the `VarStore` `FrameId`-honouring path)
//
// These resolve `name` as if `level` were the active frame, following links —
// the `set`/`get`/`unset`/`exists` above are exactly these at the active level.
// The runtime's `VarStore` uses them only for a non-active `FrameId`; the active
// frame keeps the by-name accessors verbatim.

/// Frame-addressed [`get`] — read `name` resolved against frame `level`.
#[cfg(test)]
pub(crate) fn get_at(
    frames: &FrameStack,
    ns: &Namespaces,
    name: &[u8],
    level: usize,
) -> Option<*mut TclObj> {
    match resolve_at(frames, ns, name, level) {
        Resolved::Place(p) => {
            if let Some(result) = read_retained(&p) {
                return result.ok().flatten();
            }
            let t = table(frames, ns, p.home)?;
            match &p.elem {
                Some(elem) => p
                    .array_cell
                    .as_ref()
                    .map_or_else(|| t.load_elem(&p.name, elem), |cell| cell.read(elem)),
                None => t.load_scalar(&p.name),
            }
        }
        Resolved::Error(_) => None,
    }
}

/// Frame-addressed array-element read. `name` and `key` stay separate so an
/// array base containing `(` is never reconstructed and parsed again.
#[cfg(test)]
pub(crate) fn get_elem_at(
    frames: &FrameStack,
    ns: &Namespaces,
    name: &[u8],
    key: &[u8],
    level: usize,
) -> Option<*mut TclObj> {
    let key = separate_element(ns, name, key).ok()?;
    match resolve_at(frames, ns, name, level) {
        Resolved::Place(p) if p.elem.is_none() => {
            let p = Place {
                elem: Some(key.clone()),
                ..p
            };
            if let Some(result) = read_retained(&p) {
                return result.ok().flatten();
            }
            table(frames, ns, p.home)?.load_elem(&p.name, &key)
        }
        _ => None,
    }
}

/// `set name(key)` — borrowed.
#[cfg(test)]
pub(crate) fn get_elem(
    frames: &FrameStack,
    ns: &Namespaces,
    current_ns: NsId,
    name: &[u8],
    key: &[u8],
) -> Option<*mut TclObj> {
    let key = separate_element(ns, name, key).ok()?;
    match resolve(frames, ns, current_ns, name) {
        Resolved::Place(p) if p.elem.is_none() => {
            let t = table(frames, ns, p.home)?;
            let p = Place {
                elem: Some(key.clone()),
                ..p
            };
            if let Some(result) = read_retained(&p) {
                return result.ok().flatten();
            }
            // A missing element of an array with a TIP 508 default reads as the
            // default (without creating the element).
            t.load_elem(&p.name, &key)
                .or_else(|| t.array_default(&p.name))
        }
        _ => None,
    }
}

/// Fresh native creating lookup with physical root/element classification.
pub(crate) fn replace_array_default_classified(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    current_ns: NsId,
    name: &[u8],
    obj: *mut TclObj,
) -> Result<
    Result<Option<crate::frame::RetiredArrayDefault>, tcl_runtime_api::ArrayDefaultSetFailure>,
    VarError,
> {
    use tcl_runtime_api::ArrayDefaultSetFailure;
    let protocol = ns
        .variable_name_protocol
        .ok_or(VarError::NameProtocolUnavailable)?;
    let input = protocol.combined_variable_input(name);
    let original_element = input.element().is_some();
    let place = match resolve(frames, ns, current_ns, input.root().selected()) {
        Resolved::Place(place) => place,
        Resolved::Error(error) => return Err(error),
    };
    let table = table_mut(frames, ns, place.home);
    if original_element || place.elem.is_some() {
        // The creating lookup may initialise a missing array root. Its unused
        // undefined element is cleaned up before the command reports NEEDARRAY.
        table.ensure_array(&place.name)?;
        return Ok(Err(ArrayDefaultSetFailure::Element));
    }
    if let Err(VarError::IsScalar) = table.ensure_array(&place.name) {
        return Ok(Err(ArrayDefaultSetFailure::Scalar));
    }
    table.ensure_array(&place.name)?;
    Ok(Ok(table.replace_array_default(&place.name, obj)))
}

/// `unset name` — remove the variable `name` resolves to (following links).
/// Returns whether it existed.
#[cfg(test)]
pub(crate) fn unset(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    current_ns: NsId,
    name: &[u8],
) -> bool {
    let place = match resolve(frames, ns, current_ns, name) {
        Resolved::Place(p) => p,
        Resolved::Error(_) => return false,
    };
    let root = place.elem.is_none();
    if let Some(result) = unset_retained(&place) {
        let existed = result.unwrap_or(false);
        if root && existed {
            frames.invalidate_jim_variable_frame(place.home);
        }
        return existed;
    }
    let t = table_mut(frames, ns, place.home);
    let root_defined = root && t.is_set(&place.name);
    let existed = match place.elem {
        Some(elem) => place.array_cell.as_ref().map_or_else(
            || t.remove_elem(&place.name, &elem),
            |cell| cell.remove(&elem),
        ),
        None => {
            t.remove(&place.name);
            root_defined
        }
    };
    if root && existed {
        frames.invalidate_jim_variable_frame(place.home);
    }
    existed
}

/// Flag the scalar `name` (following links to its home) `const` — the `const`
/// command, after the value has been stored.
pub(crate) fn mark_constant(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    current_ns: NsId,
    name: &[u8],
) {
    if let Resolved::Place(p) = resolve(frames, ns, current_ns, name) {
        if p.elem.is_none() {
            table_mut(frames, ns, p.home).mark_constant(&p.name);
        }
    }
}

/// Whether `name` (following links) resolves to a `const` scalar.
pub(crate) fn is_constant(
    frames: &FrameStack,
    ns: &Namespaces,
    current_ns: NsId,
    name: &[u8],
) -> bool {
    match resolve(frames, ns, current_ns, name) {
        Resolved::Place(p) if p.elem.is_none() => {
            table(frames, ns, p.home).is_some_and(|t| t.is_constant(&p.name))
        }
        _ => false,
    }
}

/// Frame-addressed [`is_constant`]: resolve `name` as if `level` were active.
pub(crate) fn is_constant_at(
    frames: &FrameStack,
    ns: &Namespaces,
    name: &[u8],
    level: usize,
) -> bool {
    match resolve_at(frames, ns, name, level) {
        Resolved::Place(p) if p.elem.is_none() => {
            table(frames, ns, p.home).is_some_and(|t| t.is_constant(&p.name))
        }
        _ => false,
    }
}

/// `info consts` candidates in the active frame: direct constant cells plus
/// automatic `TclOO` instance projections whose resolved namespace target is
/// constant. Ordinary `global`/`upvar`/`variable` aliases are excluded.
pub(crate) fn const_names(frames: &FrameStack, ns: &Namespaces) -> Vec<Vec<u8>> {
    let mut names = frames.const_names();
    let seen = frames.local_names_without_declared_variables();
    if let Ok(declared) = frames.declared_tcloo_variable_bindings() {
        for (namespace, name, storage) in declared {
            if !seen.contains(&name) && ns.var_table(namespace).is_constant(&storage) {
                names.push(name);
            }
        }
    }
    names.sort();
    names.dedup();
    names
}

/// Whether `name` resolves to an array variable (the `set a` array-vs-scalar
/// diagnostic; `array exists`).
#[cfg(test)]
pub(crate) fn is_array(
    frames: &FrameStack,
    ns: &Namespaces,
    current_ns: NsId,
    name: &[u8],
) -> bool {
    match resolve(frames, ns, current_ns, name) {
        Resolved::Place(p) if p.elem.is_none() => {
            if let Some(entry) = &p.scalar_entry {
                return entry
                    .capture_receiver(None, false)
                    .ok()
                    .flatten()
                    .is_some_and(|cell| cell.is_array());
            }
            table(frames, ns, p.home).is_some_and(|t| t.is_array(&p.name))
        }
        _ => false,
    }
}
