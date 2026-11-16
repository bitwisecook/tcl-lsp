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

//! `rename`: alias-loop checks, destination failures and relocating a renamed procedure.

use super::error_completion::completion_from_cmd_error;
use super::{Command, native_wrong_arguments_message};
use crate::interp::{Vm, ok};
use crate::value::Value;
use tcl_runtime_api::Completion;

/// `rename oldName newName` — rename a command, or delete it when `newName` is
/// empty.
pub(super) fn cmd_rename(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [old, new] = args else {
        return native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"rename oldName newName\"",
        );
    };
    let old_name = match vm.native_name_operand_bytes_checked(old) {
        Ok(bytes) => bytes,
        Err(error) => return completion_from_cmd_error(vm, error),
    };
    let new_name = match vm.native_name_operand_bytes_checked(new) {
        Ok(bytes) => bytes,
        Err(error) => return completion_from_cmd_error(vm, error),
    };
    let _original_source = match vm
        .original_jim_publication_key(old, tcl_syntax::naming::NativeNamePurpose::RenameSource)
    {
        Ok(key) => key,
        Err(error) => return completion_from_cmd_error(vm, error),
    };
    let original_destination = if new_name.is_empty() {
        None
    } else {
        match vm.original_jim_publication_key(
            new,
            tcl_syntax::naming::NativeNamePurpose::RenameDestination,
        ) {
            Ok(key) => key,
            Err(error) => return completion_from_cmd_error(vm, error),
        }
    };
    if let Some(error) = vm.jim_local_rename_error(&old_name, new_name.is_empty()) {
        return error;
    }
    // C's `TclRenameCommand` looks the source up first, so a missing source is
    // the error even when the destination is taken.
    let prepared = match vm.prepare_command_rename(&old_name) {
        Ok(prepared) => prepared,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    let Some((cmd, mut rename)) = prepared else {
        let failure = if new_name.is_empty() {
            tcl_syntax::naming::NativeRenameFailure::MissingDeleteSource
        } else {
            tcl_syntax::naming::NativeRenameFailure::MissingSource
        };
        return match vm.native_rename_diagnostic(failure, &old_name) {
            Ok(error) => completion_from_cmd_error(vm, error),
            Err(error) => vm.refuse_host_command(error.to_string()),
        };
    };
    // Tcl rejects a rename onto an existing command (leaving both intact), so
    // check the destination, once the source is found, before removing it.
    if !new_name.is_empty()
        && let Some(completion) = rename_destination_failure(vm, &new_name)
    {
        return completion;
    }
    let old_key = rename.old_key().to_owned();
    // A coroutine's state travels with its command: a delete (`rename $coro {}`)
    // drops it (no `finally` runs — the frozen continuation is inert data,
    // matching C Tcl); a rename re-keys it so it keeps working under the new name.
    // The prepared transaction holds the exact key resolved by Tcl's full
    // command lookup rule (including `namespace path`), rather than the
    // spelling rooted at the caller's current namespace.
    let is_coro = crate::cmd_coro::is_coroutine(vm, &old_key);
    if new_name.is_empty() {
        let trace_handled = vm.delete_prepared_renamed_command(&rename);
        // `rename x {}` is a delete: fire the command's `delete` traces
        // (`callback ::old {} delete`, tclsh-pinned) and drop its traces.
        if !trace_handled {
            vm.on_command_removed(&old_key);
        }
    } else {
        // Reserve the exact `(namespace token, simple name)` destination.
        // Its Tcl display can collide with another legal command, so
        // every lifecycle map below uses this private injective key.
        let key = match vm.note_rename_destination(&new_name, original_destination) {
            Ok(key) => key,
            Err(error) => return vm.refuse_host_command(error.to_string()),
        };
        // Procedure provenance keeps a display projection for compatibility,
        // but it must come from the resolved destination slot. The written
        // `b::y` inside `::a` names `::a::b::y`, not a raw `b::y` key.
        let display_key = key.clone();
        if is_coro {
            crate::cmd_coro::on_command_renamed(vm, &old_key, &key);
        }
        // A proc executes in the namespace it currently lives in, so renaming
        // it across namespaces re-homes its body (C `TclRenameCommand` updates
        // the command's `nsPtr`). `namespace current` inside the body then
        // reports the destination namespace (proc-3.4). The key is canonical,
        // so the shared qualifier split yields its namespace directly.
        let cmd = relocate_renamed_procedure(vm, cmd, &key, display_key);
        // C creates the destination's hash entry before `TclPreventAliasLoop`
        // and deletes it again on a refusal, so the resize that transient entry
        // triggers outlives the rejected rename.
        if let Err(completion) = check_renamed_alias_loop(vm, &rename, &key, &old_key, &cmd) {
            return completion;
        }
        if let Err(error) = vm.install_renamed_command(&mut rename, &key, cmd) {
            return completion_from_cmd_error(vm, error);
        }
        vm.commit_renamed_command(&rename);
        let key = rename.new_key().to_owned();
        // The rename happened: move every trace to the new key so it keeps
        // firing under the new name, then fire the command's `rename` traces
        // (`callback ::old ::new rename`, both fully qualified — tclsh-pinned).
        // The source is still registered here, matching C's window between
        // creating the destination entry and `Tcl_DeleteHashEntry(oldHPtr)`:
        // a callback resolves *both* names, and reaches the one trace list
        // through either (tclsh-pinned on 8.6.16 and 9.0.4).
        vm.on_command_renamed_traces(&old_key, &key);
        vm.retire_renamed_command_source(&rename);
    }
    ok(Value::empty())
}

fn check_renamed_alias_loop(
    vm: &mut Vm,
    rename: &crate::interp::CommandRenameTransaction,
    key: &str,
    old_key: &str,
    cmd: &Command,
) -> Result<(), Completion<Value>> {
    let is_alias = matches!(
        cmd,
        Command::Alias { .. } | Command::CallerAlias(_) | Command::CrossAlias { .. }
    );
    let loops = if is_alias {
        match vm.alias_chain_loops_for_rename_transaction(rename, key, cmd) {
            Ok(loops) => loops,
            Err(error) => {
                vm.forget_rename_destination(key);
                return Err(completion_from_cmd_error(vm, error));
            }
        }
    } else {
        false
    };
    if loops {
        let reported = vm.actual_native_name_protocol().and_then(|recipe| {
            let record = match cmd {
                Command::Alias {
                    publication_name, ..
                }
                | Command::CrossAlias {
                    publication_name, ..
                } => publication_name.clone(),
                _ => vm.command_slot_parts(old_key)?.1,
            };
            let (_, destination) = vm.command_slot_parts(key)?;
            recipe
                .rename_alias_loop_name(record.as_bytes(), destination.as_bytes())
                .map(<[u8]>::to_vec)
        });
        vm.forget_rename_destination(key);
        let Some(reported) = reported else {
            return Err(
                vm.refuse_host_command("alias rename diagnostic binding is unavailable".into())
            );
        };
        return Err(completion_from_cmd_error(
            vm,
            tcl_cmd_core::CmdError::new_bytes(
                [
                    &b"cannot define or rename alias \""[..],
                    reported.as_slice(),
                    b"\": would create a loop",
                ]
                .concat(),
            ),
        ));
    }
    Ok(())
}

fn rename_destination_failure(vm: &mut Vm, new_name: &[u8]) -> Option<Completion<Value>> {
    match vm.rename_destination_exists(new_name) {
        Ok(true) => Some(
            match vm.native_rename_diagnostic(
                tcl_syntax::naming::NativeRenameFailure::TargetExists,
                new_name,
            ) {
                Ok(error) => completion_from_cmd_error(vm, error),
                Err(error) => vm.refuse_host_command(error.to_string()),
            },
        ),
        Ok(false) => None,
        Err(error) => Some(vm.refuse_host_command(error.to_string())),
    }
}

fn relocate_renamed_procedure(
    vm: &mut Vm,
    cmd: Command,
    key: &str,
    display_key: String,
) -> Command {
    match cmd {
        Command::Proc(def) => {
            let (new_ns_id, simple_name) = vm
                .command_slot_parts(key)
                .expect("rename destination carries an exact command slot");
            let new_ns = vm.namespace_path_for_token(new_ns_id);
            let declaration = def.declaration();
            let current = declaration.actual_command_slot();
            if new_ns_id != current.namespace || simple_name != current.simple {
                let (body_namespace, jim_namespace) = if vm.uses_native_jim_lookup() {
                    let holder = vm
                        .name_policy_protocol()
                        .expect("actual Jim names")
                        .recipe()
                        .jim_procedure_relocation_namespace(simple_name.as_bytes())
                        .expect("actual Jim namespace recipe");
                    if let Some(holder) = holder {
                        let token = vm.intern_jim_namespace_object(holder);
                        (
                            crate::interp::ProcedureNamespace::Resolved {
                                path: vm.namespace_path_for_token(token),
                                token,
                            },
                            Some(vm.new_jim_declaration_namespace(token)),
                        )
                    } else {
                        (
                            declaration.actual_namespace_context(),
                            declaration.retained_jim_namespace(),
                        )
                    }
                } else {
                    (
                        crate::interp::ProcedureNamespace::Resolved {
                            path: new_ns,
                            token: new_ns_id,
                        },
                        None,
                    )
                };
                def.relocate_with_body_namespace(
                    display_key,
                    new_ns_id,
                    simple_name,
                    body_namespace,
                    jim_namespace,
                );
            }
            Command::Proc(def)
        }
        other => other,
    }
}
