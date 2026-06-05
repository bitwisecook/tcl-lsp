"""WASM ``binary`` command — wide-int encoding and the unsigned modifier.

Two contracts in ``runtime/zig/cmds/binary.zig``:

* ``binary format w`` / ``W`` packs the integer's low 64 bits regardless
  of sign or magnitude.  It previously *saturated* values >= 2^63 to the
  i64 maximum, so ``binary format w 0x8000000000000000`` produced the
  bytes for ``0x7fff…ffff`` — which, re-read as an IEEE-754 double, is
  NaN.  That broke the ``convertDouble`` helper behind every negative
  case in util-10.*.
* ``binary scan`` honours the Tcl 8.5+ ``u`` modifier (``cu`` / ``su`` /
  ``iu`` / ``wu``), scanning the value as unsigned.  A high-bit-set
  ``wu`` exceeds i64 and is returned as its unsigned decimal string.
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run(body: str) -> str:
    _, stdout = _run_wasm(_compile_tcl(body), capture_stdout=True)
    return stdout.rstrip("\n")


@pytest.mark.parametrize(
    "hexbits,expected",
    [
        ("0x7ef754e31cd072da", "4e+303"),
        ("0x8000000000000000", "-0.0"),
        ("0xd08afcef51f0fb5f", "-1e+80"),
        ("0xfee754e31cd072da", "-2e+303"),
        ("0xad3c0794d9d40e96", "-8.6e-91"),
    ],
)
def test_binary_format_w_wide_double_roundtrip(hexbits: str, expected: str) -> None:
    # convertDouble: reinterpret a 64-bit pattern as a double via binary.
    body = (
        "proc convertDouble {x} { binary scan [binary format w $x] d r; return $r }\n"
        f"puts [convertDouble {hexbits}]\n"
    )
    assert _run(body) == expected


@pytest.mark.parametrize(
    "count",
    [
        # 0x7fffffff: fits in u32 but ``n*10`` overflowed mid-parse.
        "2147483647",
        # 0x10000000: in range, exercises the same accumulator.
        "268435456",
        # 0x100000000: exceeds u32 entirely.
        "4294967296",
        # 0x10000000000000000: exceeds u64 — C's strtoull sets ERANGE.
        "18446744073709551616",
    ],
)
def test_binary_scan_count_overflow_does_not_trap(count: str) -> None:
    # GetFormatSpec count overflow (binary-37.10..37.13): a count larger
    # than the data must saturate and make ``binary scan`` return 0 (the
    # variable is left unset), never panic with an integer overflow trap
    # that truncates the whole script.  ``x`` is a single byte, so any of
    # these counts is far past the end.
    body = f"puts [binary scan x a{count} r]\n"
    assert _run(body) == "0"


@pytest.mark.parametrize(
    "body,expected",
    [
        # 'a' over-count: explicit count past the data stops the scan
        # (goto done) — result 0, the variable is not written.
        ("set r unset\nputs [binary scan abc a5 r]\nputs $r", "0\nunset"),
        # 'a' exact / short counts still read normally.
        ("binary scan abcdef a3 r\nputs $r", "abc"),
        ("binary scan abcdef a* r\nputs $r", "abcdef"),
        # x* skips to the end: the trailing a3 finds no bytes (goto done),
        # so the scan returns 1 and the second variable keeps its old value.
        ("set s foo\nputs [binary scan abcdefg a3x*a3 r s]\nputs $s", "1\nfoo"),
        # X* rewinds to the start, so a2 re-reads from the front.
        ("binary scan abcdef a3X*a2 r s\nputs [list $r $s]", "abc ab"),
        # @* seeks to the end of the data; the following a1 finds nothing.
        ("set s foo\nputs [binary scan abcdef a2@*a1 r s]\nputs $s", "1\nfoo"),
    ],
)
def test_binary_scan_seek_and_overcount(body: str, expected: str) -> None:
    assert _run(body) == expected


@pytest.mark.parametrize(
    "body,expected",
    [
        ("binary scan [binary format c 200] cu x\nputs $x", "200"),
        ("binary scan [binary format c 200] c x\nputs $x", "-56"),
        ("binary scan [binary format s 0xFFFF] su x\nputs $x", "65535"),
        ("binary scan [binary format i -1] iu x\nputs $x", "4294967295"),
        (
            "binary scan [binary format w 0x8000000000000000] wu x\nputs $x",
            "9223372036854775808",
        ),
        ("binary scan [binary format c3 {200 201 202}] cu3 x\nputs $x", "200 201 202"),
    ],
)
def test_binary_scan_unsigned_modifier(body: str, expected: str) -> None:
    assert _run(body) == expected
