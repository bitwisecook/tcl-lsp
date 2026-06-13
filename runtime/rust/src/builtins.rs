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
    interp.register_builtin(b"return", ret);
    interp.register_builtin(b"unset", unset);
    interp.register_builtin(b"subst", subst_cmd);
    crate::cmd_scan::install(interp);
    crate::cmd_format::install(interp);
    crate::cmd_binary::install(interp);
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
    crate::cmd_trace::install(interp);
    // `regexp`/`regsub` need the linked-in Tcl regex engine (the C ARE engine).
    #[cfg(have_regex)]
    crate::cmd_regex::install(interp);
    // TclOO last: its `variable`/`self`/`my`/`next` intentionally override the
    // base `variable` (OO-aware inside `oo::define`, forwarding otherwise).
    crate::cmd_oo::install(interp);
}

fn wrong_args(interp: &mut Interp, usage: &[u8]) -> Code {
    let mut msg = b"wrong # args: should be \"".to_vec();
    msg.extend_from_slice(usage);
    msg.push(b'"');
    interp.set_error(&msg)
}

pub(crate) fn var_error(interp: &mut Interp, name: &[u8], e: VarError) -> Code {
    let verb = match e {
        VarError::IsArray => &b"\": variable is array"[..],
        VarError::IsScalar => &b"\": variable isn't array"[..],
        VarError::NoSuchNamespace => &b"\": parent namespace doesn't exist"[..],
    };
    let mut msg = b"can't set \"".to_vec();
    msg.extend_from_slice(name);
    msg.extend_from_slice(verb);
    interp.set_error(&msg)
}

// -- set -------------------------------------------------------------------

/// `set varName ?value?` — write (returns the value) or read (returns it).
fn set(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    match argv.len() {
        2 => {
            let name = obj_bytes(argv[1]);
            let (base, elem) = split_array_ref(&name);
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
        _ => wrong_args(interp, b"set varName ?newValue?"),
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
        return wrong_args(interp, b"incr varName ?increment?");
    }
    let name = obj_bytes(argv[1]);
    let (base, elem) = split_array_ref(&name);

    // Current cell value (borrowed) or a fresh 0 for an unset variable; the
    // fresh-0 is `rc 0` and freed below whether or not it's used.
    let existing = match &elem {
        Some(k) => interp.var_get_elem(&base, k),
        None => interp.var_get(&base),
    };
    let zero = obj::new_wide_int_obj(0);
    let cur = existing.unwrap_or(zero);
    if !crate::bignum::is_integer(cur) {
        let bytes = obj_bytes(cur);
        drop_fresh(zero);
        return not_integer(interp, &bytes);
    }

    // Increment object: `argv[2]` (borrowed) or a fresh 1 (freed below).
    let one = obj::new_wide_int_obj(1);
    let amount = if argv.len() == 3 { argv[2] } else { one };
    if !crate::bignum::is_integer(amount) {
        let bytes = obj_bytes(amount);
        drop_fresh(zero);
        drop_fresh(one);
        return not_integer(interp, &bytes);
    }

    // Both integers → integer sum (wide fast path; bignum on overflow; the
    // result demotes back to a wide when it fits).
    let sum = match crate::bignum::add(cur, amount) {
        Ok(s) => s, // rc 0
        Err(_) => {
            drop_fresh(zero);
            drop_fresh(one);
            return interp.set_error(b"out of memory");
        }
    };
    drop_fresh(zero); // `cur` no longer needed
    drop_fresh(one); // `amount` no longer needed

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
        return wrong_args(interp, b"incr varName ?increment?");
    }
    let name = obj_bytes(argv[1]);
    let (base, elem) = split_array_ref(&name);

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

fn not_integer(interp: &mut Interp, bytes: &[u8]) -> Code {
    let mut msg = b"expected integer but got \"".to_vec();
    msg.extend_from_slice(bytes);
    msg.push(b'"');
    interp.set_error(&msg)
}

// -- return ----------------------------------------------------------------

/// Map a `-code` word (`ok`/`error`/`return`/`break`/`continue` or 0–4) to a
/// [`Code`]; `None` for an unrecognised spelling.
fn parse_code(b: &[u8]) -> Option<Code> {
    match b {
        b"ok" | b"0" => Some(Code::Ok),
        b"error" | b"1" => Some(Code::Error),
        b"return" | b"2" => Some(Code::Return),
        b"break" | b"3" => Some(Code::Break),
        b"continue" | b"4" => Some(Code::Continue),
        _ => None,
    }
}

/// `return ?-code code? ?-level n? ?-errorcode list? ?-errorinfo info?
/// ?-options dict? ?result?` — complete with `-code` after unwinding `-level`
/// proc/source boundaries (`Tcl_ReturnObjCmd`). A `-options` dict (as produced
/// by `catch`) seeds the options; explicit flags override it.
fn ret(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let mut code = Code::Ok;
    let mut level: usize = 1;
    let mut errorcode: Option<Vec<u8>> = None;
    let mut errorinfo: Option<Vec<u8>> = None;

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
    // The optional trailing result word.
    if i < argv.len() {
        interp.set_result(argv[i]);
    } else {
        interp.set_result_bytes(b"");
    }
    // On an error completion, stamp the error globals (like `error`).
    if code == Code::Error {
        let info = errorinfo.unwrap_or_else(|| obj_bytes(interp.get_obj_result()));
        let ecode = errorcode.unwrap_or_else(|| b"NONE".to_vec());
        set_global(interp, b"::errorInfo", &info);
        set_global(interp, b"::errorCode", &ecode);
    }
    if level == 0 {
        // No unwinding: complete with `code` right here.
        code
    } else {
        interp.set_return_state(level, code);
        Code::Return
    }
}

/// Set a global (`::name`) to `bytes` (for the error globals).
fn set_global(interp: &mut Interp, name: &[u8], bytes: &[u8]) {
    let o = obj::new_string_bytes(bytes);
    if interp.var_set(name, o).is_err() {
        drop_fresh(o);
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
    let mut flags = crate::subst::SubstFlags::default();
    let mut i = 1;
    while i < argv.len() {
        match obj_bytes(argv[i]).as_slice() {
            b"-nobackslashes" => flags.backslashes = false,
            b"-nocommands" => flags.cmds = false,
            b"-novariables" => flags.vars = false,
            _ => break,
        }
        i += 1;
    }
    if i != argv.len() - 1 {
        return wrong_args(interp, USAGE);
    }
    let src = obj_bytes(argv[i]);
    match interp.do_subst(&src, flags) {
        Ok(bytes) => {
            interp.set_result_bytes(&bytes);
            Code::Ok
        }
        Err(code) => code, // the failing sub already set the result
    }
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
    /// Set when an error propagated out of a sub-evaluation (`[cmd]`, a math
    /// function, or a `$arr(idx)` index subst) rather than originating in `expr`
    /// itself. In that case the inner command already built the `::errorInfo`
    /// trace, so `expr` must preserve it (and add no frame of its own) — matching
    /// C, where such an error is logged at the inner command, not at `expr`.
    propagated: bool,
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
                    Err(_) => {
                        self.propagated = true;
                        return Err(crate::expr::ExprError(obj_bytes(
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
                Err(crate::expr::ExprError(m))
            }
        }
    }

    fn eval_command(&mut self, script: &str) -> Result<crate::expr::Owned, crate::expr::ExprError> {
        if self.interp.eval_str(script.as_bytes()) == Code::Error {
            self.propagated = true;
            return Err(crate::expr::ExprError(obj_bytes(
                self.interp.get_obj_result(),
            )));
        }
        Ok(crate::expr::Owned::retain(self.interp.get_obj_result()))
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
            // propagated; `expr` raises it as its own (`while executing`).
            return Err(crate::expr::ExprError(obj_bytes(
                self.interp.get_obj_result(),
            )));
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
        return wrong_args(interp, b"expr arg ?arg ...?");
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
    };
    let result = crate::expr::eval_expr(&node, &mut ctx);
    let propagated = ctx.propagated;
    match result {
        Ok(r) => {
            // `set_result` takes its own `+1`; `r` drops its reference after.
            interp.set_result(r.as_ptr());
            Code::Ok
        }
        // A propagated sub-eval error already set the result + `::errorInfo`
        // trace at the inner command — preserve it (no `expr` frame).
        Err(_) if propagated => Code::Error,
        Err(e) => interp.set_error(&e.0),
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
    };
    let result = crate::expr::eval_expr(&node, &mut ctx);
    let propagated = ctx.propagated;
    match result {
        Ok(r) => Ok(r.into_raw()), // transfer the +1 to the caller
        Err(_) if propagated => Err(Code::Error),
        Err(e) => Err(interp.set_error(&e.0)),
    }
}

/// Evaluate `src` as a Tcl expression and coerce the result to a boolean — the
/// condition evaluator `if`/`while`/`for` share. `Err(code)` carries the
/// completion code (with the interp result already set to the error message).
#[cfg(have_tommath)]
pub(crate) fn eval_bool_expr(interp: &mut Interp, src: &[u8]) -> Result<bool, Code> {
    let Ok(s) = core::str::from_utf8(src) else {
        return Err(interp.set_error(b"expr operand is not valid UTF-8"));
    };
    let node = tcl_syntax::expr::parse_expr(s, None);
    let mut ctx = InterpExprCtx {
        interp: &mut *interp,
        propagated: false,
    };
    let result = crate::expr::eval_expr(&node, &mut ctx);
    let propagated = ctx.propagated;
    match result {
        Ok(r) => crate::expr::to_bool(r.as_ptr()).map_err(|e| interp.set_error(&e.0)),
        // Preserve a propagated sub-eval error's trace (no condition `expr` frame).
        Err(_) if propagated => Err(Code::Error),
        Err(e) => Err(interp.set_error(&e.0)),
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
