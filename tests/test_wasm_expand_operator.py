"""WASM ``{*}`` argument-expansion operator parsing.

Pins the rule in ``runtime/zig/parse/tcl_parse.zig``: per ``tclParse.c``
the three-character ``{*}`` prefix triggers expansion *only* when an
immediately-following non-whitespace, non-terminator character is
present.  A standalone ``{*}`` — at end-of-command, or followed by
whitespace / ``\n`` / ``;`` — is the ordinary brace-quoted literal word
``*``.

The previous parser treated every ``{*}`` as expansion (and even
skipped whitespace before the expanded word), so a literal ``{*}``
value silently vanished.  In the upstream ``compile.test`` bundle this
surfaced as a hard crash: tcltest's ``test`` proc received an odd
option list (``-result`` with its ``{*}`` value dropped) and
``array set testAttributes $args`` raised an uncaught "list must have an
even number of elements".
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run(source: str) -> str:
    wasm = _compile_tcl(source)
    _, stdout = _run_wasm(wasm, capture_stdout=True)
    return stdout.rstrip("\n")


_COUNT = "proc tp {args} {return [llength $args]}\n"
_ECHO = "proc tp {args} {return $args}\n"


@pytest.mark.parametrize(
    "source,expected",
    [
        # Standalone {*} is the literal word "*".
        (_COUNT + "puts [tp {*}]", "1"),
        (_COUNT + "puts [tp a -result {*}]", "3"),
        (_COUNT + "puts [tp a {*} b]", "3"),
        ("puts [list a {*} b]", "a * b"),
        ("puts [lindex [list -result {*}] 1]", "*"),
        # {*} followed by whitespace is literal "*" — the next word is NOT
        # expanded (real Tcl: `tp {*} $a` → `* {1 2 3}`).
        (_ECHO + "set a {1 2 3}\nputs [tp {*} $a]", "* {1 2 3}"),
        # Real expansion requires the word to immediately follow {*}.
        (_COUNT + "set a {1 2 3}\nputs [tp {*}$a]", "3"),
        ("puts [list {*}{a b c}]", "a b c"),
        (_COUNT + "puts [tp {*}{a b}]", "2"),
        # Same rules through the runtime eval / script parser.
        (_COUNT + "puts [eval {tp x -result {*}}]", "3"),
        (_COUNT + "puts [eval {tp {*}}]", "1"),
        (_COUNT + "set a {1 2}\nputs [eval {tp {*}$a}]", "2"),
        # array set of an option list whose value is {*} stays even
        # (the compile.test crash regression guard).
        (
            "array set t {-body x -result {*}}\nputs [list [array size t] $t(-result)]",
            "2 *",
        ),
    ],
)
def test_expand_operator(source: str, expected: str) -> None:
    assert _run(source) == expected


@pytest.mark.parametrize("n", [127, 128, 129, 500, 2000])
def test_expand_does_not_truncate_large_lists(n: int) -> None:
    # The compiled ``{*}`` slow path assembled words into a fixed 128-slot
    # stack array and silently dropped every word past 127, so any command
    # with more than ~127 expanded args was truncated (e.g.
    # ``tcl::mathop::+ {*}[lseq 1 1000]`` summed only 1..127).  WordBuf now
    # grows onto the heap; the full argument list must reach the command.
    src = _COUNT + f"puts [tp {{*}}[lseq 1 {n}]]"
    assert _run(src) == str(n)


def test_expand_large_sum_is_exact() -> None:
    # End-to-end: a >128-element {*} into a variadic builtin produces the
    # full sum (dict-24.24's wrong value came from this truncation).
    assert _run("puts [tcl::mathop::+ {*}[lseq 1 1000]]") == str(1000 * 1001 // 2)
