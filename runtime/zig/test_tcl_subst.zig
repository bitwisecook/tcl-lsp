// Unit tests for ``parse/tcl_subst.zig`` — the single-pass
// "pieces" engine behind the ``subst`` command and the
// interpreter's word expander.  Wave 4 of the WASM-runtime test
// harness (#267).
//
// ``subst_flagged(wptr, wlen, do_vars, do_cmds, do_bs)`` walks
// the byte span pointed at by *wptr* / *wlen*, expanding each
// substitution kind whose flag is enabled and concatenating the
// resulting pieces in one final memcpy.  The flag tuple matches
// the ``-novariables`` / ``-nocommands`` / ``-nobackslashes``
// switches.
//
// Most paths are exercised through the ``runtime_test_fixture``
// (#266): variable substitution needs a populated frame for
// ``frames.var_resolve`` to land on, and the ``[cmd]`` path
// drives ``interp.eval_script`` which itself needs a frame +
// namespace.  The pure-text and pure-backslash paths run without
// the fixture.

const std = @import("std");
const testing = std.testing;

const obj = @import("valtypes/tcl_obj.zig");
const subst = @import("parse/tcl_subst.zig");
const fixture = @import("runtime_test_fixture.zig");

// ---- helpers --------------------------------------------------------

fn bytes(o: i32) []const u8 {
    const r = obj.obj_ensure_string(o);
    if (r.len == 0) return &.{};
    const p: [*]const u8 = @ptrFromInt(r.ptr);
    return p[0..r.len];
}

/// Run ``subst_flagged`` on a string literal with all three
/// substitution kinds enabled (the ``subst`` command's default).
fn substAll(src: []const u8) []const u8 {
    const o = subst.subst_flagged(
        @intCast(@intFromPtr(src.ptr)),
        @intCast(src.len),
        true,
        true,
        true,
    );
    return bytes(o);
}

/// Run ``subst_flagged`` with explicit flag selection.
fn substWith(src: []const u8, do_vars: bool, do_cmds: bool, do_bs: bool) []const u8 {
    const o = subst.subst_flagged(
        @intCast(@intFromPtr(src.ptr)),
        @intCast(src.len),
        do_vars,
        do_cmds,
        do_bs,
    );
    return bytes(o);
}

// ---- pure-text fast path -------------------------------------------

test "subst_flagged — empty input returns empty result" {
    try testing.expectEqualStrings("", substAll(""));
}

test "subst_flagged — text without $ / [ / \\ passes through verbatim" {
    // The implementation has a fast path that skips the pieces
    // buffer when none of the three trigger bytes is present.
    try testing.expectEqualStrings("hello world", substAll("hello world"));
    try testing.expectEqualStrings("a/b-c", substAll("a/b-c"));
    try testing.expectEqualStrings("()", substAll("()"));
}

test "subst_flagged — backslash with do_bs=false also takes fast path" {
    // ``do_bs=false`` means the '\\' bytes are not triggers, so the
    // string contains only literal characters that should pass
    // through unchanged.
    try testing.expectEqualStrings(
        "a\\nb",
        substWith("a\\nb", false, false, false),
    );
}

// ---- backslash escapes (no fixture needed) -------------------------

test "subst_flagged — \\n / \\t / \\\\ escape conversions" {
    // The escape table is owned by ``obj.consume_bs_escape``;
    // these spot-checks confirm subst_flagged forwards each byte
    // sequence through to the conversion.
    try testing.expectEqualStrings("a\nb", substAll("a\\nb"));
    try testing.expectEqualStrings("a\tb", substAll("a\\tb"));
    try testing.expectEqualStrings("a\\b", substAll("a\\\\b"));
}

test "subst_flagged — do_bs=false leaves backslashes literal" {
    // ``-nobackslashes`` flag: the '\\' triggers the literal-run
    // accumulator instead of the escape branch.
    try testing.expectEqualStrings(
        "a\\nb",
        substWith("a\\nb", true, true, false),
    );
}

// ---- $var substitution (needs with_interp) -------------------------

const VarCtx = struct {
    var observed: []const u8 = &.{};
    fn body_dollar_name() void {
        fixture.frame.set("name", obj.obj_new_int(42));
        observed = substAll("value=$name");
    }
};

test "subst_flagged — $var resolves via frame locals" {
    VarCtx.observed = &.{};
    fixture.with_interp(&VarCtx.body_dollar_name);
    try testing.expectEqualStrings("value=42", VarCtx.observed);
}

const VarBracedCtx = struct {
    var observed: []const u8 = &.{};
    fn body_braced_name() void {
        // ``${name}`` form lets the variable name include
        // characters that aren't part of the bareword set.
        const v_str = "world";
        fixture.frame.set("greet", obj.obj_new_string(
            @intCast(@intFromPtr(v_str.ptr)),
            @intCast(v_str.len),
        ));
        observed = substAll("hello ${greet}!");
    }
};

test "subst_flagged — ${name} braced variable" {
    VarBracedCtx.observed = &.{};
    fixture.with_interp(&VarBracedCtx.body_braced_name);
    try testing.expectEqualStrings("hello world!", VarBracedCtx.observed);
}

const VarMissingCtx = struct {
    var observed: []const u8 = &.{};
    fn body_missing() void {
        // No ``frame.set`` for ``unset_var`` — ``var_resolve``
        // returns 0 and subst_flagged drops the slot (matching
        // the reference Tcl behaviour of leaving the unmatched
        // text empty rather than raising mid-substitution).
        observed = substAll("value=$unset_var!");
    }
};

test "subst_flagged — unresolved $var contributes empty bytes" {
    VarMissingCtx.observed = &.{};
    fixture.with_interp(&VarMissingCtx.body_missing);
    // The literal ``value=`` and trailing ``!`` pass through; the
    // unresolved ``$unset_var`` slot drops out without raising.
    try testing.expectEqualStrings("value=!", VarMissingCtx.observed);
}

const NoVarsCtx = struct {
    var observed: []const u8 = &.{};
    fn body_no_vars() void {
        fixture.frame.set("x", obj.obj_new_int(99));
        observed = substWith("a$x b", false, false, true);
    }
};

test "subst_flagged — do_vars=false leaves $name literal" {
    // ``-novariables`` flag: ``$x`` is bytes, not a substitution.
    NoVarsCtx.observed = &.{};
    fixture.with_interp(&NoVarsCtx.body_no_vars);
    try testing.expectEqualStrings("a$x b", NoVarsCtx.observed);
}

// ---- mixed substitutions -------------------------------------------

const MixedCtx = struct {
    var observed: []const u8 = &.{};
    fn body_mixed() void {
        fixture.frame.set("n", obj.obj_new_int(7));
        // ``\\t`` → tab, ``$n`` → 7 — the pieces buffer must
        // assemble both contributions in source order.
        observed = substAll("count=\\t$n");
    }
};

test "subst_flagged — mixed $var + backslash escape" {
    MixedCtx.observed = &.{};
    fixture.with_interp(&MixedCtx.body_mixed);
    try testing.expectEqualStrings("count=\t7", MixedCtx.observed);
}

const RepeatVarCtx = struct {
    var observed: []const u8 = &.{};
    fn body_repeat_var() void {
        fixture.frame.set("v", obj.obj_new_int(3));
        // The same variable referenced twice — ``var_resolve``
        // must return a stable value across both lookups.
        observed = substAll("$v-$v");
    }
};

test "subst_flagged — same $var referenced multiple times" {
    RepeatVarCtx.observed = &.{};
    fixture.with_interp(&RepeatVarCtx.body_repeat_var);
    try testing.expectEqualStrings("3-3", RepeatVarCtx.observed);
}

// ---- bracket / command substitution --------------------------------

test "subst_flagged — empty bracket body produces empty piece" {
    // ``[]`` is a degenerate command sub: ``eval_script`` with
    // length 0 returns 0, which subst_flagged treats as "no
    // bytes to push".  This path is the one [cmd] test we can
    // run without registered commands — full ``[set x …]``
    // coverage lives in the integration suite, where the
    // command table is initialised.
    try testing.expectEqualStrings("a", substAll("a[]"));
    try testing.expectEqualStrings("", substAll("[]"));
}

test "subst_flagged — do_cmds=false leaves [...] literal" {
    // ``-nocommands`` flag: ``[`` is just a byte, no eval.
    try testing.expectEqualStrings(
        "a[set x 5]b",
        substWith("a[set x 5]b", false, false, false),
    );
}
