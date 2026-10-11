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
    interp.register_builtin(b"rename", rename);
    interp.register_builtin(b"interp", interp_cmd);
    if interp
        .native_invocation_dialect()
        .native_jim_lookup_protocol()
        .is_some()
    {
        interp.register_builtin(b"alias", jim_alias);
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
    let original_prefix = std::rc::Rc::new(obj::Owned::fresh(interp.new_list_object(&argv[2..])));
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
    let registration_name = std::rc::Rc::new(obj::Owned::retain(argv[1]));
    let code = match arguments.bytes(1) {
        Ok(name) => match interp.install_alias_objects(&name, registration_name, original_prefix) {
            Ok(None) => Code::Ok,
            Ok(Some(simple)) => alias_loop_error(interp, &simple),
            Err(error) => interp.report_cmd_error(error),
        },
        Err(error) => interp.report_cmd_error(error),
    };
    arguments.finish(code)
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
        RenameOutcome::NoSuchCommand => {
            // TclRenameCommand chooses the verb from the requested operation:
            // an empty destination is a deletion, while a non-empty
            // destination is a rename (tclNamesp.c).
            let verb = if new.is_empty() {
                b"can't delete \"".as_slice()
            } else {
                b"can't rename \"".as_slice()
            };
            let mut m = verb.to_vec();
            m.extend_from_slice(&old);
            m.extend_from_slice(b"\": command doesn't exist");
            let code = crate::interp::error_code_list(&[b"TCL", b"LOOKUP", b"COMMAND", &old]);
            interp.error_with_code(&m, &code)
        }
        RenameOutcome::AliasLoop => match interp.original_rename_alias_loop_name(&old, &new) {
            Ok(reported) => alias_loop_error(interp, &reported),
            Err(error) => interp.report_cmd_error(error.into()),
        },
        RenameOutcome::TargetExists => {
            let mut m = b"can't rename to \"".to_vec();
            m.extend_from_slice(&new);
            m.extend_from_slice(b"\": command already exists");
            interp.error_with_code(&m, b"TCL OPERATION RENAME TARGET_EXISTS")
        }
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
                match arguments.bytes(3) {
                    Ok(bytes) => Some(bytes),
                    Err(error) => return interp.report_cmd_error(error),
                }
            } else {
                None
            };
            match interp.with_child_path(&path, |c| c.recursion_limit_apply(newlimit.as_deref())) {
                Some(Ok(n)) => {
                    interp.set_result_bytes(n.to_string().as_bytes());
                    Code::Ok
                }
                Some(Err(m)) => interp.set_error(&m),
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
        Some(target_path) => {
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
        None => {
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

/// `interp invokehidden`'s leading option words (`hiddenOptions[]`,
/// `tclInterp.c`), resolved the same way: `-g`/`-n` abbreviate and the lone `-`
/// is `ambiguous option "-"`. Only a word starting with `-` reaches the table.
const HIDDEN_OPTIONS: tcl_cmd_core::prefix::OptionTable<'static, &[u8]> =
    tcl_cmd_core::prefix::OptionTable::abbreviating("option", &[b"-global", b"-namespace", b"--"]);

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
    let Some(original) = name_obj else {
        let created = interp.create_child(None);
        if interp.host_refusal_pending() {
            return Code::Error;
        }
        if safe {
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
        if safe {
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
        return match op {
            CommandVisibilityOp::Hide => interp.error_with_code(
                b"cannot use namespace qualifiers in hidden command token (rename)",
                b"TCL VALUE HIDDENTOKEN",
            ),
            CommandVisibilityOp::Expose => interp.error_with_code(
                b"cannot expose to a namespace (use expose to toplevel, then rename)",
                b"TCL EXPOSE NON_GLOBAL",
            ),
        };
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
            interp.native_static_option_index(words[i], HIDDEN_OPTIONS.names(), false, "option");
        if let Err(error) = arguments.check() {
            return interp.report_cmd_error(error);
        }
        match selected {
            Ok(0) => {
                ns_name = Some(b"::".to_vec());
                i += 1;
            }
            Ok(1) => {
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
            Ok(_) => {
                i += 1;
                break;
            }
            Err(m) => return interp.report_cmd_error(m),
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
        if let Some(name) = &ns_name {
            let target = c.ensure_global_namespace(name);
            c.set_current_ns(target);
        }
        let code = c.invoke_hidden(&cmd, &hidden_argv);
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
        match tcl_cmd_core::list::concat_selected(interp, &argv[3..]) {
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
    if argv.len() >= 5 {
        if !match interp_path(interp, argv[4]) {
            Ok(path) => path,
            Err(code) => return code,
        }
        .is_empty()
        {
            return only_single_interp(interp);
        }
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
            std::rc::Rc::new(obj::Owned::retain(argv[3])),
            std::rc::Rc::new(obj::Owned::fresh(interp.new_list_object(&argv[5..]))),
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
                owner.delete_command(&name);
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
            interp.delete_command(&name);
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
                let mut message = b"alias \"".to_vec();
                message.extend_from_slice(&name);
                message.extend_from_slice(b"\" not found");
                interp.set_error(&message)
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
        let installed = interp.install_parent_alias_objects(
            child,
            &original_name,
            Some(registration_name),
            std::rc::Rc::clone(&original_prefix),
        );
        if let Err(first) = scope.currency().ensure_current_or_refuse() {
            owner.refuse_native_execution(first);
            owner.retire_original_parent_alias(&name, &original_prefix);
            return interp.transport_host_refusal_from(&owner);
        }
        if let Err(error) = arguments.check() {
            owner.retire_original_parent_alias(&name, &original_prefix);
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
                owner.retire_original_parent_alias(&name, &original_prefix);
                return interp.report_cmd_error(error);
            }
        }
        if let Err(first) = scope.currency().ensure_current_or_refuse() {
            owner.refuse_native_execution(first);
            owner.retire_original_parent_alias(&name, &original_prefix);
            return interp.transport_host_refusal_from(&owner);
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
    interp.set_result(argv[3]);
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
            Rc::new(obj::Owned::fresh(interp.new_list_object(&argv[3..])))
        } else {
            match interp.original_jim_parent_alias_prefix(&argv[3..]) {
                Ok(prefix) => prefix,
                Err(error) => return interp.report_cmd_error(error.into()),
            }
        };
        let registration_name = recipe
            .alias_record_retains_original_registration_name()
            .then(|| Rc::new(obj::Owned::retain(argv[2])));
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
        owner.delete_command(&name);
        if let Err(first) = scope.currency().ensure_current_or_refuse() {
            owner.refuse_native_execution(first);
        }
        if let Err(error) = arguments.check() {
            return interp.report_cmd_error(error);
        }
        if owner.host_refusal_pending() {
            return interp.transport_host_refusal_from(&owner);
        }
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
        return match query {
            Ok(Some(query)) => {
                interp.set_result(query.as_ptr());
                Code::Ok
            }
            Ok(None) => {
                let mut message = b"alias \"".to_vec();
                message.extend_from_slice(&name);
                message.extend_from_slice(b"\" not found");
                interp.set_error(&message)
            }
            Err(error) => {
                owner.refuse_completion_value_access(error);
                interp.transport_host_refusal_from(&owner)
            }
        };
    }
    let (registration_name, prefix) = captured.expect("actual child alias capture");
    let installed =
        interp.install_parent_alias_objects(child, &name, registration_name, Rc::clone(&prefix));
    if let Err(first) = scope.currency().ensure_current_or_refuse() {
        owner.refuse_native_execution(first);
    }
    if let Err(error) = arguments.check() {
        owner.retire_original_parent_alias(&name, &prefix);
        return interp.report_cmd_error(error);
    }
    if owner.host_refusal_pending() {
        owner.retire_original_parent_alias(&name, &prefix);
        return interp.transport_host_refusal_from(&owner);
    }
    match installed {
        Ok(true) => {}
        Ok(false) => return interp.error(b"could not find interpreter"),
        Err(error) => return interp.report_cmd_error(error.into()),
    }
    if recipe.alias_publication_checks_original_target() {
        if let Err(error) = arguments.bytes(3) {
            owner.retire_original_parent_alias(&name, &prefix);
            return interp.report_cmd_error(error);
        }
    }
    if let Err(first) = scope.currency().ensure_current_or_refuse() {
        owner.refuse_native_execution(first);
        owner.retire_original_parent_alias(&name, &prefix);
        return interp.transport_host_refusal_from(&owner);
    }
    if let Err(error) = arguments.check() {
        owner.retire_original_parent_alias(&name, &prefix);
        return interp.report_cmd_error(error);
    }
    if recipe.child_alias_returns_original_registration_name() {
        interp.set_result(argv[2]);
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
        Some(names) => names,
        None => return not_found_original_path(interp, argv[2]),
    };
    let elems: Vec<*mut TclObj> = names.iter().map(|n| obj::new_string_bytes(n)).collect();
    interp.set_result(interp.new_list_object(&elems));
    for e in elems {
        drop_fresh(e);
    }
    Code::Ok
}

// helpers

fn only_single_interp(interp: &mut Interp) -> Code {
    interp.set_error(b"only single-interp aliases (empty interpreter paths) are supported")
}

#[cfg(test)]
mod tests {
    use crate::counters;
    use crate::interp::{Code, Interp};
    use crate::obj::{self, TclObj};

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

    #[test]
    fn jim_child_constructor_refusal_preserves_typed_cause_and_parent_result() {
        // naming.interpreter.original-child-bootstrap-context
        // docs/design/analysis/name-resolution-proofs/interpreter-original-child-bootstrap-context.md
        // Direct constructor-worker ownership and unavailable-frame controls;
        // no native process or selected invocation after owner retirement claim.
        for retire_owner in [true, false] {
            counters::reset();
            {
                let mut interp = Interp::with_native_core(
                    crate::interp::default_host(),
                    crate::environment::profile_for_dialect("jim"),
                    tcl_registry::special_vars::NativeBootstrapInputs::default(),
                )
                .unwrap();
                let argv = obj::Owned::fresh(obj::new_string_bytes(b"PARENT ARGV"));
                interp.var_set_at(b"argv", argv.as_ptr(), 0).unwrap();
                let word = obj::Owned::fresh(obj::new_string_bytes(b"interp"));
                interp
                    .associate_native_jim_arguments(&[word.as_ptr()])
                    .unwrap();
                let original = interp.native_jim_object_context().unwrap();
                interp.set_result_bytes(b"PARENT RESULT");
                let result = interp.result_obj();
                let expected = if retire_owner {
                    original.retire();
                    "retired Jim interpreter"
                } else {
                    // Exact observed naming policy without entered event storage
                    // refuses its global getter; it cannot donate a Jim parent cell.
                    assert!(interp.set_observed_bigip_name_policy(
                        tcl_registry::f5::evidence::BigIpBuild::MEASURED_21_1_0_1,
                        tcl_registry::f5::BigIpExecutionContext::TmmIRule,
                        tcl_registry::f5::naming::BigIpNameEvent::HttpRequest,
                    ));
                    "observed event frame storage"
                };
                assert_eq!(
                    super::interp_cmd(&mut interp, &[word.as_ptr()]),
                    Code::Error
                );
                assert_eq!(
                    interp.native_access_refusal(),
                    Some(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(expected))
                );
                assert!(interp.host_refusal_pending());
                assert_eq!(interp.result_obj(), result);
                assert_eq!(interp.result_bytes(), b"PARENT RESULT");
                assert!(!interp.child_exists(b"::interp.handle0"));
                assert!(
                    interp
                        .find_command_id(crate::namespace::GLOBAL, b"::interp.handle0")
                        .is_none()
                );
            }
            assert_eq!(counters::finalize(), 0);
            assert_eq!(counters::double_free_count(), 0);
        }
    }

    #[test]
    fn c_child_constructor_refusal_keeps_owner_cause_without_success_result() {
        // naming.interpreter.original-child-bootstrap-context
        // docs/design/analysis/name-resolution-proofs/interpreter-original-child-bootstrap-context.md
        // Direct C installer host controls, including a genuinely retained
        // nested owner; native success rows do not observe these host failures.
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for spelling in [None, Some(b"child".as_slice()), Some(b"outer child")] {
                counters::reset();
                {
                    let mut interp = Interp::with_native_core(
                        crate::interp::default_host(),
                        crate::environment::profile_for_dialect(engine),
                        tcl_registry::special_vars::NativeBootstrapInputs::default(),
                    )
                    .unwrap();
                    let select = |owner: &mut Interp| {
                        assert!(owner.set_observed_bigip_name_policy(
                            tcl_registry::f5::evidence::BigIpBuild::MEASURED_21_1_0_1,
                            tcl_registry::f5::BigIpExecutionContext::TmmIRule,
                            tcl_registry::f5::naming::BigIpNameEvent::HttpRequest,
                        ));
                    };
                    if spelling == Some(b"outer child".as_slice()) {
                        interp.create_child(Some(b"outer".to_vec()));
                        assert!(interp.child_exists(b"outer"));
                        interp.with_child(b"outer", select).unwrap();
                    } else {
                        select(&mut interp);
                    }
                    let head = obj::Owned::fresh(obj::new_string_bytes(b"interp"));
                    let member = obj::Owned::fresh(obj::new_string_bytes(b"create"));
                    let path = spelling.map(|name| obj::Owned::fresh(obj::new_string_bytes(name)));
                    let mut words = vec![head.as_ptr(), member.as_ptr()];
                    if let Some(path) = &path {
                        words.push(path.as_ptr());
                    }
                    interp.set_result_bytes(b"PARENT RESULT");
                    let result = interp.result_obj();
                    assert_eq!(super::interp_create(&mut interp, &words), Code::Error);
                    assert_eq!(
                        interp.native_access_refusal(),
                        Some(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                            "unmeasured observed variable purpose",
                        ))
                    );
                    assert!(interp.host_refusal_pending());
                    assert_eq!(interp.result_obj(), result);
                    assert_eq!(interp.result_bytes(), b"PARENT RESULT");
                    if spelling == Some(b"outer child".as_slice()) {
                        let refusal = interp.native_access_refusal();
                        interp
                            .with_child(b"outer", |owner| {
                                assert!(owner.host_refusal_pending());
                                assert_eq!(owner.native_access_refusal(), refusal);
                                assert!(!owner.child_exists(b"child"));
                                assert!(
                                    owner
                                        .find_command_id(crate::namespace::GLOBAL, b"child")
                                        .is_none()
                                );
                            })
                            .unwrap();
                    } else {
                        let name = spelling.unwrap_or(b"interp0");
                        assert!(!interp.child_exists(name));
                        assert!(
                            interp
                                .find_command_id(crate::namespace::GLOBAL, name)
                                .is_none()
                        );
                    }
                }
                assert_eq!(counters::finalize(), 0);
                assert_eq!(counters::double_free_count(), 0);
            }
        }
    }

    struct AliasCaptureHost {
        inner: std::rc::Rc<dyn tcl_platform::Host>,
        stdout: std::rc::Rc<std::cell::RefCell<Vec<u8>>>,
        stderr: std::rc::Rc<std::cell::RefCell<Vec<u8>>>,
    }

    impl tcl_platform::StdIo for AliasCaptureHost {
        fn write_stdout(&self, bytes: &[u8]) {
            self.stdout.borrow_mut().extend_from_slice(bytes);
        }
        fn write_stderr(&self, bytes: &[u8]) {
            self.stderr.borrow_mut().extend_from_slice(bytes);
        }
    }

    impl tcl_platform::Host for AliasCaptureHost {
        fn capabilities(&self) -> tcl_platform::Capabilities {
            self.inner.capabilities()
        }
        fn clock(&self) -> &dyn tcl_platform::Clock {
            self.inner.clock()
        }
        fn stdio(&self) -> &dyn tcl_platform::StdIo {
            self
        }
        fn env(&self) -> &dyn tcl_platform::Env {
            self.inner.env()
        }
        fn numeric_environment(&self) -> Option<&dyn tcl_platform::NumericEnvironment> {
            self.inner.numeric_environment()
        }
        fn native_integer_formatter(&self) -> Option<&dyn tcl_platform::NativeIntegerFormatter> {
            self.inner.native_integer_formatter()
        }
        fn system_encoding(&self) -> tcl_platform::SystemEncoding {
            self.inner.system_encoding()
        }
        fn filesystem(&self) -> Option<&dyn tcl_platform::Filesystem> {
            self.inner.filesystem()
        }
        fn sockets(&self) -> Option<&dyn tcl_platform::Sockets> {
            self.inner.sockets()
        }
        fn process(&self) -> Option<&dyn tcl_platform::Process> {
            self.inner.process()
        }
    }

    fn compare_alias_publication_fixture(engine: &str, source: &[u8], native: &[u8]) {
        counters::reset();
        {
            let stdout = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
            let stderr = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
            let host = std::rc::Rc::new(AliasCaptureHost {
                inner: crate::interp::default_host(),
                stdout: std::rc::Rc::clone(&stdout),
                stderr: std::rc::Rc::clone(&stderr),
            });
            let profile = tcl_registry::model::resolve_environment(engine).unit_profile();
            let mut interp = Interp::with_native_core(
                host,
                profile,
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .expect("selected original native interpreter constructor");
            let code = interp.eval_str(source);
            assert!(
                !interp.host_refusal_pending(),
                "{engine}: admission {:?}, native access {:?}",
                interp.native_compilation_admission_error(),
                interp.native_access_refusal()
            );
            assert_eq!(code, Code::Ok, "{engine}: {:?}", interp.result_bytes());
            assert!(stderr.borrow().is_empty(), "{engine}: guest stderr");
            let stdout = stdout.borrow();
            // Provider version metadata is outside the public alias control rows.
            let observed = stdout
                .splitn(2, |byte| *byte == b'\n')
                .nth(1)
                .expect("Runtime version row");
            let expected = native
                .splitn(2, |byte| *byte == b'\n')
                .nth(1)
                .expect("native version row");
            assert_eq!(observed, expected, "{engine}: original public alias rows");
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

    // Native proof: naming.alias.c-current-namespace-publication-holder
    // docs/design/analysis/name-resolution-proofs/alias-c-current-namespace-publication-holder.md
    #[test]
    fn alias_publication_uses_current_c_holder_and_preserves_jim_incompatible_api() {
        let source = include_bytes!(
            "../../../rust/tcl-registry/tests/data/native_c_alias_holder197/probe.tcl"
        );
        for (engine, native) in [
            (
                "tcl8.4",
                include_bytes!(
                    "../../../rust/tcl-registry/tests/data/native_c_alias_holder197/8.4.20/stdout"
                )
                .as_slice(),
            ),
            (
                "tcl8.5",
                include_bytes!(
                    "../../../rust/tcl-registry/tests/data/native_c_alias_holder197/8.5.19/stdout"
                )
                .as_slice(),
            ),
            (
                "tcl8.6",
                include_bytes!(
                    "../../../rust/tcl-registry/tests/data/native_c_alias_holder197/8.6.18/stdout"
                )
                .as_slice(),
            ),
            (
                "tcl9.0",
                include_bytes!(
                    "../../../rust/tcl-registry/tests/data/native_c_alias_holder197/9.0.4/stdout"
                )
                .as_slice(),
            ),
            (
                "tcl9.1",
                include_bytes!(
                    "../../../rust/tcl-registry/tests/data/native_c_alias_holder197/9.1.0/stdout"
                )
                .as_slice(),
            ),
            (
                "jim",
                include_bytes!(
                    "../../../rust/tcl-registry/tests/data/native_c_alias_holder197/jim/stdout"
                )
                .as_slice(),
            ),
        ] {
            compare_alias_publication_fixture(engine, source, native);
        }
    }

    // Native proof: naming.alias.original-child-interpreter-publication
    // docs/design/analysis/name-resolution-proofs/alias-original-child-interpreter-publication.md
    #[test]
    fn child_alias_publication_matches_original_supported_provider_apis() {
        let source = include_bytes!(
            "../../../rust/tcl-registry/tests/data/native_child_alias_publication199/probe.tcl"
        );
        for (engine, native) in [
            ("tcl8.4", include_bytes!("../../../rust/tcl-registry/tests/data/native_child_alias_publication199/8.4.20/stdout").as_slice()),
            ("tcl8.5", include_bytes!("../../../rust/tcl-registry/tests/data/native_child_alias_publication199/8.5.19/stdout").as_slice()),
            ("tcl8.6", include_bytes!("../../../rust/tcl-registry/tests/data/native_child_alias_publication199/8.6.18/stdout").as_slice()),
            ("tcl9.0", include_bytes!("../../../rust/tcl-registry/tests/data/native_child_alias_publication199/9.0.4/stdout").as_slice()),
            ("tcl9.1", include_bytes!("../../../rust/tcl-registry/tests/data/native_child_alias_publication199/9.1.0/stdout").as_slice()),
            ("jim", include_bytes!("../../../rust/tcl-registry/tests/data/native_child_alias_publication199/jim/stdout").as_slice()),
        ] {
            compare_alias_publication_fixture(engine, source, native);
        }
    }

    // Native proof: naming.jim.rooted-alias-publication-and-lookup
    // docs/design/analysis/name-resolution-proofs/jim-rooted-alias-publication-and-lookup.md
    #[test]
    fn jim_alias_owner_replaces_canonical_root_slots_and_keeps_local_competition() {
        counters::reset();
        {
            let profile = tcl_registry::model::resolve_environment("jim").unit_profile();
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                profile,
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .expect("selected original Jim core constructor");
            let observe = |interp: &mut Interp, source: &[u8]| {
                assert_eq!(
                    interp.eval_str(source),
                    Code::Ok,
                    "{:?}",
                    interp.result_bytes()
                );
                interp.result_bytes()
            };
            // This API control isolates publication from the standalone entry.
            // The unchanged standalone source is compared independently below.
            for (original, value, setup, observation, expected) in [
                (
                    b"::alias_slot".as_slice(),
                    b"ROOT_ALIAS".as_slice(),
                    b"proc ::alias_slot {} {return ORIGINAL}".as_slice(),
                    b"set a [catch {alias_slot} ar];set b [catch {::alias_slot} br];list $a $ar $b $br [catch {info body ::alias_slot}]".as_slice(),
                    b"0 ROOT_ALIAS 0 ROOT_ALIAS 1".as_slice(),
                ),
                (
                    b"::AliasProbe slot".as_slice(),
                    b"MULTIWORD_ALIAS".as_slice(),
                    b"proc {::AliasProbe slot} {} {return ORIGINAL_MULTIWORD}".as_slice(),
                    b"set a [catch {{AliasProbe slot}} ar];set b [catch {{::AliasProbe slot}} br];list $a $ar $b $br [catch {info body {::AliasProbe slot}}]".as_slice(),
                    b"0 MULTIWORD_ALIAS 0 MULTIWORD_ALIAS 1".as_slice(),
                ),
                (
                    b"::::colon_slot".as_slice(),
                    b"COLON_ALIAS".as_slice(),
                    b"".as_slice(),
                    b"set a [catch {colon_slot} ar];set b [catch {::colon_slot} br];set c [catch {::::colon_slot} cr];list $a $ar $b $br $c $cr".as_slice(),
                    b"0 COLON_ALIAS 0 COLON_ALIAS 0 COLON_ALIAS".as_slice(),
                ),
                (
                    b"::competition_slot".as_slice(),
                    b"GLOBAL_ALIAS".as_slice(),
                    b"proc competition_slot {} {return ORIGINAL_GLOBAL};namespace eval AliasContext {proc competition_slot {} {return LOCAL_PROC}}".as_slice(),
                    b"set a [catch {competition_slot} ar];set b [catch {namespace eval AliasContext {competition_slot}} br];set c [catch {namespace eval AliasContext {::competition_slot}} cr];list $a $ar $b $br $c $cr".as_slice(),
                    b"0 GLOBAL_ALIAS 0 LOCAL_PROC 0 GLOBAL_ALIAS".as_slice(),
                ),
            ] {
                observe(&mut interp, setup);
                interp.install_alias(original, b"list".to_vec(), vec![value.to_vec()]).expect("selected original alias publication owner");
                assert_eq!(observe(&mut interp, observation), expected, "{original:?}");
            }
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

    #[test]
    fn original_jim_core_alias_matches_all_four_standalone_native_public_rows() {
        // naming.jim.rooted-alias-publication-and-lookup
        // docs/design/analysis/name-resolution-proofs/jim-rooted-alias-publication-and-lookup.md
        // Core registration and unchanged public source; no private object
        // identity follows from the native stdout observations.
        compare_alias_publication_fixture(
            "jim",
            include_bytes!(
                "../../../rust/tcl-registry/tests/data/native_jim_alias_publication194/probe.tcl"
            ),
            include_bytes!(
                "../../../rust/tcl-registry/tests/data/native_jim_alias_publication194/jim/stdout"
            ),
        );
    }

    #[test]
    fn jim_core_alias_retains_original_name_result_and_prefix_members() {
        // naming.alias.jim-original-core-prefix-object-storage
        // docs/design/analysis/name-resolution-proofs/alias-jim-original-core-prefix-object-storage.md
        // The original API observer independently records name/member identity
        // and absent resident member strings at definition, query and invocation.
        // Implementation control: the retained prefix owner supplies actual
        // objects to dispatch; native public rows above establish byte results.
        counters::reset();
        {
            let profile = tcl_registry::model::resolve_environment("jim").unit_profile();
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                profile,
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let head = obj::Owned::fresh(obj::new_string_bytes(b"alias"));
            let name = obj::Owned::fresh(obj::new_string_bytes(b"::original_prefix_alias"));
            let target = obj::Owned::fresh(obj::new_string_bytes(b"list"));
            let member = obj::Owned::fresh(obj::new_wide_int_obj(17));
            assert!(!obj::has_string_rep(member.as_ptr()));
            assert_eq!(
                interp.dispatch(&[
                    head.as_ptr(),
                    name.as_ptr(),
                    target.as_ptr(),
                    member.as_ptr()
                ]),
                Code::Ok
            );
            assert_eq!(interp.get_obj_result(), name.as_ptr());
            assert!(!obj::has_string_rep(member.as_ptr()));
            let info = obj::Owned::fresh(obj::new_string_bytes(b"info"));
            let selector = obj::Owned::fresh(obj::new_string_bytes(b"alias"));
            assert_eq!(
                interp.dispatch(&[info.as_ptr(), selector.as_ptr(), name.as_ptr()]),
                Code::Ok
            );
            let query = interp.get_obj_result();
            assert_eq!(
                tcl_syntax::value::ValueOps::list_elements(&mut interp, &query).unwrap(),
                vec![target.as_ptr(), member.as_ptr()]
            );
            assert!(!obj::has_string_rep(member.as_ptr()));
            // Current owner control: a second public query keeps this header;
            // the native273 observer separately proves member identity/cache.
            assert_eq!(
                interp.dispatch(&[info.as_ptr(), selector.as_ptr(), name.as_ptr()]),
                Code::Ok
            );
            assert_eq!(interp.get_obj_result(), query);
            assert!(!obj::has_string_rep(member.as_ptr()));
            assert_eq!(interp.dispatch(&[name.as_ptr()]), Code::Ok);
            assert!(!obj::has_string_rep(member.as_ptr()));
            let result = interp.get_obj_result();
            assert_eq!(
                tcl_syntax::value::ValueOps::list_elements(&mut interp, &result).unwrap(),
                vec![member.as_ptr()]
            );
            let c = Interp::new();
            assert!(c.resolve_cmd_token(b"alias").is_none());
        }
        assert_eq!(counters::finalize(), 0);
        assert_eq!(counters::double_free_count(), 0);
    }

    #[test]
    fn jim_child_counted_crossings_match_original_native_public_rows() {
        // naming.interpreter.jim-original-object-crossing
        // docs/design/analysis/name-resolution-proofs/jim-original-object-crossing.md
        // The counted driver metadata/result channel is separate from the
        // guest's public rows. These rows assert no private pointer ownership.
        let recorded = include_bytes!(
            "../../../rust/tcl-registry/tests/data/native_jim_child_object_crossing302/jim/original-counted-child-crossing/stdout"
        );
        let mut channels = recorded.splitn(3, |byte| *byte == b'\n');
        assert_eq!(
            channels.next().unwrap(),
            b"VERSION|0|302e38342d392d6735626163376339"
        );
        assert_eq!(channels.next().unwrap(), b"ORIGINAL|0|");
        let guest_rows = channels.next().unwrap();
        compare_alias_publication_fixture(
            "jim",
            include_bytes!(
                "../../../rust/tcl-registry/tests/data/native_jim_child_object_crossing302/probe.tcl"
            ),
            guest_rows,
        );
    }

    /// The `interp` ensemble and the child-as-command dispatch
    /// are `Tcl_GetIndexFromObj(…, "option", 0)` tables (`options[]` in
    /// `Tcl_InterpObjCmd` and `NRChildCmd`, `tclInterp.c`), so subcommands
    /// abbreviate and the empty word — a prefix of every entry — is
    /// `ambiguous option ""`. The `interp` list still names only what this
    /// runtime dispatches; the child list is tclsh's in full.
    ///
    /// tclsh 8.6.16 / 9.0.4 (the verdicts, not the shortened `interp` list):
    ///   interp {}       -> ambiguous option "": must be …
    ///   interp c j      -> ambiguous option "c": must be …
    ///   interp cr j     -> j
    ///   interp e {}     -> ambiguous option "e": must be …
    ///   interp sl       -> the children list (8.x's deprecated spelling)
    ///   kid ev {set x 1} -> 1   ;  kid h / kid hi -> ambiguous option "h" / "hi"
    ///   kid x           -> bad option "x": must be alias, aliases, bgerror,
    ///                      debug, eval, expose, hide, hidden, issafe,
    ///                      invokehidden, limit, marktrusted, or recursionlimit
    #[test]
    fn jim_interpreter_handle_factory_alias_frame_and_deletion() {
        let mut i = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("jim"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .expect("actual original Jim core constructor");
        assert_eq!(i.eval_str(br#"set ::x GLOBAL; namespace eval ::N {proc observe {} {set x LOCAL; set child [interp]; $child alias readX set x; $child alias where namespace current; set result [list [$child eval {readX}] [$child eval {where}]]; $child delete; return $result}}; ::N::observe"#), Code::Ok, "result={:?}, admission={:?}, access={:?}", i.result_bytes(), i.native_compilation_admission_error(), i.native_access_refusal());
        assert_eq!(i.result_bytes(), b"LOCAL ::N");
    }

    #[test]
    fn jim_interpreter_handle_isolation() {
        let mut i = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("jim"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .expect("actual original Jim core constructor");
        assert_eq!(i.eval_str(br#"namespace eval ::N {proc probe {} {return PARENT}}; package provide Isolated 2.0; set child [interp]; $child eval {namespace eval ::N {proc probe {} {return CHILD}}}; set result [list [::N::probe] [$child eval {::N::probe}] [$child eval {catch {package require Isolated}}]]; $child delete; set result"#), Code::Ok, "result={:?}, admission={:?}, access={:?}", i.result_bytes(), i.native_compilation_admission_error(), i.native_access_refusal());
        assert_eq!(i.result_bytes(), b"PARENT CHILD 1");
    }

    #[test]
    fn interp_abbreviated_operations_dispatch_in_root_and_child() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"interp cr j"), Code::Ok);
            assert_eq!(i.result_bytes(), b"j");
            assert_eq!(i.eval_str(b"interp ev {} {set x 1}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
            // The primary lookup table retains this spelling independently of
            // the release-specific table used to report a failed lookup.
            assert_eq!(i.eval_str(b"llength [interp sl]"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
            assert_eq!(i.eval_str(b"interp create kid"), Code::Ok);
            assert_eq!(i.eval_str(b"kid ev {set x 1}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
        });
    }

    /// `interp create`'s and `interp invokehidden`'s leading
    /// options are `Tcl_GetIndexFromObj(…, "option", 0)` tables
    /// (`createOptions[]` / `hiddenOptions[]`, `tclInterp.c`), so they
    /// abbreviate and the lone `-` — a prefix of every entry — is `ambiguous`.
    ///
    /// tclsh 8.6.16 / 9.0.4:
    ///   interp create -x k         -> bad option "-x": must be -safe or --
    ///   interp create - k          -> ambiguous option "-": must be -safe or --
    ///   interp create -s k         -> k
    ///   interp invokehidden i -x f -> bad option "-x": must be -global, -namespace, or --
    ///   interp invokehidden i - f  -> ambiguous option "-": must be -global, -namespace, or --
    #[test]
    fn interp_create_and_invokehidden_options_resolve_like_tcl_get_index_from_obj() {
        const CREATE_MUST: &str = "must be -safe or --";
        const HIDDEN_MUST: &str = "must be -global, -namespace, or --";
        leak_free(|i| {
            let err = |i: &mut Interp, script: &[u8]| {
                assert_eq!(i.eval_str(script), Code::Error, "expected an error");
                String::from_utf8_lossy(&i.result_bytes()).into_owned()
            };
            assert_eq!(
                err(i, b"interp create -x k"),
                format!("bad option \"-x\": {CREATE_MUST}")
            );
            assert_eq!(
                err(i, b"interp create - k"),
                format!("ambiguous option \"-\": {CREATE_MUST}")
            );
            assert_eq!(i.eval_str(b"interp create -s k"), Code::Ok);
            assert_eq!(i.result_bytes(), b"k");
            assert_eq!(i.eval_str(b"interp create -- k2"), Code::Ok);
            assert_eq!(i.result_bytes(), b"k2");
            assert_eq!(i.eval_str(b"interp create kid"), Code::Ok);
            assert_eq!(
                err(i, b"interp invokehidden kid -x foo"),
                format!("bad option \"-x\": {HIDDEN_MUST}")
            );
            assert_eq!(
                err(i, b"interp invokehidden kid - foo"),
                format!("ambiguous option \"-\": {HIDDEN_MUST}")
            );
        });
    }

    #[test]
    fn original_interpreter_paths_preserve_child_keys_and_create_result_objects() {
        // Native proof: naming.interpreter.counted-list-path-cstring-child-keys
        // docs/design/analysis/name-resolution-proofs/interpreter-counted-list-path-cstring-child-keys.md
        // Native proof: naming.interpreter.singleton-create-original-spelling
        // docs/design/analysis/name-resolution-proofs/interpreter-singleton-create-original-spelling.md
        // Native proof: naming.interpreter.malformed-path-list-error
        // docs/design/analysis/name-resolution-proofs/interpreter-malformed-path-list-error.md
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let call = |interp: &mut Interp, member: &[u8], path: *mut TclObj| {
                let head = obj::Owned::fresh(obj::new_string_bytes(b"interp"));
                let member = obj::Owned::fresh(obj::new_string_bytes(member));
                interp.dispatch(&[head.as_ptr(), member.as_ptr(), path])
            };
            for bytes in [
                b"child\0suffix".as_slice(),
                b"opaque\xff",
                b"surrogate\xed\xa0\x80",
                b"encoded\xc0\x80tail",
            ] {
                let original = obj::Owned::fresh(obj::new_string_bytes(bytes));
                assert_eq!(
                    call(&mut interp, b"create", original.as_ptr()),
                    Code::Ok,
                    "{engine}"
                );
                assert_eq!(interp.get_obj_result(), original.as_ptr());
                assert_eq!(call(&mut interp, b"exists", original.as_ptr()), Code::Ok);
                assert_eq!(interp.result_bytes(), b"1");
                assert_eq!(call(&mut interp, b"delete", original.as_ptr()), Code::Ok);
                assert_eq!(call(&mut interp, b"exists", original.as_ptr()), Code::Ok);
                assert_eq!(interp.result_bytes(), b"0");
            }
            let singleton = obj::Owned::fresh(obj::new_string_bytes(b"{with space}"));
            assert_eq!(call(&mut interp, b"create", singleton.as_ptr()), Code::Ok);
            assert_eq!(interp.get_obj_result(), singleton.as_ptr());
            assert_eq!(call(&mut interp, b"exists", singleton.as_ptr()), Code::Ok);
            assert_eq!(interp.result_bytes(), b"0");
            let address = obj::Owned::fresh(obj::new_string_bytes(b"{{with space}}"));
            assert_eq!(call(&mut interp, b"exists", address.as_ptr()), Code::Ok);
            assert_eq!(interp.result_bytes(), b"1");
            let empty = obj::Owned::fresh(obj::new_string_bytes(b""));
            assert_eq!(call(&mut interp, b"create", empty.as_ptr()), Code::Ok);
            assert_eq!(interp.get_obj_result(), empty.as_ptr());
            let empty_child = obj::Owned::fresh(obj::new_string_bytes(b"{}"));
            assert_eq!(call(&mut interp, b"exists", empty_child.as_ptr()), Code::Ok);
            assert_eq!(interp.result_bytes(), b"1");
            assert_eq!(call(&mut interp, b"delete", empty_child.as_ptr()), Code::Ok);
            assert_eq!(call(&mut interp, b"exists", empty_child.as_ptr()), Code::Ok);
            assert_eq!(interp.result_bytes(), b"0");
            let malformed = obj::Owned::fresh(obj::new_string_bytes(b"{"));
            assert_eq!(
                call(&mut interp, b"create", malformed.as_ptr()),
                Code::Error
            );
            assert_eq!(call(&mut interp, b"exists", malformed.as_ptr()), Code::Ok);
            assert_eq!(interp.result_bytes(), b"0");
            assert!(!interp.host_refusal_pending());
        }
    }

    // Needs the numeric tower: the child's `dbl` proc computes via `expr`.
    #[cfg(have_tommath)]
    #[test]
    fn child_interpreters() {
        // `interp create`/`eval`/`exists`/`children`/`delete` + the child as a
        // command (`$child eval …`). Verified vs tclsh 9.0.
        leak_free(|i| {
            assert_eq!(i.eval_str(b"interp create kid"), Code::Ok);
            assert_eq!(i.result_bytes(), b"kid");
            assert_eq!(i.eval_str(b"interp exists kid"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
            // The child is isolated and addressable as a command.
            i.eval_str(b"kid eval {set x 42; proc dbl n {expr {$n*2}}}");
            assert_eq!(i.eval_str(b"kid eval {dbl $x}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"84");
            // ... and via `interp eval`.
            assert_eq!(i.eval_str(b"interp eval kid {set x}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"42");
            // The parent doesn't see the child's variable.
            assert_eq!(i.eval_str(b"info exists x"), Code::Ok);
            assert_eq!(i.result_bytes(), b"0");
            assert_eq!(i.eval_str(b"interp children"), Code::Ok);
            assert_eq!(i.result_bytes(), b"kid");
            // Children get the predefined globals.
            assert_eq!(
                i.eval_str(b"kid eval {set ::tcl_platform(platform)}"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"unix");
            // Delete removes the child and its command.
            assert_eq!(i.eval_str(b"interp delete kid"), Code::Ok);
            assert_eq!(i.eval_str(b"interp exists kid"), Code::Ok);
            assert_eq!(i.result_bytes(), b"0");
            assert_eq!(i.eval_str(b"kid eval {set x}"), Code::Error);
        });
    }

    // Needs the numeric tower: alias targets are `expr`-backed (`padd`) and `::tcl::mathop::*`.
    #[cfg(have_tommath)]
    #[test]
    fn cross_interp_aliases() {
        // A child alias delegating to a parent command (both syntaxes), and
        // re-entrancy (a parent alias re-entering the *same* child mid-eval —
        // the Safe Base pattern — now recurses correctly, bounded not forbidden).
        leak_free(|i| {
            i.eval_str(b"proc padd {a b} {expr {$a+$b}}");
            i.eval_str(b"set c [interp create]");
            // `interp alias $c name {} target prefix...`
            i.eval_str(b"interp alias $c add {} padd 100");
            assert_eq!(i.eval_str(b"$c eval {add 5}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"105");
            // `$c alias name target prefix...`
            i.eval_str(b"$c alias mul ::tcl::mathop::* 3");
            assert_eq!(i.eval_str(b"$c eval {mul 4}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"12");
            // Re-entrancy: a parent alias target that evals back into the same
            // child while its outer eval is still on the stack. This recurses
            // (the child's `x` ends up set), it does not error.
            i.eval_str(b"proc reenter {} { $::c eval {set x 42} }");
            i.eval_str(b"interp alias $c cb {} reenter");
            assert_eq!(i.eval_str(b"$c eval {cb}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"42");
            assert_eq!(i.eval_str(b"$c eval {set x}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"42");
            i.eval_str(b"interp delete $c; unset -nocomplain c");
        });
    }

    // Needs the numeric tower: the `-safe` hidden-list assert compares via `expr`.
    #[cfg(have_tommath)]
    #[test]
    fn hidden_commands_and_safe() {
        // `interp hide`/`expose`/`invokehidden` + `interp create -safe`.
        leak_free(|i| {
            i.eval_str(b"set c [interp create]");
            i.eval_str(b"$c hide set");
            // hidden `set` is gone from the child but invocable via invokehidden.
            assert_eq!(i.eval_str(b"$c eval {set x 1}"), Code::Error);
            assert_eq!(i.eval_str(b"$c hidden"), Code::Ok);
            assert_eq!(i.result_bytes(), b"set");
            assert_eq!(i.eval_str(b"$c invokehidden set y 5"), Code::Ok);
            assert_eq!(i.result_bytes(), b"5");
            i.eval_str(b"$c expose set");
            assert_eq!(i.eval_str(b"$c eval {set z 9}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"9");
            // `-safe` hides the host-touching commands it has.
            i.eval_str(b"set s [interp create -safe]");
            assert_eq!(
                i.eval_str(b"expr {[lsearch [$s hidden] file] >= 0}"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"1");
            i.eval_str(b"interp delete $c; interp delete $s");
        });
    }

    #[test]
    fn rename_moves_a_command() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"rename set put"), Code::Ok);
            assert_eq!(i.result_bytes(), b"");
            // `put` now works …
            assert_eq!(i.eval_str(b"put x 5"), Code::Ok);
            assert_eq!(i.result_bytes(), b"5");
            // … and `set` no longer resolves.
            assert_eq!(i.eval_str(b"set y 1"), Code::Error);
            assert_eq!(i.result_bytes(), b"invalid command name \"set\"");
        });
    }

    #[test]
    fn rename_to_empty_deletes() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"rename incr {}"), Code::Ok);
            assert_eq!(i.eval_str(b"incr n"), Code::Error);
            assert_eq!(i.result_bytes(), b"invalid command name \"incr\"");
        });
    }

    #[test]
    fn rename_missing_is_an_error() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"rename nope gone"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"can't rename \"nope\": command doesn't exist"
            );
            assert_eq!(i.eval_str(b"rename nope {}"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"can't delete \"nope\": command doesn't exist"
            );
        });
    }

    #[test]
    fn rename_and_hide_error_shapes_and_lifecycle() {
        leak_free(|i| {
            // The verb is selected by the empty destination, including for a
            // qualified missing name (TclRenameCommand / tclNamesp.c).
            assert_eq!(i.eval_str(b"rename ::missing {}"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"can't delete \"::missing\": command doesn't exist"
            );

            // Qualified moves preserve the command and resolve from the new
            // namespace; deletion uses the same canonical rename seam.
            assert_eq!(
                i.eval_str(
                    b"namespace eval ::rename_ns {proc p {} {return qualified}}; rename ::rename_ns::p ::rename_ns::q"
                ),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"::rename_ns::q"), Code::Ok);
            assert_eq!(i.result_bytes(), b"qualified");
            assert_eq!(i.eval_str(b"rename ::rename_ns::q {}"), Code::Ok);
            assert_eq!(i.eval_str(b"::rename_ns::q"), Code::Error);
            assert_eq!(i.result_bytes(), b"invalid command name \"::rename_ns::q\"");

            // Hiding an absent command is an error, so repeated hide cannot
            // silently swallow a typo in a security-sensitive setup.
            assert_eq!(i.eval_str(b"interp hide {} nosuchcmd"), Code::Error);
            assert_eq!(i.result_bytes(), b"unknown command \"nosuchcmd\"");
            assert_eq!(
                i.eval_str(b"proc visible {} {return visible}; interp hide {} visible"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"interp hidden {}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"visible");
            // Hiding again once the source has moved into the hidden table is a
            // *lookup* miss, not a token collision: `Tcl_HideCommand` resolves
            // the source before it consults the hidden table, and the source is
            // no longer there to resolve (tclsh 8.6.16 / 9.0.4-pinned). Reaching
            // the collision needs a live source *and* a taken token, so the
            // command has to be re-created first.
            assert_eq!(i.eval_str(b"interp hide {} visible"), Code::Error);
            assert_eq!(i.result_bytes(), b"unknown command \"visible\"");
            assert_eq!(i.eval_str(b"proc visible {} {return shadow}"), Code::Ok);
            assert_eq!(i.eval_str(b"interp hide {} visible"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"hidden command named \"visible\" already exists"
            );
            assert_eq!(i.eval_str(b"rename visible {}"), Code::Ok);
            assert_eq!(i.eval_str(b"interp invokehidden {} visible"), Code::Ok);
            assert_eq!(i.result_bytes(), b"visible");
            assert_eq!(i.eval_str(b"interp expose {} visible"), Code::Ok);
            assert_eq!(i.eval_str(b"visible"), Code::Ok);
            assert_eq!(i.result_bytes(), b"visible");

            // Destination conflicts are diagnosed before either table is
            // mutated. Both the source command and existing destination keep
            // their identities and remain callable.
            assert_eq!(
                i.eval_str(
                    b"proc p {} {return P}
                      interp hide {} p held
                      proc q {} {return Q}
                      set c [catch {interp hide {} q held} m o]
                      list $c $m [dict get $o -errorcode] [q] \
                           [interp invokehidden {} held] [interp hidden {}]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"1 {hidden command named \"held\" already exists} \
                  {TCL HIDE ALREADY_HIDDEN} Q P held"
            );

            assert_eq!(
                i.eval_str(
                    b"proc E {} {return E}
                      set c [catch {interp expose {} held E} m o]
                      list $c $m [dict get $o -errorcode] [E] \
                           [interp invokehidden {} held] [interp hidden {}]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"1 {exposed command \"E\" already exists} \
                  {TCL EXPOSE COMMAND_EXISTS} E P held"
            );

            // Release-hidden builtins are absent from both rename and hide;
            // neither operation may make an 8.5+ command callable in 8.4.
            i.set_runtime_version(tcl_dialect::TclVersion::V8_4);
            assert_eq!(i.eval_str(b"rename lassign {}"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"can't delete \"lassign\": command doesn't exist"
            );
            assert_eq!(i.eval_str(b"interp hide {} lassign"), Code::Error);
            assert_eq!(i.result_bytes(), b"unknown command \"lassign\"");
        });
    }

    /// The `$child` shorthand now routes through `hidectl_in` /
    /// `invokehidden_in`, and the latter moved the manual
    /// `incr_ref_count`/`decr_ref_count` bracketing around the hidden argv out
    /// of two call sites into one. The counter harness is the guard on that
    /// move: a dropped `+1` or a missing `-1` shows up here and nowhere else.
    #[test]
    fn child_shorthand_visibility_ops_are_leak_free() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"set c [interp create]"), Code::Ok);
            assert_eq!(i.eval_str(b"$c eval {proc lst {} {return L}}"), Code::Ok);
            assert_eq!(i.eval_str(b"$c hide lst mylst"), Code::Ok);
            assert_eq!(i.eval_str(b"$c hidden"), Code::Ok);
            assert_eq!(i.result_bytes(), b"mylst");
            assert_eq!(i.eval_str(b"$c invokehidden mylst"), Code::Ok);
            assert_eq!(i.result_bytes(), b"L");
            assert_eq!(i.eval_str(b"$c expose mylst lst2"), Code::Ok);
            assert_eq!(i.eval_str(b"$c hide set"), Code::Ok);
            assert_eq!(i.eval_str(b"$c invokehidden -global set g 1"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
            assert_eq!(
                i.eval_str(b"$c invokehidden -namespace foo set q 2"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"2");
            assert_eq!(
                i.eval_str(b"catch {$c invokehidden -bogus set x 1} m; set m"),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"bad option \"-bogus\": must be -global, -namespace, or --"
            );
            assert_eq!(i.eval_str(b"catch {$c hide ::foo::bar} m; set m"), Code::Ok);
            assert_eq!(
                i.result_bytes(),
                b"cannot use namespace qualifiers in hidden command token (rename)"
            );
            assert_eq!(i.eval_str(b"interp delete $c"), Code::Ok);
        });
    }

    #[test]
    fn visibility_failures_match_direct_and_child_oracles() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"set out {}
                      foreach script {
                          {interp hide {} nosuch}
                          {interp expose {} nosuch}
                      } {
                          catch $script m o
                          lappend out $m [dict get $o -errorcode]
                      }
                      proc held {} {return OLD}
                      interp hide {} held
                      proc held {} {return NEW}
                      catch {interp hide {} held} m o
                      lappend out $m [dict get $o -errorcode]
                      proc E {} {return OLD}
                      interp hide {} E
                      proc E {} {return NEW}
                      catch {interp expose {} E} m o
                      lappend out $m [dict get $o -errorcode]
                      set out"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"{unknown command \"nosuch\"} {TCL LOOKUP COMMAND nosuch} \
                  {unknown hidden command \"nosuch\"} \
                  {TCL LOOKUP HIDDENTOKEN nosuch} \
                  {hidden command named \"held\" already exists} \
                  {TCL HIDE ALREADY_HIDDEN} \
                  {exposed command \"E\" already exists} \
                  {TCL EXPOSE COMMAND_EXISTS}"
            );
        });

        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"set c [interp create]
                      set out {}
                      foreach subcommand {hide expose} {
                          catch [list $c $subcommand nosuch] m o
                          lappend out $m [dict get $o -errorcode]
                      }
                      $c eval {proc held {} {return OLD}}
                      $c hide held
                      $c eval {proc held {} {return NEW}}
                      catch [list $c hide held] m o
                      lappend out $m [dict get $o -errorcode]
                      $c eval {proc E {} {return OLD}}
                      $c hide E
                      $c eval {proc E {} {return NEW}}
                      catch [list $c expose E] m o
                      lappend out $m [dict get $o -errorcode]
                      set out"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"{unknown command \"nosuch\"} {TCL LOOKUP COMMAND nosuch} \
                  {unknown hidden command \"nosuch\"} \
                  {TCL LOOKUP HIDDENTOKEN nosuch} \
                  {hidden command named \"held\" already exists} \
                  {TCL HIDE ALREADY_HIDDEN} \
                  {exposed command \"E\" already exists} \
                  {TCL EXPOSE COMMAND_EXISTS}"
            );
        });
    }

    /// C Tcl has no protected-command list for `rename`: even `return` may
    /// be renamed and used under its new name (tclsh 8.6.16 / 9.0.4:
    /// `rename ::return ::myreturn` succeeds).
    #[test]
    fn rename_builtin_return_is_allowed() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"rename return myreturn"), Code::Ok);
            assert_eq!(i.eval_str(b"proc p {} { myreturn done }"), Code::Ok);
            assert_eq!(i.eval_str(b"p"), Code::Ok);
            assert_eq!(i.result_bytes(), b"done");
            // Restore for any later tests in this interp.
            assert_eq!(i.eval_str(b"rename myreturn return"), Code::Ok);
        });
    }

    #[test]
    fn alias_create_and_dispatch() {
        leak_free(|i| {
            // `=` is an alias for `set`.
            assert_eq!(i.eval_str(b"interp alias {} = {} set"), Code::Ok);
            assert_eq!(i.result_bytes(), b"=");
            assert_eq!(i.eval_str(b"= x 42"), Code::Ok);
            assert_eq!(i.result_bytes(), b"42");
            assert_eq!(i.eval_str(b"set y $x"), Code::Ok);
            assert_eq!(i.result_bytes(), b"42");
        });
    }

    #[test]
    fn alias_prepends_frozen_prefix() {
        leak_free(|i| {
            // `store` aliases `set k`, so `store v` runs `set k v`.
            assert_eq!(i.eval_str(b"interp alias {} store {} set k"), Code::Ok);
            assert_eq!(i.eval_str(b"store hello"), Code::Ok);
            assert_eq!(i.result_bytes(), b"hello");
            assert_eq!(i.eval_str(b"set out $k"), Code::Ok);
            assert_eq!(i.result_bytes(), b"hello");
        });
    }

    #[test]
    fn alias_query_returns_target_and_prefix() {
        leak_free(|i| {
            i.eval_str(b"interp alias {} store {} set k");
            assert_eq!(i.eval_str(b"interp alias {} store"), Code::Ok);
            assert_eq!(i.result_bytes(), b"set k");
        });
    }

    #[test]
    fn alias_delete_unbinds() {
        leak_free(|i| {
            i.eval_str(b"interp alias {} = {} set");
            assert_eq!(i.eval_str(b"interp alias {} = {}"), Code::Ok);
            assert_eq!(i.eval_str(b"= x 1"), Code::Error);
            assert_eq!(i.result_bytes(), b"invalid command name \"=\"");
        });
    }

    #[test]
    fn aliases_lists_every_alias() {
        leak_free(|i| {
            i.eval_str(b"interp alias {} = {} set");
            i.eval_str(b"interp alias {} store {} set k");
            assert_eq!(i.eval_str(b"interp aliases {}"), Code::Ok);
            // BTreeMap order → sorted by name.
            assert_eq!(i.result_bytes(), b"= store");
        });
    }

    #[test]
    fn alias_to_deleted_target_errors_lazily() {
        leak_free(|i| {
            i.eval_str(b"interp alias {} = {} set");
            // Deleting the target makes the alias fail at *its* next dispatch,
            // attributing the miss to the target name.
            assert_eq!(i.eval_str(b"rename set {}"), Code::Ok);
            assert_eq!(i.eval_str(b"= x 1"), Code::Error);
            assert_eq!(i.result_bytes(), b"invalid command name \"set\"");
        });
    }

    #[test]
    fn alias_does_not_follow_target_rename() {
        leak_free(|i| {
            i.eval_str(b"interp alias {} = {} set");
            // Renaming the target out from under the alias: the stored name stops
            // resolving (C Tcl semantics — alias binds by name, not by command).
            assert_eq!(i.eval_str(b"rename set put"), Code::Ok);
            assert_eq!(i.eval_str(b"= x 1"), Code::Error);
            assert_eq!(i.result_bytes(), b"invalid command name \"set\"");
        });
    }

    /// C's `TclPreventAliasLoop` (`tclInterp.c`) refuses the alias that closes
    /// a cycle at *definition* time — the pair installs fine in neither order
    /// (tclsh 8.6.16 / 9.0.4-pinned wording, shared with the VM's
    /// `cross_interp_alias_e2e` vectors).
    #[test]
    fn a_mutual_alias_pair_is_refused_at_the_closing_alias() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"interp alias {} a {} b"), Code::Ok);
            assert_eq!(i.eval_str(b"interp alias {} b {} a"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"cannot define or rename alias \"b\": would create a loop"
            );
            // The refused alias is not left behind …
            assert_eq!(i.eval_str(b"info commands b"), Code::Ok);
            assert_eq!(i.result_bytes(), b"");
            // … so the surviving one still just misses its (absent) target
            // instead of recursing without bound.
            assert_eq!(i.eval_str(b"a"), Code::Error);
            assert_eq!(i.result_bytes(), b"invalid command name \"b\"");
        });
    }

    /// A self-alias is refused too — and, as in C, the command it displaced is
    /// *not* restored (the alias is created first, then unbound on the loop).
    #[test]
    fn a_self_alias_is_refused_and_destroys_the_command_it_displaced() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"proc x {} {return REAL}"), Code::Ok);
            assert_eq!(i.eval_str(b"interp alias {} x {} x"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"cannot define or rename alias \"x\": would create a loop"
            );
            assert_eq!(i.eval_str(b"x"), Code::Error);
            assert_eq!(i.result_bytes(), b"invalid command name \"x\"");
            // A qualified self-spelling is caught by *resolution*, not by a
            // string compare of the two names.
            assert_eq!(i.eval_str(b"interp alias {} q {} ::q"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"cannot define or rename alias \"q\": would create a loop"
            );
        });
    }

    /// The walk follows the whole chain, not just one hop, and a frozen prefix
    /// on the closing alias does not hide the cycle.
    #[test]
    fn a_longer_alias_cycle_is_refused_at_its_closing_hop() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"interp alias {} c {} d"), Code::Ok);
            assert_eq!(i.eval_str(b"interp alias {} d {} e"), Code::Ok);
            assert_eq!(i.eval_str(b"interp alias {} e {} c extra"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"cannot define or rename alias \"e\": would create a loop"
            );
            // A namespaced cycle is refused the same way, named by the alias's
            // simple command name (C's `Tcl_GetCommandName`), not its path.
            assert_eq!(i.eval_str(b"namespace eval ns {}"), Code::Ok);
            assert_eq!(i.eval_str(b"interp alias {} ns::p {} ns::q"), Code::Ok);
            assert_eq!(i.eval_str(b"interp alias {} ns::q {} ns::p"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"cannot define or rename alias \"q\": would create a loop"
            );
        });
    }

    /// C guards `rename` with the same check: moving an alias onto a name its
    /// own target chain resolves back to is refused, and the table is left
    /// exactly as it was.
    #[test]
    fn a_rename_that_would_close_a_loop_is_refused_and_rolled_back() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"interp alias {} a {} b"), Code::Ok);
            assert_eq!(i.eval_str(b"rename a b"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"cannot define or rename alias \"b\": would create a loop"
            );
            assert_eq!(i.eval_str(b"info commands a"), Code::Ok);
            assert_eq!(i.result_bytes(), b"a");
            assert_eq!(i.eval_str(b"info commands b"), Code::Ok);
            assert_eq!(i.result_bytes(), b"");
        });
    }

    /// The refused rename must not damage the command sitting at the
    /// destination either: the tentative move the gate makes is undone
    /// completely. (tclsh refuses this one earlier still, with `can't rename
    /// to "bb": command already exists` — this runtime's `rename` has no
    /// destination-exists check, so it reports the loop instead; the surviving
    /// table state is the same either way.)
    #[test]
    fn a_refused_rename_leaves_the_destination_command_intact() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"proc bb {} {return REAL}"), Code::Ok);
            assert_eq!(i.eval_str(b"interp alias {} aa {} bb"), Code::Ok);
            assert_eq!(i.eval_str(b"rename aa bb"), Code::Error);
            assert_eq!(i.eval_str(b"bb"), Code::Ok);
            assert_eq!(i.result_bytes(), b"REAL");
            assert_eq!(i.eval_str(b"aa"), Code::Ok);
            assert_eq!(i.result_bytes(), b"REAL");
        });
    }

    /// The gate refuses cycles only: a legitimate alias-of-alias chain still
    /// dispatches, and an alias to a target that does not exist yet is legal
    /// (aliases late-bind).
    #[test]
    fn legitimate_alias_chains_and_late_binding_still_work() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"interp alias {} = {} set"), Code::Ok);
            assert_eq!(i.eval_str(b"interp alias {} := {} ="), Code::Ok);
            assert_eq!(i.eval_str(b":= zz 42"), Code::Ok);
            assert_eq!(i.result_bytes(), b"42");
            assert_eq!(i.eval_str(b"set out $zz"), Code::Ok);
            assert_eq!(i.result_bytes(), b"42");
            assert_eq!(i.eval_str(b"interp alias {} lb {} no_such_yet"), Code::Ok);
            assert_eq!(i.eval_str(b"proc no_such_yet {} {return LATE}"), Code::Ok);
            assert_eq!(i.eval_str(b"lb"), Code::Ok);
            assert_eq!(i.result_bytes(), b"LATE");
            // Renaming a non-alias, and renaming an alias somewhere harmless,
            // are both unaffected by the gate.
            assert_eq!(i.eval_str(b"rename := assign"), Code::Ok);
            assert_eq!(i.eval_str(b"assign zz 7"), Code::Ok);
            assert_eq!(i.result_bytes(), b"7");
        });
    }

    /// Definition-direction parity: a written trailing separator names
    /// the empty-string `{}` command inside its full qualifier chain — for
    /// `proc`, `rename`'s NEW name, and dispatch alike (`proc x:: {} {…}`
    /// defines `::x::` and `x::` invokes it; `rename foo x::` / `rename bar
    /// ::` rebind the `{}` command — all tclsh 8.6.16/9.0.4-pinned). The
    /// definition split must keep the empty tail, or the proc just defined
    /// could not be invoked.
    #[test]
    fn trailing_separator_definitions_match_resolution() {
        leak_free(|i| {
            // proc with a trailing separator: defined AND callable.
            assert_eq!(
                i.eval_str(b"namespace eval x {}; proc x:: {} { return EMPTYTAIL }"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"x::"), Code::Ok);
            assert_eq!(i.result_bytes(), b"EMPTYTAIL");
            // … and reachable via any separator-run spelling.
            assert_eq!(i.eval_str(b"::x::"), Code::Ok);
            assert_eq!(i.result_bytes(), b"EMPTYTAIL");
            // rename TO a trailing-separator name binds the `{}` command.
            assert_eq!(
                i.eval_str(b"proc foo {} { return F }; rename foo y::"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"y::"), Code::Ok);
            assert_eq!(i.result_bytes(), b"F");
            // rename to `::` binds the GLOBAL `{}` command.
            assert_eq!(
                i.eval_str(b"proc bar {} { return B }; rename bar ::"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"::"), Code::Ok);
            assert_eq!(i.result_bytes(), b"B");
            // The `{}` command can be renamed AWAY from its spelling too.
            assert_eq!(i.eval_str(b"rename y:: plain"), Code::Ok);
            assert_eq!(i.eval_str(b"plain"), Code::Ok);
            assert_eq!(i.result_bytes(), b"F");
            assert_eq!(i.eval_str(b"y::"), Code::Error);
            // Variables share the rule: `set vx:: V` writes the `{}` variable
            // in `::vx` (tclsh: `info vars ::vx::*` lists `::vx::`).
            assert_eq!(i.eval_str(b"namespace eval vx {}; set vx:: V"), Code::Ok);
            assert_eq!(i.eval_str(b"set vx::"), Code::Ok);
            assert_eq!(i.result_bytes(), b"V");
            // And ensembles: `namespace ensemble create -command ::e::` binds
            // the `{}` command in `::e` (tclsh-pinned: `e:: go` dispatches).
            assert_eq!(
                i.eval_str(
                    b"namespace eval e { proc go {} {return GO}; namespace export go; \
                       namespace ensemble create -command ::e:: }"
                ),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"e:: go"), Code::Ok);
            assert_eq!(i.result_bytes(), b"GO");
            // A proc named `:` inside a namespace named `:` has NO absolute
            // spelling (W314's case) but IS reachable by relative dispatch
            // from inside its namespace — `namespace eval : { : }` and
            // `namespace inscope : :` both invoke it, while every absolute
            // spelling misses (tclsh 8.6.16/9.0.4-pinned).
            assert_eq!(
                i.eval_str(b"namespace eval : { proc : args { return hello } }"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"namespace eval : { : }"), Code::Ok);
            assert_eq!(i.result_bytes(), b"hello");
            assert_eq!(i.eval_str(b"namespace inscope : :"), Code::Ok);
            assert_eq!(i.result_bytes(), b"hello");
            // Every all-colon spelling is separator runs around an empty
            // tail — it can only ever reach the GLOBAL `{}` command (bound
            // above by `rename bar ::`), never the `:`-named proc
            // (tclsh-pinned: prints B here, and errors once no global `{}`
            // exists).
            assert_eq!(i.eval_str(b":::::"), Code::Ok);
            assert_eq!(i.result_bytes(), b"B");
        });
    }
}

#[cfg(test)]
mod native_option_tables;

#[cfg(test)]
pub(crate) mod original_operand_tests {
    use super::*;
    use crate::obj::Owned;
    use std::cell::{Cell, RefCell};
    use tcl_runtime_api::NativeExecutionError;
    use tcl_syntax::raw_string::NativeValueAccessRefusal;

    #[derive(Clone, Copy)]
    pub(crate) enum Update {
        Host,
        Restore,
        Normal,
    }
    thread_local! {
        static INTERPRETER: RefCell<Option<Interp>> = const { RefCell::new(None) };
        static FIRST_CAUSE: RefCell<Option<NativeExecutionError>> = const { RefCell::new(None) };
        static MODE: Cell<Update> = const { Cell::new(Update::Host) };
        static FIRST: Cell<usize> = const { Cell::new(0) };
        static LATER: Cell<usize> = const { Cell::new(0) };
    }
    pub(crate) struct UpdaterOwner;
    impl UpdaterOwner {
        pub(crate) fn enter(interp: &Interp, mode: Update) -> Self {
            INTERPRETER.with(|slot| *slot.borrow_mut() = Some(interp.clone()));
            MODE.with(|selected| selected.set(mode));
            FIRST.with(|count| count.set(0));
            FIRST_CAUSE.with(|cause| cause.borrow_mut().take());
            LATER.with(|count| count.set(0));
            Self
        }
    }
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
    fn restore(interp: &mut Interp) {
        let original = interp.runtime_context();
        let mut changed = original.clone();
        changed.packages = vec![("alias-trace-original-stage".to_owned(), "1.0".to_owned())];
        interp.pin_context(&changed).unwrap();
        interp.pin_context(&original).unwrap();
    }
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
    extern "C" fn later_getter(original: *mut TclObj) {
        LATER.with(|count| count.set(count.get() + 1));
        unsafe { obj::set_native_updater_string_rep(original, b"LATER", false) };
    }
    static FIRST_TYPE: obj::TclObjType = obj::TclObjType {
        name: c"originalAliasTraceGetter".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: Some(first_getter),
        set_from_any_proc: None,
    };
    static LATER_TYPE: obj::TclObjType = obj::TclObjType {
        name: c"originalAliasTraceLaterGetter".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: Some(later_getter),
        set_from_any_proc: None,
    };
    pub(crate) fn first() -> Owned {
        Owned::fresh(obj::alloc_typed(&FIRST_TYPE, 0))
    }
    fn later() -> Owned {
        Owned::fresh(obj::alloc_typed(&LATER_TYPE, 0))
    }
    pub(crate) fn words(values: &[&[u8]]) -> Vec<Owned> {
        values
            .iter()
            .map(|bytes| Owned::fresh(obj::new_string_bytes(bytes)))
            .collect()
    }
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
    pub(crate) fn reached_variable(interp: &Interp, name: &[u8]) -> *mut TclObj {
        crate::vars::get(
            &interp.frames.borrow(),
            &interp.namespaces(),
            interp.current_ns(),
            name,
        )
        .expect("actual earlier callback variable effect remains")
    }
    pub(crate) fn seed(interp: &mut Interp) -> Owned {
        interp.set_result_bytes(b"PRIOR\0\xff");
        interp.set_return_state(2, Code::Other(7));
        interp.set_return_options(vec![(b"-original".to_vec(), b"OPTIONS".to_vec())]);
        Owned::retain(interp.get_obj_result())
    }
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
    fn original_child_lazy_result(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
        INTERPRETER.with(|slot| *slot.borrow_mut() = Some(interp.clone()));
        let result = first();
        interp.set_result(result.as_ptr());
        Code::Other(7)
    }

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
mod original_rename_operation_tests {
    use super::*;
    use crate::obj::Owned;
    use std::cell::{Cell, RefCell};
    use tcl_runtime_api::NativeExecutionError;
    use tcl_syntax::raw_string::NativeValueAccessRefusal;

    #[derive(Clone, Copy)]
    enum Callback {
        Normal,
        Host,
        Restore,
    }
    thread_local! {
        static MODE: Cell<Callback> = const { Cell::new(Callback::Normal) };
        static CALLS: Cell<usize> = const { Cell::new(0) };
        static FIRST: RefCell<Option<NativeExecutionError>> = const { RefCell::new(None) };
    }
    fn victim(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
        interp.set_result_bytes(b"VICTIM");
        Code::Ok
    }
    fn observer(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
        CALLS.with(|calls| calls.set(calls.get() + 1));
        let effect = Owned::fresh(obj::new_string_bytes(b"REACHED\0\xff"));
        interp
            .var_set(b"original_rename_effect", effect.as_ptr())
            .unwrap();
        match MODE.with(Cell::get) {
            Callback::Normal => interp.set_result_bytes(b"CALLBACK"),
            Callback::Host => {
                interp.refuse_host_command("first original rename observer");
                FIRST.with(|first| *first.borrow_mut() = interp.native_execution_refusal());
            }
            Callback::Restore => {
                let original = interp.runtime_context();
                let mut changed = original.clone();
                changed.packages = vec![("original-rename-observer".to_owned(), "1.0".to_owned())];
                interp.pin_context(&changed).unwrap();
                interp.pin_context(&original).unwrap();
            }
        }
        Code::Ok
    }

    #[test]
    fn original_rename_observers_keep_reached_effects_and_stop_later_result_publication() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Actual command-trace callbacks and native entry; no external Native order claim.
        for version in ["tcl8.6", "tcl9.1"] {
            for mode in [Callback::Normal, Callback::Host, Callback::Restore] {
                for deleting in [false, true] {
                    let mut interp = super::original_operand_tests::native(version);
                    interp.register_builtin(b"original_rename_victim", victim);
                    interp.register_builtin(b"original_rename_observer", observer);
                    assert_eq!(interp.eval_str(b"trace add command original_rename_victim {rename delete} original_rename_observer"), Code::Ok);
                    MODE.with(|selected| selected.set(mode));
                    CALLS.with(|calls| calls.set(0));
                    FIRST.with(|first| first.borrow_mut().take());
                    let owned = super::original_operand_tests::words(&[
                        b"rename",
                        b"original_rename_victim",
                        if deleting {
                            b""
                        } else {
                            b"original_rename_moved"
                        },
                    ]);
                    let argv: Vec<_> = owned.iter().map(Owned::as_ptr).collect();
                    let counts: Vec<_> = argv.iter().map(|&p| unsafe { (*p).ref_count }).collect();
                    let prior = super::original_operand_tests::seed(&mut interp);
                    let code = super::rename(&mut interp, &argv);
                    assert_eq!(CALLS.with(Cell::get), 1);
                    assert!(
                        interp
                            .resolve_cmd_token(b"original_rename_victim")
                            .is_none()
                    );
                    assert_eq!(
                        interp.resolve_cmd_token(b"original_rename_moved").is_some(),
                        !deleting
                    );
                    if !matches!(mode, Callback::Normal) {
                        assert!(interp.var_get(b"original_rename_effect").is_none());
                    }
                    let effect = super::original_operand_tests::reached_variable(
                        &interp,
                        b"original_rename_effect",
                    );
                    assert_eq!(obj::bytes_of(effect), b"REACHED\0\xff");
                    assert_eq!(
                        argv.iter()
                            .map(|&p| unsafe { (*p).ref_count })
                            .collect::<Vec<_>>(),
                        counts
                    );
                    assert_eq!(interp.entered_native_operation_count(), 0);
                    if matches!(mode, Callback::Normal) {
                        assert_eq!(code, Code::Ok);
                        assert!(interp.native_execution_refusal().is_none());
                        assert_eq!(obj::bytes_of(interp.get_obj_result()), b"");
                    } else {
                        assert_eq!(code, Code::Error);
                        let expected = match mode {
                            Callback::Host => FIRST.with(|first| {
                                first.borrow().clone().expect("actual callback first Host")
                            }),
                            Callback::Restore => NativeExecutionError::ValueAccessRefusal(
                                NativeValueAccessRefusal::CommandProtocolUnavailable(
                                    "stale entered native operation",
                                ),
                            ),
                            Callback::Normal => unreachable!(),
                        };
                        assert_eq!(interp.native_execution_refusal(), Some(expected));
                        assert_eq!(interp.get_obj_result(), prior.as_ptr());
                        assert_eq!(obj::bytes_of(prior.as_ptr()), b"PRIOR\0\xff");
                        assert_eq!(interp.pending_return_level(), 2);
                        assert_eq!(interp.pending_return_code(), Code::Other(7));
                        assert_eq!(interp.pending_return_option_objects().len(), 1);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod original_alias_storage_tests {
    use super::*;
    use crate::interp::ObjCommand;
    use crate::obj::Owned;
    use std::cell::{Cell, RefCell};
    use std::collections::HashMap;

    thread_local! {
        static UPDATES: RefCell<[usize; 4]> = const { RefCell::new([0; 4]) };
        static FREES: RefCell<[usize; 4]> = const { RefCell::new([0; 4]) };
        static GETTERS: RefCell<Vec<(usize, i64)>> = const { RefCell::new(Vec::new()) };
        static MEMBERS: RefCell<Vec<*mut TclObj>> = const { RefCell::new(Vec::new()) };
        static CALLBACK_REFS: RefCell<Vec<Vec<i64>>> = const { RefCell::new(Vec::new()) };
        static CALLS: Cell<usize> = const { Cell::new(0) };
    }
    const PAYLOADS: [&[u8]; 4] = [b"aliasProbe", b"capture", b"alpha \0 x", b"beta y"];
    extern "C" fn update(original: *mut TclObj) {
        let role = usize::try_from(obj::internal_rep(original)).unwrap();
        UPDATES.with(|counts| counts.borrow_mut()[role] += 1);
        // SAFETY: the actual updater owns this live original header.
        GETTERS.with(|rows| {
            rows.borrow_mut()
                .push((role, unsafe { (*original).ref_count } as i64))
        });
        unsafe { obj::set_native_updater_string_rep(original, PAYLOADS[role], false) };
    }
    extern "C" fn free(original: *mut TclObj) {
        let role = usize::try_from(obj::internal_rep(original)).unwrap();
        FREES.with(|counts| counts.borrow_mut()[role] += 1);
    }
    static TYPE: obj::TclObjType = obj::TclObjType {
        name: c"originalAliasStorageOperand".as_ptr(),
        free_int_rep_proc: Some(free),
        dup_int_rep_proc: None,
        update_string_proc: Some(update),
        set_from_any_proc: None,
    };
    unsafe extern "C" fn capture(
        _: *mut std::ffi::c_void,
        interp: *mut Interp,
        count: std::ffi::c_int,
        argv: *const *mut TclObj,
    ) -> std::ffi::c_int {
        // SAFETY: this actual registered command receives its live owner/argv.
        let interp = unsafe { &mut *interp };
        let argv = unsafe { std::slice::from_raw_parts(argv, usize::try_from(count).unwrap()) };
        capture_original(interp, argv).as_int() as i32
    }
    // Jim's public command registration owns a fresh CString key; it does not
    // borrow the C CreateObjCommand publication address or callback ABI.
    fn capture_jim(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
        capture_original(interp, argv)
    }
    fn capture_original(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
        assert_eq!(argv.len(), 4);
        MEMBERS.with(|members| assert_eq!(&argv[..3], members.borrow().as_slice()));
        CALLBACK_REFS.with(|rows| {
            rows.borrow_mut().push(
                argv[..3]
                    .iter()
                    .map(|&p| unsafe { (*p).ref_count } as i64)
                    .collect(),
            )
        });
        CALLS.with(|calls| calls.set(calls.get() + 1));
        let arguments = match OriginalArguments::enter(interp, argv) {
            Ok(arguments) => arguments,
            Err(error) => return interp.report_cmd_error(error),
        };
        for index in [2, 1] {
            let bytes = arguments.bytes(index).expect("original callback getter");
            assert_eq!(bytes.as_ref(), PAYLOADS[index + 1]);
        }
        interp.set_result_bytes(b"SEEN");
        arguments.finish(Code::Ok)
    }
    fn reset() {
        UPDATES.with(|v| *v.borrow_mut() = [0; 4]);
        FREES.with(|v| *v.borrow_mut() = [0; 4]);
        GETTERS.with(|v| v.borrow_mut().clear());
        MEMBERS.with(|v| v.borrow_mut().clear());
        CALLBACK_REFS.with(|v| v.borrow_mut().clear());
        CALLS.with(|v| v.set(0));
    }
    fn fields<'a>(
        stream: &'a str,
        kind: &str,
        case: usize,
        label: Option<&str>,
    ) -> HashMap<&'a str, &'a str> {
        let prefix = format!("{kind}|{case}|");
        let line = stream
            .lines()
            .find(|line| {
                line.starts_with(&prefix)
                    && label.is_none_or(|label| {
                        line.strip_prefix(&prefix).unwrap().split('|').next() == Some(label)
                    })
            })
            .expect("actual pinned observer row");
        line.split('|')
            .filter_map(|field| field.split_once('='))
            .collect()
    }
    fn number(fields: &HashMap<&str, &str>, key: &str) -> i64 {
        fields[key].parse().unwrap()
    }
    fn fixtures() -> [(&'static str, &'static str); 6] {
        [
            (
                "tcl8.4",
                include_str!(
                    "../tests/data/native_alias_object_prefix_capture_original/415/providers/8.4.20/execute.stdout"
                ),
            ),
            (
                "tcl8.5",
                include_str!(
                    "../tests/data/native_alias_object_prefix_capture_original/415/providers/8.5.19/execute.stdout"
                ),
            ),
            (
                "tcl8.6",
                include_str!(
                    "../tests/data/native_alias_object_prefix_capture_original/415/providers/8.6.18/execute.stdout"
                ),
            ),
            (
                "tcl9.0",
                include_str!(
                    "../tests/data/native_alias_object_prefix_capture_original/415/providers/9.0.4/execute.stdout"
                ),
            ),
            (
                "tcl9.1",
                include_str!(
                    "../tests/data/native_alias_object_prefix_capture_original/415/providers/9.1.0/execute.stdout"
                ),
            ),
            (
                "jim",
                include_str!(
                    "../tests/data/native_alias_object_prefix_capture_original/415/providers/jim/execute.stdout"
                ),
            ),
        ]
    }
    fn query(interp: &mut Interp, original_name: *mut TclObj, jim: bool, child: bool) -> Owned {
        let jim_words: &[&[u8]] = &[b"info", b"alias"];
        let c_words: &[&[u8]] = &[b"interp", b"alias", if child { b"child" } else { b"" }];
        let words = super::original_operand_tests::words(if jim { jim_words } else { c_words });
        let mut argv = words.iter().map(Owned::as_ptr).collect::<Vec<_>>();
        argv.push(original_name);
        let code = if jim {
            interp.dispatch(&argv)
        } else {
            super::interp_cmd(interp, &argv)
        };
        assert_eq!(code, Code::Ok);
        Owned::retain(interp.get_obj_result())
    }
    fn check_row(stream: &str, case: usize, label: &str, originals: &[Owned]) {
        let row = fields(stream, "ROW", case, Some(label));
        for (role, original) in originals.iter().enumerate() {
            assert_eq!(
                unsafe { (*original.as_ptr()).ref_count } as i64,
                number(&row, &format!("r{role}_refs")),
                "{label} role{role}"
            );
            assert_eq!(
                obj::has_string_rep(original.as_ptr()),
                number(&row, &format!("r{role}_string")) != 0,
                "{label} role{role}"
            );
            assert_eq!(
                UPDATES.with(|v| v.borrow()[role]) as i64,
                number(&row, &format!("r{role}_update")),
                "{label} role{role}"
            );
        }
    }

    #[test]
    fn original_alias_capture_and_query_match_the_measured_member_and_header_recipe() {
        // naming.alias.original-object-prefix-capture-query-invocation-lifetime
        // docs/design/analysis/name-resolution-proofs/alias-original-object-prefix-capture-query-invocation-lifetime.md
        // Bounded exact physical fields; Jim body-only name updater refs are not
        // compared with its public EvalObjVector registration invocation.
        for (version, stream) in fixtures() {
            for case in [1, 2, 3, 6] {
                if version == "jim" && case == 6 {
                    continue;
                }
                reset();
                let mut interp = super::original_operand_tests::native(version);
                if version == "jim" {
                    interp.register_builtin(b"capture", capture_jim);
                    assert!(interp.native_execution_refusal().is_none());
                } else {
                    interp
                        .create_obj_command(
                            b"capture",
                            ObjCommand::new(capture, std::ptr::null_mut(), None),
                        )
                        .expect("actual original C object command");
                }
                let child = case == 6;
                if child {
                    assert_eq!(interp.eval_str(b"interp create child"), Code::Ok);
                }
                let originals = (0..4)
                    .map(|role| Owned::fresh(obj::alloc_typed(&TYPE, role)))
                    .collect::<Vec<_>>();
                MEMBERS.with(|members| {
                    *members.borrow_mut() = originals[1..].iter().map(Owned::as_ptr).collect()
                });
                let jim = version == "jim";
                let jim_words: &[&[u8]] = &[b"alias"];
                let c_words: &[&[u8]] =
                    &[b"interp", b"alias", if child { b"child" } else { b"" }];
                let words =
                    super::original_operand_tests::words(if jim { jim_words } else { c_words });
                let target_path = Owned::fresh(obj::new_string_bytes(b""));
                let mut argv = words.iter().map(Owned::as_ptr).collect::<Vec<_>>();
                argv.push(originals[0].as_ptr());
                if !jim {
                    argv.push(Owned::as_ptr(&target_path));
                }
                argv.extend(originals[1..].iter().map(Owned::as_ptr));
                assert_eq!(
                    if jim {
                        super::jim_alias(&mut interp, &argv)
                    } else {
                        super::interp_cmd(&mut interp, &argv)
                    },
                    Code::Ok
                );
                assert_eq!(interp.get_obj_result(), originals[0].as_ptr());
                check_row(stream, case, "after-capture", &originals);
                let actual_getters = GETTERS.with(|v| v.borrow().clone());
                let expected = stream
                    .lines()
                    .filter(|line| {
                        line.starts_with(&format!("EVENT|{case}|")) && line.contains("|update|")
                    })
                    .take(if jim { 1 } else { 2 })
                    .map(|line| {
                        let row = line
                            .split('|')
                            .filter_map(|field| field.split_once('='))
                            .collect::<HashMap<_, _>>();
                        (
                            usize::try_from(number(&row, "role")).unwrap(),
                            number(&row, "refs"),
                        )
                    })
                    .collect::<Vec<_>>();
                assert_eq!(
                    actual_getters.iter().map(|v| v.0).collect::<Vec<_>>(),
                    expected.iter().map(|v| v.0).collect::<Vec<_>>()
                );
                if !jim {
                    assert_eq!(actual_getters, expected);
                }
                interp.set_result_bytes(b"");
                let mut queries = Vec::new();
                if case != 3 {
                    let first = query(&mut interp, originals[0].as_ptr(), jim, child);
                    assert_eq!(
                        ValueOps::list_elements(&mut interp, &first.as_ptr()).unwrap(),
                        originals[1..].iter().map(Owned::as_ptr).collect::<Vec<_>>()
                    );
                    check_row(stream, case, "query-one-raw", &originals);
                    let second = query(&mut interp, originals[0].as_ptr(), jim, child);
                    let measured = fields(stream, "LIST", case, Some("query-two-raw"));
                    assert_eq!(
                        first.as_ptr() == second.as_ptr(),
                        number(&measured, "same_other") != 0
                    );
                    assert_eq!(
                        ValueOps::list_elements(&mut interp, &second.as_ptr()).unwrap(),
                        originals[1..].iter().map(Owned::as_ptr).collect::<Vec<_>>()
                    );
                    check_row(stream, case, "query-two-raw", &originals);
                    if case == 2 {
                        let bytes = interp.native_object_string_bytes(first.as_ptr()).unwrap();
                        let present = fields(stream, "PRESENT", case, None);
                        assert_eq!(
                            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
                            present["hex"]
                        );
                        let measured = fields(
                            stream,
                            "LIST",
                            case,
                            Some("query-two-after-other-presentation"),
                        );
                        assert_eq!(
                            obj::has_string_rep(second.as_ptr()),
                            number(&measured, "string") != 0
                        );
                    }
                    queries.extend([first, second]);
                }
                let tail = Owned::fresh(obj::new_string_bytes(b"TAIL"));
                for label in ["first-invocation", "second-invocation"] {
                    let argv = [originals[0].as_ptr(), tail.as_ptr()];
                    let code = if child {
                        interp
                            .with_child(b"child", |owner| owner.dispatch(&argv))
                            .unwrap()
                    } else {
                        interp.dispatch(&argv)
                    };
                    assert_eq!(
                        code,
                        Code::Ok,
                        "{version}/{case}/{label}: parent cause {:?}, parent resident {:?}, child cause/resident {:?}",
                        interp.native_execution_refusal(),
                        obj::native_object_snapshot(interp.get_obj_result())
                            .map(|snapshot| snapshot.resident),
                        if child {
                            interp.with_child(b"child", |owner| {
                                (
                                    owner.native_execution_refusal(),
                                    obj::native_object_snapshot(owner.get_obj_result())
                                        .map(|snapshot| snapshot.resident),
                                )
                            })
                        } else {
                            None
                        }
                    );
                    check_row(stream, case, label, &originals);
                }
                assert_eq!(CALLS.with(Cell::get), 2);
                let measured = fields(stream, "ROW", case, Some("capture-before-getters"));
                for refs in CALLBACK_REFS.with(|v| v.borrow().clone()) {
                    for role in 0..3 {
                        assert_eq!(
                            refs[role],
                            number(&measured, &format!("r{}_refs", role + 1))
                        );
                    }
                }
                let prefix_updates = GETTERS.with(|v| {
                    v.borrow()
                        .iter()
                        .filter(|(role, _)| *role >= 2)
                        .copied()
                        .collect::<Vec<_>>()
                });
                let expected = stream
                    .lines()
                    .filter(|line| {
                        line.starts_with(&format!("EVENT|{case}|")) && line.contains("|update|")
                    })
                    .filter_map(|line| {
                        let row = line
                            .split('|')
                            .filter_map(|field| field.split_once('='))
                            .collect::<HashMap<_, _>>();
                        let role = usize::try_from(number(&row, "role")).unwrap();
                        (role >= 2).then(|| (role, number(&row, "refs")))
                    })
                    .collect::<Vec<_>>();
                assert_eq!(prefix_updates, expected);
                interp.set_result_bytes(b"");
                drop(originals);
                let deleted = if child {
                    interp
                        .with_child(b"child", |owner| owner.delete_command(b"aliasProbe"))
                        .unwrap()
                } else {
                    interp.delete_command(b"aliasProbe")
                };
                assert!(deleted);
                if !queries.is_empty() {
                    let measured =
                        fields(stream, "LIST", case, Some("query-two-held-after-delete"));
                    let members =
                        ValueOps::list_elements(&mut interp, &queries[1].as_ptr()).unwrap();
                    for (index, &member) in members.iter().enumerate() {
                        assert_eq!(
                            unsafe { (*member).ref_count } as i64,
                            number(&measured, &format!("m{index}_refs"))
                        );
                        assert_eq!(
                            interp.native_object_string_bytes(member).unwrap().as_ref(),
                            PAYLOADS[index + 1]
                        );
                    }
                    drop(queries.remove(0));
                    assert_eq!(FREES.with(|v| v.borrow()[2..].to_vec()), vec![0, 0]);
                    drop(queries);
                }
                assert_eq!(FREES.with(|v| v.borrow()[2..].to_vec()), vec![1, 1]);
                assert_eq!(UPDATES.with(|v| v.borrow()[2..].to_vec()), vec![1, 1]);
                assert_eq!(interp.entered_native_operation_count(), 0);
            }
        }
    }

    #[test]
    fn original_alias_query_retains_unread_members_until_the_last_query_is_released() {
        // naming.alias.original-object-prefix-capture-query-invocation-lifetime
        // docs/design/analysis/name-resolution-proofs/alias-original-object-prefix-capture-query-invocation-lifetime.md
        for (version, stream) in fixtures() {
            reset();
            let mut interp = super::original_operand_tests::native(version);
            let originals = (0..4)
                .map(|role| Owned::fresh(obj::alloc_typed(&TYPE, role)))
                .collect::<Vec<_>>();
            let jim = version == "jim";
            let words = super::original_operand_tests::words(if jim {
                &[b"alias"]
            } else {
                &[b"interp", b"alias", b""]
            });
            let mut argv = words.iter().map(Owned::as_ptr).collect::<Vec<_>>();
            argv.push(originals[0].as_ptr());
            if !jim {
                argv.push(words[2].as_ptr());
            }
            argv.extend(originals[1..].iter().map(Owned::as_ptr));
            assert_eq!(
                if jim {
                    super::jim_alias(&mut interp, &argv)
                } else {
                    super::interp_cmd(&mut interp, &argv)
                },
                Code::Ok
            );
            let first = query(&mut interp, originals[0].as_ptr(), jim, false);
            let second = query(&mut interp, originals[0].as_ptr(), jim, false);
            let measured = fields(stream, "LIST", 1, Some("query-two-raw"));
            assert_eq!(
                first.as_ptr() == second.as_ptr(),
                number(&measured, "same_other") != 0
            );
            interp.set_result_bytes(b"");
            drop(originals);
            assert!(interp.delete_command(b"aliasProbe"));
            let members = ValueOps::list_elements(&mut interp, &second.as_ptr()).unwrap();
            assert_eq!(UPDATES.with(|v| v.borrow()[2..].to_vec()), vec![0, 0]);
            assert!(
                members[1..]
                    .iter()
                    .all(|&original| !obj::has_string_rep(original))
            );
            drop(first);
            assert_eq!(FREES.with(|v| v.borrow()[2..].to_vec()), vec![0, 0]);
            drop(second);
            assert_eq!(FREES.with(|v| v.borrow()[2..].to_vec()), vec![1, 1]);
            assert_eq!(UPDATES.with(|v| v.borrow()[2..].to_vec()), vec![0, 0]);
        }
    }

    #[test]
    fn original_child_handle_alias_retains_lazy_parent_members_and_selected_name_role() {
        // naming.alias.original-object-prefix-capture-query-invocation-lifetime
        // docs/design/analysis/name-resolution-proofs/alias-original-object-prefix-capture-query-invocation-lifetime.md
        // C aliases own the supplied name; Jim_RegisterCmd owns a fresh child
        // key. These are software role controls, not unmeasured native counts.
        for version in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            reset();
            let jim = version == "jim";
            let mut interp = super::original_operand_tests::native(version);
            if jim {
                interp.register_builtin(b"capture", capture_jim);
                assert!(interp.native_execution_refusal().is_none());
            } else {
                interp
                    .create_obj_command(
                        b"capture",
                        ObjCommand::new(capture, std::ptr::null_mut(), None),
                    )
                    .expect("actual parent C object command");
            }
            let child = if jim {
                interp.create_jim_child()
            } else {
                interp.create_child(Some(b"child".to_vec()))
            };
            let originals = (0..4)
                .map(|role| Owned::fresh(obj::alloc_typed(&TYPE, role)))
                .collect::<Vec<_>>();
            MEMBERS.with(|members| {
                *members.borrow_mut() = originals[1..].iter().map(Owned::as_ptr).collect();
            });
            let words = super::original_operand_tests::words(&[&child, b"alias"]);
            let mut argv = words.iter().map(Owned::as_ptr).collect::<Vec<_>>();
            argv.extend(originals.iter().map(Owned::as_ptr));
            assert_eq!(interp.dispatch(&argv), Code::Ok);
            if jim {
                assert_eq!(interp.result_bytes(), b"");
            } else {
                assert_eq!(interp.get_obj_result(), originals[0].as_ptr());
            }
            assert_eq!(
                UPDATES.with(|counts| *counts.borrow()),
                if jim { [1, 0, 0, 0] } else { [1, 1, 0, 0] }
            );
            assert_eq!(
                GETTERS.with(|rows| rows.borrow().iter().map(|row| row.0).collect::<Vec<_>>()),
                if jim { vec![0] } else { vec![1, 0] }
            );
            interp.set_result_bytes(b"");
            assert_eq!(
                unsafe { (*originals[0].as_ptr()).ref_count },
                if jim { 1 } else { 2 }
            );
            let prefix = interp
                .with_child(&child, |owner| {
                    let Some(crate::interp::Command::ParentAlias {
                        original_prefix,
                        registration_name,
                        ..
                    }) = owner
                        .namespaces()
                        .command_in(crate::namespace::GLOBAL, b"aliasProbe")
                    else {
                        panic!("actual original child alias record");
                    };
                    assert_eq!(registration_name.is_some(), !jim);
                    original_prefix
                })
                .unwrap();
            assert_eq!(
                ValueOps::list_elements(&mut interp, &prefix.as_ptr()).unwrap(),
                originals[1..].iter().map(Owned::as_ptr).collect::<Vec<_>>()
            );
            for member in &originals[1..] {
                assert_eq!(unsafe { (*member.as_ptr()).ref_count }, 2);
            }
            assert!(!obj::has_string_rep(originals[2].as_ptr()));
            assert!(!obj::has_string_rep(originals[3].as_ptr()));
            if !jim {
                let first = interp
                    .with_child(&child, |owner| {
                        owner.alias_query_object(b"aliasProbe").unwrap().unwrap()
                    })
                    .unwrap();
                let second = interp
                    .with_child(&child, |owner| {
                        owner.alias_query_object(b"aliasProbe").unwrap().unwrap()
                    })
                    .unwrap();
                assert_ne!(first.as_ptr(), second.as_ptr());
                assert_ne!(first.as_ptr(), prefix.as_ptr());
                assert_eq!(
                    ValueOps::list_elements(&mut interp, &first.as_ptr()).unwrap(),
                    originals[1..].iter().map(Owned::as_ptr).collect::<Vec<_>>()
                );
                drop(first);
                drop(second);
            }
            let call = super::original_operand_tests::words(&[b"aliasProbe", b"TAIL"]);
            let call_argv = call.iter().map(Owned::as_ptr).collect::<Vec<_>>();
            assert_eq!(
                interp
                    .with_child(&child, |owner| owner.dispatch(&call_argv))
                    .unwrap(),
                Code::Ok,
                "{version}: parent cause {:?}, parent resident {:?}, child cause/resident {:?}",
                interp.native_execution_refusal(),
                obj::native_object_snapshot(interp.get_obj_result())
                    .map(|snapshot| snapshot.resident),
                interp.with_child(&child, |owner| {
                    (
                        owner.native_execution_refusal(),
                        obj::native_object_snapshot(owner.get_obj_result())
                            .map(|snapshot| snapshot.resident),
                    )
                })
            );
            assert_eq!(CALLS.with(Cell::get), 1);
            assert_eq!(UPDATES.with(|counts| *counts.borrow()), [1, 1, 1, 1]);
            let getter_roles =
                GETTERS.with(|rows| rows.borrow().iter().map(|row| row.0).collect::<Vec<_>>());
            assert_eq!(&getter_roles[getter_roles.len() - 2..], &[3, 2]);
            assert_eq!(
                MEMBERS.with(|members| members.borrow().clone()),
                originals[1..].iter().map(Owned::as_ptr).collect::<Vec<_>>()
            );
            interp
                .with_child(&child, |owner| {
                    assert_eq!(owner.result_bytes(), b"SEEN");
                    owner.delete_command(b"aliasProbe");
                })
                .unwrap();
            drop(prefix);
            assert_eq!(unsafe { (*originals[0].as_ptr()).ref_count }, 1);
            for member in &originals[1..] {
                assert_eq!(unsafe { (*member.as_ptr()).ref_count }, 1);
            }
        }
    }
}

#[cfg(test)]
mod original_alias_key_tests {
    use super::*;
    use crate::obj::Owned;
    use std::cell::{Cell, RefCell};
    use tcl_runtime_api::NativeExecutionError;

    thread_local! {
        static OWNER: RefCell<Option<Interp>> = const { RefCell::new(None) };
        static RENAME: Cell<bool> = const { Cell::new(false) };
        static RETIRE: Cell<u8> = const { Cell::new(0) };
        static RETIRE_ROLE: Cell<usize> = const { Cell::new(0) };
        static ORDER: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
        static GETTERS: RefCell<Vec<(usize, i64)>> = const { RefCell::new(Vec::new()) };
        static FIRST: RefCell<Option<NativeExecutionError>> = const { RefCell::new(None) };
    }
    fn payload(role: usize) -> &'static [u8] {
        match role {
            0 => b"aliasKeyProbe",
            1 => b"list",
            2 => b"first \0 member",
            3 => b"last member",
            4 if RENAME.with(Cell::get) => b"aliasKeyMoved",
            4 => b"aliasKeyProbe",
            _ => unreachable!("known fixed original role"),
        }
    }
    extern "C" fn update(original: *mut TclObj) {
        let role = usize::try_from(obj::internal_rep(original)).unwrap();
        GETTERS.with(|rows| {
            rows.borrow_mut()
                .push((role, unsafe { (*original).ref_count } as i64))
        });
        unsafe { obj::set_native_updater_string_rep(original, payload(role), false) };
    }
    extern "C" fn free(original: *mut TclObj) {
        let role = usize::try_from(obj::internal_rep(original)).unwrap();
        ORDER.with(|rows| rows.borrow_mut().push(role));
        if role == RETIRE_ROLE.with(Cell::get) && RETIRE.with(Cell::get) != 0 {
            let mut interp = OWNER.with(|owner| owner.borrow().as_ref().unwrap().clone());
            // A real key destructor runs outside the Namespace borrow. The
            // original entry is already unlinked; later slot owners are distinct.
            assert!(interp.resolve_cmd_token(b"aliasKeyProbe").is_none());
            if role == 0 {
                assert_eq!(ORDER.with(|rows| rows.borrow().clone()), vec![0]);
            } else {
                // Replacement retires only the old value. Its occupied original
                // key remains owned even while the binding payload is absent.
                let key = interp
                    .namespaces()
                    .original_jim_command_key(crate::namespace::GLOBAL, b"aliasKeyProbe")
                    .unwrap();
                assert_eq!(obj::bytes_of(key), b"aliasKeyProbe");
                assert_eq!(unsafe { (*key).ref_count }, 1);
            }
            match RETIRE.with(Cell::get) {
                1 => {
                    interp.refuse_host_command("original Jim key destructor first cause");
                    FIRST.with(|first| *first.borrow_mut() = interp.native_execution_refusal());
                }
                2 => {
                    let original = interp.runtime_context();
                    let mut changed = original.clone();
                    changed.packages =
                        vec![("original-key-retirement".to_owned(), "1.0".to_owned())];
                    interp.pin_context(&changed).unwrap();
                    interp.pin_context(&original).unwrap();
                }
                3 => interp.register_builtin(b"aliasKeyProbe", replacement),
                _ => unreachable!(),
            }
        }
    }
    fn replacement(interp: &mut Interp, _: &[*mut TclObj]) -> Code {
        interp.set_result_bytes(b"REPLACEMENT");
        Code::Ok
    }
    static TYPE: obj::TclObjType = obj::TclObjType {
        name: c"originalAliasKeyOperand".as_ptr(),
        free_int_rep_proc: Some(free),
        dup_int_rep_proc: None,
        update_string_proc: Some(update),
        set_from_any_proc: None,
    };
    fn reset() {
        OWNER.with(|owner| owner.borrow_mut().take());
        RENAME.with(|rename| rename.set(false));
        RETIRE.with(|mode| mode.set(0));
        RETIRE_ROLE.with(|role| role.set(0));
        ORDER.with(|rows| rows.borrow_mut().clear());
        GETTERS.with(|rows| rows.borrow_mut().clear());
        FIRST.with(|first| first.borrow_mut().take());
    }
    fn originals() -> Vec<Owned> {
        (0..5)
            .map(|role| Owned::fresh(obj::alloc_typed(&TYPE, role)))
            .collect()
    }
    fn define(interp: &mut Interp, originals: &[Owned], name: usize, jim: bool) {
        let jim_words = [b"alias".as_slice()];
        let c_words = [b"interp".as_slice(), b"alias", b"", b"unused", b""];
        let words = super::original_operand_tests::words(if jim {
            &jim_words
        } else {
            &c_words
        });
        let mut argv = words.iter().map(Owned::as_ptr).collect::<Vec<_>>();
        if jim {
            argv.push(originals[name].as_ptr());
        } else {
            argv[3] = originals[name].as_ptr();
        }
        argv.extend(originals[1..4].iter().map(Owned::as_ptr));
        assert_eq!(
            if jim {
                super::jim_alias(interp, &argv)
            } else {
                super::interp_cmd(interp, &argv)
            },
            Code::Ok
        );
        interp.set_result_bytes(b"");
    }
    fn rows() -> [(&'static str, &'static str); 6] {
        [
            ("tcl8.4", include_str!("../tests/data/native_alias_command_key_replacement_child_capture_rename_original/421b/providers/8.4.20/execute.stdout")),
            ("tcl8.5", include_str!("../tests/data/native_alias_command_key_replacement_child_capture_rename_original/421b/providers/8.5.19/execute.stdout")),
            ("tcl8.6", include_str!("../tests/data/native_alias_command_key_replacement_child_capture_rename_original/421b/providers/8.6.18/execute.stdout")),
            ("tcl9.0", include_str!("../tests/data/native_alias_command_key_replacement_child_capture_rename_original/421b/providers/9.0.4/execute.stdout")),
            ("tcl9.1", include_str!("../tests/data/native_alias_command_key_replacement_child_capture_rename_original/421b/providers/9.1.0/execute.stdout")),
            ("jim", include_str!("../tests/data/native_alias_command_key_replacement_child_capture_rename_original/421b/providers/jim/execute.stdout")),
        ]
    }
    fn expected(stream: &str, case: usize, label: &str, role: usize, field: &str) -> i64 {
        let prefix = format!("ROW|{case}|{label}|");
        let row = stream
            .lines()
            .find(|row| row.starts_with(&prefix))
            .expect("actual pinned observer stage");
        let key = format!("r{role}_{field}=");
        row.split('|')
            .find_map(|field| field.strip_prefix(&key))
            .unwrap()
            .parse()
            .unwrap()
    }

    #[test]
    fn original_alias_replacement_keeps_measured_name_key_and_query_roles() {
        // naming.alias.original-command-key-replacement-child-capture-and-rename
        // docs/design/analysis/name-resolution-proofs/alias-original-command-key-replacement-child-capture-and-rename.md
        // Only observed name refs and query header/member roles are compared;
        // no Jim body-only updater refcount or general frame claim is inferred.
        for (version, stream) in rows() {
            reset();
            let jim = version == "jim";
            let mut interp = super::original_operand_tests::native(version);
            let originals = originals();
            define(&mut interp, &originals, 0, jim);
            let first_query = interp.alias_query_object(payload(0)).unwrap().unwrap();
            define(&mut interp, &originals, 4, jim);
            for role in [0, 4] {
                assert_eq!(
                    unsafe { (*originals[role].as_ptr()).ref_count } as i64,
                    expected(
                        stream,
                        7,
                        "replacement-name-after-result-clear",
                        role,
                        "refs"
                    ),
                    "{version} role {role}"
                );
            }
            let current_query = interp.alias_query_object(payload(0)).unwrap().unwrap();
            assert_ne!(first_query.as_ptr(), current_query.as_ptr());
            let selected = interp.alias_query_object(payload(0)).unwrap().unwrap();
            assert_eq!(selected.as_ptr() == current_query.as_ptr(), jim);
            let members =
                tcl_syntax::value::ValueOps::list_elements(&mut interp, &current_query.as_ptr())
                    .unwrap();
            assert_eq!(
                members,
                originals[1..4]
                    .iter()
                    .map(Owned::as_ptr)
                    .collect::<Vec<_>>()
            );
            if jim {
                assert_eq!(
                    interp
                        .namespaces()
                        .original_jim_command_key(crate::namespace::GLOBAL, payload(0)),
                    Some(originals[0].as_ptr())
                );
            }
            assert!(interp.delete_command(payload(0)));
            for role in [0, 4] {
                assert_eq!(unsafe { (*originals[role].as_ptr()).ref_count }, 1);
            }
            assert_eq!(
                tcl_syntax::value::ValueOps::list_elements(&mut interp, &first_query.as_ptr()).unwrap(),
                members
            );
            assert!(interp.native_execution_refusal().is_none());
        }
    }

    #[test]
    fn original_alias_rename_keeps_measured_record_and_destination_key_roles() {
        // naming.alias.original-command-key-replacement-child-capture-and-rename
        // docs/design/analysis/name-resolution-proofs/alias-original-command-key-replacement-child-capture-and-rename.md
        for (version, stream) in rows() {
            reset();
            RENAME.with(|rename| rename.set(true));
            let jim = version == "jim";
            let mut interp = super::original_operand_tests::native(version);
            let originals = originals();
            define(&mut interp, &originals, 0, jim);
            let words = super::original_operand_tests::words(&[b"rename"]);
            assert_eq!(
                super::rename(
                    &mut interp,
                    &[
                        words[0].as_ptr(),
                        originals[0].as_ptr(),
                        originals[4].as_ptr()
                    ]
                ),
                Code::Ok
            );
            for role in [0, 4] {
                assert_eq!(
                    unsafe { (*originals[role].as_ptr()).ref_count } as i64,
                    expected(stream, 10, "rename-name-after-result-clear", role, "refs"),
                    "{version} role {role}"
                );
            }
            if jim {
                assert_eq!(
                    interp
                        .namespaces()
                        .original_jim_command_key(crate::namespace::GLOBAL, payload(4)),
                    Some(originals[4].as_ptr())
                );
                assert!(interp
                    .namespaces()
                    .original_jim_command_key(crate::namespace::GLOBAL, payload(0))
                    .is_none());
            }
            assert!(interp.delete_command(payload(4)));
            for role in [0, 4] {
                assert_eq!(unsafe { (*originals[role].as_ptr()).ref_count }, 1);
            }
        }
    }

    #[test]
    fn original_jim_key_retirement_runs_before_value_cleanup_without_adopting_replacement() {
        // naming.alias.original-command-key-replacement-child-capture-and-rename
        // docs/design/analysis/name-resolution-proofs/alias-original-command-key-replacement-child-capture-and-rename.md
        // Real destructor callbacks are software controls of cleanup/reentry.
        // Native421b did not execute these Host/context callback modes.
        for mode in [1, 2, 3] {
            reset();
            let mut interp = super::original_operand_tests::native("jim");
            let originals = originals();
            define(&mut interp, &originals, 0, true);
            let prior = super::original_operand_tests::seed(&mut interp);
            drop(originals);
            ORDER.with(|rows| rows.borrow_mut().clear());
            OWNER.with(|owner| *owner.borrow_mut() = Some(interp.clone()));
            RETIRE.with(|selected| selected.set(mode));
            let scope =
                crate::interp::native_operation_currency::NativeOperationScope::enter(&interp)
                    .unwrap();
            assert!(interp.delete_command(b"aliasKeyProbe"));
            let order = ORDER.with(|rows| rows.borrow().clone());
            assert_eq!(order.first(), Some(&0));
            let mut members = order[1..].to_vec();
            members.sort_unstable();
            assert_eq!(members, vec![1, 2, 3]);
            if mode == 3 {
                assert!(scope.currency().ensure_current_or_refuse().is_ok());
                assert!(interp.resolve_cmd_token(b"aliasKeyProbe").is_some());
                assert_ne!(
                    interp
                        .namespaces()
                        .original_jim_command_key(crate::namespace::GLOBAL, b"aliasKeyProbe"),
                    None
                );
            } else {
                assert!(scope.currency().ensure_current_or_refuse().is_err());
                if mode == 1 {
                    assert_eq!(
                        interp.native_execution_refusal(),
                        FIRST.with(|first| first.borrow().clone())
                    );
                }
                assert_eq!(interp.get_obj_result(), prior.as_ptr());
            }
            drop(scope);
            RETIRE.with(|selected| selected.set(0));
            OWNER.with(|owner| owner.borrow_mut().take());
        }
    }

    #[test]
    fn original_jim_replacement_refusal_keeps_occupied_key_and_reached_name_result() {
        // naming.alias.original-command-key-replacement-child-capture-and-rename
        // docs/design/analysis/name-resolution-proofs/alias-original-command-key-replacement-child-capture-and-rename.md
        // These real value-destructor Host/context modes are software controls;
        // the original421b probe did not execute these refusal callbacks.
        for mode in [1, 2] {
            reset();
            let mut interp = super::original_operand_tests::native("jim");
            let originals = originals();
            define(&mut interp, &originals, 0, true);
            drop(originals);
            ORDER.with(|rows| rows.borrow_mut().clear());
            OWNER.with(|owner| *owner.borrow_mut() = Some(interp.clone()));
            RETIRE_ROLE.with(|role| role.set(1));
            RETIRE.with(|selected| selected.set(mode));
            let name = Owned::fresh(obj::alloc_typed(&TYPE, 4));
            let words = super::original_operand_tests::words(&[b"alias", b"list", b"replacement"]);
            let code = super::jim_alias(
                &mut interp,
                &[
                    words[0].as_ptr(),
                    name.as_ptr(),
                    words[1].as_ptr(),
                    words[2].as_ptr(),
                ],
            );
            assert_eq!(code, Code::Error);
            assert_eq!(
                interp.get_obj_result(),
                name.as_ptr(),
                "Jim's original name result was reached before replacement"
            );
            assert_eq!(
                unsafe { (*name.as_ptr()).ref_count },
                2,
                "caller plus reached result; no failed incoming table-key role"
            );
            assert!(interp.native_execution_refusal().is_some());
            if mode == 1 {
                assert_eq!(
                    interp.native_execution_refusal(),
                    FIRST.with(|first| first.borrow().clone())
                );
            }
            assert!(interp
                .namespaces()
                .command_in(crate::namespace::GLOBAL, b"aliasKeyProbe")
                .is_none());
            assert!(interp
                .namespaces()
                .original_jim_command_key(crate::namespace::GLOBAL, b"aliasKeyProbe")
                .is_some());
            RETIRE.with(|selected| selected.set(0));
            OWNER.with(|owner| owner.borrow_mut().take());
        }
    }

    #[test]
    fn original_jim_rename_qualification_and_table_key_keep_distinct_native_references() {
        // naming.alias.original-command-key-replacement-child-capture-and-rename
        // docs/design/analysis/name-resolution-proofs/alias-original-command-key-replacement-child-capture-and-rename.md
        // The held temporary stage checks the pinned QualifyName/keyDup source
        // roles. Native421b's measured comparison is after both temporaries
        // have actually retired, rather than a claim of an observed callback.
        reset();
        let mut interp = super::original_operand_tests::native("jim");
        let originals = originals();
        define(&mut interp, &originals, 0, true);
        RENAME.with(|rename| rename.set(true));
        let scope =
            crate::interp::native_operation_currency::NativeOperationScope::enter(&interp).unwrap();
        let (old, new, old_key, new_key) = interp
            .original_jim_rename_operands(originals[0].as_ptr(), originals[4].as_ptr())
            .unwrap();
        assert_eq!(old_key.as_ptr(), originals[0].as_ptr());
        let new_key = new_key.unwrap();
        assert_eq!(new_key.as_ptr(), originals[4].as_ptr());
        assert_eq!(
            unsafe { (*old_key.as_ptr()).ref_count },
            3,
            "caller, original source table key and qualification temporary"
        );
        assert_eq!(
            unsafe { (*new_key.as_ptr()).ref_count },
            2,
            "caller and destination qualification temporary"
        );
        assert_eq!(
            interp.rename_command_with_original_jim_key(
                &old,
                &new,
                Some(std::rc::Rc::clone(&new_key))
            ),
            RenameOutcome::Renamed
        );
        assert_eq!(
            unsafe { (*old_key.as_ptr()).ref_count },
            2,
            "retired source table role leaves caller plus qualification temporary"
        );
        assert_eq!(
            unsafe { (*new_key.as_ptr()).ref_count },
            3,
            "fresh table keyDup is independent of caller and qualification temporary"
        );
        drop(old_key);
        drop(new_key);
        let stream = rows()
            .into_iter()
            .find(|(provider, _)| *provider == "jim")
            .unwrap()
            .1;
        for role in [0, 4] {
            assert_eq!(
                unsafe { (*originals[role].as_ptr()).ref_count } as i64,
                expected(stream, 10, "rename-name-after-result-clear", role, "refs")
            );
        }
        assert!(scope.currency().ensure_current_or_refuse().is_ok());
    }
}
