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

//! Built-in commands.
//!
//! A minimal set — `set`, `incr`, `return`, `unset` — sufficient to drive the
//! eval loop end to end and prove command substitution + variable integration.
//! The full builtin surface (string/list/dict/expr/control-flow/proc/…) lives
//! in the sibling `cmd_*` modules, each with its own tcltest coverage.
//!
//! Each handler matches the [`BuiltinFn`](crate::interp::BuiltinFn) shape:
//! `argv[0]` is the command name (Tcl's `objv` convention).

use tcl_registry::native_rmw::LegacyIncrementDialect;
use tcl_registry::native_rmw::NativeRmwDialect;
use tcl_syntax::expr::operators::{ALL_BIN_OPS, ALL_UNARY_OPS};

use crate::frame::VarError;
use crate::interp::native_arguments::OriginalArguments;
use crate::interp::{obj_bytes, Code, Interp};
// The transient `1` of a tower `incr` is the only fresh object left to drop
// by hand; every other path now stores through `store_var_result`.
use crate::interp::drop_fresh;
use crate::obj::{self, TclObj};

#[cfg(test)]
mod native_jim_library_tests;

/// Register the starter builtins on a fresh interp.
pub fn install(interp: &mut Interp) {
    install_for_bootstrap(interp, None);
}

/// Register core commands without sourcing distribution-owned procedures.
pub(crate) fn install_native_core(
    interp: &mut Interp,
    protocol: tcl_registry::special_vars::NativeBootstrapProtocol,
) {
    install_for_bootstrap(interp, Some(protocol));
}

fn install_for_bootstrap(
    interp: &mut Interp,
    protocol: Option<tcl_registry::special_vars::NativeBootstrapProtocol>,
) {
    interp.register_stock_builtin(b"set", set);
    interp.register_stock_builtin(b"incr", incr);
    interp.register_stock_builtin(b"const", const_cmd);
    interp.register_stock_builtin(b"return", ret);
    interp.register_stock_builtin(b"unset", unset);
    interp.register_stock_builtin(b"exit", exit_cmd);
    interp.register_stock_builtin(b"subst", subst_cmd);
    crate::cmd_scan::install(interp);
    crate::cmd_format::install(interp);
    if protocol.is_none_or(|protocol| protocol.registers_core_binary()) {
        crate::cmd_binary::install(interp);
    }
    crate::cmd_clock::install(interp);
    // `expr` needs the numeric tower (libtommath); registered only when linked.
    #[cfg(have_tommath)]
    interp.register_stock_builtin(b"expr", expr_cmd);
    // `::tcl::mathfunc::*` are real commands `expr`'s function path resolves
    // through (overridable); they need the tower too.
    #[cfg(have_tommath)]
    if protocol
        .is_none_or(tcl_registry::special_vars::NativeBootstrapProtocol::registers_core_mathfunc)
    {
        crate::cmd_mathfunc::install(interp);
    }
    // `::tcl::mathop::*` — the operators as real commands (tower-gated).
    #[cfg(have_tommath)]
    if protocol
        .is_none_or(tcl_registry::special_vars::NativeBootstrapProtocol::registers_core_mathop)
    {
        crate::cmd_mathop::install(interp);
    }
    crate::cmd_list::install(interp);
    #[cfg(have_tommath)]
    crate::cmd_lseq::install(interp);
    crate::cmd_dict::install(interp);
    crate::cmd_alias::install(interp);
    if protocol.is_some() {
        crate::cmd_namespace::install_core(interp);
    } else {
        crate::cmd_namespace::install(interp);
    }
    crate::cmd_var::install(interp);
    crate::cmd_control::install(interp);
    crate::cmd_proc::install(interp);
    if protocol.is_some() {
        crate::cmd_error::install_for_bootstrap(interp, protocol);
    } else {
        crate::cmd_error::install(interp);
    }
    crate::cmd_eval::install(interp);
    crate::cmd_info::install(interp);
    crate::cmd_jim_exists::install(interp);
    crate::cmd_array::install(interp);
    crate::cmd_switch::install(interp);
    crate::cmd_package::install(interp);
    crate::cmd_fs::install(interp);
    crate::cmd_misc::install(interp);
    crate::cmd_chan::install(interp);
    crate::cmd_zlib::install(interp);
    crate::cmd_trace::install(interp);
    // The event loop (`after`/`vwait`/`update`) — registers `update`, replacing
    // the bgerror-only stub in `cmd_alias`.
    crate::cmd_event::install(interp);
    crate::cmd_coro::install(interp);
    // `regexp`/`regsub`, on the pure-Rust `tcl-regex` ARE engine.
    crate::cmd_regex::install(interp);
    // TclOO last: its `variable`/`self`/`my`/`next` intentionally override the
    // base `variable` (OO-aware inside `oo::define`, forwarding otherwise).
    if protocol.is_none_or(|protocol| protocol.initializes_tcl_oo()) {
        crate::cmd_oo::install(interp);
    }
    // Register the spec-backed string implementation after the ordinary
    // startup sweep so its derived intrinsic identities remain attested.
    crate::cmd_string::install(interp);
    // With the handler table complete, attach the registry's identities to the
    // builtins in it.
    interp.attach_identities();
}

/// Commands that only a build with the numeric tower registers: `expr`, the
/// expression-driven control commands, `lseq`, and the `::tcl::mathop::*` and
/// `::tcl::mathfunc::*` families. A build without the tower registers none of
/// them and reports them as needing it
/// ([`tcl_runtime_api::RegisteredBacking::NeedsNumericTower`]), so a consumer
/// of the backing report reads one answer for the registration surface whether
/// or not libtommath was available when the runtime was built. Names are
/// without a leading `::`.
#[cfg(any(not(have_tommath), test))]
pub(crate) fn tower_command_names() -> Vec<String> {
    let mut names: Vec<String> = ["expr", "if", "while", "for", "lseq"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    names.extend(
        mathop_names()
            .into_iter()
            .map(|op| tcl_syntax::naming::unrooted_member_key("tcl::mathop", op)),
    );
    names.extend(
        mathfunc_names()
            .into_iter()
            .map(|name| tcl_syntax::naming::unrooted_member_key("tcl::mathfunc", name)),
    );
    names
}

/// Every operator spelling with a `::tcl::mathop` command form — derived from
/// `tcl_syntax::expr::operators`, the single source of truth for which
/// operators exist and whether they have a mathop command form at all,
/// rather than a hand-typed list that could silently drift from the
/// operator grammar it mirrors.
///
/// `BinOp`/`UnaryOp` share a spelling for `-`/`+` (`Sub`/`Neg`, `Add`/`Pos`) —
/// one command handles both the fold and the single-argument reading, so the
/// binary pass alone already covers them; the unary pass only contributes
/// truly unary-only spellings (`~`, `!`).
pub(crate) fn mathop_names() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = ALL_BIN_OPS
        .iter()
        .filter_map(|op| op.spec().mathop_shape.map(|_| op.spec().spelling))
        .collect();
    for op in ALL_UNARY_OPS {
        if op.spec().mathop_shape.is_some() {
            let spelling = op.spec().spelling;
            if !names.contains(&spelling) {
                names.push(spelling);
            }
        }
    }
    names
}

/// Every math function name — registered as `::tcl::mathfunc::<name>`. Most
/// forward to the shared [`dispatch`]; `rand`/`srand` are handled inline
/// (interp state).
///
/// Derived from `tcl_syntax::expr::mathfunc::all()` rather than a hand-typed
/// list, so this loop cannot drift out of sync with `dispatch()` (the
/// function it wires every one of these names up to): a function
/// `dispatch()` implements but a stale hand-typed list omitted would report
/// "invalid command name" instead of dispatching.
pub(crate) fn mathfunc_names() -> Vec<&'static str> {
    tcl_syntax::expr::mathfunc::all()
        .into_iter()
        .map(|spec| spec.name)
        .collect()
}

/// Report a store through an already split array reference. This display name
/// is never reparsed to select storage; keys containing parentheses remain intact.
pub(crate) fn var_element_error(
    interp: &mut Interp,
    base: &[u8],
    key: &[u8],
    error: VarError,
) -> Code {
    let mut display = Vec::with_capacity(base.len() + key.len() + 2);
    display.extend_from_slice(base);
    display.push(b'(');
    display.extend_from_slice(key);
    display.push(b')');
    var_error(interp, &display, error)
}

pub(crate) fn var_error(interp: &mut Interp, name: &[u8], e: VarError) -> Code {
    var_error_using(interp, name, e, None)
}

pub(crate) fn var_error_using(
    interp: &mut Interp,
    name: &[u8],
    e: VarError,
    stage: Option<&dyn Fn() -> Result<(), tcl_cmd_core::CmdError>>,
) -> Code {
    if let Some(check) = stage {
        if let Err(first) = check() {
            return interp.report_cmd_error(first);
        }
    }

    if e == VarError::NameProtocolUnavailable {
        return interp.refuse_native_access(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "variable naming",
            ),
        );
    }
    // A write-trace error: wrap the trace's own message (stashed in pending_err)
    // as `can't set "name": <msg>` (C's TclObjVarErrMsg), *keeping* the
    // errorInfo chain the trace machinery already built.
    if e == VarError::TraceError {
        let reason = interp
            .traces
            .borrow_mut()
            .pending_err
            .take()
            .unwrap_or_default();
        return interp.var_trace_error_using(name, b"write", &reason, stage);
    }
    if e == VarError::Confined {
        return interp.confined_store_error_using(name, stage);
    }
    let verb = match e {
        VarError::NameProtocolUnavailable => unreachable!("host refusal handled above"),
        VarError::IsArray => &b"\": variable is array"[..],
        VarError::IsScalar => &b"\": variable isn't array"[..],
        VarError::DeletedNamespace => &b"\": upvar refers to variable in deleted namespace"[..],
        VarError::DeletedArray => &b"\": upvar refers to element in deleted array"[..],
        VarError::NoSuchNamespace => &b"\": parent namespace doesn't exist"[..],
        VarError::IsConstant => &b"\": variable is a constant"[..],
        VarError::TraceError | VarError::Confined => unreachable!("handled above"),
    };
    let mut msg = b"can't set \"".to_vec();
    msg.extend_from_slice(name);
    msg.extend_from_slice(verb);
    interp.error_using(&msg, stage)
}

/// `can't incr "name": variable is a constant` when `incr` targets a `const`
/// scalar (C checks this before the read-modify-write, so no read trace fires).
fn incr_constant_error(
    interp: &mut Interp,
    base: &[u8],
    elem: &Option<Vec<u8>>,
    name: &[u8],
) -> Option<Code> {
    if elem.is_none() && interp.is_constant(base) {
        let mut msg = b"can't incr \"".to_vec();
        msg.extend_from_slice(name);
        msg.extend_from_slice(b"\": variable is a constant");
        return Some(interp.set_error(&msg));
    }
    None
}

/// `const varName value` (TIP 677) — set `varName` and flag it unmodifiable.
/// Re-`const`'ing an existing constant is a silent no-op; any other existing
/// variable, an array, or an array element is an error.
fn const_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return interp.wrong_args(b"const varName value");
    }
    let name = obj_bytes(argv[1]);
    let (base, elem) = match interp.variable_name_parts(&name) {
        Ok(parts) => parts,
        Err(error) => return var_error(interp, &name, error),
    };
    // `const X(a)` — a constant may not be an array element.
    if elem.is_some() {
        return make_constant_error(interp, &name, b"name refers to an element in an array");
    }
    // `const X` where X is already an array.
    if interp.var_is_array(&base) {
        return make_constant_error(interp, &name, b"variable is array");
    }
    // Already defined: a constant is a no-op; anything else already exists.
    if interp.var_exists(&base) {
        if interp.is_constant(&base) {
            interp.set_result_bytes(b""); // C's `const` yields an empty result
            return Code::Ok;
        }
        return make_constant_error(interp, &name, b"variable already exists");
    }
    // Set the value (firing write traces; a trace error aborts the const), then
    // flag the cell constant.
    match interp.var_set(&base, argv[2]) {
        Ok(()) => {
            interp.mark_constant(&base);
            interp.set_result_bytes(b""); // C's `const` yields an empty result
            Code::Ok
        }
        Err(e) => var_error(interp, &name, e),
    }
}

/// `can't make constant "name": <reason>` (the `const` command's lookup errors).
fn make_constant_error(interp: &mut Interp, name: &[u8], reason: &[u8]) -> Code {
    let mut msg = b"can't make constant \"".to_vec();
    msg.extend_from_slice(name);
    msg.extend_from_slice(b"\": ");
    msg.extend_from_slice(reason);
    interp.set_error(&msg)
}

// set

/// `set varName ?value?` — write (returns the value) or read (returns it).
fn set(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if interp.observed_name_policy_selected() && matches!(argv.len(), 2 | 3) {
        let name = obj_bytes(argv[1]);
        let level = interp.frames.borrow().current_level();
        return match argv.len() {
            2 => match interp.observed_variable_get(&name, level) {
                Ok(Some(value)) => {
                    interp.set_result(value);
                    Code::Ok
                }
                Ok(None) => {
                    let _ = interp.observed_refusal::<()>();
                    Code::Error
                }
                Err(_) => Code::Error,
            },
            _ => match interp.observed_variable_set(&name, level, argv[2]) {
                Ok(()) => {
                    interp.set_result(argv[2]);
                    Code::Ok
                }
                Err(_) => Code::Error,
            },
        };
    }
    match argv.len() {
        2 => {
            let name = obj_bytes(argv[1]);
            let (base, elem) = match interp.variable_name_parts(&name) {
                Ok(parts) => parts,
                Err(error) => return var_error(interp, &name, error),
            };
            // A read trace fires before the read (C's Tcl_ObjGetVar2); a trace
            // error fails the read with `can't read "name": <msg>`.
            if let Some(c) = interp.fire_read_trace(&base, elem.as_deref()) {
                return c;
            }
            let val = match &elem {
                Some(k) => interp.var_get_elem(&base, k),
                None => interp.var_get(&base),
            };
            match val {
                Some(o) => {
                    interp.set_result(o);
                    Code::Ok
                }
                None => {
                    // The C three-way distinction (`tclVar.c`): scalar read of an
                    // array ("variable is array"), missing element of an existing
                    // array ("no such element in array"), or wholly missing
                    // variable ("no such variable").
                    let msg = interp.read_miss_msg(&base, elem.as_deref());
                    interp.set_error(&msg)
                }
            }
        }
        3 => {
            let name = obj_bytes(argv[1]);
            let (base, elem) = match interp.variable_name_parts(&name) {
                Ok(parts) => parts,
                Err(error) => return var_error(interp, &name, error),
            };
            let value = argv[2];
            let stored = match &elem {
                Some(k) => interp.var_set_elem(&base, k, value),
                None => interp.var_set(&base, value),
            };
            match stored {
                Ok(()) => {
                    interp.set_result(value);
                    Code::Ok
                }
                Err(e) => var_error(interp, &name, e),
            }
        }
        _ => interp.wrong_args_for_invocation(argv, b"varName ?newValue?"),
    }
}

// incr

/// `incr varName ?increment?` — add (default 1) over the **numeric tower**,
/// storing and returning the sum. Both operands must be integers; the sum
/// follows the selected native overflow policy, and an existing
/// bignum cell increments correctly. Object-preserving (reads the cell's value
/// object, not its string).
pub(crate) fn incr(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 || argv.len() > 3 {
        return interp.wrong_args(b"incr varName ?increment?");
    }
    let arguments = match OriginalArguments::enter(interp, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    let code = incr_original(interp, argv, &arguments, None);
    arguments.finish(code)
}

pub(crate) fn incr_with_cached_sum(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    hint: (i64, i64),
) -> Code {
    if argv.len() < 2 || argv.len() > 3 {
        return interp.wrong_args(b"incr varName ?increment?");
    }
    let arguments = match OriginalArguments::enter(interp, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    let code = incr_original(interp, argv, &arguments, Some(hint));
    arguments.finish(code)
}

fn incr_original(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    arguments: &OriginalArguments,
    cached_sum: Option<(i64, i64)>,
) -> Code {
    let legacy_amount = if let Some(protocol) = interp
        .native_invocation_dialect()
        .native_legacy_increment_protocol()
    {
        let recipe = protocol.recipe();
        Some(match argv.get(2) {
            Some(original) => match tcl_cmd_core::native_increment::prepare_legacy_amount(
                &mut crate::value_ops::RuntimeLegacyIncrementAmountOps(interp),
                recipe,
                original,
            ) {
                Ok(amount) => amount,
                Err(error) => return interp.report_cmd_error(error),
            },
            None => tcl_cmd_core::native_increment::default_legacy_amount(recipe),
        })
    } else {
        None
    };
    if let Err(error) = arguments.check() {
        return interp.report_cmd_error(error);
    }
    if interp.native_c_variable_name_protocol().is_some() {
        return match cached_sum {
            Some(hint) => interp.increment_original_c_variable_with_sum(
                argv[1],
                argv.get(2).copied(),
                legacy_amount,
                Some(hint),
            ),
            None => {
                interp.increment_original_c_variable(argv[1], argv.get(2).copied(), legacy_amount)
            }
        };
    }
    if let Some(amount) = legacy_amount {
        if interp
            .native_invocation_dialect()
            .native_jim_lookup_protocol()
            .is_some()
        {
            let code = interp.increment_native_jim_original(argv[1], amount);
            return arguments.finish(code);
        }
    }
    let name = match arguments.bytes(1) {
        Ok(name) => name,
        Err(error) => return interp.report_cmd_error(error),
    };
    let (base, elem) = match interp.variable_name_parts(&name) {
        Ok(parts) => parts,
        Err(error) => return var_error(interp, &name, error),
    };
    if let Some(c) = incr_constant_error(interp, &base, &elem, &name) {
        return c;
    }
    if let Some(amount) = legacy_amount {
        if interp.invocation_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            return interp.increment_native_jim_original(argv[1], amount);
        }
        if let Some(code) = interp.increment_captured(
            &name,
            &base,
            elem.as_deref(),
            argv.get(2).copied(),
            Some(amount),
        ) {
            return code;
        }
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native legacy increment container",
            )
            .into(),
        );
    }

    let normalized_amount = match (
        argv.get(2),
        interp
            .native_invocation_dialect()
            .native_rmw_amount_grammar(tcl_registry::native_rmw::NativeRmwOperation::Increment),
    ) {
        (
            Some(&amount),
            Some(grammar @ tcl_registry::native_rmw::NativeRmwAmountGrammar::SafeIntegerExpression),
        ) => match grammar.prepare(interp, &amount) {
            Ok(value) => Some(crate::obj::Owned::fresh(value)),
            Err(error) => {
                if let Err(first) = arguments.check() {
                    return interp.report_cmd_error(first);
                }
                match error {
                    tcl_syntax::value::IntegerOperandError::Integer(error) => {
                        return crate::value_ops::integer_error(interp, error);
                    }
                    error @ tcl_syntax::value::IntegerOperandError::SafeExpression => {
                        let bytes = match arguments.bytes(2) {
                            Ok(bytes) => bytes,
                            Err(error) => return interp.report_cmd_error(error),
                        };
                        let Some(message) = error.safe_expression_message_bytes(&bytes) else {
                            return interp.report_cmd_error(
                                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                                    "selected increment amount presentation",
                                )
                                .into(),
                            );
                        };
                        return interp.set_error(&message);
                    }
                }
            }
        },
        _ => None,
    };
    let selected_amount = normalized_amount
        .as_ref()
        .map(crate::obj::Owned::as_ptr)
        .or_else(|| argv.get(2).copied());

    if interp
        .native_invocation_dialect()
        .native_rmw_amount_validation(tcl_registry::native_rmw::NativeRmwOperation::Increment)
        == Some(tcl_registry::native_rmw::NativeRmwAmountValidation::BeforeRead)
    {
        if let Some(amount) = selected_amount {
            match tcl_syntax::value::ValueOps::int_add(interp, None, &amount) {
                Ok(validated) => drop_fresh(validated),
                Err(error) => return crate::value_ops::integer_error(interp, error),
            }
        }
    }

    if let Some(code) =
        interp.increment_captured(&name, &base, elem.as_deref(), selected_amount, None)
    {
        return code;
    }

    // Current cell value (borrowed; `None` for an unset variable → the shared
    // seam treats it as 0). The increment is `argv[2]` (borrowed) or a fresh 1.
    //
    // The read goes through the read-trace chokepoint: C's `TclPtrIncrObjVar`
    // fetches with `TclPtrGetVarIdx`, so `incr x` on a read-traced `x` fires
    // `read` and then `write`. A read trace that *errors* leaves
    // the fetch NULL, which C counts as 0 — so the error is swallowed here too,
    // exactly as `lappend` does.
    let cur = match interp.read_for_update(&base, elem.as_deref()) {
        Ok(value) => value,
        Err(first) => return interp.refuse_native_execution(first),
    };
    let one = obj::new_wide_int_obj(1);
    let amount = selected_amount.unwrap_or(one);

    // The numeric-tower addition is the shared `ValueOps::int_add` seam, run over
    // this runtime's bignum (overflow widens; a non-integer operand is the
    // canonical `expected integer but got "…"`). The store below — with its write
    // traces and the per-runtime result protocol — stays here, since a write
    // trace that errors must still store yet fail the command.
    let sum = tcl_syntax::value::ValueOps::int_add(interp, cur.as_ref(), &amount);
    drop_fresh(one); // the transient `1` (used or not) is no longer needed
    let sum = match sum {
        Ok(s) => s, // rc 0
        Err(e) => return crate::value_ops::integer_error(interp, e),
    };

    // The protected store: a write trace that rewrites or unsets
    // the cell drops the store's reference to this fresh sum, so a bare
    // `set_result` would read freed memory. the installed command and
    // compiled ABI share this same protected body — a direct
    // caller (the codegen ABI was one) must not be able to reach a use-after-free.
    match interp.store_var_result(&base, elem.as_deref(), sum) {
        Ok(()) => Code::Ok,
        Err(e) => var_error(interp, &name, e),
    }
}

// return

/// `exit ?returnCode?` — record the requested exit code and unwind uncatchably.
///
/// The embedded runtime never terminates the host process (that would kill the
/// LSP / analysis server it is embedded in). Instead it records the code (so
/// `catch` re-propagates while it is pending and the embedder can read it via
/// [`Interp::take_exit`]) and returns `Error` to unwind out of the script.
fn exit_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let code: i32 = match argv.len() {
        1 => 0,
        2 => {
            let bytes = obj_bytes(argv[1]);
            let dialect = interp.native_invocation_dialect();
            match dialect.process_exit_conversion.and_then(|conversion| {
                tcl_syntax::number::Numbers::Target(dialect.numbers)
                    .parse_exit_status(&String::from_utf8_lossy(&bytes), conversion)
            }) {
                Some(status) => status,
                None => {
                    let mut m = b"expected integer but got \"".to_vec();
                    m.extend_from_slice(&bytes);
                    m.push(b'"');
                    return interp.error_with_code(&m, b"TCL VALUE NUMBER");
                }
            }
        }
        _ => {
            return interp
                .wrong_arguments_message(b"wrong # args: should be \"exit ?returnCode?\"");
        }
    };
    interp.set_exit(code);
    interp.set_result_bytes(b"");
    Code::Error
}

/// `return ?-code code? ?-level n? ?-errorcode list? ?-errorinfo info?
/// ?-options dict? ?result?` — complete with `-code` after unwinding `-level`
/// proc/source boundaries (`Tcl_ReturnObjCmd`). A `-options` dict (as produced
/// by `catch`) seeds the options; explicit flags override it.
pub(crate) fn ret(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let code = crate::return_options::command(interp, &argv[1..]);
    if let Err(error) = interp.record_original_namespace_info_return(argv, code) {
        return interp.report_cmd_error(error.into());
    }
    code
}

// unset

/// `unset varName ...` — remove variables (scalars or array elements).
fn unset(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let arguments = match OriginalArguments::enter(interp, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    let code = unset_original(interp, argv, &arguments);
    arguments.finish(code)
}

fn unset_original(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    arguments: &OriginalArguments,
) -> Code {
    let Some(protocol) = interp.native_invocation_dialect().unset_option_protocol() else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("unset option grammar")
                .into(),
        );
    };
    let options = match protocol.parse(argv.len() - 1, |index| {
        arguments
            .bytes(index + 1)
            .map_err(|error| interp.report_cmd_error(error))
    }) {
        Ok(options) => options,
        Err(code) => return code,
    };
    let i = options.names_from + 1;
    let nocomplain = !options.complain;
    for index in i..argv.len() {
        let a = match arguments.original(index) {
            Ok(original) => original,
            Err(error) => return interp.report_cmd_error(error),
        };
        // A confined interpreter refuses the unset before any name policy
        // looks the variable up, and `-nocomplain` does not excuse it.
        if interp.stores_confined() {
            let name = match arguments.bytes(index) {
                Ok(name) => name,
                Err(error) => return interp.report_cmd_error(error),
            };
            match interp.variable_name_parts(&name) {
                Ok((base, _)) if interp.store_escapes(&base) => {
                    return interp.confined_unset_error(&name);
                }
                Ok(_) => {}
                Err(error) => return var_error(interp, &name, error),
            }
        }
        if interp.observed_name_policy_selected() {
            let name = match arguments.bytes(index) {
                Ok(name) => name,
                Err(error) => return interp.report_cmd_error(error),
            };
            let level = interp.frames.borrow().current_level();
            let reached = interp.observed_variable_unset(&name, level);
            if let Err(error) = arguments.check() {
                return interp.report_cmd_error(error);
            }
            match reached {
                Ok(true) => continue,
                Ok(false) if nocomplain => continue,
                Ok(false) => {
                    let _ = interp.observed_refusal::<()>();
                    return Code::Error;
                }
                Err(_) => return Code::Error,
            }
        }
        if interp.native_c_variable_name_protocol().is_some() {
            let reached = interp.unset_original_c_variable(a, !nocomplain);
            if let Err(error) = arguments.check() {
                return interp.report_cmd_error(error);
            }
            if let Err(code) = reached {
                return code;
            }
            continue;
        }
        if interp
            .native_invocation_dialect()
            .native_jim_lookup_protocol()
            .is_some()
        {
            let reached = interp.unset_original_jim_variable(a);
            if let Err(error) = arguments.check() {
                return interp.report_cmd_error(error);
            }
            match reached {
                Ok(true) => continue,
                Ok(false) if nocomplain => continue,
                Ok(false) => {
                    let name = match arguments.bytes(index) {
                        Ok(name) => name,
                        Err(error) => return interp.report_cmd_error(error),
                    };
                    let mut message = b"can't unset \"".to_vec();
                    message.extend_from_slice(&name);
                    message.extend_from_slice(b"\": no such variable");
                    return interp.set_error(&message);
                }
                Err(_) if nocomplain && !interp.host_refusal_pending() => continue,
                Err(code) => return code,
            }
        }
        let name = match arguments.bytes(index) {
            Ok(name) => name,
            Err(error) => return interp.report_cmd_error(error),
        };
        let (base, elem) = match interp.variable_name_parts(&name) {
            Ok(parts) => parts,
            Err(error) => return var_error(interp, &name, error),
        };
        if interp.store_escapes(&base) {
            return interp.confined_unset_error(&name);
        }
        // A constant cannot be unset; `-nocomplain` leaves it in place silently
        // (var-26.11/26.12), otherwise it is an error.
        if elem.is_none() && interp.is_constant(&base) {
            if nocomplain {
                continue;
            }
            let mut msg = b"can't unset \"".to_vec();
            msg.extend_from_slice(&name);
            msg.extend_from_slice(b"\": variable is a constant");
            return interp.set_error(&msg);
        }
        let existed = match &elem {
            Some(k) => interp.var_unset_elem(&base, k),
            None => interp.var_unset(&base),
        };
        if let Err(error) = arguments.check() {
            return interp.report_cmd_error(error);
        }
        if !existed && !nocomplain {
            let mut msg = b"can't unset \"".to_vec();
            msg.extend_from_slice(&name);
            msg.extend_from_slice(b"\": no such variable");
            return interp.set_error(&msg);
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `subst ?-nobackslashes? ?-nocommands? ?-novariables? string` — perform the
/// requested substitutions on `string` (default: all three). Errors from an
/// unset variable or a failing command substitution propagate.
fn subst_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    use tcl_registry::substitution::{NativeSubstitutionOptions, SubstitutionOptionError};
    let Some(options) = NativeSubstitutionOptions::select(interp.native_invocation_dialect())
    else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original Subst option declaration is unavailable",
            )
            .into(),
        );
    };
    if argv.len() < 2 {
        return interp.wrong_args(options.usage().as_bytes());
    }
    let table = tcl_cmd_core::prefix::OptionTable::abbreviating(options.noun(), options.names());
    let mut selected = Vec::with_capacity(argv.len() - 2);
    for &opt in &argv[1..argv.len() - 1] {
        match table.index_of_original(interp, &opt) {
            Ok(index) => selected.push(index),
            Err(error) => return interp.report_cmd_error(error),
        }
    }
    let kinds = match options.kinds(&selected) {
        Ok(kinds) => kinds,
        Err(SubstitutionOptionError::MixedFamilies) => {
            return interp.set_error(b"cannot combine positive and negative options");
        }
        Err(SubstitutionOptionError::UnknownOption) => {
            return interp.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "foreign Subst option ordinal",
                )
                .into(),
            );
        }
    };
    let flags = crate::subst::SubstFlags {
        backslashes: kinds.backslashes,
        cmds: kinds.commands,
        vars: kinds.variables,
    };
    let last = argv[argv.len() - 1];
    if interp
        .invocation_string_protocol()
        .is_some_and(|protocol| protocol.is_jim084())
    {
        let flags = u8::from(!flags.vars)
            | (u8::from(!flags.cmds) << 1)
            | (u8::from(!flags.backslashes) << 2)
            | 128;
        return match interp.substitute_native_jim_original(last, flags) {
            Ok(value) => {
                interp.set_result(value);
                Code::Ok
            }
            Err(code) => code,
        };
    }
    if matches!(
        interp.native_invocation_dialect().tcl_version,
        Some(
            tcl_dialect::TclVersion::V8_6
                | tcl_dialect::TclVersion::V9_0
                | tcl_dialect::TclVersion::V9_1
        )
    ) {
        return interp.substitute_original_c_template(
            last,
            tcl_runtime_api::native_substitution::NativeSubstitutionFlags::new(
                flags.backslashes,
                flags.cmds,
                flags.vars,
            ),
        );
    }
    let src = obj_bytes(last);
    // TIP 280: a `[...]` inside the substituted string reports the line it sits
    // on, derived from the argument word's recorded source location.
    let loc = interp.arg_location(last);
    match interp.do_subst_located(&src, flags, loc) {
        Ok(bytes) => {
            match interp.native_invocation_dialect().native_string_protocol() {
                Some(protocol) => interp.set_result_bytes(&protocol.host_decoded_bytes(&bytes)),
                None => interp.set_result_bytes(&bytes),
            }
            Code::Ok
        }
        Err(code) => code, // the failing sub already set the result
    }
}

// helpers

// expr

/// The interp's [`ExprCtx`](crate::expr::ExprCtx): `$var` resolves through the
/// frame store (preserving the value's object → `$bignum` stays a bignum), and
/// `[cmd]` recurses through the eval loop.
#[cfg(have_tommath)]
struct InterpExprCtx<'a> {
    interp: &'a mut Interp,
    /// Set when a completion code propagated out of a sub-evaluation (`[cmd]`, a
    /// math function, or a `$arr(idx)` index subst) rather than originating in
    /// `expr` itself. For an *error* the inner command already built the
    /// `::errorInfo` trace, so `expr` must preserve it (and add no frame of its
    /// own) — matching C, where such an error is logged at the inner command, not
    /// at `expr`. For a non-error code (`return`/`break`/`continue` from a `[cmd]`
    /// substitution) the code propagates out of the whole `expr` (and thus out of
    /// the enclosing `if`/`while`/`for` condition).
    propagated: bool,
    /// The completion code carried by `propagated` (only meaningful when
    /// `propagated` is set). Defaults to `Error`; a `[cmd]` substitution that
    /// completes with `return`/`break`/`continue` records that code here.
    propagated_code: Code,
}

/// Original arithmetic over existing owned operands, sharing the interpreter
/// expression context and its selected getter/currency/error owners.
#[cfg(have_tommath)]
pub(crate) fn eval_binary_operands(
    interp: &mut Interp,
    op: tcl_syntax::expr::BinOp,
    left: crate::obj::Owned,
    right: crate::obj::Owned,
) -> Result<crate::obj::Owned, crate::expr_error::ExprError> {
    let mut context = InterpExprCtx {
        interp,
        propagated: false,
        propagated_code: Code::Error,
    };
    crate::expr::eval_binary_operands(op, left, right, &mut context)
}

/// Enter an actual math-function operand operation through the same
/// interpreter expression context as arithmetic and Boolean producers.
#[cfg(have_tommath)]
pub(crate) fn math_operation(
    interp: &mut Interp,
) -> Result<crate::expr::MathOperation, crate::expr_error::ExprError> {
    crate::expr::MathOperation::enter(&mut InterpExprCtx {
        interp,
        propagated: false,
        propagated_code: Code::Error,
    })
}

#[cfg(have_tommath)]
impl InterpExprCtx<'_> {
    fn propagated_error(&mut self, code: Code) -> crate::expr_error::ExprError {
        self.propagated = true;
        self.propagated_code = code;
        if let Some(error) = self.interp.native_execution_refusal() {
            return crate::expr_error::ExprError::from_execution_refusal(error);
        }
        match self.interp.completion_result_bytes_checked() {
            Ok(message) => crate::expr_error::ExprError::from_bytes(message),
            Err(error) => crate::expr_error::ExprError::from_execution_refusal(error),
        }
    }

    fn read_selected_variable(
        &mut self,
        base: &[u8],
        elem: Option<Vec<u8>>,
        substitute_index: bool,
    ) -> Result<crate::obj::Owned, crate::expr_error::ExprError> {
        // `expr {$arr($i)}`: the array index is itself substituted (Tcl parses
        // `$name(index)` with `$`/`[`/`\` substitution in the index).
        let elem = match elem {
            Some(k) if substitute_index && k.iter().any(|&c| matches!(c, b'$' | b'[' | b'\\')) => {
                match self.interp.do_expression_subst(&k, false) {
                    Ok(v) => Some(v),
                    // The index's `[cmd]` completed with a non-OK code — carry it
                    // (error, or `return`/`break`/`continue`) out of the whole
                    // expression, exactly like a `[cmd]` operand.
                    Err(code) => {
                        return Err(self.propagated_error(code));
                    }
                }
            }
            other => other,
        };
        if elem.is_none() {
            if let Some(value) = self.interp.direct_variable_value(base) {
                return Ok(crate::obj::Owned::retain(value));
            }
        }
        if let Some(code) = self.interp.fire_read_trace(base, elem.as_deref()) {
            return Err(self.propagated_error(code));
        }
        if let Some(error) = self.interp.native_execution_refusal() {
            return Err(crate::expr_error::ExprError::from_execution_refusal(error));
        }
        let obj = match &elem {
            Some(k) => self.interp.var_get_elem(base, k),
            None => self.interp.var_get(base),
        };
        match obj {
            Some(o) => Ok(crate::obj::Owned::retain(o)),
            None => {
                let m = self.interp.read_miss_msg(base, elem.as_deref());
                Err(crate::expr_error::ExprError::from_bytes(m))
            }
        }
    }
}

#[cfg(have_tommath)]
impl crate::expr::ExprCtx for InterpExprCtx<'_> {
    fn boolean_interpreter(&mut self) -> Option<&mut Interp> {
        Some(self.interp)
    }
    fn numeric_host(&self) -> Option<std::rc::Rc<dyn tcl_platform::Host>> {
        Some(self.interp.host())
    }
    fn invocation_dialect(&self) -> tcl_native_model::InvocationDialect {
        self.interp.native_invocation_dialect()
    }
    fn f5_string_predicate_provider(
        &self,
    ) -> Option<tcl_syntax::expr::operators::AuthoredF5StringPredicateProvider> {
        tcl_registry::native_expression_program::authored_f5_string_predicate_provider(
            self.interp.expression_evaluation_policy().as_ref(),
        )
    }
    fn read_var(&mut self, name: &str) -> Result<crate::obj::Owned, crate::expr_error::ExprError> {
        let (base, elem) = self
            .interp
            .variable_name_parts(name.as_bytes())
            .map_err(|_| {
                crate::expr_error::ExprError::host_refusal(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "variable naming",
                    ),
                )
            })?;
        self.read_selected_variable(&base, elem, true)
    }

    fn read_variable_reference(
        &mut self,
        reference: &str,
    ) -> Result<crate::obj::Owned, crate::expr_error::ExprError> {
        self.read_variable_reference_bytes(reference.as_bytes())
    }

    fn read_variable_reference_bytes(
        &mut self,
        reference: &[u8],
    ) -> Result<crate::obj::Owned, crate::expr_error::ExprError> {
        let scanned = tcl_lexer::scan_var_ref(reference, 0, self.interp.lexer_config())
            .map_err(|message| {
                crate::expr_error::ExprError::from_bytes(message.as_bytes().to_vec())
            })?
            .ok_or_else(|| {
                crate::expr_error::ExprError::from_bytes(b"invalid variable reference".to_vec())
            })?;
        if let Some(index) = scanned.index {
            self.read_selected_variable(scanned.name, Some(index.to_vec()), true)
        } else {
            let (base, elem) = self.interp.variable_name_parts(scanned.name).map_err(|_| {
                crate::expr_error::ExprError::host_refusal(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "variable naming",
                    ),
                )
            })?;
            self.read_selected_variable(&base, elem, false)
        }
    }

    fn eval_command(
        &mut self,
        script: &str,
    ) -> Result<crate::obj::Owned, crate::expr_error::ExprError> {
        self.eval_command_bytes(script.as_bytes())
    }

    fn eval_command_bytes(
        &mut self,
        script: &[u8],
    ) -> Result<crate::obj::Owned, crate::expr_error::ExprError> {
        // A `[cmd]` operand that completes with any non-OK code (`error`, or a
        // `return`/`break`/`continue`) propagates that code out of the whole
        // expression. C does this implicitly: the bytecode for the substitution
        // returns the code, which unwinds the `expr`-bearing command. For an
        // error the interp result already holds the message + `::errorInfo`; for
        // the others it holds the substitution's result value.
        let code = self.interp.eval_str(script);
        if code != Code::Ok {
            return Err(self.propagated_error(code));
        }
        Ok(crate::obj::Owned::retain(self.interp.get_obj_result()))
    }

    fn eval_command_object(
        &mut self,
        original: &crate::obj::Owned,
    ) -> Result<crate::obj::Owned, crate::expr_error::ExprError> {
        let code = self.interp.eval_body_obj(original.as_ptr());
        if code != Code::Ok {
            return Err(self.propagated_error(code));
        }
        Ok(crate::obj::Owned::retain(self.interp.get_obj_result()))
    }

    fn subst_string(
        &mut self,
        inner: &str,
    ) -> Result<crate::obj::Owned, crate::expr_error::ExprError> {
        self.subst_string_bytes(inner.as_bytes())
    }

    fn subst_string_bytes(
        &mut self,
        inner: &[u8],
    ) -> Result<crate::obj::Owned, crate::expr_error::ExprError> {
        // A `"…"` operand substitutes like a double-quoted word ($var/[cmd]/\).
        match self.interp.do_expression_subst(inner, true) {
            Ok(v) => Ok(crate::obj::Owned::fresh(crate::obj::new_string_bytes(&v))),
            // A `"…"` operand whose `[cmd]` completed non-OK carries that code
            // (error, or `return`/`break`/`continue`) out of the expression.
            Err(code) => Err(self.propagated_error(code)),
        }
    }

    fn call_function(
        &mut self,
        name: &str,
        args: &[crate::obj::Owned],
    ) -> Result<crate::obj::Owned, crate::expr_error::ExprError> {
        tcl_registry::native_expression_program::expression_function_dispatch(
            self.interp.expression_evaluation_policy().as_ref(),
            self.interp.native_invocation_dialect(),
        )
        .ok_or_else(|| {
            crate::expr_error::ExprError::host_refusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "expression function dispatch policy",
                ),
            )
        })?;
        // Tcl 8.4 uses its closed C function table; later releases route through
        // the open command table so overridden/renamed `::tcl::mathfunc::NAME`
        // entries win. The registry owns that distinction. Args are passed as
        // live objects (object-preserving).
        let arg_ptrs: Vec<*mut TclObj> = args.iter().map(crate::obj::Owned::as_ptr).collect();
        let surface = tcl_registry::expr_surface::RuntimeExprSurface::for_profile(
            self.interp.dialect_profile(),
        );
        let code = match surface.math_function_call_target(name) {
            tcl_registry::expr_surface::MathFunctionCallTarget::FixedBuiltin(spec) => {
                self.interp.eval_fixed_math_call(spec, &arg_ptrs)
            }
            tcl_registry::expr_surface::MathFunctionCallTarget::CommandTable => {
                self.interp.eval_math_call(name.as_bytes(), &arg_ptrs)
            }
            tcl_registry::expr_surface::MathFunctionCallTarget::FixedTableMiss => {
                let mut message = b"unknown math function \"".to_vec();
                message.extend_from_slice(name.as_bytes());
                message.push(b'\"');
                self.interp.set_error(&message)
            }
        };
        if code == Code::Error {
            // A math-function error (e.g. `sqrt(-1)` domain error) is logged at
            // the `expr` command, not as an inner frame — so it is *not*
            // propagated; `expr` raises it as its own (`while executing`). Carry
            // the math function's `-errorcode` (TCL WRONGARGS / ARITH DOMAIN) so
            // `expr`'s re-raise preserves it.
            let code = self
                .interp
                .error_code_bytes_checked()
                .map_err(crate::expr_error::ExprError::from_execution_refusal)?;
            let msg = self
                .interp
                .native_object_string_bytes(self.interp.get_obj_result())
                .map_err(|error| {
                    crate::expr_error::ExprError::from_execution_refusal(
                        self.interp.refuse_completion_value_access(error),
                    )
                })?;
            if let Some(cause) = self.interp.native_execution_refusal() {
                return Err(crate::expr_error::ExprError::from_execution_refusal(cause));
            }
            return Err(crate::expr_error::ExprError::from_guest_parts(
                msg.to_vec(),
                code,
            ));
        }
        Ok(crate::obj::Owned::retain(self.interp.get_obj_result()))
    }
}

/// `expr arg ?arg ...?` — concatenate the args (space-separated), parse as a Tcl
/// expression, and evaluate it over the numeric tower
/// ([shared walk](tcl_syntax::expr::eval)).
#[cfg(have_tommath)]
fn expr_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let Some(arguments) = interp.native_invocation_dialect().expression_arguments() else {
        return interp.set_error(b"expression argument dialect is not selected");
    };
    if !arguments.accepts_len(argv.len().saturating_sub(1)) {
        return interp.wrong_args(arguments.synopsis().as_bytes());
    }
    let values: Vec<_> = match argv[1..]
        .iter()
        .map(|value| {
            tcl_syntax::value::ValueOps::native_string_bytes(interp, value)
                .map(|bytes| bytes.to_vec())
        })
        .collect::<Result<_, _>>()
    {
        Ok(values) => values,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let src = if values.len() == 1 {
        values[0].clone()
    } else {
        tcl_syntax::list::concat_bytes(values.iter().map(Vec::as_slice))
    };
    // The one-word form — `expr {$a < $b}`, the shape every compiled or hot
    // expression takes — is exactly the text of `argv[1]`, so its parse caches
    // on that object. A multi-word `expr` concatenates into fresh text that
    // belongs to no object, and re-parses.
    let cached_on = (argv.len() == 2).then(|| argv[1]);
    eval_expr_prepared_source(interp, cached_on, &src)
}

/// Evaluate an original expression token without command lookup or argv reconstruction.
#[cfg(have_tommath)]
pub(crate) fn eval_expr_original(interp: &mut Interp, original: *mut TclObj) -> Code {
    let bytes = match tcl_syntax::value::ValueOps::native_string_bytes(interp, &original) {
        Ok(bytes) => bytes,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    eval_expr_prepared_source(interp, Some(original), &bytes)
}

#[cfg(have_tommath)]
fn eval_expr_prepared_source(
    interp: &mut Interp,
    cached_on: Option<*mut TclObj>,
    src: &[u8],
) -> Code {
    let node = match parse_runtime_expr_cached(interp, cached_on, src) {
        Ok(node) => node,
        Err(e) => {
            return raise_expr_parse_error(interp, src, e);
        }
    };
    let mut ctx = InterpExprCtx {
        interp: &mut *interp,
        propagated: false,
        propagated_code: Code::Error,
    };
    let _lease = cached_on.and_then(crate::expr::retain_expression_primary);
    let result = match cached_on.and_then(crate::expr::native_jim_expression_objects) {
        Some(objects) => crate::expr::eval_jim_expr(&node, &mut ctx, objects, false),
        None => crate::expr::eval_expr(&node, &mut ctx),
    };
    drop(_lease);
    let propagated = ctx.propagated;
    let propagated_code = ctx.propagated_code;
    match result {
        Ok(r) => {
            // `set_result` takes its own `+1`; `r` drops its reference after.
            interp.set_result(r.as_ptr());
            Code::Ok
        }
        // A propagated sub-eval code already set the interp result at the inner
        // command — preserve it (an error keeps its `::errorInfo`; a
        // `return`/`break`/`continue` keeps the substitution's value).
        Err(_) if propagated => propagated_code,
        Err(e) => raise_expr_error(interp, &node, e),
    }
}

/// Evaluate `src` as a Tcl expression, returning the result object (owned — the
/// caller holds a `+1` and must release it) or the error `Code` (interp result =
/// message). Used where an argument "may be a valid expression" — e.g. `lseq`'s
/// numeric arguments (`SequenceIdentifyArgument`).
#[cfg(have_tommath)]
pub(crate) fn eval_expr_obj(interp: &mut Interp, src: &[u8]) -> Result<*mut TclObj, Code> {
    eval_expr_obj_at_original(interp, None, src)
}

/// Preserve a caller's actual expression object and retained Jim terms while
/// performing the same single public expression result operation.
#[cfg(have_tommath)]
pub(crate) fn eval_expr_obj_original(
    interp: &mut Interp,
    original: *mut TclObj,
    src: &[u8],
) -> Result<*mut TclObj, Code> {
    eval_expr_obj_at_original(interp, Some(original), src)
}

#[cfg(have_tommath)]
fn eval_expr_obj_at_original(
    interp: &mut Interp,
    original: Option<*mut TclObj>,
    src: &[u8],
) -> Result<*mut TclObj, Code> {
    let currency = crate::interp::native_operation_currency::NativeOperationCurrency::issue(interp)
        .map_err(|error| {
            interp.report_cmd_error(tcl_cmd_core::CmdError::from_execution_refusal(error))
        })?;
    let node = parse_runtime_expr_cached(interp, original, src)
        .map_err(|e| raise_expr_parse_error(interp, src, e))?;
    let mut ctx = InterpExprCtx {
        interp: &mut *interp,
        propagated: false,
        propagated_code: Code::Error,
    };
    let lease = original.and_then(crate::expr::retain_expression_primary);
    let result = match original.and_then(crate::expr::native_jim_expression_objects) {
        Some(objects) => crate::expr::eval_jim_expr(&node, &mut ctx, objects, false),
        None => crate::expr::eval_compiled_expression_node(&node, &mut ctx),
    };
    drop(lease);
    let propagated = ctx.propagated;
    let propagated_code = ctx.propagated_code;
    // This check precedes every success and propagated Guest branch. Re-entry
    // cannot replace the original world before the public result producer.
    currency.ensure_current().map_err(|error| {
        interp.report_cmd_error(tcl_cmd_core::CmdError::from_execution_refusal(error))
    })?;
    let result = match result {
        Ok(value) => crate::typed_value::normalize_boolean_result_for_interp(
            interp,
            value.as_ptr(),
            tcl_native_model::native_boolean_truth::NativeBooleanExpressionResultProduction::PublicExpressionApi,
        ).map_err(|e| interp.report_cmd_error(e)),
        Err(_) if propagated => Err(propagated_code),
        Err(e) => Err(raise_expr_error(interp, &node, e)),
    };
    currency.ensure_current().map_err(|error| {
        interp.report_cmd_error(tcl_cmd_core::CmdError::from_execution_refusal(error))
    })?;
    result.map(crate::obj::Owned::into_raw)
}

/// Whether `error` is a guest error (not a host refusal) an expression frame
/// may annotate.
#[cfg(have_tommath)]
fn is_guest_expr_error(error: &crate::expr_error::ExprError) -> bool {
    error.native_execution_refusal.is_none() && error.native_access_refusal.is_none()
}

/// The outermost operations of `node` C folds at compile time (an operator
/// over constant operands), in order (`ExprNode::folded_operations` over the
/// byte-leaf tree the runtime parses).
#[cfg(have_tommath)]
fn folded_operations<Text: tcl_syntax::expr::ExprText>(
    node: &tcl_syntax::expr::ExprNode<Text>,
) -> Vec<&tcl_syntax::expr::ExprNode<Text>> {
    use tcl_syntax::expr::{BinOp, ExprNode, UnaryOp};
    fn foldable<Text>(node: &ExprNode<Text>) -> bool {
        match node {
            ExprNode::Binary { op, .. } => matches!(
                op,
                BinOp::Add
                    | BinOp::Sub
                    | BinOp::Mul
                    | BinOp::Div
                    | BinOp::Mod
                    | BinOp::Pow
                    | BinOp::LShift
                    | BinOp::RShift
                    | BinOp::BitAnd
                    | BinOp::BitOr
                    | BinOp::BitXor
                    | BinOp::And
                    | BinOp::Or
                    | BinOp::Eq
                    | BinOp::Ne
                    | BinOp::Lt
                    | BinOp::Le
                    | BinOp::Gt
                    | BinOp::Ge
            ),
            ExprNode::Unary { op, .. } => matches!(
                op,
                UnaryOp::Neg | UnaryOp::Pos | UnaryOp::BitNot | UnaryOp::Not
            ),
            _ => false,
        }
    }
    fn constant<Text: tcl_syntax::expr::ExprText>(node: &ExprNode<Text>) -> bool {
        match node {
            ExprNode::Literal { .. } => true,
            ExprNode::String { text, .. } => {
                let bytes = text.bytes();
                bytes.first() == Some(&b'{') || !(bytes.contains(&b'$') || bytes.contains(&b'['))
            }
            ExprNode::Binary { left, right, .. } => constant(left) && constant(right),
            ExprNode::Unary { operand, .. } => constant(operand),
            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
            } => constant(condition) && constant(true_branch) && constant(false_branch),
            _ => false,
        }
    }
    if foldable(node) && constant(node) {
        return vec![node];
    }
    match node {
        ExprNode::Binary { left, right, .. } => {
            let mut folded = folded_operations(left);
            folded.extend(folded_operations(right));
            folded
        }
        ExprNode::Unary { operand, .. } => folded_operations(operand),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => [condition, true_branch, false_branch]
            .into_iter()
            .flat_map(|node| folded_operations(node))
            .collect(),
        ExprNode::Call { args, .. } => args.iter().flat_map(|arg| folded_operations(arg)).collect(),
        _ => Vec::new(),
    }
}

/// Raise `error`, which evaluating `node` failed with: where it is the error C
/// raises folding a constant operation of the expression as it compiles it
/// (from 8.5), its message already starts the trace, as C's does.
#[cfg(have_tommath)]
fn raise_expr_error(
    interp: &mut Interp,
    node: &tcl_syntax::expr::NativeExprNode,
    error: crate::expr_error::ExprError,
) -> Code {
    let folded = is_guest_expr_error(&error)
        && !interp.uses_jim_error_stack()
        && interp.runtime_version().raises_folded_expr_errors()
        && folded_operations(node).into_iter().any(|operation| {
            let mut ctx = InterpExprCtx {
                interp: &mut *interp,
                propagated: false,
                propagated_code: Code::Error,
            };
            matches!(crate::expr::eval_expr(operation, &mut ctx), Err(e) if e.msg == error.msg)
        });
    let code = interp.report_expr_error(error);
    if folded && !interp.host_refusal_pending() {
        interp.start_error_info_at_message();
    }
    code
}

/// Raise the parse error `error` of the expression `source`: from 8.5 with
/// the `(parsing expression "…")` frame `ParseExpr` adds, so the command
/// holding the expression logs itself as invoked from within.
#[cfg(have_tommath)]
fn raise_expr_parse_error(
    interp: &mut Interp,
    source: &[u8],
    error: crate::expr_error::ExprError,
) -> Code {
    let guest = is_guest_expr_error(&error);
    let code = interp.report_expr_error(error);
    if guest
        && !interp.host_refusal_pending()
        && !interp.uses_jim_error_stack()
        && !interp.runtime_version().parses_expressions_as_8_4()
    {
        let frame = tcl_syntax::expr::ExprSyntaxError::error_info_frame_bytes(source);
        let text = frame.strip_prefix(b"\n    (".as_slice()).unwrap_or(&frame);
        let text = text.strip_suffix(b")".as_slice()).unwrap_or(text);
        interp.append_frame_noline(text);
    }
    code
}

/// Evaluate the condition object `cond` as a Tcl expression and coerce the result
/// to a boolean — the condition evaluator `if`/`while`/`for` share. `Err(code)`
/// carries the completion code (with the interp result already set to the error
/// message). A located-literal condition shifts the shared frame's `line_base` to
/// the condition word so a `[cmd]` substitution inside reports its file-absolute
/// line (TIP 280); the base is restored afterward.
#[cfg(have_tommath)]
pub(crate) fn eval_bool_expr(interp: &mut Interp, cond: *mut TclObj) -> Result<bool, Code> {
    let currency = crate::interp::native_operation_currency::NativeOperationCurrency::issue(interp)
        .map_err(|error| {
            interp.report_cmd_error(tcl_cmd_core::CmdError::from_execution_refusal(error))
        })?;
    let src = tcl_syntax::value::ValueOps::native_string_bytes(interp, &cond)
        .map_err(|error| interp.report_cmd_error(error.into()))?;
    currency.ensure_current().map_err(|error| {
        interp.report_cmd_error(tcl_cmd_core::CmdError::from_execution_refusal(error))
    })?;
    let saved = match interp.arg_location(cond) {
        Some((_, line)) => interp.push_cond_line_base(line),
        None => None,
    };
    let node = match parse_runtime_expr_cached(interp, Some(cond), &src) {
        Ok(node) => node,
        Err(e) => {
            if let Some(old) = saved {
                interp.restore_line_base(old);
            }
            return Err(raise_expr_parse_error(interp, &src, e));
        }
    };
    let mut ctx = InterpExprCtx {
        interp: &mut *interp,
        propagated: false,
        propagated_code: Code::Error,
    };
    let _lease = crate::expr::retain_expression_primary(cond);
    let result = match crate::expr::native_jim_expression_objects(cond) {
        Some(objects) => crate::expr::eval_jim_expr(&node, &mut ctx, objects, false),
        None => crate::expr::eval_compiled_expression_node(&node, &mut ctx),
    };
    drop(_lease);
    let propagated = ctx.propagated;
    let propagated_code = ctx.propagated_code;
    if let Some(old) = saved {
        interp.restore_line_base(old);
    }
    // Keep the entry world across effectful raw evaluation. A changed-and-
    // restored engine cannot donate a fresh result-producer receipt.
    currency.ensure_current().map_err(|error| {
        interp.report_cmd_error(tcl_cmd_core::CmdError::from_execution_refusal(error))
    })?;
    let result = match result {
        // The boolean-context refusal keeps its own `-errorcode` (tclsh:
        // `set x o; if {$x} {}` is `TCL VALUE NUMBER`, a NaN condition is
        // `TCL VALUE DOUBLE NAN`), exactly as the eval failure below does.
        Ok(r) => crate::typed_value::expression_boolean_for_interp(interp, r.as_ptr(),
            tcl_native_model::native_boolean_truth::NativeBooleanExpressionResultProduction::PublicExpressionApi)
            .map_err(|e| interp.report_cmd_error(e)),
        // A propagated sub-eval code unwinds the condition: an error keeps its
        // trace (no condition `expr` frame); a `return`/`break`/`continue` from a
        // `[cmd]` substitution carries that code out of the loop/`if`.
        Err(_) if propagated => Err(propagated_code),
        Err(e) => Err(raise_expr_error(interp, &node, e)),
    };
    currency.ensure_current().map_err(|error| {
        interp.report_cmd_error(tcl_cmd_core::CmdError::from_execution_refusal(error))
    })?;
    result
}

/// Parse an expression against the registry-owned surface for this
/// interpreter's emulated release. The parser deliberately runs over the
/// union grammar: a later-release operator is then rejected by the registry
/// with C Tcl's distinct `BAREWORD` diagnostic instead of becoming the parser's
/// generic raw fallback. Function calls remain open command-table lookups, so
/// user-defined `tcl::mathfunc` commands continue to work on every release.
#[cfg(have_tommath)]
fn parse_runtime_expr(
    interp: &Interp,
    source: &[u8],
) -> Result<tcl_syntax::expr::NativeExprNode, crate::expr_error::ExprError> {
    #[cfg(test)]
    crate::expr::note_expr_parse();
    let context = interp.expression_parse_context();
    if tcl_registry::runtime_expr_validation::requires_fixed_function_preparation(&context) {
        return prepare_runtime_fixed_expression(interp, source);
    }
    let node = match tcl_syntax::expr::parser::parse_expr_bytes_checked_with_context(
        source, &context,
    ) {
        tcl_syntax::expr::parser::CheckedExprParse::Parsed(node) => node,
        tcl_syntax::expr::parser::CheckedExprParse::ProvedSyntaxFailure(failure) => {
            let diagnostic = failure.native_diagnostic_bytes_with_context(source, &context).ok_or_else(||
                crate::expr_error::ExprError::host_refusal(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("native expression syntax presentation")))?;
            return Err(crate::expr_error::ExprError::syntax(
                diagnostic.message,
                diagnostic.error_code,
                context.native_syntax,
            ));
        }
        tcl_syntax::expr::parser::CheckedExprParse::Unsupported(_) => {
            return Err(crate::expr_error::ExprError::host_refusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "native expression source grammar",
                ),
            ));
        }
    };
    tcl_registry::expr_surface::RuntimeExprSurface::for_profile(interp.dialect_profile())
        .validate(&node)
        .map_err(|_| {
            crate::expr_error::ExprError::host_refusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "native expression operator surface",
                ),
            )
        })?;
    Ok(node)
}

#[cfg(have_tommath)]
fn expression_value_error(error: tcl_syntax::value::ValueError) -> crate::expr_error::ExprError {
    crate::expr_error::ExprError::host_refusal(
        error
            .native_access_refusal()
            .expect("original expression storage refusal"),
    )
}

/// Actual Jim GetWideExpr on the same original object; safe mode never performs
/// variable lookup or evaluates a reached command/escaped substitution.
pub(crate) fn native_jim_wide_expression(
    interp: &mut Interp,
    original: *mut TclObj,
) -> Result<i64, tcl_cmd_core::CmdError> {
    let arguments = OriginalArguments::enter(interp, &[original])?;
    let reached = (|| {
        #[cfg(have_tommath)]
        use tcl_syntax::scalar_getter::NativeScalarGetterValue;
        use tcl_syntax::scalar_getter::{NativeScalarCache, NativeScalarGetterKind};
        let dialect = interp.native_invocation_dialect();
        let context = interp.native_jim_object_context()?;
        let bound = crate::native_source::bind_context(original, &context);
        arguments.check()?;
        bound?;
        let kind = obj::obj_type_ptr(original);
        if kind.is_null() || kind == &crate::native_source::JIM_SOURCE_TYPE {
            let reached = crate::typed_value::native_scalar_probe(
                original,
                dialect,
                NativeScalarGetterKind::Wide,
            );
            arguments.check()?;
            let _ = reached?;
        }
        let cached = obj::native_scalar_cache(original);
        arguments.check()?;
        if let Some(NativeScalarCache::Number(tcl_syntax::number::Number::Int(value))) = cached? {
            return Ok(value);
        }
        #[cfg(not(have_tommath))]
        return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "native Jim integer expression backend",
        )
        .into());
        #[cfg(have_tommath)]
        {
            let source = arguments.bytes(0)?;
            let parsed = parse_runtime_expr_cached(interp, Some(original), &source);
            arguments.check()?;
            let evaluated = match parsed {
                Ok(node) => {
                    let _lease = crate::expr::retain_expression_primary(original);
                    let objects = crate::expr::native_jim_expression_objects(original).ok_or(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "original Jim integer expression terms",
                        ),
                    )?;
                    let mut ctx = InterpExprCtx {
                        interp,
                        propagated: false,
                        propagated_code: Code::Error,
                    };
                    crate::expr::eval_jim_expr(&node, &mut ctx, objects, true)
                }
                Err(error) => Err(error),
            };
            arguments.check()?;
            match evaluated {
                Ok(result) => {
                    let bound = crate::native_source::bind_context(result.as_ptr(), &context);
                    arguments.check()?;
                    bound?;
                    interp.set_result(result.as_ptr());
                    arguments.check()?;
                    let reached = crate::typed_value::native_scalar_probe(
                        result.as_ptr(),
                        dialect,
                        NativeScalarGetterKind::Wide,
                    );
                    arguments.check()?;
                    if let Ok(NativeScalarGetterValue::Wide(value)) = reached? {
                        return Ok(value);
                    }
                }
                Err(error) if error.native_execution_refusal.is_some() => {
                    return Err(tcl_cmd_core::CmdError::from_execution_refusal(
                        error
                            .native_execution_refusal
                            .expect("matched original host refusal"),
                    ));
                }
                Err(error) if error.native_access_refusal.is_some() => {
                    return Err(error
                        .native_access_refusal
                        .expect("matched exact native refusal")
                        .into());
                }
                Err(_) => {}
            }
            let message = tcl_syntax::expr::native_objects::jim_integer_expression_message(&source);
            interp.set_result_bytes(&message);
            Err(tcl_cmd_core::CmdError::from_byte_details(
                tcl_cmd_core::CmdErrorDetails {
                    string_result: None,
                    message,
                    error_code: tcl_cmd_core::CmdErrorCodeUpdate::Unchanged,
                    error_info: None,
                    error_line: None,
                    primitive_getter: None,
                },
            ))
        }
    })();
    // Retire evaluated temporaries under the same original operation before settlement.
    arguments.check()?;
    reached
}

/// Jim prepares all functions before substitution and executes its native term tree.
#[cfg(have_tommath)]
pub(crate) fn prepare_runtime_fixed_expression(
    interp: &Interp,
    source: impl AsRef<[u8]>,
) -> Result<tcl_syntax::expr::NativeExprNode, crate::expr_error::ExprError> {
    prepare_runtime_fixed_expression_on(interp, source.as_ref(), None, None)
}

#[cfg(have_tommath)]
fn prepare_runtime_fixed_expression_on(
    interp: &Interp,
    source: &[u8],
    cached_on: Option<*mut TclObj>,
    source_info: Option<&crate::native_source::NativeJimSourceInfo>,
) -> Result<tcl_syntax::expr::NativeExprNode, crate::expr_error::ExprError> {
    use tcl_registry::native_compilation::NativeMathFunctionResolution as Resolution;
    use tcl_registry::runtime_expr_validation::RuntimeExpressionPreparation as Preparation;
    let context = interp.expression_parse_context();
    let table = interp.native_math_function_table();
    let prepared = tcl_registry::runtime_expr_validation::prepare_fixed_function_expression_bytes_with_preparation(
        source,
        &context,
        |name| {
            let Some(table) = &table else {
                return Resolution::Unknown;
            };
            use tcl_runtime_api::native_compilation::NativeMathFunctionResolution as Lookup;
            match table.lookup(name) {
                Lookup::Present(binding) => binding
                    .arity
                    .map_or(Resolution::Unknown, |arity| Resolution::Known { arity }),
                Lookup::Absent => Resolution::Absent,
                Lookup::Unknown => Resolution::Unknown,
            }
        },
    );
    if let Some(original) = cached_on {
        if prepared.jim.as_ref().is_some_and(|preparation| {
            preparation.action
                == tcl_syntax::expr::native_objects::JimExpressionCacheAction::Rejected
        }) {
            crate::expr::prepare_expr_cache(
                original,
                interp.native_invocation_dialect(),
                interp.logical_expression_parse_policy(),
            );
        }
    }
    match prepared.tree {
        Preparation::Parsed(node) => {
            if let (Some(original), Some(info), Some(preparation)) =
                (cached_on, source_info, prepared.jim.as_ref())
            {
                if crate::expr::cached_expr(
                    original,
                    interp.native_invocation_dialect(),
                    interp.logical_expression_parse_policy(),
                )
                .is_none()
                {
                    crate::expr::install_jim_expression(
                        original,
                        crate::expr::JimExpressionInstall {
                            dialect: interp.native_invocation_dialect(),
                            parser_policy: interp.logical_expression_parse_policy(),
                            node: &node,
                            source,
                            preparation,
                            info,
                        },
                    )
                    .map_err(expression_value_error)?;
                }
            }
            Ok(node)
        }
        Preparation::Rejected(diagnostic) => Err(crate::expr_error::ExprError::syntax(
            diagnostic.message,
            diagnostic.error_code,
            context.native_syntax,
        )),
        Preparation::Unknown => Err(crate::expr_error::ExprError::host_refusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "native expression function preparation",
            ),
        )),
    }
}

/// [`parse_runtime_expr`] with the parsed, validated AST cached on the object
/// the expression text came from (`TCL_EXPR_TYPE`, [`crate::expr::cache_expr`]).
///
/// Without this the condition of every `if`/`while`/`for` iteration is re-lexed,
/// re-parsed, and re-validated from its text on every evaluation. `cached_on`
/// is `None` where the text belongs to no single object (a multi-word
/// `expr`), which simply re-parses.
///
/// Successful parses retain their validated tree. Jim preparation separately
/// records rejected Expression storage when its native action retires the prior
/// primary; empty input and missing wrappers preserve that prior representation.
#[cfg(have_tommath)]
fn parse_runtime_expr_cached(
    interp: &Interp,
    cached_on: Option<*mut TclObj>,
    source: &[u8],
) -> Result<std::rc::Rc<tcl_syntax::expr::NativeExprNode>, crate::expr_error::ExprError> {
    let dialect = interp.native_invocation_dialect();
    let source_info = if dialect.native_string_protocol()
        == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
    {
        cached_on
            .map(|original| {
                let context = interp
                    .native_jim_object_context()
                    .map_err(expression_value_error)?;
                crate::native_source::bind_context(original, &context)
                    .map_err(expression_value_error)?;
                crate::native_source::pin_source_info(original, &context)
                    .map_err(expression_value_error)
            })
            .transpose()?
    } else {
        None
    };
    if dialect.native_string_protocol()
        == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        && cached_on.is_some_and(|original| {
            crate::expr::expression_rejected(
                original,
                dialect,
                interp.logical_expression_parse_policy(),
            )
        })
    {
        return Err(crate::expr_error::ExprError::retained_result());
    }
    let materialized;
    let source = if let Some(original) = cached_on {
        materialized = if crate::obj::has_string_rep(original) {
            crate::obj::bytes_of(original)
        } else {
            let protocol = match interp.logical_expression_parse_policy() {
                Some((_, host)) => tcl_native_model::InvocationDialect::of_profile(host.profile())
                    .native_string_protocol(),
                None => dialect.native_string_materialization(Some(
                    tcl_native_model::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation,
                )).map(|recipe| recipe.protocol()),
            }.ok_or_else(|| crate::expr_error::ExprError::host_refusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "native expression original source string",
                ),
            ))?;
            crate::dict::native_object_bytes(original, protocol).map_err(|error| {
                crate::expr_error::ExprError::host_refusal(
                    error
                        .native_access_refusal()
                        .expect("checked native string access has a host refusal"),
                )
            })?
        };
        materialized.as_slice()
    } else {
        source
    };
    if !tcl_registry::runtime_expr_validation::requires_fixed_function_preparation(
        &tcl_syntax::expr::parser::ExprParseContext::for_profile(interp.dialect_profile()),
    ) {
        if let Some(obj) = cached_on {
            if let Some(node) =
                crate::expr::cached_expr(obj, dialect, interp.logical_expression_parse_policy())
            {
                return Ok(node);
            }
        }
    }
    let preparation = interp
        .expression_parse_context()
        .native_syntax
        .source_cache_preparation()
        .ok_or_else(|| {
            crate::expr_error::ExprError::host_refusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "native expression source preparation",
                ),
            )
        })?;
    if preparation == tcl_syntax::expr::parser::ExpressionSourceCachePreparation::BeforeParsing {
        if let Some(object) = cached_on {
            crate::expr::prepare_expr_cache(
                object,
                dialect,
                interp.logical_expression_parse_policy(),
            );
        }
    }
    let node = std::rc::Rc::new(
        if preparation == tcl_syntax::expr::parser::ExpressionSourceCachePreparation::JimOutcome {
            #[cfg(test)]
            crate::expr::note_expr_parse();
            prepare_runtime_fixed_expression_on(interp, source, cached_on, source_info.as_ref())?
        } else {
            parse_runtime_expr(interp, source)?
        },
    );
    if let Some(obj) = cached_on {
        crate::expr::cache_expr(
            obj,
            dialect,
            interp.logical_expression_parse_policy(),
            &node,
        );
    }
    Ok(node)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod native_subst_operand_tests;

#[cfg(test)]
mod native_variable_operand_tests;
