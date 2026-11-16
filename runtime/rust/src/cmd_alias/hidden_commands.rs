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

//! `interp hide`, `expose` and `invokehidden`, shared with the child-handle shorthand.

use super::child_interpreters::{interp_path, not_found_path};
use crate::interp::native_arguments::OriginalArguments;
use crate::interp::{Code, CommandVisibilityOp, Interp};
use crate::obj;
use crate::obj::TclObj;

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
pub(super) fn interp_hidectl(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    op: CommandVisibilityOp,
) -> Code {
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
pub(super) fn interp_invokehidden(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
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
