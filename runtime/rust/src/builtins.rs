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

//! Built-in commands (T1.4 starter set).
//!
//! A minimal set — `set`, `incr`, `return`, `unset` — sufficient to drive the
//! eval loop end to end and prove command substitution + variable integration.
//! The full builtin surface (string/list/dict/expr/control-flow/proc/…) is
//! ported incrementally in T1.5, each command (or small group) as its own gated
//! change with its tcltest delta.
//!
//! Each handler matches the [`BuiltinFn`](crate::interp::BuiltinFn) shape:
//! `argv[0]` is the command name (Tcl's `objv` convention).

use crate::frame::{split_array_ref, VarError};
use crate::interp::{drop_fresh, obj_bytes, Code, Interp};
use crate::obj::{self, TclObj};

/// Register the starter builtins on a fresh interp.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"set", set);
    interp.register_builtin(b"incr", incr);
    interp.register_builtin(b"const", const_cmd);
    interp.register_builtin(b"return", ret);
    interp.register_builtin(b"unset", unset);
    interp.register_builtin(b"exit", exit_cmd);
    interp.register_builtin(b"subst", subst_cmd);
    crate::cmd_scan::install(interp);
    crate::cmd_format::install(interp);
    crate::cmd_binary::install(interp);
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
    crate::cmd_string::install(interp);
    crate::cmd_alias::install(interp);
    crate::cmd_namespace::install(interp);
    crate::cmd_var::install(interp);
    crate::cmd_control::install(interp);
    crate::cmd_proc::install(interp);
    crate::cmd_error::install(interp);
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
    crate::cmd_oo::install(interp);
}

pub(crate) fn var_error(interp: &mut Interp, name: &[u8], e: VarError) -> Code {
    // A write-trace error: wrap the trace's own message (stashed in pending_err)
    // as `can't set "name": <msg>` (C's TclObjVarErrMsg).
    if e == VarError::TraceError {
        let reason = interp
            .traces
            .borrow_mut()
            .pending_err
            .take()
            .unwrap_or_default();
        let mut msg = b"can't set \"".to_vec();
        msg.extend_from_slice(name);
        msg.extend_from_slice(b"\": ");
        msg.extend_from_slice(&reason);
        return interp.set_error(&msg);
    }
    let verb = match e {
        VarError::IsArray => &b"\": variable is array"[..],
        VarError::IsScalar => &b"\": variable isn't array"[..],
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
    let (base, elem) = split_array_ref(&name);
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

// -- set -------------------------------------------------------------------

/// `set varName ?value?` — write (returns the value) or read (returns it).
fn set(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    match argv.len() {
        2 => {
            let name = obj_bytes(argv[1]);
            let (base, elem) = split_array_ref(&name);
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
            let (base, elem) = split_array_ref(&name);
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
        _ => interp.wrong_args(b"set varName ?newValue?"),
    }
}

// -- incr ------------------------------------------------------------------

/// `incr varName ?increment?` — add (default 1) over the **numeric tower**,
/// storing and returning the sum. Both operands must be integers; the sum
/// promotes to a bignum on overflow (Tcl integers never wrap) and an existing
/// bignum cell increments correctly. Object-preserving (reads the cell's value
/// object, not its string).
#[cfg(have_tommath)]
fn incr(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 || argv.len() > 3 {
        return interp.wrong_args(b"incr varName ?increment?");
    }
    let name = obj_bytes(argv[1]);
    let (base, elem) = split_array_ref(&name);
    if let Some(c) = incr_constant_error(interp, &base, &elem, &name) {
        return c;
    }

    // Current cell value (borrowed; `None` for an unset variable → the shared
    // seam treats it as 0). The increment is `argv[2]` (borrowed) or a fresh 1.
    let cur = match &elem {
        Some(k) => interp.var_get_elem(&base, k),
        None => interp.var_get(&base),
    };
    let one = obj::new_wide_int_obj(1);
    let amount = if argv.len() == 3 { argv[2] } else { one };

    // The numeric-tower addition is the shared `ValueOps::int_add` seam, run over
    // this runtime's bignum (overflow widens; a non-integer operand is the
    // canonical `expected integer but got "…"`). The store below — with its write
    // traces and the per-runtime result protocol — stays here, since a write
    // trace that errors must still store yet fail the command.
    let sum = tcl_syntax::value::ValueOps::int_add(interp, cur.as_ref(), &amount);
    drop_fresh(one); // the transient `1` (used or not) is no longer needed
    let sum = match sum {
        Ok(s) => s, // rc 0
        Err(e) => return interp.set_error(e.message().as_bytes()),
    };

    let stored = match &elem {
        Some(k) => interp.var_set_elem(&base, k, sum),
        None => interp.var_set(&base, sum),
    };
    match stored {
        Ok(()) => {
            interp.set_result(sum);
            Code::Ok
        }
        Err(e) => {
            drop_fresh(sum);
            var_error(interp, &name, e)
        }
    }
}

/// `incr` fallback for a build without the bignum tower (`have_tommath` off):
/// `i64` only, failing loudly on overflow rather than wrapping.
#[cfg(not(have_tommath))]
fn incr(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 || argv.len() > 3 {
        return interp.wrong_args(b"incr varName ?increment?");
    }
    let name = obj_bytes(argv[1]);
    let (base, elem) = split_array_ref(&name);
    if let Some(c) = incr_constant_error(interp, &base, &elem, &name) {
        return c;
    }

    let cur = match read_cell(interp, &base, &elem) {
        Some(bytes) => match parse_i64(&bytes) {
            Some(n) => n,
            None => return not_integer(interp, &bytes),
        },
        None => 0, // incr of an unset variable starts at 0 (Tcl)
    };
    let amount = if argv.len() == 3 {
        let b = obj_bytes(argv[2]);
        match parse_i64(&b) {
            Some(n) => n,
            None => return not_integer(interp, &b),
        }
    } else {
        1
    };
    let sum = match cur.checked_add(amount) {
        Some(s) => s,
        None => return interp.set_error(b"integer overflow (bignum promotion needs the tower)"),
    };
    let obj = obj::new_wide_int_obj(sum); // rc 0
    let stored = match &elem {
        Some(k) => interp.var_set_elem(&base, k, obj),
        None => interp.var_set(&base, obj),
    };
    match stored {
        Ok(()) => {
            interp.set_result(obj);
            Code::Ok
        }
        Err(e) => {
            drop_fresh(obj);
            var_error(interp, &name, e)
        }
    }
}

#[cfg(not(have_tommath))]
fn read_cell(interp: &Interp, base: &[u8], elem: &Option<Vec<u8>>) -> Option<Vec<u8>> {
    let obj = match elem {
        Some(k) => interp.var_get_elem(base, k),
        None => interp.var_get(base),
    }?;
    Some(obj_bytes(obj))
}

/// The canonical `expected integer but got "…"` for the no-tower `incr`
/// fallback. The tower build reports this through the shared `ValueOps::int_add`
/// seam instead, so this is only needed when `have_tommath` is off.
#[cfg(not(have_tommath))]
fn not_integer(interp: &mut Interp, bytes: &[u8]) -> Code {
    let mut msg = b"expected integer but got \"".to_vec();
    msg.extend_from_slice(bytes);
    msg.push(b'"');
    interp.set_error(&msg)
}

// -- return ----------------------------------------------------------------

/// Map a `-code` word (`ok`/`error`/`return`/`break`/`continue` or any integer)
/// to a [`Code`]; `None` for an unrecognised spelling. A non-0..4 integer maps to
/// [`Code::Other`] (`TclGetCompletionCodeFromObj`).
fn parse_code(b: &[u8]) -> Option<Code> {
    match b {
        b"ok" => Some(Code::Ok),
        b"error" => Some(Code::Error),
        b"return" => Some(Code::Return),
        b"break" => Some(Code::Break),
        b"continue" => Some(Code::Continue),
        _ => crate::interp::parse_completion_int(b).map(Code::from_int),
    }
}

/// `return ?-code code? ?-level n? ?-errorcode list? ?-errorinfo info?
/// ?-options dict? ?result?` — complete with `-code` after unwinding `-level`
/// proc/source boundaries (`Tcl_ReturnObjCmd`). A `-options` dict (as produced
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
            match tcl_cmd_core::sort::parse_wide(&bytes) {
                // C truncates an out-of-`int`-range exit status.
                Some(n) => i32::try_from(n).unwrap_or(n as i32),
                None => {
                    let mut m = b"expected integer but got \"".to_vec();
                    m.extend_from_slice(&bytes);
                    m.push(b'"');
                    return interp.set_error(&m);
                }
            }
        }
        _ => return interp.set_error(b"wrong # args: should be \"exit ?returnCode?\""),
    };
    interp.set_exit(code);
    interp.set_result_bytes(b"");
    Code::Error
}

/// by `catch`) seeds the options; explicit flags override it.
fn ret(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let mut code = Code::Ok;
    let mut level: usize = 1;
    let mut errorcode: Option<Vec<u8>> = None;
    let mut errorinfo: Option<Vec<u8>> = None;
    let mut errorstack: Option<Vec<u8>> = None;

    let mut i = 1;
    while i + 1 < argv.len() {
        let opt = obj_bytes(argv[i]);
        match opt.as_slice() {
            b"-code" => match parse_code(&obj_bytes(argv[i + 1])) {
                Some(c) => code = c,
                None => {
                    let mut m = b"bad completion code \"".to_vec();
                    m.extend_from_slice(&obj_bytes(argv[i + 1]));
                    m.extend_from_slice(
                        b"\": must be ok, error, return, break, continue, or an integer",
                    );
                    return interp.set_error(&m);
                }
            },
            b"-level" => match core::str::from_utf8(&obj_bytes(argv[i + 1]))
                .ok()
                .and_then(|s| s.trim().parse::<usize>().ok())
            {
                Some(l) => level = l,
                None => return interp.set_error(b"bad -level value"),
            },
            b"-errorcode" => errorcode = Some(obj_bytes(argv[i + 1])),
            b"-errorinfo" => errorinfo = Some(obj_bytes(argv[i + 1])),
            b"-errorstack" => errorstack = Some(obj_bytes(argv[i + 1])),
            b"-options" => {
                // Seed code/level/errorcode/errorinfo from the options dict.
                let opts = obj_bytes(argv[i + 1]);
                if let Ok(d) = crate::parse::split_list(&opts) {
                    let mut j = 0;
                    while j + 1 < d.len() {
                        match d[j].as_slice() {
                            b"-code" => code = parse_code(&d[j + 1]).unwrap_or(code),
                            b"-level" => {
                                level = core::str::from_utf8(&d[j + 1])
                                    .ok()
                                    .and_then(|s| s.trim().parse().ok())
                                    .unwrap_or(level);
                            }
                            b"-errorcode" => errorcode = Some(d[j + 1].clone()),
                            b"-errorinfo" => errorinfo = Some(d[j + 1].clone()),
                            b"-errorstack" => errorstack = Some(d[j + 1].clone()),
                            _ => {}
                        }
                        j += 2;
                    }
                }
            }
            _ => break, // not an option → the result word
        }
        i += 2;
    }
    // Validate -errorcode / -errorstack (`TclMergeReturnOptions`). A malformed
    // value errors before the return takes effect, with a specific `-errorcode`.
    if let Some(ec) = &errorcode {
        if crate::parse::split_list(ec).is_err() {
            let mut m = b"bad -errorcode value: expected a list but got \"".to_vec();
            m.extend_from_slice(ec);
            m.push(b'"');
            return interp.error_with_code(&m, b"TCL RESULT ILLEGAL_ERRORCODE");
        }
    }
    if let Some(es) = &errorstack {
        match crate::parse::split_list(es) {
            Err(_) => {
                let mut m = b"bad -errorstack value: expected a list but got \"".to_vec();
                m.extend_from_slice(es);
                m.push(b'"');
                return interp.error_with_code(&m, b"TCL RESULT NONLIST_ERRORSTACK");
            }
            Ok(parts) if parts.len() % 2 != 0 => {
                let mut m = b"forbidden odd-sized list for -errorstack: \"".to_vec();
                m.extend_from_slice(es);
                m.push(b'"');
                return interp.error_with_code(&m, b"TCL RESULT ODDSIZEDLIST_ERRORSTACK");
            }
            Ok(_) => {}
        }
    }
    // The optional trailing result word.
    if i < argv.len() {
        interp.set_result(argv[i]);
    } else {
        interp.set_result_bytes(b"");
    }
    // On an error completion, populate the live exception state from the error
    // options (`TclProcessReturn`) — *not* the `::errorInfo`/`::errorCode`
    // globals, which are written only when the error is reported. This runs
    // regardless of `-level`, so a deferred (`-level > 0`) error carries its
    // info/code once the return boundary turns it into a real error.
    if code == Code::Error {
        interp.process_return_error(
            errorinfo.as_deref(),
            errorcode.as_deref(),
            errorstack.as_deref(),
        );
    }
    if level == 0 {
        // No unwinding: complete with `code` right here.
        code
    } else {
        interp.set_return_state(level, code);
        Code::Return
    }
}

// -- unset -----------------------------------------------------------------

/// `unset varName ...` — remove variables (scalars or array elements).
fn unset(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    // `unset ?-nocomplain? ?--? ?name name ...?`. `-nocomplain` suppresses the
    // no-such-variable error; `--` ends option parsing (so a var literally named
    // `-nocomplain` can still be unset).
    let mut i = 1;
    let mut nocomplain = false;
    while i < argv.len() {
        match obj_bytes(argv[i]).as_slice() {
            b"-nocomplain" => {
                nocomplain = true;
                i += 1;
            }
            b"--" => {
                i += 1;
                break;
            }
            _ => break,
        }
    }
    for &a in &argv[i..] {
        let name = obj_bytes(a);
        let (base, elem) = split_array_ref(&name);
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
        let name = obj_bytes(opt);
        match subst_option_index(&name) {
            SubstOpt::Backslashes => flags.backslashes = false,
            SubstOpt::Commands => flags.cmds = false,
            SubstOpt::Variables => flags.vars = false,
            SubstOpt::Bad => return subst_bad_option(interp, b"bad", &name),
            SubstOpt::Ambiguous => return subst_bad_option(interp, b"ambiguous", &name),
        }
    }
    let last = argv[argv.len() - 1];
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

/// The resolution of a `subst` option word against `{-nobackslashes,
/// -nocommands, -novariables}` (Tcl's exact-or-unique-prefix matching).
enum SubstOpt {
    Backslashes,
    Commands,
    Variables,
    Bad,
    Ambiguous,
}

fn subst_option_index(name: &[u8]) -> SubstOpt {
    const NAMES: [&[u8]; 3] = [b"-nobackslashes", b"-nocommands", b"-novariables"];
    let mut found = None;
    let mut count = 0;
    for (k, n) in NAMES.iter().enumerate() {
        if *n == name {
            count = 1;
            found = Some(k);
            break;
        }
        if n.starts_with(name) {
            found = Some(k);
            count += 1;
        }
    }
    match (count, found) {
        (1, Some(0)) => SubstOpt::Backslashes,
        (1, Some(1)) => SubstOpt::Commands,
        (1, Some(2)) => SubstOpt::Variables,
        (0, _) => SubstOpt::Bad,
        _ => SubstOpt::Ambiguous,
    }
}

fn subst_bad_option(interp: &mut Interp, kind: &[u8], name: &[u8]) -> Code {
    let mut m = kind.to_vec();
    m.extend_from_slice(b" option \"");
    m.extend_from_slice(name);
    m.extend_from_slice(b"\": must be -nobackslashes, -nocommands, or -novariables");
    interp.set_error(&m)
}

// -- helpers ---------------------------------------------------------------

/// Minimal Tcl integer parse for the no-tower `incr` fallback (the tower build
/// reads operands through `tcl_syntax::number` via `bignum`).
#[cfg(not(have_tommath))]
fn parse_i64(bytes: &[u8]) -> Option<i64> {
    let s = bytes;
    let mut i = 0;
    let len = s.len();
    while i < len && s[i].is_ascii_whitespace() {
        i += 1;
    }
    let mut end = len;
    while end > i && s[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    let s = &s[i..end];
    if s.is_empty() {
        return None;
    }
    let (neg, s) = match s[0] {
        b'-' => (true, &s[1..]),
        b'+' => (false, &s[1..]),
        _ => (false, s),
    };
    if s.is_empty() {
        return None;
    }
    let (radix, digits) = if s.len() > 2 && s[0] == b'0' && (s[1] == b'x' || s[1] == b'X') {
        (16u32, &s[2..])
    } else {
        (10u32, s)
    };
    if digits.is_empty() {
        return None;
    }
    let mut acc: i64 = 0;
    for &c in digits {
        let d = (c as char).to_digit(radix)? as i64;
        acc = acc.checked_mul(radix as i64)?.checked_add(d)?;
    }
    Some(if neg { -acc } else { acc })
}

// -- expr ------------------------------------------------------------------

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
impl crate::expr::ExprCtx for InterpExprCtx<'_> {
    fn read_var(&mut self, name: &str) -> Result<crate::expr::Owned, crate::expr::ExprError> {
        let (base, elem) = split_array_ref(name.as_bytes());
        // `expr {$arr($i)}`: the array index is itself substituted (Tcl parses
        // `$name(index)` with `$`/`[`/`\` substitution in the index).
        let elem = match elem {
            Some(k) if k.iter().any(|&c| matches!(c, b'$' | b'[' | b'\\')) => {
                match self
                    .interp
                    .do_subst(&k, crate::subst::SubstFlags::default())
                {
                    Ok(v) => Some(v),
                    // The index's `[cmd]` completed with a non-OK code — carry it
                    // (error, or `return`/`break`/`continue`) out of the whole
                    // expression, exactly like a `[cmd]` operand.
                    Err(code) => {
                        self.propagated = true;
                        self.propagated_code = code;
                        return Err(crate::expr::ExprError::from_bytes(obj_bytes(
                            self.interp.get_obj_result(),
                        )));
                    }
                }
            }
            other => other,
        };
        let obj = match &elem {
            Some(k) => self.interp.var_get_elem(&base, k),
            None => self.interp.var_get(&base),
        };
        match obj {
            Some(o) => Ok(crate::expr::Owned::retain(o)),
            None => {
                let m = self.interp.read_miss_msg(&base, elem.as_deref());
                Err(crate::expr::ExprError::from_bytes(m))
            }
        }
    }

    fn eval_command(&mut self, script: &str) -> Result<crate::expr::Owned, crate::expr::ExprError> {
        // A `[cmd]` operand that completes with any non-OK code (`error`, or a
        // `return`/`break`/`continue`) propagates that code out of the whole
        // expression. C does this implicitly: the bytecode for the substitution
        // returns the code, which unwinds the `expr`-bearing command. For an
        // error the interp result already holds the message + `::errorInfo`; for
        // the others it holds the substitution's result value.
        let code = self.interp.eval_str(script.as_bytes());
        if code != Code::Ok {
            self.propagated = true;
            self.propagated_code = code;
            return Err(crate::expr::ExprError::from_bytes(obj_bytes(
                self.interp.get_obj_result(),
            )));
        }
        Ok(crate::expr::Owned::retain(self.interp.get_obj_result()))
    }

    fn subst_string(&mut self, inner: &str) -> Result<crate::expr::Owned, crate::expr::ExprError> {
        // A `"…"` operand substitutes like a double-quoted word ($var/[cmd]/\).
        match self
            .interp
            .do_subst(inner.as_bytes(), crate::subst::SubstFlags::default())
        {
            Ok(v) => Ok(crate::expr::Owned::fresh(crate::obj::new_string_bytes(&v))),
            // A `"…"` operand whose `[cmd]` completed non-OK carries that code
            // (error, or `return`/`break`/`continue`) out of the expression.
            Err(code) => {
                self.propagated = true;
                self.propagated_code = code;
                Err(crate::expr::ExprError::from_bytes(obj_bytes(
                    self.interp.get_obj_result(),
                )))
            }
        }
    }

    fn call_function(
        &mut self,
        name: &str,
        args: &[crate::expr::Owned],
    ) -> Result<crate::expr::Owned, crate::expr::ExprError> {
        // Route through the command table so an overridden/renamed
        // `::tcl::mathfunc::NAME` wins (A3); the default builtins forward to the
        // shared dispatch. Args are passed as live objects (object-preserving).
        let arg_ptrs: Vec<*mut TclObj> = args.iter().map(crate::expr::Owned::as_ptr).collect();
        if self.interp.eval_math_call(name.as_bytes(), &arg_ptrs) == Code::Error {
            // A math-function error (e.g. `sqrt(-1)` domain error) is logged at
            // the `expr` command, not as an inner frame — so it is *not*
            // propagated; `expr` raises it as its own (`while executing`). Carry
            // the math function's `-errorcode` (TCL WRONGARGS / ARITH DOMAIN) so
            // `expr`'s re-raise preserves it.
            let msg = obj_bytes(self.interp.get_obj_result());
            let code = self.interp.error_code();
            return Err(crate::expr::ExprError::from_parts(msg, code));
        }
        Ok(crate::expr::Owned::retain(self.interp.get_obj_result()))
    }
}

/// `expr arg ?arg ...?` — concatenate the args (space-separated), parse as a Tcl
/// expression, and evaluate it over the numeric tower
/// ([shared walk](tcl_syntax::expr::eval)).
#[cfg(have_tommath)]
fn expr_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args(b"expr arg ?arg ...?");
    }
    let mut text = Vec::new();
    for (i, &a) in argv[1..].iter().enumerate() {
        if i > 0 {
            text.push(b' ');
        }
        text.extend_from_slice(&obj_bytes(a));
    }
    let Ok(src) = core::str::from_utf8(&text) else {
        return interp.set_error(b"expr operand is not valid UTF-8");
    };
    let node = tcl_syntax::expr::parse_expr(src, None);
    let mut ctx = InterpExprCtx {
        interp: &mut *interp,
        propagated: false,
        propagated_code: Code::Error,
    };
    let result = crate::expr::eval_expr(&node, &mut ctx);
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
        Err(e) => match e.code {
            Some(c) => interp.error_with_code(&e.msg, &c),
            None => interp.set_error(&e.msg),
        },
    }
}

/// Evaluate `src` as a Tcl expression, returning the result object (owned — the
/// caller holds a `+1` and must release it) or the error `Code` (interp result =
/// message). Used where an argument "may be a valid expression" — e.g. `lseq`'s
/// numeric arguments (`SequenceIdentifyArgument`).
#[cfg(have_tommath)]
pub(crate) fn eval_expr_obj(interp: &mut Interp, src: &[u8]) -> Result<*mut TclObj, Code> {
    let Ok(s) = core::str::from_utf8(src) else {
        return Err(interp.set_error(b"expr operand is not valid UTF-8"));
    };
    let node = tcl_syntax::expr::parse_expr(s, None);
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
        Err(e) => Err(match e.code {
            Some(c) => interp.error_with_code(&e.msg, &c),
            None => interp.set_error(&e.msg),
        }),
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
    let src = obj_bytes(cond);
    let Ok(s) = core::str::from_utf8(&src) else {
        return Err(interp.set_error(b"expr operand is not valid UTF-8"));
    };
    let saved = match interp.arg_location(cond) {
        Some((_, line)) => interp.push_cond_line_base(line),
        None => None,
    };
    let node = tcl_syntax::expr::parse_expr(s, None);
    let mut ctx = InterpExprCtx {
        interp: &mut *interp,
        propagated: false,
        propagated_code: Code::Error,
    };
    let result = crate::expr::eval_expr(&node, &mut ctx);
    let propagated = ctx.propagated;
    let propagated_code = ctx.propagated_code;
    if let Some(old) = saved {
        interp.restore_line_base(old);
    }
    match result {
        Ok(r) => crate::expr::to_bool(r.as_ptr()).map_err(|e| interp.set_error(&e.msg)),
        // A propagated sub-eval code unwinds the condition: an error keeps its
        // trace (no condition `expr` frame); a `return`/`break`/`continue` from a
        // `[cmd]` substitution carries that code out of the loop/`if`.
        Err(_) if propagated => Err(propagated_code),
        Err(e) => Err(match e.code {
            Some(c) => interp.error_with_code(&e.msg, &c),
            None => interp.set_error(&e.msg),
        }),
    }
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
            "eval {:?} → {:?}",
            String::from_utf8_lossy(src),
            String::from_utf8_lossy(&i.result_bytes())
        );
        i.result_bytes()
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
