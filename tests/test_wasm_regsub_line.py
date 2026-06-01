"""WASM ``regsub -line`` — newline-anchored ``^`` / ``$`` substitution.

Pins the contract in ``runtime/zig/valtypes/tcl_regex.zig``'s
``eval_regsub_cmd`` loop:

* In ``-line`` mode ``^`` / ``$`` anchor at every embedded line break,
  not just the string ends.  The matcher walks suffixes and reproduces
  ``Tcl_RegsubObjCmd``'s ``REG_NOTBOL`` rule: NOTBOL is set unless the
  character before the search start is a newline.  Regression guard for
  ``regexp-24.2`` .. ``regexp-24.11``.
* A zero-width match (``^`` / ``$`` / word boundary, which can land
  mid-string at ``rm_so > 0`` in line mode) advances past the match and
  consumes one codepoint so the next iteration can't re-match the same
  empty span.  Without ``-line`` the anchors still bind only to the
  string ends (regexp-9.6).
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
        # regexp-24.2: empty lines around a single newline.
        ('puts [regsub -line -all {^} "\n" {<&>}]', "<>\n<>"),
        ('puts [regsub -line -all {^$} "\n" {<&>}]', "<>\n<>"),
        ('puts [regsub -line -all {$} "\n" {<&>}]', "<>\n<>"),
        # regexp-24.8: ``a\nb``.
        ('puts [regsub -line -all {^} "a\nb" {<&>}]', "<>a\n<>b"),
        ('puts [regsub -line -all {^.*$} "a\nb" {<&>}]', "<a>\n<b>"),
        ('puts [regsub -line -all {$} "a\nb" {<&>}]', "a<>\nb<>"),
        # regexp-24.10: three lines, no trailing newline.
        ('puts [regsub -line -all {^.*$} "a\nb\nc" {<&>}]', "<a>\n<b>\n<c>"),
        # regexp-24.11: a literal match interleaved with a line break.
        ('puts [regsub -line -all {b} "abb\nb" {<&>}]', "a<b><b>\n<b>"),
        # Return value is the match count.
        ('puts [regsub -line -all {^} "a\nb\n" {<&>} v]', "3"),
    ],
)
def test_regsub_line_anchors(source: str, expected: str) -> None:
    assert _run(source) == expected


@pytest.mark.parametrize(
    "source,expected",
    [
        # Without -line, ``^`` binds only at the absolute start — the
        # newline does NOT introduce a fresh anchor (regexp-9.6 family).
        ('puts [regsub -all {^} "a\nb" {<&>}]', "<>a\nb"),
        ("puts [regsub -all {^} xxx 123]", "123xxx"),
        # Word boundaries are still zero-width matches handled correctly
        # (this matches real Tcl 9, which only fires at the leading edge
        # of each word as the suffix walk sees it).
        ('puts [regsub -all {\\y} "ab cd" {|}]', "|a|b |c|d"),
    ],
)
def test_regsub_no_line(source: str, expected: str) -> None:
    assert _run(source) == expected
