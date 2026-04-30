// Unit tests for ``parse/tcl_parse.zig`` — the script / word
// tokeniser ported from reference Tcl 9.0's ``Tcl_ParseCommand``.
//
// Mirrors the corpus exercised by ``tests/test_lexer*.py`` on the
// Python side, so a divergence in either parser shows up here first.
// Asserts both the flat-array API (``parse_command``) and the
// token-tree API (``ParseCommand``).

const std = @import("std");
const testing = std.testing;

const tp = @import("tcl_parse.zig");

// ---- helpers --------------------------------------------------------

const ParsedWord = struct {
    text: []const u8,
    braced: bool,
    expand: bool,
};

fn parse(src: []const u8, out: *[tp.MAX_WORDS]ParsedWord) struct { count: u32, next: u32 } {
    var ptrs: [tp.MAX_WORDS]u32 = undefined;
    var lens: [tp.MAX_WORDS]u32 = undefined;
    var braced: [tp.MAX_WORDS]bool = undefined;
    var expand: [tp.MAX_WORDS]bool = undefined;
    const r = tp.parse_command(src.ptr, 0, @intCast(src.len), &ptrs, &lens, &braced, &expand);
    var i: u32 = 0;
    while (i < r.count) : (i += 1) {
        const word_ptr: [*]const u8 = @ptrFromInt(ptrs[i]);
        out[i] = .{
            .text = word_ptr[0..lens[i]],
            .braced = braced[i],
            .expand = expand[i],
        };
    }
    return .{ .count = r.count, .next = r.next };
}

// ---- skip_space -----------------------------------------------------

test "skip_space — spaces, tabs, line continuation" {
    const src = "  \tabc";
    try testing.expectEqual(@as(u32, 3), tp.skip_space(src.ptr, 0, @intCast(src.len)));

    // ``\<newline>`` collapses to whitespace and is skipped along
    // with the surrounding spaces — cursor lands on the first
    // non-space byte (``a`` at offset 6 in the source below).
    const cont = "  \\\n  abc";
    try testing.expectEqual(@as(u32, 6), tp.skip_space(cont.ptr, 0, @intCast(cont.len)));

    // Newline / semicolon are command terminators, NOT in-word spaces.
    const term = "\nabc";
    try testing.expectEqual(@as(u32, 0), tp.skip_space(term.ptr, 0, @intCast(term.len)));
}

// ---- parse_braced ---------------------------------------------------

test "parse_braced — simple, nested, and escaped braces" {
    // ``{abc}`` → start=1, len=3, end=5.
    var src: []const u8 = "{abc}";
    var r = tp.parse_braced(src.ptr, 0, @intCast(src.len));
    try testing.expectEqual(@as(u32, 1), r.start);
    try testing.expectEqual(@as(u32, 3), r.wlen);
    try testing.expectEqual(@as(u32, 5), r.end);

    // Nested braces keep their structure.
    src = "{a {b c} d}";
    r = tp.parse_braced(src.ptr, 0, @intCast(src.len));
    try testing.expectEqualStrings("a {b c} d", src[r.start .. r.start + r.wlen]);

    // Escaped braces don't change depth — content runs to outer ``}``.
    src = "{a\\{b\\}c}";
    r = tp.parse_braced(src.ptr, 0, @intCast(src.len));
    try testing.expectEqualStrings("a\\{b\\}c", src[r.start .. r.start + r.wlen]);
}

test "parse_braced — unterminated brace doesn't underflow" {
    // Single ``{`` should produce a sane span without trapping.
    const src: []const u8 = "{";
    const r = tp.parse_braced(src.ptr, 0, @intCast(src.len));
    // Content span runs to ``len`` (no trailing ``}`` to subtract).
    try testing.expectEqual(@as(u32, 1), r.start);
    try testing.expectEqual(@as(u32, 0), r.wlen);
}

// ---- parse_quoted ---------------------------------------------------

test "parse_quoted — bytes between matching double quotes" {
    const src: []const u8 = "\"hello world\" tail";
    const r = tp.parse_quoted(src.ptr, 0, @intCast(src.len));
    try testing.expectEqualStrings("hello world", src[r.start .. r.start + r.wlen]);
    try testing.expectEqual(@as(u32, 13), r.end); // one past the closing "
}

test "parse_quoted — backslash-quote does not terminate" {
    const src: []const u8 = "\"a\\\"b\" rest";
    const r = tp.parse_quoted(src.ptr, 0, @intCast(src.len));
    try testing.expectEqualStrings("a\\\"b", src[r.start .. r.start + r.wlen]);
}

// ---- parse_bare -----------------------------------------------------

test "parse_bare — terminates on whitespace / semicolon / newline" {
    const src: []const u8 = "foo bar";
    const r = tp.parse_bare(src.ptr, 0, @intCast(src.len));
    try testing.expectEqualStrings("foo", src[r.start .. r.start + r.wlen]);
}

test "parse_bare — keeps [...] command subst inside the word" {
    const src: []const u8 = "[clock seconds]xy more";
    const r = tp.parse_bare(src.ptr, 0, @intCast(src.len));
    try testing.expectEqualStrings("[clock seconds]xy", src[r.start .. r.start + r.wlen]);
}

test "parse_bare — keeps ${...} variable inside the word" {
    const src: []const u8 = "x${name}y rest";
    const r = tp.parse_bare(src.ptr, 0, @intCast(src.len));
    try testing.expectEqualStrings("x${name}y", src[r.start .. r.start + r.wlen]);
}

test "parse_bare — backslash-newline terminates the word" {
    // ``foo\<NL>bar`` is two words joined by line continuation: the
    // backslash-newline acts as a word separator, so parse_bare stops
    // at the backslash.
    const src: []const u8 = "foo\\\nbar";
    const r = tp.parse_bare(src.ptr, 0, @intCast(src.len));
    try testing.expectEqualStrings("foo", src[r.start .. r.start + r.wlen]);
}

// ---- skip_command_subst --------------------------------------------

test "skip_command_subst — balances nested brackets" {
    const src: []const u8 = "[a [b] c] tail";
    try testing.expectEqual(@as(u32, 9), tp.skip_command_subst(src.ptr, 0, @intCast(src.len)));
}

test "skip_command_subst — backslash escapes brackets" {
    const src: []const u8 = "[a\\] b] tail";
    // The escaped ``\]`` does not close the substitution; the real
    // close is the second ``]`` at index 6, so the function returns 7
    // (one past the close).
    try testing.expectEqual(@as(u32, 7), tp.skip_command_subst(src.ptr, 0, @intCast(src.len)));
}

// ---- parse_command — flat-array API --------------------------------

test "parse_command — simple two-word command" {
    var words: [tp.MAX_WORDS]ParsedWord = undefined;
    const r = parse("puts hello", &words);
    try testing.expectEqual(@as(u32, 2), r.count);
    try testing.expectEqualStrings("puts", words[0].text);
    try testing.expectEqualStrings("hello", words[1].text);
    try testing.expect(!words[0].braced and !words[1].braced);
    try testing.expect(!words[0].expand and !words[1].expand);
}

test "parse_command — braced word sets braced flag" {
    var words: [tp.MAX_WORDS]ParsedWord = undefined;
    const r = parse("set x {hello world}", &words);
    try testing.expectEqual(@as(u32, 3), r.count);
    try testing.expectEqualStrings("set", words[0].text);
    try testing.expectEqualStrings("x", words[1].text);
    try testing.expectEqualStrings("hello world", words[2].text);
    try testing.expect(!words[0].braced and !words[1].braced and words[2].braced);
}

test "parse_command — quoted word strips surrounding quotes" {
    var words: [tp.MAX_WORDS]ParsedWord = undefined;
    const r = parse("puts \"hi there\"", &words);
    try testing.expectEqual(@as(u32, 2), r.count);
    try testing.expectEqualStrings("hi there", words[1].text);
    try testing.expect(!words[1].braced);
}

test "parse_command — semicolon ends the command" {
    var words: [tp.MAX_WORDS]ParsedWord = undefined;
    const r = parse("a b ; c d", &words);
    try testing.expectEqual(@as(u32, 2), r.count);
    try testing.expectEqualStrings("a", words[0].text);
    try testing.expectEqualStrings("b", words[1].text);
    // ``next`` must point past the semicolon so the caller can resume.
    try testing.expect(r.next > 4);
}

test "parse_command — comment line is consumed and produces zero words" {
    var words: [tp.MAX_WORDS]ParsedWord = undefined;
    const r = parse("# this is a comment\n", &words);
    try testing.expectEqual(@as(u32, 0), r.count);
}

test "parse_command — {*}-expansion sets expand flag and strips marker" {
    var words: [tp.MAX_WORDS]ParsedWord = undefined;
    const r = parse("foo {*}$args bar", &words);
    try testing.expectEqual(@as(u32, 3), r.count);
    try testing.expectEqualStrings("foo", words[0].text);
    try testing.expectEqualStrings("$args", words[1].text);
    try testing.expect(words[1].expand);
    try testing.expectEqualStrings("bar", words[2].text);
    try testing.expect(!words[2].expand);
}

test "parse_command — line continuation joins logical line" {
    var words: [tp.MAX_WORDS]ParsedWord = undefined;
    const r = parse("cmd a \\\n   b c", &words);
    try testing.expectEqual(@as(u32, 4), r.count);
    try testing.expectEqualStrings("cmd", words[0].text);
    try testing.expectEqualStrings("a", words[1].text);
    try testing.expectEqualStrings("b", words[2].text);
    try testing.expectEqualStrings("c", words[3].text);
}

test "parse_command — bracketed substitution stays in one word" {
    var words: [tp.MAX_WORDS]ParsedWord = undefined;
    const r = parse("set x [clock seconds]", &words);
    try testing.expectEqual(@as(u32, 3), r.count);
    try testing.expectEqualStrings("[clock seconds]", words[2].text);
}

// ---- ParseCommand — token-tree API ---------------------------------

test "ParseCommand — emits one token per word with braced flag" {
    var tokens: [2 * tp.MAX_WORDS]tp.Token = undefined;
    const src: []const u8 = "set x {hello}";
    const p = tp.ParseCommand(src.ptr, 0, @intCast(src.len), &tokens, tokens.len);

    try testing.expectEqual(@as(u32, 3), p.n_words);
    try testing.expectEqual(@as(u32, 3), p.tokens_len);
    try testing.expectEqual(tp.TokKind.WORD, tokens[0].kind);
    try testing.expect(!tokens[0].braced);
    try testing.expectEqual(tp.TokKind.WORD, tokens[1].kind);
    try testing.expect(!tokens[1].braced);
    // Braced word becomes SIMPLE_WORD with braced=true.
    try testing.expectEqual(tp.TokKind.SIMPLE_WORD, tokens[2].kind);
    try testing.expect(tokens[2].braced);
    try testing.expectEqualStrings("hello", src[tokens[2].start .. tokens[2].start + tokens[2].len]);
}

test "ParseCommand — {*} word emits an EXPAND_WORD prefix" {
    var tokens: [2 * tp.MAX_WORDS]tp.Token = undefined;
    const src: []const u8 = "foo {*}$args";
    const p = tp.ParseCommand(src.ptr, 0, @intCast(src.len), &tokens, tokens.len);

    // foo + EXPAND_WORD + $args = 3 tokens, but only 2 ``words``.
    try testing.expectEqual(@as(u32, 2), p.n_words);
    try testing.expectEqual(@as(u32, 3), p.tokens_len);
    try testing.expectEqual(tp.TokKind.WORD, tokens[0].kind);
    try testing.expectEqual(tp.TokKind.EXPAND_WORD, tokens[1].kind);
    try testing.expectEqual(tp.TokKind.WORD, tokens[2].kind);
}

test "ParseCommand — empty source yields no tokens" {
    var tokens: [2 * tp.MAX_WORDS]tp.Token = undefined;
    const src: []const u8 = "";
    const p = tp.ParseCommand(src.ptr, 0, @intCast(src.len), &tokens, tokens.len);
    try testing.expectEqual(@as(u32, 0), p.n_words);
    try testing.expectEqual(@as(u32, 0), p.tokens_len);
    try testing.expectEqual(@as(u32, 0), p.tokens_ptr);
}
