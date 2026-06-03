"""WASM ``format`` — out-of-range ``%c`` and bignum args in multi-arg calls.

Pins two contracts in ``runtime/zig/valtypes/tcl_format.zig``:

* ``%c`` mirrors ``FormatObjCmd``'s ``case 'c'``: the argument is taken as
  a 32-bit int and any value whose UNSIGNED form exceeds U+10FFFF maps to
  U+FFFD (the replacement character) — WITHOUT first masking to 21 bits.
  Masking first let ``format %c 0x10000041`` (Tcl's internal TCL_COMBINE
  bit plus ``'A'``) wrap back to a valid code point instead of U+FFFD
  (format-8.28).

* A bignum argument survives a ``format`` call with more than three args.
  The >3-arg path builds a Tcl list and re-materialises each element as a
  plain string, dropping the operand's ``TYPE_BIGNUM`` rep; ``emit_int``
  now re-parses such a string operand as a bignum for the render (without
  mutating the obj, so a ``%s`` of the same operand keeps its original
  spelling).  Without it ``2**100`` truncated to its low 64 bits and
  rendered as ``0`` (format-1.12).
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402

_P100 = "1" + "0" * 100  # 2**100 in binary
_P100_DEC = str(2**100)


def _run(source: str) -> str:
    wasm = _compile_tcl(source)
    _, stdout = _run_wasm(wasm, capture_stdout=True)
    return stdout.rstrip("\n")


@pytest.mark.parametrize(
    "source,expected",
    [
        # format-8.28: 0x10000041 carries the internal TCL_COMBINE bit;
        # its unsigned value exceeds U+10FFFF -> U+FFFD (scan reports the
        # code point so the control char never reaches the terminal).
        ("puts [scan [format %c 0x10000041] %c]", "65533"),
        # A value just past the last valid code point also folds to FFFD.
        ("puts [scan [format %c 0x110000] %c]", "65533"),
        # In-range code points are unaffected.
        ("puts [scan [format %c 65] %c]", "65"),
        ("puts [scan [format %c 0x10FFFF] %c]", "1114111"),
    ],
)
def test_format_char_out_of_range(source: str, expected: str) -> None:
    assert _run(source) == expected


@pytest.mark.parametrize(
    "source,expected",
    [
        # format-1.12: the bignum (4th arg, %llb) renders in full.
        (
            'puts [format "%b %#b %#b %llb" 5 0 5 [expr {2**100}]]',
            f"101 0 0b101 {_P100}",
        ),
        # The bignum survives in any position of a >3-arg call.
        ('puts [format "%d %d %lld" 7 8 [expr {2**100}]]', f"7 8 {_P100_DEC}"),
        ('puts [format "%lld %d %d" [expr {2**100}] 7 8]', f"{_P100_DEC} 7 8"),
        # %llx of the bignum.
        ('puts [format "%d %d %llx" 1 2 [expr {2**100}]]', "1 2 " + "1" + "0" * 25),
        # Negative bignum.
        (
            'puts [format "%d %d %lld" 1 2 [expr {-(2**100)}]]',
            f"1 2 -{_P100_DEC}",
        ),
    ],
)
def test_format_bignum_multi_arg(source: str, expected: str) -> None:
    assert _run(source) == expected


@pytest.mark.parametrize(
    "source,expected",
    [
        # A %s argument that merely looks numeric keeps its exact spelling
        # — emit_int's bignum re-parse must not touch the %s path.
        ('puts [format "%d %d %s" 1 2 0x10]', "1 2 0x10"),
        ('puts [format "%d %d %s" 1 2 007]', "1 2 007"),
        # Small integer args (well within i64) still format normally.
        ('puts [format "%d %d %d" 1 2 5]', "1 2 5"),
    ],
)
def test_format_multi_arg_string_safety(source: str, expected: str) -> None:
    assert _run(source) == expected
