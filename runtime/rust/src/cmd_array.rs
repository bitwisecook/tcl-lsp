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

//! `array` — the array-variable ensemble (toward running tcltest; used ~30×).
//! C ref `tclVar.c` (`Tcl_ArrayObjCmd`). Operates on the array variables the
//! frame/namespace var tables already hold (`a(key)`).
//!
//! Implemented: `set`/`get`/`names`/`exists`/`size`/`unset`. (`statistics`,
//! `nextelement`/`startsearch` searches, `-exact`/`-regexp` name modes follow.)

use crate::interp::{new_string, obj_bytes, Code, Interp};
use crate::obj::TclObj;

/// Register `array`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"array", array_cmd);
}

/// This build's `array` subcommands, in the order the unknown-subcommand
/// message lists them, each with the `argv` index its array name sits at —
/// `array default <sub> <name>` names its array one word later than the rest.
///
/// One table, two consumers: the message below and [`array_name_index`], which
/// is what makes the variable's `array` traces fire for every subcommand. A
/// subcommand added to the message without a row here would silently stop
/// being trace-visible.
const SUBCOMMANDS: &[(&[u8], usize)] = &[
    (b"default", 3),
    (b"exists", 2),
    (b"for", 2),
    (b"get", 2),
    (b"names", 2),
    (b"set", 2),
    (b"size", 2),
    (b"unset", 2),
];

/// The `argv` index of `sub`'s array name, or `None` when `sub` is not a
/// subcommand of this build — C resolves the subcommand index *before* calling
/// `LocateArray`, so an unknown subcommand fires no trace.
fn array_name_index(sub: &[u8]) -> Option<usize> {
    SUBCOMMANDS
        .iter()
        .find(|(name, _)| *name == sub)
        .map(|(_, index)| *index)
}

/// `unknown or ambiguous subcommand "x": must be a, b, or c`, over the same
/// table.
fn unknown_subcommand(interp: &mut Interp, sub: &[u8]) -> Code {
    let mut m = b"unknown or ambiguous subcommand \"".to_vec();
    m.extend_from_slice(sub);
    m.extend_from_slice(b"\": must be ");
    for (i, (name, _)) in SUBCOMMANDS.iter().enumerate() {
        if i > 0 {
            m.extend_from_slice(b", ");
        }
        if i + 1 == SUBCOMMANDS.len() {
            m.extend_from_slice(b"or ");
        }
        m.extend_from_slice(name);
    }
    interp.set_error(&m)
}

fn array_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return interp.wrong_args(b"array subcommand arrayName ?arg ...?");
    }
    let sub = obj_bytes(argv[1]);
    // C's `LocateArray` (tclVar.c:330-350) sits at the top of every `array`
    // subcommand and fires the variable's `array` traces before the subcommand
    // reads anything — `array names`, `array get`, `array set`, `array exists`
    // and the rest each fire exactly one `<name> {} array` callback, while an
    // ordinary `$arr(k)` read or `set arr(k)` write fires none. This is that
    // one site (issue #1569), not a per-subcommand branch: the array command
    // owns the hook, so the only per-subcommand fact it needs is where the
    // array name is.
    if let Some(name_obj) = array_name_index(&sub).and_then(|i| argv.get(i)) {
        if let Some(code) = interp.fire_array_trace(&obj_bytes(*name_obj)) {
            return code;
        }
    }
    let sub_str = String::from_utf8_lossy(&sub);
    // The read-side + `unset` are the shared `tcl_cmd_core::array` core (over
    // this runtime's `VarStore`/`Frames`/`ValueOps`); a fresh-or-borrowed result
    // object is retained by `set_result`.
    if let Some(result) = tcl_cmd_core::array::dispatch(interp, &sub_str, &argv[2..]) {
        return match result {
            Ok(v) => {
                interp.set_result(v);
                Code::Ok
            }
            Err(e) => interp.set_error(e.message().as_bytes()),
        };
    }
    // Per-runtime: `set` (per-element write traces), `default` (TIP 508), `for`
    // (Family-B iteration), and the unknown-subcommand message.
    let name = obj_bytes(argv[2]);
    match sub.as_slice() {
        b"set" => array_set(interp, argv, &name),
        b"for" => array_for(interp, argv),
        b"default" => array_default(interp, argv),
        other => unknown_subcommand(interp, other),
    }
}

/// `array default set|get|exists|unset arrayName ?value?` (TIP 508) — the array's
/// default value for reads of missing elements.
fn array_default(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    // argv: array default <subcmd> arrayName ?value?
    if argv.len() < 4 {
        return interp.wrong_args(b"array default subcommand arrayName ?value?");
    }
    let sub = obj_bytes(argv[2]);
    let name = obj_bytes(argv[3]);
    match sub.as_slice() {
        b"set" => {
            if argv.len() != 5 {
                return interp.wrong_args(b"array default set arrayName value");
            }
            match interp.set_array_default(&name, argv[4]) {
                Ok(()) => {
                    interp.set_result(argv[4]);
                    Code::Ok
                }
                // C: `can't array default set "ary": variable isn't array`.
                Err(_) => {
                    let mut m = b"can't array default set \"".to_vec();
                    m.extend_from_slice(&name);
                    m.extend_from_slice(b"\": variable isn't array");
                    interp.set_error(&m)
                }
            }
        }
        b"get" => {
            if argv.len() != 4 {
                return interp.wrong_args(b"array default get arrayName");
            }
            // Missing var or scalar both error (C: `!varPtr || undefined || !isArray`).
            if !interp.var_is_array(&name) {
                return not_array(interp, &name);
            }
            match interp.array_default(&name) {
                Some(o) => {
                    interp.set_result(o);
                    Code::Ok
                }
                None => {
                    interp.error_with_code(b"array has no default value", b"TCL READ ARRAY DEFAULT")
                }
            }
        }
        b"exists" => {
            if argv.len() != 4 {
                return interp.wrong_args(b"array default exists arrayName");
            }
            // An undefined variable has no default — not an error (C).
            if !interp.var_exists(&name) {
                interp.set_result_bytes(b"0");
            } else if !interp.var_is_array(&name) {
                return not_array(interp, &name);
            } else {
                interp.set_result_bytes(if interp.array_default(&name).is_some() {
                    b"1"
                } else {
                    b"0"
                });
            }
            Code::Ok
        }
        b"unset" => {
            if argv.len() != 4 {
                return interp.wrong_args(b"array default unset arrayName");
            }
            // A missing variable is a silent no-op; a scalar errors (C).
            if interp.var_exists(&name) {
                if !interp.var_is_array(&name) {
                    return not_array(interp, &name);
                }
                interp.unset_array_default(&name);
            }
            interp.set_result_bytes(b"");
            Code::Ok
        }
        other => {
            let mut m = b"unknown or ambiguous subcommand \"".to_vec();
            m.extend_from_slice(other);
            m.extend_from_slice(b"\": must be exists, get, set, or unset");
            interp.set_error(&m)
        }
    }
}

/// `array for {key value} arrayName script` — iterate the array's elements,
/// binding the two variables and running `script` each time (C's `ArrayForNRCmd`
/// / `ArrayForLoopCallback`). A structural change to the array (an element added
/// or removed) during iteration is an error, matching the invalidated hash
/// search; changing an existing element's value is fine.
fn array_for(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 5 {
        return interp.wrong_args(b"array for {key value} arrayName script");
    }
    let varlist = obj_bytes(argv[2]);
    let vars = match crate::parse::split_list(&varlist) {
        Ok(v) => v,
        Err(e) => return interp.set_error(e.message()),
    };
    if vars.len() != 2 {
        return interp.error_with_code(b"must have two variable names", b"TCL SYNTAX array for");
    }
    let kvar = vars[0].clone();
    let vvar = vars[1].clone();
    let name = obj_bytes(argv[3]);
    if !interp.var_is_array(&name) {
        return not_array(interp, &name);
    }
    let body = argv[4];

    // Snapshot the element names; the iteration order is the snapshot order and a
    // change to the *set* of keys (not their values) aborts the loop.
    let snapshot = interp.array_names(&name).unwrap_or_default();
    let snapshot_set: std::collections::BTreeSet<Vec<u8>> = snapshot.iter().cloned().collect();

    for idx in 0..=snapshot.len() {
        // Detect a structural change since the snapshot (C's search invalidation).
        let current: std::collections::BTreeSet<Vec<u8>> = interp
            .array_names(&name)
            .unwrap_or_default()
            .into_iter()
            .collect();
        if current != snapshot_set {
            return interp
                .error_with_code(b"array changed during iteration", b"TCL READ array for");
        }
        if idx == snapshot.len() {
            break;
        }
        let key = &snapshot[idx];
        // Read the value through the trace-firing path (var-23.13 counts reads).
        if let Some(c) = interp.fire_read_trace(&name, Some(key)) {
            return c;
        }
        let Some(value) = interp.var_get_elem(&name, key) else {
            continue; // element became undefined mid-iteration
        };
        let ko = new_string(key);
        if let Err(e) = interp.var_set(&kvar, ko) {
            crate::interp::drop_fresh(ko);
            return crate::builtins::var_error(interp, &kvar, e);
        }
        // `value` is borrowed from the store; `var_set` retains it (no drop here).
        if let Err(e) = interp.var_set(&vvar, value) {
            return crate::builtins::var_error(interp, &vvar, e);
        }
        match interp.eval_control_body(body) {
            Code::Ok | Code::Continue => {}
            Code::Break => break,
            Code::Error => {
                if !interp.in_proc() {
                    interp.append_body_frame(b"array for");
                }
                return Code::Error;
            }
            other => return other,
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `"<name>" isn't an array`.
fn not_array(interp: &mut Interp, name: &[u8]) -> Code {
    let mut m = b"\"".to_vec();
    m.extend_from_slice(name);
    m.extend_from_slice(b"\" isn't an array");
    interp.set_error(&m)
}

/// `array set arrayName {key value …}` — store each pair as an element.
fn array_set(interp: &mut Interp, argv: &[*mut TclObj], name: &[u8]) -> Code {
    if argv.len() != 4 {
        return interp.wrong_args(b"array set arrayName list");
    }
    // An array-element name (`foo(bar)`) can't be the target of `array set`.
    if crate::frame::split_array_ref(name).1.is_some() {
        let mut m = b"can't set \"".to_vec();
        m.extend_from_slice(name);
        m.extend_from_slice(b"\": variable isn't array");
        return interp.set_error(&m);
    }
    // Read the *element objects* (not a re-split into fresh strings) so each
    // value keeps its `Tcl_Obj` identity through the array — C shares objs by
    // reference, and TIP 280 keys a literal's source location on that identity
    // (so a `-body {…}` stored via `array set` still evaluates as `type source`).
    let kvs = match crate::list::list_elements(argv[3]) {
        Ok(v) => v,
        Err(e) => return interp.set_error(e.message()),
    };
    if kvs.len() % 2 != 0 {
        return interp.set_error(b"list must have an even number of elements");
    }
    // `array set a {}` still materialises an empty array (and a scalar `a`
    // errors `variable isn't array`), so ensure the array up front — the loop
    // below never runs for an empty value list.
    if let Err(e) = interp.ensure_array(name) {
        return crate::builtins::var_error(interp, name, e);
    }
    for pair in kvs.chunks_exact(2) {
        // `var_set_elem` retains the live value obj (no fresh allocation).
        let key = obj_bytes(pair[0]);
        if let Err(e) = interp.var_set_elem(name, &key, pair[1]) {
            return crate::builtins::var_error(interp, name, e);
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

#[cfg(test)]
mod tests {
    use crate::counters;
    use crate::interp::{Code, Interp};

    fn leak_free(body: impl FnOnce(&mut Interp)) {
        counters::reset();
        {
            let mut interp = Interp::new();
            body(&mut interp);
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

    fn run(i: &mut Interp, src: &[u8]) -> Vec<u8> {
        assert_eq!(
            i.eval_str(src),
            Code::Ok,
            "eval {:?}",
            String::from_utf8_lossy(src)
        );
        i.result_bytes()
    }

    #[test]
    fn array_set_get_names_size() {
        leak_free(|i| {
            assert_eq!(run(i, b"array exists a"), b"0");
            run(i, b"array set a {x 1 y 2 z 3}");
            assert_eq!(run(i, b"array exists a"), b"1");
            assert_eq!(run(i, b"array size a"), b"3");
            assert_eq!(run(i, b"array names a"), b"x y z"); // sorted (BTreeMap)
            assert_eq!(run(i, b"array names a {[xy]}"), b"x y");
            assert_eq!(run(i, b"set a(y)"), b"2");
            // array get is a flat key/value list (sorted by key).
            assert_eq!(run(i, b"array get a"), b"x 1 y 2 z 3");
            i.eval_str(b"unset a");
        });
    }

    #[test]
    fn array_get_preserves_parentheses_in_the_array_base() {
        leak_free(|i| {
            for name in [b"z(b".as_slice(), b"z)b", b"z(b)c"] {
                let mut script = b"array set {".to_vec();
                script.extend_from_slice(name);
                script.extend_from_slice(b"} {a 1 b 2}; array get {");
                script.extend_from_slice(name);
                script.push(b'}');
                assert_eq!(run(i, &script), b"a 1 b 2", "base {name:?}");
            }
        });
    }

    #[test]
    fn array_unset() {
        leak_free(|i| {
            run(i, b"array set a {x 1 y 2 z 3}");
            run(i, b"array unset a y");
            assert_eq!(run(i, b"array names a"), b"x z");
            run(i, b"array unset a"); // whole array
            assert_eq!(run(i, b"array exists a"), b"0");
        });
    }
}
