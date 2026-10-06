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

//! `try` / `throw` — structured exception handling (TIP 329).
//!
//! The VM compiles `try` to a runtime CALL (the bytecode backend has no
//! exception-range/`beginCatch` support), so the whole construct runs here.
//! Modelled on C Tcl 9's `Tcl_TryObjCmd`/`Tcl_ThrowObjCmd` (`tclCmdMZ.c`) and
//! `runtime/rust`'s `cmd_error.rs`: handlers (`on code varList script`,
//! `trap pattern varList script`) are tried in order, the first match runs and
//! its completion becomes the `try` result; a `-` body falls through to the next
//! clause; `finally` always runs and only its own exception overrides the
//! result. Semantics pinned against tclsh 9.0.
//!
//! Body/handler/finally each run as a phase of an explicit-stack state machine
//! (`TryState`/`TryPhase`/[`advance_try`]) rather than through
//! `Vm::eval_source`'s nested drive, so a `yield` inside any of them stays
//! yieldable — the phase transitions (handler matching, var
//! binding, `-during` chaining) are the same Rust-side logic a synchronous
//! version would need, just resumed from `Vm::unwind` instead of run
//! inline between two `eval_source` calls.

use std::rc::Rc;

use tcl_runtime_api::{Code, Completion, FatalTail};
use tcl_syntax::value::ValueOps;

use crate::command::{completion_options, opt_get, options_dict};
use crate::interp::{Vm, err, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    register_for_bootstrap(vm, None);
}

pub(crate) fn register_for_bootstrap(
    vm: &mut Vm,
    native: Option<tcl_registry::special_vars::NativeBootstrapProtocol>,
) {
    if native.is_none_or(tcl_registry::special_vars::NativeBootstrapProtocol::registers_core_try) {
        vm.register_stock_builtin("try", cmd_try);
    }
    if native.is_none_or(tcl_registry::special_vars::NativeBootstrapProtocol::registers_core_throw)
    {
        vm.register_stock_builtin("throw", cmd_throw);
    }
}

/// A parsed `try` handler clause.
struct Handler {
    /// The completion code an `on` clause matches (a `trap` is always `1`).
    code: i64,
    /// `true` for a `trap` clause (match an error by `-errorcode` prefix).
    is_trap: bool,
    /// The `trap` errorcode prefix (a list value); unused for `on`.
    pattern: Value,
    /// The `[resultVar ?optionsVar?]` bind list.
    vars: Value,
    /// The handler body, or `-` (see `is_dash`).
    script: Value,
    /// Whether the body is the fall-through marker `-`.
    is_dash: bool,
}

/// A `try`'s handlers + `finally` script, parsed once up front and shared
/// (via `Rc`) across every phase of the state machine — nothing here changes
/// once `cmd_try` has validated the grammar.
struct TryPlan {
    handlers: Vec<Handler>,
    finally: Option<Value>,
    jim: bool,
    deferred_clauses: Option<Vec<Value>>,
    ignored_codes: Vec<i64>,
    ignored_body: bool,
    body_error_code: Option<Value>,
}

/// Which phase of a `try` an explicit-stack activation is running, and the
/// state carried into the *next* phase once this one completes (see
/// [`advance_try`]).
pub(crate) enum TryPhase {
    /// Running the body.
    Body,
    /// Running the matched handler's script. `body_opts` is the body's options
    /// dict, kept for `-during` chaining if the handler itself throws.
    Handler { body_opts: Value },
    /// Running `finally`. `outcome` is the completion `finally` will restore
    /// on success (the body's or handler's) — captured before `finally` ran, so
    /// its own `-during` chaining (if `finally` throws) sees the right prior
    /// options.
    Finally { outcome: Completion<Value> },
}

/// A `try`'s live state, carried on the currently-running phase's activation
/// ([`Frame::try_ctx`](crate::exec::Frame)) until [`advance_try`] either moves
/// it to the next phase or delivers a final completion.
pub(crate) struct TryState {
    plan: Rc<TryPlan>,
    phase: TryPhase,
    pub(crate) fatal_tail: Option<FatalTail>,
}

/// One phase of a `try` deferred to the explicit stack: the phase's compiled
/// script plus the state to resume from once it completes. Parked in
/// `Vm.pending.try_phase` by `cmd_try`/[`advance_try`], drained into a try activation
/// (see [`Frame::new_try`](crate::exec::Frame::new_try)).
pub(crate) struct TryReq {
    pub(crate) script: crate::compiled::CompiledUnit,
    pub(crate) state: TryState,
}

/// The outcome of folding a completed try-phase activation (see
/// [`advance_try`]): either move on to the next phase (push a new try
/// activation for it) or the whole `try` is done.
pub(crate) enum TryOutcome {
    Push(Box<TryReq>),
    Deliver(Completion<Value>),
}

/// Append `-during prior` to an error's options dict (TIP 329 exception
/// chaining): the superseded exception's options ride along on the new one.
/// Replaces an existing `-during` (a handler over a handler) rather than dup it.
fn add_during(vm: &mut Vm, options: &Value, prior: &Value) -> Result<Value, Completion<Value>> {
    let mut pairs = ValueOps::dict_pairs(vm, options)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
    for (key, value) in &mut pairs {
        let bytes = ValueOps::native_string_bytes(vm, key)
            .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
        if bytes.as_ref() == b"-during" {
            *value = prior.clone();
            return ValueOps::new_dict_checked(vm, pairs)
                .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()));
        }
    }
    pairs.push((Value::string("-during"), prior.clone()));
    ValueOps::new_dict_checked(vm, pairs)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))
}

/// Compare original trap elements through the native physical equality owner.
fn errorcode_prefix_match(
    vm: &mut Vm,
    pattern: &Value,
    errorcode: &Value,
) -> Result<bool, Completion<Value>> {
    let pattern = ValueOps::list_elements(vm, pattern)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
    let code = ValueOps::list_elements(vm, errorcode)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
    if pattern.len() > code.len() {
        return Ok(false);
    }
    for (left, right) in pattern.iter().zip(&code) {
        let equal = tcl_syntax::native_equality::full_native_equality(vm, left, right)
            .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
        if !equal {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Bind a handler's `[resultVar ?optionsVar?]` variables. A failed set becomes
/// the handler outcome (and skips the body), matching C's `handlerFailed`.
fn bind_handler_vars(
    vm: &mut Vm,
    vars: &Value,
    result: &Value,
    opts: &Value,
    body_code: Code,
    captured_error_code: Option<Value>,
) -> Result<(), Completion<Value>> {
    let names = ValueOps::list_elements(vm, vars)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
    let Some(policy) = vm.name_policy_protocol() else {
        return Err(vm.refuse_host_command("try variable binding protocol is unavailable".into()));
    };
    // `var_set`, not `set_var`: a handler variable written as an array element
    // (`on error {x(y)}`) must resolve `x(y)` to the element — and fail if the
    // base `x` is a scalar (`can't set "x(y)": variable isn't array`), which C's
    // `handlerFailed` turns into the handler outcome, skipping the body.
    if let Some(rv) = names.first() {
        let name = ValueOps::native_string_bytes(vm, rv)
            .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
        if !(policy.recipe().is_jim084() && name.is_empty())
            && let Err(e) = vm.set_var_bytes(&name, result.clone())
        {
            return Err(e);
        }
    }
    if let Some(ov) = names.get(1) {
        let name = ValueOps::native_string_bytes(vm, ov)
            .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
        if !(policy.recipe().is_jim084() && name.is_empty()) {
            let options = if policy.recipe().is_jim084() {
                vm.current_jim_options_for_exit_code_with_error_code(body_code, captured_error_code)
                    .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?
            } else {
                opts.clone()
            };
            vm.set_var_bytes(&name, options)?;
        }
    }
    Ok(())
}

/// `try`'s handler-type words, in C table order (`TryObjCmd`'s `handlerNames`,
/// `tclCmdMZ.c`): `Tcl_GetIndexFromObj(…, "handler type", 0)`, so `f`/`o`/`t`
/// abbreviate and the empty word — a prefix of all three — is
/// `ambiguous handler type ""`. The type is resolved before the clause's
/// arity, as it is in C (`try {} x` → `bad handler type "x"`).
const HANDLER_TYPES: tcl_cmd_core::prefix::OptionTable<'static> =
    tcl_cmd_core::prefix::OptionTable::abbreviating("handler type", &["finally", "on", "trap"]);

fn clause_argument_error(
    vm: &mut Vm,
    clause: tcl_registry::NativeTryClauseArgument,
    message: impl Into<Vec<u8>>,
) -> Completion<Value> {
    if vm
        .name_policy_protocol()
        .is_some_and(|policy| policy.recipe().is_jim084())
    {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"try ?options? script ?on|trap code varlist script ...? ?finally script?\"",
        );
    }
    let Some(code) = vm
        .actual_native_invocation_dialect()
        .try_clause_argument_error_code(clause)
    else {
        return vm.refuse_host_command("try clause argument protocol is unavailable".to_owned());
    };
    crate::command::err_with_code(message.into(), code)
}

fn clause_failure(
    vm: &mut Vm,
    failure: tcl_registry::NativeTryClauseFailure,
    message: impl Into<Vec<u8>>,
) -> Completion<Value> {
    let Some(code) = vm
        .actual_native_invocation_dialect()
        .try_clause_failure_error_code(failure)
    else {
        return vm.refuse_host_command("try clause failure protocol is unavailable".into());
    };
    crate::command::err_with_code(message.into(), code)
}

fn original_handler_type(
    vm: &mut Vm,
    original: &Value,
    jim: bool,
) -> Result<&'static str, Completion<Value>> {
    let selected = if jim {
        const JIM_HANDLERS: &[&str] = &["on", "trap", "finally"];
        let table = tcl_registry::native_index_lookup::NativeStaticIndexTable::supported_backend(
            JIM_HANDLERS,
        );
        let index = vm
            .native_jim_enum_from_original(
                original,
                &table,
                tcl_registry::native_jim_enum::NativeJimEnumFlags(1),
                Some(b"handler"),
            )
            .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?
            .map_err(|message| {
                let mut details =
                    tcl_cmd_core::CmdError::new_bytes(message.expect("ERRMSG handler lookup"))
                        .into_byte_details();
                details.error_code = tcl_cmd_core::CmdErrorCodeUpdate::Unchanged;
                crate::command::completion_from_cmd_error(
                    vm,
                    tcl_cmd_core::CmdError::from_byte_details(details),
                )
            })?;
        JIM_HANDLERS[index]
    } else {
        let index = vm
            .native_static_option_index(original, HANDLER_TYPES.names(), false, "handler type")
            .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
        HANDLER_TYPES.names()[index]
    };
    Ok(selected)
}

fn original_handler_code(
    vm: &mut Vm,
    original: &Value,
    is_trap: bool,
    jim: bool,
    body_code: Option<Code>,
) -> Result<i64, Completion<Value>> {
    let selected = if is_trap {
        if let Err(error) = ValueOps::list_elements(vm, original) {
            if error.native_access_refusal().is_some() {
                return Err(crate::command::completion_from_cmd_error(vm, error.into()));
            }
            let original = ValueOps::native_string_bytes(vm, original)
                .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
            let mut message = b"bad prefix '".to_vec();
            message.extend_from_slice(tcl_core_types::c_string_extent(&original));
            message.extend_from_slice(b"': must be a list");
            return Err(clause_failure(
                vm,
                tcl_registry::NativeTryClauseFailure::TrapPrefixFormat,
                message,
            ));
        }
        1
    } else {
        let (mut ops, protocol) = crate::return_options::NativeReturnOps::selected(vm)
            .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
        if jim {
            let requested = ValueOps::list_elements(vm, original)
                .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
            let Some(body_code) = body_code else {
                return Err(vm.refuse_host_command("Jim handlers require a completed body".into()));
            };
            let mut matched = false;
            for original in requested {
                match tcl_cmd_core::return_options::parse_completion_code(
                    &mut ops, protocol, &original,
                ) {
                    Ok(code) if i64::from(code) == body_code.as_int() => {
                        matched = true;
                        break;
                    }
                    Ok(_) => {}
                    Err(error) if error.native_access_refusal().is_some() => {
                        return Err(crate::command::completion_from_cmd_error(vm, error));
                    }
                    Err(_) => {
                        return Err(crate::command::native_wrong_arguments_message(
                            vm,
                            "wrong # args: should be \"try ?options? script ?on|trap code varlist script ...? ?finally script?\"",
                        ));
                    }
                }
            }
            if matched {
                body_code.as_int()
            } else {
                i64::MIN
            }
        } else {
            i64::from(
                tcl_cmd_core::return_options::parse_completion_code(&mut ops, protocol, original)
                    .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?,
            )
        }
    };
    Ok(selected)
}

/// Parse a `try`'s handler (`on`/`trap`) and `finally` clauses, validating the
/// grammar (a bad clause errors before the body runs). Returns the handlers and
/// the optional `finally` script.
fn parse_clauses(
    vm: &mut Vm,
    rest: &[Value],
    body_code: Option<Code>,
) -> Result<(Vec<Handler>, Option<Value>), Completion<Value>> {
    let mut handlers: Vec<Handler> = Vec::new();
    let mut finally: Option<Value> = None;
    let mut j = 0;
    while j < rest.len() {
        let Some(policy) = vm.name_policy_protocol() else {
            return Err(vm.refuse_host_command("try clause protocol is unavailable".into()));
        };
        let jim = policy.recipe().is_jim084();
        let handler_type = original_handler_type(vm, &rest[j], jim)?;
        match handler_type {
            "finally" => {
                if j + 2 < rest.len() {
                    return Err(if jim {
                        crate::command::native_wrong_arguments_message(
                            vm,
                            "wrong # args: should be \"try ?options? script ?on|trap code varlist script ...? ?finally script?\"",
                        )
                    } else {
                        clause_failure(
                            vm,
                            tcl_registry::NativeTryClauseFailure::FinallyNonterminal,
                            b"finally clause must be last".to_vec(),
                        )
                    });
                }
                if j + 1 >= rest.len() {
                    return Err(clause_argument_error(
                        vm,
                        tcl_registry::NativeTryClauseArgument::Finally,
                        b"wrong # args to finally clause: must be \"... finally script\"".to_vec(),
                    ));
                }
                finally = Some(rest[j + 1].clone());
                j += 2;
            }
            kind @ ("on" | "trap") => {
                if j + 4 > rest.len() {
                    return Err(clause_argument_error(vm, if kind == "on" {
                        tcl_registry::NativeTryClauseArgument::On
                    } else {
                        tcl_registry::NativeTryClauseArgument::Trap
                    }, format!(
                        "wrong # args to {kind} clause: must be \"... {kind} {} variableList script\"",
                        if kind == "on" { "code" } else { "pattern" }
                    ).into_bytes()));
                }
                let is_trap = kind == "trap";
                let code = original_handler_code(vm, &rest[j + 1], is_trap, jim, body_code)?;
                let Some(policy) = vm.name_policy_protocol() else {
                    return Err(vm.refuse_host_command("try clause protocol is unavailable".into()));
                };
                if !policy.recipe().is_jim084() {
                    ValueOps::list_elements(vm, &rest[j + 2]).map_err(|error| {
                        crate::command::completion_from_cmd_error(vm, error.into())
                    })?;
                }
                let script = rest[j + 3].clone();
                let script_bytes = ValueOps::native_string_bytes(vm, &script)
                    .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
                let is_dash = !policy.recipe().is_jim084()
                    && tcl_core_types::c_string_extent(&script_bytes) == b"-";
                handlers.push(Handler {
                    code,
                    is_trap,
                    pattern: rest[j + 1].clone(),
                    vars: rest[j + 2].clone(),
                    script,
                    is_dash,
                });
                j += 4;
            }
            // Unreachable: `HANDLER_TYPES` has exactly the three arms above.
            other => {
                return Err(err(format!(
                    "bad handler type \"{other}\": must be {}",
                    tcl_cmd_core::prefix::choice_list(HANDLER_TYPES.names())
                )));
            }
        }
    }
    if handlers.last().is_some_and(|h| h.is_dash) {
        return Err(clause_failure(
            vm,
            tcl_registry::NativeTryClauseFailure::BadFallthrough,
            b"last non-finally clause must not have a body of \"-\"".to_vec(),
        ));
    }
    Ok((handlers, finally))
}

fn jim_ignored_completion_code(vm: &mut Vm, code_name: &[u8]) -> Result<i64, Completion<Value>> {
    let names = [
        b"ok".as_slice(),
        b"error",
        b"return",
        b"break",
        b"continue",
        b"signal",
        b"exit",
        b"eval",
    ];
    let Some(integer_protocol) = vm
        .actual_native_invocation_dialect()
        .native_scalar_getter_protocol()
    else {
        return Err(vm.refuse_host_command("Jim try decimal switch protocol is unavailable".into()));
    };
    let Some(decimal) = integer_protocol.jim_decimal_wide_probe(code_name) else {
        return Err(vm.refuse_host_command("Jim try decimal switch protocol is unavailable".into()));
    };
    let code = match decimal {
        Ok(code) if (0..64).contains(&code) => code,
        Ok(code) if code >= 64 => {
            return Err(
                vm.refuse_host_command("Jim try ignore-mask shift exceeds its native width".into())
            );
        }
        _ => match names.iter().position(|name| *name == code_name) {
            Some(code) => i64::try_from(code).expect("fixed eight-name completion table"),
            None => {
                return Err(crate::command::native_wrong_arguments_message(
                    vm,
                    "wrong # args: should be \"try ?options? script ?on|trap code varlist script ...? ?finally script?\"",
                ));
            }
        },
    };
    Ok(code)
}

/// `try body ?handler ...? ?finally script?` — structured exception handling.
///
/// Parses and validates the grammar synchronously (unchanged), then defers the
/// body to the explicit stack via `vm.pending.try_phase` instead of
/// running it through `Vm::eval_source`. [`advance_try`] carries the
/// handler-matching / `finally` logic forward from there, one phase per
/// `Vm::unwind` fold.
fn cmd_try(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    const USAGE: &str = "wrong # args: should be \"try body ?handler ...? ?finally script?\"";
    let Some(policy) = vm.name_policy_protocol() else {
        return vm.refuse_host_command("try protocol is unavailable".into());
    };
    let jim = policy.recipe().is_jim084();
    let mut rest_args = args;
    let mut ignored_codes = vec![5, 6, 7];
    if jim {
        while rest_args.len() > 1 {
            let bytes = match ValueOps::native_string_bytes(vm, &rest_args[0]) {
                Ok(bytes) => bytes,
                Err(error) => return crate::command::completion_from_cmd_error(vm, error.into()),
            };
            let word = tcl_core_types::c_string_extent(&bytes);
            if word == b"--" {
                rest_args = &rest_args[1..];
                break;
            }
            if !word.starts_with(b"-") {
                break;
            }
            let (ignore, code_name) = if let Some(name) = word.strip_prefix(b"-no") {
                (true, name)
            } else {
                (false, &word[1..])
            };
            let code = match jim_ignored_completion_code(vm, code_name) {
                Ok(code) => code,
                Err(error) => return error,
            };
            ignored_codes.retain(|existing| *existing != code);
            if ignore {
                ignored_codes.push(code);
            }
            rest_args = &rest_args[1..];
        }
    }
    let Some((body, rest)) = rest_args.split_first() else {
        return crate::command::native_wrong_arguments_message(vm, USAGE);
    };
    let (handlers, finally, deferred_clauses) = if jim {
        if let Err(error) = vm.set_var_bytes(b"::errorCode", Value::string("NONE")) {
            return error;
        }
        (Vec::new(), None, Some(rest.to_vec()))
    } else {
        match parse_clauses(vm, rest, None) {
            Ok((handlers, finally)) => (handlers, finally, None),
            Err(error) => return error,
        }
    };
    let plan = Rc::new(TryPlan {
        handlers,
        finally,
        jim,
        deferred_clauses,
        ignored_codes,
        ignored_body: false,
        body_error_code: None,
    });
    match vm.prepare_script_commands_value(body) {
        Ok(prepared) if prepared.prefix.is_some() => {
            vm.pending.try_phase = Some(TryReq {
                script: prepared.prefix.expect("checked above"),
                state: TryState {
                    plan,
                    phase: TryPhase::Body,
                    fatal_tail: prepared.fatal_tail,
                },
            });
            ok(Value::empty())
        }
        Ok(prepared) => {
            let body_completion = prepared
                .fatal_tail
                .map_or_else(|| ok(Value::empty()), |tail| vm.raise_fatal_tail(tail));
            match advance_after_body(vm, &plan, body_completion) {
                TryOutcome::Push(req) => {
                    vm.pending.try_phase = Some(*req);
                    ok(Value::empty())
                }
                TryOutcome::Deliver(c) => c,
            }
        }
        // A body parse error is a regular runtime error, subject to the same
        // `on error`/`trap`/`finally` handling as any other body error: its
        // `Err(TclError)` becomes a plain `Completion{Error}` fed through
        // `advance_after_body`'s handler matching, not returned as a hard
        // failure that skips it (try-body-parse-error tclsh-pinned test).
        Err(error) => {
            let completion = crate::command::completion_from_tcl_error(vm, error);
            match advance_after_body(vm, &plan, completion) {
                TryOutcome::Push(req) => {
                    vm.pending.try_phase = Some(*req);
                    ok(Value::empty())
                }
                TryOutcome::Deliver(c) => c,
            }
        }
    }
}

/// Advance a `try`'s state machine once its current phase's activation
/// completes with `c`: either the next phase to push (a matched handler, or
/// `finally`), or the whole construct's final completion. Called from
/// `Vm::unwind` when a `try_ctx`-tagged frame completes.
pub(crate) fn advance_try(vm: &mut Vm, state: TryState, c: Completion<Value>) -> TryOutcome {
    let TryState {
        plan,
        phase,
        fatal_tail: _,
    } = state;
    match phase {
        TryPhase::Body => advance_after_body(vm, &plan, c),
        TryPhase::Handler { body_opts } => advance_after_handler(vm, &plan, &body_opts, c),
        TryPhase::Finally { outcome } => advance_after_finally(vm, &plan, outcome, c),
    }
}

/// The body just completed as `body_comp`: match a handler (if any), bind its
/// variables, and either push it as the next phase or fall through to
/// [`finish_body_or_handler`].
fn advance_after_body(vm: &mut Vm, plan: &Rc<TryPlan>, body_comp: Completion<Value>) -> TryOutcome {
    if let Some(refusal) = vm.refused_completion() {
        return TryOutcome::Deliver(refusal);
    }
    if let Some(clauses) = &plan.deferred_clauses {
        let captured_error_code = vm.get_var_bytes(b"::errorCode");
        let (handlers, finally) = match parse_clauses(vm, clauses, Some(body_comp.code)) {
            Ok(parsed) => parsed,
            Err(error) => return TryOutcome::Deliver(error),
        };
        let parsed = Rc::new(TryPlan {
            handlers,
            finally,
            jim: true,
            deferred_clauses: None,
            ignored_codes: plan.ignored_codes.clone(),
            ignored_body: plan.ignored_codes.contains(&body_comp.code.as_int()),
            body_error_code: captured_error_code,
        });
        return advance_after_body(vm, &parsed, body_comp);
    }
    if plan.ignored_body {
        return finish_body_or_handler(vm, plan, body_comp);
    }
    // `exit` is not catchable (C Tcl's `Tcl_Exit`): propagate the unwind
    // without running handlers or the `finally` clause.
    if vm.exit_pending() {
        return TryOutcome::Deliver(body_comp);
    }
    let errorcode = if plan.jim {
        plan.body_error_code.clone().unwrap_or_else(Value::empty)
    } else if body_comp.code == Code::Error {
        crate::command::resolved_error_code(&body_comp)
    } else {
        Value::empty()
    };
    // The body's options dict (bound to a handler's optionsVar and reused as the
    // `-during` chain link if a handler/finally throws over the body's exception).
    let body_opts = if plan.jim {
        match vm.current_jim_options_for_exit_code_with_error_code(
            body_comp.code,
            plan.body_error_code.clone(),
        ) {
            Ok(options) => options,
            Err(error) => {
                return TryOutcome::Deliver(crate::command::completion_from_cmd_error(
                    vm,
                    error.into(),
                ));
            }
        }
    } else {
        vm.completion_options_snapshot(&body_comp)
    };
    let mut matched = None;
    for (index, handler) in plan.handlers.iter().enumerate() {
        let applies = if handler.is_trap
            && (plan.jim || body_comp.code == Code::Error)
            && (!plan.jim || plan.body_error_code.is_some())
        {
            match errorcode_prefix_match(vm, &handler.pattern, &errorcode) {
                Ok(applies) => applies,
                Err(completion) => return TryOutcome::Deliver(completion),
            }
        } else {
            !handler.is_trap && handler.code == body_comp.code.as_int()
        };
        if applies {
            matched = Some(index);
            break;
        }
    }
    let Some(matched_index) = matched else {
        return finish_body_or_handler(vm, plan, body_comp);
    };
    // Scan forward over `-` fall-through bodies to the clause that runs.
    let mut handler_index = matched_index;
    while plan.handlers[handler_index].is_dash {
        handler_index += 1; // guaranteed to terminate (the last body is not `-`)
    }
    enter_matched_handler(vm, plan, handler_index, &body_comp, body_opts, &errorcode)
}

fn enter_matched_handler(
    vm: &mut Vm,
    plan: &Rc<TryPlan>,
    handler_index: usize,
    body_comp: &Completion<Value>,
    body_opts: Value,
    errorcode: &Value,
) -> TryOutcome {
    match bind_handler_vars(
        vm,
        &plan.handlers[handler_index].vars,
        &body_comp.result,
        &body_opts,
        body_comp.code,
        plan.body_error_code.clone(),
    ) {
        Ok(()) => {
            // The body's exception is now handled: publish `errorInfo`/
            // `errorCode` (so the handler reads the body's error) and reset the
            // trace so the handler's own errors start fresh.
            if body_comp.code == Code::Error && !plan.jim {
                let einfo = vm.take_error_info().unwrap_or_else(|| {
                    opt_get(&body_opts, "-errorinfo").map_or_else(
                        || body_comp.result.string_bytes().to_vec(),
                        |v| v.string_bytes().to_vec(),
                    )
                });
                vm.publish_error(&einfo, errorcode);
            }
            match vm.prepare_script_commands_value(&plan.handlers[handler_index].script) {
                Ok(prepared) if prepared.prefix.is_some() => TryOutcome::Push(Box::new(TryReq {
                    script: prepared.prefix.expect("checked above"),
                    state: TryState {
                        plan: Rc::clone(plan),
                        phase: TryPhase::Handler { body_opts },
                        fatal_tail: prepared.fatal_tail,
                    },
                })),
                Ok(prepared) => {
                    let completion = prepared
                        .fatal_tail
                        .map_or_else(|| ok(Value::empty()), |tail| vm.raise_fatal_tail(tail));
                    advance_after_handler(vm, plan, &body_opts, completion)
                }
                Err(error) => {
                    let completion = crate::command::completion_from_tcl_error(vm, error);
                    finish_body_or_handler(vm, plan, completion)
                }
            }
        }
        // A failed var bind also chains to the body (C's `handlerFailed`).
        Err(mut e) => {
            if plan.jim {
                let options = match vm.current_jim_options_for_exit_code(body_comp.code) {
                    Ok(options) => options,
                    Err(error) => {
                        return TryOutcome::Deliver(crate::command::completion_from_cmd_error(
                            vm,
                            error.into(),
                        ));
                    }
                };
                e = Completion::new(body_comp.code, e.result, options);
            }
            if e.code == Code::Error && !plan.jim {
                let options = vm.completion_options_snapshot(&e);
                e.options = match add_during(vm, &options, &body_opts) {
                    Ok(options) => options,
                    Err(completion) => return TryOutcome::Deliver(completion),
                };
            }
            finish_body_or_handler(vm, plan, e)
        }
    }
}

/// The matched handler just completed as `handler_comp`: it becomes the
/// outcome (chaining `-during` if it threw over the body's exception), then
/// falls through to [`finish_body_or_handler`].
fn advance_after_handler(
    vm: &mut Vm,
    plan: &Rc<TryPlan>,
    body_opts: &Value,
    handler_comp: Completion<Value>,
) -> TryOutcome {
    if let Some(refusal) = vm.refused_completion() {
        return TryOutcome::Deliver(refusal);
    }
    let mut outcome = handler_comp;
    if outcome.code == Code::Error && !plan.jim {
        let options = vm.completion_options_snapshot(&outcome);
        outcome.options = match add_during(vm, &options, body_opts) {
            Ok(options) => options,
            Err(completion) => return TryOutcome::Deliver(completion),
        };
    }
    finish_body_or_handler(vm, plan, outcome)
}

/// After the body (no handler matched, or a matched handler's var bind
/// failed) or after the matched handler ran: `finally` always runs next if
/// present, else `outcome` is the whole `try`'s final completion.
fn finish_body_or_handler(
    vm: &mut Vm,
    plan: &Rc<TryPlan>,
    mut outcome: Completion<Value>,
) -> TryOutcome {
    if let Some(refusal) = vm.refused_completion() {
        return TryOutcome::Deliver(refusal);
    }
    if plan.jim {
        outcome.options = match vm.current_jim_options_for_exit_code(outcome.code) {
            Ok(options) => options,
            Err(error) => {
                return TryOutcome::Deliver(crate::command::completion_from_cmd_error(
                    vm,
                    error.into(),
                ));
            }
        };
    } else if outcome.code == Code::Error {
        outcome.options = vm.completion_options_snapshot(&outcome);
    }
    let Some(fin) = plan.finally.clone() else {
        return TryOutcome::Deliver(outcome);
    };
    // Finally preserves a genuine saved result reference while another command runs.
    outcome.result = outcome.result.into_native_reference();
    if outcome.code == Code::Error && !plan.jim {
        let _ = vm.take_error_info();
    }
    // Compiled lazily here rather than in `cmd_try` up front: a `finally`
    // never runs before this point, so a
    // body/handler compile error is reported before `finally`'s own grammar
    // is ever touched.
    let prepared = match vm.prepare_script_commands_value(&fin) {
        Ok(prepared) => prepared,
        // A `finally` parse error is `finally`'s own exception overriding the
        // prior outcome, same as a runtime error in `finally` would (chains
        // `-during` to what `finally` superseded).
        Err(error) => {
            let completion = crate::command::completion_from_tcl_error(vm, error);
            return advance_after_finally(vm, plan, outcome, completion);
        }
    };
    let Some(script) = prepared.prefix else {
        let completion = prepared
            .fatal_tail
            .map_or_else(|| ok(Value::empty()), |tail| vm.raise_fatal_tail(tail));
        return advance_after_finally(vm, plan, outcome, completion);
    };
    TryOutcome::Push(Box::new(TryReq {
        script,
        state: TryState {
            plan: Rc::clone(plan),
            phase: TryPhase::Finally { outcome },
            fatal_tail: prepared.fatal_tail,
        },
    }))
}

/// `finally` just completed as `fc`: only its own non-`Ok` completion
/// overrides — chaining `-during` to the prior outcome's options if it threw.
fn advance_after_finally(
    vm: &mut Vm,
    plan: &Rc<TryPlan>,
    outcome: Completion<Value>,
    fc: Completion<Value>,
) -> TryOutcome {
    if let Some(refusal) = vm.refused_completion() {
        return TryOutcome::Deliver(refusal);
    }
    if plan.jim {
        let raw = if plan.ignored_body || fc.code == Code::Ok {
            outcome.code
        } else {
            fc.code
        };
        let result = if !plan.ignored_body && fc.code == Code::Ok {
            outcome.result
        } else {
            fc.result
        };
        let options = match vm.current_jim_options_for_exit_code(raw) {
            Ok(options) => options,
            Err(error) => {
                return TryOutcome::Deliver(crate::command::completion_from_cmd_error(
                    vm,
                    error.into(),
                ));
            }
        };
        return TryOutcome::Deliver(Completion::new(raw, result, options));
    }
    if fc.code == Code::Ok {
        vm.restore_completion_error_state(&outcome);
        return TryOutcome::Deliver(outcome);
    }
    let mut fc = fc;
    if fc.code == Code::Error {
        let prior_opts = completion_options(&outcome);
        let options = vm.completion_options_snapshot(&fc);
        fc.options = match add_during(vm, &options, &prior_opts) {
            Ok(options) => options,
            Err(completion) => return TryOutcome::Deliver(completion),
        };
    }
    TryOutcome::Deliver(fc)
}

/// `throw type message` — raise an error with `-errorcode type` (a non-empty
/// list). Equivalent to `return -code error -errorcode $type $message`.
fn cmd_throw(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [ty, msg] = args else {
        return crate::command::native_wrong_args(vm, "throw type message");
    };
    match tcl_syntax::value::ValueOps::list_elements(vm, ty) {
        Ok(parts) if !parts.is_empty() => {}
        Ok(_) => return err("type must be non-empty list"),
        Err(e) => return crate::command::completion_from_cmd_error(vm, e.into()),
    }
    let message = match tcl_syntax::value::ValueOps::native_string_bytes(vm, msg) {
        Ok(bytes) => bytes,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error.into()),
    };
    // Like `return -code error -errorcode $type $msg`: the message is the result,
    // the `while executing`/`invoked from within` trace accumulates as it unwinds.
    let options = options_dict(
        Code::Error,
        0,
        &[
            ("-errorcode", ty.clone()),
            ("-errorinfo", Value::from_string_bytes(message)),
        ],
    );
    Completion::new(Code::Error, msg.clone(), options)
}

#[cfg(test)]
mod native_fixture_tests {
    use super::*;

    fn bytes_from_hex(hex: &str) -> Vec<u8> {
        assert_eq!(hex.len() % 2, 0);
        hex.as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                let text = core::str::from_utf8(pair).unwrap();
                u8::from_str_radix(text, 16).unwrap()
            })
            .collect()
    }

    #[test]
    fn try_grammar_binding_and_finally_match_fixed_native_results() {
        const CASES: &str = include_str!("../tests/data/native_try/cases.tsv");
        let engines = [
            (
                "tcl8.6",
                include_str!("../tests/data/native_try/8.6.18.tsv"),
            ),
            ("tcl9.0", include_str!("../tests/data/native_try/9.0.4.tsv")),
            ("tcl9.1", include_str!("../tests/data/native_try/9.1.0.tsv")),
            ("jim", include_str!("../tests/data/native_try/Jim.tsv")),
        ];
        let mut compared = 0;
        for (engine, expected) in engines {
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            assert_eq!(CASES.lines().count(), expected.lines().count());
            for (input, expected) in CASES.lines().zip(expected.lines()) {
                let (name, source) = input.split_once('\t').unwrap();
                let mut wanted = expected.splitn(3, '\t');
                assert_eq!(wanted.next().unwrap(), name);
                let code: i64 = wanted.next().unwrap().parse().unwrap();
                let result = bytes_from_hex(wanted.next().unwrap());
                let source = bytes_from_hex(source);
                let mut vm = Vm::new();
                vm.set_dialect_profile(profile);
                vm.set_compiler(Box::new(
                    tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
                ));
                let completion = vm
                    .try_eval_source_bytes(&source)
                    .unwrap_or_else(|error| panic!("{engine}/{name}: {error:?}"));
                assert_eq!(completion.code.as_int(), code, "{engine}/{name}");
                let actual = ValueOps::native_string_bytes(&mut vm, &completion.result).unwrap();
                assert_eq!(actual.as_ref(), result.as_slice(), "{engine}/{name}");
                compared += 1;
            }
        }
        assert_eq!(compared, 60);
    }
}
