//! Call frames + the variable store (T1.3) — a **re-derived** Rust structure.
//!
//! Canonical model: `tclInt.h`'s `Var` is a tagged union
//! `{ scalar objPtr | array tablePtr | linkPtr }` held in a per-frame hash
//! table (`tmp/tcl9.0.3/generic/tclVar.c`). The Zig PoC encodes this as i32
//! handles with sentinels (`ALIAS_GLOBAL = -1`, negated heap addresses) — a
//! handle-world artifact. The Rust port uses the union directly as a [`Var`]
//! enum, in a [`FrameStack`] of [`Frame`]s.
//!
//! Representation decisions (see `rust-runtime-port.md` T1.3):
//! - **`BTreeMap`** (not `HashMap`) for both the var-name table and array
//!   elements — deterministic iteration (`std::HashMap`'s `RandomState` would
//!   make `info vars` / `array names` vary run-to-run, poison for an
//!   oracle-diffed port) and zero external deps.
//! - **Links resolved by path** (`{level, name, elem}`), not Tcl's direct
//!   `linkPtr`, so a target map reallocating can't dangle a pointer.
//! - **Explicit release, no `Drop` on `Var`/`Frame`** — matches `TclFreeVar`
//!   and keeps every refcount move visible to the leak counters
//!   (`crate::counters`). [`FrameStack`] *does* impl `Drop` (releases all) so a
//!   dropped stack never leaks.
//!
//! Refcount discipline (`memory-management.md` MM-B, `refcount-contract.md`):
//! a scalar cell / array element owns **+1** of its object; storing retains,
//! overwriting/unsetting/popping releases. Links own nothing.
//!
//! This module is the interpreter-fallback frame; the AOT fast path keeps
//! non-escaping locals in WASM locals (S2) and never touches this table. It
//! closes the **variable** half of T1.2's `subst` seam (see
//! [`resolve_var_bytes`]); the command/eval half + the deferred-free queue land
//! with the eval loop (T1.4).

use std::collections::BTreeMap;

use crate::obj::{self, TclObj};

/// A variable cell: the `tclInt.h` `Var` union as an enum.
pub enum Var {
    /// A scalar — owns **+1** of the object.
    Scalar(*mut TclObj),
    /// An associative array: element key → value (each owns **+1**).
    Array(BTreeMap<Vec<u8>, *mut TclObj>),
    /// An `upvar`/`global` alias to another variable (owns nothing).
    Link(Link),
}

/// A path-resolved alias target (an `upvar`/`global` link).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    /// Absolute frame index of the target (`0` = global).
    pub level: usize,
    /// Target variable name.
    pub name: Vec<u8>,
    /// Target array element, for `upvar … a(b) x`.
    pub elem: Option<Vec<u8>>,
}

impl Var {
    /// Release every object reference this var owns. Consumes `self`; call
    /// exactly once when the cell leaves the store.
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

/// Why a variable write failed (the type-mismatch cases Tcl reports).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarError {
    /// `set a v` where `a` is an array (`can't set "a": variable is array`).
    IsArray,
    /// `set a(k) v` where `a` is a scalar (`can't set "a(k)": variable isn't array`).
    IsScalar,
}

/// One call frame: its local variable table and its absolute level.
struct Frame {
    vars: BTreeMap<Vec<u8>, Var>,
    #[allow(dead_code)] // used by info level / upvar level translation (T1.4)
    level: usize,
}

impl Frame {
    fn new(level: usize) -> Self {
        Frame {
            vars: BTreeMap::new(),
            level,
        }
    }
}

/// Where a link chain ultimately lands: a concrete cell (never itself a link).
struct Place {
    frame: usize,
    name: Vec<u8>,
    elem: Option<Vec<u8>>,
}

/// The call-frame stack. `frames[0]` is the global frame; the last frame is the
/// current one. Frame index == level.
pub struct FrameStack {
    frames: Vec<Frame>,
}

impl Default for FrameStack {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameStack {
    /// A new stack with just the global frame (level 0).
    pub fn new() -> Self {
        FrameStack {
            frames: vec![Frame::new(0)],
        }
    }

    /// Current frame index (== level). `0` when only the global frame exists.
    pub fn current_level(&self) -> usize {
        self.frames.len() - 1
    }

    /// Push a new (proc) call frame; returns its level.
    pub fn push(&mut self) -> usize {
        let level = self.frames.len();
        self.frames.push(Frame::new(level));
        level
    }

    /// Pop the current frame, releasing every object its locals own. The global
    /// frame is never popped.
    pub fn pop(&mut self) {
        if self.frames.len() <= 1 {
            return;
        }
        let frame = self.frames.pop().expect("non-global frame");
        release_frame(frame);
    }

    // -- link resolution ------------------------------------------------------

    /// Follow `upvar`/`global` links from `(frame, name)` to the concrete cell
    /// they ultimately address. Stops at a non-link var or an absent name.
    fn resolve(&self, frame: usize, name: &[u8]) -> Place {
        let mut place = Place {
            frame,
            name: name.to_vec(),
            elem: None,
        };
        // Bound the walk so a pathological upvar cycle can't spin forever.
        for _ in 0..1000 {
            match self
                .frames
                .get(place.frame)
                .and_then(|f| f.vars.get(&place.name))
            {
                Some(Var::Link(l)) => {
                    let elem = match (place.elem.take(), l.elem.clone()) {
                        (None, e) => e,
                        // An element-on-element chain (`a(b)(c)`) is invalid; keep
                        // the outer element and stop chaining elements.
                        (Some(e), _) => Some(e),
                    };
                    place = Place {
                        frame: l.level,
                        name: l.name.clone(),
                        elem,
                    };
                }
                _ => break,
            }
        }
        place
    }

    // -- scalar variables -----------------------------------------------------

    /// `set name value` (link-following). The cell takes a **+1** on `obj`.
    pub fn set(&mut self, name: &[u8], obj: *mut TclObj) -> Result<(), VarError> {
        let place = self.resolve(self.current_level(), name);
        match place.elem {
            Some(elem) => self.store_elem(place.frame, &place.name, &elem, obj),
            None => self.store_scalar(place.frame, &place.name, obj),
        }
    }

    /// `set name` — the scalar's value, or an array element if `name` resolves
    /// through a link to one. **Borrowed** (the store keeps its +1).
    pub fn get(&self, name: &[u8]) -> Option<*mut TclObj> {
        let place = self.resolve(self.current_level(), name);
        match &place.elem {
            Some(elem) => self.load_elem(place.frame, &place.name, elem),
            None => self.load_scalar(place.frame, &place.name),
        }
    }

    fn store_scalar(
        &mut self,
        frame: usize,
        name: &[u8],
        obj: *mut TclObj,
    ) -> Result<(), VarError> {
        let vars = &mut self.frames[frame].vars;
        match vars.get_mut(name) {
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
            Some(Var::Link(_)) => unreachable!("resolve() never lands on a link"),
            None => {
                // SAFETY: fresh cell takes a +1.
                unsafe { obj::incr_ref_count(obj) };
                vars.insert(name.to_vec(), Var::Scalar(obj));
                Ok(())
            }
        }
    }

    fn load_scalar(&self, frame: usize, name: &[u8]) -> Option<*mut TclObj> {
        match self.frames.get(frame)?.vars.get(name) {
            Some(Var::Scalar(p)) => Some(*p),
            _ => None,
        }
    }

    // -- array elements -------------------------------------------------------

    /// `set name(key) value`. Errors if `name` is a scalar.
    pub fn set_elem(&mut self, name: &[u8], key: &[u8], obj: *mut TclObj) -> Result<(), VarError> {
        let place = self.resolve(self.current_level(), name);
        if place.elem.is_some() {
            // `a(b)(c)` — the resolved place is already an element.
            return Err(VarError::IsScalar);
        }
        self.store_elem(place.frame, &place.name, key, obj)
    }

    /// `set name(key)` — borrowed.
    pub fn get_elem(&self, name: &[u8], key: &[u8]) -> Option<*mut TclObj> {
        let place = self.resolve(self.current_level(), name);
        if place.elem.is_some() {
            return None;
        }
        self.load_elem(place.frame, &place.name, key)
    }

    fn store_elem(
        &mut self,
        frame: usize,
        name: &[u8],
        key: &[u8],
        obj: *mut TclObj,
    ) -> Result<(), VarError> {
        let vars = &mut self.frames[frame].vars;
        match vars.get_mut(name) {
            Some(Var::Scalar(_)) => Err(VarError::IsScalar),
            Some(Var::Array(map)) => {
                // SAFETY: retain the new element value; release any prior one.
                unsafe { obj::incr_ref_count(obj) };
                if let Some(old) = map.insert(key.to_vec(), obj) {
                    unsafe { obj::decr_ref_count(old) };
                }
                Ok(())
            }
            Some(Var::Link(_)) => unreachable!("resolve() never lands on a link"),
            None => {
                let mut map = BTreeMap::new();
                unsafe { obj::incr_ref_count(obj) };
                map.insert(key.to_vec(), obj);
                vars.insert(name.to_vec(), Var::Array(map));
                Ok(())
            }
        }
    }

    fn load_elem(&self, frame: usize, name: &[u8], key: &[u8]) -> Option<*mut TclObj> {
        match self.frames.get(frame)?.vars.get(name) {
            Some(Var::Array(map)) => map.get(key).copied(),
            _ => None,
        }
    }

    // -- existence / unset ----------------------------------------------------

    /// Whether `name` (scalar or, through a link, an element) is set.
    pub fn exists(&self, name: &[u8]) -> bool {
        let place = self.resolve(self.current_level(), name);
        match &place.elem {
            Some(elem) => self.load_elem(place.frame, &place.name, elem).is_some(),
            None => matches!(
                self.frames
                    .get(place.frame)
                    .and_then(|f| f.vars.get(&place.name)),
                Some(Var::Scalar(_)) | Some(Var::Array(_))
            ),
        }
    }

    /// Whether `name(key)` is set.
    pub fn exists_elem(&self, name: &[u8], key: &[u8]) -> bool {
        self.get_elem(name, key).is_some()
    }

    /// `unset name` — removes the variable the name resolves to (following
    /// links, like Tcl). Returns whether it existed. Releases owned objects.
    pub fn unset(&mut self, name: &[u8]) -> bool {
        let place = self.resolve(self.current_level(), name);
        match place.elem {
            Some(elem) => {
                let vars = &mut self.frames[place.frame].vars;
                if let Some(Var::Array(map)) = vars.get_mut(&place.name) {
                    if let Some(old) = map.remove(&elem) {
                        unsafe { obj::decr_ref_count(old) };
                        return true;
                    }
                }
                false
            }
            None => {
                let removed = self.frames[place.frame].vars.remove(&place.name);
                match removed {
                    Some(v) => {
                        v.release();
                        true
                    }
                    None => false,
                }
            }
        }
    }

    /// `unset name(key)` — remove one array element. Returns whether it existed.
    pub fn unset_elem(&mut self, name: &[u8], key: &[u8]) -> bool {
        let place = self.resolve(self.current_level(), name);
        if place.elem.is_some() {
            return false; // `a(b)(c)` is not a thing
        }
        if let Some(Var::Array(map)) = self.frames[place.frame].vars.get_mut(&place.name) {
            if let Some(old) = map.remove(key) {
                // SAFETY: the array element owned a +1; releasing balances it.
                unsafe { obj::decr_ref_count(old) };
                return true;
            }
        }
        false
    }

    // -- links ----------------------------------------------------------------

    /// `global name` — alias `name` in the current frame to the same-named
    /// global. Overwrites any existing local of that name (releasing it).
    pub fn make_global(&mut self, name: &[u8]) {
        let link = Var::Link(Link {
            level: 0,
            name: name.to_vec(),
            elem: None,
        });
        self.replace_cell(self.current_level(), name, link);
    }

    /// `upvar` — alias `my_name` in the current frame to `target` at
    /// `target_level`. `target` may be an array element `a(b)`.
    pub fn make_upvar(&mut self, target_level: usize, target: &[u8], my_name: &[u8]) {
        let (name, elem) = split_array_ref(target);
        let link = Var::Link(Link {
            level: target_level,
            name,
            elem,
        });
        self.replace_cell(self.current_level(), my_name, link);
    }

    fn replace_cell(&mut self, frame: usize, name: &[u8], var: Var) {
        if let Some(old) = self.frames[frame].vars.insert(name.to_vec(), var) {
            old.release();
        }
    }

    // -- enumeration ----------------------------------------------------------

    /// Names of variables defined in the current frame (`info locals`/`vars`).
    /// Sorted (BTreeMap order) — deterministic.
    pub fn var_names(&self) -> Vec<&[u8]> {
        self.frames
            .last()
            .map(|f| f.vars.keys().map(|k| k.as_slice()).collect())
            .unwrap_or_default()
    }

    /// Whether `name` is an array variable in the current frame (`array exists`).
    pub fn is_array(&self, name: &[u8]) -> bool {
        let place = self.resolve(self.current_level(), name);
        matches!(
            self.frames
                .get(place.frame)
                .and_then(|f| f.vars.get(&place.name)),
            Some(Var::Array(_))
        )
    }

    /// Element names of an array (`array names`), sorted. `None` if not an array.
    pub fn array_names(&self, name: &[u8]) -> Option<Vec<&[u8]>> {
        let place = self.resolve(self.current_level(), name);
        match self.frames.get(place.frame)?.vars.get(&place.name) {
            Some(Var::Array(map)) => Some(map.keys().map(|k| k.as_slice()).collect()),
            _ => None,
        }
    }

    // -- subst glue (closes T1.2's variable seam) -----------------------------

    /// Resolve a variable reference to its string bytes — the closure
    /// [`crate::subst::resolve_with`] needs for `$var` / `$arr(idx)`. Returns
    /// `None` for an unset variable (the eval loop turns that into an error).
    pub fn resolve_var_bytes(&self, name: &[u8], index: Option<&[u8]>) -> Option<Vec<u8>> {
        let obj = match index {
            Some(key) => self.get_elem(name, key)?,
            None => self.get(name)?,
        };
        // SAFETY: `obj` is a live, store-owned object; `get_string` reads/shims
        // its string rep and returns a borrowed pointer we copy out immediately.
        unsafe {
            let mut len: obj::TclSize = 0;
            let p = obj::get_string(obj, &mut len);
            if p.is_null() {
                return Some(Vec::new());
            }
            Some(std::slice::from_raw_parts(p as *const u8, len as usize).to_vec())
        }
    }
}

impl Drop for FrameStack {
    /// Release every object every frame owns, so a dropped stack never leaks.
    fn drop(&mut self) {
        for frame in std::mem::take(&mut self.frames) {
            release_frame(frame);
        }
    }
}

fn release_frame(frame: Frame) {
    for (_, var) in frame.vars {
        var.release();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capi::Tcl_NewStringObj;
    use crate::counters;

    fn sobj(s: &[u8]) -> *mut TclObj {
        // fresh_zero string obj; the store takes the owning +1
        unsafe { Tcl_NewStringObj(s.as_ptr() as *const std::ffi::c_char, s.len() as isize) }
    }

    /// Run `body` against a fresh stack, then assert zero residual once the
    /// stack is dropped — the T1.3 leak gate.
    fn leak_free(body: impl FnOnce(&mut FrameStack)) {
        counters::reset();
        {
            let mut st = FrameStack::new();
            body(&mut st);
        } // FrameStack::drop releases everything
        assert_eq!(
            counters::finalize(),
            0,
            "residual: {} objs, {} bufs",
            counters::live_objs(),
            counters::live_bufs()
        );
        assert_eq!(counters::double_free_count(), 0);
    }

    fn as_str(p: Option<*mut TclObj>) -> Option<Vec<u8>> {
        p.map(|obj| unsafe {
            let mut len: obj::TclSize = 0;
            let s = obj::get_string(obj, &mut len);
            std::slice::from_raw_parts(s as *const u8, len as usize).to_vec()
        })
    }

    #[test]
    fn scalar_set_get_overwrite_unset() {
        leak_free(|st| {
            st.set(b"x", sobj(b"hello")).unwrap();
            assert_eq!(as_str(st.get(b"x")), Some(b"hello".to_vec()));
            // overwrite releases the old value
            st.set(b"x", sobj(b"world")).unwrap();
            assert_eq!(as_str(st.get(b"x")), Some(b"world".to_vec()));
            assert!(st.exists(b"x"));
            assert!(st.unset(b"x"));
            assert!(!st.exists(b"x"));
            assert_eq!(st.get(b"x"), None);
        });
    }

    #[test]
    fn array_elements() {
        leak_free(|st| {
            st.set_elem(b"a", b"k1", sobj(b"v1")).unwrap();
            st.set_elem(b"a", b"k2", sobj(b"v2")).unwrap();
            assert_eq!(as_str(st.get_elem(b"a", b"k1")), Some(b"v1".to_vec()));
            assert!(st.is_array(b"a"));
            assert_eq!(st.array_names(b"a"), Some(vec![&b"k1"[..], b"k2"])); // sorted
                                                                             // Setting a scalar over an array errors. `set` does NOT take
                                                                             // ownership on the error path (it only retains on success), so the
                                                                             // caller still owns the obj it passed and must free it — the
                                                                             // borrowed-on-error contract.
            let rejected = sobj(b"scalar");
            unsafe { obj::incr_ref_count(rejected) }; // caller owns +1
            assert_eq!(st.set(b"a", rejected), Err(VarError::IsArray));
            unsafe { obj::decr_ref_count(rejected) }; // caller frees; set didn't keep it
            assert!(st.unset(b"a"));
        });
    }

    #[test]
    fn global_link_from_proc_frame() {
        leak_free(|st| {
            st.set(b"g", sobj(b"global-val")).unwrap(); // in global frame
            st.push(); // enter a proc frame
            assert_eq!(st.get(b"g"), None); // not visible without `global`
            st.make_global(b"g");
            assert_eq!(as_str(st.get(b"g")), Some(b"global-val".to_vec()));
            // writing through the link updates the global
            st.set(b"g", sobj(b"updated")).unwrap();
            st.pop(); // leave proc frame
            assert_eq!(as_str(st.get(b"g")), Some(b"updated".to_vec()));
            st.unset(b"g");
        });
    }

    #[test]
    fn upvar_link_to_caller() {
        leak_free(|st| {
            st.set(b"caller_var", sobj(b"42")).unwrap(); // level 0
            st.push(); // level 1
            st.make_upvar(0, b"caller_var", b"local"); // upvar #0 caller_var local
            assert_eq!(as_str(st.get(b"local")), Some(b"42".to_vec()));
            st.set(b"local", sobj(b"99")).unwrap(); // writes through to caller_var
            st.pop();
            assert_eq!(as_str(st.get(b"caller_var")), Some(b"99".to_vec()));
            st.unset(b"caller_var");
        });
    }

    #[test]
    fn upvar_to_array_element() {
        leak_free(|st| {
            st.set_elem(b"arr", b"key", sobj(b"elem-val")).unwrap();
            st.push();
            st.make_upvar(0, b"arr(key)", b"e");
            assert_eq!(as_str(st.get(b"e")), Some(b"elem-val".to_vec()));
            st.set(b"e", sobj(b"changed")).unwrap();
            st.pop();
            assert_eq!(
                as_str(st.get_elem(b"arr", b"key")),
                Some(b"changed".to_vec())
            );
            st.unset(b"arr");
        });
    }

    #[test]
    fn pop_releases_locals() {
        // The whole point: a proc frame's locals are freed on pop, no leak.
        leak_free(|st| {
            st.push();
            st.set(b"loc1", sobj(b"a")).unwrap();
            st.set_elem(b"loc2", b"k", sobj(b"b")).unwrap();
            st.pop(); // releases loc1 + loc2's element
        });
    }

    #[test]
    fn subst_resolves_real_frame_vars() {
        // Closes T1.2's variable seam: subst over a real frame store.
        use crate::subst::{resolve_with, scan, SubstFlags};
        leak_free(|st| {
            st.set(b"name", sobj(b"world")).unwrap();
            st.set_elem(b"a", b"k", sobj(b"42")).unwrap();
            let var = |n: &[u8], idx: Option<&[u8]>| st.resolve_var_bytes(n, idx);
            let cmd = |_: &[u8]| None; // command subst arrives with the eval loop (T1.4)
            let f = SubstFlags {
                vars: true,
                cmds: false,
                backslashes: true,
            };
            let body = scan(b"hi $name, a=$a(k)\\n", f);
            assert_eq!(resolve_with(&body, &var, &cmd), b"hi world, a=42\n");
            st.unset(b"name");
            st.unset(b"a");
        });
    }
}
