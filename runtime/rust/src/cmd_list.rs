//! List commands (T1.6) — `list` / `llength` / `lindex` / `lappend` / `lrange`
//! / `lreverse` / `concat` / `join` / `split` / `lassign`, over the [`crate::list`]
//! value type. (`lsort`/`lsearch`/`lset`/`linsert`/`lreplace`/`lrepeat` follow,
//! once string match/comparison lands.)
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

/// Resolve a Tcl list index spec against a list of `len` elements:
/// integer, `end`, `end-N`, `end+N`. Returns a (possibly out-of-range) signed
/// index; callers clamp/range-check.
fn index_spec(spec: &[u8], len: usize) -> Option<isize> {
    let len = len as isize;
    if spec == b"end" {
        return Some(len - 1);
    }
    if let Some(rest) = spec.strip_prefix(b"end") {
        match rest.first() {
            Some(b'-') => return parse_isize(&rest[1..]).map(|n| len - 1 - n),
            Some(b'+') => return parse_isize(&rest[1..]).map(|n| len - 1 + n),
            _ => return None,
        }
    }
    parse_isize(spec)
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
        Err(_) => bad_list(interp),
    }
}

/// `lindex list ?index?` — element at `index`, the whole list if no index, or
/// the empty string if out of range.
fn lindex(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    match argv.len() {
        2 => {
            interp.set_result(argv[1]);
            Code::Ok
        }
        3 => {
            let n = match list::list_length(argv[1]) {
                Ok(n) => n,
                Err(_) => return bad_list(interp),
            };
            let spec = obj_bytes(argv[2]);
            let idx = match index_spec(&spec, n) {
                Some(i) => i,
                None => return bad_index(interp, &spec),
            };
            if idx < 0 || idx as usize >= n {
                interp.set_result_bytes(b""); // out of range → empty (Tcl)
                return Code::Ok;
            }
            match list::list_index(argv[1], idx as usize) {
                Ok(Some(e)) => interp.set_result(e),
                _ => interp.set_result_bytes(b""),
            }
            Code::Ok
        }
        _ => wrong_args(interp, b"lindex list ?index?"),
    }
}

/// `lappend varName ?value ...?` — append to the list in `varName` (creating it
/// if unset), copy-on-write if the value is shared. Returns the new list.
fn lappend(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return wrong_args(interp, b"lappend varName ?value ...?");
    }
    let name = obj_bytes(argv[1]);
    let values = &argv[2..];

    // Determine the target list object (in-place if unshared, else a copy/new).
    let (target, is_new) = match interp.frames.get(&name) {
        None => (list::new_list_obj(&[]), true), // fresh empty list (rc 0)
        Some(o) if obj::is_shared(o) => (obj::duplicate(o), true), // COW copy (rc 0)
        Some(o) => (o, false),                   // mutate in place (frame owns it)
    };

    for &v in values {
        if list::list_append(target, v).is_err() {
            if is_new {
                drop_fresh(target);
            }
            return bad_list(interp);
        }
    }

    // A new/copied list must be stored back into the variable.
    if is_new {
        // `target` is rc 0; `set` retains it into the variable (and releases the
        // prior value for the COW/overwrite case).
        if interp.frames.set(&name, target).is_err() {
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
        Err(_) => return bad_list(interp),
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
        Err(_) => bad_list(interp),
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
        Err(_) => return bad_list(interp),
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

    match chars {
        Some(ref c) if c.is_empty() => {
            // each byte becomes its own element
            for &b in &s {
                elems.push(obj::new_string_bytes(&[b]));
            }
        }
        _ => {
            let is_sep = |b: u8| match &chars {
                Some(c) => c.contains(&b),
                None => is_ws(b),
            };
            let mut cur: Vec<u8> = Vec::new();
            for &b in &s {
                if is_sep(b) {
                    elems.push(obj::new_string_bytes(&cur));
                    cur.clear();
                } else {
                    cur.push(b);
                }
            }
            // trailing element (also handles the empty-string → {} case only
            // when there was a separator; Tcl: split "" -> "" i.e. empty list)
            if !s.is_empty() {
                elems.push(obj::new_string_bytes(&cur));
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
        Err(_) => return bad_list(interp),
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
        let r = interp.frames.set(&name, val);
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

fn bad_list(interp: &mut Interp) -> Code {
    interp.set_error(b"unmatched open brace in list")
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
    fn list_and_llength() {
        assert_eq!(ok(b"list a b c"), b"a b c");
        assert_eq!(ok(b"llength {a b c d}"), b"4");
        assert_eq!(ok(b"llength {}"), b"0");
        assert_eq!(ok(b"list a {b c} {}"), b"a {b c} {}"); // quoting
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
