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
use tcl_syntax::value::ValueOps;

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
/// dispatch here. `cancel` (script cancellation), `share`, and `transfer`
/// (cross-interp channel sharing) are tclsh subcommands this runtime has no
/// infrastructure for — no cancellation flag on eval, no channel-table
/// sharing between interps — so, unlike `target`, implementing them is not
/// cheap; the bad-option list below advertises only what actually dispatches
/// here, rather than tclsh's full list.
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

/// `interp target path alias` — the interp-path (from this interp) to the
/// target interpreter of `alias`, as installed in the interpreter addressed
/// by `path`. Cheap given the two alias shapes this runtime supports
/// (same-interp, or child-to-immediate-parent) — see
/// [`Interp::alias_target_path`]. `cancel`/`share`/`transfer` are the other
/// three subcommands tclsh advertises here that this runtime does not
/// implement; unlike `target` they need infrastructure (script cancellation,
/// cross-interp channel sharing) this runtime has none of.
fn interp_target(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let arguments = match OriginalArguments::enter(interp, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    let code = interp_target_original(interp, argv, &arguments);
    arguments.finish(code)
}

fn interp_target_original(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    arguments: &OriginalArguments,
) -> Code {
    if argv.len() != 4 {
        return interp.wrong_args(b"interp target path alias");
    }
    let path = match interp_path(interp, argv[2]) {
        Ok(path) => path,
        Err(code) => return code,
    };
    let alias = match arguments.bytes(3) {
        Ok(bytes) => bytes,
        Err(error) => return interp.report_cmd_error(error),
    };
    match interp.alias_target_path(&path, &alias) {
        Ok(Some(target_path)) => {
            let elems: Vec<*mut TclObj> = target_path
                .iter()
                .map(|n| obj::new_string_bytes(n))
                .collect();
            interp.set_result(interp.new_list_object(&elems));
            for e in elems {
                drop_fresh(e);
            }
            Code::Ok
        }
        Err(error) => interp.report_cmd_error(error.into()),
        Ok(None) => {
            let mut m = b"alias \"".to_vec();
            m.extend_from_slice(&alias);
            m.extend_from_slice(b"\" in path \"");
            let original_path = match arguments.bytes(2) {
                Ok(bytes) => bytes,
                Err(error) => return interp.report_cmd_error(error),
            };
            m.extend_from_slice(&original_path);
            m.extend_from_slice(b"\" not found");
            let code = crate::interp::error_code_list(&[b"TCL", b"LOOKUP", b"ALIAS", &alias]);
            interp.error_with_code(&m, &code)
        }
    }
}

/// `interp create`'s option words (`createOptions[]`, `tclInterp.c`), resolved
/// with `Tcl_GetIndexFromObj(…, "option", 0)`: `-s` abbreviates `-safe` and the
/// lone `-` — a prefix of both entries — is `ambiguous option "-"`. Only a word
/// starting with `-` reaches the table, so an empty word is a path, not a miss.
const CREATE_OPTIONS: tcl_cmd_core::prefix::OptionTable<'static, &[u8]> =
    tcl_cmd_core::prefix::OptionTable::abbreviating("option", &[b"-safe", b"--"]);

/// `interp create ?-safe? ?--? ?path?` — create a child interpreter, returning
/// its name (auto-generated `interpN` when omitted). `-safe` hides the
/// host-touching commands; it does not re-alias `source`/`load`/`file`
/// through the Safe Base.
fn interp_create(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let arguments = match OriginalArguments::enter(interp, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    let code = interp_create_original(interp, argv, &arguments);
    arguments.finish(code)
}

fn interp_create_original(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    arguments: &OriginalArguments,
) -> Code {
    // C's "weird historical rule": `-safe` is accepted anywhere before `--`
    // (`interp create a -safe` is valid), and the path is the lone non-option
    // word — so scan all args rather than stopping at the first non-flag.
    let mut name_obj: Option<*mut TclObj> = None;
    let mut safe = false;
    let mut last = false;
    let mut i = 2;
    while i < argv.len() {
        let a = match arguments.bytes(i) {
            Ok(bytes) => bytes,
            Err(error) => return interp.report_cmd_error(error),
        };
        if !last && a.first() == Some(&b'-') {
            let selected =
                interp.native_static_option_index(argv[i], CREATE_OPTIONS.names(), false, "option");
            if let Err(error) = arguments.check() {
                return interp.report_cmd_error(error);
            }
            match selected {
                Ok(0) => {
                    safe = true;
                    i += 1;
                    continue;
                }
                Ok(_) => {
                    i += 1;
                    last = true;
                }
                Err(m) => {
                    return interp.report_cmd_error(m);
                }
            }
        }
        if name_obj.is_some() {
            return interp.wrong_args(b"interp create ?-safe? ?--? ?path?");
        }
        if i < argv.len() {
            name_obj = Some(argv[i]);
        }
        i += 1;
    }
    // A safe parent makes every child safe, with or without `-safe`
    // (`Tcl_CreateChild`).
    let Some(original) = name_obj else {
        let created = interp.create_child(None);
        if interp.host_refusal_pending() {
            return Code::Error;
        }
        if safe || interp.is_safe() {
            interp.with_child(&created, |child| child.make_safe());
        }
        interp.set_result_bytes(&created);
        return Code::Ok;
    };
    let path = match interp_path(interp, original) {
        Ok(path) => path,
        Err(code) => return code,
    };
    interp_create_from_original_path(interp, original, &path, safe)
}

fn interp_create_from_original_path(
    interp: &mut Interp,
    original: *mut TclObj,
    path: &[Vec<u8>],
    safe: bool,
) -> Code {
    let (leaf, parent) = if path.len() < 2 {
        let bytes = match interp.native_string_bytes(&original) {
            Ok(bytes) => bytes,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        let policy = interp
            .name_policy_protocol()
            .expect("path selected the original protocol");
        let selected = match policy.recipe().interpreter_child_input(&bytes) {
            Ok(selected) => selected.selected().to_vec(),
            Err(_) => {
                return interp.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "interpreter creation key",
                    )
                    .into(),
                );
            }
        };
        (selected, &path[..0])
    } else {
        (path.last().unwrap().clone(), &path[..path.len() - 1])
    };
    let outcome = interp.with_child_path(parent, |owner| {
        if owner.child_exists(&leaf) {
            return Ok(false);
        }
        owner.create_child(Some(leaf.clone()));
        if owner.host_refusal_pending() {
            return Err(owner.clone());
        }
        if safe || owner.is_safe() {
            owner.with_child(&leaf, |child| child.make_safe());
        }
        Ok(true)
    });
    match outcome {
        Some(Err(owner)) if !interp.host_refusal_pending() => {
            interp.transport_host_refusal_from(&owner)
        }
        Some(Err(_)) => Code::Error,
        Some(Ok(true)) => {
            interp.set_result(original);
            Code::Ok
        }
        Some(Ok(false)) => interp.set_error(
            &[
                b"interpreter named \"".as_slice(),
                &leaf,
                b"\" already exists, cannot create",
            ]
            .concat(),
        ),
        None => not_found_path(interp, parent),
    }
}

/// Parse the original list object, then select each C child-table CString key.
/// A failed native list/getter operation never becomes a literal-name fallback.
pub(crate) fn interp_path(
    interp: &mut Interp,
    original: *mut TclObj,
) -> Result<Vec<Vec<u8>>, Code> {
    let arguments = OriginalArguments::enter(interp, &[original])
        .map_err(|error| interp.report_cmd_error(error))?;
    let policy = interp.name_policy_protocol().ok_or_else(|| {
        interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("interpreter path protocol")
                .into(),
        )
    })?;
    let elements = ValueOps::list_elements(interp, &original);
    arguments
        .check()
        .map_err(|error| interp.report_cmd_error(error))?;
    let elements = elements.map_err(|error| interp.report_cmd_error(error.into()))?;
    let members = OriginalArguments::enter(interp, &elements)
        .map_err(|error| interp.report_cmd_error(error))?;
    let selected = (0..elements.len())
        .map(|index| {
            let bytes = members.bytes(index);
            arguments
                .check()
                .map_err(|error| interp.report_cmd_error(error))?;
            let bytes = bytes.map_err(|error| interp.report_cmd_error(error))?;
            let selected = policy
                .recipe()
                .interpreter_child_input(&bytes)
                .map_err(|_| {
                    interp.report_cmd_error(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "interpreter child key",
                        )
                        .into(),
                    )
                })?;
            Ok(selected.selected().to_vec())
        })
        .collect::<Result<Vec<_>, Code>>()?;
    members
        .check()
        .map_err(|error| interp.report_cmd_error(error))?;
    arguments
        .check()
        .map_err(|error| interp.report_cmd_error(error))?;
    Ok(selected)
}

pub(crate) fn not_found_original_path(interp: &mut Interp, original: *mut TclObj) -> Code {
    let bytes = match interp.native_string_bytes(&original) {
        Ok(bytes) => bytes,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let Some(policy) = interp.name_policy_protocol() else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "interpreter diagnostic protocol",
            )
            .into(),
        );
    };
    let reported = match policy.recipe().interpreter_child_input(&bytes) {
        Ok(selected) => selected.selected().to_vec(),
        Err(_) => {
            return interp.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "interpreter diagnostic extent",
                )
                .into(),
            );
        }
    };
    let code = tcl_syntax::list_result::NativeListResultSerialization::for_string_protocol(
        policy.string_protocol(),
    )
    .render(&[b"TCL".as_slice(), b"LOOKUP", b"INTERP", &reported]);
    interp.error_with_code(
        &[
            b"could not find interpreter \"".as_slice(),
            &reported,
            b"\"",
        ]
        .concat(),
        &code,
    )
}

/// The `could not find interpreter "a b"` error for a path that failed to
/// resolve, rendering the path as a Tcl list.
fn not_found_path(interp: &mut Interp, path: &[Vec<u8>]) -> Code {
    let elems: Vec<*mut TclObj> = path.iter().map(|n| obj::new_string_bytes(n)).collect();
    let joined = obj::Owned::fresh(interp.new_list_object(&elems));
    for e in elems {
        drop_fresh(e);
    }
    let arguments = match OriginalArguments::enter(interp, &[joined.as_ptr()]) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    let rendered = match arguments.bytes(0) {
        Ok(bytes) => bytes,
        Err(error) => return interp.report_cmd_error(error),
    };
    let mut m = b"could not find interpreter \"".to_vec();
    m.extend_from_slice(&rendered);
    m.push(b'"');
    arguments.finish(interp.set_error(&m))
}

/// `interp limit path limitType ?-option value …?` — query/configure the
/// `commands` or `time` limit on a child interp.
fn interp_limit(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let arguments = match OriginalArguments::enter(interp, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    let code = interp_limit_original(interp, argv, &arguments);
    arguments.finish(code)
}

fn interp_limit_original(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    arguments: &OriginalArguments,
) -> Code {
    // argv = [interp, limit, path, limitType, opts…]
    if argv.len() < 4 {
        return interp.wrong_args(b"interp limit path limitType ?-option value ...?");
    }
    // Validate the limit type before the current-interp guard so a bad type is
    // reported ahead of the inaccessibility error (interp-35.3 vs .23).
    if let Err(m) = interp.native_static_option_index(
        argv[3],
        crate::interp::LIMIT_TYPES.names(),
        false,
        "limit type",
    ) {
        return interp.report_cmd_error(m);
    }
    let path = match interp_path(interp, argv[2]) {
        Ok(path) => path,
        Err(code) => return code,
    };
    if path.is_empty() {
        return interp.set_error(b"limits on current interpreter inaccessible");
    }
    let ltype = match arguments.bytes(3) {
        Ok(bytes) => bytes,
        Err(error) => return interp.report_cmd_error(error),
    };
    let opts: Vec<*mut TclObj> = argv[4..].to_vec();
    match interp.with_child_path(&path, |c| c.limit_apply(&ltype, &opts)) {
        Some(Ok(o)) => {
            interp.set_result(o);
            Code::Ok
        }
        Some(Err(error)) => interp.report_cmd_error(error),
        None => not_found_original_path(interp, argv[2]),
    }
}

/// `interp marktrusted path` — clear a child interp's safe flag (denied from a
/// safe interpreter).
fn interp_marktrusted(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return interp.wrong_args(b"interp marktrusted path");
    }
    if interp.is_safe() {
        return interp.set_error(b"permission denied: safe interpreter cannot mark trusted");
    }
    let path = match interp_path(interp, argv[2]) {
        Ok(path) => path,
        Err(code) => return code,
    };
    if path.is_empty() {
        interp.set_result_bytes(b"");
        return Code::Ok;
    }
    match interp.with_child_path(&path, |c| c.mark_trusted()) {
        Some(()) => {
            interp.set_result_bytes(b"");
            Code::Ok
        }
        None => not_found_original_path(interp, argv[2]),
    }
}

/// `interp debug path ?-frame ?bool??` — the per-interp frame-debug switch.
fn interp_debug(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 || argv.len() > 5 {
        return interp.wrong_args(b"interp debug path ?-frame ?bool??");
    }
    let path = match interp_path(interp, argv[2]) {
        Ok(path) => path,
        Err(code) => return code,
    };
    let opts: Vec<*mut TclObj> = argv[3..].to_vec();
    match interp.with_child_path(&path, |c| c.debug_apply(&opts)) {
        Some(Ok(o)) => {
            interp.set_result(o);
            Code::Ok
        }
        Some(Err(error)) => interp.report_cmd_error(error),
        None => not_found_original_path(interp, argv[2]),
    }
}

/// The shared owner of `interp hide|expose path cmdName ?other?` and the
/// `$child hide|expose cmdName ?other?` shorthand — C's `ChildHide` /
/// `ChildExpose`, which `NRInterpCmd` and `NRChildCmd` both call rather than
/// each re-deriving the argument rules.
///
/// `words` is `[cmdName ?hiddenCmdName?]`, already arity-checked by the caller
/// (the two entry points word their `wrong # args` differently, and only they
/// know the noun). `cmd` is the source — the visible command when hiding, the
/// hidden token when exposing — and `token` the destination, defaulting to
/// `cmd` in the one-word form.
///
/// Only the destination test lives here; the rest of C's order is inside
/// [`Interp::hide_command`] / [`Interp::expose_command`], where the lookups
/// happen. The two directions are **not** mirror images:
///
/// - hiding refuses a qualified *token* — `Tcl_HideCommand` runs
///   `strstr(hiddenCmdToken, "::")` first (`tclBasic.c:2314`) — but accepts a
///   qualified-looking source, because that lookup is global-anchored anyway;
/// - exposing refuses a *destination* containing `::` anywhere, a leading one
///   included, because C tests it with the same raw `strstr`
///   (`Tcl_ExposeCommand`, `:2469`) — and has no token check at all, so a
///   qualified token is simply a token that is not in the hidden table.
pub(crate) fn hidectl_in(
    interp: &mut Interp,
    path: &[Vec<u8>],
    op: CommandVisibilityOp,
    words: &[*mut TclObj],
) -> Code {
    let arguments = match OriginalArguments::enter(interp, words) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    let code = hidectl_in_original(interp, path, op, words, &arguments);
    arguments.finish(code)
}

fn hidectl_in_original(
    interp: &mut Interp,
    path: &[Vec<u8>],
    op: CommandVisibilityOp,
    words: &[*mut TclObj],
    arguments: &OriginalArguments,
) -> Code {
    let cmd = match arguments.bytes(0) {
        Ok(bytes) => bytes,
        Err(error) => return interp.report_cmd_error(error),
    };
    let token = if words.len() == 2 {
        match arguments.bytes(1) {
            Ok(bytes) => bytes,
            Err(error) => return interp.report_cmd_error(error),
        }
    } else {
        cmd.clone()
    };
    if tcl_syntax::naming::is_qualified(&token) {
        return interp.report_command_visibility_failure(
            match op {
                CommandVisibilityOp::Hide => {
                    tcl_syntax::naming::NativeCommandVisibilityFailure::InvalidHiddenToken
                }
                CommandVisibilityOp::Expose => {
                    tcl_syntax::naming::NativeCommandVisibilityFailure::ExposeNonGlobal
                }
            },
            &token,
        );
    }
    let moved = interp.with_child_path(path, |c| match op {
        CommandVisibilityOp::Hide => c.hide_command(&cmd, &token),
        CommandVisibilityOp::Expose => c.expose_command(&cmd, &token),
    });
    if let Err(error) = arguments.check() {
        return interp.report_cmd_error(error);
    }
    let Some(moved) = moved else {
        return not_found_path(interp, path);
    };
    interp.finish_command_visibility(op, &cmd, &token, moved)
}

/// `interp hide|expose path cmdName` — move a command into/out of the hidden
/// table of the named (or current, when path is `{}`) interpreter. The
/// argument rules are [`hidectl_in`]'s; this arm owns only the arity message
/// and the executing-interp permission check.
fn interp_hidectl(interp: &mut Interp, argv: &[*mut TclObj], op: CommandVisibilityOp) -> Code {
    // `interp hide   path cmdName     ?hiddenCmdName?`
    // `interp expose path hiddenName  ?cmdName?`
    if argv.len() != 4 && argv.len() != 5 {
        return match op {
            CommandVisibilityOp::Hide => {
                interp.wrong_args(b"interp hide path cmdName ?hiddenCmdName?")
            }
            CommandVisibilityOp::Expose => {
                interp.wrong_args(b"interp expose path hiddenCmdName ?cmdName?")
            }
        };
    }
    // A safe interpreter may not touch the hidden-command table of itself or
    // any of its children (the check is on the *executing* interp).
    if interp.is_safe() {
        return interp.set_error(match op {
            CommandVisibilityOp::Hide => {
                b"permission denied: safe interpreter cannot hide commands"
            }
            CommandVisibilityOp::Expose => {
                b"permission denied: safe interpreter cannot expose commands"
            }
        });
    }
    let path = match interp_path(interp, argv[2]) {
        Ok(path) => path,
        Err(code) => return code,
    };
    hidectl_in(interp, &path, op, &argv[3..])
}

/// `interp invokehidden path ?-namespace ns? ?-global? ?--? cmdName ?arg
/// ...?` — invoke a hidden command in the named (or current) interpreter, in
/// the `-namespace`/`-global` evaluation context when given.
///
/// C's `ChildInvokeHidden` (`tclInterp.c`) takes the *last* of `-global`
/// (`::`) / `-namespace ns` given, not a mutual-exclusion refusal — passing
/// both is legal on tclsh 8.6.16/9.0.4, the last one simply wins; no
/// `cannot use -global option and -namespace option together` error exists
/// on either release. An unrecognised option is a hard `bad option` error.
/// `-namespace`'s namespace is resolved from the **global**
/// namespace regardless of the caller's current one, matching
/// `TCL_GLOBAL_ONLY` (tclsh-pinned: `-namespace bar` from inside `::foo`
/// still names `::bar`, not `::foo::bar`).
///
/// Original option words use the selected shared static-option owner, including
/// the declared unique-prefix abbreviations. Counted String production and
/// namespace/command name selection remain separate operations.
fn interp_invokehidden(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    const USAGE: &[u8] = b"interp invokehidden path ?-namespace ns? ?-global? ?--? cmd ?arg ..?";
    if argv.len() < 4 {
        return interp.wrong_args(USAGE);
    }
    let path = match interp_path(interp, argv[2]) {
        Ok(path) => path,
        Err(code) => return code,
    };
    invokehidden_in(interp, &path, &argv[3..], USAGE)
}

/// The shared owner of `interp invokehidden path …` and the `$child
/// invokehidden …` shorthand — C's `ChildInvokeHidden`, reached from both
/// `NRInterpCmd` and `NRChildCmd`.
///
/// `words` is everything after the path: the option words, the command word,
/// and its arguments. `usage` is the caller's own `wrong # args` text, which is
/// all that differs between the two entry points (`interp invokehidden path …`
/// versus `<child> invokehidden …`).
///
/// The safe-interpreter check runs **after** option parsing, as C does: the
/// options are read by the ensemble dispatcher and only then is
/// `ChildInvokeHidden` entered, so `interp invokehidden {} -bogus …` from a
/// safe interpreter reports the bad option rather than the permission denial.
pub(crate) fn invokehidden_in(
    interp: &mut Interp,
    path: &[Vec<u8>],
    words: &[*mut TclObj],
    usage: &[u8],
) -> Code {
    let arguments = match OriginalArguments::enter(interp, words) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    let code = invokehidden_in_original(interp, path, words, usage, &arguments);
    arguments.finish(code)
}

fn invokehidden_in_original(
    interp: &mut Interp,
    path: &[Vec<u8>],
    words: &[*mut TclObj],
    usage: &[u8],
    arguments: &OriginalArguments,
) -> Code {
    use tcl_syntax::naming::NativeHiddenInvocationOption;
    let Some(grammar) = interp
        .invocation_name_protocol()
        .and_then(|protocol| protocol.hidden_invocation_grammar().ok())
    else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native hidden-invocation grammar",
            )
            .into(),
        );
    };
    let mut ns_name: Option<Vec<u8>> = None;
    let mut i = 0;
    while i < words.len() {
        let opt = match arguments.bytes(i) {
            Ok(bytes) => bytes,
            Err(error) => return interp.report_cmd_error(error),
        };
        if opt.first() != Some(&b'-') {
            break;
        }
        let selected =
            interp.native_static_option_index(words[i], grammar.byte_names(), false, "option");
        if let Err(error) = arguments.check() {
            return interp.report_cmd_error(error);
        }
        let option = match selected {
            Ok(index) => match grammar.option(index) {
                Some(option) => option,
                None => {
                    return interp.report_cmd_error(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "native hidden-invocation option ordinal",
                        )
                        .into(),
                    );
                }
            },
            Err(error) => return interp.report_cmd_error(error),
        };
        match option {
            NativeHiddenInvocationOption::Global => {
                ns_name = Some(b"::".to_vec());
                i += 1;
            }
            NativeHiddenInvocationOption::Namespace => {
                i += 1;
                if i == words.len() {
                    // C: "there must be more arguments" — stop scanning
                    // options and fall through to the arg-count check below.
                    break;
                }
                ns_name = match arguments.bytes(i) {
                    Ok(bytes) => Some(bytes.to_vec()),
                    Err(error) => return interp.report_cmd_error(error),
                };
                i += 1;
            }
            NativeHiddenInvocationOption::End => {
                i += 1;
                break;
            }
        }
    }
    if i >= words.len() {
        return interp.wrong_args(usage);
    }
    if interp.is_safe() {
        return interp.set_error(b"not allowed to invoke hidden commands from safe interpreter");
    }
    let cmd = match arguments.bytes(i) {
        Ok(bytes) => bytes,
        Err(error) => return interp.report_cmd_error(error),
    };
    // Build the hidden command's argv (cmd + remaining args).
    let mut hidden_argv: Vec<*mut TclObj> = Vec::with_capacity(words.len() - i);
    for &a in &words[i..] {
        unsafe { obj::incr_ref_count(a) };
        hidden_argv.push(a);
    }
    // Run in the addressed interp (the current one for an empty path), in the
    // requested namespace context if any, copying its result back up the path.
    let outcome = interp.with_child_path(path, |c| {
        let scope = match crate::interp::native_operation_currency::NativeOperationScope::enter(c) {
            Ok(scope) => scope,
            Err(first) => {
                c.refuse_native_execution(first);
                return Err(c.clone());
            }
        };
        let saved_ns = c.current_ns();
        let code = match ns_name.as_ref() {
            Some(name) => c.invoke_hidden_in_namespace(name, &cmd, &hidden_argv),
            None => c.invoke_hidden(&cmd, &hidden_argv),
        };
        let result = scope.currency().ensure_current_or_refuse().and_then(|()| {
            let original = obj::Owned::retain(c.get_obj_result());
            let bytes = c
                .native_object_string_bytes(original.as_ptr())
                .map_err(|error| c.refuse_completion_value_access(error));
            drop(original);
            scope.currency().ensure_current_or_refuse()?;
            bytes
        });
        // Namespace restoration remains mandatory after Guest or Host failure.
        c.set_current_ns(saved_ns);
        if let Err(first) = scope.currency().ensure_current_or_refuse() {
            c.refuse_native_execution(first);
            return Err(c.clone());
        }
        match result {
            Ok(bytes) => Ok((code, bytes)),
            Err(first) => {
                c.refuse_native_execution(first);
                Err(c.clone())
            }
        }
    });
    for a in hidden_argv {
        unsafe { obj::decr_ref_count(a) };
    }
    if let Err(error) = arguments.check() {
        return interp.report_cmd_error(error);
    }
    match outcome {
        Some(Ok((code, result))) => {
            interp.set_result_bytes(&result);
            code
        }
        Some(Err(child)) => interp.transport_host_refusal_from(&child),
        None => not_found_path(interp, path),
    }
}

/// `interp eval path arg ?arg ...?` — evaluate a script in a child interpreter.
fn interp_eval(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let arguments = match OriginalArguments::enter(interp, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    let code = interp_eval_original(interp, argv, &arguments);
    arguments.finish(code)
}

fn interp_eval_original(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    arguments: &OriginalArguments,
) -> Code {
    if argv.len() < 4 {
        return interp.wrong_args(b"interp eval path arg ?arg ...?");
    }
    let path = match interp_path(interp, argv[2]) {
        Ok(path) => path,
        Err(code) => return code,
    };
    let script = if argv.len() == 4 {
        obj::Owned::retain(argv[3])
    } else {
        match interp.concat_original_arguments(&argv[3..]) {
            Ok(script) => obj::Owned::retain(script),
            Err(error) => return interp.report_cmd_error(error),
        }
    };
    if let Err(error) = arguments.check() {
        return interp.report_cmd_error(error);
    }
    let outcome = interp.with_child_path(&path, |target| {
        let scope =
            match crate::interp::native_operation_currency::NativeOperationScope::enter(target) {
                Ok(scope) => scope,
                Err(first) => {
                    target.refuse_native_execution(first);
                    return Err(target.clone());
                }
            };
        let code = target.eval_body_obj(script.as_ptr());
        if let Err(first) = scope.currency().ensure_current_or_refuse() {
            target.refuse_native_execution(first);
            return Err(target.clone());
        }
        // Preserve the actual owned result, without an extra options getter or
        // a String reconstruction that would change this existing contract.
        let result = obj::Owned::retain(target.get_obj_result());
        if let Err(first) = scope.currency().ensure_current_or_refuse() {
            target.refuse_native_execution(first);
            return Err(target.clone());
        }
        Ok((code, result))
    });
    drop(script);
    if let Err(error) = arguments.check() {
        return interp.report_cmd_error(error);
    }
    match outcome {
        Some(Ok((code, result))) => {
            interp.set_result(result.as_ptr());
            drop(result);
            code
        }
        Some(Err(child)) => interp.transport_host_refusal_from(&child),
        None => not_found_original_path(interp, argv[2]),
    }
}

/// `interp delete ?path ...?` — delete each named child interpreter.
fn interp_delete(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    for &a in &argv[2..] {
        let path = match interp_path(interp, a) {
            Ok(path) => path,
            Err(code) => return code,
        };
        let Some((leaf, parent)) = path.split_last() else {
            return not_found_original_path(interp, a);
        };
        let leaf = leaf.clone();
        match interp.with_child_path(parent, |p| p.delete_child(&leaf)) {
            Some(true) => {}
            _ => return not_found_original_path(interp, a),
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `interp alias {} aliasName ?{} target ?arg ...??` — create / query / delete.
fn interp_alias(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let arguments = match OriginalArguments::enter(interp, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    let code = interp_alias_original(interp, argv, &arguments);
    arguments.finish(code)
}

fn interp_alias_original(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    arguments: &OriginalArguments,
) -> Code {
    if argv.len() < 4 {
        return interp.wrong_args(b"interp alias srcPath srcCmd ?targetPath targetCmd? ?arg ...?");
    }
    let source_path = match interp_path(interp, argv[2]) {
        Ok(path) => path,
        Err(code) => return code,
    };
    if source_path.len() > 1 {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "nested Runtime alias source interpreter",
            )
            .into(),
        );
    }
    let child = source_path.first();
    if child.is_some_and(|child| !interp.child_exists(child)) {
        return not_found_original_path(interp, argv[2]);
    }
    if argv.len() >= 5
        && !match interp_path(interp, argv[4]) {
            Ok(path) => path,
            Err(code) => return code,
        }
        .is_empty()
    {
        return only_single_interp(interp);
    }
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
    // C84/85 read the target before AliasCreate captures members. C86+ reach
    // that same original target only after publication, in the loop check.
    let delete = if argv.len() >= 6 && recipe.alias_target_string_precedes_capture() {
        let target = match arguments.bytes(5) {
            Ok(target) => target,
            Err(error) => return interp.report_cmd_error(error),
        };
        if recipe.alias_pretested_target_is_empty(&target) == Some(true) {
            if argv.len() > 6 {
                return interp
                    .wrong_args(b"interp alias srcPath srcCmd ?targetPath targetCmd? ?arg ...?");
            }
            true
        } else {
            false
        }
    } else {
        argv.len() == 5
    };
    // The capture is allocation-only. Its List retains original members, not
    // their bytes, before the C name getter; no prefix getter occurs here.
    let captured = if argv.len() >= 6 && !delete {
        Some((
            std::rc::Rc::new(crate::interp::NativeAliasHeader::new(obj::Owned::retain(
                argv[3],
            ))),
            std::rc::Rc::new(crate::interp::NativeAliasHeader::new(obj::Owned::fresh(
                interp.new_list_object(&argv[5..]),
            ))),
        ))
    } else {
        None
    };
    let original_name = match arguments.bytes(3) {
        Ok(name) => name,
        Err(error) => return interp.report_cmd_error(error),
    };
    let name = match recipe.alias_publication_input(
        tcl_syntax::naming::NativeNameContext::root(),
        &original_name,
    ) {
        Ok(name) => name.selected().to_vec(),
        Err(_) => {
            return interp.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "alias publication input",
                )
                .into(),
            );
        }
    };
    if delete {
        if let Some(child) = child {
            let deleted = interp.with_child(child, |owner| {
                let scope =
                    match crate::interp::native_operation_currency::NativeOperationScope::enter(
                        owner,
                    ) {
                        Ok(scope) => scope,
                        Err(first) => {
                            owner.refuse_native_execution(first);
                            return Err(owner.clone());
                        }
                    };
                if let Err(error) = owner.delete_original_alias_registration(&name) {
                    owner.refuse_completion_value_access(error);
                    return Err(owner.clone());
                }
                match scope.currency().ensure_current_or_refuse() {
                    Ok(()) => Ok(()),
                    Err(first) => {
                        owner.refuse_native_execution(first);
                        Err(owner.clone())
                    }
                }
            });
            if let Err(error) = arguments.check() {
                return interp.report_cmd_error(error);
            }
            if let Some(Err(owner)) = deleted {
                return interp.transport_host_refusal_from(&owner);
            }
        } else {
            if let Err(error) = interp.delete_original_alias_registration(&name) {
                return interp.report_cmd_error(error.into());
            }
            if let Err(error) = arguments.check() {
                return interp.report_cmd_error(error);
            }
        }
        interp.set_result_bytes(b"");
        return Code::Ok;
    }
    if argv.len() == 4 {
        let queried = if let Some(child) = child {
            interp
                .with_child(child, |owner| {
                    let scope =
                        match crate::interp::native_operation_currency::NativeOperationScope::enter(
                            owner,
                        ) {
                            Ok(scope) => scope,
                            Err(first) => {
                                owner.refuse_native_execution(first);
                                return Err(owner.clone());
                            }
                        };
                    let query = owner.alias_query_object(&name);
                    if let Err(first) = scope.currency().ensure_current_or_refuse() {
                        owner.refuse_native_execution(first);
                        return Err(owner.clone());
                    }
                    match query {
                        Ok(query) => Ok(query),
                        Err(error) => {
                            owner.refuse_completion_value_access(error);
                            Err(owner.clone())
                        }
                    }
                })
                .expect("source child was selected")
        } else {
            match interp.alias_query_object(&name) {
                Ok(query) => Ok(query),
                Err(error) => {
                    interp.refuse_completion_value_access(error);
                    Err(interp.clone())
                }
            }
        };
        if let Err(error) = arguments.check() {
            return interp.report_cmd_error(error);
        }
        return match queried {
            Ok(Some(original)) => {
                interp.set_result(original.as_ptr());
                Code::Ok
            }
            Ok(None) => {
                interp.set_result_bytes(b"");
                Code::Ok
            }
            Err(owner) => interp.transport_host_refusal_from(&owner),
        };
    }
    let (registration_name, original_prefix) =
        captured.expect("actual original alias creation operands");
    if let Some(child) = child {
        let mut owner = interp
            .with_child(child, |owner| owner.clone())
            .expect("source child was selected");
        let scope =
            match crate::interp::native_operation_currency::NativeOperationScope::enter(&owner) {
                Ok(scope) => scope,
                Err(first) => {
                    owner.refuse_native_execution(first);
                    return interp.transport_host_refusal_from(&owner);
                }
            };
        let retained_registration = std::rc::Rc::clone(&registration_name);
        let installed = interp.install_parent_alias_objects(
            child,
            &original_name,
            Some(registration_name),
            std::rc::Rc::clone(&original_prefix),
        );
        let generation = owner.original_parent_alias_generation(&name, &original_prefix);
        if let Err(first) = scope.currency().ensure_current_or_refuse() {
            owner.refuse_native_execution(first);
            owner.retire_original_parent_alias(&name, &original_prefix, generation);
            return interp.transport_host_refusal_from(&owner);
        }
        if let Err(error) = arguments.check() {
            owner.retire_original_parent_alias(&name, &original_prefix, generation);
            return interp.report_cmd_error(error);
        }
        match installed {
            Ok(true) => {}
            Ok(false) => return not_found_original_path(interp, argv[2]),
            Err(error) => return interp.report_cmd_error(error.into()),
        }
        if recipe.alias_publication_checks_original_target() {
            // The supported cross-interpreter alias targets its parent. Reach
            // the original target under that parent's genuine operation;
            // never resolve an equal spelling in the child's namespace.
            if let Err(error) = arguments.bytes(5) {
                owner.retire_original_parent_alias(&name, &original_prefix, generation);
                return interp.report_cmd_error(error);
            }
        }
        if let Err(first) = scope.currency().ensure_current_or_refuse() {
            owner.refuse_native_execution(first);
            owner.retire_original_parent_alias(&name, &original_prefix, generation);
            return interp.transport_host_refusal_from(&owner);
        }
        let token = match owner.complete_original_alias_registration(
            &original_name,
            tcl_syntax::naming::NativeNamePurpose::ChildAliasPublication,
            &retained_registration,
            generation,
        ) {
            Ok(token) => token,
            Err(error) => {
                owner.report_cmd_error(error);
                return interp.transport_host_refusal_from(&owner);
            }
        };
        if let Err(error) = arguments.check() {
            return interp.report_cmd_error(error);
        }
        if let Some(token) = token {
            interp.set_result(token.as_ptr());
        }
    } else {
        let installed =
            interp.install_alias_objects(&original_name, registration_name, original_prefix);
        if let Err(error) = arguments.check() {
            return interp.report_cmd_error(error);
        }
        match installed {
            Ok(None) => {}
            Ok(Some(simple)) => return alias_loop_error(interp, &simple),
            Err(error) => return interp.report_cmd_error(error),
        }
    }
    if recipe == tcl_syntax::naming::NativeNameProtocol::C(tcl_dialect::TclVersion::V8_4) {
        interp.set_result(argv[3]);
    }
    Code::Ok
}

/// The actual child-handle alias operation. C's target pretest precedes
/// capture on every supported release; Jim captures its parent List before
/// the name and keeps the original target/prefix Strings lazy.
pub(crate) fn child_alias_handle_original(
    interp: &mut Interp,
    child: &[u8],
    argv: &[*mut TclObj],
    arguments: &OriginalArguments,
) -> Code {
    use crate::interp::native_operation_currency::NativeOperationScope;
    use std::rc::Rc;
    use tcl_syntax::value::ValueError;
    let Some(recipe) = interp
        .name_policy_protocol()
        .map(|protocol| protocol.recipe())
    else {
        return interp.report_cmd_error(
            ValueError::CommandProtocolUnavailable("original child alias recipe").into(),
        );
    };
    let usage = b"alias aliasName ?targetName? ?arg ...?";
    if argv.len() < 3 || (!recipe.child_alias_target_string_precedes_capture() && argv.len() < 4) {
        return interp.wrong_args_for_invocation(argv, usage);
    }
    let delete = if argv.len() >= 4 && recipe.child_alias_target_string_precedes_capture() {
        let target = match arguments.bytes(3) {
            Ok(target) => target,
            Err(error) => return interp.report_cmd_error(error),
        };
        if recipe.child_alias_pretested_target_is_empty(&target) == Some(true) {
            if argv.len() != 4 {
                return interp.wrong_args_for_invocation(argv, usage);
            }
            true
        } else {
            false
        }
    } else {
        false
    };
    let captured = if argv.len() >= 4 && !delete {
        let prefix = if recipe.child_alias_target_string_precedes_capture() {
            Rc::new(crate::interp::NativeAliasHeader::new(obj::Owned::fresh(
                interp.new_list_object(&argv[3..]),
            )))
        } else {
            match interp.original_jim_parent_alias_prefix(&argv[3..]) {
                Ok(prefix) => prefix,
                Err(error) => return interp.report_cmd_error(error.into()),
            }
        };
        let registration_name = recipe
            .alias_record_retains_original_registration_name()
            .then(|| {
                Rc::new(crate::interp::NativeAliasHeader::new(obj::Owned::retain(
                    argv[2],
                )))
            });
        if let Err(error) = arguments.check() {
            return interp.report_cmd_error(error);
        }
        Some((registration_name, prefix))
    } else {
        None
    };
    let name = match arguments.bytes(2) {
        Ok(name) => name,
        Err(error) => return interp.report_cmd_error(error),
    };
    let Some(mut owner) = interp.with_child(child, |owner| owner.clone()) else {
        return interp.error(b"could not find interpreter");
    };
    let scope = match NativeOperationScope::enter(&owner) {
        Ok(scope) => scope,
        Err(first) => {
            owner.refuse_native_execution(first);
            return interp.transport_host_refusal_from(&owner);
        }
    };
    if delete {
        if let Err(error) = owner.delete_original_alias_registration(&name) {
            owner.refuse_completion_value_access(error);
            return interp.transport_host_refusal_from(&owner);
        }
        if let Err(first) = scope.currency().ensure_current_or_refuse() {
            owner.refuse_native_execution(first);
        }
        if let Err(error) = arguments.check() {
            return interp.report_cmd_error(error);
        }
        if owner.host_refusal_pending() {
            return interp.transport_host_refusal_from(&owner);
        }
        drop(scope);
        interp.set_result_bytes(b"");
        return Code::Ok;
    }
    if argv.len() == 3 {
        let query = owner.alias_query_object(&name);
        if let Err(first) = scope.currency().ensure_current_or_refuse() {
            owner.refuse_native_execution(first);
        }
        if let Err(error) = arguments.check() {
            return interp.report_cmd_error(error);
        }
        if owner.host_refusal_pending() {
            return interp.transport_host_refusal_from(&owner);
        }
        drop(scope);
        return match query {
            Ok(Some(query)) => {
                interp.set_result(query.as_ptr());
                Code::Ok
            }
            Ok(None) if recipe.is_jim084() => {
                let mut message = b"alias \"".to_vec();
                message.extend_from_slice(&name);
                message.extend_from_slice(b"\" not found");
                interp.set_error(&message)
            }
            Ok(None) => {
                interp.set_result_bytes(b"");
                Code::Ok
            }
            Err(error) => {
                owner.refuse_completion_value_access(error);
                interp.transport_host_refusal_from(&owner)
            }
        };
    }
    let (registration_name, prefix) = captured.expect("actual child alias capture");
    let retained_registration = registration_name.as_ref().map(Rc::clone);
    let installed =
        interp.install_parent_alias_objects(child, &name, registration_name, Rc::clone(&prefix));
    let generation = owner.original_parent_alias_generation(&name, &prefix);
    if let Err(first) = scope.currency().ensure_current_or_refuse() {
        owner.refuse_native_execution(first);
    }
    if let Err(error) = arguments.check() {
        owner.retire_original_parent_alias(&name, &prefix, generation);
        return interp.report_cmd_error(error);
    }
    if owner.host_refusal_pending() {
        owner.retire_original_parent_alias(&name, &prefix, generation);
        return interp.transport_host_refusal_from(&owner);
    }
    match installed {
        Ok(true) => {}
        Ok(false) => return interp.error(b"could not find interpreter"),
        Err(error) => return interp.report_cmd_error(error.into()),
    }
    if recipe.alias_publication_checks_original_target() {
        if let Err(error) = arguments.bytes(3) {
            owner.retire_original_parent_alias(&name, &prefix, generation);
            return interp.report_cmd_error(error);
        }
    }
    if let Err(first) = scope.currency().ensure_current_or_refuse() {
        owner.refuse_native_execution(first);
        owner.retire_original_parent_alias(&name, &prefix, generation);
        return interp.transport_host_refusal_from(&owner);
    }
    if let Err(error) = arguments.check() {
        owner.retire_original_parent_alias(&name, &prefix, generation);
        return interp.report_cmd_error(error);
    }
    let mut token = None;
    if let Some(registration) = retained_registration {
        token = match owner.complete_original_alias_registration(
            &name,
            tcl_syntax::naming::NativeNamePurpose::ChildAliasPublication,
            &registration,
            generation,
        ) {
            Ok(token) => token,
            Err(error) => {
                owner.report_cmd_error(error);
                return interp.transport_host_refusal_from(&owner);
            }
        };
        if let Err(error) = arguments.check() {
            return interp.report_cmd_error(error);
        }
    }
    drop(scope);
    if recipe.child_alias_returns_original_registration_name() {
        if recipe == tcl_syntax::naming::NativeNameProtocol::C(tcl_dialect::TclVersion::V8_4) {
            interp.set_result(argv[2]);
        } else if let Some(token) = token {
            interp.set_result(token.as_ptr());
        }
    } else {
        interp.set_result_bytes(b"");
    }
    Code::Ok
}

/// `interp aliases ?path?` — every alias command's name in the named interp (the
/// current one for an empty/missing path) as a Tcl list.
fn interp_aliases(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() > 3 {
        return interp.wrong_args(b"interp aliases ?path?");
    }
    let path = match argv.get(2) {
        None => Vec::new(),
        Some(&original) => match interp_path(interp, original) {
            Ok(path) => path,
            Err(code) => return code,
        },
    };
    let names = match interp.with_child_path(&path, |owner| owner.alias_names()) {
        Some(Ok(names)) => names,
        Some(Err(error)) => return interp.report_cmd_error(error.into()),
        None => return not_found_original_path(interp, argv[2]),
    };
    let elems = names.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
    interp.set_result(interp.new_list_object(&elems));
    Code::Ok
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
pub(crate) mod original_operand_tests {
    use super::*;
    use crate::obj::Owned;
    use std::cell::{Cell, RefCell};
    use tcl_runtime_api::NativeExecutionError;
    #[cfg(debug_assertions)]
    use tcl_syntax::raw_string::NativeValueAccessRefusal;

    #[derive(Clone, Copy)]
    #[cfg(debug_assertions)]
    pub(crate) enum Update {
        Host,
        Restore,
        Normal,
    }
    thread_local! {
        static INTERPRETER: RefCell<Option<Interp>> = const { RefCell::new(None) };
        static FIRST_CAUSE: RefCell<Option<NativeExecutionError>> = const { RefCell::new(None) };
        #[cfg(debug_assertions)]
        static MODE: Cell<Update> = const { Cell::new(Update::Host) };
        static FIRST: Cell<usize> = const { Cell::new(0) };
        static LATER: Cell<usize> = const { Cell::new(0) };
    }
    #[cfg(debug_assertions)]
    pub(crate) struct UpdaterOwner;
    #[cfg(debug_assertions)]
    impl UpdaterOwner {
        #[cfg(debug_assertions)]
        pub(crate) fn enter(interp: &Interp, mode: Update) -> Self {
            INTERPRETER.with(|slot| *slot.borrow_mut() = Some(interp.clone()));
            MODE.with(|selected| selected.set(mode));
            FIRST.with(|count| count.set(0));
            FIRST_CAUSE.with(|cause| cause.borrow_mut().take());
            LATER.with(|count| count.set(0));
            Self
        }
    }
    #[cfg(debug_assertions)]
    impl Drop for UpdaterOwner {
        fn drop(&mut self) {
            INTERPRETER.with(|slot| slot.borrow_mut().take());
        }
    }
    pub(crate) fn native(version: &str) -> Interp {
        Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(version),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }
    #[cfg(debug_assertions)]
    fn restore(interp: &mut Interp) {
        let original = interp.runtime_context();
        let mut changed = original.clone();
        changed.packages = vec![("alias-trace-original-stage".to_owned(), "1.0".to_owned())];
        interp.pin_context(&changed).unwrap();
        interp.pin_context(&original).unwrap();
    }
    #[cfg(debug_assertions)]
    extern "C" fn first_getter(original: *mut TclObj) {
        FIRST.with(|count| count.set(count.get() + 1));
        let mut interp = INTERPRETER.with(|slot| slot.borrow().as_ref().unwrap().clone());
        let reached = Owned::fresh(obj::new_string_bytes(b"REACHED"));
        interp
            .var_set(b"original_argument_effect", reached.as_ptr())
            .unwrap();
        match MODE.with(Cell::get) {
            Update::Host => {
                interp.refuse_host_command("first alias/trace original getter");
                FIRST_CAUSE.with(|cause| *cause.borrow_mut() = interp.native_execution_refusal());
            }
            Update::Restore => restore(&mut interp),
            Update::Normal => {}
        }
        unsafe { obj::set_native_updater_string_rep(original, b"reached\0\xff", false) };
    }
    #[cfg(debug_assertions)]
    extern "C" fn later_getter(original: *mut TclObj) {
        LATER.with(|count| count.set(count.get() + 1));
        unsafe { obj::set_native_updater_string_rep(original, b"LATER", false) };
    }
    #[cfg(debug_assertions)]
    static FIRST_TYPE: obj::TclObjType = obj::TclObjType {
        name: c"originalAliasTraceGetter".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: Some(first_getter),
        set_from_any_proc: None,
    };
    #[cfg(debug_assertions)]
    static LATER_TYPE: obj::TclObjType = obj::TclObjType {
        name: c"originalAliasTraceLaterGetter".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: Some(later_getter),
        set_from_any_proc: None,
    };
    #[cfg(debug_assertions)]
    pub(crate) fn first() -> Owned {
        Owned::fresh(obj::alloc_typed(&FIRST_TYPE, 0))
    }
    #[cfg(debug_assertions)]
    fn later() -> Owned {
        Owned::fresh(obj::alloc_typed(&LATER_TYPE, 0))
    }
    pub(crate) fn words(values: &[&[u8]]) -> Vec<Owned> {
        values
            .iter()
            .map(|bytes| Owned::fresh(obj::new_string_bytes(bytes)))
            .collect()
    }
    #[cfg(debug_assertions)]
    fn expected(mode: Update) -> NativeExecutionError {
        match mode {
            Update::Host => FIRST_CAUSE.with(|cause| {
                cause
                    .borrow()
                    .as_ref()
                    .expect("actual updater first cause")
                    .clone()
            }),
            Update::Restore => NativeExecutionError::ValueAccessRefusal(
                NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "stale entered native operation",
                ),
            ),
            Update::Normal => unreachable!(),
        }
    }
    /// Inspect the reached storage without starting another native name operation.
    /// A pending first Host intentionally makes `var_get` terminal.
    #[cfg(debug_assertions)]
    pub(crate) fn reached_variable(interp: &Interp, name: &[u8]) -> *mut TclObj {
        crate::vars::get(
            &interp.frames.borrow(),
            &interp.namespaces(),
            interp.current_ns(),
            name,
        )
        .expect("actual earlier callback variable effect remains")
    }
    #[cfg(debug_assertions)]
    pub(crate) fn seed(interp: &mut Interp) -> Owned {
        interp.set_result_bytes(b"PRIOR\0\xff");
        interp.set_return_state(2, Code::Other(7));
        interp.set_return_options(vec![(b"-original".to_vec(), b"OPTIONS".to_vec())]);
        Owned::retain(interp.get_obj_result())
    }
    #[cfg(debug_assertions)]
    pub(crate) fn unchanged(interp: &Interp, result: &Owned, mode: Update) {
        assert_eq!(interp.native_execution_refusal(), Some(expected(mode)));
        assert_eq!(interp.get_obj_result(), result.as_ptr());
        assert_eq!(obj::bytes_of(result.as_ptr()), b"PRIOR\0\xff");
        assert_eq!(interp.pending_return_level(), 2);
        assert_eq!(interp.pending_return_code(), Code::Other(7));
        assert_eq!(interp.pending_return_option_objects().len(), 1);
        assert_eq!(FIRST.with(Cell::get), 1);
        assert_eq!(LATER.with(Cell::get), 0);
        assert_eq!(interp.entered_native_operation_count(), 0);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn original_alias_reads_stop_before_later_prefix_and_publication() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Native415 observes no prefix String read at definition. Only name and
        // target are reached here; target failure retains post-publication effects.
        // Prefix refusal is exercised separately at actual invocation below.
        for mode in [Update::Host, Update::Restore] {
            for child in [false, true] {
                for failed_position in [3, 5] {
                    let mut interp = native("tcl8.6");
                    if child {
                        assert_eq!(interp.eval_str(b"interp create child"), Code::Ok);
                    }
                    let _updater = UpdaterOwner::enter(&interp, mode);
                    let failed = first();
                    let unread = later();
                    let owned = words(&[
                        b"interp",
                        b"alias",
                        if child { b"child" } else { b"" },
                        b"original_alias",
                        b"",
                        b"list",
                    ]);
                    let mut argv: Vec<_> = owned.iter().map(Owned::as_ptr).collect();
                    argv[failed_position] = failed.as_ptr();
                    argv.push(unread.as_ptr());
                    let count = unsafe { (*failed.as_ptr()).ref_count };
                    let before = interp.namespaces().native_command_generations();
                    let child_before = child.then(|| {
                        interp
                            .with_child(b"child", |owner| {
                                owner.namespaces().native_command_generations()
                            })
                            .unwrap()
                    });
                    let prior = seed(&mut interp);
                    assert_eq!(super::interp_cmd(&mut interp, &argv), Code::Error);
                    unchanged(&interp, &prior, mode);
                    assert_eq!(unsafe { (*failed.as_ptr()).ref_count }, count);
                    assert_eq!(obj::bytes_of(failed.as_ptr()), b"reached\0\xff");
                    assert!(!obj::has_string_rep(unread.as_ptr()));
                    let after = interp.namespaces().native_command_generations();
                    if child || failed_position == 3 {
                        assert_eq!(after, before);
                    } else {
                        assert_ne!(after, before, "target getter follows reached publication");
                    }
                    if let Some(child_before) = child_before {
                        let child_after = interp
                            .with_child(b"child", |owner| {
                                owner.namespaces().native_command_generations()
                            })
                            .unwrap();
                        if failed_position == 3 {
                            assert_eq!(child_after, child_before);
                        } else {
                            assert_ne!(child_after, child_before);
                        }
                    }
                    if child {
                        interp
                            .with_child(b"child", |owner| {
                                assert!(owner.resolve_cmd_token(b"original_alias").is_none())
                            })
                            .unwrap();
                    } else {
                        assert!(interp.resolve_cmd_token(b"original_alias").is_none());
                    }
                }
            }
        }
    }

    thread_local! {
        static INVOCATION_PRIOR: RefCell<Option<Owned>> = const { RefCell::new(None) };
    }
    #[cfg(debug_assertions)]
    unsafe extern "C" fn consume_original_prefix(
        _: *mut std::ffi::c_void,
        interp: *mut Interp,
        count: std::ffi::c_int,
        argv: *const *mut TclObj,
    ) -> std::ffi::c_int {
        // SAFETY: actual registered command receives the live interp and argv.
        let interp = unsafe { &mut *interp };
        let argv = unsafe { std::slice::from_raw_parts(argv, usize::try_from(count).unwrap()) };
        let prior = seed(interp);
        INVOCATION_PRIOR.with(|slot| *slot.borrow_mut() = Some(prior));
        let arguments = match OriginalArguments::enter(interp, argv) {
            Ok(arguments) => arguments,
            Err(error) => return interp.report_cmd_error(error).as_int() as i32,
        };
        for index in [1, 2] {
            if let Err(error) = arguments.bytes(index) {
                return arguments.finish(interp.report_cmd_error(error)).as_int() as i32;
            }
        }
        arguments.finish(Code::Ok).as_int() as i32
    }

    #[cfg(debug_assertions)]
    #[test]
    fn original_alias_prefix_refusal_is_reached_only_at_actual_invocation() {
        // naming.alias.original-object-prefix-capture-query-invocation-lifetime
        // docs/design/analysis/name-resolution-proofs/alias-original-object-prefix-capture-query-invocation-lifetime.md
        for version in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for mode in [Update::Host, Update::Restore] {
                let mut interp = native(version);
                interp
                    .create_obj_command(
                        b"consume_original_prefix",
                        crate::interp::ObjCommand::new(
                            consume_original_prefix,
                            std::ptr::null_mut(),
                            None,
                        ),
                    )
                    .expect("actual original command record");
                let _updater = UpdaterOwner::enter(&interp, mode);
                let failed = first();
                let unread = later();
                let owned = words(&[
                    b"interp",
                    b"alias",
                    b"",
                    b"original_alias",
                    b"",
                    b"consume_original_prefix",
                ]);
                let mut argv = owned.iter().map(Owned::as_ptr).collect::<Vec<_>>();
                argv.extend([failed.as_ptr(), unread.as_ptr()]);
                assert_eq!(super::interp_cmd(&mut interp, &argv), Code::Ok);
                assert_eq!(FIRST.with(Cell::get), 0);
                assert_eq!(LATER.with(Cell::get), 0);
                assert!(!obj::has_string_rep(failed.as_ptr()));
                assert!(!obj::has_string_rep(unread.as_ptr()));
                let query = interp
                    .alias_query_object(b"original_alias")
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    ValueOps::list_elements(&mut interp, &query.as_ptr()).unwrap(),
                    vec![owned[5].as_ptr(), failed.as_ptr(), unread.as_ptr()]
                );
                assert_eq!(
                    FIRST.with(Cell::get),
                    0,
                    "query retains the same unread prefix"
                );
                let count = unsafe { (*failed.as_ptr()).ref_count };
                let before = interp.namespaces().native_command_generations();
                assert_eq!(interp.dispatch(&[owned[3].as_ptr()]), Code::Error);
                let prior = INVOCATION_PRIOR.with(|slot| slot.borrow_mut().take()).unwrap_or_else(|| {
                    let mode = match mode {
                        Update::Host => "Host",
                        Update::Restore => "Restore",
                        Update::Normal => "Normal",
                    };
                    let result = interp.get_obj_result();
                    let resident_result = obj::has_string_rep(result).then(|| obj::bytes_of(result));
                    panic!(
                        "actual alias target callback was not reached: version={version}, mode={mode}, first={:?}, resident_result={resident_result:?}, first_getters={}, later_getters={}",
                        interp.native_execution_refusal(), FIRST.with(Cell::get), LATER.with(Cell::get),
                    );
                });
                unchanged(&interp, &prior, mode);
                assert_eq!(interp.namespaces().native_command_generations(), before);
                assert_eq!(unsafe { (*failed.as_ptr()).ref_count }, count);
                assert_eq!(obj::bytes_of(failed.as_ptr()), b"reached\0\xff");
                assert!(!obj::has_string_rep(unread.as_ptr()));
                assert!(interp.var_get(b"original_argument_effect").is_none());
                assert_eq!(
                    obj::bytes_of(reached_variable(&interp, b"original_argument_effect")),
                    b"REACHED"
                );
            }
        }
    }

    #[cfg(debug_assertions)]
    #[test]
    fn original_interpreter_options_and_names_keep_actual_read_order_and_refusal() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let cases: &[(&[&[u8]], usize)] = &[
            (&[b"interp", b"create", b"flag", b"LATER"], 2),
            (&[b"interp", b"target", b"", b"name"], 3),
            (&[b"interp", b"hide", b"", b"name", b"LATER"], 3),
            (&[b"interp", b"hide", b"", b"puts", b"token"], 4),
            (&[b"interp", b"invokehidden", b"", b"option", b"LATER"], 3),
            (
                &[
                    b"interp",
                    b"invokehidden",
                    b"",
                    b"-namespace",
                    b"ns",
                    b"LATER",
                ],
                4,
            ),
            (
                &[b"interp", b"invokehidden", b"", b"--", b"head", b"LATER"],
                4,
            ),
        ];
        for mode in [Update::Host, Update::Restore] {
            for &(texts, selected) in cases {
                let mut interp = native("tcl8.6");
                let _updater = UpdaterOwner::enter(&interp, mode);
                let owned = words(texts);
                let failed = first();
                let unread = later();
                let mut argv: Vec<_> = owned.iter().map(Owned::as_ptr).collect();
                argv[selected] = failed.as_ptr();
                if selected + 1 < argv.len() {
                    argv[selected + 1] = unread.as_ptr();
                }
                let before = interp.namespaces().native_command_generations();
                let prior = seed(&mut interp);
                assert_eq!(
                    super::interp_cmd(&mut interp, &argv),
                    Code::Error,
                    "{texts:?}"
                );
                unchanged(&interp, &prior, mode);
                assert!(!obj::has_string_rep(unread.as_ptr()));
                assert_eq!(interp.namespaces().native_command_generations(), before);
                assert!(interp.child_names().is_empty());
            }
        }
    }

    #[cfg(debug_assertions)]
    #[test]
    fn original_interpreter_path_reads_selected_list_and_original_elements() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let mut interp = native("tcl8.6");
        let child = Owned::fresh(obj::new_string_bytes(b"child\0ignored"));
        let path = Owned::fresh(interp.new_list_object(&[child.as_ptr()]));
        assert!(!obj::has_string_rep(path.as_ptr()));
        assert_eq!(
            super::interp_path(&mut interp, path.as_ptr()).unwrap(),
            vec![b"child".to_vec()]
        );
        assert!(!obj::has_string_rep(path.as_ptr()));
        assert_eq!(obj::bytes_of(child.as_ptr()), b"child\0ignored");
        let _updater = UpdaterOwner::enter(&interp, Update::Restore);
        let failed = first();
        let unread = later();
        let path = Owned::fresh(interp.new_list_object(&[failed.as_ptr(), unread.as_ptr()]));
        let prior = seed(&mut interp);
        assert_eq!(
            super::interp_path(&mut interp, path.as_ptr()),
            Err(Code::Error)
        );
        unchanged(&interp, &prior, Update::Restore);
        assert!(!obj::has_string_rep(path.as_ptr()));
        assert!(!obj::has_string_rep(unread.as_ptr()));
    }

    #[cfg(debug_assertions)]
    fn original_child_worker(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
        match MODE.with(Cell::get) {
            Update::Host => {
                interp.refuse_host_command("first alias/trace child worker");
            }
            Update::Restore => restore(interp),
            Update::Normal => {}
        }
        let result = Owned::fresh(obj::new_string_bytes(b"CHILD\0\xff"));
        interp.set_result(result.as_ptr());
        Code::Other(7)
    }
    #[cfg(debug_assertions)]
    fn original_child_lazy_result(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
        INTERPRETER.with(|slot| *slot.borrow_mut() = Some(interp.clone()));
        let result = first();
        interp.set_result(result.as_ptr());
        Code::Other(7)
    }

    #[cfg(debug_assertions)]
    #[test]
    fn original_hidden_child_result_getter_preserves_namespace_cleanup_and_first_cause() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Actual child lazy result getter and mandatory cleanup, not a Native provider assertion.
        for mode in [Update::Host, Update::Restore] {
            let mut interp = native("tcl8.6");
            assert_eq!(interp.eval_str(b"interp create child"), Code::Ok);
            interp
                .with_child(b"child", |child| {
                    child.register_builtin(b"original_lazy", original_child_lazy_result);
                    assert!(matches!(
                        child.hide_command(b"original_lazy", b"original_lazy"),
                        crate::interp::CommandVisibilityOutcome::Moved
                    ));
                })
                .unwrap();
            let _updater = UpdaterOwner::enter(&interp, mode);
            let owned = words(&[
                b"interp",
                b"invokehidden",
                b"child",
                b"-namespace",
                b"::child_work",
                b"original_lazy",
            ]);
            let argv: Vec<_> = owned.iter().map(Owned::as_ptr).collect();
            let prior = seed(&mut interp);
            assert_eq!(super::interp_cmd(&mut interp, &argv), Code::Error);
            assert_eq!(interp.get_obj_result(), prior.as_ptr());
            assert_eq!(FIRST.with(Cell::get), 1);
            let cause = interp.native_execution_refusal().unwrap();
            assert_eq!(cause, expected(mode));
            interp
                .with_child(b"child", |child| {
                    assert_eq!(child.native_execution_refusal(), Some(cause));
                    assert_eq!(child.current_ns(), crate::namespace::GLOBAL);
                    assert_eq!(obj::bytes_of(child.get_obj_result()), b"reached\0\xff");
                    assert_eq!(child.entered_native_operation_count(), 0);
                })
                .unwrap();
            assert_eq!(interp.entered_native_operation_count(), 0);
        }
    }

    #[cfg(debug_assertions)]
    #[test]
    fn original_child_alias_adapters_keep_guest_code_result_and_typed_refusal() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        for hidden in [false, true] {
            for mode in [Update::Normal, Update::Host, Update::Restore] {
                let mut interp = native("tcl8.6");
                assert_eq!(interp.eval_str(b"interp create child"), Code::Ok);
                interp
                    .with_child(b"child", |child| {
                        child.register_builtin(b"original_worker", original_child_worker);
                        if hidden {
                            child.hide_command(b"original_worker", b"original_worker");
                        }
                    })
                    .unwrap();
                let _updater = UpdaterOwner::enter(&interp, mode);
                let owned = words(&[
                    b"interp",
                    if hidden { b"invokehidden" } else { b"eval" },
                    b"child",
                    b"original_worker",
                ]);
                let argv: Vec<_> = owned.iter().map(Owned::as_ptr).collect();
                let prior = seed(&mut interp);
                assert_eq!(
                    super::interp_cmd(&mut interp, &argv),
                    if matches!(mode, Update::Normal) {
                        Code::Other(7)
                    } else {
                        Code::Error
                    }
                );
                if matches!(mode, Update::Normal) {
                    assert!(interp.native_execution_refusal().is_none());
                    assert_eq!(obj::bytes_of(interp.get_obj_result()), b"CHILD\0\xff");
                } else {
                    assert_eq!(interp.get_obj_result(), prior.as_ptr());
                    let cause = interp.native_execution_refusal().unwrap();
                    let child_cause = interp
                        .with_child(b"child", |child| child.native_execution_refusal())
                        .unwrap();
                    assert_eq!(child_cause, Some(cause));
                }
                interp
                    .with_child(b"child", |child| {
                        assert_eq!(child.current_ns(), crate::namespace::GLOBAL);
                        assert_eq!(child.entered_native_operation_count(), 0);
                    })
                    .unwrap();
                assert_eq!(interp.entered_native_operation_count(), 0);
            }
        }
    }

    #[cfg(debug_assertions)]
    #[test]
    fn original_child_handle_alias_reads_the_selected_operand_before_publication() {
        // naming.alias.original-object-prefix-capture-query-invocation-lifetime
        // docs/design/analysis/name-resolution-proofs/alias-original-object-prefix-capture-query-invocation-lifetime.md
        // This actual worker control distinguishes the C target pretest from
        // Jim's List-first capture. It does not equate child and core name roles.
        for version in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            for mode in [Update::Host, Update::Restore] {
                let mut interp = native(version);
                let jim = version == "jim";
                let child = if jim {
                    interp.create_jim_child()
                } else {
                    interp.create_child(Some(b"child".to_vec()))
                };
                let _updater = UpdaterOwner::enter(&interp, mode);
                let failed = first();
                let unread = later();
                let owned = words(&[&child, b"alias", b"original_child_alias", b"list"]);
                let mut argv = owned.iter().map(Owned::as_ptr).collect::<Vec<_>>();
                argv[if jim { 2 } else { 3 }] = failed.as_ptr();
                argv.push(unread.as_ptr());
                let count = unsafe { (*failed.as_ptr()).ref_count };
                let before = interp
                    .with_child(&child, |owner| {
                        owner.namespaces().native_command_generations()
                    })
                    .unwrap();
                let prior = seed(&mut interp);
                let code = {
                    let arguments = OriginalArguments::enter(&interp, &argv).unwrap();
                    let code =
                        super::child_alias_handle_original(&mut interp, &child, &argv, &arguments);
                    arguments.finish(code)
                };
                assert_eq!(code, Code::Error);
                unchanged(&interp, &prior, mode);
                assert_eq!(unsafe { (*failed.as_ptr()).ref_count }, count);
                assert_eq!(obj::bytes_of(failed.as_ptr()), b"reached\0\xff");
                assert!(!obj::has_string_rep(unread.as_ptr()));
                interp
                    .with_child(&child, |owner| {
                        assert_eq!(owner.namespaces().native_command_generations(), before);
                        assert!(owner.resolve_cmd_token(b"original_child_alias").is_none());
                        assert!(owner.native_execution_refusal().is_none());
                    })
                    .unwrap();
                assert!(interp.var_get(b"original_argument_effect").is_none());
                assert_eq!(
                    obj::bytes_of(reached_variable(&interp, b"original_argument_effect")),
                    b"REACHED",
                );
            }
        }
    }
}

#[cfg(test)]
mod original_rename_operation_tests;

#[cfg(test)]
mod original_alias_storage_tests;

#[cfg(test)]
mod original_alias_key_tests;

#[cfg(test)]
mod native_positive_option_prefixes;
