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

//! `rename` + `interp alias`.
//!
//! Both layer on the one command resolver in [`crate::namespace`]: `rename`
//! moves/deletes a binding in the table; `interp alias` installs a
//! [`Command::Alias`](crate::interp::Command) redirect that the dispatch
//! trampoline re-resolves *by name, anchored at global, on every call*. See
//! `docs/design/runtime/rename-alias.md` for the as-built contract and
//! `docs/design/contracts/command-alias-resolution.md` for the binding rules.
//!
//! An alias whose source and target interpreter paths are both `{}` binds
//! within one interpreter; a child-side alias naming the parent binds as
//! `Command::ParentAlias`, dispatched through the parent `Weak` under
//! `CROSS_INTERP_DEPTH`. Querying either alias retains original members: C
//! returns a fresh List, while Jim returns its stored List. A non-empty alias
//! target path remains unsupported.
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::interp::native_arguments::OriginalArguments;
use crate::interp::{drop_fresh, Code, CommandVisibilityOp, Interp};
use crate::namespace::RenameOutcome;
use crate::obj::{self, TclObj};
#[cfg(test)]
use tcl_syntax::value::ValueOps;

mod alias_subcommands;
mod child_interpreters;
mod hidden_commands;

pub(crate) use alias_subcommands::child_alias_handle_original;
use alias_subcommands::{interp_alias, interp_aliases};
use child_interpreters::{
    interp_create, interp_debug, interp_delete, interp_eval, interp_limit, interp_marktrusted,
    interp_target,
};
pub(crate) use child_interpreters::{interp_path, not_found_original_path};
pub(crate) use hidden_commands::{hidectl_in, invokehidden_in};
use hidden_commands::{interp_hidectl, interp_invokehidden};

/// Register `rename`, `interp`, and the selected Jim core `alias`.
pub fn install(interp: &mut Interp) {
    interp.register_stock_builtin(b"rename", rename);
    interp.register_stock_builtin(b"interp", interp_cmd);
    if interp
        .native_invocation_dialect()
        .native_bootstrap_protocol()
        .is_some_and(|protocol| protocol.registers_core_init_auto_path())
    {
        interp.register_stock_builtin(b"::tcl::InitAutoPath", Interp::init_auto_path);
    }
    if interp
        .native_invocation_dialect()
        .native_jim_lookup_protocol()
        .is_some()
    {
        interp.register_stock_builtin(b"alias", jim_alias);
    }
    // `update` is registered by `cmd_event` (the real event loop).
}

/// Jim's core command retains the original name result and prefix members.
/// Publication and replacement still use the shared alias-slot owner.
fn jim_alias(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return interp.wrong_args_for_invocation(argv, b"newname command ?args ...?");
    }
    let arguments = match OriginalArguments::enter(interp, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    if let Err(error) = interp.original_import_alias_publication(argv, false) {
        return interp.report_cmd_error(error.into());
    }
    let original_prefix = std::rc::Rc::new(crate::interp::NativeAliasHeader::new(
        obj::Owned::fresh(interp.new_list_object(&argv[2..])),
    ));
    let Some(recipe) = interp
        .name_policy_protocol()
        .map(|protocol| protocol.recipe())
    else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original alias capture issuer",
            )
            .into(),
        );
    };
    // Jim retains the prefix and sets the original name result before reaching
    // command registration. No target or prefix String getter is reached.
    if recipe.alias_result_precedes_name_registration() {
        interp.set_result(argv[1]);
        if let Err(error) = arguments.check() {
            return interp.report_cmd_error(error);
        }
    }
    // Jim_RegisterCommand's temporary begins after the original-name result
    // publication and remains live through the hash lookup/keyDup operation.
    let registration_name = std::rc::Rc::new(crate::interp::NativeAliasHeader::new(
        obj::Owned::retain(argv[1]),
    ));
    let code = match arguments.bytes(1) {
        Ok(name) => match interp.install_alias_objects(&name, registration_name, original_prefix) {
            Ok(None) => Code::Ok,
            Ok(Some(simple)) => alias_loop_error(interp, &simple),
            Err(error) => interp.report_cmd_error(error),
        },
        Err(error) => interp.report_cmd_error(error),
    };
    let code = arguments.finish(code);
    if code == Code::Ok {
        if let Err(error) = interp.original_import_alias_publication(argv, true) {
            return interp.report_cmd_error(error.into());
        }
    }
    code
}

// rename

/// `rename oldName newName` — move a command, or delete it when `newName` is the
/// empty string. Any command may be renamed, builtins included — C Tcl has no
/// protected list here (`rename ::return ::myreturn` succeeds on tclsh
/// 8.6.16 / 9.0.4).
fn rename(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let arguments = match OriginalArguments::enter(interp, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    let code = rename_original(interp, argv, &arguments);
    arguments.finish(code)
}

fn rename_original(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    arguments: &OriginalArguments,
) -> Code {
    if argv.len() != 3 {
        return interp.wrong_args(b"rename oldName newName");
    }
    let (old, new, original_keys) = if interp
        .name_policy_protocol()
        .is_some_and(|protocol| protocol.recipe().is_jim084())
    {
        match interp.original_jim_rename_operands(argv[1], argv[2]) {
            Ok((old, new, old_key, new_key)) => (old, new, Some((old_key, new_key))),
            Err(error) => return interp.report_cmd_error(error.into()),
        }
    } else {
        match interp.original_rename_operands(argv[1], argv[2]) {
            Ok((old, new)) => (old, new, None),
            Err(error) => return interp.report_cmd_error(error.into()),
        }
    };
    if let Some(recipe) = interp
        .native_invocation_dialect()
        .native_jim_local_protocol()
    {
        let previous = {
            let namespaces = interp.namespaces();
            namespaces
                .resolve_generation(interp.current_ns(), &old)
                .is_some_and(|token| namespaces.jim_has_previous(token))
        };
        if !recipe.accepts_rename(previous, new.is_empty()) {
            return interp.error(&recipe.local_rename_error(&old));
        }
    }
    let outcome = interp.rename_command_with_original_jim_key(
        &old,
        &new,
        original_keys
            .as_ref()
            .and_then(|(_, new)| new.as_ref())
            .cloned(),
    );
    // Jim_RenameCommand retires old/new qualification temporaries before its
    // caller publishes a result or constructs a Guest diagnostic. Their real
    // final-reference callbacks are checked by the same original operation.
    drop(original_keys);
    // Rename/delete observers may refuse or replace the execution context.
    // Keep their reached effects and mandatory retirement, then stop before
    // either success-result publication or Guest error construction.
    if let Err(error) = arguments.check() {
        return interp.report_cmd_error(error);
    }
    match outcome {
        RenameOutcome::Renamed | RenameOutcome::Deleted => {
            interp.set_result_bytes(b"");
            Code::Ok
        }
        RenameOutcome::NoSuchCommand => rename_diagnostic(
            interp,
            if new.is_empty() {
                tcl_syntax::naming::NativeRenameFailure::MissingDeleteSource
            } else {
                tcl_syntax::naming::NativeRenameFailure::MissingSource
            },
            &old,
        ),
        RenameOutcome::AliasLoop => match interp.original_rename_alias_loop_name(&old, &new) {
            Ok(reported) => alias_loop_error(interp, &reported),
            Err(error) => interp.report_cmd_error(error.into()),
        },
        RenameOutcome::TargetExists => rename_diagnostic(
            interp,
            tcl_syntax::naming::NativeRenameFailure::TargetExists,
            &new,
        ),
    }
}

fn rename_diagnostic(
    interp: &mut Interp,
    failure: tcl_syntax::naming::NativeRenameFailure,
    original: &[u8],
) -> Code {
    let Some(policy) = interp.name_policy_protocol() else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("rename name reporting")
                .into(),
        );
    };
    let error = policy.recipe().rename_diagnostic(failure, original);
    match error.error_code {
        Some(code) => interp.error_with_code(&error.message, &code),
        None => interp.error(&error.message),
    }
}

/// C's `TclPreventAliasLoop` refusal (`tclInterp.c`), shared by the `interp
/// alias` and `rename` gates.
fn alias_loop_error(interp: &mut Interp, simple: &[u8]) -> Code {
    let mut m = b"cannot define or rename alias \"".to_vec();
    m.extend_from_slice(simple);
    m.extend_from_slice(b"\": would create a loop");
    interp.error_with_code(&m, b"TCL OPERATION INTERP ALIASLOOP")
}

// interp

/// Resolve the selected Jim child-handle buffer query. Actual C root and child
/// command operands use `native_interpreter_option_from_original`, whose
/// retained native declarations and C9 second lookup also update the original.
pub(crate) fn resolve_interp_option(
    dispatch: &'static [&'static [u8]],
    advertised: &[&'static [u8]],
    word: &[u8],
) -> Result<&'static [u8], Vec<u8>> {
    match tcl_cmd_core::prefix::scan(dispatch, word, false) {
        tcl_cmd_core::prefix::Resolution::Exact(i)
        | tcl_cmd_core::prefix::Resolution::UniquePrefix(i) => Ok(dispatch[i]),
        miss => Err(tcl_cmd_core::prefix::bad_key_message(
            advertised,
            b"option",
            word,
            matches!(miss, tcl_cmd_core::prefix::Resolution::Ambiguous),
        )),
    }
}

/// The `interp` ensemble. `alias`, `aliases`, `create`, `delete`, `eval`,
/// `exists`, `hide`, `expose`, `hidden`, `invokehidden`, `issafe`,
/// `marktrusted`, `recursionlimit`, `bgerror`, `debug`, `limit`, and `target`
/// dispatch here. The option list is the release's own; `cancel`, `share`
/// and `transfer` resolve but have no worker in this runtime yet.
fn interp_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let arguments = match OriginalArguments::enter(interp, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    let code = interp_cmd_original(interp, argv, &arguments);
    arguments.finish(code)
}

fn interp_cmd_original(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    arguments: &OriginalArguments,
) -> Code {
    if interp
        .native_invocation_dialect()
        .native_jim_lookup_protocol()
        .is_some()
    {
        return if argv.len() == 1 {
            let handle = interp.create_jim_child();
            if interp.host_refusal_pending() {
                Code::Error
            } else {
                interp.set_result_bytes(&handle);
                Code::Ok
            }
        } else {
            interp.wrong_args(b"interp")
        };
    }
    if argv.len() < 2 {
        return interp.wrong_args(b"interp cmd ?arg ...?");
    }
    let selected = interp.native_interpreter_option_from_original(argv[1], false);
    if let Err(error) = arguments.check() {
        return interp.report_cmd_error(error);
    }
    let sub = match selected {
        Ok(name) => name.as_bytes(),
        Err(error) => return interp.report_cmd_error(error),
    };
    match sub {
        b"alias" => interp_alias(interp, argv),
        b"aliases" => interp_aliases(interp, argv),
        b"create" => interp_create(interp, argv),
        b"eval" => interp_eval(interp, argv),
        b"delete" => interp_delete(interp, argv),
        b"exists" => {
            if argv.len() > 3 {
                return interp.wrong_args(b"interp exists ?path?");
            }
            let path = match argv.get(2) {
                None => Vec::new(),
                Some(&original) => match interp_path(interp, original) {
                    Ok(path) => path,
                    Err(code) if interp.host_refusal_pending() => return code,
                    Err(_) => {
                        interp.set_result_bytes(b"0");
                        return Code::Ok;
                    }
                },
            };
            let exists = interp.with_child_path(&path, |_| ()).is_some();
            interp.set_result_bytes(if exists { b"1" } else { b"0" });
            Code::Ok
        }
        b"children" | b"slaves" => {
            if argv.len() > 3 {
                return interp.wrong_args(b"interp children ?path?");
            }
            // Children of the interp addressed by the (possibly nested) path.
            let path = match argv.get(2) {
                None => Vec::new(),
                Some(&original) => match interp_path(interp, original) {
                    Ok(path) => path,
                    Err(code) => return code,
                },
            };
            let names = interp.with_child_path(&path, |c| c.child_names());
            let Some(names) = names else {
                return not_found_original_path(interp, argv[2]);
            };
            let elems: Vec<*mut TclObj> = names.iter().map(|n| obj::new_string_bytes(n)).collect();
            interp.set_result(interp.new_list_object(&elems));
            for e in elems {
                drop_fresh(e);
            }
            Code::Ok
        }
        b"bgerror" => {
            // `interp bgerror path ?cmdPrefix?` — get/set the (possibly nested)
            // interp's background-error handler.
            if argv.len() < 3 || argv.len() > 4 {
                return interp.wrong_args(b"interp bgerror path ?cmdPrefix?");
            }
            let path = match interp_path(interp, argv[2]) {
                Ok(path) => path,
                Err(code) => return code,
            };
            let prefix = argv.get(3).copied();
            match interp.with_child_path(&path, |c| c.bgerror_apply(prefix)) {
                Some(Ok(h)) => {
                    unsafe {
                        interp.set_obj_result(h.as_ptr());
                    }
                    Code::Ok
                }
                Some(Err(error)) => interp.report_cmd_error(error),
                None => not_found_original_path(interp, argv[2]),
            }
        }
        b"hide" => interp_hidectl(interp, argv, CommandVisibilityOp::Hide),
        b"expose" => interp_hidectl(interp, argv, CommandVisibilityOp::Expose),
        b"invokehidden" => interp_invokehidden(interp, argv),
        b"limit" => interp_limit(interp, argv),
        b"marktrusted" => interp_marktrusted(interp, argv),
        b"debug" => interp_debug(interp, argv),
        b"hidden" => {
            // `interp hidden ?path?` — hidden command names in the interp
            // addressed by the (possibly nested) path.
            if argv.len() > 3 {
                return interp.wrong_args(b"interp hidden ?path?");
            }
            let path = match argv.get(2) {
                None => Vec::new(),
                Some(&original) => match interp_path(interp, original) {
                    Ok(path) => path,
                    Err(code) => return code,
                },
            };
            let names = interp.with_child_path(&path, |c| c.hidden_names());
            let Some(names) = names else {
                return not_found_original_path(interp, argv[2]);
            };
            let elems: Vec<*mut TclObj> = names.iter().map(|n| obj::new_string_bytes(n)).collect();
            interp.set_result(interp.new_list_object(&elems));
            for e in elems {
                drop_fresh(e);
            }
            Code::Ok
        }
        b"issafe" => {
            // `interp issafe ?path?` — the current interp (no path) or a child
            // addressed by a (possibly nested) path.
            if argv.len() > 3 {
                return interp.wrong_args(b"interp issafe ?path?");
            }
            let path = match argv.get(2) {
                None => Vec::new(),
                Some(&original) => match interp_path(interp, original) {
                    Ok(path) => path,
                    Err(code) => return code,
                },
            };
            let safe = interp.with_child_path(&path, |c| c.is_safe());
            let Some(safe) = safe else {
                return not_found_original_path(interp, argv[2]);
            };
            interp.set_result_bytes(if safe { b"1" } else { b"0" });
            Code::Ok
        }
        b"recursionlimit" => {
            // `interp recursionlimit path ?newlimit?` — get/set a (possibly
            // nested) interp's recursion bound.
            if argv.len() < 3 || argv.len() > 4 {
                return interp.wrong_args(b"interp recursionlimit path ?newlimit?");
            }
            let path = match interp_path(interp, argv[2]) {
                Ok(path) => path,
                Err(code) => return code,
            };
            let newlimit = if argv.len() == 4 {
                if let Err(error) = arguments.bytes(3) {
                    return interp.report_cmd_error(error);
                }
                argv.get(3).copied()
            } else {
                None
            };
            match interp.with_child_path(&path, |c| c.recursion_limit_apply(newlimit)) {
                Some(Ok(n)) => {
                    // C answers a new limit with the word it was given.
                    match newlimit {
                        Some(word) => interp.set_result(word),
                        None => interp.set_result_bytes(n.to_string().as_bytes()),
                    }
                    Code::Ok
                }
                Some(Err(error)) => interp.report_cmd_error(error),
                None => not_found_original_path(interp, argv[2]),
            }
        }
        b"target" => interp_target(interp, argv),
        _ => interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("native interpreter worker")
                .into(),
        ),
    }
}

// helpers

fn only_single_interp(interp: &mut Interp) -> Code {
    interp.set_error(b"only single-interp aliases (empty interpreter paths) are supported")
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod native_option_tables;

#[cfg(test)]
pub(crate) mod original_operand_tests;

#[cfg(test)]
mod original_rename_operation_tests;

#[cfg(test)]
mod original_alias_storage_tests;

#[cfg(test)]
mod original_alias_key_tests;

#[cfg(test)]
mod native_positive_option_prefixes;
