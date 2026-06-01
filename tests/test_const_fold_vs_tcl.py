"""Differential verification of constant folding against real C Tcl 9.

Every fold the optimiser performs for a pure builtin command substitution must
be byte-identical to what ``tclsh9.0`` produces — otherwise O129 (and the SCCP
variable-propagation path) would bake a wrong literal into the source.  These
tests run the actual interpreter and compare:

* :func:`fold_cmd_subst_to_string` (the registry folder the optimiser uses) for
  a broad matrix of commands, and
* the end-to-end optimiser output executed on tclsh9.0,

so a regression in any ``const_fold`` callback (e.g. the historical
``format`` width/precision bug) fails loudly.
"""

from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from compiler.core_analyses import fold_cmd_subst_to_string
from compiler.optimiser import optimise_source_multipass
from compiler.registry.runtime import configure_signatures


def _find_tcl9() -> str | None:
    for cand in (shutil.which("tclsh9.0"), shutil.which("tclsh")):
        if not cand or not Path(cand).exists():
            continue
        try:
            proc = subprocess.run(
                [cand], input="puts [info patchlevel]", capture_output=True, text=True, timeout=5
            )
        except (subprocess.TimeoutExpired, OSError):
            continue
        if proc.stdout.strip().startswith("9."):
            return cand
    return None


_TCLSH9 = _find_tcl9()
pytestmark = pytest.mark.skipif(_TCLSH9 is None, reason="tclsh9.0 not available")


def _tcl_value(cmd: str) -> str | None:
    """Run ``puts -nonewline [cmd]`` on tclsh9; None on error."""
    assert _TCLSH9 is not None
    proc = subprocess.run(
        [_TCLSH9], input=f"puts -nonewline [{cmd}]", capture_output=True, text=True, timeout=5
    )
    return proc.stdout if proc.returncode == 0 else None


def _fold(cmd: str) -> str | None:
    configure_signatures(dialect="tcl9.0")
    return fold_cmd_subst_to_string(f"[{cmd}]", {}, {})


# Commands the folder may or may not fold; when it DOES, it must match tclsh.
_FOLDABLE = [
    "list a b c", "list", "list {a b} c",
    "llength {a b c}", "llength {}", "llength {a {b c} d}",
    "lindex {a b c} 1", "lindex {a b c} end", "lindex {a b c} end-1",
    "lrange {a b c d} 1 2", "lrange {a b c d} 0 end",
    "lreverse {a b c}", "lrepeat 3 x",
    "concat a b c", "concat {a b} {c d}",
    "join {a b c} -", "join {a b c}", "join {a b c} :",
    "split a,b,c ,", "split abc {}",
    "string length hello", "string index hello 1", "string index hello end",
    "string range hello 1 3", "string reverse hello",
    "string toupper abc", "string tolower ABC", "string totitle abc",
    "string cat a b c", "string repeat ab 3",
    "string trim {  x  }", "string map {a X b Y} abab",
    "string first l hello", "string last l hello",
    "string equal abc abc", "string equal abc abd", "string compare a b",
    "string is integer 42", "string is alpha abc", "string is double 3.14",
    "dict get {a 1 b 2} b", "dict size {a 1 b 2}",
    "dict keys {a 1 b 2}", "dict values {a 1 b 2}", "dict exists {a 1 b 2} a",
    "expr {2 + 3 * 4}", "expr {10 % 3}", "expr {2 ** 10}",
]


@pytest.mark.parametrize("cmd", _FOLDABLE)
def test_fold_matches_tcl9(cmd: str) -> None:
    want = _tcl_value(cmd)
    if want is None:
        pytest.skip(f"tclsh errors on [{cmd}]")
    got = _fold(cmd)
    if got is None:
        return  # not folded — acceptable (a miss, never wrong)
    assert got == want, f"[{cmd}] folded to {got!r} but tclsh9 gives {want!r}"


# ``format`` is the historically broken one — flags / width / precision were
# silently dropped.  Verify a wide matrix end-to-end.
_FORMATS = [
    ("%d", "42"), ("%d", "-7"), ("%5d", "42"), ("%-5d", "42"), ("%05d", "7"),
    ("%+d", "42"), ("%x", "255"), ("%#x", "255"), ("%X", "255"), ("%o", "8"),
    ("%08x", "255"), ("%s", "hi"), ("%10s", "hi"), ("%-10s", "hi"), ("%.3s", "hello"),
    ("%f", "3.14"), ("%.2f", "3.14159"), ("%8.2f", "3.14159"), ("%-8.2f", "3.14159"),
    ("%+.2f", "3.14"), ("%08.3f", "2.5"), ("%e", "31400.0"), ("%g", "3.14"),
    ("%g", "1000000.0"), ("%c", "65"), ("%5c", "65"),
]


@pytest.mark.parametrize(("fmt", "arg"), _FORMATS)
def test_format_matches_tcl9(fmt: str, arg: str) -> None:
    cmd = f"format {{{fmt}}} {{{arg}}}"
    want = _tcl_value(cmd)
    assert want is not None
    got = _fold(cmd)
    if got is None:
        return
    assert got == want, f"format {fmt!r} {arg!r} -> {got!r}, tclsh9 {want!r}"


# End-to-end: optimise `puts [cmd]` and run the result on tclsh9 — output must
# be unchanged (the fold must be behaviour-preserving).
_E2E = [
    "puts [string length abcde]",
    "puts [join {a b c} -]",
    "puts [llength {a b c}]",
    "puts [format %03d 7]",
    "puts [format %5.2f 3.14159]",
    "puts [string toupper hello]",
    "puts [string range abcdef 1 3]",
    "puts [dict get {a 1 b 2} b]",
    "puts [string map {a X b Y} abab]",
    "puts [concat a {b c} d]",
    "puts [lrange {a b c d} 1 2]",
    "puts [string repeat = 6]",
    "set r [format %5.2f 3.14159]\nputs $r",
    "set n [llength {a b c}]\nputs [expr {$n + 1}]",
    # lappend list-build chains (O130)
    "set l {}\nlappend l a b\nputs $l",
    "set l {x}\nlappend l {a b}\nputs $l",
    "proc f {} {set acc {}\nlappend acc 1\nlappend acc 2\nlappend acc 3\nreturn $acc}\nputs [f]",
    # scan of constant literals (no varName form)
    "puts [scan 42 %d]",
    "puts [scan ff %x]",
    "puts [scan {1 2 3} {%d %d %d}]",
    "set n [scan 99 %d]\nputs $n",
]


@pytest.mark.parametrize("src", _E2E)
def test_optimised_output_matches_original(src: str) -> None:
    assert _TCLSH9 is not None
    configure_signatures(dialect="tcl9.0")
    optimised, _opts, _iters = optimise_source_multipass(src, max_iterations=15)
    before = subprocess.run([_TCLSH9], input=src, capture_output=True, text=True, timeout=10)
    after = subprocess.run([_TCLSH9], input=optimised, capture_output=True, text=True, timeout=10)
    assert (after.returncode, after.stdout) == (before.returncode, before.stdout), (
        f"behaviour changed for {src!r}\noptimised:\n{optimised}"
    )
