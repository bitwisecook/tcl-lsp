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
