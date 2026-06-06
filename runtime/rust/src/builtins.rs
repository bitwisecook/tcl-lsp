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
                    // A scalar read of an array name is "variable is array", not
                    // "no such variable" (the array-vs-scalar distinction Tcl
                    // reports — matches `var_error(IsArray)`).
                    let mut msg = b"can't read \"".to_vec();
                    msg.extend_from_slice(&name);
                    if elem.is_none() && interp.var_is_array(&base) {
                        msg.extend_from_slice(b"\": variable is array");
                    } else {
                        msg.extend_from_slice(b"\": no such variable");
                    }
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

/// `return ?value?` — set the result and complete with [`Code::Return`].
/// (The `-code`/`-options` forms arrive with full `return` support in T1.5.)
fn ret(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() >= 2 {
        interp.set_result(argv[1]);
    } else {
        interp.set_result_bytes(b"");
    }
    Code::Return
}

// -- unset -----------------------------------------------------------------

/// `unset varName ...` — remove variables (scalars or array elements).
fn unset(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return wrong_args(interp, b"unset ?varName ...?");
    }
    for &a in &argv[1..] {
        let name = obj_bytes(a);
        let (base, elem) = split_array_ref(&name);
        let existed = match &elem {
            Some(k) => interp.var_unset_elem(&base, k),
            None => interp.var_unset(&base),
        };
        if !existed {
            let mut msg = b"can't unset \"".to_vec();
            msg.extend_from_slice(&name);
            msg.extend_from_slice(b"\": no such variable");
            return interp.set_error(&msg);
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
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
}

#[cfg(have_tommath)]
impl crate::expr::ExprCtx for InterpExprCtx<'_> {
    fn read_var(&mut self, name: &str) -> Result<crate::expr::Owned, crate::expr::ExprError> {
        let (base, elem) = split_array_ref(name.as_bytes());
        let obj = match &elem {
            Some(k) => self.interp.var_get_elem(&base, k),
            None => self.interp.var_get(&base),
        };
        match obj {
            Some(o) => Ok(crate::expr::Owned::retain(o)),
            None => {
                let mut m = b"can't read \"".to_vec();
                m.extend_from_slice(name.as_bytes());
                m.extend_from_slice(b"\": no such variable");
                Err(crate::expr::ExprError(m))
            }
        }
    }

    fn eval_command(&mut self, script: &str) -> Result<crate::expr::Owned, crate::expr::ExprError> {
        if self.interp.eval_str(script.as_bytes()) == Code::Error {
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
    let result = {
        let mut ctx = InterpExprCtx {
            interp: &mut *interp,
        };
        crate::expr::eval_expr(&node, &mut ctx)
    };
    match result {
        Ok(r) => {
            // `set_result` takes its own `+1`; `r` drops its reference after.
            interp.set_result(r.as_ptr());
            Code::Ok
        }
        Err(e) => interp.set_error(&e.0),
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
    let result = {
        let mut ctx = InterpExprCtx {
            interp: &mut *interp,
        };
        crate::expr::eval_expr(&node, &mut ctx)
    };
    match result {
        Ok(r) => crate::expr::to_bool(r.as_ptr()).map_err(|e| interp.set_error(&e.0)),
        Err(e) => Err(interp.set_error(&e.0)),
    }
}
