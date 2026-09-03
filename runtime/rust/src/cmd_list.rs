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

//! List commands (T1.6) — `list` / `llength` / `lindex` / `lappend` / `lrange`
//! / `lreverse` / `concat` / `join` / `split` / `lassign` / `lrepeat` /
//! `linsert` / `lreplace` / `lset` / `ledit` / `lsearch` / `lsort`, over the
//! [`crate::list`] value type.
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use tcl_cmd_core::list as list_core;

use crate::interp::{obj_bytes, Code, Interp};
use crate::list;
use crate::obj::{self, TclObj};

/// Map a portable `tcl-cmd-core` result onto the runtime's set-result/`Code` ABI.
/// A fresh result object (rc 0) is retained by `set_result`; a borrowed element
/// (e.g. `lindex`) is retained too, its parent list keeping its own ref.
fn adapt(interp: &mut Interp, result: Result<*mut TclObj, tcl_cmd_core::CmdError>) -> Code {
    match result {
        Ok(v) => {
            interp.set_result(v);
            Code::Ok
        }
        Err(e) => interp.set_error(e.message().as_bytes()),
    }
}

/// Register the list commands.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"list", list_cmd);
    interp.register_builtin(b"llength", llength);
    interp.register_builtin(b"lindex", lindex);
    interp.register_builtin(b"lappend", lappend);
    interp.register_builtin(b"lrange", lrange);
    interp.register_builtin(b"lreverse", lreverse);
    interp.register_builtin(b"concat", concat);
    interp.register_builtin(b"join", join);
    interp.register_builtin(b"split", split);
    interp.register_builtin(b"lassign", lassign);
    interp.register_builtin(b"lrepeat", lrepeat);
    interp.register_builtin(b"linsert", linsert);
    interp.register_builtin(b"lreplace", lreplace);
    interp.register_builtin(b"lset", lset);
    interp.register_builtin(b"ledit", ledit);
    interp.register_builtin(b"lpop", lpop);
    interp.register_builtin(b"lremove", lremove);
    interp.register_builtin(b"lsearch", lsearch);
    interp.register_builtin(b"lsort", lsort);
}

// -- helpers ---------------------------------------------------------------

/// Set the result to a list built from element objects (each retained).
fn set_list(interp: &mut Interp, elems: &[*mut TclObj]) {
    interp.set_result(list::new_list_obj(elems));
}

/// Resolve a Tcl list index spec against a container of `len` elements via the
/// shared, radix-aware [`tcl_cmd_core::index`] core — the same parser `lindex`
/// / `lrange` / `lreplace` / `linsert` use — so a hex index like `0x1` or
/// `end-0x1` resolves the way real Tcl's `Tcl_GetIntForIndex` does instead of
/// being rejected by a decimal-only reader. Returns a
/// (possibly out-of-range) signed index; callers clamp/range-check.
pub(crate) fn index_spec(spec: &[u8], len: usize) -> Option<isize> {
    let s = core::str::from_utf8(spec).ok()?;
    let v = tcl_cmd_core::index::resolve_opt(s, len)?;
    isize::try_from(v).ok()
}

// -- commands --------------------------------------------------------------

/// `list ?arg ...?` — a list of its arguments.
fn list_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let v = list_core::list(interp, &argv[1..]);
    interp.set_result(v);
    Code::Ok
}

/// `llength list`.
fn llength(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 2 {
        return interp.wrong_args(b"llength list");
    }
    let r = list_core::llength(interp, &argv[1]);
    adapt(interp, r)
}

/// `lindex list ?index ...?` — drill into a (nested) list. With no index the
/// whole list is returned; a single index argument is itself split into an
/// index *path* (so `lindex {{a b} c} {0 1}` works); multiple index arguments
/// each step one level. An out-of-range step yields the empty string. Mirrors
/// `Tcl_LindexObjCmd` (`TclLindexList`/`TclLindexFlat`).
fn lindex(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args(b"lindex list ?index ...?");
    }
    let r = list_core::lindex(interp, &argv[1], &argv[2..]);
    adapt(interp, r)
}

/// `lappend varName ?value ...?` — append to the list in `varName` (creating it
/// if unset), copy-on-write if the value is shared. Returns the new list.
pub(crate) fn lappend(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args(b"lappend varName ?value ...?");
    }
    let name = obj_bytes(argv[1]);
    let values = &argv[2..];
    // `lappend a(k) ...` must address the array element, not a scalar literally
    // named `a(k)` — split the array ref like `set`/`incr` do.
    let (base, elem) = crate::frame::split_array_ref(&name);

    // `lappend x` with no values is a read: validate the current value as a list
    // (erroring on a malformed one, like tclsh) and return it *unchanged* — no
    // store, no trace, no re-rendering. An unset variable is created as an empty
    // list.
    if values.is_empty() {
        // `lappend` fires a read trace on the variable it reads (restored in Tcl
        // 8.4 after 8.0 dropped it), but swallows a trace error, unlike `append`
        // (append-7.2/7.3/7.4, bug 3057639).
        let cur = interp.lappend_read(&base, elem.as_deref());
        return match cur {
            Some(o) => match list::list_elements(o) {
                Ok(_) => {
                    interp.set_result(o);
                    Code::Ok
                }
                Err(e) => bad_list(interp, e),
            },
            None => {
                let empty = list::new_list_obj(&[]); // rc 0
                match interp.store_var_result(&base, elem.as_deref(), empty) {
                    Ok(()) => Code::Ok,
                    Err(e) => crate::builtins::var_error(interp, &name, e),
                }
            }
        };
    }

    // `lappend` reads the current value (to append to it), firing the read trace
    // first — before the write, matching C's get-then-set order — and swallowing
    // a trace error (a missing element is then created; bug 3057639, append-9.0).
    let cur = interp.lappend_read(&base, elem.as_deref());

    // A `lappend` with values writes; reject a constant before the update.
    if let Some(c) = interp.const_write_check(&name) {
        return c;
    }

    // COW-aware list append, shared with the VM via `lappend_value`: it appends in
    // place when the current value is an unshared list (returning that same
    // object), else builds a fresh list. Byte-exact (elements are never
    // stringified).
    let result = match tcl_cmd_core::var::lappend_value(interp, cur, values) {
        Ok(v) => v,
        Err(e) => return interp.set_error(e.message().as_bytes()),
    };

    // Always store back: rebinds the variable (a refcount-neutral re-set when
    // appended in place) and fires the write trace once. `store_var_result`
    // holds a protective reference across the store so a write trace that unsets
    // the variable can't free a fresh `result` before it becomes the result.
    match interp.store_var_result(&base, elem.as_deref(), result) {
        Ok(()) => Code::Ok,
        Err(e) => crate::builtins::var_error(interp, &name, e),
    }
}

/// `lrange list first last` — the sublist from `first` to `last` (inclusive),
/// clamped to range.
fn lrange(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 4 {
        return interp.wrong_args(b"lrange list first last");
    }
    let r = list_core::lrange(interp, &argv[1], &argv[2], &argv[3]);
    adapt(interp, r)
}

/// `lreverse list`.
fn lreverse(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 2 {
        return interp.wrong_args(b"lreverse list");
    }
    let r = list_core::lreverse(interp, &argv[1]);
    adapt(interp, r)
}

/// `concat ?arg ...?` — trim each arg of surrounding whitespace, drop empties,
/// join with single spaces (Tcl's string-level concat).
fn concat(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let v = list_core::concat(interp, &argv[1..]);
    interp.set_result(v);
    Code::Ok
}

/// `join list ?joinString?` — element string reps joined by `joinString`
/// (default a single space).
fn join(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 || argv.len() > 3 {
        return interp.wrong_args(b"join list ?joinString?");
    }
    let sep = if argv.len() == 3 {
        Some(&argv[2])
    } else {
        None
    };
    let r = list_core::join(interp, &argv[1], sep);
    adapt(interp, r)
}

/// `split string ?splitChars?` — split into a list on any byte of `splitChars`
/// (default whitespace). An empty `splitChars` makes each byte an element.
fn split(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 || argv.len() > 3 {
        return interp.wrong_args(b"split string ?splitChars?");
    }
    let chars = if argv.len() == 3 {
        Some(&argv[2])
    } else {
        None
    };
    let v = list_core::split(interp, &argv[1], chars);
    interp.set_result(v);
    Code::Ok
}

/// `lassign list ?varName ...?` — assign successive elements to the vars
/// (missing → empty string); return the unassigned tail as a list.
fn lassign(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args(b"lassign list ?varName ...?");
    }
    let elems = match list::list_elements(argv[1]) {
        Ok(e) => e,
        Err(e) => return bad_list(interp, e),
    };
    let vars = &argv[2..];
    for (i, &var) in vars.iter().enumerate() {
        let name = obj_bytes(var);
        // `arr(a)` writes the array *element*, not a literal scalar named
        // `arr(a)` (issue #1577) — the same `split_array_ref` + `var_set`/
        // `var_set_elem` routing `set`/`lset` already use, so this doesn't
        // hand-roll a second name parser.
        let (base, elem) = crate::frame::split_array_ref(&name);
        let val = if i < elems.len() {
            elems[i]
        } else {
            obj::new_string_bytes(b"")
        };
        let fresh = i >= elems.len();
        let r = match &elem {
            Some(k) => interp.var_set_elem(&base, k, val),
            None => interp.var_set(&base, val),
        };
        if fresh {
            // `set` retained `val`; release our construction ref to the empty obj
            drop_fresh(val);
        }
        if let Err(e) = r {
            return crate::builtins::var_error(interp, &name, e);
        }
    }
    if vars.len() < elems.len() {
        set_list(interp, &elems[vars.len()..]);
    } else {
        interp.set_result_bytes(b"");
    }
    Code::Ok
}

// -- error helpers ---------------------------------------------------------

fn bad_list(interp: &mut Interp, e: crate::parse::ListError) -> Code {
    interp.set_error(e.message())
}

// -- lrepeat / linsert / lreplace / lsearch / lsort ------------------------

/// `lrepeat count ?value ...?` — `count` copies of the value sequence.
///
/// Delegates to the shared, radix-aware [`list_core::lrepeat`] so the count
/// accepts the full Tcl integer grammar (`0x3` → `a a a`), the negative-count
/// error reports the *actual* count (`lrepeat -3 a` → `bad count "-3"…`, not a
/// hard-coded `"-1"`), and the result-capacity multiply is `saturating_mul`
/// rather than an overflowing `count * values.len()`.
fn lrepeat(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args(b"lrepeat count ?value ...?");
    }
    let r = list_core::lrepeat(interp, &argv[1], &argv[2..]);
    adapt(interp, r)
}

/// `linsert list index ?element ...?` — insert before `index` (`end` appends).
fn linsert(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return interp.wrong_args(b"linsert list index ?element ...?");
    }
    let r = list_core::linsert(interp, &argv[1], &argv[2], &argv[3..]);
    adapt(interp, r)
}

/// `lreplace list first last ?element ...?` — replace the `[first,last]` range.
fn lreplace(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 {
        return interp.wrong_args(b"lreplace list first last ?element ...?");
    }
    let r = list_core::lreplace(interp, &argv[1], &argv[2], &argv[3], &argv[4..]);
    adapt(interp, r)
}

/// `ledit listVar first last ?element ...?` — the in-place `lreplace` on a list
/// *variable* (Tcl 8.7/9.0). Reads `listVar`, replaces the `[first,last]` range
/// with the new elements, stores the result back into the variable, and returns
/// the new list value. Mirrors `Tcl_LeditObjCmd` (`tclCmdIL.c`): the variable
/// must already exist (a read miss is `can't read ...: no such variable`), the
/// index clamping is identical to `lreplace`, and `listVar` may name an array
/// element (`a(k)`), addressed like `set`/`lappend`.
/// `index "X" out of range` (the `lset` index range error).
fn lset_out_of_range(interp: &mut Interp, spec: &[u8]) -> Code {
    let mut m = b"index \"".to_vec();
    m.extend_from_slice(spec);
    m.extend_from_slice(b"\" out of range");
    interp.set_error(&m)
}

/// Recursively set the element at the index `path` of `list_obj` to `value`,
/// returning the new (sub)list. Mirrors C's `TclLsetFlat`: each index resolves
/// against its sublist's length (`end`/`end±N` aware), range `0..=len` with
/// `len` appending; an empty `path` returns `value` itself (whole-list replace).
///
/// The returned object is either `value` (borrowed) or a fresh `new_list_obj`
/// (rc 0) — in both cases the caller retains it (the parent via `new_list_obj`,
/// the command via `var_set`/`set_result`), so no extra refcount is taken here.
fn lset_descend(
    interp: &mut Interp,
    list_obj: *mut TclObj,
    path: &[Vec<u8>],
    value: *mut TclObj,
) -> Result<*mut TclObj, Code> {
    let Some((spec, rest)) = path.split_first() else {
        // No (more) indices: [lset] is [set] — the value replaces the list.
        return Ok(value);
    };
    let elems = match list::list_elements(list_obj) {
        Ok(v) => v,
        Err(e) => return Err(bad_list(interp, e)),
    };
    let len = elems.len();
    let Some(idx) = index_spec(spec, len) else {
        return Err(bad_index(interp, spec));
    };
    if idx < 0 || idx as usize > len {
        return Err(lset_out_of_range(interp, spec));
    }
    let idx = idx as usize;
    let appending = idx == len;
    // Descend into the existing element, or a fresh empty list when appending a
    // new (possibly nested) slot.
    let child = if appending {
        list::new_list_obj(&[])
    } else {
        elems[idx]
    };
    let new_child = match lset_descend(interp, child, rest, value) {
        Ok(c) => c,
        Err(e) => {
            if appending {
                drop_fresh(child);
            }
            return Err(e);
        }
    };
    // Rebuild this level with the element replaced (or the new element pushed).
    // `new_list_obj` retains every element, including `new_child`.
    let mut out: Vec<*mut TclObj> = elems;
    if appending {
        out.push(new_child);
    } else {
        out[idx] = new_child;
    }
    let result = list::new_list_obj(&out);
    if appending {
        drop_fresh(child);
    }
    Ok(result)
}

/// `lset listVar ?index ...? value` — set the element at the index path in the
/// list stored in `listVar`, store it back (firing write traces), and return
/// the new list (`Tcl_LsetObjCmd`). A lone index arg is split into an index
/// path (`lset x {1 0} v`); multiple index args are each one index.
fn lset(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return interp.wrong_args(b"lset listVar ?index? ?index ...? value");
    }
    let name = obj_bytes(argv[1]);
    let (base, elem) = crate::frame::split_array_ref(&name);
    let cur = match &elem {
        Some(k) => interp.var_get_elem(&base, k),
        None => interp.var_get(&base),
    };
    let Some(listobj) = cur else {
        let msg = interp.read_miss_msg(&base, elem.as_deref());
        return interp.set_error(&msg);
    };
    let value = argv[argv.len() - 1];
    let idx_args = &argv[2..argv.len() - 1];
    // Build the index path: a lone arg is itself split into a list of indices
    // (matching C's TclLsetList); a malformed list falls back to one index.
    let path: Vec<Vec<u8>> = if idx_args.len() == 1 {
        match crate::parse::split_list(&obj_bytes(idx_args[0])) {
            Ok(p) => p,
            Err(_) => vec![obj_bytes(idx_args[0])],
        }
    } else {
        idx_args.iter().map(|&a| obj_bytes(a)).collect()
    };
    let newlist = match lset_descend(interp, listobj, &path, value) {
        Ok(l) => l,
        Err(c) => return c,
    };
    let stored = match &elem {
        Some(k) => interp.var_set_elem(&base, k, newlist),
        None => interp.var_set(&base, newlist),
    };
    if let Err(e) = stored {
        // `newlist` is fresh (rc 0) unless the path was empty (then it is the
        // borrowed `value`); only the fresh one needs releasing.
        if newlist != value {
            drop_fresh(newlist);
        }
        return crate::builtins::var_error(interp, &name, e);
    }
    interp.set_result(newlist);
    Code::Ok
}

fn ledit(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 {
        return interp.wrong_args(b"ledit listVar first last ?element ...?");
    }
    let name = obj_bytes(argv[1]);
    let (base, elem) = crate::frame::split_array_ref(&name);
    let cur = match &elem {
        Some(k) => interp.var_get_elem(&base, k),
        None => interp.var_get(&base),
    };
    let Some(listobj) = cur else {
        // C reads via `Tcl_ObjGetVar2(..., TCL_LEAVE_ERR_MSG)`: a missing
        // variable is a read error, with the C three-way distinction
        // (variable-is-array / no-such-element / no-such-variable).
        let msg = interp.read_miss_msg(&base, elem.as_deref());
        return interp.set_error(&msg);
    };
    let elems = match list::list_elements(listobj) {
        Ok(v) => v,
        Err(e) => return bad_list(interp, e),
    };
    let len = elems.len();
    let Some(first) = index_spec(&obj_bytes(argv[2]), len) else {
        return bad_index(interp, &obj_bytes(argv[2]));
    };
    let Some(last) = index_spec(&obj_bytes(argv[3]), len) else {
        return bad_index(interp, &obj_bytes(argv[3]));
    };
    let lo = first.max(0).min(len as isize) as usize;
    // `last.saturating_add(1)` — an `end`-relative or explicit index at
    // `isize::MAX` must not overflow the `+ 1` before the clamp.
    let hi = (last.saturating_add(1).max(0) as usize).clamp(lo, len);
    let mut out: Vec<*mut TclObj> = Vec::with_capacity(len + argv.len());
    out.extend_from_slice(&elems[..lo]);
    out.extend_from_slice(&argv[4..]);
    out.extend_from_slice(&elems[hi..]);
    // Build the new list first (retains every element), *then* store it: the
    // store releases the old value, but the elements survive because the new
    // list now holds its own refs. `new_list_obj` is rc 0; `var_set*` retains it.
    let newlist = list::new_list_obj(&out);
    let stored = match &elem {
        Some(k) => interp.var_set_elem(&base, k, newlist),
        None => interp.var_set(&base, newlist),
    };
    if stored.is_err() {
        drop_fresh(newlist);
        let mut m = b"can't set \"".to_vec();
        m.extend_from_slice(&name);
        m.extend_from_slice(b"\": variable is array");
        return interp.set_error(&m);
    }
    interp.set_result(newlist);
    Code::Ok
}

/// `lpop varName ?index?` — remove and return the element at `index` (default
/// the last), storing the shortened list back into the variable. A radix or
/// `end`-relative index resolves via the shared index core.
fn lpop(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args(b"lpop varName ?index?");
    }
    let name = obj_bytes(argv[1]);
    let (base, elem) = crate::frame::split_array_ref(&name);
    let cur = match &elem {
        Some(k) => interp.var_get_elem(&base, k),
        None => interp.var_get(&base),
    };
    let Some(listobj) = cur else {
        let msg = interp.read_miss_msg(&base, elem.as_deref());
        return interp.set_error(&msg);
    };
    let elems = match list::list_elements(listobj) {
        Ok(v) => v,
        Err(e) => return bad_list(interp, e),
    };
    let len = elems.len();
    let (idx, spec_desc): (isize, Vec<u8>) = if argv.len() == 2 {
        (len as isize - 1, b"end".to_vec())
    } else if argv.len() == 3 {
        let spec = obj_bytes(argv[2]);
        match index_spec(&spec, len) {
            Some(i) => (i, spec),
            None => return bad_index(interp, &spec),
        }
    } else {
        // A nested index path drills into sub-lists (rare); not specialised here.
        return interp.set_error(b"lpop with a nested index path is not supported");
    };
    if idx < 0 || idx as usize >= len {
        let mut m = b"index \"".to_vec();
        m.extend_from_slice(&spec_desc);
        m.extend_from_slice(b"\" out of range");
        return interp.set_error(&m);
    }
    let idx = idx as usize;
    let removed = elems[idx];
    let out: Vec<*mut TclObj> = elems
        .iter()
        .enumerate()
        .filter_map(|(i, e)| (i != idx).then_some(*e))
        .collect();
    let newlist = list::new_list_obj(&out); // retains survivors
                                            // Retain `removed` (via the result) *before* the store releases the old
                                            // list, so it survives to be returned.
    interp.set_result(removed);
    let stored = match &elem {
        Some(k) => interp.var_set_elem(&base, k, newlist),
        None => interp.var_set(&base, newlist),
    };
    if stored.is_err() {
        drop_fresh(newlist);
        let mut m = b"can't set \"".to_vec();
        m.extend_from_slice(&name);
        m.extend_from_slice(b"\": variable is array");
        return interp.set_error(&m);
    }
    Code::Ok
}

/// `lremove list ?index ...?` — return `list` with the elements at the given
/// indices removed. Indices resolve (radix + `end`-relative), duplicates
/// collapse, and out-of-range indices are ignored.
fn lremove(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args(b"lremove list ?index ...?");
    }
    let elems = match list::list_elements(argv[1]) {
        Ok(v) => v,
        Err(e) => return bad_list(interp, e),
    };
    let len = elems.len();
    let mut remove = vec![false; len];
    for &iv in &argv[2..] {
        let spec = obj_bytes(iv);
        match index_spec(&spec, len) {
            Some(i) if i >= 0 && (i as usize) < len => remove[i as usize] = true,
            Some(_) => {} // out of range — ignored, as C's lremove does
            None => return bad_index(interp, &spec),
        }
    }
    let out: Vec<*mut TclObj> = elems
        .iter()
        .enumerate()
        .filter_map(|(i, e)| (!remove[i]).then_some(*e))
        .collect();
    set_list(interp, &out);
    Code::Ok
}

/// `lsearch ?-option value ...? list pattern` — a thin adapter over the shared
/// [`tcl_cmd_core::lsearch`] core, driven by the real Tcl ARE engine for the
/// `-regexp` mode. The whole command is a pure value->value function in the
/// core (`lsearch` never writes a variable); this adapter only maps the result
/// onto `set_result` and the error onto `set_error`/`error_with_code`.
fn lsearch(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    match tcl_cmd_core::lsearch::lsearch::<Interp, crate::cmd_regex::AreEngine>(interp, &argv[1..])
    {
        Ok(v) => {
            interp.set_result(v);
            Code::Ok
        }
        Err(e) => match e.code {
            Some(code) => interp.error_with_code(&e.message, code),
            None => interp.set_error(&e.message),
        },
    }
}

/// Parse a Tcl integer (decimal, or `0x`/`0o`/`0b` radix, optional sign) into an
/// `i128` for `-integer` sort keys. `None` if not an integer.
/// Parse a Tcl integer for `-integer` sort/search keys (shared core).
fn parse_wide(b: &[u8]) -> Option<i128> {
    tcl_cmd_core::sort::parse_wide(b)
}

/// `lsort ?-option value ...? list` — sort the list (`Tcl_LsortObjCmd`).
/// `lsort ?-option value ...? list` — a thin adapter over the shared
/// [`tcl_cmd_core::lsort`] core. Non-command modes are sorted+built entirely in
/// the core; `-command` is split (the core prepares, this adapter runs the merge
/// sort over the user comparator via `lsort_cmd_compare`, then the core builds).
fn lsort(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    use tcl_cmd_core::lsort::{build_command, prepare, sort_command, Lsort};
    let job = match prepare(interp, &argv[1..]) {
        Ok(Lsort::Done(v)) => {
            interp.set_result(v);
            return Code::Ok;
        }
        Ok(Lsort::Command(job)) => job,
        Err(e) => return interp.set_error(&e.message),
    };
    // `-command`: pre-split the comparison prefix into words, run the reentrant
    // merge sort over the user comparator (which evaluates Tcl), then build.
    let words = match list::list_elements(job.cmd_prefix) {
        Ok(v) => v.iter().map(|&w| obj_bytes(w)).collect::<Vec<_>>(),
        Err(e) => return bad_list(interp, e),
    };
    let mut job = job;
    if let Err(c) = sort_command(&mut job, |a, b| lsort_cmd_compare(interp, &words, *a, *b)) {
        return c;
    }
    let v = build_command(interp, &job);
    interp.set_result(v);
    Code::Ok
}

/// Evaluate `<prefix words...> a b` and read its integer result (the `-command`
/// comparator). Returns the sign as an `i32`.
fn lsort_cmd_compare(
    interp: &mut Interp,
    words: &[Vec<u8>],
    a: *mut TclObj,
    b: *mut TclObj,
) -> Result<i32, Code> {
    use crate::interp::new_string;
    let mut call: Vec<*mut TclObj> = Vec::with_capacity(words.len() + 2);
    for w in words {
        call.push(new_string(w));
    }
    call.push(a);
    call.push(b);
    for &o in &call {
        unsafe { obj::incr_ref_count(o) };
    }
    let code = interp.dispatch(&call);
    let result = interp.result_bytes();
    for &o in &call {
        unsafe { obj::decr_ref_count(o) };
    }
    if code != Code::Ok {
        return Err(code);
    }
    match parse_wide(&result) {
        Some(v) => Ok(v.signum() as i32),
        None => {
            let mut m = b"-command comparison script returned non-integer result: ".to_vec();
            m.extend_from_slice(&result);
            Err(interp.set_error(&m))
        }
    }
}

fn bad_index(interp: &mut Interp, spec: &[u8]) -> Code {
    let mut m = b"bad index \"".to_vec();
    m.extend_from_slice(spec);
    m.extend_from_slice(b"\": must be integer?[+-]integer? or end?[+-]integer?");
    interp.set_error(&m)
}

/// Free a freshly created (`rc 0`) object not stored anywhere.
fn drop_fresh(obj: *mut TclObj) {
    // SAFETY: `obj` is a live rc-0 object; retain-then-release frees it cleanly.
    unsafe {
        obj::incr_ref_count(obj);
        obj::decr_ref_count(obj);
    }
}

#[cfg(test)]
mod tests {
    use crate::counters;
    use crate::interp::{Code, Interp};

    fn run(src: &[u8]) -> (Code, Vec<u8>) {
        // Returns (code, result-bytes). Leak-checked across the interp lifetime.
        counters::reset();
        let (code, bytes);
        {
            let mut i = Interp::new();
            code = i.eval_str(src);
            bytes = i.result_bytes();
        }
        assert_eq!(
            counters::finalize(),
            0,
            "leak: {} objs {} bufs",
            counters::live_objs(),
            counters::live_bufs()
        );
        assert_eq!(counters::double_free_count(), 0);
        (code, bytes)
    }

    fn ok(src: &[u8]) -> Vec<u8> {
        let (c, b) = run(src);
        assert_eq!(c, Code::Ok, "result={:?}", String::from_utf8_lossy(&b));
        b
    }

    // Needs the numeric tower: `-command` comparators are `expr` lambdas.
    #[cfg(have_tommath)]
    #[test]
    fn lsort_shared_core() {
        // Pinned against tclsh 9.0 (each case leak-checked by `ok`/`run`).
        assert_eq!(ok(b"lsort {b a c}"), b"a b c");
        assert_eq!(ok(b"lsort -decreasing {b a c}"), b"c b a");
        assert_eq!(ok(b"lsort -integer {10 2 33 4}"), b"2 4 10 33");
        assert_eq!(ok(b"lsort -real {1.5 0.5 2.25}"), b"0.5 1.5 2.25");
        assert_eq!(ok(b"lsort -nocase {B a C b}"), b"a B b C");
        assert_eq!(ok(b"lsort -dictionary {x10 x9 x100}"), b"x9 x10 x100");
        assert_eq!(ok(b"lsort -unique {a b a c b}"), b"a b c");
        assert_eq!(ok(b"lsort -integer -unique {1 01 1 2}"), b"1 2");
        assert_eq!(ok(b"lsort -indices {c a b}"), b"1 2 0");
        assert_eq!(
            ok(b"lsort -index 1 {{a 3} {b 1} {c 2}}"),
            b"{b 1} {c 2} {a 3}"
        );
        assert_eq!(ok(b"lsort -index 0 {{b 1} {a 2}}"), b"{a 2} {b 1}");
        assert_eq!(ok(b"lsort -stride 2 {c 3 a 1 b 2}"), b"a 1 b 2 c 3");
        assert_eq!(
            ok(b"lsort -stride 2 -index 1 {x 3 y 1 z 2}"),
            b"y 1 z 2 x 3"
        );
        assert_eq!(
            ok(b"lsort -stride 2 -indices {c 3 a 1 b 2}"),
            b"2 3 4 5 0 1"
        );
        assert_eq!(
            ok(b"lsort -decreasing -dictionary {x9 x10 x100}"),
            b"x100 x10 x9"
        );
        // `-command` (Family-B: the comparator evaluates Tcl).
        assert_eq!(
            ok(b"lsort -command {apply {{a b} {expr {$a - $b}}}} {3 1 2}"),
            b"1 2 3"
        );
        assert_eq!(
            ok(b"lsort -command {apply {{a b} {expr {$b - $a}}}} {3 1 2}"),
            b"3 2 1"
        );
        assert_eq!(
            ok(b"lsort -unique -command {apply {{a b} {expr {$a - $b}}}} {3 1 3 2 1}"),
            b"1 2 3"
        );
        assert_eq!(ok(b"lsort {}"), b"");
        // Errors.
        let (c, b) = run(b"lsort -bogus {a b}");
        assert_eq!(c, Code::Error);
        assert!(b.starts_with(b"bad option \"-bogus\""));
        let (c, b) = run(b"lsort -integer {1 x 3}");
        assert_eq!(c, Code::Error);
        assert_eq!(b, b"expected integer but got \"x\"");
        let (c, b) = run(b"lsort -index 5 {{a b} {c d}}");
        assert_eq!(c, Code::Error);
        assert_eq!(b, b"element 5 missing from sublist \"a b\"");
        let (c, b) = run(b"lsort -stride 3 {a b}");
        assert_eq!(c, Code::Error);
        assert_eq!(b, b"list size must be a multiple of the stride length");
    }

    #[test]
    fn lsearch_shared_core() {
        // Pinned against tclsh 9.0 (every option exercised; each case is also
        // leak-checked by `ok`/`run`).
        assert_eq!(ok(b"lsearch {a b c d} c"), b"2");
        assert_eq!(ok(b"lsearch {a b c d} x"), b"-1");
        assert_eq!(ok(b"lsearch -exact {aa ab ac} ab"), b"1");
        assert_eq!(ok(b"lsearch -all {a b a c a} a"), b"0 2 4");
        assert_eq!(ok(b"lsearch -inline {foo bar baz} ba*"), b"bar");
        assert_eq!(ok(b"lsearch -all -inline {x1 y2 x3} x*"), b"x1 x3");
        assert_eq!(ok(b"lsearch -not {a b a} a"), b"1");
        assert_eq!(ok(b"lsearch -all -not {a b a c} a"), b"1 3");
        assert_eq!(ok(b"lsearch -start 2 {a b a a} a"), b"2");
        assert_eq!(ok(b"lsearch -nocase {AB cd EF} ef"), b"2");
        assert_eq!(ok(b"lsearch -integer {3 1 4 1 5} 4"), b"2");
        assert_eq!(ok(b"lsearch -real {1.5 2.5 3.5} 2.5"), b"1");
        assert_eq!(ok(b"lsearch -sorted {1 3 5 7 9} 7"), b"3");
        assert_eq!(ok(b"lsearch -sorted -integer {1 3 5 7} 5"), b"2");
        assert_eq!(ok(b"lsearch -sorted -decreasing {9 7 5 3 1} 5"), b"2");
        assert_eq!(ok(b"lsearch -bisect -integer {2 4 6 8} 5"), b"1");
        assert_eq!(ok(br"lsearch -regexp {foo123 bar456} {[0-9]+}"), b"0");
        assert_eq!(
            ok(br"lsearch -all -inline -regexp {a1 b2 c3} {\d}"),
            b"a1 b2 c3"
        );
        assert_eq!(ok(b"lsearch -index 1 {{a 1} {b 2} {c 3}} 2"), b"1");
        assert_eq!(ok(b"lsearch -index 0 -inline {{a 1} {b 2}} b"), b"b 2");
        assert_eq!(ok(b"lsearch -all -index 1 {{a 1} {b 2} {c 1}} 1"), b"0 2");
        assert_eq!(ok(b"lsearch -subindices -index 1 {{a 1} {b 2}} 2"), b"1 1");
        assert_eq!(ok(b"lsearch -stride 2 -index 0 {a 1 b 2 c 3} b"), b"2");
        assert_eq!(ok(b"lsearch -stride 2 {a 1 b 2} 2"), b"-1");
        assert_eq!(
            ok(b"lsearch -all -inline -stride 2 {a 1 b 2 c 3} *"),
            b"a 1 b 2 c 3"
        );
        assert_eq!(ok(b"lsearch -exact -sorted {a b c d} c"), b"2");
        assert_eq!(ok(b"lsearch {} x"), b"-1");
        assert_eq!(ok(b"lsearch -integer {1 x 3} 3"), b"2");
        assert_eq!(ok(b"lsearch -index end {{a b} {c d}} d"), b"1");
        // Errors.
        let (c, b) = run(b"lsearch -bogus {a b} a");
        assert_eq!(c, Code::Error);
        assert!(b.starts_with(b"bad option \"-bogus\""));
        let (c, b) = run(b"lsearch -subindices {a b} a");
        assert_eq!(c, Code::Error);
        assert_eq!(b, b"-subindices cannot be used without -index option");
    }

    #[test]
    fn lrepeat_linsert_lreplace() {
        assert_eq!(ok(b"lrepeat 3 a b"), b"a b a b a b");
        assert_eq!(ok(b"lrepeat 0 a"), b"");
        assert_eq!(ok(b"linsert {a b c} end X"), b"a b c X");
        assert_eq!(ok(b"linsert {a b c} 1 X Y"), b"a X Y b c");
        assert_eq!(ok(b"linsert {a b c} 0 X"), b"X a b c");
        assert_eq!(ok(b"lreplace {a b c d} 1 2 X"), b"a X d");
        assert_eq!(ok(b"lreplace {a b c d} 1 2"), b"a d");
        assert_eq!(ok(b"lreplace {a b c} end end Z"), b"a b Z");
        // `first > last` is a pure insertion at `first`.
        assert_eq!(ok(b"lreplace {a b c} 1 0 X"), b"a X b c");
        // Out-of-range indices clamp (no error); `end`/`end±N` offset correctly:
        // past-end appends, negative prepends.
        assert_eq!(ok(b"linsert {a b c} end+1 X"), b"a b c X");
        assert_eq!(ok(b"linsert {a b c} -5 X"), b"X a b c");
        assert_eq!(ok(b"lreplace {a b c} 5 7 X"), b"a b c X");
        // A malformed index spec errors faithfully (shared index parser).
        assert!(err(b"linsert {a b c} foo X").starts_with(b"bad index"));
        assert!(err(b"lreplace {a b c} foo 1 X").starts_with(b"bad index"));
        assert!(err(b"lreplace {a b c} 1 foo X").starts_with(b"bad index"));
    }

    fn err(src: &[u8]) -> Vec<u8> {
        let (c, b) = run(src);
        assert_eq!(
            c,
            Code::Error,
            "expected error, got {:?}",
            String::from_utf8_lossy(&b)
        );
        b
    }

    #[test]
    fn lset_sets_and_appends() {
        // Replace, append, end-relative; updates the variable and returns it.
        assert_eq!(ok(b"set x {a b c}; lset x 1 Z"), b"a Z c");
        assert_eq!(ok(b"set x {a b c}; lset x 1 Z; set x"), b"a Z c");
        assert_eq!(ok(b"set x {a b c}; lset x end Z"), b"a b Z");
        assert_eq!(ok(b"set x {a b c}; lset x 3 Z"), b"a b c Z"); // append at len
                                                                  // No index → whole-list replace (lset is set).
        assert_eq!(ok(b"set x {a b c}; lset x Z"), b"Z");
        assert_eq!(ok(b"set x {a b}; lset x {} Z"), b"Z");
        // Nested: a lone arg is an index path; multiple args each an index.
        assert_eq!(ok(b"set x {{a b} {c d}}; lset x 1 0 Z"), b"{a b} {Z d}");
        assert_eq!(ok(b"set x {{a b} {c d}}; lset x {1 0} Z"), b"{a b} {Z d}");
        assert_eq!(ok(b"set x {a {b c}}; lset x 1 1 Z"), b"a {b Z}");
        // Empty-list quirks (single-element sublists stringify without braces).
        assert_eq!(ok(b"set x {}; lset x 0 0 Z"), b"Z");
        assert_eq!(ok(b"set x {}; lset x end+1 Z"), b"Z");
        // Array-element addressing, like `ledit`/`lappend`.
        assert_eq!(ok(b"set a(k) {1 2 3}; lset a(k) 1 Z; set a(k)"), b"1 Z 3");
        // COW: a shared value isn't mutated through the alias.
        assert_eq!(
            ok(b"set l {a b c}; set m $l; lset l 0 X; list $l $m"),
            b"{X b c} {a b c}"
        );
    }

    #[test]
    fn lset_errors() {
        assert_eq!(
            err(b"set x {a b c}; lset x 5 Z"),
            b"index \"5\" out of range"
        );
        assert_eq!(
            err(b"set x {a b c}; lset x -1 Z"),
            b"index \"-1\" out of range"
        );
        assert_eq!(
            err(b"set x {a b c}; lset x a Z"),
            b"bad index \"a\": must be integer?[+-]integer? or end?[+-]integer?"
        );
        assert_eq!(
            err(b"lset nosuchvar 0 Z"),
            b"can't read \"nosuchvar\": no such variable"
        );
        assert_eq!(
            err(b"lset"),
            b"wrong # args: should be \"lset listVar ?index? ?index ...? value\""
        );
    }

    #[test]
    fn ledit_replaces_in_place() {
        // Returns the new value *and* updates the variable in place.
        assert_eq!(ok(b"set l {1 2 3 4 5}; ledit l 1 1 a"), b"1 a 3 4 5");
        assert_eq!(ok(b"set l {1 2 3 4 5}; ledit l 1 1 a; set l"), b"1 a 3 4 5");
        assert_eq!(ok(b"set l {1 2 3 4 5}; ledit l 1 3; set l"), b"1 5");
        assert_eq!(ok(b"set l {1 2 3}; ledit l 1 0 x y; set l"), b"1 x y 2 3"); // first>last
        assert_eq!(ok(b"set l {a b c d}; ledit l end-1 end Z"), b"a b Z");
        assert_eq!(ok(b"set l {a b}; ledit l end+1 end+1 c"), b"a b c"); // append
                                                                         // Array-element addressing, like `lappend a(k)`.
        assert_eq!(
            ok(b"set a(k) {1 2 3}; ledit a(k) 0 0 X; set a(k)"),
            b"X 2 3"
        );
        // COW: a shared value isn't mutated through the alias.
        assert_eq!(
            ok(b"set l {a b c}; set m $l; ledit l 0 0 X; list $l $m"),
            b"{X b c} {a b c}"
        );
    }

    #[test]
    fn ledit_errors() {
        let (c, b) = run(b"ledit l 0");
        assert_eq!(c, Code::Error);
        assert_eq!(
            b,
            b"wrong # args: should be \"ledit listVar first last ?element ...?\""
        );
        // A wholly missing variable is a read error (C's TCL_LEAVE_ERR_MSG).
        let (c, b) = run(b"ledit nope 0 0 x");
        assert_eq!(c, Code::Error);
        assert_eq!(b, b"can't read \"nope\": no such variable");
        // Missing element of an existing array → "no such element in array".
        let (c, b) = run(b"set arr(y) y; ledit arr(x) 0 0 z");
        assert_eq!(c, Code::Error);
        assert_eq!(b, b"can't read \"arr(x)\": no such element in array");
    }

    #[test]
    fn var_read_miss_three_way() {
        // The C `tclVar.c` distinction, shared by `set`/`ledit`/`expr $var`.
        let (c, b) = run(b"set nope");
        assert_eq!(
            (c, b),
            (
                Code::Error,
                b"can't read \"nope\": no such variable".to_vec()
            )
        );
        let (c, b) = run(b"set arr(y) y; set arr");
        assert_eq!(
            (c, b),
            (
                Code::Error,
                b"can't read \"arr\": variable is array".to_vec()
            )
        );
        let (c, b) = run(b"set arr(y) y; set arr(x)");
        assert_eq!(
            (c, b),
            (
                Code::Error,
                b"can't read \"arr(x)\": no such element in array".to_vec()
            )
        );
    }

    #[test]
    fn lsearch_modes() {
        assert_eq!(ok(b"lsearch {a b c b} b"), b"1");
        assert_eq!(ok(b"lsearch -all {a b c b} b"), b"1 3");
        assert_eq!(ok(b"lsearch {x ab cd} a*"), b"1"); // default glob
        assert_eq!(ok(b"lsearch -exact {x ab cd} ab"), b"1");
        assert_eq!(ok(b"lsearch -inline {one two three} t*"), b"two");
        assert_eq!(ok(b"lsearch {a b c} z"), b"-1");
        // Datatypes, -not, -all -inline.
        assert_eq!(ok(b"lsearch -integer {1 5 3 5} 5"), b"1");
        assert_eq!(ok(b"lsearch -not {a a b a} a"), b"2");
        assert_eq!(ok(b"lsearch -all -inline {a1 b2 a3} a*"), b"a1 a3");
        // -sorted binary search + -bisect.
        assert_eq!(ok(b"lsearch -sorted -integer {1 3 5 7 9} 5"), b"2");
        assert_eq!(ok(b"lsearch -sorted -integer {1 3 5 7 9} 6"), b"-1");
        assert_eq!(ok(b"lsearch -sorted -bisect -integer {1 3 5 7 9} 6"), b"2");
        // -index, -stride, -subindices, -start, -regexp, -dictionary.
        assert_eq!(ok(b"lsearch -index 1 {{a 1} {b 2} {c 3}} 2"), b"1");
        assert_eq!(ok(b"lsearch -stride 2 -index 0 {a 1 b 2 c 3} b"), b"2");
        assert_eq!(ok(b"lsearch -subindices -index 1 {{a 1} {b 2}} 2"), b"1 1");
        assert_eq!(ok(b"lsearch -start 2 {a b a b} a"), b"2");
        assert_eq!(ok(b"lsearch -all -regexp {foo bar baz} {^ba}"), b"1 2");
        assert_eq!(ok(b"lsearch -dictionary -sorted {x1 x9 x10} x9"), b"1");
    }

    #[test]
    fn lsearch_errors() {
        assert_eq!(
            err(b"lsearch -stride 0 {a b} x"),
            b"stride length must be at least 1"
        );
        assert_eq!(
            err(b"lsearch -exact -integer {a b} 1"),
            b"expected integer but got \"a\""
        );
        assert_eq!(
            err(b"lsearch -bogus {a} b"),
            b"bad option \"-bogus\": must be -all, -ascii, -bisect, -decreasing, -dictionary, -exact, -glob, -increasing, -index, -inline, -integer, -nocase, -not, -real, -regexp, -sorted, -start, -stride, or -subindices"
        );
    }

    // Needs the numeric tower: `-command` comparators are `expr` lambdas.
    #[cfg(have_tommath)]
    #[test]
    fn lsort_options() {
        assert_eq!(ok(b"lsort {c a b}"), b"a b c");
        assert_eq!(ok(b"lsort -decreasing {c a b}"), b"c b a");
        assert_eq!(ok(b"lsort -integer {10 2 33 4}"), b"2 4 10 33");
        assert_eq!(ok(b"lsort -unique {b a a c}"), b"a b c");
        assert_eq!(ok(b"lsort -nocase {B a C}"), b"a B C");
        // -stride groups; the key defaults to the group's first element.
        assert_eq!(ok(b"lsort -stride 2 {c 3 a 1 b 2}"), b"a 1 b 2 c 3");
        assert_eq!(
            ok(b"lsort -stride 2 -index 1 {c 3 a 1 b 2}"),
            b"a 1 b 2 c 3"
        );
        // -index drills into each element.
        assert_eq!(ok(b"lsort -index 0 {{b 2} {a 1}}"), b"{a 1} {b 2}");
        assert_eq!(ok(b"lsort -index 1 {{b 1} {a 2}}"), b"{b 1} {a 2}");
        // -dictionary: embedded numbers compared numerically, case-insensitive.
        assert_eq!(ok(b"lsort -dictionary {x10 x9 x1}"), b"x1 x9 x10");
        // -indices returns positions; -real; -command.
        assert_eq!(ok(b"lsort -indices {c a b}"), b"1 2 0");
        assert_eq!(ok(b"lsort -real {1.5 0.2 3}"), b"0.2 1.5 3");
        assert_eq!(
            ok(b"lsort -command {apply {{a b} {expr {$a - $b}}}} {3 1 2}"),
            b"1 2 3"
        );
    }

    #[test]
    fn lsort_errors() {
        assert_eq!(
            err(b"lsort -stride 1 {a b}"),
            b"stride length must be at least 2"
        );
        assert_eq!(
            err(b"lsort -stride 2 {a b c}"),
            b"list size must be a multiple of the stride length"
        );
        assert_eq!(
            err(b"lsort -integer {a b}"),
            b"expected integer but got \"a\""
        );
        assert_eq!(
            err(b"lsort -real {a b}"),
            b"expected floating-point number but got \"a\""
        );
        assert_eq!(
            err(b"lsort -index 5 {{a b}}"),
            b"element 5 missing from sublist \"a b\""
        );
        assert_eq!(
            err(b"lsort -bogus {a}"),
            b"bad option \"-bogus\": must be -ascii, -command, -decreasing, -dictionary, -increasing, -index, -indices, -integer, -nocase, -real, -stride, or -unique"
        );
    }

    #[test]
    fn list_and_llength() {
        assert_eq!(ok(b"list a b c"), b"a b c");
        assert_eq!(ok(b"llength {a b c d}"), b"4");
        assert_eq!(ok(b"llength {}"), b"0");
        assert_eq!(ok(b"list a {b c} {}"), b"a {b c} {}"); // quoting
    }

    #[test]
    fn string_rep_survives_shimmer() {
        // A string→list shimmer (here via `llength`) keeps the original spelling
        // (irregular spacing), Tcl's dual-rep — not the canonical list form.
        assert_eq!(ok(b"set x {a  b   c}; llength $x; set x"), b"a  b   c");
        // An in-place mutation invalidates the cached rep → canonical regenerates.
        assert_eq!(
            ok(b"set x {a  b   c}; llength $x; lappend x d; set x"),
            b"a b c d"
        );
    }

    #[test]
    fn duplicate_preserves_string_rep() {
        // `set y $x` shares x's (shimmered) obj; `lappend x d` copies-on-write,
        // so the original obj y holds must keep its original spelling, and x gets
        // the canonical mutated form.
        assert_eq!(
            ok(b"set x {a  b   c}; llength $x; set y $x; lappend x d; set y"),
            b"a  b   c"
        );
        assert_eq!(
            ok(b"set x {a  b   c}; llength $x; set y $x; lappend x d; set x"),
            b"a b c d"
        );
    }

    #[test]
    fn hex_indices_and_counts_accepted() {
        // Lset/ledit and string index/repeat now share the
        // radix-aware index/integer core, so hex specs resolve like real Tcl.
        assert_eq!(ok(b"set x {a b c}; lset x 0x1 Z; set x"), b"a Z c");
        assert_eq!(ok(b"string index abcdef 0x2"), b"c");
        assert_eq!(ok(b"string index abcdef end-0x1"), b"e");
        assert_eq!(ok(b"string repeat x 0x3"), b"xxx");
        // lrepeat count is radix-aware too.
        assert_eq!(ok(b"lrepeat 0x2 a"), b"a a");
    }

    #[test]
    fn lindex_and_lrange() {
        assert_eq!(ok(b"lindex {a b c} 1"), b"b");
        assert_eq!(ok(b"lindex {a b c} end"), b"c");
        assert_eq!(ok(b"lindex {a b c} end-1"), b"b");
        assert_eq!(ok(b"lindex {a b c} 9"), b""); // out of range
        assert_eq!(ok(b"lrange {a b c d e} 1 3"), b"b c d");
        assert_eq!(ok(b"lrange {a b c} 1 end"), b"b c");
    }

    #[test]
    fn lappend_builds_and_cow() {
        assert_eq!(ok(b"lappend x a; lappend x b c"), b"a b c");
        // COW: y shares x's value; appending to y must not change x
        assert_eq!(
            ok(b"set x {a b}; set y $x; lappend y c; list $x $y"),
            b"{a b} {a b c}"
        );
        // lappend onto a string var shimmers it to a list
        assert_eq!(ok(b"set s {1 2}; lappend s 3"), b"1 2 3");
        // lappend addresses array elements (not a scalar literally named `a(k)`),
        // including fully-qualified element keys (the safe-base / opt case).
        assert_eq!(ok(b"lappend a(k) 1 2; lappend a(k) 3"), b"1 2 3");
        assert_eq!(ok(b"lappend a(k) 1 2; set a(k)"), b"1 2");
        assert_eq!(
            ok(b"namespace eval n { variable arr; lappend arr(::x::y) a b }; set ::n::arr(::x::y)"),
            b"a b"
        );
    }

    /// `lappend` routed through the shared COW core: the no-values form now
    /// validates the current value as a list (erroring on a malformed one, like
    /// tclsh — the old runtime skipped this), creates an empty list when unset,
    /// and a mutating `lappend` fires the write trace once.
    #[test]
    fn lappend_shared_core_parity() {
        // no-values on a malformed list errors (was silently returned before).
        let (c, m) = run(b"set y \"{\"; lappend y");
        assert_eq!(c, Code::Error);
        assert_eq!(m, b"unmatched open brace in list");
        // no-values on an unset variable creates an empty list.
        assert_eq!(ok(b"lappend fresh; info exists fresh"), b"1");
        assert_eq!(ok(b"lappend fresh; set fresh"), b"");
        // a mutating lappend fires the write trace exactly once.
        assert_eq!(
            ok(b"set l {1 2}; set m 0; trace add variable l write {incr ::m;#}; lappend l 3; set m"),
            b"1"
        );
    }

    /// A write trace that mutates or unsets the variable during `lappend`
    /// (append-7.x): the result is the variable's *post-trace* value (empty when
    /// unset, the trace's new value otherwise), matching C — and the fresh list
    /// object is not freed mid-command (the `run` helper's leak / double-free
    /// counters guard against the use-after-free this used to be).
    #[test]
    fn lappend_write_trace_unset_and_rewrite() {
        // The write trace unsets the variable: result is empty, var gone.
        assert_eq!(
            ok(b"proc foo args {global x; unset x}\ntrace add variable x write foo\nlappend x 1"),
            b""
        );
        assert_eq!(
            ok(b"proc foo args {global x; unset x}\ntrace add variable x write foo\nlappend x 1; info exists x"),
            b"0"
        );
        // The write trace rewrites the variable: result reflects the new value.
        assert_eq!(
            ok(b"proc foo args {global y; set y ZZZ}\ntrace add variable y write foo\nlappend y 1"),
            b"ZZZ"
        );
    }

    /// `lappend` fires a read trace (its side effects run) but swallows a trace
    /// error, creating a missing element instead of failing (append-7.2/9.0,
    /// bug 3057639) — where `set`/`append`-read would propagate the error.
    #[test]
    fn lappend_read_trace_fires_but_swallows_error() {
        // Side effects run: the read trace observes `name {} read`.
        assert_eq!(
            ok(b"set ::r {}\nproc foo args {append ::r $args}\ntrace add variable v read foo\nlappend v a\nset ::r"),
            b"v {} read"
        );
        // A read trace that errors does not fail lappend; it appends to empty.
        assert_eq!(
            ok(b"set v 1\ntrace add variable v read {error boom}\nlappend v a"),
            b"a"
        );
        // A succeeding read trace lets lappend see the real current value.
        assert_eq!(
            ok(b"set v 1\nproc foo args {}\ntrace add variable v read foo\nlappend v a"),
            b"1 a"
        );
        // bug 3057639: a read trace erroring on a missing element still creates it.
        let (c, m) = run(
            b"array set a {}\nproc nn {var key val} {upvar 1 $var l\n if {![info exists l($key)]} {return -code error x}}\ntrace add variable a read nn\nlappend a(key) hi",
        );
        assert_eq!(c, Code::Ok);
        assert_eq!(&m, b"hi");
    }

    #[test]
    fn lreverse_concat_join_split() {
        assert_eq!(ok(b"lreverse {a b c}"), b"c b a");
        assert_eq!(ok(b"concat {a b} {c  d} { e }"), b"a b c  d e");
        assert_eq!(ok(b"join {a b c} -"), b"a-b-c");
        assert_eq!(ok(b"split a,b,c ,"), b"a b c");
        assert_eq!(ok(b"split {a b c}"), b"a b c"); // default whitespace
    }

    #[test]
    fn lassign_assigns_and_returns_rest() {
        assert_eq!(ok(b"lassign {a b c} x y; list $x $y"), b"a b");
        assert_eq!(ok(b"lassign {a b c d} x y"), b"c d"); // returns the tail
        assert_eq!(ok(b"lassign {a} x y; list $x $y"), b"a {}"); // missing → empty
    }

    #[test]
    fn errors() {
        let (c, b) = run(b"llength");
        assert_eq!(c, Code::Error);
        assert!(b.starts_with(b"wrong # args"));
    }
}
