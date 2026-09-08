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

//! The `array` ensemble builtin — a thin adapter over the shared
//! [`tcl_cmd_core::array`] core. The read-side (`exists`/`size`/`names`/`get`) and
//! `unset` are shared over the VM's `VarStore`/`Frames`/`ValueOps`; `set` (whose
//! per-element write traces must fail the command) stays here. Sharing fixed the
//! VM's `array unset a` (no pattern), which used to iterate-and-unset elements
//! (leaving an empty array) instead of removing the whole array.

use tcl_registry::{ArgRole, InvocationWord, InvocationWords};
use tcl_runtime_api::{ArrayTarget, Code, Completion, VarStore};

use crate::command::{completion_from_cmd_error, completion_from_tcl_error};
use crate::interp::{Vm, err, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    vm.register("array", cmd_array);
    // Ensemble member commands the codegen rewrites `array <sub>` into.
    vm.register("::tcl::array::exists", |vm, a| array_op(vm, "exists", a));
    vm.register("::tcl::array::names", |vm, a| array_op(vm, "names", a));
    vm.register("::tcl::array::get", |vm, a| array_op(vm, "get", a));
    vm.register("::tcl::array::set", |vm, a| array_op(vm, "set", a));
    vm.register("::tcl::array::size", |vm, a| array_op(vm, "size", a));
    vm.register("::tcl::array::unset", |vm, a| array_op(vm, "unset", a));
    vm.register("::tcl::array::for", |vm, a| array_op(vm, "for", a));
}

/// `array`'s subcommand set, alphabetical as `TclMakeEnsemble` sorts it.
/// C's table also carries `anymore`, `donesearch`, `nextelement`,
/// `startsearch`, `statistics`, and (9.0) `default`; like the rest of this
/// engine's ensembles it names only what it dispatches, so an advertised name
/// always works.
///
/// tclsh 9.0.4, for contrast:
///   array x a -> unknown or ambiguous subcommand "x": must be anymore,
///                default, donesearch, exists, for, get, names, nextelement,
///                set, size, startsearch, statistics, or unset
const ARRAY_SUBS: &[&str] = &["exists", "for", "get", "names", "set", "size", "unset"];

/// `array option arrayName ?arg ...?` — dispatch to the subcommand handler.
fn cmd_array(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return err("wrong # args: should be \"array subcommand ?arg ...?\"");
    };
    let word = sub.to_str();
    // `array` is a `TclMakeEnsemble` command: exact match, else a unique
    // prefix, so `array e a` is `array exists a`.
    // `for` is Tcl 9 (as is `default`, which this engine does not dispatch),
    // so under an earlier pin it must neither run nor claim the prefix `f`.
    let subs = crate::environment::release_subcommands(
        vm.command_surface_profile().name,
        "array",
        ARRAY_SUBS,
    );
    let Some(index) = tcl_cmd_core::ensemble::resolve_subcommand(subs, word.as_bytes(), true)
    else {
        return err(
            String::from_utf8_lossy(&tcl_cmd_core::ensemble::unknown_subcommand_message(
                subs,
                word.as_bytes(),
                true,
                b"::tcl::array",
            ))
            .into_owned(),
        );
    };
    array_op(vm, subs[index], rest)
}

fn array_op(vm: &mut Vm, sub: &str, rest: &[Value]) -> Completion<Value> {
    // `LocateArray` is the one semantic entry for every array subcommand,
    // including the compiler-lowered `::tcl::array::*` commands above. Derive
    // the target from the dialect-selected registry member: it is the unique
    // VarRead/VarWrite argument, not a second command-name/index table.
    let trace_target = array_trace_target(vm, sub, rest);
    if sub == "for" {
        return array_for(vm, rest, trace_target.as_deref());
    }
    if let Some(name) = trace_target {
        return vm.with_array_trace_target(&name, |vm, target| {
            array_op_after_trace(vm, sub, rest, Some(target))
        });
    }
    array_op_after_trace(vm, sub, rest, None)
}

fn array_op_after_trace(
    vm: &mut Vm,
    sub: &str,
    rest: &[Value],
    target: Option<&ArrayTarget>,
) -> Completion<Value> {
    // The read-side + `unset` live in the shared core.
    if let Some(result) = tcl_cmd_core::array::dispatch_at(vm, sub, rest, target) {
        return match result {
            Ok(v) => ok(v),
            Err(e) => completion_from_cmd_error(e),
        };
    }
    // Per-runtime: `array set` (its per-element write traces must fail the
    // command), `array for` (iterates a body), and the unknown-subcommand message.
    match sub {
        "set" => match rest {
            [n, list] => {
                // C's `Tcl_ArrayObjCmd` set path resolves the target through
                // the standard variable lookup *before* it looks at the list,
                // and that lookup parses the name: an element-form name yields
                // a scalar element cell, never an array, so the command refuses
                // it (issue #1578). The check therefore precedes both the list
                // parse and the even-length test.
                //
                // Oracle, identical on tclsh 8.4.20 / 8.5.19 / 8.6.14 / 9.0.4 /
                // 9.1 (`catch` result : message):
                //
                //   array set (x) {a 1}     -> 1:can't set "(x)": variable isn't array
                //   array set (x) {}        -> 1:can't set "(x)": variable isn't array
                //   array set (x) {a}       -> 1:can't set "(x)": variable isn't array
                //   array set (x) "a \{b"   -> 1:can't set "(x)": variable isn't array
                //   array set arr(k) {a 1}  -> 1:can't set "arr(k)": variable isn't array
                //   array set {arr(k)} {a 1}-> 1:can't set "arr(k)": variable isn't array
                //   array set okarr {a}     -> 1:list must have an even number of elements
                //   array set {a)b} {a 1}   -> 0:            (a `)` with no `(` is a name)
                //   array set {a(b} {a 1}   -> 0:            (an unclosed `(` is a name)
                //
                // 8.6+ also carry `errorCode` `TCL LOOKUP VARNAME <name>` (8.4 /
                // 8.5: `NONE`); the VM's shared `TCL LOOKUP VARNAME` spelling
                // omits the trailing name element here as it does at its
                // sibling site (`missing_parent_ns`).
                let name = n.to_str();
                if tcl_syntax::naming::split_element_ref(&name).is_some() {
                    return crate::command::err_with_code(
                        format!("can't set \"{name}\": variable isn't array"),
                        "TCL LOOKUP VARNAME",
                    );
                }
                let items = match list.as_list() {
                    Ok(i) => i,
                    Err(e) => return completion_from_tcl_error(e),
                };
                if items.len() % 2 != 0 {
                    return completion_from_cmd_error(tcl_cmd_core::CmdError::argument_format(
                        "list must have an even number of elements",
                    ));
                }
                if items.is_empty() {
                    // `array set a {}` still materialises an empty array; onto an
                    // existing scalar it errors. C words *this* case as the
                    // command (`can't array set "a"`), distinct from the
                    // per-element `set` message taken on a non-empty list.
                    if let Err(e) = vm.ensure_array(&name) {
                        return e;
                    }
                } else {
                    // Write element by element so a scalar target fails at the
                    // *element* write — naming `a(key)`, as C's `TclArraySet`
                    // does — rather than pre-checked under the bare name.
                    let mut i = 0;
                    while i + 1 < items.len() {
                        if let Err(e) =
                            vm.set_array_elem(&name, &items[i].to_str(), items[i + 1].clone())
                        {
                            return e;
                        }
                        i += 2;
                    }
                }
                ok(Value::empty())
            }
            _ => err("wrong # args: should be \"array set arrayName list\""),
        },
        // Unreachable from `cmd_array` (every `ARRAY_SUBS` name is handled
        // above or by the shared core); the registered `::tcl::array::*`
        // entry points pass canonical names.
        other => err(
            String::from_utf8_lossy(&tcl_cmd_core::ensemble::unknown_subcommand_message(
                ARRAY_SUBS,
                other.as_bytes(),
                true,
                b"::tcl::array",
            ))
            .into_owned(),
        ),
    }
}

fn array_trace_target(vm: &Vm, sub: &str, rest: &[Value]) -> Option<String> {
    let profile = vm.command_surface_profile();
    let registry = crate::environment::store_for_profile(profile);
    let words: Vec<InvocationWord<'_>> = std::iter::once(InvocationWord::Literal(sub))
        .chain(rest.iter().map(|_| InvocationWord::Dynamic))
        .collect();
    let resolved = registry
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal("array"), &words),
            Some(crate::environment::surface_point(profile)),
        )
        .resolved()?;
    if resolved.subcommand.resolved()?.canonical_name != sub {
        return None;
    }
    let facts = resolved.facts();
    let index = facts
        .sole_argument_index_for_roles(words.len(), &[ArgRole::VarRead, ArgRole::VarWrite])?
        .checked_sub(1)?;
    rest.get(index).map(|value| value.to_str().to_string())
}

/// `array for {keyVar valueVar} arrayName script` — iterate the array's elements,
/// binding the two vars and running the body once per pair (mirrors `dict for`;
/// `break`/`continue` apply, an error/return propagates). The element set is
/// snapshotted up front so body mutations don't perturb the walk.
fn array_for(vm: &mut Vm, rest: &[Value], trace_target: Option<&str>) -> Completion<Value> {
    let [vars, arrname, body] = rest else {
        return err("wrong # args: should be \"array for {key value} arrayName script\"");
    };
    let vnames = match vars.as_list() {
        Ok(v) => v,
        Err(e) => return completion_from_tcl_error(e),
    };
    let [kvar, vvar] = vnames.as_slice() else {
        return err("must have two variable names");
    };
    let kvar = kvar.to_str().to_string();
    let vvar = vvar.to_str().to_string();
    let name = arrname.to_str().to_string();
    let body = body.to_str().to_string();
    // C validates the loop-variable list before `LocateArray`; all later
    // validation happens after the array operation trace.
    if let Some(located_name) = trace_target {
        return vm.with_array_trace_target(located_name, |vm, target| {
            array_for_after_trace(vm, &kvar, &vvar, &name, &body, target)
        });
    }
    let target = VarStore::array_target(vm, tcl_runtime_api::Frames::current(vm), &name);
    array_for_after_trace(vm, &kvar, &vvar, &name, &body, &target)
}

fn array_for_after_trace(
    vm: &mut Vm,
    kvar: &str,
    vvar: &str,
    name: &str,
    body: &str,
    target: &ArrayTarget,
) -> Completion<Value> {
    let Some(keys) = VarStore::array_search_keys_at(vm, target) else {
        return crate::command::lookup_error(format!("\"{name}\" isn't an array"), "ARRAY", name);
    };
    let revision = VarStore::array_revision_at(vm, target);
    // Snapshot the physical hash keys, including undefined shells created by a
    // trace or link. C skips a candidate only when the iterator reaches it, so
    // defining an existing shell during an earlier body makes it a later row;
    // insertion/removal and unsetting a value invalidate the search revision.
    for k in &keys {
        if !same_array_target(vm, name, target, revision) {
            return array_for_changed();
        }
        if !VarStore::array_elem_exists_at(vm, target, k) {
            continue;
        }
        let value = vm.read_elem_swallowing_trace_error(name, k);
        // The read trace runs before Tcl assigns either loop variable. If it
        // deletes this element, the key is still assigned, the value variable
        // retains its previous value, and the body runs once; the structural
        // revision is diagnosed after that body unless it breaks the search.
        if let Err(e) = vm.set_var(kvar, Value::string(k.clone())) {
            return e;
        }
        if let Some(value) = value
            && let Err(e) = vm.set_var(vvar, value)
        {
            return e;
        }
        match vm.eval_source(body) {
            Ok(c) => match c.code {
                Code::Ok | Code::Continue => {}
                Code::Break => break,
                _ => return c,
            },
            Err(e) => return err(e.message),
        }
        // The body may have added/removed elements (a structural change), which
        // invalidates the enumeration — abort as C does.
        if !same_array_target(vm, name, target, revision) {
            return array_for_changed();
        }
    }
    ok(Value::empty())
}

fn same_array_target(vm: &Vm, name: &str, original: &ArrayTarget, revision: Option<u64>) -> bool {
    let current = VarStore::array_target(vm, tcl_runtime_api::Frames::current(vm), name);
    current.cell_id() == original.cell_id()
        && VarStore::array_keys_at(vm, original).is_some()
        && revision
            .is_none_or(|expected| VarStore::array_revision_at(vm, original) == Some(expected))
}

fn array_for_changed() -> Completion<Value> {
    crate::command::err_with_code("array changed during iteration", "TCL READ array for")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_array_unset_preserves_constant_error_identity() {
        let mut vm = Vm::new();
        vm.ensure_array("a").expect("array");
        vm.mark_constant("a");

        let completion = array_op_after_trace(&mut vm, "unset", &[Value::string("a")], None);

        assert_eq!(completion.code, Code::Error);
        assert_eq!(
            completion.result.to_str().as_ref(),
            r#"can't unset "a": variable is a constant"#
        );
        assert_eq!(
            crate::command::resolved_error_code(&completion)
                .to_str()
                .as_ref(),
            "TCL UNSET CONST"
        );
        assert!(VarStore::array_keys(&vm, tcl_runtime_api::Frames::current(&vm), "a").is_some());
    }
}
