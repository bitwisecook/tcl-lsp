//! List commands (T1.6) — `list` / `llength` / `lindex` / `lappend` / `lrange`
//! / `lreverse` / `concat` / `join` / `split` / `lassign` / `lrepeat` /
//! `linsert` / `lreplace` / `lset` / `ledit` / `lsearch` / `lsort`, over the
//! [`crate::list`] value type.
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::interp::{obj_bytes, Code, Interp};
use crate::list;
use crate::obj::{self, TclObj};

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
    interp.register_builtin(b"lsearch", lsearch);
    interp.register_builtin(b"lsort", lsort);
}

// -- helpers ---------------------------------------------------------------

fn wrong_args(interp: &mut Interp, usage: &[u8]) -> Code {
    let mut m = b"wrong # args: should be \"".to_vec();
    m.extend_from_slice(usage);
    m.push(b'"');
    interp.set_error(&m)
}

/// Set the result to an integer.
fn set_int(interp: &mut Interp, n: i64) {
    interp.set_result(obj::new_wide_int_obj(n));
}

/// Set the result to a list built from element objects (each retained).
fn set_list(interp: &mut Interp, elems: &[*mut TclObj]) {
    interp.set_result(list::new_list_obj(elems));
}

/// Parse a plain signed decimal integer (for list indices / counts).
fn parse_isize(b: &[u8]) -> Option<isize> {
    let s = core::str::from_utf8(b).ok()?.trim();
    s.parse::<isize>().ok()
}

/// Parse a leading optionally-signed integer, returning its value and the
/// unconsumed tail. `None` if no digits follow the optional sign.
fn parse_int_prefix(s: &[u8]) -> Option<(isize, &[u8])> {
    let mut i = 0;
    if matches!(s.first(), Some(b'+' | b'-')) {
        i += 1;
    }
    let digits_start = i;
    while i < s.len() && s[i].is_ascii_digit() {
        i += 1;
    }
    if i == digits_start {
        return None;
    }
    let val = parse_isize(&s[..i])?;
    Some((val, &s[i..]))
}

/// Resolve a Tcl list index spec against a list of `len` elements. Supports the
/// full `TclGetIntForIndex` grammar: an integer, `end`, and a base (`end` or an
/// integer) with an optional `±integer` offset (`end-2`, `0-1`, `-2+1`, …).
/// Returns a (possibly out-of-range) signed index; callers clamp/range-check.
pub(crate) fn index_spec(spec: &[u8], len: usize) -> Option<isize> {
    let len = len as isize;
    let s = {
        let t = core::str::from_utf8(spec).ok()?.trim();
        t.as_bytes()
    };
    if s.is_empty() {
        return None;
    }
    // Base: `end` or a signed integer.
    let (base, rest) = if let Some(r) = s.strip_prefix(b"end") {
        (len - 1, r)
    } else {
        parse_int_prefix(s)?
    };
    if rest.is_empty() {
        return Some(base);
    }
    // Optional offset: a `+`/`-` connector then a (possibly signed) integer, so
    // `end--1` is `end - (-1)` and `0-1` is `0 - 1` (matches `GetEndOffsetFromObj`).
    let connector = rest[0];
    if connector != b'+' && connector != b'-' {
        return None;
    }
    let operand = parse_isize(&rest[1..])?;
    let offset = if connector == b'-' { -operand } else { operand };
    Some(base + offset)
}

#[inline]
fn is_ws(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

// -- commands --------------------------------------------------------------

/// `list ?arg ...?` — a list of its arguments.
fn list_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    set_list(interp, &argv[1..]);
    Code::Ok
}

/// `llength list`.
fn llength(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 2 {
        return wrong_args(interp, b"llength list");
    }
    match list::list_length(argv[1]) {
        Ok(n) => {
            set_int(interp, n as i64);
            Code::Ok
        }
        Err(e) => bad_list(interp, e),
    }
}

/// `lindex list ?index ...?` — drill into a (nested) list. With no index the
/// whole list is returned; a single index argument is itself split into an
/// index *path* (so `lindex {{a b} c} {0 1}` works); multiple index arguments
/// each step one level. An out-of-range step yields the empty string. Mirrors
/// `Tcl_LindexObjCmd` (`TclLindexList`/`TclLindexFlat`).
fn lindex(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return wrong_args(interp, b"lindex list ?index ...?");
    }
    let index_args = &argv[2..];
    if index_args.is_empty() {
        interp.set_result(argv[1]);
        return Code::Ok;
    }
    // Build the index path: a lone argument is split into a list of indices;
    // multiple arguments are each a single index.
    let path: Vec<Vec<u8>> = if index_args.len() == 1 {
        match crate::parse::split_list(&obj_bytes(index_args[0])) {
            Ok(p) => p,
            Err(e) => return interp.set_error(e.message()),
        }
    } else {
        index_args.iter().map(|&a| obj_bytes(a)).collect()
    };

    let mut cur = argv[1];
    for spec in &path {
        let n = match list::list_length(cur) {
            Ok(n) => n,
            Err(e) => return bad_list(interp, e),
        };
        let idx = match index_spec(spec, n) {
            Some(i) => i,
            None => return bad_index(interp, spec),
        };
        if idx < 0 || idx as usize >= n {
            interp.set_result_bytes(b""); // out of range → empty (Tcl)
            return Code::Ok;
        }
        match list::list_index(cur, idx as usize) {
            // Each element is owned by its parent list (alive up the chain to
            // `argv[1]`), so borrowing it for the next step is safe.
            Ok(Some(e)) => cur = e,
            _ => {
                interp.set_result_bytes(b"");
                return Code::Ok;
            }
        }
    }
    interp.set_result(cur);
    Code::Ok
}

/// `lappend varName ?value ...?` — append to the list in `varName` (creating it
/// if unset), copy-on-write if the value is shared. Returns the new list.
fn lappend(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return wrong_args(interp, b"lappend varName ?value ...?");
    }
    let name = obj_bytes(argv[1]);
    let values = &argv[2..];
    // `lappend a(k) ...` must address the array element, not a scalar literally
    // named `a(k)` — split the array ref like `set`/`incr` do.
    let (base, elem) = crate::frame::split_array_ref(&name);

    let cur = match &elem {
        Some(k) => interp.var_get_elem(&base, k),
        None => interp.var_get(&base),
    };

    // Determine the target list object (in-place if unshared, else a copy/new).
    let (target, is_new) = match cur {
        None => (list::new_list_obj(&[]), true), // fresh empty list (rc 0)
        Some(o) if obj::is_shared(o) => (obj::duplicate(o), true), // COW copy (rc 0)
        Some(o) => (o, false),                   // mutate in place (frame owns it)
    };

    for &v in values {
        if let Err(e) = list::list_append(target, v) {
            if is_new {
                drop_fresh(target);
            }
            return bad_list(interp, e);
        }
    }

    // A new/copied list must be stored back into the variable.
    if is_new {
        // `target` is rc 0; `set` retains it into the variable (and releases the
        // prior value for the COW/overwrite case).
        let stored = match &elem {
            Some(k) => interp.var_set_elem(&base, k, target),
            None => interp.var_set(&base, target),
        };
        if stored.is_err() {
            drop_fresh(target);
            let mut m = b"can't set \"".to_vec();
            m.extend_from_slice(&name);
            m.extend_from_slice(b"\": variable is array");
            return interp.set_error(&m);
        }
    }
    interp.set_result(target);
    Code::Ok
}

/// `lrange list first last` — the sublist from `first` to `last` (inclusive),
/// clamped to range.
fn lrange(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 4 {
        return wrong_args(interp, b"lrange list first last");
    }
    let elems = match list::list_elements(argv[1]) {
        Ok(e) => e,
        Err(e) => return bad_list(interp, e),
    };
    let n = elems.len();
    let first_b = obj_bytes(argv[2]);
    let last_b = obj_bytes(argv[3]);
    let first = match index_spec(&first_b, n) {
        Some(i) => i.max(0) as usize,
        None => return bad_index(interp, &first_b),
    };
    let last = match index_spec(&last_b, n) {
        Some(i) => i,
        None => return bad_index(interp, &last_b),
    };
    if last < 0 || first >= n || (last as usize) < first {
        interp.set_result_bytes(b"");
        return Code::Ok;
    }
    let last = (last as usize).min(n - 1);
    set_list(interp, &elems[first..=last]);
    Code::Ok
}

/// `lreverse list`.
fn lreverse(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 2 {
        return wrong_args(interp, b"lreverse list");
    }
    match list::list_elements(argv[1]) {
        Ok(mut e) => {
            e.reverse();
            set_list(interp, &e);
            Code::Ok
        }
        Err(e) => bad_list(interp, e),
    }
}

/// `concat ?arg ...?` — trim each arg of surrounding whitespace, drop empties,
/// join with single spaces (Tcl's string-level concat).
fn concat(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let mut out: Vec<u8> = Vec::new();
    for &a in &argv[1..] {
        let b = obj_bytes(a);
        let start = b.iter().position(|&c| !is_ws(c));
        let Some(start) = start else { continue }; // all-whitespace → skip
        let end = b.iter().rposition(|&c| !is_ws(c)).unwrap() + 1;
        if !out.is_empty() {
            out.push(b' ');
        }
        out.extend_from_slice(&b[start..end]);
    }
    interp.set_result_bytes(&out);
    Code::Ok
}

/// `join list ?joinString?` — element string reps joined by `joinString`
/// (default a single space).
fn join(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 || argv.len() > 3 {
        return wrong_args(interp, b"join list ?joinString?");
    }
    let sep = if argv.len() == 3 {
        obj_bytes(argv[2])
    } else {
        b" ".to_vec()
    };
    let elems = match list::list_elements(argv[1]) {
        Ok(e) => e,
        Err(e) => return bad_list(interp, e),
    };
    let mut out = Vec::new();
    for (i, &e) in elems.iter().enumerate() {
        if i > 0 {
            out.extend_from_slice(&sep);
        }
        out.extend_from_slice(&obj_bytes(e));
    }
    interp.set_result_bytes(&out);
    Code::Ok
}

/// `split string ?splitChars?` — split into a list on any byte of `splitChars`
/// (default whitespace). An empty `splitChars` makes each byte an element.
fn split(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 || argv.len() > 3 {
        return wrong_args(interp, b"split string ?splitChars?");
    }
    let s = obj_bytes(argv[1]);
    let chars = if argv.len() == 3 {
        Some(obj_bytes(argv[2]))
    } else {
        None
    };
    let mut elems: Vec<*mut TclObj> = Vec::new();

    // `split` works on characters (code points), not bytes — a multi-byte
    // separator or an empty split string must respect UTF-8 boundaries.
    let s_chars: Vec<char> = String::from_utf8_lossy(&s).chars().collect();
    let push_str = |elems: &mut Vec<*mut TclObj>, cur: &str| {
        elems.push(obj::new_string_bytes(cur.as_bytes()));
    };

    match chars {
        Some(ref c) if c.is_empty() => {
            // Each character becomes its own element.
            let mut b = [0u8; 4];
            for &ch in &s_chars {
                elems.push(obj::new_string_bytes(ch.encode_utf8(&mut b).as_bytes()));
            }
        }
        _ => {
            let sep: Vec<char> = chars
                .as_ref()
                .map(|c| String::from_utf8_lossy(c).chars().collect())
                .unwrap_or_default();
            let is_sep = |ch: char| match &chars {
                Some(_) => sep.contains(&ch),
                None => matches!(ch, ' ' | '\t' | '\n' | '\r' | '\u{0b}' | '\u{0c}'),
            };
            let mut cur = String::new();
            for &ch in &s_chars {
                if is_sep(ch) {
                    push_str(&mut elems, &cur);
                    cur.clear();
                } else {
                    cur.push(ch);
                }
            }
            // Trailing element (Tcl: split "" → empty list, no trailing "").
            if !s_chars.is_empty() {
                push_str(&mut elems, &cur);
            }
        }
    }
    // new_list_obj retains each element; release our construction refs.
    set_list(interp, &elems);
    for e in elems {
        drop_fresh(e);
    }
    Code::Ok
}

/// `lassign list ?varName ...?` — assign successive elements to the vars
/// (missing → empty string); return the unassigned tail as a list.
fn lassign(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return wrong_args(interp, b"lassign list ?varName ...?");
    }
    let elems = match list::list_elements(argv[1]) {
        Ok(e) => e,
        Err(e) => return bad_list(interp, e),
    };
    let vars = &argv[2..];
    for (i, &var) in vars.iter().enumerate() {
        let name = obj_bytes(var);
        let val = if i < elems.len() {
            elems[i]
        } else {
            obj::new_string_bytes(b"")
        };
        let fresh = i >= elems.len();
        let r = interp.var_set(&name, val);
        if fresh {
            // `set` retained `val`; release our construction ref to the empty obj
            drop_fresh(val);
        }
        if r.is_err() {
            let mut m = b"can't set \"".to_vec();
            m.extend_from_slice(&name);
            m.extend_from_slice(b"\": variable is array");
            return interp.set_error(&m);
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
fn lrepeat(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return wrong_args(interp, b"lrepeat count ?value ...?");
    }
    let Some(count) = parse_isize(&obj_bytes(argv[1])) else {
        return not_integer(interp, &obj_bytes(argv[1]));
    };
    if count < 0 {
        return interp.set_error(b"bad count \"-1\": must be integer >= 0");
    }
    let values = &argv[2..];
    let mut out: Vec<*mut TclObj> = Vec::with_capacity(count as usize * values.len());
    for _ in 0..count {
        out.extend_from_slice(values);
    }
    set_list(interp, &out);
    Code::Ok
}

/// `linsert list index ?element ...?` — insert before `index` (`end` appends).
fn linsert(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return wrong_args(interp, b"linsert list index ?element ...?");
    }
    let elems = match list::list_elements(argv[1]) {
        Ok(v) => v,
        Err(e) => return bad_list(interp, e),
    };
    let len = elems.len();
    // For `linsert`, the index names the insertion point, so `end` is *after*
    // the last element (= `len`); resolving against `len + 1` makes `end`,
    // `end-N`, etc. land correctly.
    let spec = obj_bytes(argv[2]);
    let raw = match index_spec(&spec, len + 1) {
        Some(i) => i,
        None => return bad_index(interp, &spec),
    };
    let at = raw.clamp(0, len as isize) as usize;
    let mut out: Vec<*mut TclObj> = Vec::with_capacity(len + argv.len() - 3);
    out.extend_from_slice(&elems[..at]);
    out.extend_from_slice(&argv[3..]);
    out.extend_from_slice(&elems[at..]);
    set_list(interp, &out);
    Code::Ok
}

/// `lreplace list first last ?element ...?` — replace the `[first,last]` range.
fn lreplace(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 {
        return wrong_args(interp, b"lreplace list first last ?element ...?");
    }
    let elems = match list::list_elements(argv[1]) {
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
    // Exclusive end of the removed range; `last < first` removes nothing.
    let hi = ((last + 1).max(0) as usize).clamp(lo, len);
    let mut out: Vec<*mut TclObj> = Vec::with_capacity(len + argv.len());
    out.extend_from_slice(&elems[..lo]);
    out.extend_from_slice(&argv[4..]);
    out.extend_from_slice(&elems[hi..]);
    set_list(interp, &out);
    Code::Ok
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
        return wrong_args(interp, b"lset listVar ?index? ?index ...? value");
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
        return wrong_args(interp, b"ledit listVar first last ?element ...?");
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
    let hi = ((last + 1).max(0) as usize).clamp(lo, len);
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

/// `lsearch ?-exact|-glob? ?-nocase? ?-all? ?-not? ?-inline? list pattern`.
fn lsearch(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let (mut glob, mut nocase, mut all, mut not, mut inline) = (true, false, false, false, false);
    let mut i = 1;
    while i < argv.len() {
        match obj_bytes(argv[i]).as_slice() {
            b"-glob" => glob = true,
            b"-exact" => glob = false,
            b"-nocase" => nocase = true,
            b"-all" => all = true,
            b"-not" => not = true,
            b"-inline" => inline = true,
            b"--" => {
                i += 1;
                break;
            }
            opt if opt.starts_with(b"-") => {
                let mut m = b"bad option \"".to_vec();
                m.extend_from_slice(opt);
                m.extend_from_slice(b"\": must be -all, -exact, -glob, -inline, -nocase, or -not");
                return interp.set_error(&m);
            }
            _ => break,
        }
        i += 1;
    }
    if argv.len() - i != 2 {
        return wrong_args(interp, b"lsearch ?-option ...? list pattern");
    }
    let elems = match list::list_elements(argv[i]) {
        Ok(v) => v,
        Err(e) => return bad_list(interp, e),
    };
    let pattern = obj_bytes(argv[i + 1]);
    let mut hits: Vec<usize> = Vec::new();
    for (idx, &e) in elems.iter().enumerate() {
        let m = elem_matches(glob, nocase, &pattern, &obj_bytes(e)) != not;
        if m {
            hits.push(idx);
            if !all {
                break;
            }
        }
    }
    if inline {
        let objs: Vec<*mut TclObj> = if all {
            hits.iter().map(|&h| elems[h]).collect()
        } else {
            hits.first().map(|&h| vec![elems[h]]).unwrap_or_default()
        };
        // -inline (non -all) returns the element itself, or "" if none.
        if all {
            set_list(interp, &objs);
        } else if let Some(&e) = objs.first() {
            interp.set_result(e);
        } else {
            interp.set_result_bytes(b"");
        }
    } else if all {
        let idx_objs: Vec<*mut TclObj> = hits
            .iter()
            .map(|&h| obj::new_wide_int_obj(h as i64))
            .collect();
        set_list(interp, &idx_objs);
    } else {
        set_int(interp, hits.first().map_or(-1, |&h| h as i64));
    }
    Code::Ok
}

fn elem_matches(glob: bool, nocase: bool, pat: &[u8], elem: &[u8]) -> bool {
    if glob {
        match (core::str::from_utf8(pat), core::str::from_utf8(elem)) {
            (Ok(p), Ok(e)) => tcl_syntax::glob::string_case_match(p, e, nocase),
            _ => false,
        }
    } else if nocase {
        pat.eq_ignore_ascii_case(elem)
    } else {
        pat == elem
    }
}

/// `lsort ?-ascii|-integer|-real? ?-nocase? ?-increasing|-decreasing? ?-unique?
#[derive(Clone, Copy, PartialEq)]
enum SortMode {
    Ascii,
    Dictionary,
    Integer,
    Real,
    Command,
}

/// Parse a Tcl integer (decimal, or `0x`/`0o`/`0b` radix, optional sign) into an
/// `i128` for `-integer` sort keys. `None` if not an integer.
fn parse_wide(b: &[u8]) -> Option<i128> {
    let s = core::str::from_utf8(b).ok()?.trim();
    let (neg, body) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    if body.is_empty() {
        return None;
    }
    let v: i128 = if let Some(h) = body.strip_prefix("0x").or_else(|| body.strip_prefix("0X")) {
        i128::from_str_radix(h, 16).ok()?
    } else if let Some(o) = body.strip_prefix("0o").or_else(|| body.strip_prefix("0O")) {
        i128::from_str_radix(o, 8).ok()?
    } else if let Some(bb) = body.strip_prefix("0b").or_else(|| body.strip_prefix("0B")) {
        i128::from_str_radix(bb, 2).ok()?
    } else {
        body.parse::<i128>().ok()?
    };
    Some(if neg { -v } else { v })
}

/// Parse a Tcl floating-point value for `-real` sort keys.
fn parse_real(b: &[u8]) -> Option<f64> {
    core::str::from_utf8(b).ok()?.trim().parse::<f64>().ok()
}

/// Dictionary comparison (`lsort -dictionary`): case-insensitive, with embedded
/// decimal runs compared as numbers; a run with more leading zeros sorts later
/// only as a secondary tiebreak. Ported from C's `DictionaryCompare`.
fn dictionary_compare(left: &[u8], right: &[u8]) -> core::cmp::Ordering {
    use core::cmp::Ordering;
    let (mut li, mut ri) = (0usize, 0usize);
    let mut secondary: i64 = 0;
    loop {
        let l_is_digit = left.get(li).is_some_and(u8::is_ascii_digit);
        let r_is_digit = right.get(ri).is_some_and(u8::is_ascii_digit);
        if l_is_digit && r_is_digit {
            // Skip and tally leading zeros (more zeros → later, as a secondary).
            let mut zeros: i64 = 0;
            while left.get(li) == Some(&b'0') && left.get(li + 1).is_some_and(u8::is_ascii_digit) {
                li += 1;
                zeros += 1;
            }
            while right.get(ri) == Some(&b'0') && right.get(ri + 1).is_some_and(u8::is_ascii_digit)
            {
                ri += 1;
                zeros -= 1;
            }
            if secondary == 0 {
                secondary = zeros;
            }
            // Compare the digit runs by length, then by value.
            let mut diff: i64 = 0;
            loop {
                if diff == 0 {
                    diff = left.get(li).map_or(0, |&c| c as i64)
                        - right.get(ri).map_or(0, |&c| c as i64);
                }
                li += 1;
                ri += 1;
                let ld = left.get(li).is_some_and(u8::is_ascii_digit);
                let rd = right.get(ri).is_some_and(u8::is_ascii_digit);
                if !rd {
                    if ld {
                        return Ordering::Greater;
                    }
                    if diff != 0 {
                        return diff.cmp(&0);
                    }
                    break;
                } else if !ld {
                    return Ordering::Less;
                }
            }
            continue;
        }
        match (left.get(li).copied(), right.get(ri).copied()) {
            (Some(l), Some(r)) => {
                let (ll, rl) = (l.to_ascii_lowercase(), r.to_ascii_lowercase());
                if ll != rl {
                    return ll.cmp(&rl);
                }
                if secondary == 0 && l != r {
                    secondary = (l as i64) - (r as i64);
                }
                li += 1;
                ri += 1;
            }
            (l, r) => {
                let diff = l.map_or(0i64, |c| c as i64) - r.map_or(0i64, |c| c as i64);
                if diff != 0 {
                    return diff.cmp(&0);
                }
                return secondary.cmp(&0);
            }
        }
    }
}

/// Compare two sort-key objects under a non-command `mode`.
fn lsort_key_cmp(
    mode: SortMode,
    nocase: bool,
    a: *mut TclObj,
    b: *mut TclObj,
) -> core::cmp::Ordering {
    use core::cmp::Ordering;
    let (ab, bb) = (obj_bytes(a), obj_bytes(b));
    match mode {
        SortMode::Dictionary => dictionary_compare(&ab, &bb),
        SortMode::Integer => parse_wide(&ab)
            .unwrap_or(0)
            .cmp(&parse_wide(&bb).unwrap_or(0)),
        SortMode::Real => parse_real(&ab)
            .unwrap_or(0.0)
            .partial_cmp(&parse_real(&bb).unwrap_or(0.0))
            .unwrap_or(Ordering::Equal),
        _ => {
            if nocase {
                ab.to_ascii_lowercase().cmp(&bb.to_ascii_lowercase())
            } else {
                ab.cmp(&bb)
            }
        }
    }
}

/// Drill into `obj` by the (nested) `path` of index specs (`lsort -index`).
/// Errors `element X missing from sublist "..."` on an out-of-range step
/// (C's `SelectObjFromSublist`).
fn select_by_index(
    interp: &mut Interp,
    obj: *mut TclObj,
    path: &[Vec<u8>],
) -> Result<*mut TclObj, Code> {
    let mut cur = obj;
    for spec in path {
        let n = match list::list_length(cur) {
            Ok(n) => n,
            Err(e) => return Err(bad_list(interp, e)),
        };
        let idx = match index_spec(spec, n) {
            Some(i) => i,
            None => return Err(bad_index(interp, spec)),
        };
        if idx < 0 || idx as usize >= n {
            let mut m = b"element ".to_vec();
            m.extend_from_slice(spec);
            m.extend_from_slice(b" missing from sublist \"");
            m.extend_from_slice(&obj_bytes(cur));
            m.push(b'"');
            return Err(interp.set_error(&m));
        }
        cur = match list::list_index(cur, idx as usize) {
            Ok(Some(e)) => e,
            _ => return Ok(cur),
        };
    }
    Ok(cur)
}

/// `"-X" option must be followed by <what>` (C's missing-value errors).
fn lsort_needs_value(interp: &mut Interp, opt: &[u8], what: &[u8]) -> Code {
    let mut m = b"\"".to_vec();
    m.extend_from_slice(opt);
    m.extend_from_slice(b"\" option must be followed by ");
    m.extend_from_slice(what);
    interp.set_error(&m)
}

/// `lsort ?-option value ...? list` — sort the list (`Tcl_LsortObjCmd`).
fn lsort(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let n = argv.len();
    if n < 2 {
        return wrong_args(interp, b"lsort ?-option value ...? list");
    }
    let (mut mode, mut nocase, mut increasing, mut unique, mut indices_out) =
        (SortMode::Ascii, false, true, false, false);
    let mut group = 1usize;
    let mut index_path: Vec<Vec<u8>> = Vec::new();
    let mut cmd_prefix: Option<*mut TclObj> = None;
    // All args before the final one (the list) are options (or option values).
    let mut i = 1;
    while i < n - 1 {
        match obj_bytes(argv[i]).as_slice() {
            b"-ascii" => mode = SortMode::Ascii,
            b"-dictionary" => mode = SortMode::Dictionary,
            b"-integer" => mode = SortMode::Integer,
            b"-real" => mode = SortMode::Real,
            b"-nocase" => nocase = true,
            b"-increasing" => increasing = true,
            b"-decreasing" => increasing = false,
            b"-unique" => unique = true,
            b"-indices" => indices_out = true,
            b"-index" => {
                if i == n - 2 {
                    return lsort_needs_value(interp, b"-index", b"list index");
                }
                index_path = match crate::parse::split_list(&obj_bytes(argv[i + 1])) {
                    Ok(p) => p,
                    Err(e) => return interp.set_error(e.message()),
                };
                i += 1;
            }
            b"-stride" => {
                if i == n - 2 {
                    return lsort_needs_value(interp, b"-stride", b"stride length");
                }
                match parse_wide(&obj_bytes(argv[i + 1])) {
                    Some(w) if w >= 2 => group = w as usize,
                    Some(_) => return interp.set_error(b"stride length must be at least 2"),
                    None => return not_integer(interp, &obj_bytes(argv[i + 1])),
                }
                i += 1;
            }
            b"-command" => {
                if i == n - 2 {
                    return lsort_needs_value(interp, b"-command", b"comparison command");
                }
                mode = SortMode::Command;
                cmd_prefix = Some(argv[i + 1]);
                i += 1;
            }
            other => {
                let mut m = b"bad option \"".to_vec();
                m.extend_from_slice(other);
                m.extend_from_slice(b"\": must be -ascii, -command, -decreasing, -dictionary, -increasing, -index, -indices, -integer, -nocase, -real, -stride, or -unique");
                return interp.set_error(&m);
            }
        }
        i += 1;
    }

    let elems = match list::list_elements(argv[n - 1]) {
        Ok(v) => v,
        Err(e) => return bad_list(interp, e),
    };
    let len = elems.len();
    if len == 0 {
        interp.set_result_bytes(b"");
        return Code::Ok;
    }

    // -stride groups the flat list; the leading -index value (default 0) picks
    // the key element within each group, the rest of the path applies within it.
    let mut group_offset = 0usize;
    let mut key_path: &[Vec<u8>] = &index_path;
    if group > 1 {
        if len % group != 0 {
            return interp.set_error(b"list size must be a multiple of the stride length");
        }
        if !index_path.is_empty() {
            match index_spec(&index_path[0], group) {
                Some(o) if o >= 0 && (o as usize) < group => group_offset = o as usize,
                _ => {
                    return interp.set_error(
                        b"when used with \"-stride\", the leading \"-index\" value must be within the group",
                    )
                }
            }
            key_path = &index_path[1..];
        }
    }
    let logical = len / group;

    // Decorate each logical element with the object its sort key comes from.
    let mut items: Vec<(usize, *mut TclObj)> = Vec::with_capacity(logical);
    for l in 0..logical {
        let base = l * group;
        let key_obj = match select_by_index(interp, elems[base + group_offset], key_path) {
            Ok(o) => o,
            Err(c) => return c,
        };
        items.push((base, key_obj));
    }

    if mode == SortMode::Command {
        let prefix = cmd_prefix.expect("set with Command mode");
        if let Err(c) = lsort_command(interp, &mut items, prefix, increasing) {
            return c;
        }
    } else {
        // Validate numeric keys up front (Tcl errors on the first non-number).
        if mode == SortMode::Integer {
            if let Some((_, k)) = items
                .iter()
                .find(|(_, k)| parse_wide(&obj_bytes(*k)).is_none())
            {
                return not_integer(interp, &obj_bytes(*k));
            }
        } else if mode == SortMode::Real {
            if let Some((_, k)) = items
                .iter()
                .find(|(_, k)| parse_real(&obj_bytes(*k)).is_none())
            {
                let mut m = b"expected floating-point number but got \"".to_vec();
                m.extend_from_slice(&obj_bytes(*k));
                m.push(b'"');
                return interp.set_error(&m);
            }
        }
        items.sort_by(|a, b| {
            let ord = lsort_key_cmp(mode, nocase, a.1, b.1);
            if increasing {
                ord
            } else {
                ord.reverse()
            }
        });
    }

    if unique {
        let eq_mode = if mode == SortMode::Command {
            SortMode::Ascii
        } else {
            mode
        };
        items.dedup_by(|a, b| lsort_key_cmp(eq_mode, nocase, a.1, b.1).is_eq());
    }

    // Build the result: -indices yields positions; -stride emits whole groups.
    let mut out: Vec<*mut TclObj> = Vec::with_capacity(logical * group);
    let mut fresh: Vec<*mut TclObj> = Vec::new();
    for (base, _) in &items {
        for j in 0..group {
            if indices_out {
                let o = obj::new_wide_int_obj((base + j) as i64);
                fresh.push(o);
                out.push(o);
            } else {
                out.push(elems[base + j]);
            }
        }
    }
    set_list(interp, &out);
    for o in fresh {
        drop_fresh(o);
    }
    Code::Ok
}

/// `-command` sort: a stable merge sort whose comparator evaluates the user
/// command prefix with the two elements and reads its integer result. Reentrant
/// (the comparator runs arbitrary Tcl), so a plain `sort_by` won't do.
fn lsort_command(
    interp: &mut Interp,
    items: &mut [(usize, *mut TclObj)],
    prefix: *mut TclObj,
    increasing: bool,
) -> Result<(), Code> {
    // Pre-split the command prefix into its words once.
    let words = match list::list_elements(prefix) {
        Ok(v) => v.iter().map(|&w| obj_bytes(w)).collect::<Vec<_>>(),
        Err(e) => return Err(bad_list(interp, e)),
    };
    let mut buf = items.to_vec();
    merge_sort_cmd(interp, &mut buf, &words, increasing)?;
    items.copy_from_slice(&buf);
    Ok(())
}

fn merge_sort_cmd(
    interp: &mut Interp,
    a: &mut [(usize, *mut TclObj)],
    words: &[Vec<u8>],
    increasing: bool,
) -> Result<(), Code> {
    let n = a.len();
    if n <= 1 {
        return Ok(());
    }
    let mid = n / 2;
    let mut left: Vec<_> = a[..mid].to_vec();
    let mut right: Vec<_> = a[mid..].to_vec();
    merge_sort_cmd(interp, &mut left, words, increasing)?;
    merge_sort_cmd(interp, &mut right, words, increasing)?;
    let (mut li, mut ri, mut k) = (0usize, 0usize, 0usize);
    while li < left.len() && ri < right.len() {
        // Stable: take from left unless right strictly precedes it.
        let ord = lsort_cmd_compare(interp, words, left[li].1, right[ri].1)?;
        let take_left = if increasing { ord <= 0 } else { ord >= 0 };
        if take_left {
            a[k] = left[li];
            li += 1;
        } else {
            a[k] = right[ri];
            ri += 1;
        }
        k += 1;
    }
    while li < left.len() {
        a[k] = left[li];
        li += 1;
        k += 1;
    }
    while ri < right.len() {
        a[k] = right[ri];
        ri += 1;
        k += 1;
    }
    Ok(())
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

fn not_integer(interp: &mut Interp, bytes: &[u8]) -> Code {
    let mut m = b"expected integer but got \"".to_vec();
    m.extend_from_slice(bytes);
    m.push(b'"');
    interp.set_error(&m)
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
        assert_eq!(ok(b"lreplace {a b c} 1 0 X"), b"a X b c"); // first>last → insert
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
    }

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
