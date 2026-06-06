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
    interp.register_builtin(b"lrepeat", lrepeat);
    interp.register_builtin(b"linsert", linsert);
    interp.register_builtin(b"lreplace", lreplace);
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

    // Determine the target list object (in-place if unshared, else a copy/new).
    let (target, is_new) = match interp.var_get(&name) {
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
        if interp.var_set(&name, target).is_err() {
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
    // For `linsert`, `end` means "after the last element" (append).
    let spec = obj_bytes(argv[2]);
    let raw = if spec.as_slice() == b"end" {
        len as isize
    } else {
        match index_spec(&spec, len) {
            Some(i) => i,
            None => return bad_index(interp, &spec),
        }
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
/// list` — sort the list elements.
fn lsort(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    #[derive(Clone, Copy, PartialEq)]
    enum Kind {
        Ascii,
        Integer,
        Real,
    }
    let (mut kind, mut nocase, mut decreasing, mut unique) = (Kind::Ascii, false, false, false);
    let mut i = 1;
    while i < argv.len() {
        match obj_bytes(argv[i]).as_slice() {
            b"-ascii" => kind = Kind::Ascii,
            b"-integer" => kind = Kind::Integer,
            b"-real" => kind = Kind::Real,
            b"-nocase" => nocase = true,
            b"-increasing" => decreasing = false,
            b"-decreasing" => decreasing = true,
            b"-unique" => unique = true,
            b"--" => {
                i += 1;
                break;
            }
            opt if opt.starts_with(b"-") => {
                let mut m = b"bad option \"".to_vec();
                m.extend_from_slice(opt);
                m.extend_from_slice(b"\": must be -ascii, -decreasing, -increasing, -integer, -nocase, -real, or -unique");
                return interp.set_error(&m);
            }
            _ => break,
        }
        i += 1;
    }
    if argv.len() - i != 1 {
        return wrong_args(interp, b"lsort ?-option ...? list");
    }
    let elems = match list::list_elements(argv[i]) {
        Ok(v) => v,
        Err(e) => return bad_list(interp, e),
    };
    // Decorate each element with its sort key.
    let mut items: Vec<(*mut TclObj, Vec<u8>)> = elems.iter().map(|&e| (e, obj_bytes(e))).collect();
    let cmp = |a: &(*mut TclObj, Vec<u8>), b: &(*mut TclObj, Vec<u8>)| -> core::cmp::Ordering {
        use core::cmp::Ordering;
        match kind {
            Kind::Ascii => {
                if nocase {
                    a.1.to_ascii_lowercase().cmp(&b.1.to_ascii_lowercase())
                } else {
                    a.1.cmp(&b.1)
                }
            }
            Kind::Integer => parse_isize(&a.1)
                .unwrap_or(0)
                .cmp(&parse_isize(&b.1).unwrap_or(0)),
            Kind::Real => {
                let fa = core::str::from_utf8(&a.1)
                    .ok()
                    .and_then(|s| s.trim().parse::<f64>().ok())
                    .unwrap_or(0.0);
                let fb = core::str::from_utf8(&b.1)
                    .ok()
                    .and_then(|s| s.trim().parse::<f64>().ok())
                    .unwrap_or(0.0);
                fa.partial_cmp(&fb).unwrap_or(Ordering::Equal)
            }
        }
    };
    items.sort_by(cmp);
    if decreasing {
        items.reverse();
    }
    if unique {
        items.dedup_by(|a, b| a.1 == b.1);
    }
    let out: Vec<*mut TclObj> = items.iter().map(|(e, _)| *e).collect();
    set_list(interp, &out);
    Code::Ok
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
