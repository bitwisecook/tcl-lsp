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

//! Cells selected by an original name object: Jim variable cells and cached links, namespace-only entries, C name cells and alias locals.

use super::captured_targets::{
    capture_original_indexed_receiver, original_indexed_link_target_using,
};
use super::trace_homes::TraceHome;
use super::{
    classify, entered_container, follow_links, resolve_at, separate_element, table, table_mut,
    NativeOriginalNameCell, Place, Resolved,
};
use crate::frame::{FrameStack, Link, VarError, VarHome};
use crate::namespace::{Namespaces, NsId};
use crate::obj;
use crate::obj::TclObj;
use tcl_syntax::naming::{NativeNameProtocol, NativeNameQualification};

/// Select the original ordinary Jim VarVal, retaining no table or value owner.
pub(crate) fn original_jim_variable_cell(
    frames: &FrameStack,
    ns: &Namespaces,
    current: NsId,
    name: &[u8],
) -> Option<crate::frame::WeakJimVariableCell> {
    let Resolved::Place(place) = classify(frames, ns, current, name) else {
        return None;
    };
    if place.elem.is_some() {
        return None;
    }
    table(frames, ns, place.home)?
        .weak_jim_cell(&place.name)
        .map(|cell| cell.with_home(place.home))
}

/// Explicit namespace-only lookup does not consult procedure locals or the
/// namespace-scope global fallback. Ordinary lookup retains its own policy.
pub(super) fn classify_original_name(
    frames: &FrameStack,
    ns: &Namespaces,
    current: NsId,
    name: &[u8],
    namespace: Option<NsId>,
) -> Resolved {
    let Some(namespace) = namespace else {
        return classify(frames, ns, current, name);
    };
    let Some(protocol) = ns.variable_name_protocol else {
        return Resolved::Error(VarError::NameProtocolUnavailable);
    };
    let input = protocol.variable_root_input(name);
    match ns.var_home(namespace, input.selected()) {
        Some((namespace, name)) => Resolved::Place(Place {
            home: VarHome::Namespace(namespace),
            name,
            elem: None,
            array_cell: None,
            scalar_entry: None,
        }),
        None => Resolved::Error(VarError::NoSuchNamespace),
    }
}

/// Resolve links after the original object selected its namespace-only entry.
pub(crate) fn original_namespace_link_target(
    frames: &FrameStack,
    ns: &Namespaces,
    namespace: NsId,
    root: &[u8],
    element: Option<Vec<u8>>,
) -> Result<Link, VarError> {
    let mut place = match classify_original_name(frames, ns, namespace, root, Some(namespace)) {
        Resolved::Place(place) => place,
        Resolved::Error(error) => return Err(error),
    };
    place.elem = element;
    let place = follow_links(frames, ns, place)?;
    Ok(Link {
        original_jim_target: None,
        native_scalar_entry: place
            .scalar_entry
            .as_ref()
            .map(|entry| entry.try_new_binding())
            .transpose()
            .map_err(|_| VarError::NameProtocolUnavailable)?,
        native_element_entry: None,
        array_identity: place
            .array_cell
            .as_ref()
            .map(crate::frame::RetainedArrayCell::identity),
        array_cell: place.array_cell,
        home: place.home,
        name: place.name,
        elem: place.elem,
    })
}

/// Capture the selected namespace cell without a second ordinary name lookup.
pub(crate) fn capture_original_namespace_receiver(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    namespace: NsId,
    root: &[u8],
    element: Option<Vec<u8>>,
    create: bool,
) -> Result<Option<(crate::frame::VariableReceiver, TraceHome)>, VarError> {
    let mut place = match classify_original_name(frames, ns, namespace, root, Some(namespace)) {
        Resolved::Place(place) => place,
        Resolved::Error(error) => return Err(error),
    };
    place.elem = element;
    let place = follow_links(frames, ns, place)?;
    capture_resolved_receiver(frames, ns, place, create)
}

/// Select the actual addressed store cell before publication and callbacks.
/// This supplies receiver/trace identity only; the caller owns naming, entered
/// currency and the independently selected observer operation.
pub(crate) fn capture_variable_store_receiver_at(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    name: &[u8],
    element: Option<Vec<u8>>,
    level: usize,
) -> Result<Option<(crate::frame::VariableReceiver, TraceHome)>, VarError> {
    if frames.table(level).is_none() {
        return Err(VarError::NameProtocolUnavailable);
    }
    let mut place = match resolve_at(frames, ns, name, level) {
        Resolved::Place(place) => place,
        Resolved::Error(error) => return Err(error),
    };
    if let Some(element) = element {
        if place.elem.is_some() {
            return Err(VarError::IsScalar);
        }
        place.elem = Some(separate_element(ns, name, &element)?);
    }
    capture_resolved_receiver(frames, ns, place, true)
}

fn capture_resolved_receiver(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    place: Place,
    create: bool,
) -> Result<Option<(crate::frame::VariableReceiver, TraceHome)>, VarError> {
    let selected = if let (Some(array), Some(element)) = (&place.array_cell, &place.elem) {
        if !array.is_live() {
            return Err(VarError::DeletedArray);
        }
        Some(array.capture_receiver(element.clone()))
    } else if let Some(entry) = &place.scalar_entry {
        entry.capture_receiver(place.elem.clone(), create)?
    } else if create {
        Some(table_mut(frames, ns, place.home).capture_receiver(&place.name, place.elem.clone())?)
    } else {
        table_mut(frames, ns, place.home).capture_get_receiver(&place.name, place.elem.clone())?
    };
    let (namespace, level) = match place.home {
        VarHome::Namespace(namespace) => (Some(namespace), None),
        VarHome::Frame(level) => (None, Some(level)),
    };
    Ok(selected.map(|receiver| {
        let home = TraceHome {
            binding_id: receiver.binding_id(),
            selected_member: receiver.trace_member(),
            ns: namespace,
            level,
            base: place.name,
            link_elem: place.elem,
        };
        (receiver, home)
    }))
}

/// A matching existing cache supplies an actual ordinal, never a name replay.
pub(crate) fn prepare_original_c_indexed_name_cell(
    frames: &mut FrameStack,
    ns: &Namespaces,
    index: usize,
) -> Result<NativeOriginalNameCell, VarError> {
    if !frames.prepare_native_compiled_name_cell(index) {
        return Err(VarError::NameProtocolUnavailable);
    }
    let (level, name, cell, link) = frames
        .original_compiled_name_observation(index)
        .ok_or(VarError::NameProtocolUnavailable)?;
    NativeOriginalNameCell::capture(
        frames,
        ns,
        VarHome::Frame(level),
        name,
        Some(index),
        cell,
        link,
    )
}

/// Project the already selected home/entry, with the existing link worker.
pub(crate) fn capture_original_c_selected_receiver(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    selected: &NativeOriginalNameCell,
    element: Option<Vec<u8>>,
    create: bool,
) -> Result<Option<(crate::frame::VariableReceiver, TraceHome)>, VarError> {
    selected
        .check_selected(frames, ns)
        .map_err(|_| VarError::NameProtocolUnavailable)?;
    if let Some(index) = selected.compiled {
        return capture_original_indexed_receiver(frames, ns, index, element, create);
    }
    let mut place = selected.place();
    place.elem = element;
    let place = follow_links(frames, ns, place)?;
    capture_resolved_receiver(frames, ns, place, create)
}

pub(crate) fn original_c_selected_link_target(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    selected: &NativeOriginalNameCell,
    element: Option<Vec<u8>>,
) -> Result<Link, VarError> {
    original_c_selected_link_target_using(frames, ns, selected, element, true)
}

pub(crate) fn selected_c_link_target(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    selected: &NativeOriginalNameCell,
    element: Option<Vec<u8>>,
) -> Result<Link, VarError> {
    original_c_selected_link_target_using(frames, ns, selected, element, false)
}

fn original_c_selected_link_target_using(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    selected: &NativeOriginalNameCell,
    element: Option<Vec<u8>>,
    acquire: bool,
) -> Result<Link, VarError> {
    selected
        .check_selected(frames, ns)
        .map_err(|_| VarError::NameProtocolUnavailable)?;
    if let Some(index) = selected.compiled {
        return original_indexed_link_target_using(frames, ns, index, element, acquire);
    }
    let mut place = selected.place();
    place.elem = element;
    let place = follow_links(frames, ns, place)?;
    Ok(Link {
        original_jim_target: None,
        native_scalar_entry: place
            .scalar_entry
            .as_ref()
            .map(|entry| {
                if acquire {
                    entry.try_new_binding()
                } else {
                    entry.select_new_binding()
                }
            })
            .transpose()
            .map_err(|_| VarError::NameProtocolUnavailable)?,
        native_element_entry: None,
        array_identity: place
            .array_cell
            .as_ref()
            .map(crate::frame::RetainedArrayCell::identity),
        array_cell: place.array_cell,
        home: place.home,
        name: place.name,
        elem: place.elem,
    })
}

/// Exact target table/root observed before old-local role withdrawal. No name
/// lookup or new native alias role is issued by this observation.
pub(crate) struct OriginalAliasTargetObservation {
    pub(super) table: entered_container::TableObservation,
    pub(super) cell: crate::frame::VariablePublicationObservation,
    pub(super) home: VarHome,
    pub(super) name: Vec<u8>,
    pub(super) compiled: Option<usize>,
}

impl OriginalAliasTargetObservation {
    pub(crate) fn capture(
        frames: &FrameStack,
        ns: &Namespaces,
        target: &Link,
    ) -> Result<Self, VarError> {
        Self::capture_home(frames, ns, target.home, &target.name)
    }
    pub(crate) fn capture_home(
        frames: &FrameStack,
        ns: &Namespaces,
        home: VarHome,
        name: &[u8],
    ) -> Result<Self, VarError> {
        let table = entered_container::TableObservation::capture(frames, ns, home)
            .ok_or(VarError::NameProtocolUnavailable)?;
        let selected = super::table(frames, ns, home).ok_or(VarError::NameProtocolUnavailable)?;
        let (cell, _) = selected.observe_original_name_cell(name)?;
        Ok(Self {
            table,
            cell,
            home,
            name: name.to_vec(),
            compiled: None,
        })
    }
    pub(crate) fn capture_compiled(
        frames: &mut FrameStack,
        ns: &Namespaces,
        slot: usize,
    ) -> Result<Self, VarError> {
        if !frames.prepare_native_compiled_name_cell(slot) {
            return Err(VarError::NameProtocolUnavailable);
        }
        let (level, name, cell, _) = frames
            .original_compiled_name_observation(slot)
            .ok_or(VarError::NameProtocolUnavailable)?;
        let home = VarHome::Frame(level);
        let table = entered_container::TableObservation::capture(frames, ns, home)
            .ok_or(VarError::NameProtocolUnavailable)?;
        Ok(Self {
            table,
            cell,
            home,
            name,
            compiled: Some(slot),
        })
    }
    pub(crate) fn check(
        &self,
        interpreter: &crate::interp::Interp,
        target: &Link,
        currency: &crate::interp::native_operation_currency::NativeOperationCurrency,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        self.check_cell(interpreter, currency)?;
        target.check_target_identity().map_err(|error| {
            tcl_cmd_core::CmdError::from_execution_refusal(
                interpreter.clone().refuse_completion_value_access(error),
            )
        })
    }
    pub(crate) fn check_cell(
        &self,
        interpreter: &crate::interp::Interp,
        currency: &crate::interp::native_operation_currency::NativeOperationCurrency,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        currency.ensure_current_or_refuse()?;
        let physical =
            entered_container::same_table(interpreter, self.home, &self.table).and_then(|()| {
                let frames = interpreter.frames.borrow();
                let ns = interpreter.namespaces();
                let same = match (self.home, self.compiled) {
                    (VarHome::Frame(level), Some(slot)) => {
                        frames.original_compiled_cell_at_is_current(level, slot, &self.cell)
                    }
                    (_, None) => super::table(&frames, &ns, self.home)
                        .is_some_and(|table| self.cell.matches_root(table, &self.name)),
                    _ => false,
                };
                if !same {
                    return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "original alias target table cell",
                    ));
                }
                self.cell.check_current()
            });
        physical.map_err(|error| {
            tcl_cmd_core::CmdError::from_execution_refusal(
                interpreter.clone().refuse_completion_value_access(error),
            )
        })?;
        currency.ensure_current_or_refuse().map_err(Into::into)
    }
}

/// Simple local-side alias receiver. It owns no original-name cache transition.
pub(crate) struct NativeAliasLocalCell {
    pub home: VarHome,
    pub name: Vec<u8>,
    pub compiled: Option<usize>,
}

pub(crate) fn prepare_original_c_alias_local(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    current: NsId,
    name: &[u8],
    original: *mut TclObj,
    protocol: tcl_syntax::native_variable_name::NativeVariableNameProtocol,
) -> Result<NativeAliasLocalCell, VarError> {
    let input = protocol.alias_local_input(name);
    let namespace = (!frames.in_proc()
        || input.qualification() != NativeNameQualification::Unqualified)
        .then_some(current);
    // MakeUpvar's local side selects TCL_AVOID_RESOLVERS: an unqualified
    // procedure local must not be redirected by the TclOO variable resolver.
    // naming.tcloo.original-constant-link-introspection
    // docs/design/analysis/name-resolution-proofs/tcloo-original-constant-link-introspection.md
    let place = if namespace.is_none() {
        Place {
            home: VarHome::Frame(frames.current_level()),
            name: input.selected().to_vec(),
            elem: None,
            array_cell: None,
            scalar_entry: None,
        }
    } else {
        match classify_original_name(frames, ns, current, input.selected(), namespace) {
            Resolved::Place(place) => place,
            Resolved::Error(error) => return Err(error),
        }
    };
    let selected = prepare_original_c_variable_cell_at_place(
        frames,
        ns,
        &place,
        input.selected(),
        original,
        true,
        protocol.version(),
    )?
    .ok_or(VarError::NameProtocolUnavailable)?;
    Ok(NativeAliasLocalCell {
        home: place.home,
        name: place.name,
        compiled: selected.compiled,
    })
}

/// Actual namespace selection shared by original scalar and element lookup.
/// Both tokens come from the live receiver; no spelling reconstructs either.
#[derive(Clone, Copy)]
pub(crate) struct NativeOriginalNameScope {
    pub current: NsId,
    pub namespace: Option<NsId>,
}

pub(crate) fn prepare_original_c_variable_cell(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    scope: NativeOriginalNameScope,
    name: &[u8],
    original: *mut TclObj,
    create: bool,
    version: tcl_dialect::TclVersion,
) -> Result<Option<NativeOriginalNameCell>, VarError> {
    let NativeOriginalNameScope { current, namespace } = scope;
    let place = match classify_original_name(frames, ns, current, name, namespace) {
        Resolved::Place(place) => place,
        Resolved::Error(error) => return Err(error),
    };
    prepare_original_c_variable_cell_at_place(frames, ns, &place, name, original, create, version)
}

/// Materialise the already selected cell without re-running another purpose's
/// namespace resolver. Original key/header ownership follows the same rules.
fn prepare_original_c_variable_cell_at_place(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    place: &Place,
    name: &[u8],
    original: *mut TclObj,
    create: bool,
    version: tcl_dialect::TclVersion,
) -> Result<Option<NativeOriginalNameCell>, VarError> {
    let local = match place.home {
        VarHome::Frame(level) => frames.native_compiled_name_index(level, &place.name),
        VarHome::Namespace(_) => None,
    };
    let qualified = NativeNameProtocol::C(version)
        .variable_root_input(name)
        .qualification()
        != NativeNameQualification::Unqualified;
    let selected = table_mut(frames, ns, place.home);
    let birth = !selected.has_native_name_cell(&place.name);
    if !selected.prepare_native_name_cell(&place.name, create) {
        return Ok(None);
    }
    if birth && local.is_none() && version >= tcl_dialect::TclVersion::V8_5 {
        let tail;
        let key = if qualified {
            tail = obj::Owned::fresh(obj::new_string_bytes(&place.name));
            tail.as_ptr()
        } else {
            original
        };
        selected.retain_native_key(&place.name, key, false);
    }
    let (cell, link) = selected.observe_original_name_cell(&place.name)?;
    NativeOriginalNameCell::capture(
        frames,
        ns,
        place.home,
        place.name.clone(),
        local,
        cell,
        link,
    )
    .map(Some)
}

/// Retain the new original hash key only after the native birth succeeded.
pub(crate) fn retain_original_jim_variable_key(
    frames: &FrameStack,
    ns: &Namespaces,
    current: NsId,
    name: &[u8],
    original: *mut TclObj,
) {
    if let Resolved::Place(place) = classify(frames, ns, current, name) {
        if place.elem.is_none() {
            if let Some(table) = table(frames, ns, place.home) {
                table.retain_jim_key(&place.name, original);
            }
        }
    }
}

/// Read a retained alias target through its selected physical frame/table.
pub(crate) fn read_jim_cached_link(
    frames: &FrameStack,
    ns: &Namespaces,
    link: &Link,
) -> Option<*mut TclObj> {
    let place = follow_links(
        frames,
        ns,
        Place {
            home: link.home,
            name: link.name.clone(),
            elem: link.elem.clone(),
            array_cell: link.array_cell.clone(),
            scalar_entry: link.native_scalar_entry.clone(),
        },
    )
    .ok()?;
    let selected = table(frames, ns, place.home)?;
    match place.elem {
        Some(key) => selected.load_elem(&place.name, &key),
        None => selected.load_scalar(&place.name),
    }
}

/// Capture the existing physical compatibility target once. This supplies no
/// original Jim recursive-name or dictionary COW purpose.
pub(crate) fn capture_jim_cached_link(
    frames: &FrameStack,
    ns: &Namespaces,
    link: &Link,
) -> Result<crate::frame::CapturedJimLinkRoot, VarError> {
    let place = follow_links(
        frames,
        ns,
        Place {
            home: link.home,
            name: link.name.clone(),
            elem: link.elem.clone(),
            array_cell: link.array_cell.clone(),
            scalar_entry: link.native_scalar_entry.clone(),
        },
    )?;
    if place.array_cell.is_some() || place.scalar_entry.is_some() {
        return Err(VarError::NameProtocolUnavailable);
    }
    table(frames, ns, place.home)
        .ok_or(VarError::NameProtocolUnavailable)?
        .capture_jim_link_root(&place.name, place.elem)
}
