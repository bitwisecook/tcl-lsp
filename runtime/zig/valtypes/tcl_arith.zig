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

const TYPE_INT = obj.TYPE_INT;
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
    if (a == 0) {
        // Empty/null arg — match reference Tcl's
        // ``can't use empty string as operand of "double"``.  In
        // practice this only fires when the slot was never written;
        // fall through to a numeric error so callers see a Tcl
        // diagnostic instead of a silent 0.0.
        return raise_double_error(a);
    }
    if (obj.obj_type(a) == TYPE_FLOAT) return a;
    if (obj.obj_type(a) == TYPE_INT or obj.is_immediate(a)) {
        return obj.obj_new_float(@floatFromInt(obj.obj_get_int(a)));
    }
    // String-typed arg — must parse as a number.  Matches reference
    // Tcl's ``expected floating-point number but got "X"``.
    const s = obj.obj_ensure_string(a);
    if (obj.try_parse_float(s.ptr, s.len)) |val| return obj.obj_new_float(val);
    if (obj.try_parse_int(s.ptr, s.len)) |val| return obj.obj_new_float(@floatFromInt(val));
    return raise_double_error(a);
}

fn raise_double_error(a: i32) i32 {
    if (@import("../interp/tcl_catch.zig").error_flag != 0) return obj.obj_new_float(0.0);
    const s = obj.obj_ensure_string(a);
    const prefix: []const u8 = "expected floating-point number but got \"";
    const suffix: []const u8 = "\"";
    const total: u32 = @intCast(prefix.len + s.len + suffix.len);
    const buf_addr: u32 = obj.alloc(total);
    const buf: [*]u8 = @ptrFromInt(buf_addr);
    var off: usize = 0;
    for (prefix) |c| { buf[off] = c; off += 1; }
    if (s.len > 0) {
        const sp: [*]const u8 = @ptrFromInt(s.ptr);
        for (0..s.len) |i| { buf[off] = sp[i]; off += 1; }
    }
    for (suffix) |c| { buf[off] = c; off += 1; }
    const msg = obj.obj_new_string(@bitCast(buf_addr), @bitCast(total));
    @import("../interp/tcl_catch.zig").tcl_cmd_error(msg);
    return obj.obj_new_float(0.0);
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

// ----------------------------------------------------------------------
// Bitwise / shift helpers — issues #260, #261, #262
//
// Tcl 9.0 rejects floating-point operands in bitwise (``&`` ``|`` ``^``
// ``~``) and shift (``<<`` ``>>``) operators with errors of the form:
//
//   cannot use floating-point value "X" as operand of "OP"
//   cannot use floating-point value "X" as left operand of "OP"
//   cannot use floating-point value "X" as right operand of "OP"
//
// Shift counts must additionally be non-negative; a negative count
// raises ``negative shift argument``.  The Python VM enforces both
// rules in ``vm/machine.py::_bitwise_binary``; the WASM expression
// emitter previously inlined ``i64.shl`` / ``i64.and`` / ``i64.or`` /
// ``i64.xor`` directly, which silently truncated floats and accepted
// negative shift counts (WASM masks the count by 63).  These helpers
// are called from ``core/compiler/codegen/wasm/_emitter/_expressions.py``
// to recover the missing domain checks.

/// Build ``cannot use floating-point value "X" as <position> operand of "<op>"``
/// and route it through the Tcl error path.
fn raise_float_in_bitwise(o: i32, op_sym: []const u8, position: []const u8) void {
    // Preserve the first error in a chain: if a prior helper in the
    // same statement (e.g. a missing-variable read on the other
    // operand) already set ``error_flag``, don't overwrite the
    // pending diagnostic with a follow-on ``cannot use floating-
    // point value`` error — match reference Tcl's "first error
    // wins" semantics for a single command.
    if (@import("../interp/tcl_catch.zig").error_flag != 0) return;
    const s = obj.obj_ensure_string(o);
    const prefix: []const u8 = "cannot use floating-point value \"";
    const middle: []const u8 = "\" as ";
    const between: []const u8 = " operand of \"";
    const suffix: []const u8 = "\"";
    const total: u32 = @intCast(prefix.len + s.len + middle.len + position.len + between.len + op_sym.len + suffix.len);
    const buf_addr: u32 = obj.alloc(total);
    const buf: [*]u8 = @ptrFromInt(buf_addr);
    var off: usize = 0;
    for (prefix) |c| {
        buf[off] = c;
        off += 1;
    }
    if (s.len > 0) {
        const sp: [*]const u8 = @ptrFromInt(s.ptr);
        for (0..s.len) |i| {
            buf[off] = sp[i];
            off += 1;
        }
    }
    for (middle) |c| {
        buf[off] = c;
        off += 1;
    }
    for (position) |c| {
        buf[off] = c;
        off += 1;
    }
    for (between) |c| {
        buf[off] = c;
        off += 1;
    }
    for (op_sym) |c| {
        buf[off] = c;
        off += 1;
    }
    for (suffix) |c| {
        buf[off] = c;
        off += 1;
    }
    // Issue #317: ``obj_new_string_take`` so the error TclObj owns
    // the message buffer; the older ``obj_new_string`` left
    // ``OBJ_STR_CAP = 0`` and the buf was leaked on release inside
    // a ``catch``.  Outside of ``catch`` ``tcl_cmd_error`` traps
    // the process so the leak doesn't accumulate, but io.test
    // exercises the catched path heavily through tcltest.
    const msg = obj.obj_new_string_take(buf_addr, total, total);
    @import("../interp/tcl_catch.zig").tcl_cmd_error(msg);
}

/// Build ``cannot use floating-point value "X" as operand of "<op>"``
/// (unary form — no left/right qualifier).
fn raise_float_in_unary_bitwise(o: i32, op_sym: []const u8) void {
    // Preserve the first error in a chain — see the binary form for
    // the rationale.
    if (@import("../interp/tcl_catch.zig").error_flag != 0) return;
    const s = obj.obj_ensure_string(o);
    const prefix: []const u8 = "cannot use floating-point value \"";
    const middle: []const u8 = "\" as operand of \"";
    const suffix: []const u8 = "\"";
    const total: u32 = @intCast(prefix.len + s.len + middle.len + op_sym.len + suffix.len);
    const buf_addr: u32 = obj.alloc(total);
    const buf: [*]u8 = @ptrFromInt(buf_addr);
    var off: usize = 0;
    for (prefix) |c| {
        buf[off] = c;
        off += 1;
    }
    if (s.len > 0) {
        const sp: [*]const u8 = @ptrFromInt(s.ptr);
        for (0..s.len) |i| {
            buf[off] = sp[i];
            off += 1;
        }
    }
    for (middle) |c| {
        buf[off] = c;
        off += 1;
    }
    for (op_sym) |c| {
        buf[off] = c;
        off += 1;
    }
    for (suffix) |c| {
        buf[off] = c;
        off += 1;
    }
    // Issue #317: ``obj_new_string_take`` so the error TclObj owns
    // the message buffer; the older ``obj_new_string`` left
    // ``OBJ_STR_CAP = 0`` and the buf was leaked on release inside
    // a ``catch``.  Outside of ``catch`` ``tcl_cmd_error`` traps
    // the process so the leak doesn't accumulate, but io.test
    // exercises the catched path heavily through tcltest.
    const msg = obj.obj_new_string_take(buf_addr, total, total);
    @import("../interp/tcl_catch.zig").tcl_cmd_error(msg);
}

fn check_int_binary(a: i32, b: i32, op_sym: []const u8) bool {
    if (is_float(a)) {
        raise_float_in_bitwise(a, op_sym, "left");
        return false;
    }
    if (is_float(b)) {
        raise_float_in_bitwise(b, op_sym, "right");
        return false;
    }
    return true;
}

pub export fn tcl_arith_lshift(a: i32, b: i32) i32 {
    if (!check_int_binary(a, b, "<<")) return obj.obj_new_int(0);
    const bi = obj.obj_get_int(b);
    if (bi < 0) {
        stubs.raise("negative shift argument");
        return obj.obj_new_int(0);
    }
    const ai = obj.obj_get_int(a);
    if (bi >= 64) {
        // Tcl 9 raises ``integer value too large to represent`` for
        // very large shift counts on non-zero values.  For simplicity
        // we collapse to 0 for ``a == 0`` (mirrors ``0 << N == 0``)
        // and rely on i64 wrap semantics for non-zero ``a`` (matches
        // the previous inline ``i64.shl`` behaviour up to the count
        // mask).  Tcl-style ``integer too large`` is out of scope.
        if (ai == 0) return obj.obj_new_int(0);
        // Use a safe default: shift by (bi & 63) — this matches what
        // ``i64.shl`` did before.  Avoids a hard error for callers
        // that previously relied on the implicit mask.
        const shift: u6 = @intCast(@as(u64, @bitCast(bi)) & 63);
        return obj.obj_new_int(ai << shift);
    }
    const shift: u6 = @intCast(bi);
    return obj.obj_new_int(ai << shift);
}

pub export fn tcl_arith_rshift(a: i32, b: i32) i32 {
    if (!check_int_binary(a, b, ">>")) return obj.obj_new_int(0);
    const bi = obj.obj_get_int(b);
    if (bi < 0) {
        stubs.raise("negative shift argument");
        return obj.obj_new_int(0);
    }
    const ai = obj.obj_get_int(a);
    if (bi >= 64) {
        // Arithmetic shift by 64+ produces 0 for non-negative ``a``
        // and -1 for negative ``a``.  Match that exactly rather than
        // letting WASM mask the count.
        return obj.obj_new_int(if (ai < 0) -1 else 0);
    }
    const shift: u6 = @intCast(bi);
    return obj.obj_new_int(ai >> shift);
}

pub export fn tcl_arith_band(a: i32, b: i32) i32 {
    if (!check_int_binary(a, b, "&")) return obj.obj_new_int(0);
    return obj.obj_new_int(obj.obj_get_int(a) & obj.obj_get_int(b));
}

pub export fn tcl_arith_bor(a: i32, b: i32) i32 {
    if (!check_int_binary(a, b, "|")) return obj.obj_new_int(0);
    return obj.obj_new_int(obj.obj_get_int(a) | obj.obj_get_int(b));
}

pub export fn tcl_arith_bxor(a: i32, b: i32) i32 {
    if (!check_int_binary(a, b, "^")) return obj.obj_new_int(0);
    return obj.obj_new_int(obj.obj_get_int(a) ^ obj.obj_get_int(b));
}

pub export fn tcl_arith_bnot(a: i32) i32 {
    if (is_float(a)) {
        raise_float_in_unary_bitwise(a, "~");
        return obj.obj_new_int(0);
    }
    return obj.obj_new_int(~obj.obj_get_int(a));
}

/// Float-preserving unary negation.  Mirrors ``tcl_arith_sub(0, x)``:
/// int → int, any float → float.  Used by the WASM expression emitter
/// in object-context paths (``_emit_expr_obj``) so that ``-$x`` of a
/// float-string variable keeps its TYPE_FLOAT tag end-to-end and the
/// bitwise / shift domain checks (``tcl_arith_lshift`` /
/// ``tcl_arith_bnot``) observe the float on the operand chain.  Without
/// this helper the inline ``0 - x`` i64 path silently truncates to int
/// and the float check is bypassed (Codex review on PR #287).
pub export fn tcl_arith_neg(a: i32) i32 {
    if (is_float(a)) return obj.obj_new_float(-obj.obj_get_float(a));
    return obj.obj_new_int(-%obj.obj_get_int(a));
}
