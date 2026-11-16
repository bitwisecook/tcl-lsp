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

//! Cells resolved once and retained across callbacks: array targets, variable receivers and indexed link targets, with their array operations.

use super::trace_homes::TraceHome;
use super::{
    entered_container, resolve, resolve_at, separate_element, table, table_mut, ArrayCellTarget,
    Resolved,
};
use crate::frame::{FrameStack, Link, VarError, VarHome};
use crate::namespace::{Namespaces, NsId};
use crate::obj::TclObj;
use tcl_runtime_api::VarId;

/// Resolve an array-command spelling to the concrete variable binding it
/// currently reaches. The binding may be undefined or scalar; callers retain
/// it so a callback cannot steer the operation onto a replacement.
pub(crate) fn array_target_at(
    frames: &FrameStack,
    ns: &Namespaces,
    name: &[u8],
    level: usize,
) -> Option<ArrayCellTarget> {
    let Resolved::Place(place) = resolve_at(frames, ns, name, level) else {
        return None;
    };
    if place.elem.is_some() {
        return None;
    }
    let cell = if let Some(entry) = &place.scalar_entry {
        entry.array_operation_cell()?
    } else {
        table(frames, ns, place.home)?.array_operation_cell(&place.name)?
    };
    let id = cell.identity();
    let array_cell = cell.array();
    Some(ArrayCellTarget {
        table: entered_container::TableObservation::capture(frames, ns, place.home)?,
        id,
        home: place.home,
        root: place.name,
        array_cell,
        cell,
    })
}

/// Resolve once, then retain the actual root/element cell across callbacks.
pub(crate) fn capture_variable_receiver(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    current_ns: NsId,
    name: &[u8],
    element: Option<&[u8]>,
) -> Result<crate::frame::VariableReceiver, VarError> {
    let mut place = match resolve(frames, ns, current_ns, name) {
        Resolved::Place(place) => place,
        Resolved::Error(error) => return Err(error),
    };
    if let Some(element) = element {
        if place.elem.is_some() {
            return Err(VarError::IsScalar);
        }
        place.elem = Some(separate_element(ns, name, element)?.to_vec());
    }
    if let (Some(array), Some(element)) = (&place.array_cell, &place.elem) {
        return Ok(array.capture_receiver(element.clone()));
    }
    if let Some(entry) = place.scalar_entry {
        return entry
            .capture_receiver(place.elem, true)?
            .ok_or(VarError::NameProtocolUnavailable);
    }
    table_mut(frames, ns, place.home).capture_receiver(&place.name, place.elem)
}

/// Native GET does not create a missing root, but permits element shells in
/// an existing selected array. The receiver remains physical across callbacks.
pub(crate) fn capture_get_variable_receiver(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    current_ns: NsId,
    name: &[u8],
    element: Option<&[u8]>,
) -> Result<Option<crate::frame::VariableReceiver>, VarError> {
    let mut place = match resolve(frames, ns, current_ns, name) {
        Resolved::Place(place) => place,
        Resolved::Error(VarError::NameProtocolUnavailable) => {
            return Err(VarError::NameProtocolUnavailable);
        }
        Resolved::Error(_) => return Ok(None),
    };
    if let Some(element) = element {
        if place.elem.is_some() {
            return Ok(None);
        }
        place.elem = Some(separate_element(ns, name, element)?.to_vec());
    }
    if let (Some(array), Some(element)) = (&place.array_cell, &place.elem) {
        return Ok(array
            .is_live()
            .then(|| array.capture_receiver(element.clone())));
    }
    if let Some(entry) = place.scalar_entry {
        return entry.capture_receiver(place.elem, false);
    }
    table_mut(frames, ns, place.home).capture_get_receiver(&place.name, place.elem)
}

/// Follow the actual indexed local, never its canonical reporting spelling.
pub(crate) fn capture_original_indexed_receiver(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    index: usize,
    element: Option<Vec<u8>>,
    create: bool,
) -> Result<Option<(crate::frame::VariableReceiver, TraceHome)>, VarError> {
    if let Some(link) = frames.original_compiled_link(index) {
        if link
            .native_element_entry
            .as_ref()
            .is_some_and(|entry| !entry.is_live())
        {
            return Err(VarError::DeletedArray);
        }
        if matches!(link.home, VarHome::Namespace(namespace) if ns.namespace_variables_are_deleted(namespace))
        {
            return Err(VarError::DeletedNamespace);
        }
        if link.elem.is_some() && element.is_some() {
            return Err(VarError::IsScalar);
        }
        let selected_element = element.or_else(|| link.elem.clone());
        let receiver = if let Some(array) = &link.array_cell {
            if !array.is_live() {
                return Err(VarError::DeletedArray);
            }
            Some(
                array.capture_receiver(
                    selected_element
                        .clone()
                        .ok_or(VarError::NameProtocolUnavailable)?,
                ),
            )
        } else if let Some(cell) = &link.native_scalar_entry {
            cell.capture_receiver(selected_element, create)?
        } else {
            let target = table_mut(frames, ns, link.home);
            if create {
                Some(target.capture_receiver(&link.name, selected_element)?)
            } else {
                target.capture_get_receiver(&link.name, selected_element)?
            }
        };
        return Ok(receiver.map(|receiver| {
            let (namespace, level) = match link.home {
                VarHome::Namespace(namespace) => (Some(namespace), None),
                VarHome::Frame(level) => (None, Some(level)),
            };
            let home = TraceHome {
                binding_id: receiver.binding_id(),
                selected_member: receiver.trace_member(),
                ns: namespace,
                level,
                base: link.name,
                link_elem: link.elem,
            };
            (receiver, home)
        }));
    }
    let level = frames.current_level();
    let name = frames
        .compiled_slot_name(index)
        .ok_or(VarError::NameProtocolUnavailable)?
        .to_vec();
    let receiver = frames.capture_original_compiled_receiver(index, element, create)?;
    Ok(receiver.map(|receiver| {
        let home = TraceHome {
            binding_id: receiver.binding_id(),
            selected_member: receiver.trace_member(),
            ns: None,
            level: Some(level),
            base: name,
            link_elem: None,
        };
        (receiver, home)
    }))
}

#[cfg(test)]
pub(crate) fn original_indexed_link_target(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    index: usize,
    element: Option<Vec<u8>>,
) -> Result<Link, VarError> {
    original_indexed_link_target_using(frames, ns, index, element, true)
}

pub(super) fn original_indexed_link_target_using(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    index: usize,
    element: Option<Vec<u8>>,
    acquire: bool,
) -> Result<Link, VarError> {
    if let Some(mut link) = frames.original_compiled_link(index) {
        if link
            .native_element_entry
            .as_ref()
            .is_some_and(|entry| !entry.is_live())
            || link
                .array_cell
                .as_ref()
                .is_some_and(|array| !array.is_live())
        {
            return Err(VarError::DeletedArray);
        }
        if matches!(link.home, VarHome::Namespace(namespace) if ns.namespace_variables_are_deleted(namespace))
        {
            return Err(VarError::DeletedNamespace);
        }
        if let Some(entry) = &link.native_scalar_entry {
            entry.capture_receiver(None, false)?;
        }
        if element.is_none() {
            if let Some(entry) = &link.native_scalar_entry {
                link.native_scalar_entry = Some(
                    if acquire {
                        entry.try_new_binding()
                    } else {
                        entry.select_new_binding()
                    }
                    .map_err(|_| VarError::NameProtocolUnavailable)?,
                );
            }
            if let Some(entry) = &link.native_element_entry {
                link.native_element_entry = Some(
                    if acquire {
                        entry.try_new_binding()
                    } else {
                        entry.select_new_binding()
                    }
                    .map_err(|_| VarError::NameProtocolUnavailable)?,
                );
            }
            return Ok(link);
        }
    }
    let name = frames
        .compiled_slot_name(index)
        .ok_or(VarError::NameProtocolUnavailable)?
        .to_vec();
    let level = frames.current_level();
    let mut link = Link {
        original_jim_target: None,
        native_scalar_entry: None,
        native_element_entry: None,
        array_identity: None,
        array_cell: None,
        home: VarHome::Frame(level),
        name,
        elem: element.clone(),
    };
    if element.is_none() {
        link.native_scalar_entry = frames.select_original_compiled_alias(index, acquire);
        return Ok(link);
    }
    let (receiver, home) = capture_original_indexed_receiver(frames, ns, index, element, true)?
        .ok_or(VarError::NameProtocolUnavailable)?;
    let array = receiver.selected_array().ok_or(VarError::IsScalar)?;
    // The receiver's empty element shell retires with it; the alias then
    // selects the array's own member entry, which outlives this operation.
    drop(receiver);
    link.home = match (home.ns, home.level) {
        (Some(namespace), _) => VarHome::Namespace(namespace),
        (_, Some(level)) => VarHome::Frame(level),
        _ => return Err(VarError::NameProtocolUnavailable),
    };
    link.name = home.base;
    link.array_identity = Some(array.identity());
    link.native_element_entry =
        array.select_element_alias(link.elem.as_deref().expect("element alias"), acquire);
    link.array_cell = Some(array);
    Ok(link)
}

/// Enumerate an exact binding only while it is still the array selected when
/// an operation began.
pub(crate) fn array_names_at_target(
    frames: &FrameStack,
    ns: &Namespaces,
    target: &ArrayCellTarget,
) -> Option<Vec<Vec<u8>>> {
    let _ = (frames, ns);
    Some(target.cell.array()?.elements())
}

/// Check the actual original table/frame and captured array cell. This uses
/// the receipt captured by LocateArray, never a new lookup of its spelling.
pub(crate) fn array_search_target_current(
    interpreter: &crate::interp::Interp,
    target: &ArrayCellTarget,
) -> Result<(), tcl_syntax::value::ValueError> {
    entered_container::same_table(interpreter, target.home, &target.table)?;
    let same_binding = {
        let frames = interpreter.frames.borrow();
        let namespaces = interpreter.namespaces();
        table(&frames, &namespaces, target.home)
            .is_some_and(|table| table.trace_binding_identity(&target.root) == Some(target.id))
    };
    if same_binding
        && target
            .array_cell
            .as_ref()
            .is_some_and(|array| array.search_is_current())
    {
        Ok(())
    } else {
        Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "original array search cell",
        ))
    }
}

/// Physical search rows belong to the captured array incarnation, not its name.
pub(crate) fn array_search_keys_at_target(target: &ArrayCellTarget) -> Option<Vec<Vec<u8>>> {
    target.array_cell.as_ref()?.search_keys()
}

/// Inspect one candidate in the same captured incarnation without callbacks.
pub(crate) fn array_search_element_exists_at_target(target: &ArrayCellTarget, key: &[u8]) -> bool {
    target
        .array_cell
        .as_ref()
        .is_some_and(|array| array.read(key).is_some())
}

/// Remove a byte key only from the array binding captured for this operation.
pub(crate) fn unset_element_at_target(
    target: &ArrayCellTarget,
    key: &[u8],
    currency: &crate::interp::native_operation_currency::NativeOperationCurrency,
) -> Result<Option<(bool, TraceHome)>, crate::frame::JimLinkStoreError> {
    currency.ensure_current_or_refuse().map_err(|first| {
        crate::frame::JimLinkStoreError::Command(tcl_cmd_core::CmdError::from_execution_refusal(
            first,
        ))
    })?;
    let Some(array) = target.cell.array() else {
        return Ok(None);
    };
    let Some(member) = array.trace_member_identity(key) else {
        return Ok(None);
    };
    let receiver = array.capture_receiver(key.to_vec());
    let home = target.selected_trace_home(Some((key.to_vec(), member)));
    let reached = receiver.unset_checked(currency);
    drop(receiver);
    currency.ensure_current_or_refuse().map_err(|first| {
        crate::frame::JimLinkStoreError::Command(tcl_cmd_core::CmdError::from_execution_refusal(
            first,
        ))
    })?;
    reached.map(|removed| Some((removed, home)))
}

pub(crate) fn array_default_state_at_target(
    frames: &FrameStack,
    ns: &Namespaces,
    target: &ArrayCellTarget,
) -> tcl_runtime_api::ArrayDefaultState<*mut TclObj> {
    let _ = (frames, ns);
    target.cell.default_state()
}

pub(crate) fn take_array_default_at_target(
    target: &ArrayCellTarget,
) -> Option<crate::frame::RetiredArrayDefault> {
    target.cell.take_default()
}

pub(crate) fn array_target_is_set(
    frames: &FrameStack,
    ns: &Namespaces,
    target: &ArrayCellTarget,
) -> bool {
    let _ = (frames, ns);
    target.cell.is_set()
}

pub(crate) fn retain_array_target(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    target: &ArrayCellTarget,
) -> bool {
    let _ = (frames, ns);
    target.cell.retain_operation()
}

pub(crate) fn release_array_target(target: &ArrayCellTarget) {
    target.cell.release_operation();
}

/// Stable identities of the live array and element reached by a spelling just
/// before its read traces fire.
pub(crate) fn array_element_target_at(
    frames: &FrameStack,
    ns: &Namespaces,
    name: &[u8],
    key: &[u8],
    level: usize,
) -> Option<(ArrayCellTarget, VarId)> {
    let target = array_target_at(frames, ns, name, level)?;
    let element = target.cell.element_id(key)?;
    Some((target, element))
}

pub(crate) fn retain_array_element_target(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    target: &ArrayCellTarget,
    key: &[u8],
    element: VarId,
) -> bool {
    let _ = (frames, ns);
    target.cell.retain_element(key, element)
}

pub(crate) fn release_array_element_target(target: &ArrayCellTarget, key: &[u8], element: VarId) {
    target.cell.release_element(key, element);
}

/// Read the exact element selected before a callback, returning `None` after
/// either the array or the element was unset and recreated under the same name.
pub(crate) fn get_element_at_target(
    frames: &FrameStack,
    ns: &Namespaces,
    target: &ArrayCellTarget,
    key: &[u8],
    element: VarId,
) -> Option<*mut TclObj> {
    let _ = (frames, ns);
    target.cell.read_element(key, element)
}
