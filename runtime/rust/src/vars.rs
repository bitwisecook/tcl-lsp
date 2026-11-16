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

//! The variable resolver — the variable parallel of the command resolver.
//!
//! One classification + one link walk, modelled on `tclVar.c:TclLookupSimpleVar`
//! (`tmp/tcl9.0.4`) and `namespace-tree.md` §5.3. Given a name and the current
//! `(frame, namespace)` context, decide which table holds it and follow any
//! `global`/`variable`/`upvar` links to the concrete cell:
//!
//! A name is a **namespace variable** (per `TclLookupSimpleVar`) when it is
//! qualified (`::`-containing) **or** there is no active proc frame (the global
//! scope or a `namespace eval` body). Otherwise it is a **frame-local** (proc)
//! variable. So:
//!
//! 1. **Qualified** (`::a::b::x`) → namespace `::a::b` (absolute if `::`-led, else
//!    relative to the current ns), simple tail `x`. The namespace must exist
//!    (writes into a missing one raise *parent namespace doesn't exist*; reads
//!    just miss).
//! 2. **Unqualified, in a proc** → the current frame's local table.
//! 3. **Unqualified, at global / `namespace eval` scope** → the current
//!    namespace's var table (so `set x` and `set ::x` at top level are the
//!    *same* global).
//!
//! All storage releases through [`crate::frame::VarTable`]'s refcount discipline;
//! `global`/`variable`/`upvar` install [`Link`]s ([`make_variable`] /
//! [`make_upvar`]). The coordinator borrows both the frame stack and the
//! namespace tree (the two var-table owners) from the interp.

use tcl_runtime_api::VarId;
use tcl_syntax::naming::{
    variable_namespace_lookup_bases, NativeNameProtocol, NativeNameQualification,
    VariableLookupBase,
};

use crate::frame::{FrameStack, Link, Var, VarError, VarHome, VarTable};
use crate::namespace::{Namespaces, NsId, GLOBAL};
use crate::obj::{self, TclObj};

mod captured_targets;
mod link_installation;
mod named_access;
mod original_element_cells;
mod original_name_cells;
mod trace_homes;

#[cfg(test)]
pub(crate) use captured_targets::original_indexed_link_target;
pub(crate) use captured_targets::{
    array_default_state_at_target, array_element_target_at, array_names_at_target,
    array_search_element_exists_at_target, array_search_keys_at_target,
    array_search_target_current, array_target_at, array_target_is_set,
    capture_get_variable_receiver, capture_original_indexed_receiver, capture_variable_receiver,
    get_element_at_target, release_array_element_target, release_array_target,
    retain_array_element_target, retain_array_target, take_array_default_at_target,
    unset_element_at_target,
};
#[cfg(test)]
pub(crate) use link_installation::{make_upvar, make_upvar_in, make_variable};
pub(crate) use link_installation::{
    publish_upvar, publish_upvar_in, publish_variable_mapped, select_tcloo_compiled_variable,
    PublishedVariableLink,
};
pub(crate) use named_access::{
    capture_static_source, const_names, is_constant, is_constant_at, mark_constant,
    replace_array_default_classified, CapturedByteStore,
};
#[cfg(test)]
pub(crate) use named_access::{get, get_at, get_elem, get_elem_at, is_array, set, set_elem, unset};
#[cfg(test)]
#[cfg(debug_assertions)]
pub(crate) use original_element_cells::{
    capture_original_c_element_materialisation, prepare_original_c_element_cell,
    OriginalElementOptions,
};
pub(crate) use original_element_cells::{
    capture_original_c_selected_element_materialisation, OriginalArrayMemberSelection,
};
pub(crate) use original_name_cells::{
    capture_jim_cached_link, capture_original_c_selected_receiver,
    capture_original_namespace_receiver, capture_variable_store_receiver_at,
    original_c_selected_link_target, original_jim_variable_cell, original_namespace_link_target,
    prepare_original_c_alias_local, prepare_original_c_indexed_name_cell,
    prepare_original_c_variable_cell, read_jim_cached_link, retain_original_jim_variable_key,
    selected_c_link_target, NativeOriginalNameScope, OriginalAliasTargetObservation,
};
pub(crate) use trace_homes::{trace_element_identity, trace_home, TraceHome};

/// Bound on the link walk so a pathological alias cycle can't spin forever
/// (matches the frame model's conservative guard; a real recursion limit lands
/// with the proc chunk).
const LINK_LIMIT: usize = 1000;
mod entered_container;
mod jim_alias_publication;
mod trace_shell;
pub(crate) use entered_container::{capture_byte_store_entered, EnteredVariablePlace};
pub(crate) use jim_alias_publication::{CapturedJimAliasTarget, PreparedJimAliasLocal};
pub(crate) use trace_shell::CapturedTraceShell;

/// A concrete place a name resolves to: a table ([`VarHome`]) + simple name +
/// optional array element. Never itself a link.
struct Place {
    home: VarHome,
    name: Vec<u8>,
    elem: Option<Vec<u8>>,
    array_cell: Option<crate::frame::RetainedArrayCell>,
    scalar_entry: Option<std::rc::Rc<crate::frame::NativeScalarAliasEntry>>,
}

// Retained native links use their actual cell, even when another declaration
// has the same canonical spelling. The table path is used only without a receipt.
fn read_retained(place: &Place) -> Option<Result<Option<*mut TclObj>, VarError>> {
    if let Some(entry) = &place.scalar_entry {
        return Some(
            entry
                .capture_receiver(place.elem.clone(), false)
                .and_then(|receiver| receiver.map_or(Ok(None), |receiver| receiver.read())),
        );
    }
    place
        .array_cell
        .as_ref()
        .map(|array| Ok(place.elem.as_ref().and_then(|key| array.read(key))))
}
fn exists_retained(place: &Place) -> Option<bool> {
    read_retained(place).map(|result| match result {
        Ok(value) => value.is_some(),
        Err(VarError::IsArray) => place.elem.is_none(),
        Err(_) => false,
    })
}
#[cfg(test)]
fn store_retained(place: &Place, value: *mut TclObj) -> Option<Result<(), VarError>> {
    if let Some(entry) = &place.scalar_entry {
        return Some(
            entry
                .capture_receiver(place.elem.clone(), true)
                .and_then(|receiver| {
                    receiver
                        .ok_or(VarError::NameProtocolUnavailable)?
                        .store(value)
                }),
        );
    }
    place
        .array_cell
        .as_ref()
        .map(|array| array.store(place.elem.as_ref().ok_or(VarError::IsScalar)?, value))
}
#[cfg(test)]
fn unset_retained(place: &Place) -> Option<Result<bool, VarError>> {
    if let Some(entry) = &place.scalar_entry {
        return Some(
            entry
                .capture_receiver(place.elem.clone(), false)
                .and_then(|receiver| receiver.map_or(Ok(false), |receiver| receiver.unset())),
        );
    }
    place
        .array_cell
        .as_ref()
        .map(|array| Ok(place.elem.as_ref().is_some_and(|key| array.remove(key))))
}

/// The outcome of classifying + walking a name.
enum Resolved {
    /// A concrete place to read/write.
    Place(Place),
    /// A qualified name whose namespace does not exist. Writes raise
    /// `parent namespace doesn't exist`; reads/unsets treat it as not-found.
    Error(VarError),
}

/// One resolved standalone variable binding selected by stable identity.
///
/// The table slot is deliberately not the identity: compiled slots survive an
/// ordinary `unset`, while a later recreation receives a new identity. During
/// an operation on this exact binding Tcl retains the `Var` cell, however, so
/// an unset-and-recreate inside one callback refills the retained identity.
/// `array get` holds this record across callbacks and compares `id` before
/// observing the table again.
#[derive(Clone, Debug)]
pub(crate) struct ArrayCellTarget {
    table: entered_container::TableObservation,
    id: VarId,
    home: VarHome,
    root: Vec<u8>,
    array_cell: Option<crate::frame::RetainedArrayCell>,
    cell: crate::frame::NativeArrayTargetCell,
}

impl ArrayCellTarget {
    pub(crate) const fn id(&self) -> VarId {
        self.id
    }

    fn selected_trace_home(&self, member: Option<(Vec<u8>, VarId)>) -> TraceHome {
        let (namespace, level) = match self.home {
            VarHome::Namespace(namespace) => (Some(namespace), None),
            VarHome::Frame(level) => (None, Some(level)),
        };
        TraceHome {
            binding_id: Some(self.id),
            selected_member: member,
            ns: namespace,
            level,
            base: self.root.clone(),
            link_elem: None,
        }
    }

    pub(crate) fn original_array_cell(&self) -> Option<&crate::frame::RetainedArrayCell> {
        self.array_cell.as_ref()
    }
}

/// The var home where the current context's *local* (unqualified, non-`::`)
/// names live, and where `global`/`variable`/`upvar` install their links. In a
/// proc that is the frame's table; at global / namespace-eval scope it is the
/// current namespace's table (the global frame and global ns share one table).
fn current_home(frames: &FrameStack, current_ns: NsId) -> VarHome {
    if frames.owns_local_variables_at(frames.current_level()) {
        VarHome::Frame(frames.current_level())
    } else {
        VarHome::Namespace(current_ns)
    }
}

/// The table for `home` (read), or `None` if the level/ns is gone.
fn table<'a>(frames: &'a FrameStack, ns: &'a Namespaces, home: VarHome) -> Option<&'a VarTable> {
    match home {
        VarHome::Frame(level) => frames.table(level),
        VarHome::Namespace(id) => Some(ns.var_table(id)),
    }
}

/// The table for `home` (mutable). The caller has already proven `home` exists.
fn table_mut<'a>(
    frames: &'a mut FrameStack,
    ns: &'a mut Namespaces,
    home: VarHome,
) -> &'a mut VarTable {
    match home {
        VarHome::Frame(level) => frames.table_mut(level).expect("frame level exists"),

        VarHome::Namespace(id) => ns.var_table_mut(id),
    }
}

fn variable_is_qualified(ns: &Namespaces, name: &[u8]) -> bool {
    ns.variable_name_protocol.is_some_and(|protocol| {
        protocol.variable_root_input(name).qualification() != NativeNameQualification::Unqualified
    })
}

/// Project an explicit second name before touching the physical array table.
fn separate_element(ns: &Namespaces, name: &[u8], key: &[u8]) -> Result<Vec<u8>, VarError> {
    let protocol = ns
        .variable_name_protocol
        .ok_or(VarError::NameProtocolUnavailable)?;
    Ok(protocol
        .separate_variable_input(name, Some(key))
        .element()
        .expect("explicit element")
        .selected()
        .to_vec())
}

/// Jim namespace variables occupy its global byte-key table; procedure locals
/// still occupy the selected activation table.
fn unqualified_home(
    ns: &Namespaces,
    home: VarHome,
    name: &[u8],
    protocol: NativeNameProtocol,
) -> (VarHome, Vec<u8>) {
    unqualified_home_using(ns, home, name, protocol, false)
}
fn unqualified_home_using(
    ns: &Namespaces,
    home: VarHome,
    name: &[u8],
    protocol: NativeNameProtocol,
    original: bool,
) -> (VarHome, Vec<u8>) {
    if protocol.is_jim084() {
        if let VarHome::Namespace(namespace) = home {
            let (namespace, key) = ns
                .var_home(namespace, name)
                .expect("Jim byte namespace key");
            return (VarHome::Namespace(namespace), key);
        }
    }
    (
        ns_scope_fallback_using(ns, home, name, original),
        name.to_vec(),
    )
}

fn qualified_home_using(
    ns: &Namespaces,
    context: NsId,
    name: &[u8],
    original: bool,
    qualification: NativeNameQualification,
) -> Option<(NsId, Vec<u8>)> {
    let has_cell = |namespace, key: &[u8]| {
        let table = if original {
            ns.original_variable_table_at(namespace)
        } else {
            Some(ns.var_table(namespace))
        };
        table.is_some_and(|table| table.has_native_name_cell(key))
    };
    let mut bases = variable_namespace_lookup_bases(
        qualification == NativeNameQualification::Absolute,
        context,
        ns.ns_var_global_fallback,
    )
    .peekable();
    let namespace = |base| match base {
        VariableLookupBase::Context(token) => token,
        VariableLookupBase::Root => GLOBAL,
    };
    let primary = ns.var_home(namespace(bases.next()?), name);
    if bases.peek().is_some() && primary.as_ref().is_none_or(|(id, key)| !has_cell(*id, key)) {
        for base in bases {
            if let Some(alternate) = ns.var_home(namespace(base), name) {
                if has_cell(alternate.0, &alternate.1) {
                    return Some(alternate);
                }
            }
        }
    }
    primary
}

/// Classify `name` to its starting `(home, simple-name)`, or an error if it
/// is qualified into a namespace that does not exist.
fn classify(frames: &FrameStack, ns: &Namespaces, current_ns: NsId, name: &[u8]) -> Resolved {
    classify_using(frames, ns, current_ns, name, false)
}
fn classify_using(
    frames: &FrameStack,
    ns: &Namespaces,
    current_ns: NsId,
    name: &[u8],
    original: bool,
) -> Resolved {
    let Some(protocol) = ns.variable_name_protocol else {
        return Resolved::Error(VarError::NameProtocolUnavailable);
    };
    let input = protocol.variable_root_input(name);
    let name = input.selected();
    if let Some((namespace, key)) =
        match frames.tcloo_variable_target(frames.current_level(), protocol, name) {
            Ok(target) => target,
            Err(error) => return Resolved::Error(error),
        }
    {
        return Resolved::Place(Place {
            home: VarHome::Namespace(namespace),
            name: key,
            elem: None,
            array_cell: None,
            scalar_entry: None,
        });
    }
    let (home, key) = if input.qualification() != NativeNameQualification::Unqualified {
        match qualified_home_using(ns, current_ns, name, original, input.qualification()) {
            Some((id, simple)) => (VarHome::Namespace(id), simple),
            None => return Resolved::Error(VarError::NoSuchNamespace),
        }
    } else {
        unqualified_home_using(
            ns,
            current_home(frames, current_ns),
            name,
            protocol,
            original,
        )
    };
    Resolved::Place(Place {
        home,
        name: key,
        elem: None,
        array_cell: None,
        scalar_entry: None,
    })
}

/// Select an existing global cell only when the caller enables fallback.
/// Undefined namespace cells block it; procedure-local homes stay local.
fn ns_scope_fallback_using(ns: &Namespaces, home: VarHome, name: &[u8], original: bool) -> VarHome {
    if !ns.ns_var_global_fallback {
        return home;
    }
    let VarHome::Namespace(id) = home else {
        return home;
    };
    if id == crate::namespace::GLOBAL {
        return home;
    }
    let has_cell = |namespace| {
        let table = if original {
            ns.original_variable_table_at(namespace)
        } else {
            Some(ns.var_table(namespace))
        };
        table.is_some_and(|table| table.has_native_name_cell(name))
    };
    for base in variable_namespace_lookup_bases(false, id, ns.ns_var_global_fallback) {
        let namespace = match base {
            VariableLookupBase::Context(token) => token,
            VariableLookupBase::Root => GLOBAL,
        };
        if has_cell(namespace) {
            return VarHome::Namespace(namespace);
        }
    }
    home
}

/// Follow `global`/`variable`/`upvar` links from `place` to the concrete cell.
fn follow_links(frames: &FrameStack, ns: &Namespaces, place: Place) -> Result<Place, VarError> {
    follow_selected_links(frames, ns, place, false)
}

fn follow_selected_links(
    frames: &FrameStack,
    ns: &Namespaces,
    place: Place,
    trace_registration: bool,
) -> Result<Place, VarError> {
    follow_selected_links_observed(frames, ns, place, trace_registration, None)
}

fn follow_selected_links_observed(
    frames: &FrameStack,
    ns: &Namespaces,
    mut place: Place,
    trace_registration: bool,
    mut observed: Option<&mut Vec<OriginalCLinkSelection>>,
) -> Result<Place, VarError> {
    for _ in 0..LINK_LIMIT {
        if place.array_cell.is_some() || place.scalar_entry.is_some() {
            break;
        }
        let Some(link) = selected_link(frames, ns, &place) else {
            break;
        };
        let self_link = link.home == place.home && link.name == place.name && link.elem.is_none();
        if !self_link {
            if let Some(observed) = observed.as_deref_mut() {
                let selected =
                    table(frames, ns, place.home).ok_or(VarError::NameProtocolUnavailable)?;
                observed.push(OriginalCLinkSelection {
                    home: place.home,
                    table: entered_container::TableObservation::capture(frames, ns, place.home)
                        .ok_or(VarError::NameProtocolUnavailable)?,
                    link: selected.observe_original_c_link(&place.name)?,
                });
            }
        }
        let next = follow_selected_link(
            frames,
            ns,
            Place {
                home: place.home,
                name: place.name.clone(),
                elem: place.elem.clone(),
                array_cell: place.array_cell.clone(),
                scalar_entry: place.scalar_entry.clone(),
            },
            link,
            trace_registration,
        )?;
        if self_link {
            break;
        }
        place = next.expect("non-self link advances its original place");
    }
    Ok(place)
}
fn selected_link(frames: &FrameStack, ns: &Namespaces, place: &Place) -> Option<Link> {
    selected_link_using(frames, ns, place, false)
}
fn selected_table<'a>(
    frames: &'a FrameStack,
    ns: &'a Namespaces,
    home: VarHome,
    original: bool,
) -> Option<&'a VarTable> {
    if !original {
        return table(frames, ns, home);
    }
    match home {
        VarHome::Frame(level) => frames.original_variable_table_at(level),
        VarHome::Namespace(namespace) => ns.original_variable_table_at(namespace),
    }
}
fn selected_link_using(
    frames: &FrameStack,
    ns: &Namespaces,
    place: &Place,
    original: bool,
) -> Option<Link> {
    match selected_table(frames, ns, place.home, original)
        .and_then(|table| table.cell(&place.name))
        .as_deref()
    {
        Some(Var::Link(link)) => Some(link.clone()),
        _ => None,
    }
}
fn follow_selected_link(
    frames: &FrameStack,
    ns: &Namespaces,
    place: Place,
    link: Link,
    trace_registration: bool,
) -> Result<Option<Place>, VarError> {
    follow_selected_link_using(frames, ns, place, link, trace_registration, false)
}
fn follow_selected_link_using(
    frames: &FrameStack,
    ns: &Namespaces,
    mut place: Place,
    link: Link,
    trace_registration: bool,
    original: bool,
) -> Result<Option<Place>, VarError> {
    if ns.variable_link_binding == tcl_dialect::VariableLinkBinding::StableCell
        && matches!(link.home, VarHome::Namespace(namespace) if ns.namespace_variables_are_deleted(namespace))
    {
        return Err(VarError::DeletedNamespace);
    }
    if link.array_cell.as_ref().is_some_and(|cell| !cell.is_live()) {
        return Err(VarError::DeletedArray);
    }
    if link.native_element_entry.as_ref().is_some_and(|entry| {
        if trace_registration {
            !entry.permits_trace_registration()
        } else {
            !entry.is_live()
        }
    }) {
        return Err(VarError::DeletedArray);
    }
    if link.array_cell.is_none()
        && link.array_identity.is_some_and(|identity| {
            selected_table(frames, ns, link.home, original)
                .and_then(|table| table.binding_id(&link.name))
                != Some(identity)
        })
    {
        return Err(VarError::DeletedArray);
    }
    // A self-link is the declared-but-undefined marker — stop here; the
    // cell reads as missing.
    if link.home == place.home && link.name == place.name && link.elem.is_none() {
        return Ok(None);
    }
    // An element-on-element chain (`a(b)(c)`) is invalid; keep the outer
    // element and stop chaining elements (mirrors the frame model).
    let elem = match (place.elem.take(), link.elem) {
        (None, e) => e,
        (outer @ Some(_), _) => outer,
    };
    Ok(Some(Place {
        home: link.home,
        name: link.name,
        elem,
        array_cell: link.array_cell,
        scalar_entry: link.native_scalar_entry,
    }))
}

/// Whether installing `local` in the current context would make a namespace
/// variable point into a procedure frame. C rejects this inverted `upvar`
/// because the procedure cell can disappear before the namespace cell.
///
/// Follow an existing target link first: a proc-local `global`/`variable`
/// alias ultimately has namespace lifetime and is therefore safe.
pub(crate) fn upvar_would_invert(
    frames: &FrameStack,
    ns: &Namespaces,
    current_ns: NsId,
    target: &Link,
    local: &[u8],
) -> bool {
    let local_is_namespace = variable_is_qualified(ns, local)
        || matches!(current_home(frames, current_ns), VarHome::Namespace(_));
    if !local_is_namespace {
        return false;
    }
    let target = follow_links(
        frames,
        ns,
        Place {
            home: target.home,
            name: target.name.clone(),
            elem: target.elem.clone(),
            array_cell: target.array_cell.clone(),
            scalar_entry: target.native_scalar_entry.clone(),
        },
    );
    target.is_ok_and(|target| matches!(target.home, VarHome::Frame(level) if frames.owns_local_variables_at(level)))
}

/// Classify then follow `global`/`variable`/`upvar` links to the concrete cell.
fn resolve(frames: &FrameStack, ns: &Namespaces, current_ns: NsId, name: &[u8]) -> Resolved {
    match classify(frames, ns, current_ns, name) {
        Resolved::Place(p) => {
            follow_links(frames, ns, p).map_or_else(Resolved::Error, Resolved::Place)
        }
        other => other,
    }
}

/// Trace registration can reach the exact entry held by its active unset
/// callback. That receipt grants no read/write access to a retired member.
fn resolve_trace_registration(
    frames: &FrameStack,
    ns: &Namespaces,
    current_ns: NsId,
    name: &[u8],
) -> Resolved {
    match classify(frames, ns, current_ns, name) {
        Resolved::Place(place) => follow_selected_links(frames, ns, place, true)
            .map_or_else(Resolved::Error, Resolved::Place),
        other => other,
    }
}

/// Whether a store to `name` resolved from the current context lands in the
/// running procedure's own frame: a local of it, reached by name or through a
/// link that stays within it. A name at the global level, a qualified or
/// namespace name, and a local linked to a variable outside the frame all land
/// elsewhere.
pub(crate) fn lands_in_own_frame(
    frames: &FrameStack,
    ns: &Namespaces,
    current_ns: NsId,
    name: &[u8],
) -> bool {
    frames.in_proc()
        && matches!(
            resolve(frames, ns, current_ns, name),
            Resolved::Place(place) if place.home == VarHome::Frame(frames.current_level())
        )
}

/// [`lands_in_own_frame`] for a store resolved as if `level` were active: only
/// the running procedure's own level can be its frame.
pub(crate) fn lands_in_own_frame_at(
    frames: &FrameStack,
    ns: &Namespaces,
    name: &[u8],
    level: usize,
) -> bool {
    level == frames.current_level()
        && frames.is_proc_at(level)
        && matches!(
            resolve_at(frames, ns, name, level),
            Resolved::Place(place) if place.home == VarHome::Frame(level)
        )
}

/// The var home an unqualified name lives in when resolving against frame
/// `level` — the frame-addressed analogue of [`current_home`]: a proc frame's
/// own table, else that frame's namespace table (the global level → the global
/// namespace). Also the `Frames::link` target home (`state_traits.rs`).
pub(crate) fn home_at(frames: &FrameStack, level: usize) -> VarHome {
    if frames.owns_local_variables_at(level) {
        VarHome::Frame(level)
    } else {
        VarHome::Namespace(frames.frame_ns(level))
    }
}

/// Frame-addressed [`classify`]: an unqualified name is a local of frame `level`
/// (or, for a non-proc frame, its namespace); a qualified name resolves in that
/// frame's namespace context.
fn classify_at(frames: &FrameStack, ns: &Namespaces, name: &[u8], level: usize) -> Resolved {
    classify_at_using(frames, ns, name, level, false)
}
fn classify_at_using(
    frames: &FrameStack,
    ns: &Namespaces,
    name: &[u8],
    level: usize,
    original: bool,
) -> Resolved {
    let Some(protocol) = ns.variable_name_protocol else {
        return Resolved::Error(VarError::NameProtocolUnavailable);
    };
    let input = protocol.variable_root_input(name);
    let name = input.selected();
    if let Some((namespace, key)) = match frames.tcloo_variable_target(level, protocol, name) {
        Ok(target) => target,
        Err(error) => return Resolved::Error(error),
    } {
        return Resolved::Place(Place {
            home: VarHome::Namespace(namespace),
            name: key,
            elem: None,
            array_cell: None,
            scalar_entry: None,
        });
    }
    let (home, key) = if input.qualification() != NativeNameQualification::Unqualified {
        match qualified_home_using(
            ns,
            frames.frame_ns(level),
            name,
            original,
            input.qualification(),
        ) {
            Some((id, simple)) => (VarHome::Namespace(id), simple),
            None => return Resolved::Error(VarError::NoSuchNamespace),
        }
    } else {
        unqualified_home_using(ns, home_at(frames, level), name, protocol, original)
    };
    Resolved::Place(Place {
        home,
        name: key,
        elem: None,
        array_cell: None,
        scalar_entry: None,
    })
}

/// Frame-addressed [`resolve`]: classify `name` as if `level` were the active
/// frame (`FrameId`-addressed access), then follow links.
fn resolve_at(frames: &FrameStack, ns: &Namespaces, name: &[u8], level: usize) -> Resolved {
    match classify_at(frames, ns, name, level) {
        Resolved::Place(p) => {
            follow_links(frames, ns, p).map_or_else(Resolved::Error, Resolved::Place)
        }
        other => other,
    }
}

/// Resolve an alias target in the selected activation using ordinary variable
/// lookup, including namespace fallback and links already installed there.
pub(crate) fn link_target_at(
    frames: &FrameStack,
    ns: &Namespaces,
    name: &[u8],
    elem: Option<Vec<u8>>,
    level: usize,
) -> Option<Link> {
    let Resolved::Place(mut place) = classify_at(frames, ns, name, level) else {
        return None;
    };
    place.elem = match elem {
        Some(element) => Some(separate_element(ns, name, &element).ok()?.to_vec()),
        None => None,
    };
    let place = follow_links(frames, ns, place).ok()?;
    let array_identity = place
        .array_cell
        .as_ref()
        .map(crate::frame::RetainedArrayCell::identity);
    Some(Link {
        original_jim_target: None,
        native_scalar_entry: place.scalar_entry.as_ref().map(|entry| entry.new_binding()),
        native_element_entry: None,
        array_identity,
        array_cell: place.array_cell,
        home: place.home,
        name: place.name,
        elem: place.elem,
    })
}

/// An element alias captures its containing array in C Tcl. Jim instead
/// looks the array up by name on each access, including after root recreation.
pub(crate) fn prepare_upvar_target(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    target: &mut Link,
) -> Result<(), VarError> {
    prepare_upvar_target_using(frames, ns, target, true)
}
pub(crate) fn prepare_selected_upvar_target(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    target: &mut Link,
) -> Result<(), VarError> {
    prepare_upvar_target_using(frames, ns, target, false)
}
fn prepare_upvar_target_using(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    target: &mut Link,
    acquire: bool,
) -> Result<(), VarError> {
    if target.home == VarHome::Frame(0) {
        target.home = VarHome::Namespace(GLOBAL);
    }
    if ns.variable_link_binding != tcl_dialect::VariableLinkBinding::StableCell {
        return Ok(());
    }
    // An unresolved namespace declaration can already be an alias. Follow its
    // actual link before retaining a cell: retaining the intermediate alias
    // would make later physical reads inspect Var::Link as if it were a scalar.
    // Targets that already retain a physical owner keep that exact generation.
    if target.native_scalar_entry.is_none()
        && target.native_element_entry.is_none()
        && target.array_cell.is_none()
        && target.array_identity.is_none()
    {
        let selected = follow_links(
            frames,
            ns,
            Place {
                home: target.home,
                name: target.name.clone(),
                elem: target.elem.clone(),
                array_cell: None,
                scalar_entry: None,
            },
        )?;
        target.home = selected.home;
        target.name = selected.name;
        target.elem = selected.elem;
        target.array_identity = selected
            .array_cell
            .as_ref()
            .map(crate::frame::RetainedArrayCell::identity);
        target.array_cell = selected.array_cell;
        target.native_scalar_entry = selected
            .scalar_entry
            .map(|entry| {
                if acquire {
                    entry.try_new_binding()
                } else {
                    entry.select_new_binding()
                }
            })
            .transpose()
            .map_err(|_| VarError::NameProtocolUnavailable)?;
    }
    let target_table = table_mut(frames, ns, target.home);
    if target.elem.is_none() {
        if target.native_scalar_entry.is_none() {
            target.native_scalar_entry = Some(if acquire {
                target_table.retain_native_scalar_alias(&target.name)
            } else {
                target_table.select_native_scalar_alias(&target.name)
            });
        }
        return Ok(());
    }
    if target.array_identity.is_none() {
        if let Some(entry) = &target.native_scalar_entry {
            let receiver = entry
                .capture_selected_receiver(target.elem.clone(), true)?
                .ok_or(VarError::NameProtocolUnavailable)?;
            target.array_cell = receiver.selected_array();
            target.array_identity = target
                .array_cell
                .as_ref()
                .map(crate::frame::RetainedArrayCell::identity);
        } else {
            target_table.ensure_array(&target.name)?;
            target.array_identity = target_table.binding_id(&target.name);
            target.array_cell = target_table.capture_array_cell(&target.name);
        }
        target.native_scalar_entry.take();
    }
    if target.native_element_entry.is_none() {
        target.native_element_entry = target.array_cell.as_ref().and_then(|array| {
            array.select_element_alias(target.elem.as_deref().expect("element alias"), acquire)
        });
        if target.native_element_entry.is_none() {
            return Err(VarError::DeletedArray);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;

/// Original C simple-name lookup, before link traversal or guest observers.
/// A hash key is adopted by its actual entry at birth; compiled cells own none.
#[derive(Clone)]
pub(crate) struct NativeOriginalNameCell {
    pub compiled: Option<usize>,
    home: VarHome,
    name: Vec<u8>,
    table: entered_container::TableObservation,
    cell: crate::frame::VariablePublicationObservation,
    link: Option<crate::frame::OriginalVariableLinkObservation>,
}
#[cfg(test)]
static ORIGINAL_SIMPLE_CELL_DIAGNOSTIC: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

impl NativeOriginalNameCell {
    fn capture(
        frames: &FrameStack,
        ns: &Namespaces,
        home: VarHome,
        name: Vec<u8>,
        compiled: Option<usize>,
        cell: crate::frame::VariablePublicationObservation,
        mut link: Option<crate::frame::OriginalVariableLinkObservation>,
    ) -> Result<Self, VarError> {
        // The declared-but-undefined self marker is not a traversed alias. Its
        // own selected array conversion may legitimately replace that marker.
        if link
            .as_ref()
            .is_some_and(|link| link.targets_itself(home, &name))
        {
            link = None;
        }
        Ok(Self {
            compiled,
            home,
            name,
            table: entered_container::TableObservation::capture(frames, ns, home)
                .ok_or(VarError::NameProtocolUnavailable)?,
            cell,
            link,
        })
    }
    pub(crate) fn check(
        &self,
        interpreter: &crate::interp::Interp,
        currency: &crate::interp::native_operation_currency::NativeOperationCurrency,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        currency
            .ensure_current_or_refuse()
            .map_err(tcl_cmd_core::CmdError::from_execution_refusal)?;
        if !crate::VERIFY_CURRENCY {
            return Ok(());
        }
        let physical =
            entered_container::same_table(interpreter, self.home, &self.table).and_then(|()| {
                let frames = interpreter.frames.borrow();
                let ns = interpreter.namespaces();
                self.check_selected(&frames, &ns)
            });
        physical.map_err(|error| {
            tcl_cmd_core::CmdError::from_execution_refusal(
                interpreter.clone().refuse_completion_value_access(error),
            )
        })?;
        currency
            .ensure_current_or_refuse()
            .map_err(tcl_cmd_core::CmdError::from_execution_refusal)
    }
    fn check_selected(
        &self,
        frames: &FrameStack,
        ns: &Namespaces,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        let same = match (self.home, self.compiled) {
            (VarHome::Frame(level), Some(index)) => {
                frames.original_compiled_cell_at_is_current(level, index, &self.cell)
            }
            (_, None) => table(frames, ns, self.home)
                .is_some_and(|table| self.cell.matches_root(table, &self.name)),
            _ => false,
        };
        if !same {
            #[cfg(test)]
            if std::env::var_os("TCL_LSP_RUNTIME_BASELINE_DIAGNOSTIC").as_deref()
                == Some(std::ffi::OsStr::new("1"))
                && !ORIGINAL_SIMPLE_CELL_DIAGNOSTIC.swap(true, std::sync::atomic::Ordering::Relaxed)
            {
                eprintln!(
                    "ORIGINAL_SIMPLE_CELL_DIAGNOSTIC home={:?} compiled={:?} link_present={} root_matches={} name_prefix={:?}",
                    self.home,
                    self.compiled,
                    self.link.is_some(),
                    same,
                    &self.name[..self.name.len().min(256)],
                );
            }
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original simple variable cell",
            ));
        }
        self.cell.check_current()?;
        if let Some(link) = &self.link {
            link.check_current()?;
        }
        Ok(())
    }
    fn place(&self) -> Place {
        Place {
            home: self.home,
            name: self.name.clone(),
            elem: None,
            array_cell: None,
            scalar_entry: None,
        }
    }
}

struct OriginalCLinkSelection {
    home: VarHome,
    table: entered_container::TableObservation,
    link: crate::frame::OriginalVariableLinkObservation,
}
impl OriginalCLinkSelection {
    fn check(
        &self,
        interpreter: &crate::interp::Interp,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        entered_container::same_table(interpreter, self.home, &self.table)?;
        self.link.check_current()
    }
}
