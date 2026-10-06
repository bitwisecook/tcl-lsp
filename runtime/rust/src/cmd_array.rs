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
//! Implemented: `set`/`get`/`names`/`exists`/`size`/`unset` and native searches. (`statistics`,
//! `-exact`/`-regexp` name modes follow.)

use tcl_registry::{ArgRole, InvocationWord, InvocationWords};

use crate::interp::{new_string, obj_bytes, Code, Interp};
use crate::obj::TclObj;

/// Register `array`.
pub fn install(interp: &mut Interp) {
    const NAMES: &[&[u8]] = &[
        b"anymore".as_slice(),
        b"donesearch".as_slice(),
        b"nextelement".as_slice(),
        b"startsearch".as_slice(),
        b"default".as_slice(),
        b"exists".as_slice(),
        b"for".as_slice(),
        b"get".as_slice(),
        b"names".as_slice(),
        b"set".as_slice(),
        b"size".as_slice(),
        b"unset".as_slice(),
    ];
    let admitted = crate::environment::release_subcommands(
        interp.native_ensemble_profile_name(),
        "array",
        NAMES,
    );
    interp.register_stock_ensemble(
        tcl_registry::invocation_words::EnsembleImplementationFamily::Array,
        b"array",
        array_cmd,
        STOCK_MEMBERS,
        admitted,
    );
}

/// This build's `array` subcommands, in the order the unknown-subcommand
/// message lists them. Their argument shapes and variable roles live in the
/// registry rather than a second runtime table.
const SUBCOMMANDS: &[&[u8]] = &[
    b"anymore",
    b"default",
    b"donesearch",
    b"exists",
    b"for",
    b"get",
    b"names",
    b"nextelement",
    b"set",
    b"size",
    b"startsearch",
    b"unset",
];

/// The subcommand names alone, for the shared ensemble scan and its miss
/// sentence — `array` is a `TclMakeEnsemble` command, so both belong to
/// `tcl_cmd_core::ensemble` (its enumeration keeps a comma before `or`).
/// `default` and `for` are Tcl 9 additions, so the scan and the sentence both
/// take the emulated release's slice of the table — under an 8.6 pin `array f`
/// must not reach `for`, and `array d` must not be made ambiguous by
/// `default`.
const STOCK_MEMBERS: &[(&[u8], crate::interp::BuiltinFn)] = &[
    (b"anymore", stock_anymore),
    (b"donesearch", stock_donesearch),
    (b"nextelement", stock_nextelement),
    (b"startsearch", stock_startsearch),
    (b"default", stock_default),
    (b"exists", stock_exists),
    (b"for", stock_for),
    (b"get", stock_get),
    (b"names", stock_names),
    (b"set", stock_set),
    (b"size", stock_size),
    (b"unset", stock_unset),
];

fn stock_anymore(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"array", b"anymore"], array_cmd)
}

fn stock_donesearch(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"array", b"donesearch"], array_cmd)
}

fn stock_nextelement(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"array", b"nextelement"], array_cmd)
}

fn stock_startsearch(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"array", b"startsearch"], array_cmd)
}

fn stock_default(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"array", b"default"], array_cmd)
}

fn stock_exists(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"array", b"exists"], array_cmd)
}

fn stock_for(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"array", b"for"], array_cmd)
}

fn stock_get(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"array", b"get"], array_cmd)
}

fn stock_names(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"array", b"names"], array_cmd)
}

fn stock_set(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"array", b"set"], array_cmd)
}

fn stock_size(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"array", b"size"], array_cmd)
}

fn stock_unset(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"array", b"unset"], array_cmd)
}

fn subcommand_names(interp: &Interp) -> &'static [&'static [u8]] {
    crate::environment::release_subcommands(
        interp.native_ensemble_profile_name(),
        "array",
        SUBCOMMANDS,
    )
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
    let protocol = interp
        .native_invocation_dialect()
        .native_array_default_protocol()
        .ok_or_else(|| {
            interp.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable("array default command")
                    .into(),
            )
        })?;
    tcl_cmd_core::array::prepare_default_option_original(interp, protocol, &argv[2])
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
        return interp.wrong_args_for_invocation(argv, b"subcommand ?arg ...?");
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
    if let Some(result) =
        tcl_cmd_core::native_array_search::dispatch(interp, &sub_str, &argv[2..], target)
    {
        return match result {
            Ok(value) => {
                interp.set_result(value);
                Code::Ok
            }
            Err(error) => interp.report_cmd_error(error),
        };
    }
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
        b"default" => array_default(interp, argv, default_option, target),
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
    target: Option<&tcl_runtime_api::ArrayTarget>,
) -> Code {
    if !(4..=5).contains(&argv.len()) {
        return interp.wrong_args_for_prefix(argv, 2, b"option arrayName ?value?");
    }
    let protocol = match interp
        .native_invocation_dialect()
        .native_array_default_protocol()
    {
        Some(protocol) => protocol,
        None => {
            return interp.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable("array default command")
                    .into(),
            );
        }
    };
    let option = match prepared_option {
        Some(option) => option,
        None => match array_default_option(interp, argv) {
            Ok(option) => option,
            Err(code) => return code,
        },
    };
    match tcl_cmd_core::array::default_at(interp, protocol, option, &argv[3..], target) {
        Ok(tcl_cmd_core::array::ArrayDefaultCommandResult::Value(value)) => {
            interp.set_result(value);
            Code::Ok
        }
        Ok(tcl_cmd_core::array::ArrayDefaultCommandResult::WrongArguments { suffix }) => {
            interp.wrong_args_for_prefix(argv, 3, suffix)
        }
        Err(error) => interp.report_cmd_error(error),
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
        return interp.wrong_args_for_prefix(argv, 2, b"{key value} arrayName script");
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
        return interp.wrong_args_for_prefix(argv, 2, b"arrayName list");
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
    let kvs = match crate::list::list_elements(argv[3]) {
        Ok(v) => v,
        Err(e) => return interp.report_cmd_error(e.into()),
    };
    if kvs.len() % 2 != 0 {
        return interp.report_cmd_error(tcl_cmd_core::CmdError::argument_format(
            "list must have an even number of elements",
        ));
    }
    // `array set a {}` still materialises an empty array (and a scalar `a`
    // errors `variable isn't array`), so initialise that empty case explicitly.
    // Nonempty input selects and stores each actual element in order; a failing
    // first store reports the element spelling rather than the root spelling.
    let name_text = match tcl_syntax::raw_string::RawString::from_bytes(name.clone()).unicode() {
        Ok(name) => name,
        Err(error) => return interp.refuse_unicode_access(error),
    };
    let target = tcl_runtime_api::VarStore::array_target(
        interp,
        tcl_runtime_api::Frames::current(interp),
        &name_text,
    );
    if let Err(error) = tcl_runtime_api::VarStore::array_key_bytes_checked_at(interp, &target) {
        return interp.report_cmd_error(error.into());
    }
    let initialised = if kvs.is_empty() {
        interp.ensure_array(&name)
    } else {
        Ok(())
    };
    if let Err(e) = initialised {
        return crate::builtins::var_error(interp, &name, e);
    }
    for pair in kvs.chunks_exact(2) {
        // `var_set_elem` retains the live value obj (no fresh allocation).
        let key = obj_bytes(pair[0]);
        if let Err(e) = interp.var_set_elem(&name, &key, pair[1]) {
            return crate::builtins::var_element_error(interp, &name, &key, e);
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

#[cfg(test)]
mod tests {
    use crate::counters;
    use crate::interp::{new_string, Code, Interp};

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
    fn search_absence_and_original_root_retirement_are_guest_observations() {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            leak_free(|interp| {
                interp.set_dialect_profile(
                    tcl_registry::model::resolve_environment(dialect).unit_profile(),
                );
                assert_eq!(
                    run(
                        interp,
                        b"catch {array startsearch missing} result; set result"
                    ),
                    b"\"missing\" isn't an array"
                );
                assert_eq!(run(interp, b"array set a {k V}; set s [array startsearch a]; unset a; array set a {k V}; catch {array anymore a $s} result; set result"), b"couldn't find search \"s-1-a\"");
            });
        }
        leak_free(|interp| {
            interp.set_dialect_profile(
                tcl_registry::model::resolve_environment("jim").unit_profile(),
            );
            assert!(
                !crate::environment::release_subcommands("jim", "array", super::SUBCOMMANDS)
                    .contains(&b"startsearch".as_slice())
            );
        });
    }

    #[test]
    fn original_search_handle_cache_matches_all_native_columns() {
        use crate::obj::Owned;
        let inputs: &[&[u8]] = &[
            b"s-1-a",
            b"s-01-a",
            b"s-+1-a",
            b"s- 1-a",
            b"s--1-a",
            b"s-4294967297-a",
            b"s-18446744073709551617-a",
            b"s-1-other",
            b"s-1-a\0tail",
            b"BAD",
            b"s-1",
            b"s--a",
            b"s-1-a\xff",
        ];
        let captures = [
            (
                "tcl8.4",
                include_str!("../../../rust/tcl-syntax/tests/data/native_array_search/8.4.20.tsv"),
            ),
            (
                "tcl8.5",
                include_str!("../../../rust/tcl-syntax/tests/data/native_array_search/8.5.19.tsv"),
            ),
            (
                "tcl8.6",
                include_str!("../../../rust/tcl-syntax/tests/data/native_array_search/8.6.18.tsv"),
            ),
            (
                "tcl9.0",
                include_str!("../../../rust/tcl-syntax/tests/data/native_array_search/9.0.4.tsv"),
            ),
            (
                "tcl9.1",
                include_str!("../../../rust/tcl-syntax/tests/data/native_array_search/9.1.0.tsv"),
            ),
        ];
        let mut rows = 0;
        for (dialect, capture) in captures {
            for line in capture.lines() {
                let fields: Vec<_> = line.split('\t').collect();
                leak_free(|interp| {
                    interp.set_dialect_profile(
                        tcl_registry::model::resolve_environment(dialect).unit_profile(),
                    );
                    run(
                        interp,
                        b"array set a {k00 V k01 V k02 V}; array startsearch a",
                    );
                    let handle =
                        Owned::fresh(new_string(inputs[fields[0].parse::<usize>().unwrap()]));
                    let head = Owned::fresh(new_string(b"array"));
                    let member = Owned::fresh(new_string(b"anymore"));
                    let name = Owned::fresh(new_string(b"a"));
                    let argv = [
                        head.as_ptr(),
                        member.as_ptr(),
                        name.as_ptr(),
                        handle.as_ptr(),
                    ];
                    let code = super::array_cmd(interp, &argv);
                    assert_eq!(code == Code::Error, fields[1] == "1", "{dialect}: {line}");
                    let result: String = interp
                        .result_bytes()
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect();
                    assert_eq!(result, fields[5], "{dialect}: {line}");
                    let protocol = <Interp as tcl_cmd_core::native_array_search::NativeArraySearchBackend>::array_search_protocol(interp).unwrap();
                    let cache = crate::obj::native_array_search_cache_in(handle.as_ptr(), protocol)
                        .unwrap();
                    assert_eq!(
                        cache.is_some(),
                        fields[2] == "array search",
                        "{dialect}: {line}"
                    );
                    if let Some(cache) = cache {
                        assert_eq!(cache.id, fields[3].parse::<i32>().unwrap());
                        assert_eq!(cache.name_offset, fields[4].parse::<usize>().unwrap());
                    }
                });
                rows += 1;
            }
        }
        assert_eq!(rows, 65);
    }

    #[test]
    fn native_search_lifecycle_keeps_shell_gc_and_invalidates_real_mutations() {
        let captures = [
            (
                "tcl8.4",
                include_str!(
                    "../../../rust/tcl-syntax/tests/data/native_array_search/search-lifecycle-8.4.txt"
                ),
            ),
            (
                "tcl8.5",
                include_str!(
                    "../../../rust/tcl-syntax/tests/data/native_array_search/search-lifecycle-8.5.txt"
                ),
            ),
            (
                "tcl8.6",
                include_str!(
                    "../../../rust/tcl-syntax/tests/data/native_array_search/search-lifecycle-8.6.txt"
                ),
            ),
            (
                "tcl9.0",
                include_str!(
                    "../../../rust/tcl-syntax/tests/data/native_array_search/search-lifecycle-9.0.txt"
                ),
            ),
            (
                "tcl9.1",
                include_str!(
                    "../../../rust/tcl-syntax/tests/data/native_array_search/search-lifecycle-9.1.txt"
                ),
            ),
        ];
        let mut source = include_str!(
            "../../../rust/tcl-syntax/tests/data/native_array_search/search-lifecycle.tcl"
        )
        .replace(
            "puts [list $label $code $hex]",
            "lappend ::receipts [list $label $code $hex]",
        );
        source.push_str("\nset receipts\n");
        let mut rows = 0;
        for (dialect, capture) in captures {
            let expected = capture
                .lines()
                .map(|line| format!("{{{line}}}"))
                .collect::<Vec<_>>()
                .join(" ");
            leak_free(|interp| {
                interp.set_dialect_profile(
                    tcl_registry::model::resolve_environment(dialect).unit_profile(),
                );
                assert_eq!(
                    run(interp, source.as_bytes()),
                    expected.as_bytes(),
                    "{dialect}"
                );
            });
            rows += capture.lines().count();
        }
        assert_eq!(rows, 65);
    }

    #[test]
    fn array_default_matches_all_native_state_and_callback_controls() {
        let decode = |text: &str| -> Vec<u8> {
            text.as_bytes()
                .chunks_exact(2)
                .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect()
        };
        let mut count = 0;
        for row in include_str!("../tests/data/native_array_default/controls.tsv").lines() {
            let fields: Vec<_> = row.split('\t').collect();
            let dialect = match fields[0] {
                "9.0.4" => "tcl9.0",
                "9.1.0" => "tcl9.1",
                _ => unreachable!(),
            };
            let source = decode(fields[2]);
            let expected = decode(fields[3]);
            leak_free(|interp| {
                interp.set_dialect_profile(
                    tcl_registry::model::resolve_environment(dialect).unit_profile(),
                );
                assert_eq!(interp.eval_str(&source), Code::Ok, "{row}");
                assert_eq!(interp.result_bytes(), expected, "{row}");
            });
            count += 1;
        }
        assert_eq!(count, 88);
    }

    #[test]
    fn array_default_usage_matches_native_literal_dynamic_and_callback_entries() {
        let decode = |text: &str| -> Vec<u8> {
            text.as_bytes()
                .chunks_exact(2)
                .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect()
        };
        let mut count = 0;
        for row in include_str!("../tests/data/native_array_default/usage.tsv").lines() {
            let fields: Vec<_> = row.split('\t').collect();
            let dialect = match fields[0] {
                "9.0.4" => "tcl9.0",
                "9.1.0" => "tcl9.1",
                _ => unreachable!(),
            };
            leak_free(|interp| {
                interp.set_dialect_profile(
                    tcl_registry::model::resolve_environment(dialect).unit_profile(),
                );
                assert_eq!(interp.eval_str(&decode(fields[2])), Code::Ok, "{row}");
                assert_eq!(interp.result_bytes(), decode(fields[3]), "{row}");
            });
            count += 1;
        }
        assert_eq!(count, 16);
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
    fn variable_container_storage_matches_all_native_columns() {
        use tcl_test_support::variable_containers::{
            variable_container_expectations, variable_container_scripts,
        };
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let environment = tcl_registry::model::resolve_environment(dialect);
            let profile = environment.unit_profile();
            let expected = variable_container_expectations(
                profile.variable_container_model().unwrap(),
                profile.runtime_version(),
            )
            .unwrap();
            for (index, (source, wanted)) in variable_container_scripts()
                .iter()
                .zip(expected.lines())
                .enumerate()
            {
                leak_free(|interp| {
                    interp.set_dialect_profile(profile);
                    assert_eq!(
                        interp.eval_str(source.as_bytes()),
                        Code::Ok,
                        "{dialect} case {index}: {source}"
                    );
                    assert_eq!(
                        interp.result_bytes(),
                        wanted.as_bytes(),
                        "{dialect} case {index}: {source}"
                    );
                });
            }
        }
    }

    #[test]
    fn array_ensemble_and_default_option_resolve_like_tclsh() {
        const MUST: &str = "must be anymore, default, donesearch, exists, for, get, names, nextelement, set, size, startsearch, or unset";
        const DEFAULT_MUST: &str = "must be get, set, exists, or unset";
        leak_free(|i| {
            let err_of = |i: &mut Interp, src: &[u8]| {
                assert_eq!(i.eval_str(src), Code::Error, "expected an error");
                String::from_utf8_lossy(&i.result_bytes()).into_owned()
            };
            run(i, b"array set a {x 1}");
            assert_eq!(run(i, b"array e a"), b"1");
            assert_eq!(run(i, b"array ex a"), b"1");
            assert_eq!(run(i, b"array na a"), b"x");
            assert_eq!(
                err_of(i, b"array n a"),
                format!("unknown or ambiguous subcommand \"n\": {MUST}")
            );
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
    fn jim_array_byte_keys_count_and_unset_without_unicode_projection() {
        leak_free(|interp| {
            interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
            let dictionary = crate::list::new_list_obj(&[
                crate::obj::new_string_bytes(b"\xff"),
                crate::obj::new_string_bytes(b"ONE"),
                crate::obj::new_string_bytes(b"b"),
                crate::obj::new_string_bytes(b"TWO"),
            ]);
            interp.var_set(b"a", dictionary).unwrap();
            interp
                .var_set(b"pattern", crate::obj::new_string_bytes(b"\xff"))
                .unwrap();
            assert_eq!(run(interp, b"array exists a"), b"1");
            assert_eq!(run(interp, b"array size a"), b"2");
            assert_eq!(run(interp, b"array names a $pattern"), b"\xff");
            assert_eq!(run(interp, b"array get a $pattern"), b"\xff ONE");
            assert_eq!(run(interp, b"array unset a $pattern"), b"");
            assert_eq!(run(interp, b"array size a"), b"1");
            assert_eq!(run(interp, b"array names a"), b"b");
            assert!(!interp.host_refusal_pending());

            run(interp, b"set a odd");
            assert_eq!(run(interp, b"array exists a"), b"0");
            assert_eq!(run(interp, b"array size a"), b"0");
            assert_eq!(interp.eval_str(b"array names a"), Code::Error);
            assert!(!interp.host_refusal_pending());
            assert_eq!(run(interp, b"array unset a x*"), b"");
            assert_eq!(run(interp, b"info exists a"), b"1");
            assert_eq!(run(interp, b"array unset a *"), b"");
            assert_eq!(run(interp, b"info exists a"), b"0");
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
