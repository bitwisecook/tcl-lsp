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

//! `interp` and `alias`: child interpreters, limits, hidden commands and aliases.

use super::error_completion::{
    completion_from_cmd_error, completion_from_tcl_error, err_with_code,
};
use super::{Command, native_wrong_arguments_message};
use crate::interp::{Vm, err, ok};
use crate::value::Value;
use tcl_runtime_api::Completion;

/// `interp create`'s option words (`createOptions[]`, `tclInterp.c`), resolved
/// with `Tcl_GetIndexFromObj(…, "option", 0)`: `-s` abbreviates `-safe` and the
/// lone `-` — a prefix of both entries — is `ambiguous option "-"`. Only a word
/// starting with `-` reaches the table, so the empty word is a path, not a miss.
const CREATE_OPTIONS: tcl_cmd_core::prefix::OptionTable<'static> =
    tcl_cmd_core::prefix::OptionTable::abbreviating("option", &["-safe", "--"]);

/// `interp create ?-safe? ?--? ?name?` — make a child interpreter.
fn interp_create_cmd(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let mut safe = false;
    let mut last = false;
    let mut name = None;
    let mut i = 0;
    while i < rest.len() {
        let original = match vm.native_name_operand_bytes_checked(&rest[i]) {
            Ok(bytes) => bytes,
            Err(error) => return completion_from_cmd_error(vm, error),
        };
        if !last && original.first() == Some(&b'-') {
            match CREATE_OPTIONS.index_of_original(vm, &rest[i]) {
                Ok(0) => {
                    safe = true;
                    i += 1;
                    continue;
                }
                Ok(_) => {
                    i += 1;
                    last = true;
                }
                Err(error) => return completion_from_cmd_error(vm, error),
            }
        }
        if name.is_some() {
            return native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"interp create ?-safe? ?--? ?path?\"",
            );
        }
        name = rest.get(i);
        i += 1;
    }
    match name {
        Some(path) => vm.create_child_original(path, safe),
        None => ok(Value::string(vm.create_child(None, safe))),
    }
}

/// `interp recursionlimit path ?newlimit?` — get/set a (possibly child) interp's
/// recursion bound.
fn interp_recursionlimit_cmd(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let (path, newlimit) = match rest {
        [path] => (path, None),
        [path, limit] => (path, Some(limit)),
        _ => {
            return native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"interp recursionlimit path ?newlimit?\"",
            );
        }
    };
    let id = match vm.resolve_interp_path_original(path) {
        Ok(id) => id,
        Err(error) => return error,
    };
    // C answers a new limit with the word it was given.
    match vm.in_interp(id, |target| target.recursion_limit_apply(newlimit)) {
        Ok(limit) => ok(newlimit.cloned().unwrap_or_else(|| Value::int(limit))),
        Err(error) => completion_from_tcl_error(vm, error),
    }
}

/// `interp limit path limitType ?-option value ...?` — query/configure the
/// `commands` or `time` limit on a child interp (stored, not enforced).
fn interp_limit_cmd(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let (path, ltype, opts) = match rest {
        [path, ltype, opts @ ..] => (path, ltype.to_str(), opts),
        _ => {
            return native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"interp limit path limitType ?-option value ...?\"",
            );
        }
    };
    // Validate the limit type before the current-interp guard so that a bad
    // type is reported ahead of the inaccessibility error (interp-35.3 vs .23).
    if let Err(e) = crate::interp::LIMIT_TYPES.index_of_str(&ltype) {
        return crate::command::completion_from_cmd_error(vm, e);
    }
    let id = match vm.resolve_interp_path_original(path) {
        Ok(id) => id,
        Err(c) => return c,
    };
    if id == vm.cur_interp() {
        return err("limits on current interpreter inaccessible");
    }
    let ltype = ltype.to_string();
    let opts = opts.to_vec();
    match vm.in_interp(id, |vm| vm.limit_apply(&ltype, &opts)) {
        Ok(v) => ok(v),
        Err(error) => completion_from_cmd_error(vm, error),
    }
}

/// `interp eval path arg ?arg ...?` — empty path evaluates like `eval`; a
/// named path (possibly multi-word, addressing a grandchild) routes into
/// that interpreter.
fn interp_eval_cmd(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let [path, scripts @ ..] = rest else {
        return native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"interp eval path arg ?arg ...?\"",
        );
    };
    if scripts.is_empty() {
        return native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"interp eval path arg ?arg ...?\"",
        );
    }
    match vm.resolve_interp_path_original(path) {
        Ok(id) => vm.eval_values_in_interp(id, scripts),
        Err(c) => c,
    }
}

/// `interp delete ?path ...?` — destroy each named interpreter.
fn interp_delete_cmd(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    for path in rest {
        let id = match vm.resolve_interp_path_original(path) {
            Ok(id) => id,
            Err(error) => return error,
        };
        if !vm.delete_interp(id) {
            return vm.interpreter_path_missing(path);
        }
    }
    ok(Value::empty())
}

/// `interp bgerror path ?cmdPrefix?` — get/set the background-error handler.
/// `interp children ?path?` — children of the current interp, or (one level
/// down) of the named child.
fn interp_children_cmd(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let id = match rest {
        [] => vm.cur_interp(),
        [path] => match vm.resolve_interp_path_original(path) {
            Ok(id) => id,
            Err(error) => return error,
        },
        _ => {
            return native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"interp children ?path?\"",
            );
        }
    };
    let names = vm.in_interp(id, |target| target.child_names());
    ok(Value::list(
        names
            .into_iter()
            .map(|name| Value::from_native_string_bytes(name.as_bytes().to_vec()))
            .collect(),
    ))
}

fn interp_bgerror_cmd(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let (path, args) = match rest {
        [path] | [path, _] => (path, &rest[1..]),
        _ => {
            return native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"interp bgerror path ?cmdPrefix?\"",
            );
        }
    };
    let id = match vm.resolve_interp_path_original(path) {
        Ok(id) => id,
        Err(error) => return error,
    };
    match vm.in_interp(id, |target| target.bgerror_apply(args)) {
        Ok(prefix) => ok(prefix),
        Err(error) => error,
    }
}

/// `interp debug path ?-frame ?bool??` — the per-interp frame-debug switch.
fn interp_debug_cmd(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    const USAGE: &str = "interp debug path ?-frame ?bool??";
    let Some((path, args)) = rest.split_first() else {
        return native_wrong_arguments_message(vm, format!("wrong # args: should be \"{USAGE}\""));
    };
    let id = match vm.resolve_interp_path_original(path) {
        Ok(id) => id,
        Err(error) => return error,
    };
    match vm.in_interp(id, |target| target.debug_apply(args, USAGE)) {
        Ok(value) => ok(value),
        Err(error) => completion_from_cmd_error(vm, error),
    }
}

/// `interp hidden ?path?` — the hidden-command names of the current or named
/// interp.
fn interp_hidden_cmd(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let id = match rest {
        [] => vm.cur_interp(),
        [path] => match vm.resolve_interp_path_original(path) {
            Ok(id) => id,
            Err(error) => return error,
        },
        _ => {
            return native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"interp hidden ?path?\"",
            );
        }
    };
    let names = vm.in_interp(id, |target| target.own_hidden_names());
    ok(Value::list(
        names
            .into_iter()
            .map(|name| Value::from_native_string_bytes(name.as_bytes().to_vec()))
            .collect(),
    ))
}

/// `interp hide|expose path cmd` — move a command between the (current or
/// child) interp's visible and hidden tables.
fn interp_hidectl_cmd(vm: &mut Vm, hide: bool, rest: &[Value]) -> Completion<Value> {
    let (path, command, token) = match rest {
        [path, command] => (path, command, command),
        [path, command, token] => (path, command, token),
        _ => {
            return native_wrong_arguments_message(
                vm,
                if hide {
                    "wrong # args: should be \"interp hide path cmdName ?hiddenCmdName?\""
                } else {
                    "wrong # args: should be \"interp expose path hiddenCmdName ?cmdName?\""
                },
            );
        }
    };
    if vm.is_safe() {
        let verb = if hide { "hide" } else { "expose" };
        return err(format!(
            "permission denied: safe interpreter cannot {verb} commands"
        ));
    }
    let id = match vm.resolve_interp_path_original(path) {
        Ok(id) => id,
        Err(error) => return error,
    };
    let command = match vm.native_name_operand_bytes_checked(command) {
        Ok(bytes) => bytes,
        Err(error) => return completion_from_cmd_error(vm, error),
    };
    let token = match vm.native_name_operand_bytes_checked(token) {
        Ok(bytes) => bytes,
        Err(error) => return completion_from_cmd_error(vm, error),
    };
    match vm.in_interp(id, |target| {
        if hide {
            target.hide_command_bytes(&command, &token)
        } else {
            target.expose_own_command_bytes(&command, &token)
        }
    }) {
        Ok(()) => ok(Value::empty()),
        Err(problem) => problem.into_completion(),
    }
}

pub(crate) struct HiddenInvocationOptions {
    pub(crate) command_index: usize,
    pub(crate) namespace: Option<Vec<u8>>,
}

/// Retain original option reads and the last selected global namespace operand.
pub(crate) fn hidden_invocation_options(
    vm: &mut Vm,
    tail: &[Value],
) -> Result<HiddenInvocationOptions, Completion<Value>> {
    use tcl_syntax::naming::NativeHiddenInvocationOption;
    let grammar = vm
        .native_invocation_dialect()
        .native_name_protocol()
        .and_then(|protocol| protocol.hidden_invocation_grammar().ok())
        .ok_or_else(|| {
            vm.refuse_host_command("native hidden-invocation grammar unavailable".into())
        })?;
    let options = tcl_cmd_core::prefix::OptionTable::abbreviating("option", grammar.names());
    let mut i = 0;
    let mut namespace = None;
    while i < tail.len() {
        let word = match vm.native_name_operand_bytes_checked(&tail[i]) {
            Ok(bytes) => bytes,
            Err(error) => return Err(completion_from_cmd_error(vm, error)),
        };
        if word.first() != Some(&b'-') {
            break;
        }
        let index = options
            .index_of_original(vm, &tail[i])
            .map_err(|error| completion_from_cmd_error(vm, error))?;
        let option = grammar.option(index).ok_or_else(|| {
            vm.refuse_host_command("native hidden-invocation option ordinal unavailable".into())
        })?;
        match option {
            NativeHiddenInvocationOption::Namespace => {
                // `-namespace ns` — the namespace word, when there is one.
                i += 1;
                if i == tail.len() {
                    break;
                }
                namespace = Some(
                    vm.native_name_operand_bytes_checked(&tail[i])
                        .map_err(|error| completion_from_cmd_error(vm, error))?
                        .to_vec(),
                );
                i += 1;
            }
            NativeHiddenInvocationOption::End => {
                i += 1;
                break;
            }
            NativeHiddenInvocationOption::Global => {
                namespace = Some(b"::".to_vec());
                i += 1;
            }
        }
    }
    Ok(HiddenInvocationOptions {
        command_index: i,
        namespace,
    })
}

#[cfg(test)]
pub(super) fn skip_hidden_options(vm: &mut Vm, tail: &[Value]) -> Result<usize, Completion<Value>> {
    hidden_invocation_options(vm, tail).map(|selected| selected.command_index)
}

/// `interp invokehidden path ?-namespace ns? ?-global? ?--? cmd ?arg ...?` —
/// invoke a hidden command in the named child.
fn interp_invokehidden_cmd(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let usage = "wrong # args: should be \"interp invokehidden path \
                 ?-namespace ns? ?-global? ?--? cmd ?arg ..?\"";
    let [path, tail @ ..] = rest else {
        return native_wrong_arguments_message(vm, usage);
    };
    if tail.is_empty() {
        return native_wrong_arguments_message(vm, usage);
    }
    if vm.is_safe() {
        return err("not allowed to invoke hidden commands from safe interpreter");
    }
    let selected = match hidden_invocation_options(vm, tail) {
        Ok(selected) => selected,
        Err(completion) => return completion,
    };
    let i = selected.command_index;
    let Some(cmd) = tail.get(i) else {
        return native_wrong_arguments_message(vm, usage);
    };
    let id = match vm.resolve_interp_path_original(path) {
        Ok(id) => id,
        Err(error) => return error,
    };
    vm.invoke_hidden_value_by_id_in_namespace(
        id,
        cmd,
        &tail[i + 1..],
        selected.namespace.as_deref(),
    )
    .unwrap_or_else(|| vm.interpreter_path_missing(path))
}

/// Resolve an `interp`-family subcommand word through the shared owner:
/// `dispatch` is the table the word may resolve against, `advertised` the
/// (possibly shorter) table the miss message enumerates — C splits the two the
/// same way for `slaves`.
pub(crate) fn resolve_interp_option_bytes(
    dispatch: &'static [&'static str],
    advertised: &[&'static str],
    word: &[u8],
) -> Result<&'static str, tcl_cmd_core::CmdError> {
    match tcl_cmd_core::prefix::scan(dispatch, word, false) {
        tcl_cmd_core::prefix::Resolution::Exact(i)
        | tcl_cmd_core::prefix::Resolution::UniquePrefix(i) => Ok(dispatch[i]),
        miss => Err(tcl_cmd_core::CmdError::new_bytes(
            tcl_cmd_core::prefix::bad_key_message(
                advertised,
                b"option",
                word,
                matches!(miss, tcl_cmd_core::prefix::Resolution::Ambiguous),
            ),
        )),
    }
}

/// `interp` — child-interpreter creation, evaluation, and the single-interp
/// alias form. A child is a full `Vm` sharing the parent's output and compile
/// service (see [`Vm::create_child`]); `interp eval`/`delete`/`issafe`/… address
/// the current interp (empty path) or a named child.
/// Jim aliases retain the actual argv prefix and resolve it at each call.
pub(super) fn cmd_alias(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [name, target, prefix @ ..] = args else {
        return err_with_code(
            r#"wrong # args: should be "alias newname command ?args ...?""#,
            "NONE",
        );
    };
    let mut words = vec![target.clone()];
    words.extend_from_slice(prefix);
    let original = match vm.native_name_operand_bytes_checked(name) {
        Ok(bytes) => bytes,
        Err(error) => return completion_from_cmd_error(vm, error),
    };
    let slot = match vm.native_alias_publication_slot(&original) {
        Ok(slot) => slot,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    let report = match vm.original_jim_publication_key(
        name,
        tcl_syntax::naming::NativeNamePurpose::AliasPublication,
    ) {
        Ok(report) => report,
        Err(error) => return completion_from_cmd_error(vm, error),
    };
    let original = Value::native_list_constructor(
        words,
        tcl_syntax::native_string::NativeStringProtocol::Jim084,
    );
    vm.register_command_in_slot_with_jim_key(slot, Command::CallerAlias(original), report);
    ok(name.clone())
}

pub(super) fn cmd_interp(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if vm
        .actual_native_invocation_dialect()
        .native_jim_lookup_protocol()
        .is_some()
    {
        return if args.is_empty() {
            ok(Value::string(vm.create_jim_child()))
        } else {
            native_wrong_arguments_message(vm, "wrong # args: should be \"interp\"")
        };
    }
    let Some((sub, rest)) = args.split_first() else {
        return native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"interp cmd ?arg ...?\"",
        );
    };
    let sub = match vm.native_interpreter_option_from_original_checked(sub, false) {
        Ok(name) => name,
        Err(error) => return completion_from_cmd_error(vm, error),
    };
    interp_command(vm, sub, rest)
}

fn interp_alias_command(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    match rest {
        [path, name] => vm.interpreter_alias_query_original(path, name),
        [path, name, empty] => vm.interpreter_alias_delete_original(path, name, empty),
        [src_path, src_cmd, target_path, target @ ..] if !target.is_empty() => {
            vm.interp_alias_create(src_path, src_cmd, target_path, target.to_vec())
        }
        _ => native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"interp alias srcPath srcCmd ?targetPath targetCmd ?arg ...??\"",
        ),
    }
}

fn interp_exists(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    match rest {
        [] => ok(Value::int(1)),
        [path] => match vm.resolve_interp_path_original(path) {
            Ok(_) => ok(Value::bool(true)),
            Err(error) if vm.execution_refusal.is_some() => error,
            Err(_) => ok(Value::bool(false)),
        },
        _ => native_wrong_arguments_message(vm, "wrong # args: should be \"interp exists ?path?\""),
    }
}

fn interp_command(vm: &mut Vm, sub: &str, rest: &[Value]) -> Completion<Value> {
    match sub {
        "create" => interp_create_cmd(vm, rest),
        "alias" => interp_alias_command(vm, rest),
        "target" => match rest {
            [path, name] => vm.interpreter_alias_target_original(path, name),
            _ => native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"interp target path alias\"",
            ),
        },
        "aliases" => match rest {
            [] => vm.interpreter_alias_names_original(None),
            [path] => vm.interpreter_alias_names_original(Some(path)),
            _ => native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"interp aliases ?path?\"",
            ),
        },
        "exists" => interp_exists(vm, rest),
        // interp eval path arg ?arg ...? — empty path is the current interp
        // (evaluate like `eval`); a named path routes into that child.
        "eval" => interp_eval_cmd(vm, rest),
        // interp delete ?path ...?
        "delete" => interp_delete_cmd(vm, rest),
        "issafe" => match rest {
            [] => ok(Value::bool(vm.is_safe())),
            [path] => match vm.resolve_interp_path_original(path) {
                Ok(id) => ok(Value::bool(vm.in_interp(id, |target| target.is_safe()))),
                Err(error) => error,
            },
            _ => native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"interp issafe ?path?\"",
            ),
        },
        "slaves" | "children" => interp_children_cmd(vm, rest),
        "recursionlimit" => interp_recursionlimit_cmd(vm, rest),
        // interp hide path cmd / interp expose path cmd
        "hide" | "expose" => interp_hidectl_cmd(vm, sub == "hide", rest),
        "hidden" => interp_hidden_cmd(vm, rest),
        "invokehidden" => interp_invokehidden_cmd(vm, rest),
        "debug" => interp_debug_cmd(vm, rest),
        "bgerror" => interp_bgerror_cmd(vm, rest),
        "limit" => interp_limit_cmd(vm, rest),
        // `interp marktrusted path` — clear a child's safe flag. A safe
        // interpreter may not mark anything trusted (checked on the executor).
        "marktrusted" => match rest {
            [path] => {
                if vm.is_safe() {
                    return err("permission denied: safe interpreter cannot mark trusted");
                }
                let id = match vm.resolve_interp_path_original(path) {
                    Ok(id) => id,
                    Err(error) => return error,
                };
                vm.mark_interp_trusted(id);
                ok(Value::empty())
            }
            _ => native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"interp marktrusted path\"",
            ),
        },
        // Channel sharing/transfer between interps. Channels are per-interp in
        // this model; accept the operation so a script that wires up shared
        // channels at top level runs to completion (the dependent reads are
        // covered by the per-interp channel table, not a global one).
        "share" | "transfer" => ok(Value::empty()),
        _ => vm.refuse_host_command("native interpreter worker unavailable".to_owned()),
    }
}
