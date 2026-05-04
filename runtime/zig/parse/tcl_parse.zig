// Tcl script / word tokeniser.
//
// Ports the reference Tcl 9.0 ``Tcl_ParseCommand`` / ``Tcl_ParseBraces``
// / ``Tcl_ParseQuotedString`` / ``Tcl_ParseVarName`` family from
// ``generic/tclParse.c`` into Zig.  Exposes two API surfaces:
//
//   - **Flat-array API** — the legacy ``parse_command(src, pos, len,
//     &word_ptrs, &word_lens, &word_braced, &word_expand)`` used by
//     ``tcl_interp.eval_script``.  Returns a ``{count, next}`` struct.
//     Re-used verbatim from what used to live in ``tcl_interp.zig``,
//     so the interpreter can keep working unchanged.
//
//   - **Token-tree API** — the richer ``ParseCommand`` that returns a
//     :data:`Parse` record with a depth-first array of :data:`Token`
//     entries, mirroring reference Tcl's ``Tcl_Parse`` / ``Tcl_Token``
//     tree.  Extension over reference Tcl: each :data:`Token` of kind
//     ``.WORD`` carries a ``braced: bool`` flag because reference C
//     infers "was braced?" from ``numComponents == 1 && child ==
//     TEXT``, and losing that distinction is exactly what produced
//     the earlier ``_emit_args_list`` / ``dispatch`` newline bugs.
//
// Layering: this module is a pure parser — it does NOT evaluate
// substitutions.  ``parse_bare`` deliberately skips across ``[...]`` and
// ``${...}`` without descending into them; the interpreter or the
// compiler resolves their contents later via ``subst_flagged`` (script
// context) or ``tcl_bs.consume_bs_escape`` (``\x`` decoding).  Whitespace
// classifiers live in ``tcl_chars.zig`` — the script parser here
// inlines its own byte checks because it also treats ``;`` / ``\n``
// specially (not whitespace per-se, but command terminators), so
// importing ``chars.is_space`` wouldn't be a drop-in swap.

pub const MAX_WORDS: u32 = 128;

// ---------------------------------------------------------------------
// Flat-array API — legacy.  Kept here so callers don't have to migrate
// in the same commit as the file move.
// ---------------------------------------------------------------------

pub const BracedRange = struct { end: u32, start: u32, wlen: u32 };

pub fn skip_space(src: [*]const u8, pos: u32, len: u32) u32 {
    var p = pos;
    while (p < len) {
        if (src[p] == ' ' or src[p] == '\t') {
            p += 1;
        } else if (src[p] == '\\' and p + 1 < len and src[p + 1] == '\n') {
            // Tcl line continuation: ``\<newline>`` collapses to a
            // single space (TclParseAllWhiteSpace, TIP #9).  Treat
            // it as inter-word whitespace so ``cmd arg1 \<NL>arg2``
            // parses as ``cmd arg1 arg2`` rather than giving concat
            // a spurious ``\`` element.
            p += 2;
        } else {
            break;
        }
    }
    return p;
}

pub fn parse_braced(src: [*]const u8, pos: u32, len: u32) BracedRange {
    var p = pos + 1;
    const start = p;
    var depth: u32 = 1;
    while (p < len and depth > 0) {
        // Tcl brace parsing: ``\<anychar>`` inside a braced word
        // consumes two bytes without affecting the brace depth —
        // so ``\{`` / ``\}`` are NOT depth-changing sequences,
        // matching Tcl's TclParseBraces.  Without this, a test body
        // like ``{lappend x \{\  abc}`` would see the ``\{`` bump
        // depth past the closing ``}`` and consume the rest of the
        // script into the single word.
        if (src[p] == '\\' and p + 1 < len) {
            p += 2;
            continue;
        }
        if (src[p] == '{') depth += 1 else if (src[p] == '}') depth -= 1;
        if (depth > 0) p += 1 else p += 1;
    }
    // Balanced close — ``p`` is now one past the ``}``, so the
    // content span is [start, p-1).  Unterminated input
    // (``depth > 0``) leaves no trailing ``}`` to subtract, so the
    // content runs all the way to ``p`` (== len) — skipping this
    // guard would u32-underflow when the source is a single ``{``.
    const wlen = if (depth == 0) p - 1 - start else p - start;
    return .{ .end = p, .start = start, .wlen = wlen };
}

pub fn parse_quoted(src: [*]const u8, pos: u32, len: u32) BracedRange {
    var p = pos + 1;
    const start = p;
    while (p < len and src[p] != '"') {
        if (src[p] == '\\' and p + 1 < len) p += 2 else p += 1;
    }
    const wlen = p - start;
    if (p < len) p += 1;
    return .{ .end = p, .start = start, .wlen = wlen };
}

pub fn parse_bare(src: [*]const u8, pos: u32, len: u32) BracedRange {
    const start = pos;
    var p = pos;
    // Scan until a top-level terminator.  Crucially, nested ``[...]``
    // command substitutions and ``${...}`` variable references must
    // be kept inside the same word — splitting on the space inside
    // ``[clock seconds]`` would truncate the inner command when
    // subst_word later runs it through eval_script (the observed
    // "unknown command: cloc" off-by-one).
    while (p < len and src[p] != ' ' and src[p] != '\t' and
        src[p] != '\n' and src[p] != ';' and src[p] != '\r')
    {
        if (src[p] == '\\' and p + 1 < len) {
            // ``\<newline>`` terminates the word — the line-continuation
            // sequence collapses to a single space (a word separator),
            // matching reference Tcl's TclParseAllWhiteSpace.  Without
            // this the backslash-newline pair would be swallowed into
            // the word span and leak through as a literal backslash.
            if (src[p + 1] == '\n') break;
            p += 2;
        } else if (src[p] == '[') {
            p = skip_command_subst(src, p, len);
        } else if (src[p] == '$' and p + 1 < len and src[p + 1] == '{') {
            // ``${...}`` keeps its braces together — normal $name
            // refs terminate on the first non-identifier char which
            // is handled by the outer loop already.
            p += 2;
            while (p < len and src[p] != '}') p += 1;
            if (p < len) p += 1;
        } else {
            p += 1;
        }
    }
    return .{ .end = p, .start = start, .wlen = p - start };
}

/// Advance past one balanced ``[...]`` command substitution.  ``p``
/// must point to the ``[``; returns the index after the matching
/// ``]``.  Shared helper for :func:`parse_bare` and the substitution
/// path (``tcl_interp.subst_flagged``) so both use the same nesting
/// / backslash-escape semantics.
///
/// Tracks ``{...}`` braces and ``"..."`` quotes inside the bracket
/// content so a ``]`` inside a brace-quoted word or quoted string is
/// treated literally — matching reference Tcl's parser.  Without
/// this, ``[catch {subst {[set a 1}} msg]`` would scan the inner ``[``
/// as a depth bump and the outer ``]`` as depth-1 (not 0), causing the
/// caller to consume the rest of the source into one giant "word".
pub fn skip_command_subst(src: [*]const u8, pos: u32, len: u32) u32 {
    var p = pos + 1;
    var depth: u32 = 1;
    while (p < len and depth > 0) {
        const c = src[p];
        if (c == '\\' and p + 1 < len) {
            p += 2;
            continue;
        }
        if (c == '{') {
            // Skip a balanced braced word (with backslash escapes).
            // Brace count is independent of bracket depth.
            p += 1;
            var bdepth: u32 = 1;
            while (p < len and bdepth > 0) {
                if (src[p] == '\\' and p + 1 < len) {
                    p += 2;
                    continue;
                }
                if (src[p] == '{') bdepth += 1
                else if (src[p] == '}') bdepth -= 1;
                p += 1;
            }
            continue;
        }
        if (c == '"') {
            // Skip a quoted string (with backslash escapes).  Inside
            // a quoted string, ``[``/``]`` are literal so the bracket
            // depth doesn't change.
            p += 1;
            while (p < len and src[p] != '"') {
                if (src[p] == '\\' and p + 1 < len) p += 2 else p += 1;
            }
            if (p < len) p += 1; // skip the closing "
            continue;
        }
        if (c == '[') depth += 1
        else if (c == ']') depth -= 1;
        p += 1;
    }
    return p;
}

pub const CommandResult = struct { count: u32, next: u32 };

pub fn parse_command(
    src: [*]const u8,
    pos: u32,
    len: u32,
    word_ptrs: *[MAX_WORDS]u32,
    word_lens: *[MAX_WORDS]u32,
    word_braced: *[MAX_WORDS]bool,
    word_expand: *[MAX_WORDS]bool,
) CommandResult {
    var p = pos;
    var count: u32 = 0;

    // Skip leading whitespace + command terminators.  ``\<newline>`` is
    // also whitespace here (Tcl line continuation) so consecutive
    // command-terminators separated by continuations don't strand the
    // parser on a lonely backslash.
    while (p < len) {
        const c = src[p];
        if (c == ' ' or c == '\t' or c == '\n' or c == '\r' or c == ';') {
            p += 1;
        } else if (c == '\\' and p + 1 < len and src[p + 1] == '\n') {
            p += 2;
        } else {
            break;
        }
    }

    if (p < len and src[p] == '#') {
        while (p < len and src[p] != '\n') p += 1;
        if (p < len) p += 1;
        return .{ .count = 0, .next = p };
    }

    while (p < len and count < MAX_WORDS) {
        p = skip_space(src, p, len);
        if (p >= len or src[p] == '\n' or src[p] == ';' or src[p] == '\r') {
            if (p < len) p += 1;
            break;
        }
        if (src[p] == '#' and count == 0) {
            while (p < len and src[p] != '\n') p += 1;
            if (p < len) p += 1;
            break;
        }

        // Detect ``{*}`` argument-expansion prefix (Tcl 8.5+).  The
        // three-character sequence ``{*}`` immediately before a word
        // signals that the word should be evaluated and then split as
        // a Tcl list, with each element inserted as a separate
        // argument.  Strip the prefix here and record the expansion
        // flag; the actual splitting happens in eval_script.
        var expand = false;
        if (src[p] == '{' and p + 2 < len and src[p + 1] == '*' and src[p + 2] == '}') {
            expand = true;
            p += 3;
            // Skip any whitespace between {*} and the word (rare but
            // valid in Tcl: ``cmd {*} $args`` is the same as
            // ``cmd {*}$args``).
            p = skip_space(src, p, len);
            if (p >= len or src[p] == '\n' or src[p] == ';') {
                // bare {*} with nothing following — treat as empty expansion
                word_ptrs[count] = 0;
                word_lens[count] = 0;
                word_braced[count] = false;
                word_expand[count] = true;
                count += 1;
                break;
            }
        }

        if (src[p] == '{') {
            const r = parse_braced(src, p, len);
            word_ptrs[count] = @intFromPtr(src) + r.start;
            word_lens[count] = r.wlen;
            word_braced[count] = true;
            word_expand[count] = expand;
            count += 1;
            p = r.end;
        } else if (src[p] == '"') {
            const r = parse_quoted(src, p, len);
            word_ptrs[count] = @intFromPtr(src) + r.start;
            word_lens[count] = r.wlen;
            word_braced[count] = false;
            word_expand[count] = expand;
            count += 1;
            p = r.end;
        } else {
            const r = parse_bare(src, p, len);
            word_ptrs[count] = @intFromPtr(src) + r.start;
            word_lens[count] = r.wlen;
            word_braced[count] = false;
            word_expand[count] = expand;
            count += 1;
            p = r.end;
        }
    }
    return .{ .count = count, .next = p };
}

// ---------------------------------------------------------------------
// Token-tree API — mirrors reference Tcl's ``Tcl_Parse`` / ``Tcl_Token``.
// ---------------------------------------------------------------------
//
// Every word in a parsed command becomes a tree of ``Token`` records
// stored depth-first in a flat bump-allocated array — parent before
// children, matching reference Tcl's layout.  Consumers walk the array
// by index, using ``n_children`` to skip subtrees.  This is a superset
// of the flat-array API: callers who only need ``(ptr, len, braced)``
// per word can read those directly off each top-level ``.WORD`` /
// ``.SIMPLE_WORD`` token.
//
// Producers emitted so far:
//   (none yet — :func:`ParseCommand` below fills a tree from the flat
//   arrays.  Future commits will migrate the parser to emit tokens
//   directly during the walk.)
//
// Consumers expected:
//   - ``tcl_interp.eval_script`` — substitution walks the tree per-word.
//   - Future compiler-side tools that need per-word component detail.

/// Token kind — mirrors reference Tcl's ``TCL_TOKEN_*`` enum.
pub const TokKind = enum(u8) {
    /// A whole word that contains substitutions — has ``n_children``
    /// sub-tokens describing the components.
    WORD,
    /// A whole word that is a single literal component.  Child count
    /// is still 1 (a single ``.TEXT`` child) so consumers that walk
    /// children uniformly don't need a special case.
    SIMPLE_WORD,
    /// Verbatim bytes from source.
    TEXT,
    /// A backslash escape sequence (the ``\x`` is included in the
    /// span).  Consumers decode via ``tcl_bs.consume_bs_escape``.
    BS,
    /// ``$name`` / ``${name}`` / ``$arr(key)``.  ``n_children``
    /// describes optional index tokens for array references.
    VARIABLE,
    /// ``[...]`` command substitution — span covers the brackets.
    COMMAND,
    /// A placeholder preceding a ``{*}``-expanded word.  Zero
    /// children; the real word follows immediately in the array.
    EXPAND_WORD,
};

/// One parse token.  Kept 16 bytes so a full MAX_WORDS-command fits
/// in a small bump allocation.
pub const Token = struct {
    kind: TokKind,
    /// Reserved for future flag bits (``NUMERIC``, ``STRICT_PARSE`` …).
    flags: u8 = 0,
    /// Direct child count — 0 for leaves, >=1 for WORD / SIMPLE_WORD
    /// / VARIABLE parents.  Children live at the next `n_children`
    /// slots in depth-first order (consumers can pre-compute a span
    /// by summing children recursively).
    n_children: u16,
    /// Byte offset of the start of this token's source span.
    start: u32,
    /// Span length in bytes.
    len: u32,
    /// Extension over reference Tcl: on ``.WORD`` / ``.SIMPLE_WORD``,
    /// indicates the source word was a ``{...}`` braced literal.
    braced: bool = false,
};

/// Result of parsing one command.  Mirrors ``Tcl_Parse`` shape.
pub const Parse = struct {
    src_ptr: u32,
    src_len: u32,
    /// Flat array of tokens, depth-first.  Lifetime: bump-allocated by
    /// the caller's arena.  ``tokens_ptr`` is 0 when ``tokens_len ==
    /// 0`` (empty command).
    tokens_ptr: u32,
    tokens_len: u32,
    /// Source offset where parsing of this command BEGAN — i.e. the
    /// ``pos`` argument passed to :func:`ParseCommand`, which is
    /// typically the byte right after the previous command's
    /// terminator.  :func:`parse_command` internally consumes any
    /// leading whitespace / semicolons / newlines; the first
    /// ``Token``'s ``start`` gives the offset of the first actual
    /// word, if a finer source range is needed for diagnostics.
    command_start: u32,
    /// Source length covered by this command, measured from
    /// ``command_start``.  ``command_start + command_len == next``.
    command_len: u32,
    /// Where the caller should resume to parse the next command.
    next: u32,
    /// Count of top-level words (``.WORD`` / ``.SIMPLE_WORD``
    /// entries) — convenience for consumers.
    n_words: u32,
};

/// Token-tree form of :func:`parse_command`.  Writes at most
/// ``MAX_WORDS`` top-level word tokens into ``dst_tokens``; the caller
/// sizes the buffer.  ``n_children`` on each WORD is currently always
/// 0 — future commits will walk each word's contents to emit TEXT /
/// BS / VARIABLE / COMMAND sub-tokens matching reference Tcl.  Today
/// this is intentionally a shallow tree so the interpreter can migrate
/// incrementally.
pub fn ParseCommand(
    src: [*]const u8,
    pos: u32,
    len: u32,
    dst_tokens: [*]Token,
    dst_cap: u32,
) Parse {
    var word_ptrs: [MAX_WORDS]u32 = undefined;
    var word_lens: [MAX_WORDS]u32 = undefined;
    var word_braced: [MAX_WORDS]bool = undefined;
    var word_expand: [MAX_WORDS]bool = undefined;
    const r = parse_command(
        src,
        pos,
        len,
        &word_ptrs,
        &word_lens,
        &word_braced,
        &word_expand,
    );
    const base = @intFromPtr(src);
    var tok_count: u32 = 0;
    var i: u32 = 0;
    while (i < r.count) : (i += 1) {
        // Worst case: a ``{*}`` marker plus its word.  Bail out early
        // if the next write would exceed the caller-owned buffer —
        // silently truncating is wrong (the eval loop would lose a
        // word) but overwriting past the buffer is worse, so we stop
        // at the last fully-written token pair.  Callers must size
        // for the worst case: ``2 * MAX_WORDS``.
        if (tok_count + 2 > dst_cap) break;
        if (word_expand[i]) {
            dst_tokens[tok_count] = .{
                .kind = .EXPAND_WORD,
                .n_children = 0,
                .start = @intCast(word_ptrs[i] - base),
                .len = 0,
                .braced = false,
            };
            tok_count += 1;
        }
        dst_tokens[tok_count] = .{
            .kind = if (word_braced[i]) .SIMPLE_WORD else .WORD,
            .n_children = 0,
            .start = @intCast(word_ptrs[i] - base),
            .len = word_lens[i],
            .braced = word_braced[i],
        };
        tok_count += 1;
    }
    return .{
        .src_ptr = base,
        .src_len = len,
        .tokens_ptr = if (tok_count == 0) 0 else @intFromPtr(dst_tokens),
        .tokens_len = tok_count,
        .command_start = pos,
        .command_len = r.next - pos,
        .next = r.next,
        .n_words = r.count,
    };
}
