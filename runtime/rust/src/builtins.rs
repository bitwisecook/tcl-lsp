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

use crate::frame::VarError;
use crate::interp::{obj_bytes, Code, Interp};
// The transient `1` of a tower `incr` is the only fresh object left to drop
// by hand; every other path now stores through `store_var_result`.
use crate::interp::drop_fresh;
use crate::obj::{self, TclObj};

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
    interp.register_builtin(b"set", set);
    interp.register_builtin(b"incr", incr);
    interp.register_builtin(b"const", const_cmd);
    interp.register_builtin(b"return", ret);
    interp.register_builtin(b"unset", unset);
    interp.register_builtin(b"exit", exit_cmd);
    interp.register_builtin(b"subst", subst_cmd);
    crate::cmd_scan::install(interp);
    crate::cmd_format::install(interp);
    if protocol.is_none_or(|protocol| protocol.registers_core_binary()) {
        crate::cmd_binary::install(interp);
    }
    crate::cmd_clock::install(interp);
    // `expr` needs the numeric tower (libtommath); registered only when linked.
    #[cfg(have_tommath)]
    interp.register_builtin(b"expr", expr_cmd);
    // `::tcl::mathfunc::*` are real commands `expr`'s function path resolves
    // through (overridable); they need the tower too.
    #[cfg(have_tommath)]
    crate::cmd_mathfunc::install(interp);
    // `::tcl::mathop::*` — the operators as real commands (tower-gated).
    #[cfg(have_tommath)]
    crate::cmd_mathop::install(interp);
    crate::cmd_list::install(interp);
    #[cfg(have_tommath)]
    crate::cmd_lseq::install(interp);
    crate::cmd_dict::install(interp);
    crate::cmd_alias::install(interp);
    crate::cmd_namespace::install(interp);
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
        return interp.var_trace_error(name, b"write", &reason);
    }
    let verb = match e {
        VarError::NameProtocolUnavailable => unreachable!("host refusal handled above"),
        VarError::IsArray => &b"\": variable is array"[..],
        VarError::IsScalar => &b"\": variable isn't array"[..],
        VarError::DeletedNamespace => &b"\": upvar refers to variable in deleted namespace"[..],
        VarError::DeletedArray => &b"\": upvar refers to element in deleted array"[..],
        VarError::NoSuchNamespace => &b"\": parent namespace doesn't exist"[..],
        VarError::IsConstant => &b"\": variable is a constant"[..],
        VarError::TraceError => unreachable!("handled above"),
    };
    let mut msg = b"can't set \"".to_vec();
    msg.extend_from_slice(name);
    msg.extend_from_slice(verb);
    interp.set_error(&msg)
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
    let name = obj_bytes(argv[1]);
    let (base, elem) = match interp.variable_name_parts(&name) {
        Ok(parts) => parts,
        Err(error) => return var_error(interp, &name, error),
    };
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
    if let Some(c) = incr_constant_error(interp, &base, &elem, &name) {
        return c;
    }
    if let Some(amount) = legacy_amount {
        if interp.native_invocation_dialect().native_string_protocol()
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
                let message = error
                    .safe_expression_message_bytes(&obj_bytes(amount))
                    .expect("safe-expression grammar error");
                return interp.set_error(&message);
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
    let cur = interp.read_for_update(&base, elem.as_deref());
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
                    return interp.set_error(&m);
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
fn ret(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    crate::return_options::command(interp, &argv[1..])
}

// unset

/// `unset varName ...` — remove variables (scalars or array elements).
fn unset(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let Some(protocol) = interp.native_invocation_dialect().unset_option_protocol() else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("unset option grammar")
                .into(),
        );
    };
    let options = match protocol.parse(argv.len() - 1, |index| {
        interp
            .native_string_bytes(&argv[index + 1])
            .map_err(|error| interp.report_cmd_error(error.into()))
    }) {
        Ok(options) => options,
        Err(code) => return code,
    };
    let i = options.names_from + 1;
    let nocomplain = !options.complain;
    for &a in &argv[i..] {
        if interp.native_c_variable_name_protocol().is_some() {
            if let Err(code) = interp.unset_original_c_variable(a, !nocomplain) {
                return code;
            }
            continue;
        }
        if interp
            .native_invocation_dialect()
            .native_jim_lookup_protocol()
            .is_some()
        {
            match interp.unset_original_jim_variable(a) {
                Ok(true) => continue,
                Ok(false) if nocomplain => continue,
                Ok(false) => {
                    let name = obj_bytes(a);
                    let mut message = b"can't unset \"".to_vec();
                    message.extend_from_slice(&name);
                    message.extend_from_slice(b"\": no such variable");
                    return interp.set_error(&message);
                }
                Err(code) => return code,
            }
        }
        let name = obj_bytes(a);
        let (base, elem) = match interp.variable_name_parts(&name) {
            Ok(parts) => parts,
            Err(error) => return var_error(interp, &name, error),
        };
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
    const USAGE: &[u8] = b"subst ?-nobackslashes? ?-nocommands? ?-novariables? string";
    if argv.len() < 2 {
        return interp.wrong_args(USAGE);
    }
    // Every argument before the last is an option (C's `TclSubstOptions` over
    // `objv[1 .. objc-1]`), matched with Tcl's unambiguous-prefix rule.
    let mut flags = crate::subst::SubstFlags::default();
    for &opt in &argv[1..argv.len() - 1] {
        match SUBST_OPTIONS.index_of(&obj_bytes(opt)) {
            Ok(0) => flags.backslashes = false,
            Ok(1) => flags.cmds = false,
            Ok(_) => flags.vars = false,
            Err(m) => return interp.set_error(&m),
        }
    }
    let last = argv[argv.len() - 1];
    if interp
        .native_invocation_dialect()
        .native_string_protocol()
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
    let src = obj_bytes(last);
    // TIP 280: a `[...]` inside the substituted string reports the line it sits
    // on, derived from the argument word's recorded source location.
    let loc = interp.arg_location(last);
    match interp.do_subst_located(&src, flags, loc) {
        Ok(bytes) => {
            interp.set_result_bytes(&bytes);
            Code::Ok
        }
        Err(code) => code, // the failing sub already set the result
    }
}

/// `subst`'s option words. C's `TclSubstOptions` (`tclCmdMZ.c:3341`) resolves
/// them with `Tcl_GetIndexFromObj` at flags `0`, so abbreviations match and the
/// *empty* word — which prefixes all three entries — is `ambiguous`, not `bad`.
/// Shared with the bytecode VM through the one `tcl-cmd-core::prefix` matcher.
const SUBST_OPTIONS: tcl_cmd_core::prefix::OptionTable<'static, &[u8]> =
    tcl_cmd_core::prefix::OptionTable::abbreviating(
        "option",
        &[b"-nobackslashes", b"-nocommands", b"-novariables"],
    );

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

#[cfg(have_tommath)]
impl InterpExprCtx<'_> {
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
                        self.propagated = true;
                        self.propagated_code = code;
                        return Err(crate::expr_error::ExprError::from_bytes(obj_bytes(
                            self.interp.get_obj_result(),
                        )));
                    }
                }
            }
            other => other,
        };
        if let Some(code) = self.interp.fire_read_trace(base, elem.as_deref()) {
            self.propagated = true;
            self.propagated_code = code;
            return Err(crate::expr_error::ExprError::from_bytes(obj_bytes(
                self.interp.get_obj_result(),
            )));
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
    fn numeric_host(&self) -> Option<std::rc::Rc<dyn tcl_platform::Host>> {
        Some(self.interp.host())
    }
    fn invocation_dialect(&self) -> tcl_registry::InvocationDialect {
        self.interp.native_invocation_dialect()
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
            self.propagated = true;
            self.propagated_code = code;
            return Err(crate::expr_error::ExprError::from_bytes(obj_bytes(
                self.interp.get_obj_result(),
            )));
        }
        Ok(crate::obj::Owned::retain(self.interp.get_obj_result()))
    }

    fn eval_command_object(
        &mut self,
        original: &crate::obj::Owned,
    ) -> Result<crate::obj::Owned, crate::expr_error::ExprError> {
        let code = self.interp.eval_body_obj(original.as_ptr());
        if code != Code::Ok {
            self.propagated = true;
            self.propagated_code = code;
            return Err(crate::expr_error::ExprError::from_bytes(obj_bytes(
                self.interp.get_obj_result(),
            )));
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
            Err(code) => {
                self.propagated = true;
                self.propagated_code = code;
                Err(crate::expr_error::ExprError::from_bytes(obj_bytes(
                    self.interp.get_obj_result(),
                )))
            }
        }
    }

    fn call_function(
        &mut self,
        name: &str,
        args: &[crate::obj::Owned],
    ) -> Result<crate::obj::Owned, crate::expr_error::ExprError> {
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
            let msg = obj_bytes(self.interp.get_obj_result());
            let code = self.interp.error_code();
            return Err(crate::expr_error::ExprError::from_parts(msg, code));
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
            return interp.report_expr_error(e);
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
        Err(e) => interp.report_expr_error(e),
    }
}

/// Evaluate `src` as a Tcl expression, returning the result object (owned — the
/// caller holds a `+1` and must release it) or the error `Code` (interp result =
/// message). Used where an argument "may be a valid expression" — e.g. `lseq`'s
/// numeric arguments (`SequenceIdentifyArgument`).
#[cfg(have_tommath)]
pub(crate) fn eval_expr_obj(interp: &mut Interp, src: &[u8]) -> Result<*mut TclObj, Code> {
    let node = parse_runtime_expr(interp, src).map_err(|e| interp.report_expr_error(e))?;
    let mut ctx = InterpExprCtx {
        interp: &mut *interp,
        propagated: false,
        propagated_code: Code::Error,
    };
    let result = crate::expr::eval_expr(&node, &mut ctx);
    let propagated = ctx.propagated;
    let propagated_code = ctx.propagated_code;
    match result {
        Ok(r) => Ok(r.into_raw()), // transfer the +1 to the caller
        Err(_) if propagated => Err(propagated_code),
        Err(e) => Err(interp.report_expr_error(e)),
    }
}

/// Evaluate the condition object `cond` as a Tcl expression and coerce the result
/// to a boolean — the condition evaluator `if`/`while`/`for` share. `Err(code)`
/// carries the completion code (with the interp result already set to the error
/// message). A located-literal condition shifts the shared frame's `line_base` to
/// the condition word so a `[cmd]` substitution inside reports its file-absolute
/// line (TIP 280); the base is restored afterward.
#[cfg(have_tommath)]
pub(crate) fn eval_bool_expr(interp: &mut Interp, cond: *mut TclObj) -> Result<bool, Code> {
    let src = tcl_syntax::value::ValueOps::native_string_bytes(interp, &cond)
        .map_err(|error| interp.report_cmd_error(error.into()))?;
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
            return Err(interp.report_expr_error(e));
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
        None => crate::expr::eval_expr(&node, &mut ctx),
    };
    drop(_lease);
    let propagated = ctx.propagated;
    let propagated_code = ctx.propagated_code;
    if let Some(old) = saved {
        interp.restore_line_base(old);
    }
    match result {
        // The boolean-context refusal keeps its own `-errorcode` (tclsh:
        // `set x o; if {$x} {}` is `TCL VALUE NUMBER`, a NaN condition is
        // `TCL VALUE DOUBLE NAN`), exactly as the eval failure below does.
        Ok(r) => crate::expr::to_bool_in(r.as_ptr(), interp.native_invocation_dialect())
            .map_err(|e| interp.report_expr_error(e)),
        // A propagated sub-eval code unwinds the condition: an error keeps its
        // trace (no condition `expr` frame); a `return`/`break`/`continue` from a
        // `[cmd]` substitution carries that code out of the loop/`if`.
        Err(_) if propagated => Err(propagated_code),
        Err(e) => Err(interp.report_expr_error(e)),
    }
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
    use tcl_syntax::scalar_getter::{
        NativeScalarCache, NativeScalarGetterKind, NativeScalarGetterValue,
    };
    let dialect = interp.native_invocation_dialect();
    let context = interp.native_jim_object_context()?;
    crate::native_source::bind_context(original, &context)?;
    let kind = obj::obj_type_ptr(original);
    if kind.is_null() || kind == &crate::native_source::JIM_SOURCE_TYPE {
        let _ = crate::typed_value::native_scalar_probe(
            original,
            dialect,
            NativeScalarGetterKind::Wide,
        )?;
    }
    if let Some(NativeScalarCache::Number(tcl_syntax::number::Number::Int(value))) =
        obj::native_scalar_cache(original)?
    {
        return Ok(value);
    }
    #[cfg(not(have_tommath))]
    return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
        "native Jim integer expression backend",
    )
    .into());
    #[cfg(have_tommath)]
    {
        let source = crate::dict::native_object_bytes(
            original,
            tcl_syntax::native_string::NativeStringProtocol::Jim084,
        )?;
        let evaluated = match parse_runtime_expr_cached(interp, Some(original), &source) {
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
        match evaluated {
            Ok(result) => {
                crate::native_source::bind_context(result.as_ptr(), &context)?;
                interp.set_result(result.as_ptr());
                if let Ok(NativeScalarGetterValue::Wide(value)) =
                    crate::typed_value::native_scalar_probe(
                        result.as_ptr(),
                        dialect,
                        NativeScalarGetterKind::Wide,
                    )?
                {
                    return Ok(value);
                }
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
                Some((_, host)) => tcl_registry::InvocationDialect::of_profile(host.profile())
                    .native_string_protocol(),
                None => dialect.native_string_materialization(Some(
                    tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation,
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

    fn ok(i: &mut Interp, src: &[u8]) -> Vec<u8> {
        assert_eq!(
            i.eval_str(src),
            Code::Ok,
            "eval {:?} → {:?}; host refusal: {:?}",
            String::from_utf8_lossy(src),
            String::from_utf8_lossy(&i.result_bytes()),
            i.native_access_refusal()
        );
        i.result_bytes()
    }

    #[cfg(have_tommath)]
    #[test]
    fn logical_parser_requires_an_independent_host_and_separate_cache_policy() {
        use tcl_registry::invocation_words::LogicalExpressionParseProvider;
        let provider = LogicalExpressionParseProvider::Tcl84CoreSimulation;
        leak_free(|interp| {
            let source = tcl_dialect::DialectProfile::irules();
            let host = crate::environment::profile_for_dialect("tcl9.0");
            interp.set_dialect_profile(source);
            assert!(super::parse_runtime_expr(interp, b"{\xff\0tail}").is_err());
            assert!(!interp.set_logical_expression_parse_provider(provider, source));
            assert!(interp.set_logical_expression_parse_provider(provider, host));
            let object = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"{\xff\0tail}"));
            super::parse_runtime_expr_cached(interp, Some(object.as_ptr()), b"{\xff\0tail}")
                .expect("explicit logical parsing");
            let dialect = interp.native_invocation_dialect();
            assert!(crate::expr::cached_expr(object.as_ptr(), dialect, None).is_none());
            assert!(crate::expr::cached_expr(
                object.as_ptr(),
                dialect,
                interp.logical_expression_parse_policy()
            )
            .is_some());
            assert_eq!(crate::interp::obj_bytes(object.as_ptr()), b"{\xff\0tail}");
            assert!(dialect.execution_point().is_none());
        });
    }

    #[cfg(have_tommath)]
    #[test]
    fn logical_parser_host_changes_retire_cached_policy_without_rebinding_objects() {
        use tcl_registry::invocation_words::LogicalExpressionParseProvider;
        let provider = LogicalExpressionParseProvider::Tcl84CoreSimulation;
        leak_free(|interp| {
            let source = tcl_dialect::DialectProfile::irules();
            let host = crate::environment::profile_for_dialect("tcl9.0");
            let replacement = crate::environment::profile_for_dialect("tcl9.1");
            interp.set_dialect_profile(source);
            assert!(interp.set_logical_expression_parse_provider(provider, host));
            let original = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"{\xff\0tail}"));
            let pointer = original.as_ptr();
            let dialect = interp.native_invocation_dialect();
            let policy = interp.logical_expression_parse_policy();
            let parsed = super::parse_runtime_expr_cached(interp, Some(pointer), b"{\xff\0tail}")
                .expect("authored logical parser");

            assert!(interp.set_logical_expression_parse_provider(provider, host));
            assert!(!interp.set_logical_expression_parse_provider(provider, source));
            assert_eq!(interp.logical_expression_parse_policy(), policy);
            let retained = crate::expr::cached_expr(pointer, dialect, policy).unwrap();
            assert!(std::rc::Rc::ptr_eq(&parsed, &retained));

            assert!(interp.set_logical_expression_parse_provider(provider, replacement));
            let current_policy = interp.logical_expression_parse_policy();
            assert_ne!(current_policy, policy);
            assert!(crate::expr::cached_expr(pointer, dialect, current_policy).is_none());
            assert!(std::rc::Rc::ptr_eq(
                &parsed,
                &crate::expr::cached_expr(pointer, dialect, policy).unwrap(),
            ));
            assert_eq!(original.as_ptr(), pointer);
            assert_eq!(crate::interp::obj_bytes(pointer), b"{\xff\0tail}");

            let reparsed = super::parse_runtime_expr_cached(interp, Some(pointer), b"{\xff\0tail}")
                .expect("replacement host policy");
            assert!(!std::rc::Rc::ptr_eq(&parsed, &reparsed));
            assert!(crate::expr::cached_expr(pointer, dialect, policy).is_none());
            assert!(std::rc::Rc::ptr_eq(
                &reparsed,
                &crate::expr::cached_expr(pointer, dialect, current_policy).unwrap(),
            ));
            assert!(interp
                .native_compiler_cache_epochs(crate::namespace::GLOBAL)
                .is_none());
        });
    }

    #[cfg(have_tommath)]
    #[test]
    fn logical_expression_materialises_original_storage_under_its_actual_host() {
        use tcl_registry::invocation_words::LogicalExpressionParseProvider;
        leak_free(|interp| {
            let source = tcl_dialect::DialectProfile::irules();
            let host = crate::environment::profile_for_dialect("tcl9.0");
            interp.set_dialect_profile(source);
            assert!(interp.set_logical_expression_parse_provider(
                LogicalExpressionParseProvider::Tcl84CoreSimulation,
                host,
            ));
            let recipe = tcl_registry::InvocationDialect::of_profile(host)
                .byte_array_string_recipe(None)
                .unwrap();
            let object = crate::obj::Owned::fresh(crate::bytearray::new_byte_array(b"1+2", recipe));
            assert!(!crate::obj::has_string_rep(object.as_ptr()));
            super::parse_runtime_expr_cached(interp, Some(object.as_ptr()), b"IGNORED")
                .expect("actual host byte-array materialisation and logical syntax");
            assert_eq!(crate::obj::bytes_of(object.as_ptr()), b"1+2");
            assert!(crate::expr::cached_expr(
                object.as_ptr(),
                interp.native_invocation_dialect(),
                interp.logical_expression_parse_policy()
            )
            .is_some());
        });
    }

    #[cfg(have_tommath)]
    #[test]
    fn jim_safe_integer_expression_matches_54_original_native_objects() {
        fn unhex(encoded: &str) -> Vec<u8> {
            if encoded == "-" {
                return Vec::new();
            }
            encoded
                .as_bytes()
                .chunks_exact(2)
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect()
        }
        leak_free(|interp| {
            interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
            let context = interp.native_jim_object_context().unwrap();
            interp.set_result_bytes(b"k 10");
            let mut compared = 0;
            for row in include_str!(
                "../../../rust/tcl-syntax/testdata/native_jim_safe_expression/observations.tsv"
            )
            .lines()
            {
                let fields: Vec<_> = row.split('\t').collect();
                let bytes = unhex(fields[9]);
                let original = crate::obj::Owned::fresh(crate::obj::new_string_bytes(&bytes));
                crate::native_source::bind_context(original.as_ptr(), &context).unwrap();
                let filename = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"file"));
                match fields[0] {
                    "0" => {}
                    "1" => crate::native_source::install_source(
                        original.as_ptr(),
                        crate::native_source::NativeJimSourceInfo { filename, line: 7 },
                        &context,
                    )
                    .unwrap(),
                    "2" => {
                        crate::obj::jim_character_count(original.as_ptr());
                    }
                    _ => panic!("fixed native original mode"),
                }
                let result = super::native_jim_wide_expression(interp, original.as_ptr());
                if let Err(error) = &result {
                    assert!(
                        error.native_access_refusal().is_none(),
                        "native getter guest failure must retain its channel"
                    );
                }
                assert_eq!(
                    result.is_err(),
                    fields[4] == "1",
                    "mode{} case{}",
                    fields[0],
                    fields[1]
                );
                assert_eq!(
                    result.unwrap_or(123),
                    fields[5].parse::<i64>().unwrap(),
                    "mode{} case{}",
                    fields[0],
                    fields[1]
                );
                let kind = crate::obj::obj_type_ptr(original.as_ptr());
                let primary = if kind.is_null() {
                    "none"
                } else if kind == &crate::expr::JIM_EXPR_TYPE {
                    "expression"
                } else if kind == &crate::native_source::JIM_SOURCE_TYPE {
                    "source"
                } else if kind == &crate::obj::TCL_INT_TYPE {
                    "int"
                } else if kind == &crate::obj::JIM_STRING_TYPE {
                    "string"
                } else {
                    panic!("unexpected original Jim primary")
                };
                assert_eq!(primary, fields[6], "mode{} case{}", fields[0], fields[1]);
                assert_eq!(
                    crate::obj::has_string_rep(original.as_ptr()),
                    fields[7] == "1"
                );
                assert_eq!(
                    interp.result_bytes(),
                    unhex(fields[8]),
                    "mode{} case{}",
                    fields[0],
                    fields[1]
                );
                compared += 1;
            }
            assert_eq!(compared, 54);
        });
    }

    #[cfg(have_tommath)]
    #[test]
    fn jim_expression_preparation_uses_native_cache_action() {
        leak_free(|interp| {
            interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
            let context = interp.native_jim_object_context().unwrap();
            let filename = crate::obj::Owned::fresh(crate::obj::new_wide_int_obj(17));
            for (bytes, preserves_source) in [
                (&b""[..], true),
                (&b"{"[..], true),
                (&b"\""[..], true),
                (&b"["[..], true),
                (&b"$"[..], false),
                (&b"1+"[..], false),
                (&b"1 2"[..], false),
                (&b"."[..], false),
            ] {
                let original = crate::obj::Owned::fresh(crate::obj::new_string_bytes(bytes));
                crate::native_source::install_source(
                    original.as_ptr(),
                    crate::native_source::NativeJimSourceInfo {
                        filename: filename.clone(),
                        line: 10,
                    },
                    &context,
                )
                .unwrap();
                assert!(
                    super::parse_runtime_expr_cached(interp, Some(original.as_ptr()), bytes)
                        .is_err()
                );
                let expected = if preserves_source {
                    &crate::native_source::JIM_SOURCE_TYPE
                } else {
                    &crate::expr::JIM_EXPR_TYPE
                };
                assert!(std::ptr::eq(
                    crate::obj::obj_type_ptr(original.as_ptr()),
                    expected
                ));
                assert!(!crate::obj::has_string_rep(filename.as_ptr()));
                assert_eq!(crate::obj::bytes_of(original.as_ptr()), bytes);
            }
        });
    }

    #[cfg(have_tommath)]
    #[test]
    fn expression_source_preparation_replaces_actual_typed_storage() {
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            leak_free(|interp| {
                interp.set_dialect_profile(crate::environment::profile_for_dialect(environment));
                let one = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"1"));
                let plus = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"+"));
                let two = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"2"));
                for valid in [true, false] {
                    let elements = [one.as_ptr(), plus.as_ptr(), two.as_ptr()];
                    let source = crate::obj::Owned::fresh(
                        interp.new_list_object(&elements[..if valid { 3 } else { 2 }]),
                    );
                    let previous = crate::obj::obj_type_ptr(source.as_ptr());
                    let bytes = crate::interp::obj_bytes(source.as_ptr());
                    let result =
                        super::parse_runtime_expr_cached(interp, Some(source.as_ptr()), &bytes);
                    assert_eq!(result.is_ok(), valid, "{environment}");
                    if valid || environment != "tcl8.4" {
                        assert!(
                            core::ptr::eq(
                                crate::obj::obj_type_ptr(source.as_ptr()),
                                if environment == "jim" {
                                    &crate::expr::JIM_EXPR_TYPE
                                } else {
                                    &crate::expr::TCL_EXPR_TYPE
                                }
                            ),
                            "{environment}"
                        );
                    } else {
                        assert_eq!(crate::obj::obj_type_ptr(source.as_ptr()), previous);
                    }
                    assert_eq!(
                        crate::interp::obj_bytes(source.as_ptr()),
                        if valid { &b"1 + 2"[..] } else { &b"1 +"[..] }
                    );
                }
            });
        }
    }

    #[cfg(have_tommath)]
    #[test]
    fn source_word_simulation_is_independent_of_expression_parser_installation() {
        leak_free(|interp| {
            let source = tcl_dialect::DialectProfile::irules();
            let host = crate::environment::profile_for_dialect("tcl9.0");
            let provider =
                tcl_registry::invocation_words::LogicalSourceWordProvider::Tcl84CoreSimulation;
            interp.set_dialect_profile(source);
            assert!(interp.source_string_protocol().is_none());
            assert!(!interp.set_logical_source_word_provider(provider, source));
            assert!(interp.set_logical_source_word_provider(provider, host));
            assert!(interp.logical_expression_parse_policy().is_none());
            assert_eq!(
                interp.source_string_protocol(),
                Some(tcl_syntax::native_string::NativeStringProtocol::C(
                    tcl_dialect::TclVersion::V8_4
                ))
            );
            assert_eq!(
                interp.do_expression_subst(br"\u0000", false),
                Ok(vec![0xc0, 0x80])
            );
            assert!(interp
                .native_invocation_dialect()
                .execution_point()
                .is_none());
        });
    }

    #[cfg(have_tommath)]
    #[test]
    fn original_expression_objects_preserve_opaque_source_and_values() {
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            leak_free(|interp| {
                interp.set_dialect_profile(crate::environment::profile_for_dialect(environment));
                let stored = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"7"));
                interp
                    .var_set(b"\xff", stored.as_ptr())
                    .expect("original byte variable name");
                let head = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"expr"));
                for (source, expected) in [
                    (&b"\"\xff\0tail\""[..], &b"\xff\0tail"[..]),
                    (&b"{\xff\0tail}"[..], &b"\xff\0tail"[..]),
                    (&b"${\xff}"[..], &b"7"[..]),
                    (&b"\"${\xff}\""[..], &b"7"[..]),
                    (&b"0 && \"${\xffmissing}\""[..], &b"0"[..]),
                ] {
                    let object = crate::obj::Owned::fresh(crate::obj::new_string_bytes(source));
                    for _ in 0..2 {
                        assert_eq!(
                            super::expr_cmd(interp, &[head.as_ptr(), object.as_ptr()]),
                            Code::Ok,
                            "{environment}: {:?}",
                            interp.result_bytes()
                        );
                        assert_eq!(interp.result_bytes(), expected, "{environment}");
                        assert_eq!(crate::interp::obj_bytes(object.as_ptr()), source);
                    }
                }
            });
        }
    }

    #[cfg(have_tommath)]
    #[test]
    fn expression_arguments_use_native_concatenation_and_selected_cardinality() {
        let case = tcl_syntax::execution_conformance::vectors(
            tcl_syntax::execution_conformance::ExecutionDomain::CommandBinding,
        )
        .into_iter()
        .find(|case| case.id == "expression_argument_concatenation_preserves_native_values")
        .expect("shared native expression argument vector");
        let script = format!("list [catch {{{}}} value] $value", case.source);
        for reference in tcl_test_support::available_tclshs() {
            let expected = tcl_test_support::run_script(&reference.path, case.script().as_bytes())
                .expect("native expression arguments")
                .strict_text()
                .expect("native result");
            leak_free(|interp| {
                interp.set_runtime_version(reference.version);
                assert_eq!(interp.eval_str(script.as_bytes()), Code::Ok);
                assert_eq!(interp.result_bytes(), expected.as_bytes());
            });
        }
        if let Some(reference) = tcl_test_support::locate_jimsh().expect("Jim discovery") {
            let expected = tcl_test_support::run_script(&reference.path, case.script().as_bytes())
                .expect("native Jim expression arguments")
                .strict_text()
                .expect("native result");
            leak_free(|interp| {
                interp.set_dialect_profile(
                    tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
                );
                assert_eq!(interp.eval_str(script.as_bytes()), Code::Ok);
                assert_eq!(interp.result_bytes(), expected.as_bytes());
            });
        }
    }

    #[cfg(have_tommath)]
    #[test]
    fn expression_variable_references_preserve_literal_indices_and_fire_traces() {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            leak_free(|i| {
                let profile = if dialect == "jim" {
                    Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
                        "jim",
                        &[],
                        "Jim",
                        tcl_dialect::model::DialectPoint::canonical(
                            tcl_dialect::model::Release::JIM_0_84,
                        ),
                    )))
                } else {
                    tcl_dialect::DialectProfile::find(dialect).expect("profile")
                };
                i.set_dialect_profile(profile);
                assert_eq!(ok(i, br#"set {a([step])} LITERAL; set a(k) LIVE; set hits 0; proc step {} {incr ::hits; return k}; set literal [expr {${a([step])}}]; set live [expr {$a([step])}]; list $literal $live $hits"#), b"LITERAL LIVE 1", "{dialect}");
                if dialect != "jim" {
                    assert_eq!(ok(i, br#"set x 5; set reads 0; proc tr {args} {incr ::reads}; trace add variable x read tr; list [expr {$x + 1}] $reads"#), b"6 1", "{dialect}");
                }
            });
        }
    }

    #[cfg(have_tommath)]
    #[test]
    fn native_expression_and_increment_follow_selected_numeric_towers() {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            leak_free(|interp| {
                interp.set_dialect_profile(
                    tcl_registry::model::ingress::resolve_environment(dialect).unit_profile(),
                );
                let fixed = matches!(dialect, "tcl8.4" | "jim");
                assert_eq!(
                    ok(interp, b"expr {(9223372036854775807 + 1) < 0}"),
                    if fixed {
                        b"1".as_slice()
                    } else {
                        b"0".as_slice()
                    },
                    "{dialect}"
                );
                assert_eq!(
                    ok(interp, b"set n 9223372036854775807; incr n; expr {$n < 0}"),
                    if fixed {
                        b"1".as_slice()
                    } else {
                        b"0".as_slice()
                    },
                    "{dialect}"
                );
                assert_eq!(
                    ok(interp, b"set n 18446744073709551615; incr n"),
                    if fixed {
                        b"0".as_slice()
                    } else {
                        b"18446744073709551616".as_slice()
                    },
                    "{dialect}"
                );
                assert_eq!(
                    ok(interp, b"set n 010; incr n"),
                    if matches!(dialect, "jim" | "tcl9.0" | "tcl9.1") {
                        b"11".as_slice()
                    } else {
                        b"9".as_slice()
                    },
                    "{dialect}"
                );
                assert_eq!(
                    ok(interp, b"expr {-(-9223372036854775808) < 0}"),
                    if fixed {
                        b"1".as_slice()
                    } else {
                        b"0".as_slice()
                    },
                    "{dialect}"
                );
                assert_eq!(
                    ok(interp, b"expr {\"010\"}"),
                    if dialect == "jim" {
                        b"010".as_slice()
                    } else if fixed {
                        b"8".as_slice()
                    } else if matches!(dialect, "tcl9.0" | "tcl9.1") {
                        b"10".as_slice()
                    } else {
                        b"8".as_slice()
                    }
                );
                assert_eq!(
                    ok(interp, b"set n 18446744073709551615; expr {$n + 0.0 < 0.0}"),
                    if fixed {
                        b"1".as_slice()
                    } else {
                        b"0".as_slice()
                    }
                );
                assert_eq!(
                    ok(interp, b"set n 18446744073709551615; expr {$n > 0.0}"),
                    if fixed {
                        b"0".as_slice()
                    } else {
                        b"1".as_slice()
                    }
                );
                if dialect == "tcl8.4" {
                    assert_eq!(
                        interp.eval_str(b"set n 18446744073709551616; incr n"),
                        Code::Error
                    );
                    assert_eq!(
                        ok(interp, b"set ::errorCode"),
                        b"ARITH IOVERFLOW {integer value too large to represent}"
                    );
                } else if dialect == "jim" {
                    assert_eq!(ok(interp, b"set n 18446744073709551616; incr n"), b"0");
                    assert_eq!(ok(interp, b"expr {1 << 64}"), b"1");
                    assert_eq!(interp.eval_str(b"expr {bool(1)}"), Code::Error);
                }
            });
        }
    }

    /// A loop's condition is parsed **once**, not once per iteration: the
    /// parsed AST caches on the condition object as `TCL_EXPR_TYPE`. A
    /// `parse_runtime_expr` call that re-lexed, re-parsed and re-validated
    /// the condition text on every evaluation would cost this repeatedly on
    /// every pass of a hot loop.
    #[cfg(have_tommath)]
    #[test]
    fn a_loop_condition_parses_once_for_the_whole_loop() {
        leak_free(|i| {
            crate::expr::reset_expr_parse_count();
            assert_eq!(ok(i, b"set n 0; while {$n < 20} { incr n }; set n"), b"20");
            assert_eq!(
                crate::expr::expr_parse_count(),
                1,
                "twenty-one condition evaluations, one parse"
            );
        });
    }

    /// The cached rep sits on the condition object, keeps its spelling, and is
    /// reused on the next evaluation of that same object.
    #[cfg(have_tommath)]
    #[test]
    fn a_condition_object_caches_its_ast_and_keeps_its_spelling() {
        leak_free(|i| {
            let cond = crate::obj::new_string_bytes(b"1 < 2");
            // SAFETY: the test owns this reference until it releases it below.
            unsafe { crate::obj::incr_ref_count(cond) };

            crate::expr::reset_expr_parse_count();
            assert_eq!(crate::builtins::eval_bool_expr(i, cond), Ok(true));
            assert_eq!(crate::expr::expr_parse_count(), 1);
            assert!(
                core::ptr::eq(crate::obj::obj_type_ptr(cond), &crate::expr::TCL_EXPR_TYPE),
                "the parse caches on the condition object"
            );
            assert_eq!(
                crate::interp::obj_bytes(cond),
                b"1 < 2",
                "the spelling survives the shimmer"
            );

            assert_eq!(crate::builtins::eval_bool_expr(i, cond), Ok(true));
            assert_eq!(
                crate::expr::expr_parse_count(),
                1,
                "the second evaluation reuses the cached AST"
            );

            // A shimmer to any other rep drops the cache, exactly like any
            // internal rep, and the next evaluation re-parses.
            assert_eq!(crate::list::list_length(cond), Ok(3));
            assert!(!core::ptr::eq(
                crate::obj::obj_type_ptr(cond),
                &crate::expr::TCL_EXPR_TYPE
            ));
            assert_eq!(crate::builtins::eval_bool_expr(i, cond), Ok(true));
            assert_eq!(crate::expr::expr_parse_count(), 2);

            // SAFETY: balances the reference taken above.
            unsafe { crate::obj::decr_ref_count(cond) };
        });
    }

    /// The cache records the release its registry validation ran under, so an
    /// embedder pinning another emulated Tcl re-validates rather than reusing an
    /// admission that release never granted.
    #[cfg(have_tommath)]
    #[test]
    fn a_cached_expression_is_revalidated_when_the_release_changes() {
        leak_free(|i| {
            let cond = crate::obj::new_string_bytes(b"1 < 2");
            // SAFETY: the test owns this reference until it releases it below.
            unsafe { crate::obj::incr_ref_count(cond) };

            crate::expr::reset_expr_parse_count();
            assert_eq!(crate::builtins::eval_bool_expr(i, cond), Ok(true));
            assert_eq!(crate::expr::expr_parse_count(), 1);

            i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            assert_eq!(crate::builtins::eval_bool_expr(i, cond), Ok(true));
            assert_eq!(
                crate::expr::expr_parse_count(),
                2,
                "a release change stales the cached admission"
            );

            // SAFETY: balances the reference taken above.
            unsafe { crate::obj::decr_ref_count(cond) };
        });
    }

    #[test]
    fn raw_cr_escapes_match_tcl9() {
        leak_free(|i| {
            // TclParseBackslash only collapses backslash-LF. Raw CR and CRLF
            // handed to the parser remain data; brace words preserve the slash.
            assert_eq!(ok(i, b"set x \"a\\\rb\"; set x"), b"a\rb");
            assert_eq!(ok(i, b"set x \"a\\\r\nb\"; set x"), b"a\r\nb");
            assert_eq!(ok(i, b"set x {a\\\rb}; set x"), b"a\\\rb");
            assert_eq!(ok(i, b"set x \"a\\\nb\"; set x"), b"a b");
        });
    }

    #[test]
    fn subst_variables_commands_backslashes() {
        leak_free(|i| {
            ok(i, b"set x hello");
            assert_eq!(ok(i, b"subst {$x world}"), b"hello world");
            assert_eq!(ok(i, b"subst {[set x] world}"), b"hello world");
            // -nocommands leaves [ ] literal but still substitutes $x.
            assert_eq!(ok(i, b"subst -nocommands {$x [foo]}"), b"hello [foo]");
            // -novariables leaves $x literal.
            assert_eq!(ok(i, b"subst -novariables {$x}"), b"$x");
            // an unset variable is an error.
            assert_eq!(i.eval_str(b"subst {$nope}"), Code::Error);
            i.eval_str(b"unset -nocomplain x");
        });
    }

    /// `subst`'s option words go through the one shared
    /// `tcl-cmd-core::prefix` matcher, so every miss is worded exactly as
    /// `Tcl_GetIndexFromObj` at flags `0` words it (`TclSubstOptions`,
    /// `tclCmdMZ.c:3341`): the empty word prefixes all three entries and is
    /// therefore `ambiguous`, not `bad`.
    #[test]
    fn subst_option_words_resolve_like_tcl_get_index_from_obj() {
        const MUST: &str = "must be -nobackslashes, -nocommands, or -novariables";
        leak_free(|i| {
            assert_eq!(i.eval_str(b"subst {} abc"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                format!("ambiguous option \"\": {MUST}").as_bytes()
            );
            assert_eq!(i.eval_str(b"subst -no abc"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                format!("ambiguous option \"-no\": {MUST}").as_bytes()
            );
            assert_eq!(i.eval_str(b"subst -q abc"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                format!("bad option \"-q\": {MUST}").as_bytes()
            );
            // Unique prefixes still resolve (subst-7.7).
            assert_eq!(ok(i, b"subst -nov {$x}"), b"$x");
            assert_eq!(ok(i, br"subst -nob {a\tb}"), br"a\tb");
            assert_eq!(ok(i, b"subst -noc {[cmd]}"), b"[cmd]");
        });
    }

    /// Tcl 8.6/9.0 (`Tcl_ParseVarName`, `generic/tclParse.c`) consumes the
    /// complete colon separator run before looking up an unbraced variable.
    #[test]
    fn subst_variable_colon_runs_match_tcl() {
        leak_free(|i| {
            ok(i, b"namespace eval a {}");
            ok(i, b"set ::a::b VALUE");
            ok(i, b"set ::a::arr(k) ARRAY");
            assert_eq!(
                ok(i, b"subst {$a:::b $::a:::b $a:::arr(k) $::a:::arr(k)}"),
                b"VALUE VALUE ARRAY ARRAY"
            );
            ok(i, b"namespace eval foo {}");
            ok(i, b"set ::foo::: EMPTY");
            assert_eq!(ok(i, b"subst {$foo:::}"), b"EMPTY");
            assert_eq!(i.eval_str(b"subst {$missing:::b}"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"can't read \"missing:::b\": no such variable"
            );
        });
    }

    /// An unterminated `${…}` is `Tcl_ParseVarName`'s
    /// `missing close-brace for variable name` — **not** a name that runs to
    /// end-of-input, and not a literal `$`.
    ///
    /// A `.unwrap_or(len)` scan of the name would silently swallow the rest
    /// of the template (and, for `${a\`, drop the backslash and succeed
    /// where C errors). The scan reports
    /// [`tcl_lexer::BracedVarEnd::Unterminated`] instead, and the eval loop
    /// raises.
    ///
    /// The error is raised in evaluation order, so a `[...]` earlier in the
    /// same template has already run — verified against tclsh 8.6.16 and
    /// 9.0.4, which both leave the counter at 1.
    /// Templates are built with `binary format H*` so the *script* parser's own
    /// brace matching cannot reshape the bytes `subst` is handed — writing
    /// `subst {${abc}` instead makes the script parser hand over `${abc}`,
    /// which is a perfectly terminated reference and tests nothing.
    #[test]
    fn unterminated_braced_var_raises_missing_close_brace() {
        const MSG: &[u8] = b"missing close-brace for variable name";
        leak_free(|i| {
            for hex in [
                &b"247B616263"[..],   // `${abc`  — no closer at all
                &b"7A247B617B62"[..], // `z${a{b` — unbalanced `{`
                &b"247B615C"[..],     // `${a\`   — trailing lone backslash
            ] {
                let script = [b"subst [binary format H* ".as_ref(), hex, b"]"].concat();
                assert_eq!(i.eval_str(&script), Code::Error, "{hex:?}");
                assert_eq!(i.result_bytes(), MSG, "{hex:?}");
            }
            // A *closed* form still resolves normally: `${a{b}c}` under the
            // default 9.x nesting rule names the variable `a{b}c`.
            ok(i, b"set {a{b}c} WORLD");
            assert_eq!(
                ok(i, b"subst [binary format H* 247B617B627D637D]"),
                b"WORLD"
            );

            // Side effects before the bad `${` survive, as in C: `[incr c]${a{b`
            // leaves the counter at 1 on both tclsh 8.6.16 and 9.0.4.
            ok(i, b"set c 0");
            assert_eq!(
                i.eval_str(b"subst [binary format H* 5B696E637220635D247B617B62]"),
                Code::Error
            );
            assert_eq!(i.result_bytes(), MSG);
            assert_eq!(ok(i, b"set c"), b"1");
        });
    }

    /// A hand-rolled `starts_with` filter for `interp limit`'s option matcher
    /// could only ever say `bad option` (never `ambiguous`), and would
    /// hand-build its own `", or"` enumeration beside the one
    /// `prefix::choice_list_bytes` owns. Both come from
    /// `OptionTable::abbreviating` instead.
    ///
    /// Byte-checked against tclsh 8.6.16 and 9.0.4.
    #[test]
    fn interp_limit_option_words_resolve_like_tcl_get_index_from_obj() {
        const MUST: &str = "must be -command, -granularity, -milliseconds, or -seconds";
        leak_free(|i| {
            ok(i, b"interp create i");
            // The empty word prefixes every entry ⇒ ambiguous, not bad.
            assert_eq!(i.eval_str(b"interp limit i time {}"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                format!("ambiguous option \"\": {MUST}").as_bytes()
            );
            assert_eq!(i.eval_str(b"interp limit i time -"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                format!("ambiguous option \"-\": {MUST}").as_bytes()
            );
            // A word prefixing nothing is bad.
            assert_eq!(i.eval_str(b"interp limit i time -zz"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                format!("bad option \"-zz\": {MUST}").as_bytes()
            );
            // A unique prefix still resolves.
            assert_eq!(i.eval_str(b"interp limit i time -sec 5"), Code::Ok);
        });
    }

    /// `interp debug`'s option word is a `Tcl_GetIndexFromObj`
    /// table whose noun is `debug option` (`debugTypes[]`, `tclInterp.c`), so
    /// `-f`/`-fr` abbreviate and the one-entry table never says `ambiguous`.
    ///
    /// tclsh 8.6.16 / 9.0.4:
    ///   interp debug i {}     -> bad debug option "": must be -frame
    ///   interp debug i -x     -> bad debug option "-x": must be -frame
    ///   interp debug i -f     -> 0
    ///   interp debug i -fr 1  -> 1
    ///   i debug -f            -> 1   (after the latch)
    ///   i debug -x 1 2        -> wrong # args: should be "i debug ?-frame ?bool??"
    #[test]
    fn interp_debug_option_uses_c_noun_and_abbreviates() {
        leak_free(|i| {
            ok(i, b"interp create i");
            assert_eq!(i.eval_str(b"interp debug i {}"), Code::Error);
            assert_eq!(i.result_bytes(), b"bad debug option \"\": must be -frame");
            assert_eq!(i.eval_str(b"interp debug i -x"), Code::Error);
            assert_eq!(i.result_bytes(), b"bad debug option \"-x\": must be -frame");
            assert_eq!(ok(i, b"interp debug i -f"), b"0");
            assert_eq!(ok(i, b"interp debug i -fr 1"), b"1");
            assert_eq!(ok(i, b"i debug -f"), b"1");
            assert_eq!(i.eval_str(b"i debug -x 1 2"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"wrong # args: should be \"i debug ?-frame ?bool??\""
            );
        });
    }

    /// `interp limit`'s type word is `Tcl_GetIndexFromObj(…,
    /// "limit type", 0)` (`limitTypes[]`, `tclInterp.c`), so `c`/`t`
    /// abbreviate and the empty word — a prefix of both entries — is
    /// `ambiguous`.
    ///
    /// tclsh 8.6.16 / 9.0.4:
    ///   interp limit i {} -> ambiguous limit type "": must be commands or time
    ///   interp limit i x  -> bad limit type "x": must be commands or time
    ///   interp limit i c  -> -command {} -granularity 1 -value {}
    ///   i limit {}        -> ambiguous limit type "": must be commands or time
    #[test]
    fn interp_limit_type_word_resolves_like_tcl_get_index_from_obj() {
        const MUST: &str = "must be commands or time";
        leak_free(|i| {
            ok(i, b"interp create i");
            assert_eq!(i.eval_str(b"interp limit i {}"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                format!("ambiguous limit type \"\": {MUST}").as_bytes()
            );
            assert_eq!(i.eval_str(b"interp limit i x"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                format!("bad limit type \"x\": {MUST}").as_bytes()
            );
            assert_eq!(
                ok(i, b"interp limit i c"),
                b"-command {} -granularity 1 -value {}"
            );
            assert_eq!(i.eval_str(b"i limit {}"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                format!("ambiguous limit type \"\": {MUST}").as_bytes()
            );
            assert_eq!(ok(i, b"i limit c"), b"-command {} -granularity 1 -value {}");
        });
    }

    #[test]
    fn unset_nocomplain_and_dashdash() {
        leak_free(|i| {
            // -nocomplain suppresses the no-such-variable error.
            assert_eq!(i.eval_str(b"unset -nocomplain nope"), Code::Ok);
            // without it, unsetting a missing var errors.
            assert_eq!(i.eval_str(b"unset alsonope"), Code::Error);
            // `--` ends options, so a var literally named -nocomplain is unset.
            ok(i, b"set -nocomplain 1");
            assert_eq!(i.eval_str(b"unset -- -nocomplain"), Code::Ok);
            assert_eq!(i.eval_str(b"info exists -nocomplain"), Code::Ok);
            assert_eq!(i.result_bytes(), b"0");
        });
    }
}
