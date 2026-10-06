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

//! Tcl exception commands, original return-options dictionaries and error state.
//!
//! `catch` and `try` snapshot original results and private metadata before
//! resetting interpreter state. The shared completion-options owner supplies
//! native key ordering; physical snapshots duplicate the private dictionary.
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::dict;
use crate::frame::VarError;
use crate::interp::{drop_fresh, new_string, obj_bytes, Code, Interp};
use crate::obj::{self, TclObj};
use tcl_runtime_api::completion_options::{self, ErrorOptions, OptionValue};

/// Register `catch`, `error`, `try`, and `throw`.
pub fn install(interp: &mut Interp) {
    install_for_bootstrap(interp, None);
}

pub(crate) fn install_for_bootstrap(
    interp: &mut Interp,
    native: Option<tcl_registry::special_vars::NativeBootstrapProtocol>,
) {
    interp.register_builtin(b"catch", catch_cmd);
    interp.register_builtin(b"error", error_cmd);
    if native.is_none_or(|protocol| protocol.registers_core_try()) {
        interp.register_builtin(b"try", try_cmd);
    }
    if native.is_none_or(|protocol| protocol.registers_core_throw()) {
        interp.register_builtin(b"throw", throw_cmd);
    }
}

// catch

/// `catch script ?resultVarName? ?optionsVarName?` — evaluate `script`, trap any
/// completion code, and return it as an integer (0=ok … 4=continue).
fn catch_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let dialect = interp.native_invocation_dialect();
    if dialect.return_options_protocol()
        == Some(tcl_cmd_core::return_options::ReturnOptionsProtocol::Jim084)
    {
        let none = obj::Owned::fresh(new_string(b"NONE"));
        if let Err(error) = interp.var_set(b"::errorCode", none.as_ptr()) {
            if interp.host_refusal_pending() {
                return Code::Error;
            }
            let _ = error;
        }
    }
    let argument_bytes: Vec<_> = argv[1..].iter().map(|&word| obj_bytes(word)).collect();
    let argument_strings: Vec<_> = argument_bytes
        .iter()
        .map(|bytes| String::from_utf8_lossy(bytes))
        .collect();
    let arguments: Vec<_> = argument_strings.iter().map(|word| word.as_ref()).collect();
    let tcl_registry::catch_invocation::CatchInvocationSelection::Valid(selected) =
        tcl_registry::catch_invocation::select_catch_invocation(
            tcl_registry::InvocationArguments::literals(&arguments),
            dialect,
        )
    else {
        let synopsis: &[u8] = if dialect.family() == Some(tcl_dialect::model::Family::Jim) {
            b"catch ?-?no?code ... --? script ?resultVarName? ?optionVarName?".as_slice()
        } else if dialect.completion_options_policy()
            == Some(tcl_registry::CompletionOptionsPolicy::Legacy)
        {
            b"catch command ?varName?"
        } else {
            b"catch script ?resultVarName? ?optionVarName?"
        };
        return interp.wrong_args(synopsis);
    };
    // The caught body is a fresh completion scope. Commands within it retain
    // carried options until a later command replaces them, but neither the
    // caller's prior options nor the caught body's options belong to the
    // successful `catch` command itself.
    interp.clear_return_options();
    // `catch` is bytecode-compiled inline (C's `TclCompileCatchCmd`): a literal
    // body runs in the **same** `info frame` level and the same `codePtr->source`
    // as the enclosing proc/script. `eval_control_body` reproduces that — sharing
    // the enclosing frame in a proc (so `info frame` depth and the body-relative
    // `errorLine` for `MakeProcError` stay correct), while a top-level or dynamic
    // body still evaluates as its own frame.
    let mut code = interp.eval_control_body(argv[selected.script_at + 1]);
    if interp.host_refusal_pending() {
        return Code::Error;
    }
    // An `exit` in the body is uncatchable (C's `Tcl_Exit`): re-propagate it
    // instead of turning it into a caught return code.
    if interp.exit_pending() {
        if dialect.family() != Some(tcl_dialect::model::Family::Jim) || selected.ignores(6) {
            return code;
        }
        let status = interp.take_exit().expect("pending process exit");
        interp.set_result_bytes(status.to_string().as_bytes());
        code = Code::from_int(6);
    }
    if selected.ignores(code.as_int()) {
        return code;
    }
    // Snapshot the body's result BEFORE we overwrite the interp result with the
    // catch return value (read the value before clearing the result). `var_set`
    // retains it into the result var, so it survives the later `set_result`.
    let result = crate::obj::Owned::retain(interp.get_obj_result());
    let options_variable = selected.options_var_at.filter(|index| {
        dialect.family() != Some(tcl_dialect::model::Family::Jim)
            || !argument_bytes[*index].is_empty()
    });
    let options =
        options_variable.map(|_| crate::obj::Owned::fresh(completion_options(interp, code)));
    interp.clear_return_options();
    let selection = interp.active_native_compilation_selection();
    let Some(order) = selected.output_order(dialect, selection).indices(selected) else {
        return interp.error(b"native catch output protocol is not selected");
    };
    for index in order.into_iter().flatten() {
        if dialect.family() == Some(tcl_dialect::model::Family::Jim)
            && argument_bytes[index].is_empty()
        {
            continue;
        }
        let value = if Some(index) == selected.options_var_at {
            options.as_ref().expect("options requested above").as_ptr()
        } else {
            result.as_ptr()
        };
        let name = obj_bytes(argv[index + 1]);
        if let Err(error) = set_var_or_elem(interp, &name, value) {
            return crate::builtins::var_error(interp, &name, error);
        }
    }
    // The error is now caught: publish the accumulated trace to the
    // `::errorInfo`/`::errorCode` globals (so a later `set ::errorInfo` reads it)
    // and reset the accumulator for the next error.
    if code == Code::Error {
        interp.publish_and_reset_error();
    }
    interp.set_result_bytes(code.as_int().to_string().as_bytes());
    Code::Ok
}

/// Write `obj` to `name`, routing `arr(a)` to the array *element* rather than
/// a literal scalar named `arr(a)` — the same
/// `split_array_ref`/`var_set`/`var_set_elem` routing `set` uses, so
/// `catch`'s result/options vars and `try`'s handler vars don't hand-roll a
/// second name parser.
fn set_var_or_elem(interp: &mut Interp, name: &[u8], obj: *mut TclObj) -> Result<(), VarError> {
    let (base, elem) = crate::frame::split_array_ref(name);
    match &elem {
        Some(k) => interp.var_set_elem(&base, k, obj),
        None => interp.var_set(&base, obj),
    }
}

/// Build a completion's return-options dict from the live interpreter state.
///
/// This is the one implementation used by `catch`, `try`, and the shared
/// [`tcl_runtime_api::Completion`] adapter. It returns a fresh (`rc 0`) dict
/// containing `-code` and `-level`, plus the live error state when applicable.
/// A caller that exports the dict across an ABI must take an owning reference
/// before returning it.
pub(crate) fn jim_options_object(
    interp: &Interp,
    receipt: tcl_runtime_api::jim_return_state::JimReturnReceipt<obj::Owned>,
    code: Code,
) -> obj::Owned {
    obj::Owned::fresh(jim_options_raw(interp, receipt, code))
}

fn jim_options_raw(
    interp: &Interp,
    receipt: tcl_runtime_api::jim_return_state::JimReturnReceipt<obj::Owned>,
    code: Code,
) -> *mut TclObj {
    let values: Vec<_> = receipt
        .option_pairs(api_code(code))
        .into_iter()
        .flat_map(|(key, value)| {
            [
                obj::Owned::fresh(new_string(key)),
                match value {
                    OptionValue::Integer(integer) => {
                        obj::Owned::fresh(obj::new_wide_int_obj(integer))
                    }
                    OptionValue::Value(original) => original,
                },
            ]
        })
        .collect();
    let pointers: Vec<_> = values.iter().map(obj::Owned::as_ptr).collect();
    interp.new_list_object(&pointers)
}

pub(crate) fn completion_options(interp: &mut Interp, code: Code) -> *mut TclObj {
    if interp.uses_jim_error_stack() {
        return jim_options_raw(interp, interp.jim_return_receipt(), code);
    }
    // A body that completed via `return` propagates the return's *own* requested
    // options (`-code C -level L`), not the settled `RETURN`(2)/level-0 — what
    // `catch`'s options dict and TIP 329 `-during` chaining record
    // (`Tcl_GetReturnOptions`, `tclResult.c`). Every other code is at level 0.
    let (eff_code, level) = if code == Code::Return {
        (interp.pending_return_code(), interp.pending_return_level())
    } else {
        (code, 0)
    };
    let carried = interp.pending_return_option_objects();
    let protocol = interp.native_invocation_dialect().return_options_protocol();
    let error = (eff_code == Code::Error).then(|| ErrorOptions {
        error_code: Some(
            carried
                .iter()
                .find(|pair| {
                    protocol.is_some_and(|protocol| pair.name_in(protocol) == b"-errorcode")
                })
                .map_or_else(
                    || {
                        interp
                            .native_private_error_object(false)
                            .unwrap_or_else(|| {
                                crate::obj::Owned::fresh(new_string(&interp.error_code()))
                            })
                    },
                    |pair| pair.value.clone(),
                ),
        ),
        error_info: (level == 0).then(|| {
            interp
                .native_private_error_object(true)
                .unwrap_or_else(|| crate::obj::Owned::fresh(new_string(&interp.error_info())))
        }),
        error_stack: (level == 0
            && !interp.uses_jim_error_stack()
            && interp.runtime_version().has_error_stack())
        .then(|| interp.original_error_stack_value()),
        error_line: (level == 0 && !interp.uses_jim_error_stack())
            .then(|| i64::from(interp.error_line())),
        during: (level == 0)
            .then(|| interp.during_opts().map(crate::obj::Owned::retain))
            .flatten(),
    });
    let values: Vec<_> = carried
        .iter()
        .map(|pair| (pair.key_bytes.clone(), pair.value.clone()))
        .collect();
    let planned = completion_options::plan(
        interp.runtime_version(),
        api_code(eff_code),
        i64::try_from(level).unwrap_or(i64::MAX),
        &values,
        error.as_ref(),
    );
    if let Some((original, strings)) = interp.duplicate_original_return_options() {
        let mut snapshot = match crate::dict::PreparedNativeDictionary::prepare(
            Some(original.as_ptr()),
            strings,
        ) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                interp.report_cmd_error(error.into());
                return obj::new_obj();
            }
        };
        drop(original);
        let overlay = completion_options::plan_with_origin(
            interp.runtime_version(),
            api_code(eff_code),
            i64::try_from(level).unwrap_or(i64::MAX),
            tcl_core_types::CompletionOptionOrigin::ErrorMetadata,
            &[],
            error.as_ref(),
        );
        for (key, value) in overlay {
            let key = obj::Owned::fresh(new_string(&key));
            let value = match value {
                OptionValue::Integer(integer) => obj::Owned::fresh(obj::new_wide_int_obj(integer)),
                OptionValue::Value(original) => original,
            };
            if let Err(error) = snapshot.set_member(key.as_ptr(), value.as_ptr()) {
                interp.report_cmd_error(error.into());
                return obj::new_obj();
            }
        }
        return snapshot.into_value().into_native_unowned();
    }
    let retained: Vec<_> = planned
        .into_iter()
        .map(|(key, value)| {
            let key = carried
                .iter()
                .find(|pair| pair.key_bytes == key)
                .map_or_else(
                    || crate::obj::Owned::fresh(new_string(&key)),
                    |pair| pair.key.clone(),
                );
            let value = match value {
                OptionValue::Integer(value) => {
                    crate::obj::Owned::fresh(new_string(value.to_string().as_bytes()))
                }
                OptionValue::Value(value) => value,
            };
            (key, value)
        })
        .collect();
    let pairs: Vec<_> = retained
        .iter()
        .map(|(key, value)| (key.as_ptr(), value.as_ptr()))
        .collect();
    dict::new_dict_obj(&pairs)
}

fn api_code(code: Code) -> tcl_runtime_api::Code {
    match code {
        Code::Ok => tcl_runtime_api::Code::Ok,
        Code::Error => tcl_runtime_api::Code::Error,
        Code::Return => tcl_runtime_api::Code::Return,
        Code::Break => tcl_runtime_api::Code::Break,
        Code::Continue => tcl_runtime_api::Code::Continue,
        Code::Other(value) => tcl_runtime_api::Code::Other(value),
    }
}

// error

/// `error message ?errorInfo? ?errorCode?` — raise an error. With an explicit
/// non-empty `errorInfo`, the trace is pre-seeded with it and the `error`
/// command itself is **not** re-logged (`ERR_ALREADY_LOGGED`); otherwise the
/// `while executing` / `invoked from within` trace accumulates as the error
/// unwinds. `errorCode` defaults to `NONE`. (`tclProc.c` `Tcl_ErrorObjCmd`.)
fn error_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let Some(protocol) = interp.native_invocation_dialect().error_arguments() else {
        return interp.error(b"native error argument grammar is not selected");
    };
    if !protocol.accepts_len(argv.len().saturating_sub(1)) {
        let suffix = protocol.synopsis().split_once(' ').unwrap().1;
        return interp.wrong_args_for_invocation(argv, suffix.as_bytes());
    }
    if protocol == tcl_registry::invocation_words::NativeErrorArguments::JimStackTrace {
        interp.set_result(argv[1]);
        if let Some(&trace) = argv.get(2) {
            interp.adopt_jim_stacktrace(obj::Owned::retain(trace));
        }
        return interp.set_error_state(b"NONE");
    }
    if interp
        .native_invocation_dialect()
        .native_error_variable_protocol()
        .is_some()
    {
        let flags: Vec<_> = [
            b"-code".as_slice(),
            b"error",
            b"-level",
            b"0",
            b"-errorinfo",
            b"-errorcode",
        ]
        .into_iter()
        .map(|bytes| obj::Owned::fresh(new_string(bytes)))
        .collect();
        let mut arguments = vec![
            flags[0].as_ptr(),
            flags[1].as_ptr(),
            flags[2].as_ptr(),
            flags[3].as_ptr(),
        ];
        if let Some(&info) = argv.get(2) {
            arguments.extend([flags[4].as_ptr(), info]);
        }
        if let Some(&code) = argv.get(3) {
            arguments.extend([flags[5].as_ptr(), code]);
        }
        arguments.push(argv[1]);
        return crate::return_options::command(interp, &arguments);
    }
    // An explicit `errorCode` arg is honoured verbatim — even when empty, it
    // reads back empty rather than the `NONE` default (error-4.5).
    let explicit_code = argv.len() == 4;
    let ecode = if explicit_code {
        obj_bytes(argv[3])
    } else {
        b"NONE".to_vec()
    };
    // An empty explicit info is treated as absent (C: zero-length info arg).
    let info = if argv.len() >= 3 {
        obj_bytes(argv[2])
    } else {
        Vec::new()
    };
    let msg = obj_bytes(argv[1]);
    let rc = if info.is_empty() {
        interp.set_result(argv[1]);
        interp.set_error_state(&ecode)
    } else {
        interp.raise_with_info(&msg, &info, &ecode)
    };
    if explicit_code {
        interp.mark_error_code_explicit();
    }
    rc
}

// try / throw

/// A `try` handler clause. The handler `script` is kept as its argument object
/// (not flattened to bytes) so it evaluates through `eval_control_body`, which
/// A parsed `try` handler clause (`on code …` or `trap pattern …`). The
/// completion code / errorcode prefix are resolved at parse time so a bad code
/// word or trap prefix errors before the body runs; `is_dash` marks a `-`
/// fall-through body (the next non-`-` clause's body runs instead).
struct Handler {
    /// The completion code an `on` clause matches (a `trap` is always `1`).
    code: i64,
    /// `true` for a `trap` clause (matches an error by `-errorcode` prefix).
    is_trap: bool,
    /// The `trap` errorcode prefix (a list); empty for `on`.
    pattern: Vec<u8>,
    /// The `[resultVar ?optionsVar?]` bind list.
    vars: Vec<u8>,
    /// The handler body, or `-` (see `is_dash`).
    script: *mut TclObj,
    /// Whether the body is the fall-through marker `-`.
    is_dash: bool,
}

/// Map a `try`/`on` completion-code word (`ok`/`error`/`return`/`break`/
/// `continue` or an integer that fits a C `int`) to its numeric code.
fn code_word_to_int(spec: &[u8]) -> Option<i64> {
    match spec {
        b"ok" => Some(0),
        b"error" => Some(1),
        b"return" => Some(2),
        b"break" => Some(3),
        b"continue" => Some(4),
        // A completion code accepts the full signed/unsigned 32-bit range (C's
        // `Tcl_GetIntFromObj`), with `0x`/`0o`/`0b`/`0d` prefixes and a sign.
        _ => crate::interp::parse_completion_int(spec).map(i64::from),
    }
}

/// Does `pattern` (a list) match `errorcode` (a list) as a leading sublist?
/// An empty pattern matches any error (`trap {} ...`).
fn errorcode_prefix_match(pattern: &[u8], errorcode: &[u8]) -> bool {
    let pat = match crate::parse::split_list(pattern) {
        Ok(p) => p,
        Err(_) => return false,
    };
    let ec = crate::parse::split_list(errorcode).unwrap_or_default();
    pat.len() <= ec.len() && pat.iter().zip(ec.iter()).all(|(a, b)| a == b)
}

/// `try`'s handler-type words, in C table order (`TryObjCmd`'s `handlerNames`,
/// `tclCmdMZ.c`): `Tcl_GetIndexFromObj(…, "handler type", 0)`, so `f`/`o`/`t`
/// abbreviate and the empty word — a prefix of all three — is
/// `ambiguous handler type ""`. The type is resolved before the clause's
/// arity, as it is in C (`try {} x` → `bad handler type "x"`).
const HANDLER_TYPES: tcl_cmd_core::prefix::OptionTable<'static, &[u8]> =
    tcl_cmd_core::prefix::OptionTable::abbreviating("handler type", &[b"finally", b"on", b"trap"]);

fn try_clause_arguments(
    interp: &mut Interp,
    clause: tcl_registry::NativeTryClauseArgument,
    message: &[u8],
) -> Code {
    let Some(code) = interp
        .native_invocation_dialect()
        .try_clause_argument_error_code(clause)
    else {
        return interp.refuse_native_access(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "try clause arguments",
            ),
        );
    };
    interp.report_cmd_error(tcl_cmd_core::CmdError::with_error_code_bytes(message, code))
}

/// `try body ?handler ...? ?finally script?` — structured exception handling
/// (TIP 329). Handlers are `on code varList script` and `trap pattern varList
/// script`, tried in order; the first match runs and its completion becomes the
/// `try` result. `finally` always runs; only an error from it overrides the
/// result. Modelled on `tclCmdMZ.c` `Tcl_TryObjCmd`.
fn try_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if interp.uses_jim_error_stack() {
        return crate::jim_try::command(interp, argv);
    }
    const USAGE: &[u8] = b"try body ?handler ...? ?finally script?";
    if argv.len() < 2 {
        return interp.wrong_args(USAGE);
    }
    let body = argv[1];

    let mut handlers: Vec<Handler> = Vec::new();
    let mut finally: Option<*mut TclObj> = None;
    let mut j = 2;
    while j < argv.len() {
        let handler_type = match interp.native_static_option_index(
            argv[j],
            HANDLER_TYPES.names(),
            false,
            "handler type",
        ) {
            Ok(i) => HANDLER_TYPES.names()[i],
            Err(m) => return interp.report_cmd_error(m),
        };
        match handler_type {
            b"finally" => {
                if j < argv.len() - 2 {
                    return interp.set_error(b"finally clause must be last");
                }
                if j == argv.len() - 1 {
                    return try_clause_arguments(
                        interp,
                        tcl_registry::NativeTryClauseArgument::Finally,
                        b"wrong # args to finally clause: must be \"... finally script\"",
                    );
                }
                finally = Some(argv[j + 1]);
                j += 2;
            }
            b"on" => {
                if j + 4 > argv.len() {
                    return try_clause_arguments(
                        interp,
                        tcl_registry::NativeTryClauseArgument::On,
                        b"wrong # args to on clause: must be \"... on code variableList script\"",
                    );
                }
                let code = match code_word_to_int(&obj_bytes(argv[j + 1])) {
                    Some(c) => c,
                    None => return bad_completion_code(interp, &obj_bytes(argv[j + 1])),
                };
                let script = argv[j + 3];
                handlers.push(Handler {
                    code,
                    is_trap: false,
                    pattern: Vec::new(),
                    vars: obj_bytes(argv[j + 2]),
                    script,
                    is_dash: obj_bytes(script).as_slice() == b"-",
                });
                j += 4;
            }
            b"trap" => {
                if j + 4 > argv.len() {
                    return try_clause_arguments(interp, tcl_registry::NativeTryClauseArgument::Trap, b"wrong # args to trap clause: must be \"... trap pattern variableList script\"");
                }
                let pattern = obj_bytes(argv[j + 1]);
                if crate::parse::split_list(&pattern).is_err() {
                    let mut m = b"bad prefix '".to_vec();
                    m.extend_from_slice(&pattern);
                    m.extend_from_slice(b"': must be a list");
                    return interp.set_error(&m);
                }
                let script = argv[j + 3];
                handlers.push(Handler {
                    code: 1,
                    is_trap: true,
                    pattern,
                    vars: obj_bytes(argv[j + 2]),
                    script,
                    is_dash: obj_bytes(script).as_slice() == b"-",
                });
                j += 4;
            }
            // Unreachable: `HANDLER_TYPES` has exactly the three arms above.
            other => {
                let mut m = b"bad handler type \"".to_vec();
                m.extend_from_slice(other);
                m.extend_from_slice(b"\": must be ");
                m.extend_from_slice(&tcl_cmd_core::prefix::choice_list_bytes(
                    HANDLER_TYPES.names(),
                ));
                return interp.set_error(&m);
            }
        }
    }
    // The last non-finally clause may not be a `-` fall-through (nothing follows).
    if handlers.last().is_some_and(|h| h.is_dash) {
        return interp.set_error(b"last non-finally clause must not have a body of \"-\"");
    }

    // Run the body, snapshotting its completion code, result, and -errorcode.
    // `eval_control_body` recovers the body literal's TIP 280 source location so
    // an `info frame` inside reports the right `type source` line.
    let body_code = interp.eval_control_body(body);
    if interp.host_refusal_pending() {
        return Code::Error;
    }
    let body_result = interp.result_bytes();
    let errorcode = if body_code == Code::Error {
        interp.error_code()
    } else {
        Vec::new()
    };

    // Locate the first matching handler, then scan forward over `-` bodies to the
    // clause whose body actually runs (binding *that* clause's variables).
    let mut outcome_code = body_code;
    let mut outcome_result = body_result.clone();
    let matched = handlers.iter().position(|h| {
        if h.is_trap {
            body_code == Code::Error && errorcode_prefix_match(&h.pattern, &errorcode)
        } else {
            h.code == body_code.as_int()
        }
    });
    if let Some(m) = matched {
        let mut b = m;
        while handlers[b].is_dash {
            b += 1; // guaranteed to terminate (the last body is not `-`)
        }
        // Build the body's options dict from the *live* body error/return state
        // (before it is published+reset). It is bound to the handler's optionsVar
        // and reused as the `-during` chain link if the handler itself throws
        // (TIP 329 exception chaining). Retained for the duration of the handler.
        let body_opts = completion_options(interp, body_code);
        // SAFETY: keep `body_opts` alive across the handler eval / var binding.
        unsafe { obj::incr_ref_count(body_opts) };
        // Bind the running clause's variables: [resultVar ?optionsVar?]. A failed
        // bind becomes the outcome (and skips the handler body), but `finally`
        // still runs; the bind error chains to the body via `-during` (C's
        // `handlerFailed`).
        match bind_handler_vars(interp, &handlers[b].vars, &body_result, body_opts) {
            Ok(()) => {
                // The body's exception is now handled: publish + reset so the
                // handler starts with clean error state.
                if body_code == Code::Error {
                    interp.publish_and_reset_error();
                }
                outcome_code = interp.eval_control_body(handlers[b].script);
                if interp.host_refusal_pending() {
                    unsafe { obj::decr_ref_count(body_opts) };
                    return Code::Error;
                }
                outcome_result = interp.result_bytes();
                if outcome_code == Code::Error {
                    // The handler threw over the body's exception: chain it.
                    interp.set_during(body_opts);
                }
            }
            Err(()) => {
                outcome_code = Code::Error;
                if interp.host_refusal_pending() {
                    unsafe { obj::decr_ref_count(body_opts) };
                    return Code::Error;
                }
                outcome_result = interp.result_bytes();
                interp.set_during(body_opts);
            }
        }
        // SAFETY: release our hold; `set_during`/the optionsVar keep their own.
        unsafe { obj::decr_ref_count(body_opts) };
    }

    // `finally` always runs; only an exception from it overrides the result.
    if let Some(fin) = finally {
        // Capture the options that would propagate from the body/handler stage
        // (carrying any `-during` already chained) in case `finally` throws and
        // must chain them in turn.
        let prior_opts = completion_options(interp, outcome_code);
        // SAFETY: keep `prior_opts` alive across the finally eval.
        unsafe { obj::incr_ref_count(prior_opts) };
        let fc = interp.eval_control_body(fin);
        if interp.host_refusal_pending() {
            unsafe { obj::decr_ref_count(prior_opts) };
            return Code::Error;
        }
        if fc != Code::Ok {
            if fc == Code::Error {
                interp.set_during(prior_opts); // chain the superseded exception
            } else {
                interp.clear_during(); // a non-error finally exception does not chain
            }
            // SAFETY: release our hold (`set_during` kept its own if it ran).
            unsafe { obj::decr_ref_count(prior_opts) };
            return fc; // finally's result is already the interp result
        }
        // SAFETY: finally completed OK — discard the speculative capture.
        unsafe { obj::decr_ref_count(prior_opts) };
    }

    interp.set_result_bytes(&outcome_result);
    outcome_code
}

/// `bad completion code "X": must be ok, error, return, break, continue, or an
/// integer` — an unrecognised `on` code word.
fn bad_completion_code(interp: &mut Interp, word: &[u8]) -> Code {
    let mut m = b"bad completion code \"".to_vec();
    m.extend_from_slice(word);
    m.extend_from_slice(b"\": must be ok, error, return, break, continue, or an integer");
    interp.set_error(&m)
}

/// Bind a `try` handler clause's `[resultVar ?optionsVar?]` to the body's result
/// and the prebuilt option dict (`body_opts`, retained into the variable).
/// `Err(())` if a variable can't be set (the interp error is left in place); the
/// result variable is set before the options variable, so a later failure still
/// leaves the result variable assigned (error-19.11).
fn bind_handler_vars(
    interp: &mut Interp,
    vars: &[u8],
    body_result: &[u8],
    body_opts: *mut TclObj,
) -> Result<(), ()> {
    let names = crate::parse::split_list(vars).unwrap_or_default();
    if let Some(rv) = names.first() {
        if !rv.is_empty() {
            let o = new_string(body_result);
            if let Err(e) = set_var_or_elem(interp, rv, o) {
                drop_fresh(o);
                crate::builtins::var_error(interp, rv, e);
                return Err(());
            }
        }
    }
    if let Some(ov) = names.get(1) {
        if let Err(e) = set_var_or_elem(interp, ov, body_opts) {
            crate::builtins::var_error(interp, ov, e);
            return Err(());
        }
    }
    Ok(())
}

/// `throw type message` — raise an error with `-errorcode type` (a non-empty
/// list). Equivalent to `return -code error -errorcode $type $message`.
fn throw_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return interp.wrong_args(b"throw type message");
    }
    let ecode = obj_bytes(argv[1]);
    match crate::parse::split_list(&ecode) {
        Ok(parts) if !parts.is_empty() => {}
        // A malformed type list reports the list parse error verbatim
        // (error-8.8/8.11); a well-formed but empty list is the type error.
        Ok(_) => return interp.set_error(b"type must be non-empty list"),
        Err(e) => return interp.set_error(&crate::parse::list_error_message(&ecode, e)),
    }
    interp.set_result(argv[2]);
    // Like `return -code error -errorcode $type $msg`: set the code, let the
    // `while executing`/`invoked from within` trace accumulate as it unwinds.
    interp.set_error_state(&ecode)
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

    /// `try`'s handler-type word is a `Tcl_GetIndexFromObj(…,
    /// "handler type", 0)` table, so the three types abbreviate and the empty
    /// word — a prefix of all three — is `ambiguous handler type ""`.
    ///
    /// tclsh 8.6.16 / 9.0.4:
    ///   try {} o error {} {}  -> {}   ;  try {} f {} -> {}  ;  try {} t {} {} {} -> {}
    ///   try {} {} error {} {} -> ambiguous handler type "": must be finally, on, or trap
    ///   try {} x              -> bad handler type "x": … (the type precedes the arity)
    #[test]
    fn try_handler_type_resolves_like_tcl_get_index_from_obj() {
        const MUST: &str = "must be finally, on, or trap";
        leak_free(|i| {
            // Unique prefixes resolve to their clause grammar.
            assert_eq!(run(i, b"try {set x ok} o error {} {set x h}"), b"ok");
            assert_eq!(run(i, b"try {set x ok} f {set y 1}"), b"ok");
            assert_eq!(run(i, b"try {set x ok} t {} {} {set x h}"), b"ok");
            // The empty word prefixes all three ⇒ ambiguous.
            assert_eq!(i.eval_str(b"try good {} error {} {x}"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                format!("ambiguous handler type \"\": {MUST}").as_bytes()
            );
            // The type is resolved before the clause's arity.
            assert_eq!(i.eval_str(b"try good x"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                format!("bad handler type \"x\": {MUST}").as_bytes()
            );
        });
    }

    #[test]
    fn exit_records_code_and_is_uncatchable() {
        leak_free(|i| {
            // `exit N` records the code and unwinds (never terminates the host).
            assert_eq!(i.eval_str(b"exit 7"), Code::Error);
            assert_eq!(i.take_exit(), Some(7));
            // Default code is 0.
            assert_eq!(i.eval_str(b"exit"), Code::Error);
            assert_eq!(i.take_exit(), Some(0));
            // Uncatchable: `catch {exit}` re-propagates, so the trailing command
            // never runs.
            assert_eq!(i.eval_str(b"catch {exit 5}; set marker ran"), Code::Error);
            assert_eq!(i.take_exit(), Some(5));
            assert_eq!(run(i, b"info exists marker"), b"0");
            // A non-integer code is the standard error; no exit is pending.
            assert_eq!(i.eval_str(b"exit foo"), Code::Error);
            assert_eq!(i.take_exit(), None);
            assert_eq!(i.result_bytes(), b"expected integer but got \"foo\"");
            // Arity.
            assert_eq!(i.eval_str(b"exit a b"), Code::Error);
            assert_eq!(i.take_exit(), None);
        });
    }

    #[test]
    fn catch_success_and_error_codes() {
        leak_free(|i| {
            // success → code 0, result var = body result (no tower needed).
            assert_eq!(run(i, b"catch {set z 2} m"), b"0");
            assert_eq!(run(i, b"set m"), b"2");
            // error → code 1, result var = error message.
            assert_eq!(run(i, b"catch {error boom} m"), b"1");
            assert_eq!(run(i, b"set m"), b"boom");
            // a no-such-variable read is caught too.
            assert_eq!(run(i, b"catch {set nope} m"), b"1");
            assert_eq!(run(i, b"set m"), b"can't read \"nope\": no such variable");
            // break/continue propagate their codes through catch.
            assert_eq!(run(i, b"catch {break} m"), b"3");
            assert_eq!(run(i, b"catch {continue} m"), b"4");
            i.eval_str(b"unset m");
        });
    }

    #[test]
    fn error_stamps_globals() {
        leak_free(|i| {
            // A bare `error` with no info accumulates the source trace as it
            // unwinds (verified byte-for-byte vs tclsh 9.0).
            assert_eq!(run(i, b"catch {error oops}"), b"1");
            assert_eq!(run(i, b"set ::errorCode"), b"NONE");
            assert_eq!(
                run(i, b"set ::errorInfo"),
                b"oops\n    while executing\n\"error oops\""
            );
            // explicit info + code: the info is the trace verbatim (the `error`
            // command itself is not re-logged — ERR_ALREADY_LOGGED).
            assert_eq!(run(i, b"catch {error msg myinfo MYCODE}"), b"1");
            assert_eq!(run(i, b"set ::errorInfo"), b"myinfo");
            assert_eq!(run(i, b"set ::errorCode"), b"MYCODE");
            i.eval_str(b"unset ::errorInfo ::errorCode");
        });
    }

    /// The incremental `::errorInfo` stack trace (`while executing` / `invoked
    /// from within` / `(procedure "x" line N)` …), every expected string
    /// captured from real tclsh 9.0. See `proc-call-and-stack-traces.md` PC-4.
    #[test]
    fn error_info_stack_traces() {
        leak_free(|i| {
            // The worked example: proc body error → proc frame → call frame.
            run(i, b"proc p {} { error foo }");
            assert_eq!(i.eval_str(b"p"), Code::Error);
            assert_eq!(
                i.var_get(b"::errorInfo").map(crate::interp::obj_bytes),
                Some(
                    b"foo\n    while executing\n\"error foo \"\n    (procedure \"p\" line 1)\n    invoked from within\n\"p\""
                        .to_vec()
                )
            );
            // Multi-line body: the proc frame cites the body-relative line (3).
            run(i, b"proc q {} {\n    set x 1\n    error boom\n}");
            i.eval_str(b"catch q");
            assert_eq!(
                run(i, b"set ::errorInfo"),
                b"boom\n    while executing\n\"error boom\"\n    (procedure \"q\" line 3)\n    invoked from within\n\"q\""
            );
            // Nested command substitution: the `[inner]` subst is logged, the
            // enclosing `set y [inner]` is suppressed.
            run(i, b"proc inner {} { error deep }");
            run(i, b"proc outer {} { set y [inner] }");
            i.eval_str(b"catch outer");
            assert_eq!(
                run(i, b"set ::errorInfo"),
                b"deep\n    while executing\n\"error deep \"\n    (procedure \"inner\" line 1)\n    invoked from within\n\"inner\"\n    (procedure \"outer\" line 1)\n    invoked from within\n\"outer\""
            );
            // apply → a `(lambda term "..." line N)` frame.
            i.eval_str(b"catch { apply {{} { error fromLambda }} }");
            assert_eq!(
                run(i, b"set ::errorInfo"),
                b"fromLambda\n    while executing\n\"error fromLambda \"\n    (lambda term \"{} { error fromLambda }\" line 1)\n    invoked from within\n\"apply {{} { error fromLambda }} \""
            );
            i.eval_str(b"unset -nocomplain ::errorInfo ::errorCode x y");
        });
    }

    /// `eval`/`uplevel`/`foreach` add a `("<cmd>" body line N)` frame; inline
    /// `if`/`while`/`for`/`switch` do not (tclsh 9.0).
    // Needs the numeric tower: the inline-`if` frame case dispatches `if`.
    #[cfg(have_tommath)]
    #[test]
    fn body_frame_commands() {
        leak_free(|i| {
            i.eval_str(b"catch { eval { error e } }");
            assert_eq!(
                run(i, b"set ::errorInfo"),
                b"e\n    while executing\n\"error e \"\n    (\"eval\" body line 1)\n    invoked from within\n\"eval { error e } \""
            );
            i.eval_str(b"catch { foreach z {1} { error f } }");
            assert_eq!(
                run(i, b"set ::errorInfo"),
                b"f\n    while executing\n\"error f \"\n    (\"foreach\" body line 1)\n    invoked from within\n\"foreach z {1} { error f } \""
            );
            // inline `if`: only the body command, no `if` frame.
            i.eval_str(b"catch { if {1} { error x } }");
            assert_eq!(
                run(i, b"set ::errorInfo"),
                b"x\n    while executing\n\"error x \""
            );
            // `foreach` inside a proc is inlined (no foreach frame) — only the
            // proc frame; the top-level foreach above did show a frame.
            run(i, b"proc fe {} { foreach x {1} { error e } }");
            i.eval_str(b"catch fe");
            assert_eq!(
                run(i, b"set ::errorInfo"),
                b"e\n    while executing\n\"error e \"\n    (procedure \"fe\" line 1)\n    invoked from within\n\"fe\""
            );
            i.eval_str(b"unset -nocomplain ::errorInfo ::errorCode");
        });
    }

    #[test]
    fn try_on_and_trap_and_finally() {
        leak_free(|i| {
            // on ok: the body succeeded; handler result becomes try's result.
            assert_eq!(run(i, b"try {set x 7} on ok {} {set y done}"), b"done");
            // on error msg: binds the result, runs the handler.
            assert_eq!(run(i, b"try {error boom} on error msg {set msg}"), b"boom");
            // trap: matches on the -errorcode leading sublist.
            assert_eq!(
                run(
                    i,
                    b"try {throw {POSIX EACCES} denied} trap {POSIX EACCES} {} {set r trapped}"
                ),
                b"trapped"
            );
            // no handler matches → the body's error propagates.
            assert_eq!(
                i.eval_str(b"try {error nope} on break {} {set r x}"),
                Code::Error
            );
            assert_eq!(i.result_bytes(), b"nope");
            // finally always runs; an OK finally doesn't change the result.
            assert_eq!(
                run(
                    i,
                    b"set fin 0; set v [try {set z 1} finally {set fin 1}]; list $v $fin"
                ),
                b"1 1"
            );
            i.eval_str(b"unset -nocomplain x msg r fin v z ::errorInfo ::errorCode");
        });
    }

    #[test]
    fn throw_sets_errorcode() {
        leak_free(|i| {
            assert_eq!(run(i, b"catch {throw {MY CODE} boom} m o"), b"1");
            assert_eq!(run(i, b"set m"), b"boom");
            assert_eq!(run(i, b"dict get $o -errorcode"), b"MY CODE");
            // an empty type is rejected.
            assert_eq!(i.eval_str(b"throw {} msg"), Code::Error);
            assert_eq!(i.result_bytes(), b"type must be non-empty list");
            i.eval_str(b"unset -nocomplain m o ::errorInfo ::errorCode");
        });
    }

    #[test]
    fn catch_options_dict() {
        leak_free(|i| {
            run(i, b"catch {error boom mycode MYERR} m o");
            // -code/-level always present; -errorcode/-errorinfo on error.
            assert_eq!(run(i, b"dict get $o -code"), b"1");
            assert_eq!(run(i, b"dict get $o -level"), b"0");
            assert_eq!(run(i, b"dict get $o -errorcode"), b"MYERR");
            assert_eq!(run(i, b"dict get $o -errorinfo"), b"mycode");
            // success path: -code 0.
            run(i, b"catch {set x 5} m o");
            assert_eq!(run(i, b"dict get $o -code"), b"0");
            i.eval_str(b"unset m o x ::errorInfo ::errorCode");
        });
    }

    #[test]
    fn explicit_errorinfo_retains_the_prior_error_stack() {
        leak_free(|i| {
            assert_eq!(run(i, b"catch {error first} m o"), b"1");
            let prior = run(i, b"dict get $o -errorstack");
            assert!(!prior.is_empty());

            assert_eq!(
                run(
                    i,
                    b"catch {return -level 0 -code error -errorinfo I second} m o"
                ),
                b"1"
            );
            assert_eq!(run(i, b"dict get $o -errorstack"), prior);
            assert_eq!(run(i, b"info errorstack"), prior);
            i.eval_str(b"unset -nocomplain m o ::errorInfo ::errorCode");
        });
    }

    /// Pre-TIP runtimes still preserve `-errorstack` as an arbitrary custom
    /// return option; only synthesis and validation of its TIP 348 meaning are
    /// release-gated. Other custom pairs take the same shared path.
    #[test]
    fn catch_preserves_custom_return_options_on_legacy_releases() {
        use tcl_dialect::TclVersion;

        leak_free(|i| {
            i.set_runtime_version(TclVersion::V8_5);
            assert_eq!(run(i, b"catch {return -bar soom} m o"), b"2");
            assert_eq!(run(i, b"set o"), b"-bar soom -code 0 -level 1");

            assert_eq!(run(i, b"catch {return -errorstack odd} m o"), b"2");
            assert_eq!(run(i, b"set o"), b"-errorstack odd -code 0 -level 1");
            i.eval_str(b"unset -nocomplain m o");
        });
    }

    #[test]
    fn catch_arity_rejects_before_body_under_legacy_c() {
        for release in tcl_dialect::TclVersion::ALL {
            leak_free(|interp| {
                interp.set_runtime_version(release);
                let result = run(interp, b"set ::hit 0; set code [catch {catch {set ::hit 1} result options} error]; list $code $::hit");
                assert_eq!(
                    result,
                    if release == tcl_dialect::TclVersion::V8_4 {
                        b"1 0"
                    } else {
                        b"0 1"
                    }
                );
            });
        }
    }

    #[test]
    fn automatic_jim_error_stacks_match_the_actual_native_interpreter() {
        let Some(reference) = tcl_test_support::locate_jimsh().expect("Jim oracle discovery")
        else {
            return;
        };
        let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        for (name, script) in tcl_test_support::automatic_errors::JIM_AUTOMATIC_ERROR_CASES {
            let source = format!("puts [eval {{{script}}}]\n");
            let expected = tcl_test_support::run_script(&reference.path, source.as_bytes())
                .expect("native Jim")
                .strict_text()
                .expect("native observation");
            leak_free(|interp| {
                interp.set_dialect_profile(profile);
                assert_eq!(
                    interp.eval_sourced(script.as_bytes(), b"stdin"),
                    Code::Ok,
                    "{name}: {}",
                    String::from_utf8_lossy(&interp.result_bytes())
                );
                assert_eq!(interp.result_bytes(), expected.as_bytes(), "{name}");
            });
        }
    }

    #[test]
    fn jim_automatic_stack_uses_evaluation_receipts_before_unwinding() {
        let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        leak_free(|interp| {
            interp.set_dialect_profile(profile);
            for (script, expected) in [
                (b"catch {error BOOM} r o; dict get $o -errorinfo".as_slice(), b"{} {} 1 {error BOOM}".as_slice()),
                (b"proc fail {} {error BOOM}; catch {fail} r o; dict get $o -errorinfo".as_slice(), b"fail {} 1 {error BOOM} {} {} 1 fail".as_slice()),
                (b"proc inner {x} {error $x}; proc outer {} {inner BOOM}; catch {outer} r o; dict get $o -errorinfo".as_slice(), b"inner {} 1 {error BOOM} outer {} 1 {inner BOOM} {} {} 1 outer".as_slice()),
                (b"rename fail saved; catch {saved} r o; dict get $o -errorinfo".as_slice(), b"saved {} 1 {error BOOM} {} {} 1 saved".as_slice()),
                (b"proc fail {} {missingCommand BOOM}; catch {fail} r o; dict get $o -errorinfo".as_slice(), b"fail {} 1 {} {} {} 1 fail".as_slice()),
                (b"proc fail {} {return -code error BOOM}; catch {fail} r o; dict get $o -errorinfo".as_slice(), b"{} {} 1 fail".as_slice()),
            ] {
                assert_eq!(run(interp, script), expected, "{}", String::from_utf8_lossy(script));
            }
        });
    }

    #[test]
    fn native_jim_error_keeps_raw_stack_trace_and_does_not_publish_c_globals() {
        let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        leak_free(|interp| {
            interp.set_dialect_profile(profile);
            assert_eq!(run(interp, b"set ::errorCode KEEP; set ::errorInfo KEEP; catch {error BOOM {P file 3}} result options; list $result [dict get $options -errorinfo] [dict get $options -errorcode] $::errorCode $::errorInfo"), b"BOOM {P file 3} NONE NONE KEEP");
            assert_eq!(
                run(interp, b"catch {error BOOM TRACE CODE} result; set result"),
                b"wrong # args: should be \"error message ?stacktrace?\""
            );
        });
    }

    #[test]
    fn jim_catch_switches_and_process_exit_use_native_completion_protocol() {
        let profile = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        )));
        leak_free(|interp| {
            interp.set_dialect_profile(profile);
            assert_eq!(run(interp, b"set ::hit 0; set code [catch {catch -bogus {set ::hit 1}} result]; list $code $::hit"), b"1 0");
            assert_eq!(run(interp, b"set code [catch {catch -noerror {error E} result} message]; list $code $message"), b"1 E");
            assert_eq!(run(interp, b"catch -exit {exit 12} result options; list $result [dict get $options -code] [dict get $options -level]"), b"12 6 0");
            assert!(!interp.exit_pending());
            assert_eq!(
                run(
                    interp,
                    b"catch -- {set ::hit 2} result options ignored; list $result $::hit"
                ),
                b"2 2"
            );
            assert_eq!(
                run(interp, b"catch {return result} {} {}; info exists {}"),
                b"0"
            );
        });
    }
}
