"""Regression: ``puts`` of an integer emits exactly one trailing newline.

Before refactoring ``itoa`` to not embed a newline, a caller that
wanted ``puts -nonewline <int>`` would see the digit plus a stray
newline from inside the formatter.  The refactor moved newline
appendance entirely to ``puts_raw`` so both shapes behave correctly.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from tests.test_wasm_real_tcl import _compile_tcl_with_diag, _run_wasm


def _run(src: str) -> str:
    wasm, _ = _compile_tcl_with_diag(src)
    _, stdout = _run_wasm(wasm, capture_stdout=True)
    return stdout


class TestPutsInt:
    def test_puts_int_single_newline(self) -> None:
        out = _run("puts 42\n")
        assert out == "42\n"

    def test_puts_nonewline_int_no_trailing_newline(self) -> None:
        # Load-bearing: the old ``itoa`` embedded a newline inside
        # the digit buffer, so ``puts -nonewline`` still printed
        # ``42\n`` instead of ``42``.  With the refactor, digits are
        # standalone and ``puts -nonewline`` emits exactly ``42``.
        out = _run("puts -nonewline 42\n")
        assert out == "42"

    def test_puts_int_followed_by_puts_marker(self) -> None:
        # Two successive puts should produce two newlines total —
        # one per call, none embedded in the digits.
        out = _run("puts 1\nputs 2\n")
        assert out == "1\n2\n"

    def test_puts_negative_int(self) -> None:
        out = _run("puts -42\n")
        assert out == "-42\n"

    def test_puts_zero(self) -> None:
        out = _run("puts 0\n")
        assert out == "0\n"
