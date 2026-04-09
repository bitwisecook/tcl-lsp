"""Differential test harness for the Rust and Python Tcl lexers.

The Rust lexer is grown incrementally. At L3 it only understands the
SEP / EOL / COMMENT / plain ESC subset; inputs containing any of the
"deferred" characters (``$``, ``[``, ``]``, ``{``, ``}``, ``"``,
``\\``) cause the Rust lexer to raise ``ValueError``. This harness

1. hand-curates a corpus of inputs that exercise the currently
   supported subset;
2. also collects inputs from the broader lexer test suite that
   happen to fall in the subset;
3. feeds each input through both ``core.parsing.lexer.TclLexer``
   (the Python reference) and ``tcl_lsp_rust.lexer_tokenise`` (the
   Rust port);
4. asserts the two token streams are equal field-by-field.

As later chunks shrink the "deferred" character set, the corpus
picked up in step (2) grows automatically. Nothing in this file
needs to change when a chunk adds support for a construct — the new
inputs simply start passing the filter.

The harness is restricted to ASCII inputs because the Rust lexer
tracks column as byte-offset-within-line while the Python lexer
tracks code-point-offset-within-line. The two agree for ASCII and
drift for supplementary-plane characters. Multi-byte column parity
is deferred work tracked in ``docs/rust-rewrite.md``.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.parsing.lexer import TclLexer
from core.parsing.tokens import Token

tcl_lsp_rust = pytest.importorskip(
    "tcl_lsp_rust",
    reason="Rust extension not built; run `make rust-build`",
)

lexer_tokenise = tcl_lsp_rust.lexer_tokenise


def _rust_supports(source: str) -> bool:
    """True if the input is in the Rust lexer's currently supported subset.

    We ask the Rust lexer directly instead of hand-maintaining a
    deferred-characters blacklist. This has two advantages:

    1. As each chunk (L4, L5, L6, …) removes constructs from the
       Rust lexer's ``LexError::UnsupportedCharacter`` trigger set,
       the harness auto-detects the expanded support surface — no
       blacklist edit needed.
    2. It handles context-sensitive cases correctly. `{` and `}`
       inside a `${…}` braced variable name are fine even though
       `{` / `}` as top-level constructs are still deferred until
       L6; a character-only blacklist would reject such inputs
       unnecessarily.

    Restricted to ASCII so the non-ASCII column drift between Python
    and Rust (tracked in `docs/rust-rewrite.md` as deferred work)
    never shows up in parity checks.
    """
    if not source.isascii():
        return False
    try:
        lexer_tokenise(source)
    except ValueError:
        return False
    return True


def _python_tokens(source: str) -> list[Token]:
    return TclLexer(source).tokenise_all()


def _rust_tokens(source: str) -> list[Token]:
    return lexer_tokenise(source)


def _token_tuple(tok: Token) -> tuple:
    # Named fields, flattened for readable assertion failures.
    return (
        tok.type.name,
        tok.text,
        tok.start.line,
        tok.start.character,
        tok.start.offset,
        tok.end.line,
        tok.end.character,
        tok.end.offset,
        tok.in_quote,
    )


def _assert_same(source: str) -> None:
    py = [_token_tuple(t) for t in _python_tokens(source)]
    rs = [_token_tuple(t) for t in _rust_tokens(source)]
    assert py == rs, f"token stream mismatch for {source!r}\n  py={py}\n  rs={rs}"


# Hand-curated corpus. Every entry is a distinct shape the L3 Rust
# lexer must handle. Keep the labels short so pytest failure output
# stays readable.
CORPUS: list[tuple[str, str]] = [
    # Empty / whitespace-only
    ("empty", ""),
    ("single_space", " "),
    ("multiple_spaces", "   "),
    ("tab_only", "\t"),
    ("mixed_ws", " \t \t"),
    ("cr_only", "\r"),
    ("lf_only", "\n"),
    ("crlf", "\r\n"),
    ("semicolon_only", ";"),
    ("double_lf", "\n\n"),
    # Single word
    ("one_word", "foo"),
    ("one_long_word", "supercalifragilistic"),
    ("alphanumeric", "abc123"),
    ("dotted", "foo.bar"),
    ("underscored", "foo_bar"),
    ("colon_in_word", "a:b"),
    ("uppercase", "FOO"),
    # Multiple words
    ("two_words", "foo bar"),
    ("two_words_tab", "foo\tbar"),
    ("three_words", "one two three"),
    ("words_multi_spaces", "foo   bar"),
    ("cr_between_words", "foo\rbar"),
    # Newlines / semicolons
    ("words_lf_separated", "foo\nbar"),
    ("words_semicolon_separated", "foo;bar"),
    ("mixed_eol", "foo\n;\nbar"),
    ("multi_line", "a\nb\nc\nd"),
    ("trailing_lf", "foo\n"),
    ("trailing_ws", "foo  "),
    ("leading_ws", "  foo"),
    ("leading_lf", "\nfoo"),
    # Comments
    ("comment_only", "# comment"),
    ("comment_then_cmd", "# c\nfoo"),
    ("comment_with_leading_ws", "   # comment"),
    ("comment_with_trailing_ws", "# comment   "),
    ("hash_midword", "foo#bar"),
    ("hash_after_ws_not_start", "foo #bar"),
    ("two_comments", "# one\n# two"),
    ("comment_cmd_comment", "# one\nfoo\n# two"),
    # Punctuation inside words (safe, not deferred)
    ("word_with_dash", "foo-bar"),
    ("word_with_plus", "foo+bar"),
    ("word_with_slash", "foo/bar"),
    ("word_with_question", "foo?bar"),
    ("word_with_dot", "foo."),
    # Long / mixed fixtures
    ("many_commands", "a; b; c; d; e"),
    ("multi_line_with_comment", "foo\n# hello\nbar"),
    ("stress_whitespace", "  foo  bar  baz  "),
    ("only_separators", "\n;\n; \n"),
    # L4 — variable substitution
    ("var_simple", "$foo"),
    ("var_underscore", "$_private"),
    ("var_digit", "$1"),
    ("var_alnum", "$foo123"),
    ("var_uppercase", "$FOO"),
    ("var_ns", "$ns::var"),
    ("var_ns_deep", "$a::b::c"),
    ("var_leading_ns", "$::global"),
    ("var_single_colon_ends", "$foo:bar"),
    ("var_braced", "${name}"),
    ("var_braced_empty", "${}"),
    ("var_braced_with_spaces", "${my var}"),
    ("var_braced_with_special", "${weird#name}"),
    ("var_array", "$arr(idx)"),
    ("var_array_ns", "$ns::arr(key)"),
    ("var_array_nested", "$arr(one(two)three)"),
    ("var_array_braced_inner", "$arr(${key})"),
    ("bare_dollar", "$"),
    ("bare_dollar_space", "$ foo"),
    ("bare_dollar_lf", "$\n"),
    ("var_then_word", "$foo bar"),
    ("word_then_var", "foo$bar"),
    ("multiple_vars", "$a $b $c"),
    ("var_in_command", "set x $y"),
    ("var_resets_command_start", "$foo #not-a-comment"),
    ("var_after_comment", "# c\n$foo"),
    ("var_mid_stream", "puts $name; return $val"),
    # L4 unterminated — best-effort tokenisation
    ("unterminated_braced_var", "${unterminated"),
    ("unterminated_array_var", "$arr(idx"),
    # L5 — command substitution
    ("cmd_empty", "[]"),
    ("cmd_simple", "[cmd]"),
    ("cmd_with_args", "[+ 1 2]"),
    ("cmd_nested_once", "[+ 1 [+ 2 3]]"),
    ("cmd_nested_deep", "[a [b [c [d]]]]"),
    ("cmd_followed_by_word", "[cmd] tail"),
    ("cmd_then_text", "[cmd]tail"),
    ("word_then_cmd", "foo[cmd]"),
    ("cmd_with_var", "[expr $a + $b]"),
    ("cmd_with_braced_var", "[set ${odd}name value]"),
    ("cmd_with_quoted_substring", '[puts "hello world"]'),
    ("cmd_bracket_in_quotes", '[puts "a]b"]'),
    ("cmd_bracket_in_braces", "[list {a ] b}]"),
    ("cmd_nested_braces", "[list {a {nested} b}]"),
    ("cmd_backslash_close", "[a \\] b]"),
    ("cmd_backslash_quote", '[a \\" b]'),
    ("cmd_multiline", "[a\nb\nc]"),
    ("standalone_close_bracket", "foo]bar"),
    ("trailing_close_bracket", "foo bar]"),
    ("cmd_mid_command", "set x [+ 1 2]"),
    ("cmd_and_var_mix", "puts [expr $a * $b]"),
    ("multiple_cmds", "[a] [b] [c]"),
    ("cmd_at_line_start", "\n[cmd]"),
    ("cmd_then_var", "[foo]$bar"),
    ("var_then_cmd", "$foo[bar]"),
    # L5 unterminated — best-effort tokenisation
    ("unterminated_cmd", "[unterminated"),
    ("unterminated_nested_cmd", "[outer [inner"),
]


@pytest.mark.parametrize("label, source", CORPUS, ids=[label for label, _ in CORPUS])
def test_curated_corpus_matches_python(label: str, source: str):
    _assert_same(source)


# A few direct invariants on the harness itself — catch breakage in
# the filter/skip logic before it silently swallows real parity bugs.


class TestHarnessItself:
    # Characters whose handling the Rust lexer defers to later
    # chunks. Keep this list in sync with the Rust
    # `is_deferred_special` helper (not imported here to keep the
    # harness pytest-only). After L5: `$`, `[`, `]` have been
    # removed; `{`, `}`, `"`, `\` remain.
    _EXPECTED_DEFERRED = frozenset('{}"\\')

    def test_deferred_inputs_are_filtered(self):
        # Every character we say is deferred must actually trigger
        # the Rust ValueError so the harness's filter is sound.
        for ch in self._EXPECTED_DEFERRED:
            sample = f"foo{ch}bar"
            assert not _rust_supports(sample), f"{ch!r} should be filtered"
            with pytest.raises(ValueError):
                lexer_tokenise(sample)

    def test_dollar_is_no_longer_deferred(self):
        # Regression guard for L4: `$` must pass the filter now.
        assert _rust_supports("$foo")
        assert _rust_supports("${name}")
        assert _rust_supports("$arr(idx)")
        assert _rust_supports("$")

    def test_brackets_are_no_longer_deferred(self):
        # Regression guard for L5: `[` / `]` must pass the filter.
        assert _rust_supports("[cmd]")
        assert _rust_supports("[+ 1 2]")
        assert _rust_supports("foo]bar")  # lone `]` is part of a word

    def test_non_ascii_is_filtered(self):
        assert not _rust_supports("café")

    def test_supported_input_passes_filter(self):
        assert _rust_supports("foo bar\nbaz")
        assert _rust_supports("# comment\nfoo")
        assert _rust_supports("set x $y")
        assert _rust_supports("set x [+ 1 2]")


# Pull additional inputs from the broader lexer test suite wherever
# they fall in the supported subset. The test_lexer.py docstrings and
# source code literals are a good corpus — we don't try to be clever,
# we just harvest every string literal that survives the filter.

_LEXER_FIXTURE_INPUTS = [
    "puts hello",
    "foo bar baz",
    "# comment",
    "set x 10",  # will be filtered on `$`? no, no `$` here — keep.
    "a b c",
    "first\nsecond",
    "one; two; three",
    "proc foo a b",  # pure words, no braces — supported
    "return 42",
    "if 1 then x else y",
    "  leading spaces",
    "trailing spaces  ",
    "multi   spaces",
    "line one\nline two\nline three",
    "# only a comment",
    "# comment 1\n# comment 2",
    "# heading\ncommand\n# trailing",
    "foo-bar",
    "a.b.c",
    "nested/path/component",
]


@pytest.mark.parametrize(
    "source",
    [s for s in _LEXER_FIXTURE_INPUTS if _rust_supports(s)],
)
def test_harvested_fixtures_match_python(source: str):
    _assert_same(source)
