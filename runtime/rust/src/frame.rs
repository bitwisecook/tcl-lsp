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

//! Variable tables + call frames (T1.3, extended for namespaces in T1.5).
//!
//! Canonical model: `tclInt.h`'s `Var` is a tagged union
//! `{ scalar objPtr | array tablePtr | linkPtr }` held in a hash table
//! (`tmp/tcl9.0.3/generic/tclVar.c`). In C every variable lives in *some*
//! [`VarTable`]: a proc call frame's locals, or a **namespace** var table
//! (`Namespace.varTable`) — the global frame's table *is* the global
//! namespace's. This module owns the cell mechanics ([`Var`], [`VarTable`]) and
//! the call-frame container ([`FrameStack`]); the **classification + link walk**
//! that ties frames and namespaces together (the variable parallel of the
//! command resolver) lives in [`crate::vars`].
//!
//! Representation decisions (see namespace-tree.md §5.3):
//! - **`BTreeMap`** (not `HashMap`) for both the var table and array elements —
//!   deterministic iteration (`std::HashMap`'s `RandomState` would make
//!   `info vars` / `array names` vary run-to-run, poison for an oracle-diffed
//!   port) and zero external deps.
//! - **Links resolved by path** ([`Link`] = `{home, name, elem}`), not Tcl's
//!   direct `linkPtr`, so a target map reallocating can't dangle a pointer. The
//!   `home` ([`VarHome`]) is either a frame level or a namespace — `global` /
//!   `variable` / `upvar` all produce one shape.
//! - **`VarTable` releases on `Drop`** (matches `TclFreeVar` and keeps every
//!   refcount move visible to the leak counters, `crate::counters`) — so a
//!   dropped frame *or* namespace never leaks, with no hand-written cleanup.
//!
//! Refcount discipline (`memory-management.md` MM-B, `refcount-contract.md`):
//! a scalar cell / array element owns **+1** of its object; storing retains,
//! overwriting/unsetting/dropping releases. Links own nothing.

use std::collections::BTreeMap;

use crate::namespace::NsId;
use crate::obj::{self, TclObj};

/// A variable cell: the `tclInt.h` `Var` union as an enum.
pub enum Var {
    /// A scalar — owns **+1** of the object.
    Scalar(*mut TclObj),
    /// An associative array: element key → value (each owns **+1**).
    Array(BTreeMap<Vec<u8>, *mut TclObj>),
    /// An `upvar`/`global`/`variable` alias to another variable (owns nothing).
    Link(Link),
}

/// Where a variable physically lives: a proc-call frame, or a namespace var
/// table. `upvar` produces `Frame`; `global`/`variable` produce `Namespace`.
/// The global frame and the global namespace share one table, so a level-0
/// frame target is canonicalised to `Namespace(GLOBAL)` at the link site.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VarHome {
    /// A proc call frame's local table, by absolute level.
    Frame(usize),
    /// A namespace's variable table, by arena id.
    Namespace(NsId),
}

/// A path-resolved alias target (an `upvar`/`global`/`variable` link).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    /// Where the target variable lives.
    pub home: VarHome,
    /// Target variable name (simple, within `home`'s table).
    pub name: Vec<u8>,
    /// Target array element, for `upvar … a(b) x`.
    pub elem: Option<Vec<u8>>,
}

impl Var {
    /// Release every object reference this var owns. Consumes `self`; call
    /// exactly once when the cell leaves its table.
    fn release(self) {
        match self {
            // SAFETY: a stored cell owns a +1 on each object; releasing balances
            // the retain taken when it was stored.
            Var::Scalar(p) => unsafe { obj::decr_ref_count(p) },
            Var::Array(map) => {
                for (_, p) in map {
                    unsafe { obj::decr_ref_count(p) }
                }
            }
            Var::Link(_) => {}
        }
    }
}

/// Why a variable write failed (the type-mismatch / namespace cases Tcl reports).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarError {
    /// `set a v` where `a` is an array (`can't set "a": variable is array`).
    IsArray,
    /// `set a(k) v` where `a` is a scalar (`can't set "a(k)": variable isn't array`).
    IsScalar,
    /// `set ::nosuch::x v` — a qualified write whose namespace doesn't exist
    /// (`can't set "::nosuch::x": parent namespace doesn't exist`). Only the
    /// *create* path raises this; reads/unsets of the same name report
    /// `no such variable` instead.
    NoSuchNamespace,
    /// A write trace's callback errored: the set fails with `can't set "name":
    /// <msg>`, the trace message carried out-of-band in `TraceTable::pending_err`
    /// (kept unit so `VarError` stays `Copy`; the `var_error` callers propagate
    /// it unchanged).
    TraceError,
    /// A write/unset of a `const` variable (`can't set "name": variable is a
    /// constant`; the command supplies the `set`/`incr`/`unset` verb).
    IsConstant,
}

// ---------------------------------------------------------------------------
// VarTable — the per-frame / per-namespace name→cell store + cell mechanics.
// ---------------------------------------------------------------------------

/// A variable table: simple-name → [`Var`] cell, with the refcount discipline
/// for the objects its scalars/arrays own. **Direct** ops only — no link
/// following (that crosses tables and is the [`crate::vars`] coordinator's job).
/// Used by both a call [`Frame`] and a namespace (`namespace.rs`).
#[derive(Default)]
pub struct VarTable {
    vars: BTreeMap<Vec<u8>, Var>,
    /// Scalars flagged `const` (TIP 677): a write or unset of one errors with
    /// `variable is a constant`. Names are simple (table-local); a name stays
    /// here for the table's lifetime since a constant cannot be unset.
    consts: std::collections::BTreeSet<Vec<u8>>,
    /// Per-array default values (TIP 508 `array default set`): array name → the
    /// value returned for a read of a missing element. Each owns a **+1**.
    array_defaults: BTreeMap<Vec<u8>, *mut TclObj>,
}

impl VarTable {
    /// The cell bound to `name`, if any (for link inspection / introspection).
    pub(crate) fn cell(&self, name: &[u8]) -> Option<&Var> {
        self.vars.get(name)
    }

    /// Whether `name` is a `const` scalar here.
    pub(crate) fn is_constant(&self, name: &[u8]) -> bool {
        self.consts.contains(name)
    }

    /// Names of the `const` scalars in this table (`info consts`).
    pub(crate) fn const_names(&self) -> Vec<&[u8]> {
        self.consts.iter().map(|k| k.as_slice()).collect()
    }

    /// Set array `name`'s default value (`array default set`), releasing any
    /// prior default. The table pins the value with **+2**: one for ownership,
    /// and one so a read of the default always sees it as *shared* — a
    /// read-modify-write (`lappend`/`append`/`dict`) then copies rather than
    /// mutating the stored default in place (TIP 508 copy-on-write).
    pub(crate) fn set_array_default(&mut self, name: &[u8], obj: *mut TclObj) {
        // SAFETY: retain the new default twice, release any prior one twice.
        unsafe {
            obj::incr_ref_count(obj);
            obj::incr_ref_count(obj);
        }
        if let Some(old) = self.array_defaults.insert(name.to_vec(), obj) {
            unsafe {
                obj::decr_ref_count(old);
                obj::decr_ref_count(old);
            }
        }
    }

    /// Array `name`'s default value, if one is set (`array default get`, and the
    /// fallback for a read of a missing element). Borrowed; the table keeps its +1.
    pub(crate) fn array_default(&self, name: &[u8]) -> Option<*mut TclObj> {
        self.array_defaults.get(name).copied()
    }

    /// Remove array `name`'s default value (`array default unset`).
    pub(crate) fn unset_array_default(&mut self, name: &[u8]) {
        if let Some(old) = self.array_defaults.remove(name) {
            // SAFETY: balances the +2 taken in `set_array_default`.
            unsafe {
                obj::decr_ref_count(old);
                obj::decr_ref_count(old);
            }
        }
    }

    /// Flag the scalar `name` `const` (the `const` command, after its value is
    /// stored). A no-op if already flagged.
    pub(crate) fn mark_constant(&mut self, name: &[u8]) {
        self.consts.insert(name.to_vec());
    }

    /// `set name value` into this table directly. The cell takes a **+1**.
    pub(crate) fn store_scalar(&mut self, name: &[u8], obj: *mut TclObj) -> Result<(), VarError> {
        if self.consts.contains(name) {
            return Err(VarError::IsConstant);
        }
        match self.vars.get_mut(name) {
            Some(Var::Array(_)) => Err(VarError::IsArray),
            Some(Var::Scalar(slot)) => {
                // SAFETY: retain the new value, release the prior occupant
                // (MM-B.2). Order: retain-then-release is alias-safe (set x $x).
                unsafe {
                    obj::incr_ref_count(obj);
                    obj::decr_ref_count(*slot);
                }
                *slot = obj;
                Ok(())
            }
            // The declared-but-undefined marker (a self-link, installed by
            // `variable` — see `vars::make_variable_mapped`) is exactly the
            // undefined Var a first write defines; any other link the
            // coordinator would have followed.
            Some(Var::Link(l)) if l.name == name && l.elem.is_none() => {
                // SAFETY: fresh cell takes a +1 (the link owned nothing).
                unsafe { obj::incr_ref_count(obj) };
                self.vars.insert(name.to_vec(), Var::Scalar(obj));
                Ok(())
            }
            Some(Var::Link(_)) => unreachable!("the coordinator never lands on a link"),
            None => {
                // SAFETY: fresh cell takes a +1.
                unsafe { obj::incr_ref_count(obj) };
                self.vars.insert(name.to_vec(), Var::Scalar(obj));
                Ok(())
            }
        }
    }

    /// `set name` — the scalar's value (borrowed; the table keeps its +1).
    pub(crate) fn load_scalar(&self, name: &[u8]) -> Option<*mut TclObj> {
        match self.vars.get(name) {
            Some(Var::Scalar(p)) => Some(*p),
            _ => None,
        }
    }

    /// `set name(key) value`. Errors if `name` is a scalar.
    pub(crate) fn store_elem(
        &mut self,
        name: &[u8],
        key: &[u8],
        obj: *mut TclObj,
    ) -> Result<(), VarError> {
        match self.vars.get_mut(name) {
            Some(Var::Scalar(_)) => Err(VarError::IsScalar),
            Some(Var::Array(map)) => {
                // SAFETY: retain the new element value; release any prior one.
                unsafe { obj::incr_ref_count(obj) };
                if let Some(old) = map.insert(key.to_vec(), obj) {
                    unsafe { obj::decr_ref_count(old) };
                }
                Ok(())
            }
            // The declared-but-undefined marker: the first element write
            // defines the array over it (see `store_scalar`).
            Some(Var::Link(l)) if l.name == name && l.elem.is_none() => {
                let mut map = BTreeMap::new();
                unsafe { obj::incr_ref_count(obj) };
                map.insert(key.to_vec(), obj);
                self.vars.insert(name.to_vec(), Var::Array(map));
                Ok(())
            }
            Some(Var::Link(_)) => unreachable!("the coordinator never lands on a link"),
            None => {
                let mut map = BTreeMap::new();
                unsafe { obj::incr_ref_count(obj) };
                map.insert(key.to_vec(), obj);
                self.vars.insert(name.to_vec(), Var::Array(map));
                Ok(())
            }
        }
    }

    /// `set name(key)` — borrowed.
    pub(crate) fn load_elem(&self, name: &[u8], key: &[u8]) -> Option<*mut TclObj> {
        match self.vars.get(name) {
            Some(Var::Array(map)) => map.get(key).copied(),
            _ => None,
        }
    }

    /// Remove the whole variable `name` (scalar or array); returns whether it
    /// existed. Releases every object it owned.
    pub(crate) fn remove(&mut self, name: &[u8]) -> bool {
        // A removed array drops its TIP 508 default too.
        self.unset_array_default(name);
        match self.vars.remove(name) {
            Some(v) => {
                v.release();
                true
            }
            None => false,
        }
    }

    /// Ensure `name` is an (at least empty) array — `array default set` creates
    /// the array even with no elements. Returns `Err(IsScalar)` if `name` is a
    /// scalar. (`IsScalar` reuses the closest existing variant; the caller maps
    /// it to the `array default set` message.)
    pub(crate) fn ensure_array(&mut self, name: &[u8]) -> Result<(), VarError> {
        match self.vars.get(name) {
            Some(Var::Array(_)) => Ok(()),
            Some(Var::Scalar(_)) => Err(VarError::IsScalar),
            // The declared-but-undefined marker defines as an array here.
            Some(Var::Link(l)) if l.name == name && l.elem.is_none() => {
                self.vars.insert(name.to_vec(), Var::Array(BTreeMap::new()));
                Ok(())
            }
            Some(Var::Link(_)) => unreachable!("the coordinator never lands on a link"),
            None => {
                self.vars.insert(name.to_vec(), Var::Array(BTreeMap::new()));
                Ok(())
            }
        }
    }

    /// Remove one array element `name(key)`; returns whether it existed.
    pub(crate) fn remove_elem(&mut self, name: &[u8], key: &[u8]) -> bool {
        if let Some(Var::Array(map)) = self.vars.get_mut(name) {
            if let Some(old) = map.remove(key) {
                // SAFETY: the array element owned a +1; releasing balances it.
                unsafe { obj::decr_ref_count(old) };
                return true;
            }
        }
        false
    }

    /// Install `link` under `name`, releasing any cell it replaces.
    pub(crate) fn insert_link(&mut self, name: &[u8], link: Link) {
        if let Some(old) = self.vars.insert(name.to_vec(), Var::Link(link)) {
            old.release();
        }
    }

    /// Whether `name` is an array variable here (the `set a` array-vs-scalar
    /// diagnostic; `array exists`).
    pub(crate) fn is_array(&self, name: &[u8]) -> bool {
        matches!(self.vars.get(name), Some(Var::Array(_)))
    }

    /// Whether `name` is a defined scalar or array here (not a link) — the
    /// terminal check behind `info exists`.
    pub(crate) fn is_set(&self, name: &[u8]) -> bool {
        matches!(self.vars.get(name), Some(Var::Scalar(_) | Var::Array(_)))
    }

    /// Names of all variables in this table, sorted (`info vars`/`locals`/
    /// `globals`).
    pub(crate) fn names(&self) -> Vec<&[u8]> {
        self.vars.keys().map(|k| k.as_slice()).collect()
    }

    /// Names of the table's *direct* variables (scalars/arrays), excluding
    /// links (`global`/`upvar`/`variable` / auto-linked instance vars) — for
    /// `info locals`, which lists only true locals.
    pub(crate) fn non_link_names(&self) -> Vec<&[u8]> {
        self.vars
            .iter()
            .filter(|(_, v)| !matches!(v, Var::Link(_)))
            .map(|(k, _)| k.as_slice())
            .collect()
    }

    /// Element names of array `name`, sorted (`array names`); `None` if not an
    /// array.
    pub(crate) fn array_names(&self, name: &[u8]) -> Option<Vec<&[u8]>> {
        match self.vars.get(name) {
            Some(Var::Array(map)) => Some(map.keys().map(|k| k.as_slice()).collect()),
            _ => None,
        }
    }
}

impl Drop for VarTable {
    /// Release every object every cell owns, so a dropped frame/namespace never
    /// leaks.
    fn drop(&mut self) {
        for (_, var) in std::mem::take(&mut self.vars) {
            var.release();
        }
        for (_, obj) in std::mem::take(&mut self.array_defaults) {
            // SAFETY: balances the +2 taken in `set_array_default`.
            unsafe {
                obj::decr_ref_count(obj);
                obj::decr_ref_count(obj);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// FrameStack — the proc call frames (level 0 is the global context).
// ---------------------------------------------------------------------------

/// One call frame: its local variable table, absolute level, and the namespace
/// it runs in (so `uplevel` can restore the target frame's namespace context —
/// the `CallFrame.nsPtr` analogue).
struct Frame {
    table: VarTable,
    /// The logical call level (`info level`, `upvar`/`uplevel` arithmetic) — the
    /// invoking var-frame's level + 1, **not** the stack index. They diverge
    /// when a proc is invoked while `uplevel` has redirected the active frame:
    /// e.g. `uplevel 1 [list SomeProc …]` (tcltest's idiom) pushes `SomeProc` at
    /// a deeper stack index but logical level `target+1`.
    level: usize,
    ns: NsId,
    /// The command words that invoked this frame (`info level N`): the proc name
    /// followed by its arguments. Empty for the global frame.
    words: Vec<Vec<u8>>,
    /// Whether this is a *proc* call frame (its own local variables) versus a
    /// *namespace* frame (`namespace eval`/`inscope` — unqualified names resolve
    /// to the namespace, not frame-local). The global frame is non-proc.
    is_proc: bool,
    /// The `active_level` to restore when this frame is popped (the var-frame in
    /// effect when it was pushed). Restores correctly even when the push
    /// happened under an `uplevel` redirection.
    saved_active: usize,
}

impl Frame {
    fn new(level: usize, ns: NsId) -> Self {
        Frame {
            table: VarTable::default(),
            level,
            ns,
            words: Vec::new(),
            is_proc: false,
            saved_active: 0,
        }
    }
}

/// The call-frame stack. `frames[0]` is the global *level* (its own table stays
/// empty — global variables live in the global **namespace's** table, which the
/// coordinator routes to). `active_level` is the frame whose variables are
/// currently visible (the `varFramePtr` analogue): normally the top, but
/// `uplevel` points it at an enclosing frame for the duration of a body. Frame
/// index == level.
pub struct FrameStack {
    frames: Vec<Frame>,
    active_level: usize,
}

impl Default for FrameStack {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameStack {
    /// A new stack with just the global level (0), in the global namespace.
    pub fn new() -> Self {
        FrameStack {
            frames: vec![Frame::new(0, crate::namespace::GLOBAL)],
            active_level: 0,
        }
    }

    /// The **active** variable frame level (`varFramePtr`); what unqualified
    /// names resolve against. Normally the top frame, but `uplevel` redirects it.
    pub fn current_level(&self) -> usize {
        self.active_level
    }

    /// Whether the active frame is a proc call frame. Unqualified names resolve
    /// frame-local only here; at global / `namespace eval` scope they resolve to
    /// the current namespace.
    pub fn in_proc(&self) -> bool {
        self.index_of_level(self.active_level)
            .is_some_and(|i| self.frames[i].is_proc)
    }

    /// Whether the frame at logical `level` is a proc call frame (vs the global
    /// or a `namespace eval` frame) — the frame-addressed analogue of
    /// [`in_proc`](Self::in_proc), for `FrameId`-addressed variable access.
    pub(crate) fn is_proc_at(&self, level: usize) -> bool {
        self.index_of_level(level)
            .is_some_and(|i| self.frames[i].is_proc)
    }

    /// Whether `name` is bound to a link (`global`/`upvar`/`variable`) in the
    /// active frame — used to decide if a variable trace on an unqualified name
    /// is frame-local (it is not when the name links to an outer variable, which
    /// outlives this frame).
    pub(crate) fn current_is_link(&self, name: &[u8]) -> bool {
        self.index_of_level(self.active_level)
            .and_then(|i| self.frames[i].table.cell(name))
            .is_some_and(|c| matches!(c, Var::Link(_)))
    }

    /// The true top-of-stack level (`framePtr`), independent of any `uplevel`
    /// redirection of the active level.
    pub fn top_level(&self) -> usize {
        self.frames.last().map_or(0, |f| f.level)
    }

    /// Push a new proc call frame running in namespace `ns`; returns its level
    /// and makes it the active frame.
    pub fn push(&mut self, ns: NsId) -> usize {
        let level = self.active_level + 1;
        let mut f = Frame::new(level, ns);
        f.is_proc = true;
        f.saved_active = self.active_level;
        self.frames.push(f);
        self.active_level = level;
        level
    }

    /// Push a proc call frame that shares the *current* level (it still gets its
    /// own local-variable table). Used for a TclOO method invoked via `next`:
    /// the whole call chain runs at the level of the original invocation, so
    /// `info level` / `upvar` / `uplevel` resolve through the chain. Returns the
    /// (unchanged) level; the new frame becomes active and, being topmost at
    /// that level, is what unqualified names and `info level` resolve to.
    pub fn push_same_level(&mut self, ns: NsId) -> usize {
        let level = self.active_level;
        let mut f = Frame::new(level, ns);
        f.is_proc = true;
        f.saved_active = self.active_level;
        self.frames.push(f);
        self.active_level = level;
        level
    }

    /// Push a *namespace* frame (`namespace eval`/`inscope`): a new scope whose
    /// unqualified variables resolve to the namespace (not frame-local). Returns
    /// its level and makes it active.
    pub fn push_namespace(&mut self, ns: NsId) -> usize {
        let level = self.active_level + 1;
        let mut f = Frame::new(level, ns); // is_proc = false
        f.saved_active = self.active_level;
        self.frames.push(f);
        self.active_level = level;
        level
    }

    /// Pop the current (top) frame, releasing its locals (via `VarTable::drop`),
    /// and restore the active level to whatever was in effect when it was pushed
    /// (correct even under an `uplevel` redirection). The global level is never
    /// popped.
    pub fn pop(&mut self) {
        if self.frames.len() > 1 {
            let f = self.frames.pop().expect("non-global frame");
            self.active_level = f.saved_active;
        }
    }

    /// The stack index of the topmost frame with logical `level` (levels are not
    /// stack indices once `uplevel` is in play; the most recent wins).
    fn index_of_level(&self, level: usize) -> Option<usize> {
        self.frames.iter().rposition(|f| f.level == level)
    }

    /// The stack index of the current active (var) frame — the identity a new
    /// `CmdFrame` records as its CallFrame (C's `framePtr->framePtr`).
    pub(crate) fn current_frame_index(&self) -> usize {
        self.index_of_level(self.active_level).unwrap_or(0)
    }

    /// The set of stack indices on the active frame's caller chain (C's
    /// `callerVarPtr` walk from `varFramePtr`). Each frame's caller is the topmost
    /// frame *below* it at the level that was active when it was pushed
    /// (`saved_active`) — so an `uplevel`-redirected call's chain skips the frame
    /// it bypassed, even when that frame shares a level. Used by `info frame` to
    /// decide whether to report a frame's `level`.
    pub(crate) fn caller_chain_indices(&self) -> std::collections::HashSet<usize> {
        let mut set = std::collections::HashSet::new();
        let mut idx = self.current_frame_index();
        loop {
            if !set.insert(idx) || idx == 0 {
                break;
            }
            let caller_level = self.frames[idx].saved_active;
            match self.frames[..idx]
                .iter()
                .rposition(|f| f.level == caller_level)
            {
                Some(c) => idx = c,
                None => break,
            }
        }
        set
    }

    /// Redirect the active variable frame to `level` (for `uplevel`), returning
    /// the previous active level so the caller can restore it.
    pub fn set_active_level(&mut self, level: usize) -> usize {
        let prev = self.active_level;
        self.active_level = level;
        prev
    }

    /// Record the invoking command words on the top frame (`info level N`).
    pub(crate) fn set_words(&mut self, words: Vec<Vec<u8>>) {
        if let Some(f) = self.frames.last_mut() {
            f.words = words;
        }
    }

    /// The command words that invoked frame `level` (`info level N`); empty if
    /// the level has none (e.g. the global frame).
    pub(crate) fn words_at(&self, level: usize) -> Option<&[Vec<u8>]> {
        let i = self.index_of_level(level)?;
        Some(self.frames[i].words.as_slice())
    }

    /// The namespace frame `level` runs in (for `uplevel` ns restoration).
    pub fn frame_ns(&self, level: usize) -> NsId {
        self.index_of_level(level)
            .map_or(crate::namespace::GLOBAL, |i| self.frames[i].ns)
    }

    /// Local variable names of the active frame, sorted (`info locals`).
    pub(crate) fn local_names(&self) -> Vec<Vec<u8>> {
        self.index_of_level(self.active_level)
            .map(|i| {
                self.frames[i]
                    .table
                    .names()
                    .into_iter()
                    .map(<[u8]>::to_vec)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// `info locals` — the active frame's true local variables (no links).
    pub(crate) fn local_names_no_links(&self) -> Vec<Vec<u8>> {
        self.index_of_level(self.active_level)
            .map(|i| {
                self.frames[i]
                    .table
                    .non_link_names()
                    .into_iter()
                    .map(<[u8]>::to_vec)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The variable table at `level` (read).
    pub(crate) fn table(&self, level: usize) -> Option<&VarTable> {
        let i = self.index_of_level(level)?;
        Some(&self.frames[i].table)
    }

    /// The variable table at `level` (mutable).
    pub(crate) fn table_mut(&mut self, level: usize) -> Option<&mut VarTable> {
        let i = self.index_of_level(level)?;
        Some(&mut self.frames[i].table)
    }
}

/// Split `a(b)` into (`a`, `Some(b)`); a plain name yields (`name`, `None`).
/// The command layer uses this to route `set a(k)` / `unset a(k)` to the array
/// element ops.
pub(crate) fn split_array_ref(name: &[u8]) -> (Vec<u8>, Option<Vec<u8>>) {
    if name.last() == Some(&b')') {
        if let Some(open) = name.iter().position(|&c| c == b'(') {
            let base = name[..open].to_vec();
            let elem = name[open + 1..name.len() - 1].to_vec();
            return (base, Some(elem));
        }
    }
    (name.to_vec(), None)
}
