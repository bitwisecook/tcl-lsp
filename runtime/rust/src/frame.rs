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
//! Representation decisions (see `rust-runtime-port.md` T1.3 / namespace-tree.md
//! §5.3):
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
}

impl VarTable {
    /// The cell bound to `name`, if any (for link inspection / introspection).
    pub(crate) fn cell(&self, name: &[u8]) -> Option<&Var> {
        self.vars.get(name)
    }

    /// `set name value` into this table directly. The cell takes a **+1**.
    pub(crate) fn store_scalar(&mut self, name: &[u8], obj: *mut TclObj) -> Result<(), VarError> {
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
        match self.vars.remove(name) {
            Some(v) => {
                v.release();
                true
            }
            None => false,
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
    /// diagnostic; `array exists` later).
    pub(crate) fn is_array(&self, name: &[u8]) -> bool {
        matches!(self.vars.get(name), Some(Var::Array(_)))
    }
}

impl Drop for VarTable {
    /// Release every object every cell owns, so a dropped frame/namespace never
    /// leaks.
    fn drop(&mut self) {
        for (_, var) in std::mem::take(&mut self.vars) {
            var.release();
        }
    }
}

// ---------------------------------------------------------------------------
// FrameStack — the proc call frames (level 0 is the global context).
// ---------------------------------------------------------------------------

/// One call frame: its local variable table and its absolute level.
struct Frame {
    table: VarTable,
    #[allow(dead_code)] // used by info level / upvar level translation (procs)
    level: usize,
}

impl Frame {
    fn new(level: usize) -> Self {
        Frame {
            table: VarTable::default(),
            level,
        }
    }
}

/// The call-frame stack. `frames[0]` is the global *level* (its own table stays
/// empty — global variables live in the global **namespace's** table, which the
/// coordinator routes to); the last frame is the current one. Frame index ==
/// level. Procs (later) push real local frames.
pub struct FrameStack {
    frames: Vec<Frame>,
}

impl Default for FrameStack {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameStack {
    /// A new stack with just the global level (0).
    pub fn new() -> Self {
        FrameStack {
            frames: vec![Frame::new(0)],
        }
    }

    /// Current frame index (== level). `0` at the global level.
    pub fn current_level(&self) -> usize {
        self.frames.len() - 1
    }

    /// Whether a proc call frame is active (i.e. we are *not* at the global /
    /// namespace-eval level). Unqualified names resolve frame-local only here.
    pub fn in_proc(&self) -> bool {
        self.frames.len() > 1
    }

    /// Push a new (proc) call frame; returns its level.
    pub fn push(&mut self) -> usize {
        let level = self.frames.len();
        self.frames.push(Frame::new(level));
        level
    }

    /// Pop the current frame, releasing every object its locals own (via
    /// `VarTable::drop`). The global level is never popped.
    pub fn pop(&mut self) {
        if self.frames.len() > 1 {
            self.frames.pop();
        }
    }

    /// The variable table at `level` (read).
    pub(crate) fn table(&self, level: usize) -> Option<&VarTable> {
        self.frames.get(level).map(|f| &f.table)
    }

    /// The variable table at `level` (mutable).
    pub(crate) fn table_mut(&mut self, level: usize) -> Option<&mut VarTable> {
        self.frames.get_mut(level).map(|f| &mut f.table)
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
