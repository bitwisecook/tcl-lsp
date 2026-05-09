"""WASM ``format`` command tests.

Pins the contracts added for tier-5 ``format`` parity:

* ``#`` flag prefixes (``0x`` / ``0X`` / ``0o`` / ``0b`` / ``0d``)
* ``%b`` (binary) and ``%u`` (unsigned) conversions
* Negative-as-32-bit-unsigned semantics for ``%x`` / ``%X`` / ``%o``
  / ``%b`` / ``%u``
* ``%c`` width handling and full-Unicode UTF-8 encoding
* ``h`` length modifier (truncate to short before formatting)
* ``bad field specifier "X"`` wording for unknown conversion verbs
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run(source: str) -> str:
    wasm = _compile_tcl(source)
    _, stdout = _run_wasm(wasm, capture_stdout=True)
    return stdout


@pytest.mark.parametrize(
    "fmt,args,expected",
    [
        # ``#`` flag — alternate-form prefixes.
        ("%#x", ["255"], "0xff"),
        # Tcl 9 emits the lower-case ``0x`` prefix even for ``%#X``
        # (digits only switch case) — see test format-1.2.
        ("%#X", ["255"], "0xFF"),
        ("%#o", ["8"], "0o10"),
        ("%#b", ["5"], "0b101"),
        ("%#d", ["5"], "0d5"),
        # Zero suppresses every alt-form prefix.
        ("%#x", ["0"], "0"),
        ("%#o", ["0"], "0"),
        ("%#d", ["0"], "0"),
        # Width with ``#`` flag.
        ("%#5x", ["6"], "  0x6"),
        ("%#-5x", ["6"], "0x6  "),
        # ``%b`` binary.
        ("%b", ["5"], "101"),
        # ``%u`` unsigned of negative.
        ("%u", ["-1"], "4294967295"),
        ("%u", ["-12"], "4294967284"),
        # ``%x`` / ``%X`` / ``%o`` / ``%b`` of negative — 32-bit unsigned wrap.
        ("%x", ["-12"], "fffffff4"),
        ("%X", ["-12"], "FFFFFFF4"),
        ("%o", ["-1"], "37777777777"),
        ("%b", ["-1"], "11111111111111111111111111111111"),
        # ``%c`` with high code points → multi-byte UTF-8.
        ("%c", ["0xA2"], "¢"),
        ("%c", ["0x4E4E"], "乎"),
        # ``%c`` with width.
        ("%6c", ["65"], "     A"),
        ("%-6c", ["65"], "A     "),
        # ``h`` length modifier.
        ("%hd", ["0xffff"], "-1"),
        ("%hx", ["0x10fff"], "fff"),
        ("%hu", ["0x100000000"], "0"),
        # ``%05s`` zero-pad on string.
        ("%05s", ["a"], "0000a"),
        ("%05c", ["61"], "0000="),
        # Space-flag for sign on positive.
        ("% d", ["10"], " 10"),
        ("% d", ["-10"], "-10"),
    ],
)
def test_format_specifiers(fmt: str, args: list[str], expected: str) -> None:
    arg_str = " ".join(args)
    out = _run(f'puts -nonewline [format "{fmt}" {arg_str}]')
    assert out == expected, f"format {fmt} {arg_str} → {out!r}, want {expected!r}"


def test_format_bad_field_specifier() -> None:
    out = _run('puts [catch {format "%r" 5} msg]; puts $msg')
    lines = out.strip().splitlines()
    assert lines[0] == "1"
    assert 'bad field specifier "r"' in lines[1]
