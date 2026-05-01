// Unit tests for ``valtypes/tcl_regex.zig`` — the Henry-Spencer
// regex engine wrapper used by ``regexp`` / ``regsub``.  Wave 4
// of the WASM-runtime test harness (#267).
//
// Two entry points are covered:
//   * ``tcl_cmd_regexp(pattern, subject)`` — the 2-arg form that
//     returns 1/0 for match / no match.
//   * ``do_regsub(pattern, string, subspec, nocase, all, …)`` —
//     the substitution worker shared between ``regsub`` and the
//     interpreter-side eval handler.
//
// ``eval_regexp_cmd`` (capture-var assignment, ``-inline``,
// ``-indices``) needs a populated frame to call ``frames.var_set``
// on capture targets, so its happy paths are covered by the
// integration suite; the wave-4 surface here exercises the
// underlying compile / execute / substitute machinery.
//
// Error-path coverage uses ``runtime_test_fixture.with_catch``
// (#266) — malformed patterns raise via ``stubs.raise`` and the
// catch fixture lets us assert the exact message without trapping
// the WASM module.

const std = @import("std");
const testing = std.testing;

const obj = @import("valtypes/tcl_obj.zig");
const re = @import("valtypes/tcl_regex.zig");
const fixture = @import("runtime_test_fixture.zig");

// ---- helpers --------------------------------------------------------

fn s(v: []const u8) i32 {
    return obj.obj_new_string(@bitCast(@intFromPtr(v.ptr)), @bitCast(v.len));
}

fn bytes(o: i32) []const u8 {
    const r = obj.obj_ensure_string(o);
    if (r.len == 0) return &.{};
    const p: [*]const u8 = @ptrFromInt(r.ptr);
    return p[0..r.len];
}

fn intResult(o: i32) i64 {
    return obj.obj_get_int(o);
}

fn matched(pat: []const u8, subj: []const u8) bool {
    return intResult(re.tcl_cmd_regexp(s(pat), s(subj))) == 1;
}

// ---- regexp basic match / no-match ---------------------------------

test "tcl_cmd_regexp — literal match returns 1" {
    try testing.expect(matched("hello", "hello world"));
    try testing.expect(matched("foo", "barfoo"));
}

test "tcl_cmd_regexp — literal no-match returns 0" {
    try testing.expect(!matched("xyz", "hello world"));
    try testing.expect(!matched("FOO", "foo")); // case-sensitive
}

test "tcl_cmd_regexp — empty pattern matches anywhere" {
    // The Spencer engine treats an empty pattern as a zero-length
    // match at position 0; reference Tcl agrees.
    try testing.expect(matched("", ""));
    try testing.expect(matched("", "anything"));
}

// ---- anchors -------------------------------------------------------

test "tcl_cmd_regexp — ^ start anchor" {
    try testing.expect(matched("^hello", "hello world"));
    try testing.expect(!matched("^world", "hello world"));
}

test "tcl_cmd_regexp — $ end anchor" {
    try testing.expect(matched("world$", "hello world"));
    try testing.expect(!matched("hello$", "hello world"));
}

test "tcl_cmd_regexp — ^pat$ exact-match anchors" {
    try testing.expect(matched("^hello$", "hello"));
    try testing.expect(!matched("^hello$", "hello world"));
}

// ---- character classes / quantifiers --------------------------------

test "tcl_cmd_regexp — . dot matches any single char" {
    try testing.expect(matched("h.llo", "hello"));
    try testing.expect(matched("h.llo", "hxllo"));
    try testing.expect(!matched("h.llo", "hllo"));
}

test "tcl_cmd_regexp — [abc] character set" {
    try testing.expect(matched("[abc]", "cat"));
    try testing.expect(!matched("[xyz]", "cat"));
}

test "tcl_cmd_regexp — [a-z] range" {
    try testing.expect(matched("[a-z]+", "Hello"));
    try testing.expect(!matched("^[A-Z]+$", "hello"));
}

test "tcl_cmd_regexp — [^abc] negated set" {
    try testing.expect(matched("[^a-z]", "Hello"));
    try testing.expect(!matched("^[^a-z]+$", "hello"));
}

test "tcl_cmd_regexp — quantifiers ? * + {N}" {
    // ``?`` zero-or-one
    try testing.expect(matched("colou?r", "color"));
    try testing.expect(matched("colou?r", "colour"));
    // ``*`` zero-or-more
    try testing.expect(matched("a*b", "b"));
    try testing.expect(matched("a*b", "aaab"));
    // ``+`` one-or-more
    try testing.expect(matched("a+b", "ab"));
    try testing.expect(!matched("^a+b$", "b"));
    // ``{N}`` exactly N
    try testing.expect(matched("^a{3}$", "aaa"));
    try testing.expect(!matched("^a{3}$", "aa"));
}

test "tcl_cmd_regexp — alternation a|b" {
    try testing.expect(matched("cat|dog", "I have a cat"));
    try testing.expect(matched("cat|dog", "I have a dog"));
    try testing.expect(!matched("cat|dog", "I have a fish"));
}

// ---- capture groups via regsub backreferences ----------------------

test "do_regsub — & substitutes whole match" {
    var n: i32 = 0;
    const out = re.do_regsub(s("[0-9]+"), s("abc 123 def"), s("<&>"), false, false, &n);
    try testing.expectEqualStrings("abc <123> def", bytes(out));
    try testing.expectEqual(@as(i32, 1), n);
}

test "do_regsub — \\1 references capture group" {
    var n: i32 = 0;
    // Swap two-letter pairs around an underscore.
    const out = re.do_regsub(
        s("([a-z]+)_([a-z]+)"),
        s("foo_bar"),
        s("\\2_\\1"),
        false,
        false,
        &n,
    );
    try testing.expectEqualStrings("bar_foo", bytes(out));
    try testing.expectEqual(@as(i32, 1), n);
}

test "do_regsub — all=true replaces every occurrence" {
    var n: i32 = 0;
    const out = re.do_regsub(
        s("[0-9]+"),
        s("a1 b22 c333"),
        s("#"),
        false, // nocase
        true,  // all
        &n,
    );
    try testing.expectEqualStrings("a# b# c#", bytes(out));
    try testing.expectEqual(@as(i32, 3), n);
}

test "do_regsub — all=false stops after first match" {
    var n: i32 = 0;
    const out = re.do_regsub(
        s("[0-9]+"),
        s("a1 b22 c333"),
        s("#"),
        false,
        false,
        &n,
    );
    try testing.expectEqualStrings("a# b22 c333", bytes(out));
    try testing.expectEqual(@as(i32, 1), n);
}

test "do_regsub — nocase=true folds case during match" {
    var n: i32 = 0;
    const out = re.do_regsub(
        s("hello"),
        s("Hello World"),
        s("hi"),
        true, // nocase
        false,
        &n,
    );
    try testing.expectEqualStrings("hi World", bytes(out));
    try testing.expectEqual(@as(i32, 1), n);
}

test "do_regsub — no match leaves string unchanged" {
    var n: i32 = 0;
    const out = re.do_regsub(s("xyz"), s("hello"), s("Q"), false, true, &n);
    try testing.expectEqualStrings("hello", bytes(out));
    try testing.expectEqual(@as(i32, 0), n);
}

test "do_regsub — \\\\ subspec emits literal backslash" {
    // A ``\\`` pair in subspec becomes a single backslash byte
    // (matches reference Tcl's regsub semantics).
    var n: i32 = 0;
    const out = re.do_regsub(s("X"), s("aXb"), s("\\\\"), false, false, &n);
    try testing.expectEqualStrings("a\\b", bytes(out));
}

// ---- error paths (need #266 fixture) -------------------------------

const BadPatternCtx = struct {
    fn body() void {
        // Unbalanced bracket — Spencer's compile rejects this.
        _ = re.tcl_cmd_regexp(s("[abc"), s("anything"));
    }
};

test "tcl_cmd_regexp — malformed pattern raises compile error" {
    const msg = fixture.with_catch(&BadPatternCtx.body);
    try testing.expect(msg != null);
    try testing.expectEqualStrings(
        "regexp: couldn't compile regular expression pattern",
        msg.?,
    );
}

const BadParenCtx = struct {
    fn body() void {
        // Unbalanced ``(`` group — also a compile error.
        _ = re.tcl_cmd_regexp(s("(abc"), s("anything"));
    }
};

test "tcl_cmd_regexp — unbalanced paren raises compile error" {
    const msg = fixture.with_catch(&BadParenCtx.body);
    try testing.expect(msg != null);
    try testing.expectEqualStrings(
        "regexp: couldn't compile regular expression pattern",
        msg.?,
    );
}

const RegsubBadPatternCtx = struct {
    fn body() void {
        var n: i32 = 0;
        _ = re.do_regsub(s("[abc"), s("anything"), s(""), false, false, &n);
    }
};

test "do_regsub — malformed pattern raises with regsub prefix" {
    const msg = fixture.with_catch(&RegsubBadPatternCtx.body);
    try testing.expect(msg != null);
    try testing.expectEqualStrings(
        "regsub: couldn't compile regular expression pattern",
        msg.?,
    );
}
