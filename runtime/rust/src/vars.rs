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
        (current_home(frames, current_ns), name.to_vec())
    };
    Resolved::Place(Place {
        home,
        name: key,
        elem: None,
    })
}

/// Classify then follow `global`/`variable`/`upvar` links to the concrete cell.
fn resolve(frames: &FrameStack, ns: &Namespaces, current_ns: NsId, name: &[u8]) -> Resolved {
    let mut place = match classify(frames, ns, current_ns, name) {
        Resolved::Place(p) => p,
        other => return other,
    };
    for _ in 0..LINK_LIMIT {
        let link = match table(frames, ns, place.home).and_then(|t| t.cell(&place.name)) {
            Some(Var::Link(l)) => l.clone(),
            _ => break,
        };
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
    Resolved::Place(place)
}

/// The namespace a (possibly linked) variable ultimately lives in — `Some(ns)`
/// if `name` resolves (following `global`/`variable`/`upvar` links) to a
/// namespace variable, `None` if it lives in a proc-call frame. Used to scope
/// variable traces to the concrete variable they were registered on.
pub(crate) fn home_namespace(
    frames: &FrameStack,
    ns: &Namespaces,
    current_ns: NsId,
    name: &[u8],
) -> Option<NsId> {
    match resolve(frames, ns, current_ns, name) {
        Resolved::Place(p) => match p.home {
            VarHome::Namespace(id) => Some(id),
            VarHome::Frame(_) => None,
        },
        Resolved::NoNamespace => None,
    }
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
            table(frames, ns, p.home)?.load_elem(&p.name, key)
        }
        _ => None,
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
    get_elem(frames, ns, current_ns, name, key).is_some()
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
            unsafe { obj::decr_ref_count(rejected) }; // caller frees; set didn't keep it
                                                      // a read of the same name simply misses (no such variable).
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
