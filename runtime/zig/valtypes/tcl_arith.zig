// Float-aware arithmetic for Tcl expr evaluation.
//
// Each function takes two TclObj i32 pointers and returns a TclObj i32.
// If either operand is a float (TYPE_FLOAT or a string containing '.'/e),
// the result is TYPE_FLOAT; otherwise TYPE_INT.
//
// Integer semantics are preserved: ``tcl_arith_div(7, 2)`` returns 3
// (integer division), while ``tcl_arith_div(7.0, 2)`` returns 3.5.
//
// **Divide-by-zero / mod-by-zero raise a Tcl error** (PR #237 second-
// pass review).  Real Tcl raises ``divide by zero`` with errorCode
// ``ARITH DIVZERO {divide by zero}``; we mirror that via
// ``stubs.raise``.  Earlier versions silently returned 0 to keep the
// tcllib counter::init path running, but that had the unacceptable
// cost of every other legitimate divide-by-zero in user code silently
// producing 0 — exactly the surprise that breaks ports.  Counter
// tests that depended on the silent-zero behaviour need their own
// initialisation-order fix downstream rather than the arithmetic
// layer covering for them.

const std = @import("std");
const obj = @import("tcl_obj.zig");
const stubs = @import("../stubs/tcl_stubs.zig");

const TYPE_INT   = obj.TYPE_INT;
const TYPE_FLOAT = obj.TYPE_FLOAT;
const TYPE_STRING = obj.TYPE_STRING;

fn is_float(o: i32) bool {
    if (o == 0) return false;
    const tag = obj.obj_type(o);
    if (tag == TYPE_FLOAT) return true;
    if (tag == TYPE_STRING) {
        const s = obj.obj_ensure_string(o);
        if (s.len == 0) return false;
        const p: [*]const u8 = @ptrFromInt(s.ptr);
        for (0..s.len) |i| {
            const c = p[i];
            if (c == '.' or c == 'e' or c == 'E') return true;
        }
    }
    return false;
}

pub export fn tcl_arith_add(a: i32, b: i32) i32 {
    if (is_float(a) or is_float(b))
        return obj.obj_new_float(obj.obj_get_float(a) + obj.obj_get_float(b));
    return obj.obj_new_int(obj.obj_get_int(a) +% obj.obj_get_int(b));
}

pub export fn tcl_arith_sub(a: i32, b: i32) i32 {
    if (is_float(a) or is_float(b))
        return obj.obj_new_float(obj.obj_get_float(a) - obj.obj_get_float(b));
    return obj.obj_new_int(obj.obj_get_int(a) -% obj.obj_get_int(b));
}

pub export fn tcl_arith_mul(a: i32, b: i32) i32 {
    if (is_float(a) or is_float(b))
        return obj.obj_new_float(obj.obj_get_float(a) * obj.obj_get_float(b));
    return obj.obj_new_int(obj.obj_get_int(a) *% obj.obj_get_int(b));
}

pub export fn tcl_arith_div(a: i32, b: i32) i32 {
    if (is_float(a) or is_float(b)) {
        const bf = obj.obj_get_float(b);
        if (bf == 0.0) {
            stubs.raise("divide by zero");
            return obj.obj_new_int(0);
        }
        return obj.obj_new_float(obj.obj_get_float(a) / bf);
    }
    const bi = obj.obj_get_int(b);
    if (bi == 0) {
        stubs.raise("divide by zero");
        return obj.obj_new_int(0);
    }
    return obj.obj_new_int(@divTrunc(obj.obj_get_int(a), bi));
}

pub export fn tcl_arith_mod(a: i32, b: i32) i32 {
    if (is_float(a) or is_float(b)) {
        const bf = obj.obj_get_float(b);
        if (bf == 0.0) {
            stubs.raise("divide by zero");
            return obj.obj_new_int(0);
        }
        return obj.obj_new_float(@rem(obj.obj_get_float(a), bf));
    }
    const bi = obj.obj_get_int(b);
    if (bi == 0) {
        stubs.raise("divide by zero");
        return obj.obj_new_int(0);
    }
    return obj.obj_new_int(@rem(obj.obj_get_int(a), bi));
}

/// double(x) — coerce to float.  Used in ``expr {$n / double($d)}``.
pub export fn tcl_math_double(a: i32) i32 {
    if (a == 0) return obj.obj_new_float(0.0);
    if (obj.obj_type(a) == TYPE_FLOAT) return a;
    return obj.obj_new_float(obj.obj_get_float(a));
}

/// int(x) — truncate to integer.
pub export fn tcl_math_int(a: i32) i32 {
    if (a == 0) return obj.obj_new_int(0);
    return obj.obj_new_int(obj.obj_get_int(a));
}

/// round(x) — round to nearest integer (returns float TclObj or int).
pub export fn tcl_math_round(a: i32) i32 {
    const f = obj.obj_get_float(a);
    return obj.obj_new_int(@intFromFloat(@round(f)));
}

/// log(x) — natural logarithm.
pub export fn tcl_math_log(a: i32) i32 {
    const f = obj.obj_get_float(a);
    if (f <= 0.0) return obj.obj_new_float(0.0);
    return obj.obj_new_float(@log(f));
}

/// sqrt(x) — square root.
pub export fn tcl_math_sqrt(a: i32) i32 {
    const f = obj.obj_get_float(a);
    if (f < 0.0) return obj.obj_new_float(0.0);
    return obj.obj_new_float(@sqrt(f));
}

/// exp(x) — e^x.
pub export fn tcl_math_exp(a: i32) i32 {
    return obj.obj_new_float(@exp(obj.obj_get_float(a)));
}

/// log10(x) — base-10 logarithm.
pub export fn tcl_math_log10(a: i32) i32 {
    const f = obj.obj_get_float(a);
    if (f <= 0.0) return obj.obj_new_float(0.0);
    return obj.obj_new_float(@log10(f));
}

/// sin(x).
pub export fn tcl_math_sin(a: i32) i32 {
    return obj.obj_new_float(@sin(obj.obj_get_float(a)));
}

/// cos(x).
pub export fn tcl_math_cos(a: i32) i32 {
    return obj.obj_new_float(@cos(obj.obj_get_float(a)));
}

/// fabs(x) — absolute value as float.
pub export fn tcl_math_fabs(a: i32) i32 {
    return obj.obj_new_float(@abs(obj.obj_get_float(a)));
}
