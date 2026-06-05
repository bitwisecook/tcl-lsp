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
    crate::cmd_list::install(interp);
    crate::cmd_dict::install(interp);
}

fn wrong_args(interp: &mut Interp, usage: &[u8]) -> Code {
    let mut msg = b"wrong # args: should be \"".to_vec();
    msg.extend_from_slice(usage);
    msg.push(b'"');
    interp.set_error(&msg)
}

fn var_error(interp: &mut Interp, name: &[u8], e: VarError) -> Code {
    let verb = match e {
        VarError::IsArray => &b"\": variable is array"[..],
        VarError::IsScalar => &b"\": variable isn't array"[..],
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
                Some(k) => interp.frames.get_elem(&base, k),
                None => interp.frames.get(&base),
            };
            match val {
                Some(o) => {
                    interp.set_result(o);
                    Code::Ok
                }
                None => {
                    let mut msg = b"can't read \"".to_vec();
                    msg.extend_from_slice(&name);
                    msg.extend_from_slice(b"\": no such variable");
                    interp.set_error(&msg)
                }
            }
        }
        3 => {
            let name = obj_bytes(argv[1]);
            let (base, elem) = split_array_ref(&name);
            let value = argv[2];
            let stored = match &elem {
                Some(k) => interp.frames.set_elem(&base, k, value),
                None => interp.frames.set(&base, value),
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

/// `incr varName ?increment?` — add (default 1), storing and returning the sum.
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

    let sum = cur.wrapping_add(amount); // bignum promotion on overflow is T1.5
    let obj = obj::new_wide_int_obj(sum); // rc 0
    let stored = match &elem {
        Some(k) => interp.frames.set_elem(&base, k, obj),
        None => interp.frames.set(&base, obj),
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

fn read_cell(interp: &Interp, base: &[u8], elem: &Option<Vec<u8>>) -> Option<Vec<u8>> {
    let obj = match elem {
        Some(k) => interp.frames.get_elem(base, k),
        None => interp.frames.get(base),
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
            Some(k) => interp.frames.unset_elem(&base, k),
            None => interp.frames.unset(&base),
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

/// Minimal Tcl integer parse: optional surrounding whitespace, optional sign,
/// decimal or `0x` hex. (Full `Tcl_GetIntFromObj` — octal/binary, bignum,
/// underscores — arrives with the numeric value type in T1.5.)
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
