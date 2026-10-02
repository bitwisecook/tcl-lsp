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

use tcl_registry::{ArgRole, InvocationWord, InvocationWords};

use crate::interp::{new_string, obj_bytes, Code, Interp};
use crate::obj::TclObj;

/// Register `array`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"array", array_cmd);
}

/// This build's `array` subcommands, in the order the unknown-subcommand
/// message lists them. Their argument shapes and variable roles live in the
/// registry rather than a second runtime table.
const SUBCOMMANDS: &[&[u8]] = &[
    b"default", b"exists", b"for", b"get", b"names", b"set", b"size", b"unset",
];

/// The subcommand names alone, for the shared ensemble scan and its miss
/// sentence — `array` is a `TclMakeEnsemble` command, so both belong to
/// `tcl_cmd_core::ensemble` (its enumeration keeps a comma before `or`).
/// `default` and `for` are Tcl 9 additions, so the scan and the sentence both
/// take the emulated release's slice of the table — under an 8.6 pin `array f`
/// must not reach `for`, and `array d` must not be made ambiguous by
/// `default`.
fn subcommand_names(interp: &Interp) -> &'static [&'static [u8]] {
    crate::environment::release_subcommands(interp.dialect_profile().name, "array", SUBCOMMANDS)
}

/// Resolve the selected member's sole array operand through the active
/// dialect's registry facts. The returned index addresses this command's
/// complete `argv`, preserving the caller's original Tcl object for trace
/// lookup and callback spelling.
fn array_trace_target_index(interp: &Interp, argv: &[*mut TclObj], sub: &[u8]) -> Option<usize> {
    let canonical = core::str::from_utf8(sub).ok()?;
    let words: Vec<InvocationWord<'_>> = std::iter::once(InvocationWord::Literal(canonical))
        .chain(argv[2..].iter().map(|_| InvocationWord::Dynamic))
        .collect();
    let profile = interp.dialect_profile();
    let resolved = crate::environment::store_for_profile(profile)
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal("array"), &words),
            Some(crate::environment::surface_point(profile)),
        )
        .resolved()?;
    if resolved.subcommand.resolved()?.canonical_name != canonical {
        return None;
    }
    resolved
        .facts()
        .sole_argument_index_for_roles(words.len(), &[ArgRole::VarRead, ArgRole::VarWrite])
        .and_then(|index| index.checked_add(1))
}

fn array_for_variables(interp: &mut Interp, argv: &[*mut TclObj]) -> Result<[Vec<u8>; 2], Code> {
    let varlist = obj_bytes(argv[2]);
    let variables = crate::parse::split_list(&varlist)
        .map_err(|error| interp.report_list_error(&varlist, error))?;
    variables.try_into().map_err(|_| {
        interp.error_with_code(b"must have two variable names", b"TCL SYNTAX array for")
    })
}

fn array_default_option(interp: &mut Interp, argv: &[*mut TclObj]) -> Result<&'static [u8], Code> {
    // TIP 508's own option table (`tclVar.c`), resolved with
    // `Tcl_GetIndexFromObj(…, "option", 0)` in C table order.
    const OPTIONS: tcl_cmd_core::prefix::OptionTable<'static, &[u8]> =
        tcl_cmd_core::prefix::OptionTable::abbreviating(
            "option",
            &[b"get", b"set", b"exists", b"unset"],
        );
    let option = obj_bytes(argv[2]);
    OPTIONS
        .index_of_cmd(&option)
        .map(|index| OPTIONS.names()[index])
        .map_err(|error| interp.report_cmd_error(error))
}

fn unknown_subcommand(interp: &mut Interp, sub: &[u8]) -> Code {
    let names = subcommand_names(interp);
    interp.set_error(&tcl_cmd_core::ensemble::unknown_subcommand_message(
        names,
        sub,
        true,
        b"::tcl::array",
    ))
}

fn array_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args(b"array subcommand ?arg ...?");
    }
    let word = obj_bytes(argv[1]);
    // Resolve the subcommand first — exact match, else a unique prefix — so
    // `array e a` reaches `exists` *and* fires its `array` trace under the
    // canonical name, as C does.
    let names = subcommand_names(interp);
    let sub: &[u8] = match tcl_cmd_core::ensemble::resolve_subcommand(names, &word, true) {
        Some(index) => names[index],
        None => return unknown_subcommand(interp, &word),
    };
    let trace_target_index = array_trace_target_index(interp, argv, sub);
    let mut for_variables = None;
    let mut default_option = None;
    if let Some(index) = trace_target_index {
        // Every member validates its outer arity before `LocateArray`. Two
        // content checks also precede it in C: `array for` validates its
        // two-variable list and `array default` resolves its inner option.
        // The selected default option's narrower arity is intentionally later.
        match sub {
            b"for" => match array_for_variables(interp, argv) {
                Ok(variables) => for_variables = Some(variables),
                Err(code) => return code,
            },
            b"default" => match array_default_option(interp, argv) {
                Ok(option) => default_option = Some(option),
                Err(code) => return code,
            },
            _ => {}
        }
        let Some(name_obj) = argv.get(index) else {
            return interp.set_error(b"array subcommand has incomplete registry metadata");
        };
        let name = obj_bytes(*name_obj);
        return interp.with_array_trace_target(&name, |interp, target| {
            array_cmd_after_trace(
                interp,
                argv,
                sub,
                for_variables,
                default_option,
                Some(target),
            )
        });
    }
    array_cmd_after_trace(interp, argv, sub, for_variables, default_option, None)
}

fn array_cmd_after_trace(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    sub: &[u8],
    for_variables: Option<[Vec<u8>; 2]>,
    default_option: Option<&'static [u8]>,
    target: Option<&tcl_runtime_api::ArrayTarget>,
) -> Code {
    let sub_str = String::from_utf8_lossy(sub);
    // The read-side + `unset` are the shared `tcl_cmd_core::array` core (over
    // this runtime's `VarStore`/`Frames`/`ValueOps`); a fresh-or-borrowed result
    // object is retained by `set_result`.
    if let Some(result) = tcl_cmd_core::array::dispatch_at(interp, &sub_str, &argv[2..], target) {
        return match result {
            Ok(result) => {
                if let Some(miss) = result.read_miss {
                    interp.set_return_options(
                        tcl_runtime_api::completion_options::retained_array_read_options(
                            &miss,
                            <[u8]>::to_vec,
                        ),
                    );
                }
                interp.set_result(result.value);
                Code::Ok
            }
            Err(e) => interp.report_cmd_error(e),
        };
    }
    // Per-runtime: `set` (per-element write traces), `default` (TIP 508), `for`
    // (Family-B iteration), and the unknown-subcommand message.
    match sub {
        b"set" => array_set(interp, argv),
        b"for" => array_for(interp, argv, for_variables),
        b"default" => array_default(interp, argv, default_option),
        // Unreachable: every `SUBCOMMANDS` name is handled above or by the
        // shared core.
        other => unknown_subcommand(interp, other),
    }
}

/// `array default set|get|exists|unset arrayName ?value?` (TIP 508) — the array's
/// default value for reads of missing elements.
fn array_default(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    prepared_option: Option<&'static [u8]>,
) -> Code {
    // argv: array default <subcmd> arrayName ?value?
    if !(4..=5).contains(&argv.len()) {
        return interp.wrong_args(b"array default option arrayName ?value?");
    }
    let sub = match prepared_option {
        Some(option) => option,
        None => match array_default_option(interp, argv) {
            Ok(option) => option,
            Err(code) => return code,
        },
    };
    let name = obj_bytes(argv[3]);
    match sub {
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
        // Unreachable: `array_default_option` returns exactly these four.
        _ => unreachable!("closed array default option table"),
    }
}

/// `array for {key value} arrayName script` — iterate the array's elements,
/// binding the two variables and running `script` each time (C's `ArrayForNRCmd`
/// / `ArrayForLoopCallback`). A structural change to the array (an element added
/// or removed) during iteration is an error, matching the invalidated hash
/// search; changing an existing element's value is fine.
fn array_for(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    prepared_variables: Option<[Vec<u8>; 2]>,
) -> Code {
    use tcl_runtime_api::completion_options::ControlOptionPolicy;

    if argv.len() != 5 {
        return interp.wrong_args(b"array for {key value} arrayName script");
    }
    let [kvar, vvar] = match prepared_variables {
        Some(variables) => variables,
        None => match array_for_variables(interp, argv) {
            Ok(variables) => variables,
            Err(code) => return code,
        },
    };
    let name = obj_bytes(argv[3]);
    if !interp.var_is_array(&name) {
        return not_array(interp, &name);
    }
    let body = argv[4];

    // Snapshot the element names; the iteration order is the snapshot order and a
    // change to the *set* of keys (not their values) aborts the loop.
    let snapshot = interp.array_names(&name).unwrap_or_default();
    let snapshot_set: std::collections::BTreeSet<Vec<u8>> = snapshot.iter().cloned().collect();
    let policy = ControlOptionPolicy::FRESH_SETTLED;
    interp.begin_control_options(policy);

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
        interp.begin_control_options(policy);
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
    interp.settle_control_options(policy, Code::Ok);
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
fn array_set(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 4 {
        return interp.wrong_args(b"array set arrayName list");
    }
    let name = obj_bytes(argv[2]);
    // An array-element name (`foo(bar)`) can't be the target of `array set`.
    if crate::frame::split_array_ref(&name).1.is_some() {
        let mut m = b"can't set \"".to_vec();
        m.extend_from_slice(&name);
        m.extend_from_slice(b"\": variable isn't array");
        return interp.set_error(&m);
    }
    // Read the *element objects* (not a re-split into fresh strings) so each
    // value keeps its `Tcl_Obj` identity through the array — C shares objs by
    // reference, and TIP 280 keys a literal's source location on that identity
    // (so a `-body {…}` stored via `array set` still evaluates as `type source`).
    let list = obj_bytes(argv[3]);
    let kvs = match crate::list::list_elements(argv[3]) {
        Ok(v) => v,
        Err(e) => return interp.report_list_error(&list, e),
    };
    if kvs.len() % 2 != 0 {
        return interp.report_cmd_error(tcl_cmd_core::CmdError::argument_format(
            "list must have an even number of elements",
        ));
    }
    // `array set a {}` still materialises an empty array (and a scalar `a`
    // errors `variable isn't array`), so ensure the array up front — the loop
    // below never runs for an empty value list.
    if let Err(e) = interp.ensure_array(&name) {
        return crate::builtins::var_error(interp, &name, e);
    }
    for pair in kvs.chunks_exact(2) {
        // `var_set_elem` retains the live value obj (no fresh allocation).
        let key = obj_bytes(pair[0]);
        if let Err(e) = interp.var_set_elem(&name, &key, pair[1]) {
            return crate::builtins::var_error(interp, &name, e);
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

    /// `array` is a `TclMakeEnsemble` command, so its scan and
    /// miss sentence belong to `tcl_cmd_core::ensemble`, rather than an exact
    /// match against a hand-joined list. Resolving first also means `array e
    /// a` fires the variable's `array` trace under the canonical name.
    /// `array default`'s own word is a `Tcl_GetIndexFromObj(…, "option", 0)`
    /// table in *C table* order, not alphabetical.
    ///
    /// tclsh 9.0.4 (the verdicts, not this runtime's shortened list):
    ///   array e a          -> 1        ;  array ex a -> 1
    ///   array s a          -> unknown or ambiguous subcommand "s": must be …
    ///   array default {} a -> ambiguous option "": must be get, set, exists, or unset
    ///   array default x a  -> bad option "x": must be get, set, exists, or unset
    ///   array default e a  -> 0        ;  array default ex a -> 0
    #[test]
    fn array_ensemble_and_default_option_resolve_like_tclsh() {
        const MUST: &str = "must be default, exists, for, get, names, set, size, or unset";
        const DEFAULT_MUST: &str = "must be get, set, exists, or unset";
        leak_free(|i| {
            let err_of = |i: &mut Interp, src: &[u8]| {
                assert_eq!(i.eval_str(src), Code::Error, "expected an error");
                String::from_utf8_lossy(&i.result_bytes()).into_owned()
            };
            run(i, b"array set a {x 1}");
            assert_eq!(run(i, b"array e a"), b"1");
            assert_eq!(run(i, b"array ex a"), b"1");
            assert_eq!(run(i, b"array n a"), b"x");
            assert_eq!(
                err_of(i, b"array s a"),
                format!("unknown or ambiguous subcommand \"s\": {MUST}")
            );
            assert_eq!(
                err_of(i, b"array {} a"),
                format!("unknown or ambiguous subcommand \"\": {MUST}")
            );
            // `array default`'s own table.
            assert_eq!(
                err_of(i, b"array default {} a"),
                format!("ambiguous option \"\": {DEFAULT_MUST}")
            );
            assert_eq!(
                err_of(i, b"array default x a"),
                format!("bad option \"x\": {DEFAULT_MUST}")
            );
            assert_eq!(run(i, b"array default e a"), b"0");
            assert_eq!(run(i, b"array default ex a"), b"0");
            i.eval_str(b"unset a");
        });
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

    #[test]
    fn whole_array_unset_preserves_constant_error_identity() {
        leak_free(|i| {
            run(i, b"array set a {x 1}");
            i.mark_constant(b"a");

            assert_eq!(i.eval_str(b"array unset a"), Code::Error);
            let message = i.result_bytes();
            assert_eq!(message, br#"can't unset "a": variable is a constant"#);
            // An outermost `eval_str` publishes the live exception state to
            // Tcl's compatibility globals before it returns.
            assert_eq!(run(i, b"set ::errorCode"), b"TCL UNSET CONST");
            assert!(i.array_names(b"a").is_some());
        });
    }
}
