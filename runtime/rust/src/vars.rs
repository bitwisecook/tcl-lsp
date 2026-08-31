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

//! The variable resolver (T1.5) — the variable parallel of the command resolver.
//!
//! One classification + one link walk, modelled on `tclVar.c:TclLookupSimpleVar`
//! (`tmp/tcl9.0.3`) and `namespace-tree.md` §5.3. Given a name and the current
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

use tcl_syntax::naming::is_qualified;

use crate::frame::{FrameStack, Link, Var, VarError, VarHome, VarTable};
use crate::namespace::{Namespaces, NsId, GLOBAL};
use crate::obj::{self, TclObj};

/// Bound on the link walk so a pathological alias cycle can't spin forever
/// (matches the frame model's conservative guard; a real recursion limit lands
/// with the proc chunk).
const LINK_LIMIT: usize = 1000;

/// A concrete place a name resolves to: a table ([`VarHome`]) + simple name +
/// optional array element. Never itself a link.
struct Place {
    home: VarHome,
    name: Vec<u8>,
    elem: Option<Vec<u8>>,
}

/// The outcome of classifying + walking a name.
enum Resolved {
    /// A concrete place to read/write.
    Place(Place),
    /// A qualified name whose namespace does not exist. Writes raise
    /// `parent namespace doesn't exist`; reads/unsets treat it as not-found.
    NoNamespace,
}

/// The var home where the current context's *local* (unqualified, non-`::`)
/// names live, and where `global`/`variable`/`upvar` install their links. In a
/// proc that is the frame's table; at global / namespace-eval scope it is the
/// current namespace's table (the global frame and global ns share one table).
fn current_home(frames: &FrameStack, current_ns: NsId) -> VarHome {
    if frames.in_proc() {
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

/// Classify `name` to its starting `(home, simple-name)`, or `NoNamespace` if it
/// is qualified into a namespace that does not exist.
fn classify(frames: &FrameStack, ns: &Namespaces, current_ns: NsId, name: &[u8]) -> Resolved {
    let (home, key) = if is_qualified(name) {
        match ns.var_home(current_ns, name) {
            Some((id, simple)) => (VarHome::Namespace(id), simple),
            None => return Resolved::NoNamespace,
        }
    } else {
        let home = ns_scope_fallback(ns, current_home(frames, current_ns), name);
        (home, name.to_vec())
    };
    Resolved::Place(Place {
        home,
        name: key,
        elem: None,
    })
}

/// M11: the Tcl 8.x namespace-scope fallback.  An unqualified name whose home
/// is a non-global namespace with **no such cell** — a `variable` declaration
/// installs a link cell, so declared names never fall through — resolves to
/// the GLOBAL namespace when it holds one, for reads and writes alike; under
/// the default 9.0 semantics (TIP 278) the home is returned unchanged.
/// tclsh 8.6/9.0-pinned in `cross_version_vars_e2e.rs` (tcl-vm) and the unit
/// pairs below.
fn ns_scope_fallback(ns: &Namespaces, home: VarHome, name: &[u8]) -> VarHome {
    if !ns.ns_var_global_fallback {
        return home;
    }
    let VarHome::Namespace(id) = home else {
        return home;
    };
    if id == crate::namespace::GLOBAL {
        return home;
    }
    if ns.var_table(id).cell(name).is_none()
        && ns.var_table(crate::namespace::GLOBAL).cell(name).is_some()
    {
        return VarHome::Namespace(crate::namespace::GLOBAL);
    }
    home
}

/// Follow `global`/`variable`/`upvar` links from `place` to the concrete cell.
fn follow_links(frames: &FrameStack, ns: &Namespaces, mut place: Place) -> Place {
    for _ in 0..LINK_LIMIT {
        let link = match table(frames, ns, place.home).and_then(|t| t.cell(&place.name)) {
            Some(Var::Link(l)) => l.clone(),
            _ => break,
        };
        // A self-link is the declared-but-undefined marker — stop here; the
        // cell reads as missing.
        if link.home == place.home && link.name == place.name && link.elem.is_none() {
            break;
        }
        // An element-on-element chain (`a(b)(c)`) is invalid; keep the outer
        // element and stop chaining elements (mirrors the frame model).
        let elem = match (place.elem.take(), link.elem) {
            (None, e) => e,
            (outer @ Some(_), _) => outer,
        };
        place = Place {
            home: link.home,
            name: link.name,
            elem,
        };
    }
    place
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
    let local_is_namespace =
        is_qualified(local) || matches!(current_home(frames, current_ns), VarHome::Namespace(_));
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
        },
    );
    matches!(target.home, VarHome::Frame(level) if frames.is_proc_at(level))
}

/// Classify then follow `global`/`variable`/`upvar` links to the concrete cell.
fn resolve(frames: &FrameStack, ns: &Namespaces, current_ns: NsId, name: &[u8]) -> Resolved {
    match classify(frames, ns, current_ns, name) {
        Resolved::Place(p) => Resolved::Place(follow_links(frames, ns, p)),
        other => other,
    }
}

/// The var home an unqualified name lives in when resolving against frame
/// `level` — the frame-addressed analogue of [`current_home`]: a proc frame's
/// own table, else that frame's namespace table (the global level → the global
/// namespace). Also the `Frames::link` target home (`state_traits.rs`).
pub(crate) fn home_at(frames: &FrameStack, level: usize) -> VarHome {
    if frames.is_proc_at(level) {
        VarHome::Frame(level)
    } else {
        VarHome::Namespace(frames.frame_ns(level))
    }
}

/// Frame-addressed [`classify`]: an unqualified name is a local of frame `level`
/// (or, for a non-proc frame, its namespace); a qualified name resolves in that
/// frame's namespace context.
fn classify_at(frames: &FrameStack, ns: &Namespaces, name: &[u8], level: usize) -> Resolved {
    let (home, key) = if is_qualified(name) {
        match ns.var_home(frames.frame_ns(level), name) {
            Some((id, simple)) => (VarHome::Namespace(id), simple),
            None => return Resolved::NoNamespace,
        }
    } else {
        let home = ns_scope_fallback(ns, home_at(frames, level), name);
        (home, name.to_vec())
    };
    Resolved::Place(Place {
        home,
        name: key,
        elem: None,
    })
}

/// Frame-addressed [`resolve`]: classify `name` as if `level` were the active
/// frame (`FrameId`-addressed access), then follow links.
fn resolve_at(frames: &FrameStack, ns: &Namespaces, name: &[u8], level: usize) -> Resolved {
    match classify_at(frames, ns, name, level) {
        Resolved::Place(p) => Resolved::Place(follow_links(frames, ns, p)),
        other => other,
    }
}

/// Resolve a variable's home plus the **simple** (unqualified) name it is
/// filed under there.
///
/// A variable trace must be keyed by the variable it resolves to, not by the
/// spelling used to register it: C Tcl hangs the trace off the `Var` struct
/// (`tclTrace.c`'s `TraceVarProc`), so `trace add variable ::v write …`
/// fires for a later `set v X` in the global namespace, and — under the 8.x
/// namespace-scope fallback — for a `set v X` inside `namespace eval` that
/// reaches the same global.  Keying on the raw spelling instead makes a trace
/// silently miss (issue #1328).
///
/// Returning both halves from the one `resolve` call keeps registration and
/// firing on exactly the same rule, including the dialect-gated fallback.
pub(crate) fn home_namespace_and_base(
    frames: &FrameStack,
    ns: &Namespaces,
    current_ns: NsId,
    name: &[u8],
) -> (Option<NsId>, Vec<u8>) {
    match resolve(frames, ns, current_ns, name) {
        Resolved::Place(p) => match p.home {
            VarHome::Namespace(id) => (Some(id), p.name),
            VarHome::Frame(_) => (None, p.name),
        },
        // Unresolvable (a qualified name into a missing namespace): keep the
        // caller's spelling so `trace info` still round-trips it.
        Resolved::NoNamespace => (None, name.to_vec()),
    }
}

/// The fully-qualified name `name` ultimately resolves to, following
/// `global`/`variable`/`upvar`/`namespace upvar` links to the target variable
/// (and array element). `Some("::ns::var")` / `Some("::ns::arr(elem)")` for a
/// namespace target; `None` if it resolves to a proc-frame local. Used by the
/// `varname` object method, which reports the real variable a link points at.
pub(crate) fn resolved_full_name(
    frames: &FrameStack,
    ns: &Namespaces,
    base_ns: NsId,
    name: &[u8],
) -> Option<Vec<u8>> {
    // Resolve as a variable of `base_ns` (the object's namespace), *not* the
    // current proc frame — `varname` reports the object's variable regardless of
    // where it is called from. Then follow links to the real target.
    let (home, key) = if is_qualified(name) {
        match ns.var_home(base_ns, name) {
            Some((id, simple)) => (VarHome::Namespace(id), simple),
            None => return None,
        }
    } else {
        (VarHome::Namespace(base_ns), name.to_vec())
    };
    let mut place = Place {
        home,
        name: key,
        elem: None,
    };
    for _ in 0..LINK_LIMIT {
        let link = match table(frames, ns, place.home).and_then(|t| t.cell(&place.name)) {
            Some(Var::Link(l)) => l.clone(),
            _ => break,
        };
        let elem = match (place.elem.take(), link.elem) {
            (None, e) => e,
            (outer @ Some(_), _) => outer,
        };
        place = Place {
            home: link.home,
            name: link.name,
            elem,
        };
    }
    let VarHome::Namespace(id) = place.home else {
        return None;
    };
    let mut fqn = ns.qualified_name(id);
    if id != GLOBAL {
        fqn.extend_from_slice(b"::"); // global's qualified name is already `::`
    }
    fqn.extend_from_slice(&place.name);
    if let Some(elem) = &place.elem {
        fqn.push(b'(');
        fqn.extend_from_slice(elem);
        fqn.push(b')');
    }
    Some(fqn)
}

// -- the public coordinator API (mirrors the old FrameStack surface) ---------

/// `set name value` — write through links to wherever `name` resolves. The cell
/// takes a **+1** on `obj`. A qualified write into a missing namespace errors.
pub(crate) fn set(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    current_ns: NsId,
    name: &[u8],
    obj: *mut TclObj,
) -> Result<(), VarError> {
    let place = match resolve(frames, ns, current_ns, name) {
        Resolved::Place(p) => p,
        Resolved::NoNamespace => return Err(VarError::NoSuchNamespace),
    };
    let t = table_mut(frames, ns, place.home);
    match place.elem {
        Some(elem) => t.store_elem(&place.name, &elem, obj),
        None => t.store_scalar(&place.name, obj),
    }
}

/// `set name(key) value`. Errors if `name` is a scalar / its ns is missing.
pub(crate) fn set_elem(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    current_ns: NsId,
    name: &[u8],
    key: &[u8],
    obj: *mut TclObj,
) -> Result<(), VarError> {
    let place = match resolve(frames, ns, current_ns, name) {
        Resolved::Place(p) => p,
        Resolved::NoNamespace => return Err(VarError::NoSuchNamespace),
    };
    if place.elem.is_some() {
        // `a(b)(c)` — the resolved place is already an element.
        return Err(VarError::IsScalar);
    }
    table_mut(frames, ns, place.home).store_elem(&place.name, key, obj)
}

/// `set name` — borrowed value, or `None` (unset / missing namespace).
pub(crate) fn get(
    frames: &FrameStack,
    ns: &Namespaces,
    current_ns: NsId,
    name: &[u8],
) -> Option<*mut TclObj> {
    match resolve(frames, ns, current_ns, name) {
        Resolved::Place(p) => {
            let t = table(frames, ns, p.home)?;
            match &p.elem {
                Some(elem) => t.load_elem(&p.name, elem),
                None => t.load_scalar(&p.name),
            }
        }
        Resolved::NoNamespace => None,
    }
}

// -- frame-addressed access (the `VarStore` `FrameId`-honouring path) ---------
//
// These resolve `name` as if `level` were the active frame, following links —
// the `set`/`get`/`unset`/`exists` above are exactly these at the active level.
// The runtime's `VarStore` uses them only for a non-active `FrameId`; the active
// frame keeps the by-name accessors verbatim.

/// Frame-addressed [`get`] — read `name` resolved against frame `level`.
pub(crate) fn get_at(
    frames: &FrameStack,
    ns: &Namespaces,
    name: &[u8],
    level: usize,
) -> Option<*mut TclObj> {
    match resolve_at(frames, ns, name, level) {
        Resolved::Place(p) => {
            let t = table(frames, ns, p.home)?;
            match &p.elem {
                Some(elem) => t.load_elem(&p.name, elem),
                None => t.load_scalar(&p.name),
            }
        }
        Resolved::NoNamespace => None,
    }
}

/// Frame-addressed array-element read. `name` and `key` stay separate so an
/// array base containing `(` is never reconstructed and parsed again.
pub(crate) fn get_elem_at(
    frames: &FrameStack,
    ns: &Namespaces,
    name: &[u8],
    key: &[u8],
    level: usize,
) -> Option<*mut TclObj> {
    match resolve_at(frames, ns, name, level) {
        Resolved::Place(p) if p.elem.is_none() => table(frames, ns, p.home)?.load_elem(&p.name, key),
        _ => None,
    }
}

/// Frame-addressed [`set`] — the cell takes a **+1** on `obj`.
pub(crate) fn set_at(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    name: &[u8],
    obj: *mut TclObj,
    level: usize,
) -> Result<(), VarError> {
    let place = match resolve_at(frames, ns, name, level) {
        Resolved::Place(p) => p,
        Resolved::NoNamespace => return Err(VarError::NoSuchNamespace),
    };
    let t = table_mut(frames, ns, place.home);
    match place.elem {
        Some(elem) => t.store_elem(&place.name, &elem, obj),
        None => t.store_scalar(&place.name, obj),
    }
}

/// Frame-addressed array-element write. The caller retains ownership on an
/// error, matching [`set_at`].
pub(crate) fn set_elem_at(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    name: &[u8],
    key: &[u8],
    obj: *mut TclObj,
    level: usize,
) -> Result<(), VarError> {
    let place = match resolve_at(frames, ns, name, level) {
        Resolved::Place(p) if p.elem.is_none() => p,
        Resolved::Place(_) => return Err(VarError::IsScalar),
        Resolved::NoNamespace => return Err(VarError::NoSuchNamespace),
    };
    table_mut(frames, ns, place.home).store_elem(&place.name, key, obj)
}

/// Frame-addressed [`unset`] — returns whether the variable existed.
pub(crate) fn unset_at(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    name: &[u8],
    level: usize,
) -> bool {
    let place = match resolve_at(frames, ns, name, level) {
        Resolved::Place(p) => p,
        Resolved::NoNamespace => return false,
    };
    let t = table_mut(frames, ns, place.home);
    match place.elem {
        Some(elem) => t.remove_elem(&place.name, &elem),
        None => t.remove(&place.name),
    }
}

/// Frame-addressed array-element removal. `name` and `key` remain a pair at
/// the storage boundary for the same reason as [`get_elem_at`].
pub(crate) fn unset_elem_at(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    name: &[u8],
    key: &[u8],
    level: usize,
) -> bool {
    match resolve_at(frames, ns, name, level) {
        Resolved::Place(p) if p.elem.is_none() => table_mut(frames, ns, p.home).remove_elem(&p.name, key),
        _ => false,
    }
}

/// Frame-addressed [`exists`].
pub(crate) fn exists_at(frames: &FrameStack, ns: &Namespaces, name: &[u8], level: usize) -> bool {
    match resolve_at(frames, ns, name, level) {
        Resolved::Place(p) if p.elem.is_none() => {
            table(frames, ns, p.home).is_some_and(|t| t.is_set(&p.name))
        }
        Resolved::Place(p) => table(frames, ns, p.home)
            .and_then(|t| p.elem.as_ref().map(|e| t.load_elem(&p.name, e).is_some()))
            .unwrap_or(false),
        Resolved::NoNamespace => false,
    }
}

/// `set name(key)` — borrowed.
pub(crate) fn get_elem(
    frames: &FrameStack,
    ns: &Namespaces,
    current_ns: NsId,
    name: &[u8],
    key: &[u8],
) -> Option<*mut TclObj> {
    match resolve(frames, ns, current_ns, name) {
        Resolved::Place(p) if p.elem.is_none() => {
            let t = table(frames, ns, p.home)?;
            // A missing element of an array with a TIP 508 default reads as the
            // default (without creating the element).
            t.load_elem(&p.name, key)
                .or_else(|| t.array_default(&p.name))
        }
        _ => None,
    }
}

/// `array default set arrayName value` — ensure the array exists and set its
/// default value. `Err(IsScalar)` if the name is a non-array scalar.
pub(crate) fn set_array_default(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    current_ns: NsId,
    name: &[u8],
    obj: *mut TclObj,
) -> Result<(), VarError> {
    let place = match resolve(frames, ns, current_ns, name) {
        Resolved::Place(p) if p.elem.is_none() => p,
        Resolved::NoNamespace => return Err(VarError::NoSuchNamespace),
        _ => return Err(VarError::IsScalar),
    };
    let t = table_mut(frames, ns, place.home);
    t.ensure_array(&place.name)?;
    t.set_array_default(&place.name, obj);
    Ok(())
}

/// Ensure `name` is an (possibly empty) array, creating an empty one if it is
/// unset — `array set name {}` with an empty value list still materialises the
/// array (C's `TclArraySet`). A scalar `name` errors `IsScalar`. Resolves
/// links/namespaces like the element setters.
pub(crate) fn ensure_array(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    current_ns: NsId,
    name: &[u8],
) -> Result<(), VarError> {
    let place = match resolve(frames, ns, current_ns, name) {
        Resolved::Place(p) if p.elem.is_none() => p,
        Resolved::NoNamespace => return Err(VarError::NoSuchNamespace),
        _ => return Err(VarError::IsScalar),
    };
    table_mut(frames, ns, place.home).ensure_array(&place.name)
}

/// Materialise an unset scalar cell for `trace add variable`.  This uses the
/// same self-link representation as `variable`: it is observable by traces,
/// but remains unset until a subsequent write.  A qualified name in a missing
/// namespace is an error, as it is for a normal variable write.
pub(crate) fn ensure_undefined(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    current_ns: NsId,
    name: &[u8],
) -> Result<(), VarError> {
    let place = match resolve(frames, ns, current_ns, name) {
        Resolved::Place(p) if p.elem.is_none() => p,
        Resolved::NoNamespace => return Err(VarError::NoSuchNamespace),
        _ => return Err(VarError::IsScalar),
    };
    let home = place.home;
    let key = place.name;
    let table = table_mut(frames, ns, home);
    if table.cell(&key).is_none() {
        table.insert_link(
            &key,
            Link {
                home,
                name: key.clone(),
                elem: None,
            },
        );
    }
    Ok(())
}

/// `array default get/exists` — the array's default value (following links), or
/// `None` if the name isn't an array or has no default.
pub(crate) fn array_default(
    frames: &FrameStack,
    ns: &Namespaces,
    current_ns: NsId,
    name: &[u8],
) -> Option<*mut TclObj> {
    match resolve(frames, ns, current_ns, name) {
        Resolved::Place(p) if p.elem.is_none() => table(frames, ns, p.home)?.array_default(&p.name),
        _ => None,
    }
}

/// `array default unset` — drop the array's default value (if any).
pub(crate) fn unset_array_default(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    current_ns: NsId,
    name: &[u8],
) {
    if let Resolved::Place(p) = resolve(frames, ns, current_ns, name) {
        if p.elem.is_none() {
            table_mut(frames, ns, p.home).unset_array_default(&p.name);
        }
    }
}

/// `unset name` — remove the variable `name` resolves to (following links).
/// Returns whether it existed.
pub(crate) fn unset(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    current_ns: NsId,
    name: &[u8],
) -> bool {
    let place = match resolve(frames, ns, current_ns, name) {
        Resolved::Place(p) => p,
        Resolved::NoNamespace => return false,
    };
    let t = table_mut(frames, ns, place.home);
    match place.elem {
        Some(elem) => t.remove_elem(&place.name, &elem),
        None => t.remove(&place.name),
    }
}

/// `unset name(key)` — remove one array element. Returns whether it existed.
pub(crate) fn unset_elem(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    current_ns: NsId,
    name: &[u8],
    key: &[u8],
) -> bool {
    let place = match resolve(frames, ns, current_ns, name) {
        Resolved::Place(p) if p.elem.is_none() => p,
        _ => return false,
    };
    table_mut(frames, ns, place.home).remove_elem(&place.name, key)
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

/// Whether `name` resolves to an array variable (the `set a` array-vs-scalar
/// diagnostic; `array exists`).
pub(crate) fn is_array(
    frames: &FrameStack,
    ns: &Namespaces,
    current_ns: NsId,
    name: &[u8],
) -> bool {
    match resolve(frames, ns, current_ns, name) {
        Resolved::Place(p) if p.elem.is_none() => {
            table(frames, ns, p.home).is_some_and(|t| t.is_array(&p.name))
        }
        _ => false,
    }
}

/// Whether the scalar/array `name` is set (`info exists`, scalar form).
pub(crate) fn exists(frames: &FrameStack, ns: &Namespaces, current_ns: NsId, name: &[u8]) -> bool {
    match resolve(frames, ns, current_ns, name) {
        Resolved::Place(p) if p.elem.is_none() => {
            table(frames, ns, p.home).is_some_and(|t| t.is_set(&p.name))
        }
        // A link resolved to an element, or a missing namespace: fall back to the
        // element check / not-found.
        Resolved::Place(p) => table(frames, ns, p.home)
            .and_then(|t| p.elem.as_ref().map(|e| t.load_elem(&p.name, e).is_some()))
            .unwrap_or(false),
        Resolved::NoNamespace => false,
    }
}

/// Whether the array element `name(key)` is set (`info exists arr(key)`).
pub(crate) fn exists_elem(
    frames: &FrameStack,
    ns: &Namespaces,
    current_ns: NsId,
    name: &[u8],
    key: &[u8],
) -> bool {
    // `info exists arr(k)` checks the *actual* element — an array default does
    // not make a missing element "exist" (TIP 508).
    match resolve(frames, ns, current_ns, name) {
        Resolved::Place(p) if p.elem.is_none() => {
            table(frames, ns, p.home).is_some_and(|t| t.load_elem(&p.name, key).is_some())
        }
        _ => false,
    }
}

/// The element names of array `name` (`array names`/`array get`), or `None` if
/// `name` isn't an array. Sorted (deterministic).
pub(crate) fn array_names(
    frames: &FrameStack,
    ns: &Namespaces,
    current_ns: NsId,
    name: &[u8],
) -> Option<Vec<Vec<u8>>> {
    match resolve(frames, ns, current_ns, name) {
        Resolved::Place(p) if p.elem.is_none() => table(frames, ns, p.home)?
            .array_names(&p.name)
            .map(|ks| ks.into_iter().map(<[u8]>::to_vec).collect()),
        _ => None,
    }
}

/// Resolve a variable reference to its string bytes (the `$var`/`$arr(idx)`
/// subst hook). `None` for an unset/missing variable.
pub(crate) fn resolve_var_bytes(
    frames: &FrameStack,
    ns: &Namespaces,
    current_ns: NsId,
    name: &[u8],
    index: Option<&[u8]>,
) -> Option<Vec<u8>> {
    let obj = match index {
        Some(key) => get_elem(frames, ns, current_ns, name, key)?,
        None => get(frames, ns, current_ns, name)?,
    };
    // SAFETY: `obj` is a live, table-owned object; `get_string` reads/shims its
    // string rep and returns a borrowed pointer we copy out immediately.
    unsafe {
        let mut len: obj::TclSize = 0;
        let p = obj::get_string(obj, &mut len);
        if p.is_null() {
            return Some(Vec::new());
        }
        Some(std::slice::from_raw_parts(p as *const u8, len as usize).to_vec())
    }
}

// -- link installation (global / variable / upvar) ---------------------------

/// Install a link from the current context's `local` name to `target`, unless it
/// would be a self-link (already that exact cell — the no-op `global`/`variable`
/// at namespace scope produces).
fn link_local(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    current_ns: NsId,
    local: &[u8],
    target: Link,
) {
    let here = current_home(frames, current_ns);
    if target.home == here && target.elem.is_none() && target.name == local {
        return; // already the same variable — `global`/`variable` is a no-op
    }
    table_mut(frames, ns, here).insert_link(local, target);
}

/// `variable tail` / `global tail` — link the current context's `tail` to the
/// namespace var `target_ns::tail` (a no-op when the current context already *is*
/// `target_ns`). `global` is just this with `target_ns` resolved in the global
/// context (`::a::x` → `::a`, bare `x` → `::`).
pub(crate) fn make_variable(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    current_ns: NsId,
    target_ns: NsId,
    tail: &[u8],
) {
    make_variable_mapped(frames, ns, current_ns, target_ns, tail, tail);
}

/// Like [`make_variable`] but links the local name `local` to a differently-
/// named namespace variable `target` in `target_ns` (TIP 500 private instance
/// variables, whose storage name is mangled per declaring class).
pub(crate) fn make_variable_mapped(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    current_ns: NsId,
    target_ns: NsId,
    local: &[u8],
    target: &[u8],
) {
    // C's `variable` (`TclLookupSimpleVar` with create) materialises the
    // namespace variable itself as an *undefined Var* before any value is
    // set.  The self-link cell is our stand-in: persistent in the namespace
    // table, it reads / `info exists` as missing, a write replaces it — and,
    // under the 8.x semantics, it blocks the M11 namespace-scope global
    // fallback exactly as C's undefined Var does.  An existing value is
    // never clobbered.
    if ns.var_table(target_ns).cell(target).is_none() {
        ns.var_table_mut(target_ns).insert_link(
            target,
            Link {
                home: VarHome::Namespace(target_ns),
                name: target.to_vec(),
                elem: None,
            },
        );
    }
    link_local(
        frames,
        ns,
        current_ns,
        local,
        Link {
            home: VarHome::Namespace(target_ns),
            name: target.to_vec(),
            elem: None,
        },
    );
}

/// `upvar` — link the current context's `local` to the variable `(home, name,
/// elem)`. A frame target at level 0 is the global namespace.
pub(crate) fn make_upvar(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    current_ns: NsId,
    target: Link,
    local: &[u8],
) {
    let target = match target.home {
        // Level 0 is the global context, whose table is the global namespace's.
        VarHome::Frame(0) => Link {
            home: VarHome::Namespace(GLOBAL),
            ..target
        },
        _ => target,
    };
    link_local(frames, ns, current_ns, local, target);
}

/// `upvar … target ns::tail` — install the link as the namespace variable
/// `home_ns::tail` (a qualified local name names a namespace link var, not a
/// frame local; C's `MakeUpvar` with a `::`-containing local name).
pub(crate) fn make_upvar_in(
    frames: &mut FrameStack,
    ns: &mut Namespaces,
    home_ns: NsId,
    tail: &[u8],
    target: Link,
) {
    let target = match target.home {
        VarHome::Frame(0) => Link {
            home: VarHome::Namespace(GLOBAL),
            ..target
        },
        _ => target,
    };
    table_mut(frames, ns, VarHome::Namespace(home_ns)).insert_link(tail, target);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capi::Tcl_NewStringObj;
    use crate::counters;

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
