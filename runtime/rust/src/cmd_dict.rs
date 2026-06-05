//! The `dict` ensemble (T1.6) — `create`/`get`/`set`/`exists`/`unset`/`size`/
//! `keys`/`values`/`merge`/`for`, over the [`crate::dict`] value type.
//! (`append`/`lappend`/`incr`/`filter`/`map`/`update`/`with`/`info`/`replace`/
//! `remove` follow.)
//!
//! `dict set`/`unset` mutate a dict **variable** (copy-on-write, like `lappend`);
//! `dict get`/`exists`/`size`/`keys`/`values`/`merge`/`for` read dict **values**.
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::dict;
use crate::interp::{obj_bytes, Code, Interp};
use crate::obj::{self, TclObj};
use crate::parse;

/// Register the `dict` ensemble.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"dict", dict_cmd);
}

fn dict_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return wrong_args(interp, b"dict subcommand ?arg ...?");
    }
    let sub = obj_bytes(argv[1]);
    match sub.as_slice() {
        b"create" => create(interp, argv),
        b"get" => get(interp, argv),
        b"set" => set(interp, argv),
        b"exists" => exists(interp, argv),
        b"unset" => unset(interp, argv),
        b"size" => size(interp, argv),
        b"keys" => keys(interp, argv),
        b"values" => values(interp, argv),
        b"merge" => merge(interp, argv),
        b"for" => for_(interp, argv),
        _ => {
            let mut m = b"unknown or ambiguous subcommand \"".to_vec();
            m.extend_from_slice(&sub);
            m.extend_from_slice(
                b"\": must be create, exists, for, get, keys, merge, set, size, unset, or values",
            );
            interp.set_error(&m)
        }
    }
}

// -- read subcommands (operate on a dict value) ----------------------------

/// `dict create ?key value ...?`
fn create(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let rest = &argv[2..];
    if rest.len() % 2 != 0 {
        return wrong_args(interp, b"dict create ?key value ...?");
    }
    let pairs: Vec<(*mut TclObj, *mut TclObj)> =
        rest.chunks_exact(2).map(|c| (c[0], c[1])).collect();
    interp.set_result(dict::new_dict_obj(&pairs));
    Code::Ok
}

/// `dict get dictValue ?key?` — the value for `key`, or the whole dict if no key.
fn get(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    match argv.len() {
        3 => {
            interp.set_result(argv[2]); // whole dict
            Code::Ok
        }
        4 => {
            let key = obj_bytes(argv[3]);
            match dict::dict_get(argv[2], &key) {
                Ok(Some(v)) => {
                    interp.set_result(v);
                    Code::Ok
                }
                Ok(None) => key_not_known(interp, &key),
                Err(_) => bad_dict(interp),
            }
        }
        _ => wrong_args(interp, b"dict get dictValue ?key ...?"),
    }
}

/// `dict exists dictValue key`
fn exists(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 4 {
        return wrong_args(interp, b"dict exists dictValue key ?key ...?");
    }
    let key = obj_bytes(argv[3]);
    match dict::dict_exists(argv[2], &key) {
        Ok(b) => {
            interp.set_result_bytes(if b { b"1" } else { b"0" });
            Code::Ok
        }
        Err(_) => bad_dict(interp),
    }
}

/// `dict size dictValue`
fn size(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return wrong_args(interp, b"dict size dictValue");
    }
    match dict::dict_size(argv[2]) {
        Ok(n) => {
            interp.set_result(obj::new_wide_int_obj(n as i64));
            Code::Ok
        }
        Err(_) => bad_dict(interp),
    }
}

/// `dict keys dictValue` — keys in insertion order (glob pattern follows).
fn keys(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return wrong_args(interp, b"dict keys dictValue ?pattern?");
    }
    match dict::dict_keys(argv[2]) {
        Ok(ks) => {
            interp.set_result(crate::list::new_list_obj(&ks));
            Code::Ok
        }
        Err(_) => bad_dict(interp),
    }
}

/// `dict values dictValue` — values in insertion order.
fn values(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return wrong_args(interp, b"dict values dictValue ?pattern?");
    }
    match dict::dict_pairs(argv[2]) {
        Ok(pairs) => {
            let vs: Vec<*mut TclObj> = pairs.iter().map(|&(_, v)| v).collect();
            interp.set_result(crate::list::new_list_obj(&vs));
            Code::Ok
        }
        Err(_) => bad_dict(interp),
    }
}

/// `dict merge ?dictValue ...?` — left to right; later values win, first-seen
/// key position is kept.
fn merge(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let acc = dict::new_dict_obj(&[]); // rc 0
    unsafe { obj::incr_ref_count(acc) }; // own it while building
    for &d in &argv[2..] {
        let pairs = match dict::dict_pairs(d) {
            Ok(p) => p,
            Err(_) => {
                unsafe { obj::decr_ref_count(acc) };
                return bad_dict(interp);
            }
        };
        for (k, v) in pairs {
            // acc is unshared (we hold the only ref) → in-place set is sound.
            if dict::dict_set(acc, k, v).is_err() {
                unsafe { obj::decr_ref_count(acc) };
                return bad_dict(interp);
            }
        }
    }
    interp.set_result(acc); // retains acc into the result
    unsafe { obj::decr_ref_count(acc) }; // drop our build-time ref
    Code::Ok
}

// -- variable-mutating subcommands (copy-on-write) -------------------------

/// `dict set dictVarName key value` — set in the dict held by the variable.
fn set(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 5 {
        return wrong_args(interp, b"dict set dictVarName key ?key ...? value");
    }
    let name = obj_bytes(argv[2]);
    let key = argv[3];
    let value = argv[4];

    let (target, is_new) = match interp.frames.get(&name) {
        None => (dict::new_dict_obj(&[]), true),
        Some(o) if obj::is_shared(o) => (obj::duplicate(o), true),
        Some(o) => (o, false),
    };
    if dict::dict_set(target, key, value).is_err() {
        if is_new {
            drop_fresh(target);
        }
        return bad_dict(interp);
    }
    if is_new && interp.frames.set(&name, target).is_err() {
        drop_fresh(target);
        return cant_set(interp, &name);
    }
    interp.set_result(target);
    Code::Ok
}

/// `dict unset dictVarName key` — remove from the dict held by the variable.
fn unset(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 4 {
        return wrong_args(interp, b"dict unset dictVarName key ?key ...?");
    }
    let name = obj_bytes(argv[2]);
    let key = obj_bytes(argv[3]);

    let (target, is_new) = match interp.frames.get(&name) {
        None => (dict::new_dict_obj(&[]), true),
        Some(o) if obj::is_shared(o) => (obj::duplicate(o), true),
        Some(o) => (o, false),
    };
    if dict::dict_unset(target, &key).is_err() {
        if is_new {
            drop_fresh(target);
        }
        return bad_dict(interp);
    }
    if is_new && interp.frames.set(&name, target).is_err() {
        drop_fresh(target);
        return cant_set(interp, &name);
    }
    interp.set_result(target);
    Code::Ok
}

// -- iteration -------------------------------------------------------------

/// `dict for {keyVar valueVar} dictValue body` — iterate in insertion order,
/// evaluating `body` in the current scope with the loop vars set.
fn for_(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 5 {
        return wrong_args(
            interp,
            b"dict for {keyVarName valueVarName} dictValue script",
        );
    }
    let var_spec = obj_bytes(argv[2]);
    let vars = match parse::split_list(&var_spec) {
        Ok(v) if v.len() == 2 => v,
        _ => return interp.set_error(b"must have exactly two variable names"),
    };
    let (kvar, vvar) = (vars[0].clone(), vars[1].clone());
    let pairs = match dict::dict_pairs(argv[3]) {
        Ok(p) => p,
        Err(_) => return bad_dict(interp),
    };
    let body = obj_bytes(argv[4]);

    for (k, v) in pairs {
        if interp.frames.set(&kvar, k).is_err() {
            return cant_set(interp, &kvar);
        }
        if interp.frames.set(&vvar, v).is_err() {
            return cant_set(interp, &vvar);
        }
        match interp.eval_str(&body) {
            Code::Ok | Code::Continue => {}
            Code::Break => break,
            other => return other, // Return / Error propagate (result already set)
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

// -- helpers ---------------------------------------------------------------

fn wrong_args(interp: &mut Interp, usage: &[u8]) -> Code {
    let mut m = b"wrong # args: should be \"".to_vec();
    m.extend_from_slice(usage);
    m.push(b'"');
    interp.set_error(&m)
}

fn bad_dict(interp: &mut Interp) -> Code {
    interp.set_error(b"missing value to go with key")
}

fn key_not_known(interp: &mut Interp, key: &[u8]) -> Code {
    let mut m = b"key \"".to_vec();
    m.extend_from_slice(key);
    m.extend_from_slice(b"\" not known in dictionary");
    interp.set_error(&m)
}

fn cant_set(interp: &mut Interp, name: &[u8]) -> Code {
    let mut m = b"can't set \"".to_vec();
    m.extend_from_slice(name);
    m.extend_from_slice(b"\": variable is array");
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
    fn create_get_size() {
        assert_eq!(ok(b"dict create a 1 b 2"), b"a 1 b 2");
        assert_eq!(ok(b"dict get {a 1 b 2} b"), b"2");
        assert_eq!(ok(b"dict size {a 1 b 2 c 3}"), b"3");
        assert_eq!(ok(b"dict exists {a 1 b 2} b"), b"1");
        assert_eq!(ok(b"dict exists {a 1 b 2} z"), b"0");
    }

    #[test]
    fn keys_values_insertion_order() {
        assert_eq!(ok(b"dict keys {z 1 a 2 m 3}"), b"z a m"); // not sorted
        assert_eq!(ok(b"dict values {z 1 a 2 m 3}"), b"1 2 3");
    }

    #[test]
    fn set_and_unset_variable_cow() {
        assert_eq!(ok(b"dict set d a 1; dict set d b 2"), b"a 1 b 2");
        assert_eq!(ok(b"set d {a 1 b 2}; dict unset d a"), b"b 2");
        // overwrite keeps key position
        assert_eq!(
            ok(b"dict set d x 1; dict set d y 2; dict set d x 9"),
            b"x 9 y 2"
        );
    }

    #[test]
    fn merge_later_wins_first_position_kept() {
        assert_eq!(ok(b"dict merge {a 1 b 2} {b 9 c 3}"), b"a 1 b 9 c 3");
    }

    #[test]
    fn dict_for_iterates_in_order() {
        assert_eq!(
            ok(b"set out {}; dict for {k v} {a 1 b 2 c 3} { lappend out $k=$v }; set out"),
            b"a=1 b=2 c=3"
        );
    }

    #[test]
    fn get_missing_key_errors() {
        let (c, b) = run(b"dict get {a 1} z");
        assert_eq!(c, Code::Error);
        assert_eq!(b, b"key \"z\" not known in dictionary");
    }
}
