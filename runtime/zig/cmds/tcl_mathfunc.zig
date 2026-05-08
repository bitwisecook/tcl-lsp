// ``::tcl::mathfunc::*`` namespace — every math function in expr is
// also reachable as a normal Tcl command in this namespace.
// ``::tcl::mathfunc::abs -0`` and ``expr {abs(-0)}`` produce the same
// value via the same underlying helper.
//
// The expr-side dispatcher in ``interp/tcl_expr_eval.zig::dispatch_math_func``
// is the canonical implementation; this file is a thin command-form
// adapter that re-uses it.

const std = @import("std");
const reg = @import("../dispatch/tcl_cmd_registry.zig");
const result_mod = @import("../interp/tcl_result.zig");
const obj = @import("../valtypes/tcl_obj.zig");
const stubs = @import("../stubs/tcl_stubs.zig");
const arith = @import("../valtypes/tcl_arith.zig");

/// Generic math function command.  *args[0]* is the command name
/// (``::tcl::mathfunc::abs`` etc.); the remaining args are the
/// numeric operands.  Some functions are 0-arg (``rand``), most
/// are 1-arg, a few (``pow`` / ``atan2`` / …) are 2-arg.  The
/// handler dispatches by stripping the ``::tcl::mathfunc::``
/// prefix and routing to the matching helper.
fn eval_mathfunc(words: []const i32) result_mod.InterpResult {
    // Always need at least the command name in argv[0].
    if (words.len < 1) {
        stubs.raise("wrong # args");
        return result_mod.from_globals(0);
    }
    const prefix = "::tcl::mathfunc::";
    const cmd_str = obj.obj_ensure_string(words[0]);
    const cmd: [*]const u8 = @ptrFromInt(cmd_str.ptr);
    // Strip the prefix.  ``prefix.len`` is computed from the literal
    // so the count can never drift out of sync with the bytes (was
    // hand-counted as 17 — the right answer, but easy to mis-count).
    if (cmd_str.len <= prefix.len) {
        stubs.raise("invalid mathfunc invocation");
        return result_mod.from_globals(0);
    }
    const cmd_slice = cmd[0..cmd_str.len];
    if (!std.mem.startsWith(u8, cmd_slice, prefix)) {
        stubs.raise("invalid mathfunc invocation");
        return result_mod.from_globals(0);
    }
    const name = cmd_slice[prefix.len..];

    // 0-arg dispatch — only ``rand`` qualifies.  Registered with
    // ``arity_min=0, arity_max=0`` so the runtime arity check has
    // already rejected any spurious args before we get here.
    if (words.len == 1) {
        if (std.mem.eql(u8, name, "rand")) {
            return result_mod.from_globals(arith.tcl_math_rand());
        }
        stubs.raise("wrong # args");
        return result_mod.from_globals(0);
    }

    // Single-arg dispatch.
    if (words.len == 2) {
        if (std.mem.eql(u8, name, "abs") or std.mem.eql(u8, name, "fabs")) {
            return result_mod.from_globals(arith.tcl_math_fabs(words[1]));
        }
        if (std.mem.eql(u8, name, "int") or std.mem.eql(u8, name, "entier")) {
            return result_mod.from_globals(arith.tcl_math_int(words[1]));
        }
        if (std.mem.eql(u8, name, "wide")) {
            return result_mod.from_globals(arith.tcl_math_wide(words[1]));
        }
        if (std.mem.eql(u8, name, "double") or std.mem.eql(u8, name, "float")) {
            return result_mod.from_globals(arith.tcl_math_double(words[1]));
        }
        if (std.mem.eql(u8, name, "round")) return result_mod.from_globals(arith.tcl_math_round(words[1]));
        if (std.mem.eql(u8, name, "log")) return result_mod.from_globals(arith.tcl_math_log(words[1]));
        if (std.mem.eql(u8, name, "log10")) return result_mod.from_globals(arith.tcl_math_log10(words[1]));
        if (std.mem.eql(u8, name, "sqrt")) return result_mod.from_globals(arith.tcl_math_sqrt(words[1]));
        if (std.mem.eql(u8, name, "isqrt")) return result_mod.from_globals(arith.tcl_math_isqrt(words[1]));
        if (std.mem.eql(u8, name, "bool")) return result_mod.from_globals(arith.tcl_math_bool(words[1]));
        if (std.mem.eql(u8, name, "exp")) return result_mod.from_globals(arith.tcl_math_exp(words[1]));
        if (std.mem.eql(u8, name, "sin")) return result_mod.from_globals(arith.tcl_math_sin(words[1]));
        if (std.mem.eql(u8, name, "cos")) return result_mod.from_globals(arith.tcl_math_cos(words[1]));
        if (std.mem.eql(u8, name, "tan")) return result_mod.from_globals(arith.tcl_math_tan(words[1]));
        if (std.mem.eql(u8, name, "asin")) return result_mod.from_globals(arith.tcl_math_asin(words[1]));
        if (std.mem.eql(u8, name, "acos")) return result_mod.from_globals(arith.tcl_math_acos(words[1]));
        if (std.mem.eql(u8, name, "atan")) return result_mod.from_globals(arith.tcl_math_atan(words[1]));
        if (std.mem.eql(u8, name, "sinh")) return result_mod.from_globals(arith.tcl_math_sinh(words[1]));
        if (std.mem.eql(u8, name, "cosh")) return result_mod.from_globals(arith.tcl_math_cosh(words[1]));
        if (std.mem.eql(u8, name, "tanh")) return result_mod.from_globals(arith.tcl_math_tanh(words[1]));
        if (std.mem.eql(u8, name, "floor")) return result_mod.from_globals(arith.tcl_math_floor(words[1]));
        if (std.mem.eql(u8, name, "ceil")) return result_mod.from_globals(arith.tcl_math_ceil(words[1]));
        if (std.mem.eql(u8, name, "isfinite")) return result_mod.from_globals(arith.tcl_math_isfinite(words[1]));
        if (std.mem.eql(u8, name, "isinf")) return result_mod.from_globals(arith.tcl_math_isinf(words[1]));
        if (std.mem.eql(u8, name, "isnan")) return result_mod.from_globals(arith.tcl_math_isnan(words[1]));
        if (std.mem.eql(u8, name, "isnormal")) return result_mod.from_globals(arith.tcl_math_isnormal(words[1]));
        if (std.mem.eql(u8, name, "issubnormal")) return result_mod.from_globals(arith.tcl_math_issubnormal(words[1]));
        if (std.mem.eql(u8, name, "fpclassify")) return result_mod.from_globals(arith.tcl_math_fpclassify(words[1]));
        if (std.mem.eql(u8, name, "srand")) return result_mod.from_globals(arith.tcl_math_srand(words[1]));
    }
    if (words.len == 3) {
        if (std.mem.eql(u8, name, "pow")) return result_mod.from_globals(arith.tcl_arith_pow(words[1], words[2]));
        if (std.mem.eql(u8, name, "atan2")) return result_mod.from_globals(arith.tcl_math_atan2(words[1], words[2]));
        if (std.mem.eql(u8, name, "fmod")) return result_mod.from_globals(arith.tcl_math_fmod(words[1], words[2]));
        if (std.mem.eql(u8, name, "hypot")) return result_mod.from_globals(arith.tcl_math_hypot(words[1], words[2]));
        if (std.mem.eql(u8, name, "isunordered")) return result_mod.from_globals(arith.tcl_math_isunordered(words[1], words[2]));
    }
    // Unknown function name or wrong arity.
    stubs.raise("invalid math function call");
    return result_mod.from_globals(0);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "::tcl::mathfunc::abs", .arity_min = 1, .arity_max = 1, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::fabs", .arity_min = 1, .arity_max = 1, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::int", .arity_min = 1, .arity_max = 1, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::entier", .arity_min = 1, .arity_max = 1, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::wide", .arity_min = 1, .arity_max = 1, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::double", .arity_min = 1, .arity_max = 1, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::float", .arity_min = 1, .arity_max = 1, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::round", .arity_min = 1, .arity_max = 1, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::log", .arity_min = 1, .arity_max = 1, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::log10", .arity_min = 1, .arity_max = 1, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::sqrt", .arity_min = 1, .arity_max = 1, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::isqrt", .arity_min = 1, .arity_max = 1, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::bool", .arity_min = 1, .arity_max = 1, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::exp", .arity_min = 1, .arity_max = 1, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::sin", .arity_min = 1, .arity_max = 1, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::cos", .arity_min = 1, .arity_max = 1, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::tan", .arity_min = 1, .arity_max = 1, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::asin", .arity_min = 1, .arity_max = 1, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::acos", .arity_min = 1, .arity_max = 1, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::atan", .arity_min = 1, .arity_max = 1, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::sinh", .arity_min = 1, .arity_max = 1, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::cosh", .arity_min = 1, .arity_max = 1, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::tanh", .arity_min = 1, .arity_max = 1, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::floor", .arity_min = 1, .arity_max = 1, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::ceil", .arity_min = 1, .arity_max = 1, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::isfinite", .arity_min = 1, .arity_max = 1, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::isinf", .arity_min = 1, .arity_max = 1, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::isnan", .arity_min = 1, .arity_max = 1, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::isnormal", .arity_min = 1, .arity_max = 1, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::issubnormal", .arity_min = 1, .arity_max = 1, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::fpclassify", .arity_min = 1, .arity_max = 1, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::pow", .arity_min = 2, .arity_max = 2, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::atan2", .arity_min = 2, .arity_max = 2, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::fmod", .arity_min = 2, .arity_max = 2, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::hypot", .arity_min = 2, .arity_max = 2, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::isunordered", .arity_min = 2, .arity_max = 2, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::rand", .arity_min = 0, .arity_max = 0, .handler = &eval_mathfunc },
    .{ .name = "::tcl::mathfunc::srand", .arity_min = 1, .arity_max = 1, .handler = &eval_mathfunc },
};
