"""WASM ``format`` integer width / length-modifier handling.

Tcl 9 renders an integer conversion with **no** length modifier as its
low 32 bits (``format %d 7810179016327718216`` -> ``1819043144``,
``format %d 4294967296`` -> ``0``); the ``l`` / ``ll`` family keeps the
full 64-bit wide value, and ``h`` / ``hh`` narrow to short / signed char.

Previously the unsigned conversions (``%x`` / ``%u`` / ``%o`` / ``%b``)
truncated to 32 bits unconditionally while the signed ones (``%d`` /
``%i``) never truncated — so ``%d`` of a wide value printed all 19 digits
and ``%lx`` dropped the high word.  ``emit_int`` now clamps to 32 bits
only when no ``l`` modifier is present, for every base.  Fixes
format-17.1 and util-17.1.

See ``emit_int`` in ``runtime/zig/valtypes/tcl_format.zig``.
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
    "spec,expected",
    [
        # No modifier -> low 32 bits (format-17.1).
        ("%d 7810179016327718216", "1819043144"),
        ("%d 4294967296", "0"),
        ("%x 7810179016327718216", "6c6c6548"),
        ("%u 7810179016327718216", "1819043144"),
        # ``l`` / ``ll`` keep the full 64-bit wide value.
        ("%ld 7810179016327718216", "7810179016327718216"),
        ("%lx 7810179016327718216", "6c63546f6c6c6548"),
        ("%lu 7810179016327718216", "7810179016327718216"),
        # Negative / unsigned two's-complement at 32 bits.
        ("%d -1", "-1"),
        ("%x -12", "fffffff4"),
        ("%u -12", "4294967284"),
        # ``h`` / ``hh`` narrowing is preserved.
        ("%hd 0xffff", "-1"),
        ("%hx 0x10fff", "fff"),
        # Small values / alt-form binary unchanged.
        ("%b 5", "101"),
        ("%#b 5", "0b101"),
    ],
)
def test_format_integer_width(spec: str, expected: str) -> None:
    assert _run(f"puts [format {spec}]") == expected
