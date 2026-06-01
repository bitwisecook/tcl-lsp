"""WASM ``scan`` command — character sets and real floating-point.

Pins two contracts added to ``runtime/zig/cmds/scan.zig``:

* ``%[...]`` character-set conversion (``BuildCharSet`` / ``CharInSet``):
  membership, ``^`` negation, ``a-z`` ranges (order-normalised), a
  leading ``]`` as a literal member, and a ``-`` adjacent to ``]`` as a
  literal.  Regression guard for the ``scan-1.*`` cluster.
* ``%e`` / ``%f`` / ``%g`` boxing a real ``TYPE_FLOAT`` instead of the
  old integer truncation (``scan 0.2 %f`` used to return ``0``).
  Regression guard for ``scan-4.49`` / ``scan-11.*`` / ``scan-14.*``.
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
        # %[...] character sets — mirrors scan-1.1 .. 1.8.
        ("puts [list [scan foo {%[^o]} x] $x]", "1 f"),
        ('set n [scan {]foo} {%[]f]} x]\nputs "$n $x"', "1 ]f"),
        ("puts [list [scan abc-def {%[a-c]} x] $x]", "1 abc"),
        ("puts [list [scan -abc-def {%[-ac]} x] $x]", "1 -a"),
        ("puts [list [scan -abc-def {%[ac-]} x] $x]", "1 -a"),
        ("puts [list [scan abc-def {%[c-a]} x] $x]", "1 abc"),
        ("puts [list [scan def-abc {%[^c-a]} x] $x]", "1 def-"),
        # charset following another conversion (scan-10.6).
        ("puts [scan 5a {%i%[a]}]", "5 a"),
        # Real float scanning — scan-4.49 / scan-11.1 / scan-14.1.
        ('scan {.1 0.2 3.} {%e %f %g} x y z\nputs "$x $y $z"', "0.1 0.2 3.0"),
        ("scan {123 13.6} {%s %f} a b\nputs $b", "13.6"),
        ("scan -2.5 %f q\nputs $q", "-2.5"),
        ("scan Inf %g d\nputs $d", "Inf"),
    ],
)
def test_scan_charset_and_float(source: str, expected: str) -> None:
    assert _run(source) == expected
