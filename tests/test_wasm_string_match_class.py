"""WASM ``string match`` ``[…]`` character-class semantics.

Pins ``glob_match_class`` in ``runtime/zig/valtypes/tcl_string.zig``
against the bracket arm of C ``Tcl_StringCaseMatch`` (tclUtil.c):

* ``-`` is always a range separator and consumes the following
  codepoint as the range end — even ``]`` (so ``[A-]`` is the range
  ``A``..``]``, not the set ``{A, -}``);
* members and ranges are full Unicode codepoints (UTF-8 decoded);
* there is no negation — ``^`` / ``!`` are ordinary members;
* a class that matches but never closes completes the whole match only
  when the input string is exhausted; a malformed / empty class (``[``
  alone) fails the match.

Regression guard for util-5.13 / 5.15 / 5.17 / 5.23 / 5.40 / 5.41 / 5.44.
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run(source: str) -> str:
    wasm = _compile_tcl(source)
    _, stdout = _run_wasm(wasm, capture_stdout=True)
    return stdout.rstrip("\n")


@pytest.mark.parametrize(
    "source,expected",
    [
        # UTF-8 members / ranges — util-5.13 / 5.17 / 5.23.
        ('puts [string match "\\[乏xy\\]bc" "乏bc"]', "1"),
        ('puts [string match "a\\[a乏c]c" "a乏c"]', "1"),
        ('puts [string match "\\[一-乏]" "丳"]', "1"),
        # ``-]`` is the range A..] and leaves the class unclosed —
        # util-5.40 / 5.41 / 5.44.
        ("puts [string match {[A-]x} Ax]", "0"),
        ("puts [string match {[A-]]x} Ax]", "1"),
        ("puts [string match {[A-]h]x} hx]", "1"),
        # A bare ``[`` is a malformed class, not a literal — util-5.15.
        ("puts [string match {[} {[}]", "0"),
        # No negation: ``^`` / ``!`` are literal members.
        ("puts [string match {[^a]} b]", "0"),
        ("puts [string match {[^a]} ^]", "1"),
        ("puts [string match {[!a]} !]", "1"),
        # Bread-and-butter classes still work.
        ("puts [string match {[abc]} b]", "1"),
        ("puts [string match {[abc]} d]", "0"),
        ("puts [string match {[a-z]} m]", "1"),
        ("puts [string match {[z-a]} m]", "1"),
        # Matched-but-unclosed completes iff the string is exhausted.
        ("puts [string match {[a} a]", "1"),
        ("puts [string match {[ab} a]", "1"),
        # Class after a star (the star backtracks).
        ("puts [string match {a*[0-9]} a1]", "1"),
        ("puts [string match {a*[0-9]} ax]", "0"),
    ],
)
def test_string_match_class(source: str, expected: str) -> None:
    assert _run(source) == expected
