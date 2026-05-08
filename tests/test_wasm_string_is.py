"""WASM ``string is`` command tests.

Pins the contracts added for tier-5 ``string is`` parity:

* canonical class disambiguation (``bad class`` / ``ambiguous class``)
* ``-strict`` semantics for empty input
* ``string is dict`` with even-element-count check
* ``string is true`` / ``string is false`` / ``string is wordchar``
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
    "expr,expected",
    [
        # Empty + strict.
        ("string is alpha {}", "1"),
        ("string is alpha -strict {}", "0"),
        # ``dict`` class.
        ("string is dict {a b c d}", "1"),
        ("string is dict {a b c}", "0"),
        ("string is dict {}", "1"),
        ("string is dict a", "0"),
        # ``true`` / ``false``.
        ("string is true 1", "1"),
        ("string is true yes", "1"),
        ("string is true 0", "0"),
        ("string is false 0", "1"),
        ("string is false off", "1"),
        ("string is false 1", "0"),
        # ``wordchar`` — alnum + underscore.
        ("string is wordchar abc_def", "1"),
        ("string is wordchar abc-def", "0"),
        # ``entier`` (alias for integer).
        ("string is entier 42", "1"),
        ("string is entier 4.5", "0"),
        # ``list`` accepts well-formed strings.
        ("string is list {a b c}", "1"),
        ('string is list "a \\{b c"', "0"),
    ],
)
def test_string_is_basic(expr: str, expected: str) -> None:
    out = _run(f"puts -nonewline [{expr}]")
    assert out == expected, f"{expr!r} → {out!r}, want {expected!r}"


def test_string_is_bad_class() -> None:
    out = _run("puts [catch {string is bogus foo} msg]; puts $msg")
    lines = out.strip().splitlines()
    assert lines[0] == "1"
    assert lines[1].startswith('bad class "bogus":')
    assert "alnum, alpha, ascii" in lines[1]


def test_string_is_ambiguous_class() -> None:
    out = _run("puts [catch {string is al foo} msg]; puts $msg")
    lines = out.strip().splitlines()
    assert lines[0] == "1"
    assert lines[1].startswith('ambiguous class "al":')


def test_string_is_failindex_dict_odd() -> None:
    out = _run(
        "set x {}; puts [string is dict -failindex x {a b c}]; puts $x"
    )
    lines = out.strip().splitlines()
    assert lines[0] == "0"
    assert lines[1] == "-1"
