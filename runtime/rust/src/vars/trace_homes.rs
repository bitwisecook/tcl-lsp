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

//! The identity a variable trace belongs to: the home a name or a selected array member resolves to.

use super::{resolve_trace_registration, table, Resolved};
use crate::frame::{FrameStack, VarHome};
use crate::namespace::{Namespaces, NsId};
use tcl_runtime_api::VarId;

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
