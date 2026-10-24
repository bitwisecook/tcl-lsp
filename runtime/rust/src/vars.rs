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

use tcl_runtime_api::{FrameLinkOrigin, VarId};
use tcl_syntax::naming::{
    variable_namespace_lookup_bases, NativeNameProtocol, NativeNameQualification,
    VariableLookupBase,
};

use crate::frame::{FrameStack, Link, Var, VarError, VarHome, VarTable};
use crate::namespace::{Namespaces, NsId, GLOBAL};
use crate::obj::{self, TclObj};

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
fn original_indexed_link_target_using(
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

/// The identity a variable trace on `name` belongs to, following
/// `global`/`variable`/`upvar` links to the concrete cell.
///
/// A trace must be keyed by the variable it resolves to, not by the spelling
/// used to register it: C Tcl hangs the trace off the `Var` struct
/// (`tclTrace.c`'s `TraceVarProc`), so `trace add variable ::v write …` fires
/// for a later `set v X` in the global namespace and — under the 8.x
/// namespace-scope fallback — for a `set v X` inside `namespace eval` that
/// reaches the same global. Reading the frame level and the element off the
/// resolved place is what extends that to an `upvar` alias, whose home
/// frame and array element the access spelling cannot show. Registration
/// and firing share this one `resolve` call, including the dialect-gated
/// fallback.
#[derive(Clone)]
pub(crate) struct TraceHome {
    /// Actual root incarnation, distinct from a recreated same-named array.
    pub(crate) binding_id: Option<tcl_runtime_api::VarId>,
    /// Original member selected before callbacks or through a retained alias.
    pub(crate) selected_member: Option<(Vec<u8>, tcl_runtime_api::VarId)>,
    /// The home namespace, for a cell that lives in one.
    pub(crate) ns: Option<NsId>,
    /// The home call-frame level, for a proc-local cell.
    pub(crate) level: Option<usize>,
    /// The simple (unqualified) name the cell is filed under at its home.
    pub(crate) base: Vec<u8>,
    /// The array element the resolution ended on, when the access reached the
    /// cell through a link that names one (`upvar #0 a(k) e`). `None` for every
    /// direct access — an explicit `a(k)` spelling is split by the caller and
    /// never appears here.
    pub(crate) link_elem: Option<Vec<u8>>,
}

impl TraceHome {
    /// Project the old member table during staged array destruction.
    pub(crate) fn for_selected_array_member(
        &self,
        array: &crate::frame::RetainedArrayCell,
        element: &[u8],
    ) -> Option<Self> {
        let identity = self.binding_id?;
        if array.identity() != identity {
            return None;
        }
        Some(Self {
            binding_id: Some(identity),
            selected_member: Some((element.to_vec(), array.trace_member_identity(element)?)),
            ns: self.ns,
            level: self.level,
            base: self.base.clone(),
            link_elem: self.link_elem.clone(),
        })
    }

    /// Project a member from the captured original root without resolving its
    /// name again. An unrelated or unretained receiver supplies no trace home.
    pub(crate) fn for_selected_receiver(
        &self,
        receiver: &crate::frame::VariableReceiver,
    ) -> Option<Self> {
        let identity = self.binding_id?;
        if receiver.binding_id() != Some(identity) {
            return None;
        }
        Some(Self {
            binding_id: Some(identity),
            selected_member: receiver.trace_member(),
            ns: self.ns,
            level: self.level,
            base: self.base.clone(),
            link_elem: self.link_elem.clone(),
        })
    }
}

pub(crate) fn trace_home(
    frames: &FrameStack,
    ns: &Namespaces,
    current_ns: NsId,
    name: &[u8],
) -> TraceHome {
    trace_home_from_resolved(
        frames,
        ns,
        resolve_trace_registration(frames, ns, current_ns, name),
        name,
    )
}

fn trace_home_from_resolved(
    frames: &FrameStack,
    ns: &Namespaces,
    resolved: Resolved,
    name: &[u8],
) -> TraceHome {
    match resolved {
        Resolved::Place(p) => {
            let (home_ns, level) = match p.home {
                VarHome::Namespace(id) => (Some(id), None),
                VarHome::Frame(level) => (None, Some(level)),
            };
            let selected_member = p.elem.as_ref().and_then(|element| {
                let identity = if let Some(array) = &p.array_cell {
                    array.trace_member_identity(element)
                } else {
                    let table = table(frames, ns, p.home)?;
                    table.native_trace_element_identity(table.binding_id(&p.name)?, element)
                }?;
                Some((element.clone(), identity))
            });
            TraceHome {
                selected_member,
                binding_id: p
                    .array_cell
                    .as_ref()
                    .map(crate::frame::RetainedArrayCell::identity)
                    .or_else(|| p.scalar_entry.as_ref().map(|entry| entry.identity()))
                    .or_else(|| {
                        table(frames, ns, p.home)
                            .and_then(|table| table.trace_binding_identity(&p.name))
                    }),
                ns: home_ns,
                level,
                base: p.name,
                link_elem: p.elem,
            }
        }
        // Unresolvable (a qualified name into a missing namespace): keep the
        // caller's spelling so `trace info` still round-trips it.
        Resolved::Error(_) => TraceHome {
            binding_id: None,
            selected_member: None,
            ns: None,
            level: None,
            base: name.to_vec(),
            link_elem: None,
        },
    }
}

/// Actual member allocation at an independently resolved root home.
/// This read-only trace guard query creates no entry and resolves no name.
pub(crate) fn trace_element_identity(
    frames: &FrameStack,
    ns: &Namespaces,
    home: &TraceHome,
    element: &[u8],
) -> Option<VarId> {
    if let Some((selected, identity)) = &home.selected_member {
        if selected.as_slice() == element {
            return Some(*identity);
        }
    }
    let selected = match (home.ns, home.level) {
        (Some(namespace), _) => VarHome::Namespace(namespace),
        (_, Some(level)) => VarHome::Frame(level),
        _ => return None,
    };
    table(frames, ns, selected)?.native_trace_element_identity(home.binding_id?, element)
}

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

// link installation (global / variable / upvar)

/// Install a link from the current context's `local` name to `target`, unless it
/// would be a self-link (already that exact cell — the no-op `global`/`variable`
/// at namespace scope produces).
/// An adopted generic link and only its actual displaced owning parts.
/// The caller finishes it after frame and namespace borrows end.
pub(crate) struct PublishedVariableLink {
    retired: crate::frame::RetiredVariableParts,
    local: Option<NativeOriginalNameCell>,
    target: Option<OriginalAliasTargetObservation>,
    link: Option<Link>,
    error: Option<VarError>,
}
impl PublishedVariableLink {
    fn empty() -> Self {
        Self {
            retired: Default::default(),
            local: None,
            target: None,
            link: None,
            error: None,
        }
    }
    fn declined(link: Link, error: VarError) -> Self {
        Self {
            retired: crate::frame::RetiredVariableParts::from_original_link(link),
            error: Some(error),
            ..Self::empty()
        }
    }
    fn check(
        &self,
        interpreter: &crate::interp::Interp,
        currency: &crate::interp::native_operation_currency::NativeOperationCurrency,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        currency.ensure_current_or_refuse()?;
        if let Some(local) = &self.local {
            local.check(interpreter, currency)?;
        }
        if let (Some(target), Some(link)) = (&self.target, &self.link) {
            target.check(interpreter, link, currency)?;
        }
        if let Some(link) = &self.link {
            link.check_stored_roles().map_err(|error| {
                tcl_cmd_core::CmdError::from_execution_refusal(
                    interpreter.clone().refuse_completion_value_access(error),
                )
            })?;
        }
        currency.ensure_current_or_refuse().map_err(Into::into)
    }
    pub(crate) fn finish(
        mut self,
        interpreter: &crate::interp::Interp,
        currency: &crate::interp::native_operation_currency::NativeOperationCurrency,
    ) -> Result<(), crate::frame::JimLinkStoreError> {
        let mut initial = self.check(interpreter, currency);
        // An unavailable actual role remains a structured cause before cleanup.
        if self.error == Some(VarError::NameProtocolUnavailable) && initial.is_ok() {
            initial = Err(tcl_cmd_core::CmdError::from_execution_refusal(
                interpreter.clone().refuse_completion_value_access(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "original generic alias publication",
                    ),
                ),
            ));
        }
        let retired = std::mem::take(&mut self.retired);
        let cleanup = retired.finish(|| self.check(interpreter, currency));
        initial.map_err(crate::frame::JimLinkStoreError::Command)?;
        cleanup.map_err(crate::frame::JimLinkStoreError::Command)?;
        if let Some(error) = self.error.take() {
            return Err(error.into());
        }
        Ok(())
    }
}

fn link_local_with_origin(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    current_ns: NsId,
    local: &[u8],
    target: Link,
    origin: FrameLinkOrigin,
) -> PublishedVariableLink {
    let Some(protocol) = ns.variable_name_protocol else {
        return PublishedVariableLink::declined(target, VarError::NameProtocolUnavailable);
    };
    let input = protocol.variable_root_input(local);
    let (here, local) = unqualified_home(
        ns,
        current_home(frames, current_ns),
        input.selected(),
        protocol,
    );
    if target.home == here && target.elem.is_none() && target.name == local {
        return PublishedVariableLink {
            retired: crate::frame::RetiredVariableParts::from_original_link(target),
            ..PublishedVariableLink::empty()
        };
    }
    publish_link_at(frames, ns, here, &local, target, origin)
}
fn publish_link_at(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    here: VarHome,
    local: &[u8],
    target: Link,
    origin: FrameLinkOrigin,
) -> PublishedVariableLink {
    let observed_target =
        if target.native_scalar_entry.is_some() || target.native_element_entry.is_some() {
            match OriginalAliasTargetObservation::capture(frames, ns, &target) {
                Ok(target) => Some(target),
                Err(error) => return PublishedVariableLink::declined(target, error),
            }
        } else {
            None
        };
    let selected = target.clone(); // allocation/role metadata only
    let retired = table_mut(frames, ns, here).publish_link_with_origin(local, target, origin);
    let observed_local = table(frames, ns, here)
        .ok_or(VarError::NameProtocolUnavailable)
        .and_then(|table| table.observe_original_name_cell(local))
        .and_then(|(cell, link)| {
            NativeOriginalNameCell::capture(frames, ns, here, local.to_vec(), None, cell, link)
        });
    match observed_local {
        Ok(local) => PublishedVariableLink {
            retired,
            local: Some(local),
            target: observed_target,
            link: Some(selected),
            error: None,
        },
        Err(error) => PublishedVariableLink {
            retired,
            target: observed_target,
            link: Some(selected),
            error: Some(error),
            ..PublishedVariableLink::empty()
        },
    }
}

#[cfg(test)]
pub(crate) fn make_variable(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    current_ns: NsId,
    target_ns: NsId,
    tail: &[u8],
) {
    drop(publish_variable_mapped(
        frames, ns, current_ns, target_ns, tail, tail,
    ));
}
pub(crate) fn publish_variable_mapped(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    current_ns: NsId,
    target_ns: NsId,
    local: &[u8],
    name: &[u8],
) -> PublishedVariableLink {
    let mut target = declared_namespace_variable(ns, target_ns, name);
    // Check the existing no-op before acquiring a genuine new alias reference.
    if let Some(protocol) = ns.variable_name_protocol {
        let input = protocol.variable_root_input(local);
        let (here, local) = unqualified_home(
            ns,
            current_home(frames, current_ns),
            input.selected(),
            protocol,
        );
        if target.home == here && target.name == local {
            return PublishedVariableLink::empty();
        }
    }
    if let Err(error) = prepare_upvar_target(frames, ns, &mut target) {
        return PublishedVariableLink::declined(target, error);
    }
    link_local_with_origin(
        frames,
        ns,
        current_ns,
        local,
        target,
        FrameLinkOrigin::Ordinary,
    )
}

/// Select the actual compiled resolver's declared namespace receiver without
/// taking its new alias role before the selected old-target retirement.
pub(crate) fn select_tcloo_compiled_variable(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    target_ns: NsId,
    name: &[u8],
) -> Result<(Link, NativeOriginalNameCell), crate::frame::JimLinkStoreError> {
    let mut target = declared_namespace_variable(ns, target_ns, name);
    let (cell, link) = ns.var_table(target_ns).observe_original_name_cell(name)?;
    let source = NativeOriginalNameCell::capture(
        frames,
        ns,
        VarHome::Namespace(target_ns),
        name.to_vec(),
        None,
        cell,
        link,
    )?;
    source.check_selected(frames, ns)?;
    prepare_selected_upvar_target(frames, ns, &mut target)?;
    Ok((target, source))
}

fn declared_namespace_variable(ns: &mut Namespaces, target_ns: NsId, name: &[u8]) -> Link {
    if ns.var_table(target_ns).cell(name).is_none() {
        ns.var_table_mut(target_ns).insert_link(
            name,
            Link {
                original_jim_target: None,
                native_scalar_entry: None,
                native_element_entry: None,
                array_identity: None,
                array_cell: None,
                home: VarHome::Namespace(target_ns),
                name: name.to_vec(),
                elem: None,
            },
        );
    }
    ns.var_table_mut(target_ns)
        .mark_undefined_root(name, VarHome::Namespace(target_ns));
    ns.var_table_mut(target_ns).mark_namespace_declared(name);
    Link {
        original_jim_target: None,
        native_scalar_entry: None,
        native_element_entry: None,
        array_identity: None,
        array_cell: None,
        home: VarHome::Namespace(target_ns),
        name: name.to_vec(),
        elem: None,
    }
}

#[cfg(test)]
pub(crate) fn make_upvar(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    current_ns: NsId,
    target: Link,
    local: &[u8],
) {
    drop(publish_upvar(frames, ns, current_ns, target, local));
}
pub(crate) fn publish_upvar(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    current_ns: NsId,
    mut target: Link,
    local: &[u8],
) -> PublishedVariableLink {
    if let Err(error) = prepare_upvar_target(frames, ns, &mut target) {
        return PublishedVariableLink::declined(target, error);
    }
    if target.home == VarHome::Frame(0) {
        target.home = VarHome::Namespace(GLOBAL);
    }
    link_local_with_origin(
        frames,
        ns,
        current_ns,
        local,
        target,
        FrameLinkOrigin::Ordinary,
    )
}
#[cfg(test)]
pub(crate) fn make_upvar_in(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    home_ns: NsId,
    tail: &[u8],
    target: Link,
) {
    drop(publish_upvar_in(frames, ns, home_ns, tail, target));
}
pub(crate) fn publish_upvar_in(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    home_ns: NsId,
    tail: &[u8],
    mut target: Link,
) -> PublishedVariableLink {
    if let Err(error) = prepare_upvar_target(frames, ns, &mut target) {
        return PublishedVariableLink::declined(target, error);
    }
    if target.home == VarHome::Frame(0) {
        target.home = VarHome::Namespace(GLOBAL);
    }
    publish_link_at(
        frames,
        ns,
        VarHome::Namespace(home_ns),
        tail,
        target,
        FrameLinkOrigin::Ordinary,
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::capi::Tcl_NewStringObj;
    use crate::counters;

    #[test]
    fn compiled_alias_observation_does_not_reselect_duplicate_names() {
        let protocol = tcl_syntax::naming::NativeCompiledVariableProtocol::for_native_point(
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0)
                .execution_point()
                .unwrap(),
        )
        .unwrap();
        let mut frames = FrameStack::new();
        let namespaces = Namespaces::new();
        let level = frames.push(crate::namespace::GLOBAL);
        frames.install_formal_cells(&[b"x".to_vec(), b"x".to_vec()], protocol);
        let observed =
            OriginalAliasTargetObservation::capture_compiled(&mut frames, &namespaces, 1).unwrap();
        assert!(frames.original_compiled_cell_at_is_current(level, 1, &observed.cell));
        assert!(!frames.original_compiled_cell_at_is_current(level, 0, &observed.cell));
        frames.pop();
        assert!(!frames.original_compiled_cell_at_is_current(level, 1, &observed.cell));
    }

    fn sobj(s: &[u8]) -> *mut TclObj {
        // fresh_zero string obj; the table takes the owning +1
        unsafe { Tcl_NewStringObj(s.as_ptr() as *const std::ffi::c_char, s.len() as isize) }
    }

    fn as_str(p: Option<*mut TclObj>) -> Option<Vec<u8>> {
        p.map(|obj| unsafe {
            let mut len: obj::TclSize = 0;
            let s = obj::get_string(obj, &mut len);
            std::slice::from_raw_parts(s as *const u8, len as usize).to_vec()
        })
    }

    /// Run `body` against a fresh `(frames, namespaces)` pair, then assert zero
    /// residual once both are dropped — the leak gate.
    fn leak_free(body: impl FnOnce(&mut FrameStack, &mut Namespaces)) {
        counters::reset();
        {
            let mut frames = FrameStack::new();
            let mut ns = Namespaces::new();
            body(&mut frames, &mut ns);
        }
        assert_eq!(
            counters::finalize(),
            0,
            "residual: {} objs, {} bufs",
            counters::live_objs(),
            counters::live_bufs()
        );
        assert_eq!(counters::double_free_count(), 0);
    }

    #[test]
    fn selected_scalar_trace_home_retains_the_actual_undefined_root_binding() {
        // naming.variable.original-array-read-destruction
        // docs/design/analysis/name-resolution-proofs/variable-original-array-read-destruction.md
        // Rust cell correspondence only; native public rows do not observe IDs.
        leak_free(|frames, ns| {
            let selected = capture_variable_receiver(frames, ns, GLOBAL, b"fresh", None).unwrap();
            let home = trace_home(frames, ns, GLOBAL, b"fresh");
            let physical = table(frames, ns, VarHome::Namespace(GLOBAL)).unwrap();
            assert!(physical.binding_id(b"fresh").is_none());
            assert!(!physical.is_set(b"fresh"));
            assert!(selected.read().unwrap().is_none());
            assert_eq!(home.binding_id, selected.binding_id());
            assert!(home.binding_id.is_some());
            assert!(home
                .for_selected_receiver(&selected)
                .unwrap()
                .selected_member
                .is_none());
            let other = capture_variable_receiver(frames, ns, GLOBAL, b"other", None).unwrap();
            assert!(home.for_selected_receiver(&other).is_none());
            let value = crate::obj::Owned::fresh(sobj(b"value"));
            selected.store(value.as_ptr()).unwrap();
            drop(selected);
            unset(frames, ns, GLOBAL, b"fresh");
            let replacement =
                capture_variable_receiver(frames, ns, GLOBAL, b"fresh", None).unwrap();
            assert!(home.for_selected_receiver(&replacement).is_none());
        });
    }

    #[test]
    fn selected_member_trace_home_requires_original_root_correspondence() {
        leak_free(|frames, ns| {
            let value = crate::obj::Owned::fresh(sobj(b"value"));
            set_elem(frames, ns, GLOBAL, b"a", b"k", value.as_ptr()).unwrap();
            set_elem(frames, ns, GLOBAL, b"b", b"k", value.as_ptr()).unwrap();
            let home = trace_home(frames, ns, GLOBAL, b"a");
            let first = capture_variable_receiver(frames, ns, GLOBAL, b"a", Some(b"k")).unwrap();
            let other = capture_variable_receiver(frames, ns, GLOBAL, b"b", Some(b"k")).unwrap();
            let selected = home.for_selected_receiver(&first).unwrap();
            assert_eq!(selected.ns, home.ns);
            assert_eq!(selected.base, home.base);
            assert_eq!(selected.selected_member, first.trace_member());
            assert!(home.for_selected_receiver(&other).is_none());
            drop(first);
            unset(frames, ns, GLOBAL, b"a");
            set_elem(frames, ns, GLOBAL, b"a", b"k", value.as_ptr()).unwrap();
            let replacement =
                capture_variable_receiver(frames, ns, GLOBAL, b"a", Some(b"k")).unwrap();
            assert!(home.for_selected_receiver(&replacement).is_none());
        });
    }

    #[test]
    fn absent_name_recipe_refuses_publication_before_any_cell_is_created() {
        leak_free(|frames, ns| {
            ns.variable_name_protocol = None;
            let value = crate::obj::Owned::fresh(sobj(b"value"));
            assert_eq!(
                set(frames, ns, GLOBAL, b"a", value.as_ptr()),
                Err(VarError::NameProtocolUnavailable)
            );
            assert!(ns.var_table(GLOBAL).cell(b"a").is_none());
        });
    }

    #[test]
    fn followed_alias_keeps_its_selected_byte_key_when_fresh_name_recipe_changes() {
        leak_free(|frames, ns| {
            ns.variable_name_protocol = Some(NativeNameProtocol::for_tcl_version(
                tcl_dialect::TclVersion::V8_5,
            ));
            set(frames, ns, GLOBAL, b"target\0tail", sobj(b"retained")).unwrap();
            frames.push(GLOBAL);
            make_upvar(
                frames,
                ns,
                GLOBAL,
                Link {
                    original_jim_target: None,
                    native_scalar_entry: None,
                    native_element_entry: None,
                    home: VarHome::Namespace(GLOBAL),
                    name: b"target\0tail".to_vec(),
                    elem: None,
                    array_identity: None,
                    array_cell: None,
                },
                b"alias",
            );
            ns.variable_name_protocol = Some(NativeNameProtocol::for_tcl_version(
                tcl_dialect::TclVersion::V8_4,
            ));
            assert_eq!(
                as_str(get(frames, ns, GLOBAL, b"alias")),
                Some(b"retained".to_vec())
            );
            assert_eq!(get(frames, ns, GLOBAL, b"::target\0tail"), None);
        });
    }

    #[test]
    fn stable_alias_preparation_follows_existing_namespace_alias_before_capture() {
        // naming.tcloo.original-constant-link-introspection
        // docs/design/analysis/name-resolution-proofs/tcloo-original-constant-link-introspection.md
        // Cell correspondence control for the shared preparation owner. Actual
        // TclOO public results are tested independently in cmd_info; this test
        // does not attribute Rust cell identities to the native observation.
        leak_free(|frames, ns| {
            set(frames, ns, GLOBAL, b"G", sobj(b"9")).unwrap();
            make_upvar_in(
                frames,
                ns,
                GLOBAL,
                b"X",
                Link {
                    original_jim_target: None,
                    native_scalar_entry: None,
                    native_element_entry: None,
                    home: VarHome::Namespace(GLOBAL),
                    name: b"G".to_vec(),
                    elem: None,
                    array_identity: None,
                    array_cell: None,
                },
            );
            frames.push(GLOBAL);
            make_variable(frames, ns, GLOBAL, GLOBAL, b"X");
            let local = frames
                .table(frames.current_level())
                .unwrap()
                .cell(b"X")
                .unwrap();
            let Var::Link(link) = &*local else {
                panic!("actual local alias")
            };
            assert_eq!(link.home, VarHome::Namespace(GLOBAL));
            assert_eq!(link.name, b"G");
            assert!(link.native_scalar_entry.is_some());
            drop(local);
            assert_eq!(as_str(get(frames, ns, GLOBAL, b"X")), Some(b"9".to_vec()));
            set(frames, ns, GLOBAL, b"X", sobj(b"10")).unwrap();
            assert_eq!(
                as_str(get(frames, ns, GLOBAL, b"::G")),
                Some(b"10".to_vec())
            );
        });
    }

    #[test]
    fn global_scalar_set_get_unset() {
        leak_free(|f, ns| {
            set(f, ns, GLOBAL, b"x", sobj(b"hello")).unwrap();
            assert_eq!(as_str(get(f, ns, GLOBAL, b"x")), Some(b"hello".to_vec()));
            set(f, ns, GLOBAL, b"x", sobj(b"world")).unwrap(); // overwrite releases
            assert_eq!(as_str(get(f, ns, GLOBAL, b"x")), Some(b"world".to_vec()));
            assert!(get(f, ns, GLOBAL, b"x").is_some());
            assert!(unset(f, ns, GLOBAL, b"x"));
            assert_eq!(get(f, ns, GLOBAL, b"x"), None);
        });
    }

    #[test]
    fn plain_and_global_qualified_alias_the_same_var() {
        // The headline fix: `::x` and `x` at global scope are one variable.
        leak_free(|f, ns| {
            set(f, ns, GLOBAL, b"pinged", sobj(b"1")).unwrap();
            assert_eq!(as_str(get(f, ns, GLOBAL, b"::pinged")), Some(b"1".to_vec()));
            set(f, ns, GLOBAL, b"::pinged", sobj(b"2")).unwrap();
            assert_eq!(as_str(get(f, ns, GLOBAL, b"pinged")), Some(b"2".to_vec()));
            unset(f, ns, GLOBAL, b"pinged");
        });
    }

    #[test]
    fn qualified_set_into_existing_namespace() {
        leak_free(|f, ns| {
            let a = ns.ensure_namespace(GLOBAL, b"::a");
            set(f, ns, GLOBAL, b"::a::x", sobj(b"5")).unwrap();
            assert_eq!(as_str(get(f, ns, GLOBAL, b"::a::x")), Some(b"5".to_vec()));
            // and it reads as the unqualified `x` *inside* ::a.
            assert_eq!(as_str(get(f, ns, a, b"x")), Some(b"5".to_vec()));
            unset(f, ns, GLOBAL, b"::a::x");
        });
    }

    #[test]
    fn qualified_set_into_missing_namespace_errors() {
        leak_free(|f, ns| {
            // On the error path `set` does not take ownership, so the caller still
            // owns the fresh obj and must free it (the borrowed-on-error contract).
            let rejected = sobj(b"1");
            unsafe { obj::incr_ref_count(rejected) }; // caller owns +1
            assert_eq!(
                set(f, ns, GLOBAL, b"::nosuch::x", rejected),
                Err(VarError::NoSuchNamespace)
            );
            // The caller frees the rejected object because the failed set did
            // not retain it. A read of the same name simply misses.
            unsafe { obj::decr_ref_count(rejected) };
            assert_eq!(get(f, ns, GLOBAL, b"::nosuch::x"), None);
        });
    }

    #[test]
    fn qualified_array_element() {
        leak_free(|f, ns| {
            ns.ensure_namespace(GLOBAL, b"::a");
            set_elem(f, ns, GLOBAL, b"::a::arr", b"k", sobj(b"v")).unwrap();
            assert_eq!(
                as_str(get_elem(f, ns, GLOBAL, b"::a::arr", b"k")),
                Some(b"v".to_vec())
            );
            assert!(is_array(f, ns, GLOBAL, b"::a::arr"));
            unset(f, ns, GLOBAL, b"::a::arr");
        });
    }

    #[test]
    fn upvar_links_local_to_namespace_var() {
        leak_free(|f, ns| {
            let a = ns.ensure_namespace(GLOBAL, b"::a");
            set(f, ns, GLOBAL, b"::a::x", sobj(b"5")).unwrap();
            make_upvar(
                f,
                ns,
                GLOBAL,
                Link {
                    original_jim_target: None,
                    native_scalar_entry: None,
                    native_element_entry: None,
                    array_identity: None,
                    array_cell: None,
                    home: VarHome::Namespace(a),
                    name: b"x".to_vec(),
                    elem: None,
                },
                b"y",
            );
            assert_eq!(as_str(get(f, ns, GLOBAL, b"y")), Some(b"5".to_vec()));
            // write through the link updates the namespace var
            set(f, ns, GLOBAL, b"y", sobj(b"99")).unwrap();
            assert_eq!(as_str(get(f, ns, GLOBAL, b"::a::x")), Some(b"99".to_vec()));
            // `unset y` follows the link and unsets the *target* (C Tcl); the link
            // cell remains but now points at an unset var.
            unset(f, ns, GLOBAL, b"y");
            assert_eq!(get(f, ns, GLOBAL, b"::a::x"), None);
            assert_eq!(get(f, ns, GLOBAL, b"y"), None);
            // the residual link cell owns nothing and is released on table drop.
        });
    }

    #[test]
    fn global_in_proc_links_to_global() {
        leak_free(|f, ns| {
            set(f, ns, GLOBAL, b"g", sobj(b"global-val")).unwrap();
            f.push(GLOBAL); // enter a proc frame
            assert_eq!(get(f, ns, GLOBAL, b"g"), None); // not visible without `global`
            make_variable(f, ns, GLOBAL, GLOBAL, b"g"); // `global g` == variable in :: context
            assert_eq!(
                as_str(get(f, ns, GLOBAL, b"g")),
                Some(b"global-val".to_vec())
            );
            set(f, ns, GLOBAL, b"g", sobj(b"updated")).unwrap(); // through the link
            f.pop();
            assert_eq!(as_str(get(f, ns, GLOBAL, b"g")), Some(b"updated".to_vec()));
            unset(f, ns, GLOBAL, b"g");
        });
    }

    #[test]
    fn unqualified_in_proc_is_frame_local() {
        // Inside a proc, `set v` is a local — it does NOT touch the namespace var.
        leak_free(|f, ns| {
            let a = ns.ensure_namespace(GLOBAL, b"::a");
            set(f, ns, GLOBAL, b"::a::v", sobj(b"10")).unwrap();
            f.push(a);
            set(f, ns, a, b"v", sobj(b"99")).unwrap(); // current_ns = ::a, but in a proc
            assert_eq!(as_str(get(f, ns, a, b"v")), Some(b"99".to_vec())); // the local
            f.pop();
            assert_eq!(as_str(get(f, ns, GLOBAL, b"::a::v")), Some(b"10".to_vec())); // untouched
            unset(f, ns, GLOBAL, b"::a::v");
        });
    }

    #[test]
    fn variable_in_proc_links_to_namespace_var() {
        leak_free(|f, ns| {
            let a = ns.ensure_namespace(GLOBAL, b"::a");
            f.push(a);
            make_variable(f, ns, a, a, b"v"); // `variable v` inside a proc of ::a
            set(f, ns, a, b"v", sobj(b"7")).unwrap(); // writes through to ::a::v
            f.pop();
            assert_eq!(as_str(get(f, ns, GLOBAL, b"::a::v")), Some(b"7".to_vec()));
            unset(f, ns, GLOBAL, b"::a::v");
        });
    }

    #[test]
    fn global_at_top_level_is_a_noop() {
        // `global g` at global scope must not create a self-link (which would loop).
        leak_free(|f, ns| {
            make_variable(f, ns, GLOBAL, GLOBAL, b"g");
            assert_eq!(get(f, ns, GLOBAL, b"g"), None); // still unset, no link installed
            set(f, ns, GLOBAL, b"g", sobj(b"1")).unwrap();
            assert_eq!(as_str(get(f, ns, GLOBAL, b"g")), Some(b"1".to_vec()));
            unset(f, ns, GLOBAL, b"g");
        });
    }
}

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
fn classify_original_name(
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
        let same = if let Some(index) = self.compiled {
            frames.original_formal_cell_is_current(index, &self.cell)
        } else {
            table(frames, ns, self.home)
                .is_some_and(|table| self.cell.matches_root(table, &self.name))
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
    table: entered_container::TableObservation,
    cell: crate::frame::VariablePublicationObservation,
    home: VarHome,
    name: Vec<u8>,
    compiled: Option<usize>,
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
        let selected = self::table(frames, ns, home).ok_or(VarError::NameProtocolUnavailable)?;
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
                    (_, None) => self::table(&frames, &ns, self.home)
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
/// The selected cell and actual table/frame birth survive the reached array
/// and search retirement stages without retaining their Native roles.
pub(crate) struct OriginalArrayMemberPreparation {
    selected: crate::frame::PreparedArrayMember,
    home: VarHome,
    root: Vec<u8>,
    table: entered_container::TableObservation,
    links: Vec<OriginalCLinkSelection>,
    element: Vec<u8>,
    retained_array: bool,
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

/// The actual target chosen before metadata retirement, without native holds.
pub(crate) struct OriginalArrayMemberSelection {
    currency: crate::interp::native_operation_currency::NativeOperationCurrency,
    observation: crate::frame::VariablePublicationObservation,
    epoch: u64,
    home: VarHome,
    root: Vec<u8>,
    element: Vec<u8>,
    table: entered_container::TableObservation,
    links: Vec<OriginalCLinkSelection>,
    retained_array: bool,
    member: Option<VarId>,
}
impl OriginalArrayMemberSelection {
    fn check(
        &self,
        interpreter: &crate::interp::Interp,
        currency: &crate::interp::native_operation_currency::NativeOperationCurrency,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        self.currency
            .ensure_current_or_refuse()
            .map_err(tcl_cmd_core::CmdError::from_execution_refusal)?;
        currency
            .ensure_current_or_refuse()
            .map_err(tcl_cmd_core::CmdError::from_execution_refusal)?;
        let physical = (|| {
            entered_container::same_table(interpreter, self.home, &self.table)?;
            for link in &self.links {
                link.check(interpreter)?;
            }
            let frames = interpreter.frames.borrow();
            let namespaces = interpreter.namespaces();
            if !table(&frames, &namespaces, self.home)
                .is_some_and(|table| self.observation.matches_root(table, &self.root))
            {
                return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original materialised root slot",
                ));
            }
            self.observation
                .check_array_current(self.epoch)
                .and_then(|()| {
                    self.member.map_or(Ok(()), |member| {
                        self.observation.check_member_current(&self.element, member)
                    })
                })
        })();
        physical.map_err(|error| {
            tcl_cmd_core::CmdError::from_execution_refusal(
                interpreter.clone().refuse_completion_value_access(error),
            )
        })
    }
    pub(crate) fn capture_receiver(
        &self,
        interpreter: &crate::interp::Interp,
        currency: &crate::interp::native_operation_currency::NativeOperationCurrency,
    ) -> Result<(crate::frame::VariableReceiver, TraceHome), tcl_cmd_core::CmdError> {
        self.check(interpreter, currency)?;
        let member = self.member.ok_or_else(|| {
            tcl_cmd_core::CmdError::from_execution_refusal(
                interpreter.clone().refuse_completion_value_access(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "unissued original array member",
                    ),
                ),
            )
        })?;
        let receiver = self
            .observation
            .capture_array_receiver(
                self.epoch,
                member,
                self.element.clone(),
                self.root.clone(),
                self.retained_array,
            )
            .map_err(|error| {
                tcl_cmd_core::CmdError::from_execution_refusal(
                    interpreter.clone().refuse_completion_value_access(error),
                )
            })?;
        let (ns, level) = match self.home {
            VarHome::Namespace(namespace) => (Some(namespace), None),
            VarHome::Frame(level) => (None, Some(level)),
        };
        let home = TraceHome {
            binding_id: receiver.binding_id(),
            selected_member: receiver.trace_member(),
            ns,
            level,
            base: self.root.clone(),
            link_elem: Some(self.element.clone()),
        };
        self.check(interpreter, currency)?;
        Ok((receiver, home))
    }
    pub(crate) fn capture_link_target(
        &self,
        interpreter: &crate::interp::Interp,
        currency: &crate::interp::native_operation_currency::NativeOperationCurrency,
    ) -> Result<Link, tcl_cmd_core::CmdError> {
        self.check(interpreter, currency)?;
        let member = self.member.ok_or_else(|| {
            tcl_cmd_core::CmdError::from_execution_refusal(
                interpreter.clone().refuse_completion_value_access(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "unissued original array member",
                    ),
                ),
            )
        })?;
        let array = self
            .observation
            .capture_array_cell(self.epoch, &self.element, member)
            .map_err(|error| {
                tcl_cmd_core::CmdError::from_execution_refusal(
                    interpreter.clone().refuse_completion_value_access(error),
                )
            })?;
        let target = Link {
            original_jim_target: None,
            native_scalar_entry: None,
            native_element_entry: None,
            array_identity: Some(array.identity()),
            array_cell: Some(array),
            home: self.home,
            name: self.root.clone(),
            elem: Some(self.element.clone()),
        };
        self.check(interpreter, currency)?;
        Ok(target)
    }
}
impl OriginalArrayMemberPreparation {
    #[cfg(test)]
    pub(crate) fn finish(
        self,
        interpreter: &crate::interp::Interp,
        currency: &crate::interp::native_operation_currency::NativeOperationCurrency,
    ) -> Result<OriginalArrayMemberSelection, tcl_cmd_core::CmdError> {
        self.finish_using(interpreter, currency, None)
    }

    pub(crate) fn finish_using(
        self,
        interpreter: &crate::interp::Interp,
        currency: &crate::interp::native_operation_currency::NativeOperationCurrency,
        stage: Option<&dyn Fn() -> Result<(), tcl_cmd_core::CmdError>>,
    ) -> Result<OriginalArrayMemberSelection, tcl_cmd_core::CmdError> {
        let observation = self.selected.observation();
        let epoch = self.selected.array_epoch();
        let home = self.home;
        let root = self.root;
        let original_table = self.table;
        let links = self.links;
        let selected = self.selected;
        let check = || {
            if let Some(check) = stage {
                check()?;
            }
            currency
                .ensure_current_or_refuse()
                .map_err(tcl_cmd_core::CmdError::from_execution_refusal)?;
            let physical = entered_container::same_table(interpreter, home, &original_table)
                .and_then(|()| {
                    for link in &links {
                        link.check(interpreter)?;
                    }
                    let frames = interpreter.frames.borrow();
                    let namespaces = interpreter.namespaces();
                    let table = table(&frames, &namespaces, home).ok_or(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "original member preparation table",
                        ),
                    )?;
                    if observation.matches_root(table, &root) {
                        Ok(())
                    } else {
                        Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "original member preparation root slot",
                        ))
                    }
                });
            physical.map_err(|error| {
                tcl_cmd_core::CmdError::from_execution_refusal(
                    interpreter.clone().refuse_completion_value_access(error),
                )
            })
        };
        selected.finish(check, |error| {
            tcl_cmd_core::CmdError::from_execution_refusal(
                interpreter.clone().refuse_completion_value_access(error),
            )
        })?;
        let member = observation.member_identity(&self.element);
        Ok(OriginalArrayMemberSelection {
            currency: currency.clone(),
            observation,
            epoch,
            home,
            root,
            table: original_table,
            links,
            element: self.element,
            retained_array: self.retained_array,
            member,
        })
    }
}

/// Same original classifier and link walk, returning before any old owner is
/// released so the actual caller can leave its frame/namespace borrows first.
#[cfg(test)]
pub(crate) fn capture_original_c_element_materialisation(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    scope: NativeOriginalNameScope,
    root: &[u8],
    element: &[u8],
    original: Option<*mut TclObj>,
    create_element: bool,
    protocol: tcl_syntax::native_variable_name::NativeVariableNameProtocol,
) -> Result<OriginalArrayMemberPreparation, crate::frame::JimLinkStoreError> {
    let NativeOriginalNameScope { current, namespace } = scope;
    let place = match classify_original_name(frames, ns, current, root, namespace) {
        Resolved::Place(place) => place,
        Resolved::Error(error) => return Err(error.into()),
    };
    capture_original_c_element_materialisation_using(
        frames,
        ns,
        place,
        element,
        original,
        create_element,
        protocol,
    )
}
pub(crate) fn capture_original_c_selected_element_materialisation(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    selected: &NativeOriginalNameCell,
    element: &[u8],
    original: Option<*mut TclObj>,
    create_element: bool,
    protocol: tcl_syntax::native_variable_name::NativeVariableNameProtocol,
) -> Result<OriginalArrayMemberPreparation, crate::frame::JimLinkStoreError> {
    selected.check_selected(frames, ns)?;
    capture_original_c_element_materialisation_using(
        frames,
        ns,
        selected.place(),
        element,
        original,
        create_element,
        protocol,
    )
}
fn capture_original_c_element_materialisation_using(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    place: Place,
    element: &[u8],
    original: Option<*mut TclObj>,
    create_element: bool,
    protocol: tcl_syntax::native_variable_name::NativeVariableNameProtocol,
) -> Result<OriginalArrayMemberPreparation, crate::frame::JimLinkStoreError> {
    let mut links = Vec::new();
    let place = follow_selected_links_observed(frames, ns, place, false, Some(&mut links))?;
    if place.elem.is_some() {
        return Err(VarError::IsScalar.into());
    }
    let observation = entered_container::TableObservation::capture(frames, ns, place.home)
        .ok_or(VarError::NameProtocolUnavailable)?;
    let member = create_element.then_some(element);
    let original = protocol
        .element_table_retains_original()
        .then_some(original)
        .flatten();
    let selected = if let Some(array) = &place.array_cell {
        array.prepare_original_array_member(member, original)?
    } else if let Some(entry) = &place.scalar_entry {
        entry.prepare_original_array_member(member, original)?
    } else {
        table_mut(frames, ns, place.home).prepare_original_array_member(
            &place.name,
            member,
            original,
        )?
    };
    Ok(OriginalArrayMemberPreparation {
        selected,
        home: place.home,
        root: place.name,
        table: observation,
        links,
        element: element.to_vec(),
        retained_array: place.array_cell.is_some(),
    })
}

/// Existing neutral fixture preparation retains its independent caller contract.
#[cfg(test)]
/// The actual array hash owns the selected original element object at birth.
pub(crate) fn prepare_original_c_element_cell(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    scope: NativeOriginalNameScope,
    root: &[u8],
    element: &[u8],
    original: Option<*mut TclObj>,
    create_element: bool,
) -> Result<(), VarError> {
    let NativeOriginalNameScope { current, namespace } = scope;
    let place = match classify_original_name(frames, ns, current, root, namespace) {
        Resolved::Place(place) => {
            follow_links(frames, ns, place).map_or_else(Resolved::Error, Resolved::Place)
        }
        error => error,
    };
    let place = match place {
        Resolved::Place(place) => place,
        Resolved::Error(error) => return Err(error),
    };
    if let Some(array) = place.array_cell {
        return if create_element {
            array.prepare_original_native_element(element, original)
        } else {
            Ok(())
        };
    }
    if let Some(entry) = place.scalar_entry {
        return if create_element {
            entry.prepare_original_element(element, original)
        } else {
            entry.ensure_array()
        };
    }
    if !create_element {
        return table_mut(frames, ns, place.home).ensure_array(&place.name);
    }
    table_mut(frames, ns, place.home).prepare_original_native_element(
        &place.name,
        element,
        original,
    )
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
