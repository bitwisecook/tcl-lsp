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

//! The `dict` ensemble (T1.6 + M4) — `create`/`get`/`getdef`/`set`/`replace`/
//! `remove`/`exists`/`unset`/`size`/`keys`/`values`/`merge`/`filter`/`for`/
//! `map`/`update`/`with`/`append`/`lappend`/`incr`, over the [`crate::dict`]
//! value type. (`info` follows.)
//!
//! `dict set`/`unset`/`update`/`with` mutate a dict **variable** (copy-on-write,
//! like `lappend`); the rest read dict **values**. `get`/`exists`/`getdef` take
//! a key *path* (nested dicts); `keys`/`values`/`filter` glob-filter.
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::dict;
use crate::frame::VarError;
use crate::interp::{obj_bytes, Code, Interp};
use crate::obj::{self, TclObj};
use crate::parse;

/// Register the `dict` ensemble.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"dict", dict_cmd);
}

fn dict_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args(b"dict subcommand ?arg ...?");
    }
    let sub = obj_bytes(argv[1]);
    // Pure dict subcommands now live in the shared command core; the runtime is
    // a thin adapter. Variable-mutating subcommands fall through to the legacy
    // match below.
    if let Ok(sub_str) = std::str::from_utf8(&sub) {
        if let Some(result) = tcl_cmd_core::dict::dispatch_canon(interp, sub_str, &argv[2..]) {
            return match result {
                Ok(v) => {
                    interp.set_result(v);
                    Code::Ok
                }
                // The shared core reports errors by message only; re-attach C's
                // `-errorcode` for the dict-parse failures so `catch … optsVar`
                // sees `TCL VALUE DICTIONARY …` and not `NONE` (dict-4.x).
                Err(e) => {
                    let msg = e.message().as_bytes();
                    match dict_parse_error_code(e.message()) {
                        Some(code) => interp.error_with_code(msg, code),
                        None => interp.set_error(msg),
                    }
                }
            };
        }
    }
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
        b"filter" => filter(interp, argv),
        b"for" => for_(interp, argv),
        b"map" => map(interp, argv),
        b"update" => update(interp, argv),
        b"with" => with(interp, argv),
        b"append" => append(interp, argv),
        b"lappend" => lappend(interp, argv),
        b"incr" => incr(interp, argv),
        b"info" => info(interp, argv),
        _ => {
            let mut m = b"unknown or ambiguous subcommand \"".to_vec();
            m.extend_from_slice(&sub);
            m.extend_from_slice(
                b"\": must be append, create, exists, filter, for, get, getdef, getwithdefault, incr, info, keys, lappend, map, merge, remove, replace, set, size, unset, update, values, or with",
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
        return interp.wrong_args(b"dict create ?key value ...?");
    }
    let pairs: Vec<(*mut TclObj, *mut TclObj)> =
        rest.chunks_exact(2).map(|c| (c[0], c[1])).collect();
    interp.set_result(dict::new_dict_obj(&pairs));
    Code::Ok
}

/// `dict get dictValue ?key?` — the value for `key`, or the whole dict if no key.
fn get(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return interp.wrong_args(b"dict get dictionary ?key ...?");
    }
    if argv.len() == 3 {
        interp.set_result(argv[2]); // whole dict
        return Code::Ok;
    }
    // Drill the key path: each key descends one nested dict (the value at each
    // step is owned by its parent, alive up the chain to `argv[2]`).
    let mut cur = argv[2];
    for &k in &argv[3..] {
        let key = obj_bytes(k);
        match dict::dict_get(cur, &key) {
            Ok(Some(v)) => cur = v,
            Ok(None) => return key_not_known(interp, &key),
            Err(e) => return bad_dict(interp, e),
        }
    }
    interp.set_result(cur);
    Code::Ok
}

/// `dict exists dictValue key`
fn exists(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 {
        return interp.wrong_args(b"dict exists dictionary key ?key ...?");
    }
    // Drill the key path; a missing key or a non-dict along the way → 0 (Tcl
    // `dict exists` reports false rather than erroring).
    let keys = &argv[3..];
    let mut cur = argv[2];
    for (i, &k) in keys.iter().enumerate() {
        let key = obj_bytes(k);
        match dict::dict_exists(cur, &key) {
            Ok(true) if i + 1 == keys.len() => {
                interp.set_result_bytes(b"1");
                return Code::Ok;
            }
            Ok(true) => match dict::dict_get(cur, &key) {
                Ok(Some(v)) => cur = v,
                _ => break,
            },
            _ => break,
        }
    }
    interp.set_result_bytes(b"0");
    Code::Ok
}

/// `dict info dictionary` — a human-readable description (its exact form is
/// unspecified; we report the entry count). Validates the dict first.
fn info(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return interp.wrong_args(b"dict info dictionary");
    }
    match dict::dict_pairs(argv[2]) {
        Ok(pairs) => {
            let s = format!("{} entries in table", pairs.len());
            interp.set_result_bytes(s.as_bytes());
            Code::Ok
        }
        Err(e) => bad_dict(interp, e),
    }
}

/// `dict size dictionary`
fn size(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return interp.wrong_args(b"dict size dictionary");
    }
    match dict::dict_size(argv[2]) {
        Ok(n) => {
            interp.set_result(obj::new_wide_int_obj(n as i64));
            Code::Ok
        }
        Err(e) => bad_dict(interp, e),
    }
}

/// Glob-filter `items` by the optional pattern at `argv[3]` (`keys`/`values`
/// share this), setting the result to the matching list.
fn glob_filtered_result(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    items: Vec<*mut TclObj>,
) -> Code {
    let filtered: Vec<*mut TclObj> = match argv.get(3) {
        Some(&p) => {
            let pat = obj_bytes(p);
            let pat_s = String::from_utf8_lossy(&pat);
            items
                .into_iter()
                .filter(|&o| {
                    tcl_syntax::glob::string_match(&pat_s, &String::from_utf8_lossy(&obj_bytes(o)))
                })
                .collect()
        }
        None => items,
    };
    interp.set_result(crate::list::new_list_obj(&filtered));
    Code::Ok
}

/// `dict keys dictionary ?pattern?` — keys in insertion order, glob-filtered.
fn keys(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 || argv.len() > 4 {
        return interp.wrong_args(b"dict keys dictionary ?pattern?");
    }
    match dict::dict_keys(argv[2]) {
        Ok(ks) => glob_filtered_result(interp, argv, ks),
        Err(e) => bad_dict(interp, e),
    }
}

/// `dict values dictionary ?pattern?` — values in insertion order, glob-filtered.
fn values(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 || argv.len() > 4 {
        return interp.wrong_args(b"dict values dictionary ?pattern?");
    }
    match dict::dict_pairs(argv[2]) {
        Ok(pairs) => {
            let vs: Vec<*mut TclObj> = pairs.iter().map(|&(_, v)| v).collect();
            glob_filtered_result(interp, argv, vs)
        }
        Err(e) => bad_dict(interp, e),
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
            Err(e) => {
                unsafe { obj::decr_ref_count(acc) };
                return bad_dict(interp, e);
            }
        };
        for (k, v) in pairs {
            // acc is unshared (we hold the only ref) → in-place set is sound.
            if let Err(e) = dict::dict_set(acc, k, v) {
                unsafe { obj::decr_ref_count(acc) };
                return bad_dict(interp, e);
            }
        }
    }
    interp.set_result(acc); // retains acc into the result
    unsafe { obj::decr_ref_count(acc) }; // drop our build-time ref
    Code::Ok
}

/// Copy `src`'s pairs into a fresh owned dict (rc 1, caller balances) for
/// in-place mutation. `None` (after stamping `bad_dict`) on a malformed dict.
fn copy_dict(interp: &mut Interp, src: *mut TclObj) -> Option<*mut TclObj> {
    let pairs = match dict::dict_pairs(src) {
        Ok(p) => p,
        Err(e) => {
            bad_dict(interp, e);
            return None;
        }
    };
    let acc = dict::new_dict_obj(&[]);
    unsafe { obj::incr_ref_count(acc) };
    for (k, v) in pairs {
        let _ = dict::dict_set(acc, k, v);
    }
    Some(acc)
}

/// `dict filter dictionary key|value ?globPattern ...?` (glob forms) or
/// `dict filter dictionary script {keyVar valueVar} body` (predicate form):
/// the entries kept, as a new dict.
fn filter(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    // Only the `script` filter type reaches here: the shared core
    // (`tcl_cmd_core::dict`) handles `key`/`value` (pure glob), the bad-filterType
    // error, and the missing-filterType arg error, so `dispatch_canon` returns
    // `None` only for `script`. `argv` is therefore `[string filter dict script
    // …]` (≥ 4). Error order matches the original: the dict is parsed before the
    // script-form arg-count check.
    let pairs = match dict::dict_pairs(argv[2]) {
        Ok(p) => p,
        Err(e) => return bad_dict(interp, e),
    };
    if argv.len() != 6 {
        return interp
            .wrong_args(b"dict filter dictionary script {keyVarName valueVarName} filterScript");
    }
    // The two variable names are parsed as a *list*: a malformed list surfaces
    // the list parse error verbatim (dict-17.20), then a count other than two is
    // the dict-filter syntax error (dict-17.19).
    let vars = match crate::parse::split_list(&obj_bytes(argv[4])) {
        Ok(v) => v,
        Err(e) => return interp.set_error(e.message()),
    };
    if vars.len() != 2 {
        return interp.error_with_code(
            b"must have exactly two variable names",
            b"TCL SYNTAX dict filter",
        );
    }
    let mut kept: Vec<(*mut TclObj, *mut TclObj)> = Vec::new();
    for (k, v) in pairs {
        if interp.var_set(&vars[0], k).is_err() || interp.var_set(&vars[1], v).is_err() {
            return interp.set_error(b"couldn't set dict filter variable");
        }
        // The body's completion code drives the loop (C's `DictFilterCmd` script
        // case): OK ⇒ keep iff its result is true; CONTINUE ⇒ skip; BREAK ⇒ stop,
        // return what's kept; ERROR / RETURN / other ⇒ propagate.
        match interp.eval_control_body(argv[5]) {
            Code::Ok => {
                // The body result is coerced with the interpreter's canonical
                // Tcl boolean parser (the one `if`/`while`/`expr` use), so a
                // numeric false like `0x0`/`0.0` drops the pair and a
                // non-boolean result raises `expected boolean value` — matching
                // C's `Tcl_GetBooleanFromObj` rather than a loose string test
                // (RUST_ISSUE_092).
                match dict_filter_bool(interp.get_obj_result()) {
                    Ok(true) => kept.push((k, v)),
                    Ok(false) => {}
                    Err(msg) => return interp.set_error(&msg),
                }
            }
            Code::Continue => {}
            Code::Break => break,
            other => return other,
        }
    }
    interp.set_result(dict::new_dict_obj(&kept));
    Code::Ok
}

/// Coerce a `dict filter -filter script` body result to a boolean like C's
/// `Tcl_GetBooleanFromObj`, returning the C error message bytes on a non-boolean
/// (`RUST_ISSUE_092`). On the numeric-tower build this is the canonical `expr`
/// parser (arbitrary-precision non-zero test); the degraded no-`tommath` wasm
/// build (where `expr` is compiled out) falls back to the boolean keywords plus a
/// `parse_whole` numeric non-zero test — enough to keep `dict filter` building and
/// working for the common results in that reduced runtime.
#[cfg(have_tommath)]
fn dict_filter_bool(o: *mut TclObj) -> Result<bool, Vec<u8>> {
    crate::expr::to_bool(o).map_err(|e| e.msg)
}

#[cfg(not(have_tommath))]
fn dict_filter_bool(o: *mut TclObj) -> Result<bool, Vec<u8>> {
    use tcl_syntax::number::{parse_whole, Number};
    let bytes = obj_bytes(o);
    let s = core::str::from_utf8(&bytes).unwrap_or("");
    let trimmed = s.trim();
    match trimmed.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => return Ok(true),
        "0" | "false" | "no" | "off" => return Ok(false),
        _ => {}
    }
    if let Some(n) = parse_whole(trimmed) {
        match n {
            Number::Int(v) => return Ok(v != 0),
            Number::Double(d) => return Ok(d != 0.0),
            // A bignum is zero iff every magnitude digit is `0`.
            Number::Big { digits, .. } => return Ok(digits.bytes().any(|b| b != b'0')),
            // NaN is not a valid Tcl boolean — fall through to the error.
            Number::Nan { .. } => {}
        }
    }
    let mut m = b"expected boolean value but got ".to_vec();
    m.extend_from_slice(tcl_syntax::list::describe_bad_value(s).as_bytes());
    Err(m)
}

// -- variable-mutating subcommands (copy-on-write) -------------------------

/// The dict object to mutate for `dictVar`: the variable's (mutated in place if
/// unshared), a COW copy, or a fresh empty dict. Returns `(obj, is_new)`.
/// Read the dict-variable `name`, splitting an `arr(elem)` reference so a dict
/// command can mutate an array element (and pick up a TIP 508 array default).
fn dict_var_get(interp: &Interp, name: &[u8]) -> Option<*mut TclObj> {
    let (base, elem) = crate::frame::split_array_ref(name);
    match elem {
        Some(k) => interp.var_get_elem(&base, &k),
        None => interp.var_get(&base),
    }
}

/// Write the dict-variable `name`, splitting an `arr(elem)` reference.
fn dict_var_set(interp: &mut Interp, name: &[u8], obj: *mut TclObj) -> Result<(), VarError> {
    let (base, elem) = crate::frame::split_array_ref(name);
    match elem {
        Some(k) => interp.var_set_elem(&base, &k, obj),
        None => interp.var_set(&base, obj),
    }
}

fn working_dict(interp: &mut Interp, name: &[u8]) -> Result<(*mut TclObj, bool), Code> {
    // A constant cannot be the target of a mutating dict op; reject before the
    // in-place update would bypass the store-time check.
    if let Some(c) = interp.const_write_check(name) {
        return Err(c);
    }
    Ok(match dict_var_get(interp, name) {
        None => (dict::new_dict_obj(&[]), true),
        Some(o) if obj::is_shared(o) => (obj::duplicate(o), true),
        Some(o) => (o, false),
    })
}

/// Store a freshly built dict back into `dictVar` (when `is_new`) and set it as
/// the result.
fn store_dict(interp: &mut Interp, name: &[u8], target: *mut TclObj, is_new: bool) -> Code {
    if is_new && dict_var_set(interp, name, target).is_err() {
        drop_fresh(target);
        return cant_set(interp, name);
    }
    interp.set_result(target);
    Code::Ok
}

/// `dict append dictVarName key ?value ...?` — string-append to the key's value.
fn append(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 {
        return interp.wrong_args(b"dict append dictVarName key ?value ...?");
    }
    let name = obj_bytes(argv[2]);
    let key = argv[3];
    let (target, is_new) = match working_dict(interp, &name) {
        Ok(x) => x,
        Err(c) => return c,
    };
    let mut buf = match dict::dict_get(target, &obj_bytes(key)) {
        Ok(Some(v)) => obj_bytes(v),
        _ => Vec::new(),
    };
    for &s in &argv[4..] {
        buf.extend_from_slice(&obj_bytes(s));
    }
    let val = crate::interp::new_string(&buf); // rc 0; dict_set retains
    if let Err(e) = dict::dict_set(target, key, val) {
        drop_fresh(val);
        if is_new {
            drop_fresh(target);
        }
        return bad_dict(interp, e);
    }
    store_dict(interp, &name, target, is_new)
}

/// `dict lappend dictVarName key ?value ...?` — list-append to the key's value.
fn lappend(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 {
        return interp.wrong_args(b"dict lappend dictVarName key ?value ...?");
    }
    let name = obj_bytes(argv[2]);
    let key = argv[3];
    let (target, is_new) = match working_dict(interp, &name) {
        Ok(x) => x,
        Err(c) => return c,
    };
    let mut elems: Vec<*mut TclObj> = match dict::dict_get(target, &obj_bytes(key)) {
        Ok(Some(v)) => match crate::list::list_elements(v) {
            Ok(e) => e,
            // The existing value must be a valid list (e.g. `{` is not).
            Err(e) => {
                if is_new {
                    drop_fresh(target);
                }
                return interp.set_error(e.message());
            }
        },
        _ => Vec::new(),
    };
    elems.extend_from_slice(&argv[4..]);
    let val = crate::list::new_list_obj(&elems); // rc 0; dict_set retains
    if let Err(e) = dict::dict_set(target, key, val) {
        drop_fresh(val);
        if is_new {
            drop_fresh(target);
        }
        return bad_dict(interp, e);
    }
    store_dict(interp, &name, target, is_new)
}

/// `dict incr dictVarName key ?increment?` — integer-add to the key's value.
///
/// Both the stored value and the increment are read through the shared
/// bignum-aware `int_add` seam (the one scalar `incr` uses), so a radix literal
/// such as `0x10` is accepted and a sum past `i64::MAX` widens to a bignum
/// instead of wrapping — matching C's `TclIncrObj` rather than a decimal-only
/// `wrapping_add` (RUST_ISSUE_093 / 166).
fn incr(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 || argv.len() > 5 {
        return interp.wrong_args(b"dict incr dictVarName key ?increment?");
    }
    let name = obj_bytes(argv[2]);
    let key = argv[3];
    let (target, is_new) = match working_dict(interp, &name) {
        Ok(x) => x,
        Err(c) => return c,
    };
    // Current value object (borrowed from `target`), or `None` when the key is
    // absent — the seam then treats it as 0, as C's `dict incr` does.
    let cur = match dict::dict_get(target, &obj_bytes(key)) {
        Ok(Some(v)) => Some(v),
        _ => None,
    };
    let one = obj::new_wide_int_obj(1);
    let amount = if argv.len() == 5 { argv[4] } else { one };
    // Coercion order matches C: the current value first, then the increment.
    let sum = tcl_syntax::value::ValueOps::int_add(interp, cur.as_ref(), &amount);
    drop_fresh(one); // the transient `1` (used or not) is no longer needed
    let sum = match sum {
        Ok(s) => s, // rc 0
        Err(e) => {
            if is_new {
                drop_fresh(target);
            }
            return interp.set_error(e.message().as_bytes());
        }
    };
    if let Err(e) = dict::dict_set(target, key, sum) {
        drop_fresh(sum);
        if is_new {
            drop_fresh(target);
        }
        return bad_dict(interp, e);
    }
    store_dict(interp, &name, target, is_new)
}

/// `dict set dictVarName key ?key ...? value` — set the value at a (possibly
/// nested) key path in the dict held by the variable, creating intermediate
/// dicts along the path as needed.
fn set(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 5 {
        return interp.wrong_args(b"dict set dictVarName key ?key ...? value");
    }
    let name = obj_bytes(argv[2]);
    let keys = &argv[3..argv.len() - 1];
    let value = argv[argv.len() - 1];
    if let Some(c) = interp.const_write_check(&name) {
        return c;
    }
    let (target, is_new) = match dict_var_get(interp, &name) {
        None => (dict::new_dict_obj(&[]), true),
        Some(o) if obj::is_shared(o) => (obj::duplicate(o), true),
        Some(o) => (o, false),
    };
    if let Err(e) = dict_path_set(target, keys, value) {
        if is_new {
            drop_fresh(target);
        }
        return bad_dict(interp, e);
    }
    if is_new && dict_var_set(interp, &name, target).is_err() {
        drop_fresh(target);
        return cant_set(interp, &name);
    }
    interp.set_result(target);
    Code::Ok
}

/// Set `value` at the key path `keys` (len ≥ 1) within the **unshared** dict
/// `dict`, descending through (and copy-on-write replacing) intermediate
/// sub-dicts, creating empty ones for missing path segments.
fn dict_path_set(
    dict: *mut TclObj,
    keys: &[*mut TclObj],
    value: *mut TclObj,
) -> Result<(), dict::DictError> {
    if let [last] = keys {
        return dict::dict_set(dict, *last, value);
    }
    let (head, rest) = keys.split_first().expect("len >= 2 here");
    // The sub-dict for `head`: an unshared one we can mutate (copy a shared one,
    // create an empty one for a missing segment).
    let (sub, sub_fresh) = match dict::dict_get(dict, &obj_bytes(*head))? {
        Some(s) if !obj::is_shared(s) => (s, false),
        Some(s) => (obj::duplicate(s), true),    // rc 0
        None => (dict::new_dict_obj(&[]), true), // rc 0
    };
    // A freshly duplicated / created `sub` (rc 0) is only retained by the
    // `dict_set` re-bind below; if the recursive descent errors first, drop it
    // here so it is not orphaned (RUST_ISSUE_090).
    if let Err(e) = dict_path_set(sub, rest, value) {
        if sub_fresh {
            drop_fresh(sub);
        }
        return Err(e);
    }
    // Re-bind the (modified) sub-dict into its parent. This is required even when
    // `sub` was mutated in place: it invalidates the parent's string rep so the
    // nested change is visible up the chain.
    dict::dict_set(dict, *head, sub)
}

/// `dict unset dictVarName key ?key ...?` — remove the value at a (possibly
/// nested) key path from the dict held by the variable.
fn unset(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 {
        return interp.wrong_args(b"dict unset dictVarName key ?key ...?");
    }
    let name = obj_bytes(argv[2]);
    let keys = &argv[3..];

    if let Some(c) = interp.const_write_check(&name) {
        return c;
    }
    let (target, is_new) = match dict_var_get(interp, &name) {
        None => (dict::new_dict_obj(&[]), true),
        Some(o) if obj::is_shared(o) => (obj::duplicate(o), true),
        Some(o) => (o, false),
    };
    match dict_path_unset(target, keys) {
        Ok(()) => {}
        Err(PathErr::KeyMissing(k)) => {
            if is_new {
                drop_fresh(target);
            }
            return key_not_known(interp, &k);
        }
        Err(PathErr::Bad(e)) => {
            if is_new {
                drop_fresh(target);
            }
            return bad_dict(interp, e);
        }
    }
    if is_new && dict_var_set(interp, &name, target).is_err() {
        drop_fresh(target);
        return cant_set(interp, &name);
    }
    interp.set_result(target);
    Code::Ok
}

/// A `dict_path_unset` failure: a malformed dict on the path, or a missing
/// intermediate path segment (`key "X" not known in dictionary`).
enum PathErr {
    Bad(dict::DictError),
    KeyMissing(Vec<u8>),
}

/// Remove the value at the key path `keys` (len ≥ 1) within the **unshared**
/// dict `dict`, descending through (and re-binding) intermediate sub-dicts. A
/// missing intermediate segment errors; a missing final key is a no-op.
fn dict_path_unset(dict: *mut TclObj, keys: &[*mut TclObj]) -> Result<(), PathErr> {
    if let [last] = keys {
        dict::dict_unset(dict, &obj_bytes(*last)).map_err(PathErr::Bad)?;
        return Ok(());
    }
    let (head, rest) = keys.split_first().expect("len >= 2 here");
    let (sub, sub_fresh) = match dict::dict_get(dict, &obj_bytes(*head)).map_err(PathErr::Bad)? {
        Some(s) if !obj::is_shared(s) => (s, false),
        Some(s) => (obj::duplicate(s), true), // rc 0
        None => return Err(PathErr::KeyMissing(obj_bytes(*head))),
    };
    // Drop a freshly duplicated `sub` if the recursive descent errors before the
    // `dict_set` re-bind retains it (RUST_ISSUE_090).
    if let Err(e) = dict_path_unset(sub, rest) {
        if sub_fresh {
            drop_fresh(sub);
        }
        return Err(e);
    }
    dict::dict_set(dict, *head, sub).map_err(PathErr::Bad)
}

// -- iteration -------------------------------------------------------------

/// `dict for {keyVar valueVar} dictValue body` — iterate in insertion order,
/// evaluating `body` in the current scope with the loop vars set.
fn for_(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 5 {
        return interp.wrong_args(b"dict for {keyVarName valueVarName} dictionary script");
    }
    let var_spec = obj_bytes(argv[2]);
    let vars = match parse::split_list(&var_spec) {
        Ok(v) => v,
        Err(e) => return interp.set_error(e.message()),
    };
    if vars.len() != 2 {
        return interp.set_error(b"must have exactly two variable names");
    }
    let (kvar, vvar) = (vars[0].clone(), vars[1].clone());
    let pairs = match dict::dict_pairs(argv[3]) {
        Ok(p) => p,
        Err(e) => return bad_dict(interp, e),
    };

    for (k, v) in pairs {
        if interp.var_set(&kvar, k).is_err() {
            return cant_set(interp, &kvar);
        }
        if interp.var_set(&vvar, v).is_err() {
            return cant_set(interp, &vvar);
        }
        match interp.eval_control_body(argv[4]) {
            Code::Ok | Code::Continue => {}
            Code::Break => break,
            Code::Error => {
                if !interp.in_proc() {
                    interp.append_body_frame(b"dict for");
                }
                return Code::Error;
            }
            other => return other, // Return propagates (result already set)
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `dict map {keyVar valueVar} dictValue body` — like `dict for`, but each
/// iteration's body result becomes the new value for that key; returns the
/// transformed dict. `continue` drops the key, `break` stops.
fn map(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 5 {
        return interp.wrong_args(b"dict map {keyVarName valueVarName} dictionary script");
    }
    let vars = match parse::split_list(&obj_bytes(argv[2])) {
        Ok(v) => v,
        Err(e) => return interp.set_error(e.message()),
    };
    if vars.len() != 2 {
        return interp.set_error(b"must have exactly two variable names");
    }
    let pairs = match dict::dict_pairs(argv[3]) {
        Ok(p) => p,
        Err(e) => return bad_dict(interp, e),
    };
    let acc = dict::new_dict_obj(&[]);
    unsafe { obj::incr_ref_count(acc) };
    for (k, v) in pairs {
        if interp.var_set(&vars[0], k).is_err() || interp.var_set(&vars[1], v).is_err() {
            unsafe { obj::decr_ref_count(acc) };
            return cant_set(interp, &vars[0]);
        }
        match interp.eval_control_body(argv[4]) {
            Code::Ok => {
                let _ = dict::dict_set(acc, k, interp.get_obj_result());
            }
            Code::Continue => {}
            Code::Break => break,
            Code::Error => {
                unsafe { obj::decr_ref_count(acc) };
                if !interp.in_proc() {
                    interp.append_body_frame(b"dict map");
                }
                return Code::Error;
            }
            other => {
                unsafe { obj::decr_ref_count(acc) };
                return other;
            }
        }
    }
    interp.set_result(acc);
    unsafe { obj::decr_ref_count(acc) };
    Code::Ok
}

/// `dict update dictVar key var ?key var ...? body` — link each key's value to
/// a local var, run body, then write the (possibly changed/unset) vars back
/// into the dict variable. The body's completion is the result.
fn update(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    // [dict, update, dictVar, k, v, k, v, …, body]: at least one pair + body.
    if argv.len() < 6 || argv.len() % 2 != 0 {
        return interp.wrong_args(b"dict update dictVarName key varName ?key varName ...? script");
    }
    let dict_var = obj_bytes(argv[2]);
    let body_obj = argv[argv.len() - 1];
    let pairs_args = &argv[3..argv.len() - 1];

    let Some(d) = dict_var_get(interp, &dict_var) else {
        return no_such_var(interp, &dict_var);
    };
    // Link phase: set each local to its key's value (or unset if absent).
    for c in pairs_args.chunks_exact(2) {
        let key = obj_bytes(c[0]);
        let var = obj_bytes(c[1]);
        match dict::dict_get(d, &key) {
            Ok(Some(val)) => {
                if interp.var_set(&var, val).is_err() {
                    return cant_set(interp, &var);
                }
            }
            Ok(None) => {
                interp.var_unset(&var);
            }
            Err(e) => return bad_dict(interp, e),
        }
    }

    let code = interp.eval_control_body(body_obj);

    // Write-back: re-read the dict (the body may have replaced it), then apply
    // each local var (set if it exists, drop the key if it was unset).
    if let Some(cur) = dict_var_get(interp, &dict_var) {
        if let Some(acc) = copy_dict(interp, cur) {
            for c in pairs_args.chunks_exact(2) {
                let var = obj_bytes(c[1]);
                match interp.var_get(&var) {
                    Some(val) => {
                        let _ = dict::dict_set(acc, c[0], val);
                    }
                    None => {
                        let _ = dict::dict_unset(acc, &obj_bytes(c[0]));
                    }
                }
            }
            if dict_var_set(interp, &dict_var, acc).is_err() {
                unsafe { obj::decr_ref_count(acc) };
                return cant_set(interp, &dict_var);
            }
            unsafe { obj::decr_ref_count(acc) };
        }
    }
    code
}

/// `dict with dictVarName ?key ...? script` — map every key of the (sub-)dict to a
/// local var, run body, write the vars back. Supports a leading key path.
fn with(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 {
        return interp.wrong_args(b"dict with dictVarName ?key ...? script");
    }
    let dict_var = obj_bytes(argv[2]);
    let body_obj = argv[argv.len() - 1];
    let path = &argv[3..argv.len() - 1];

    let Some(d) = dict_var_get(interp, &dict_var) else {
        return no_such_var(interp, &dict_var);
    };
    // Navigate the optional key path to the sub-dict.
    let mut sub = d;
    for &k in path {
        match dict::dict_get(sub, &obj_bytes(k)) {
            Ok(Some(v)) => sub = v,
            Ok(None) => return key_not_known(interp, &obj_bytes(k)),
            Err(e) => return bad_dict(interp, e),
        }
    }
    let pairs = match dict::dict_pairs(sub) {
        Ok(p) => p,
        Err(e) => return bad_dict(interp, e),
    };
    // Map every key to a local var.
    let keys: Vec<Vec<u8>> = pairs.iter().map(|&(k, _)| obj_bytes(k)).collect();
    for (k, v) in &pairs {
        if interp.var_set(&obj_bytes(*k), *v).is_err() {
            return cant_set(interp, &obj_bytes(*k));
        }
    }

    let code = interp.eval_control_body(body_obj);

    // Write-back: rebuild the (sub-)dict at the key path from the mapped locals,
    // then store it back through the path. The body may have replaced the dict,
    // so re-read it; if the path no longer resolves, skip the write-back.
    if let Some(cur) = dict_var_get(interp, &dict_var) {
        // A malformed current/sub dict is a write-back error, not a silent
        // skip (the `copy_dict` call has already set the result).
        let Some(acc) = copy_dict(interp, cur) else {
            return Code::Error;
        };
        // Navigate `acc` to the sub-dict at `path`.
        let mut sub_src = acc;
        let mut reached = true;
        for &k in path {
            match dict::dict_get(sub_src, &obj_bytes(k)) {
                Ok(Some(v)) => sub_src = v,
                _ => {
                    reached = false;
                    break;
                }
            }
        }
        if reached {
            let Some(newsub) = copy_dict(interp, sub_src) else {
                unsafe { obj::decr_ref_count(acc) };
                return Code::Error;
            };
            for key in &keys {
                let kobj = crate::interp::new_string(key);
                unsafe { obj::incr_ref_count(kobj) };
                match interp.var_get(key) {
                    Some(val) => {
                        let _ = dict::dict_set(newsub, kobj, val);
                    }
                    None => {
                        let _ = dict::dict_unset(newsub, key);
                    }
                }
                unsafe { obj::decr_ref_count(kobj) };
            }
            // The rebuilt sub is the whole dict (no path) or is set back
            // at the path within `acc`.
            let store = if path.is_empty() {
                newsub
            } else {
                let _ = dict_path_set(acc, path, newsub);
                acc
            };
            if dict_var_set(interp, &dict_var, store).is_err() {
                unsafe {
                    obj::decr_ref_count(newsub);
                    obj::decr_ref_count(acc);
                }
                return cant_set(interp, &dict_var);
            }
            unsafe { obj::decr_ref_count(newsub) };
        }
        unsafe { obj::decr_ref_count(acc) };
    }
    code
}

// -- helpers ---------------------------------------------------------------

/// The C `-errorcode` for a dict string-parse failure, keyed off the message the
/// shared `tcl-cmd-core` core produced (which carries no code of its own). The
/// message set mirrors [`bad_dict`] / C's `SetDictFromAny`.
fn dict_parse_error_code(msg: &str) -> Option<&'static [u8]> {
    if msg == "missing value to go with key" {
        Some(b"TCL VALUE DICTIONARY")
    } else if msg.starts_with("dict element in braces followed by")
        || msg.starts_with("dict element in quotes followed by")
    {
        Some(b"TCL VALUE DICTIONARY JUNK")
    } else if msg == "unmatched open brace in dict" {
        Some(b"TCL VALUE DICTIONARY BRACE")
    } else if msg == "unmatched open quote in dict" {
        Some(b"TCL VALUE DICTIONARY QUOTE")
    } else {
        None
    }
}

/// Map a dict string-parse failure to its C-faithful message + `-errorcode`
/// (`SetDictFromAny`/`FindElement`, type strings `dict`/`DICTIONARY`).
fn bad_dict(interp: &mut Interp, e: crate::dict::DictError) -> Code {
    use crate::dict::DictError as E;
    let (msg, code): (Vec<u8>, &[u8]) = match e {
        E::MissingValue => (
            b"missing value to go with key".to_vec(),
            b"TCL VALUE DICTIONARY",
        ),
        E::BraceJunk(frag) => {
            let mut m = b"dict element in braces followed by \"".to_vec();
            m.extend_from_slice(&frag);
            m.extend_from_slice(b"\" instead of space");
            (m, b"TCL VALUE DICTIONARY JUNK")
        }
        E::QuoteJunk(frag) => {
            let mut m = b"dict element in quotes followed by \"".to_vec();
            m.extend_from_slice(&frag);
            m.extend_from_slice(b"\" instead of space");
            (m, b"TCL VALUE DICTIONARY JUNK")
        }
        E::UnmatchedBrace => (
            b"unmatched open brace in dict".to_vec(),
            b"TCL VALUE DICTIONARY BRACE",
        ),
        E::UnmatchedQuote => (
            b"unmatched open quote in dict".to_vec(),
            b"TCL VALUE DICTIONARY QUOTE",
        ),
        E::NotUtf8 => (
            b"missing value to go with key".to_vec(),
            b"TCL VALUE DICTIONARY",
        ),
    };
    interp.error_with_code(&msg, code)
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
fn no_such_var(interp: &mut Interp, name: &[u8]) -> Code {
    let mut m = b"can't read \"".to_vec();
    m.extend_from_slice(name);
    m.extend_from_slice(b"\": no such variable");
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
    fn replace_and_remove() {
        // `replace` upserts (existing key keeps position; new key appends).
        assert_eq!(ok(b"dict replace {a 1 b 2} b 3 c 4"), b"a 1 b 3 c 4");
        assert_eq!(ok(b"dict replace {a 1 b 2}"), b"a 1 b 2");
        // `remove` drops the named keys; a missing key is not an error.
        assert_eq!(ok(b"dict remove {a 1 b 2 c 3} b d"), b"a 1 c 3");
        assert_eq!(ok(b"dict remove {a 1 b 2}"), b"a 1 b 2");
        // An odd key/value count to `replace` is a wrong-# args error.
        let (c, _) = run(b"dict replace {a 1} b");
        assert_eq!(c, Code::Error);
    }

    #[test]
    fn getdef_returns_default_on_miss() {
        assert_eq!(ok(b"dict getdef {a 1 b 2} a X"), b"1");
        assert_eq!(ok(b"dict getdef {a 1 b 2} z X"), b"X");
        // Nested key path.
        assert_eq!(ok(b"dict getdef {a {b 1}} a b X"), b"1");
        assert_eq!(ok(b"dict getdef {a {b 1}} a z X"), b"X");
        // The alias behaves identically and echoes its own name on misuse.
        assert_eq!(ok(b"dict getwithdefault {a 1} z D"), b"D");
        let (c, b) = run(b"dict getwithdefault {a 1} z");
        assert_eq!(c, Code::Error);
        assert!(b.starts_with(b"wrong # args"));
        assert!(b.windows(14).any(|w| w == b"getwithdefault"));
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

    // Needs the numeric tower: the filter scripts test via `expr`.
    #[cfg(have_tommath)]
    #[test]
    fn filter_key_value_via_shared_core() {
        // `key`/`value` globs now come from `tcl_cmd_core::dict`.
        assert_eq!(ok(b"dict filter {a 1 b 2 aa 3} key a*"), b"a 1 aa 3");
        assert_eq!(ok(b"dict filter {a 1 b 2 aa 3} value 2"), b"b 2");
        assert_eq!(ok(b"dict filter {a 1 b 2} key"), b""); // no patterns → empty
                                                           // The filterType is validated *before* the dict is parsed (was a bug:
                                                           // a bad dict + bogus type reported the dict error first).
        let (c, b) = run(b"dict filter {a b c} bogus");
        assert_eq!(c, Code::Error);
        assert_eq!(
            b,
            b"bad filterType \"bogus\": must be key, script, or value"
        );
        // `script` stays in the runtime adapter (Family-B).
        assert_eq!(
            ok(b"dict filter {a 1 b 2 c 3} script {k v} {expr {$v > 1}}"),
            b"b 2 c 3"
        );
    }
}
