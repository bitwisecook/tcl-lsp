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


@pytest.mark.parametrize(
    "source,expected",
    [
        # regexp-27.1 / 27.2: the command result replaces each match.
        ("puts [regsub -command {.x.} {abcxdef} {string length}]", "ab3ef"),
        (
            "puts [regsub -command {.x.} {abcxdefxghi} {string length}]",
            "ab3efxghi",
        ),
        # regexp-27.3: -all with a zero-width lookahead, command counts.
        (
            "set x 0; puts [regsub -all -command {(?=.)} abcde {apply {args {incr ::x}}}]",
            "1a2b3c4d5e",
        ),
        # regexp-27.5 / 27.6: whole match + capture groups are appended.
        ("puts [regsub -command {(.)(.)} {abcdef} {list ,}]", ", ab a bcdef"),
        (
            "puts [regsub -command -all {(.)(.)} {abcdef} {list ,}]",
            ", ab a b, cd c d, ef e f",
        ),
        # regexp-27.7 / 27.8 / 27.12: representation-smash safety — the
        # command body rewrites the variable holding the subject /
        # subSpec, which must not corrupt the in-flight substitution.
        (
            "set ::s {123=456 789}; "
            r"puts [regsub -command -all {\d+} $::s {apply {n {expr {[llength $::s] + $n}}}}]",
            "125=458 791",
        ),
        (
            "set s {list (.+)}; puts [regsub -command $s {list list} $s]",
            "(.+) {list list} list",
        ),
        # Error cases — regexp-27.10 / 27.11.
        (
            'puts [list [catch {regsub -command . abc "def \\{ghi"} m] $m]',
            "1 {unmatched open brace in list}",
        ),
        (
            "puts [list [catch {regsub -command . abc {}} m] $m]",
            "1 {command prefix must be a list of at least one element}",
        ),
    ],
)
def test_regsub_command(source: str, expected: str) -> None:
    assert _run(source) == expected


@pytest.mark.parametrize(
    "source,expected",
    [
        # regexp-26.7: -line ^ anchors after a newline, with -start.
        (
            'puts [list [regexp -start 0 -indices -line {^a} "\nab" m1] $m1 '
            '[regexp -start 1 -indices -line {^a} "\nab" m2] $m2]',
            "1 {1 1} 1 {1 1}",
        ),
        # regexp-26.8: -all -inline -line diff-style block matching.
        (
            'set d "@1\n2\n+3\n@4\n-5\n+6\n7\n@8\n9\n"; '
            "puts [regexp -all -inline -line {^@.*\n(?:[^@].*\n?)*} $d]",
            "{@1\n2\n+3\n} {@4\n-5\n+6\n7\n} {@8\n9\n}",
        ),
        # regexp-26.9: embedded (?n) line flag, no -line switch.
        (
            'set d "@1\n2\n+3\n@4\n-5\n+6\n7\n@8\n9\n"; '
            "puts [regexp -all -inline {(?n)^@.*\n(?:[^@].*\n?)*} $d]",
            "{@1\n2\n+3\n} {@4\n-5\n+6\n7\n} {@8\n9\n}",
        ),
    ],
)
def test_regexp_line_anchors(source: str, expected: str) -> None:
    assert _run(source) == expected
