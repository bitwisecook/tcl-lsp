"""WASM ``Tcl_GetIntForIndex`` parsing — whitespace, signed offsets, ``0d``.

``string index`` (and every index-taking command, via the shared
``resolve_list_index``) must parse the Tcl 9 index grammar:

* leading / trailing whitespace is skipped (``{ 0 }`` / ``{end-1 }``);
* an offset after ``end±`` / between ``±`` may itself be signed
  (``end+-1`` = end + (-1), ``end--1`` = end - (-1), ``-1--2`` = 1);
* integer literals accept the ``0d`` explicit-decimal radix alongside
  ``0x`` / ``0o`` / ``0b``;
* out-of-range / overflowing indices yield the empty string.

Regression guard for the util-9.* cluster.
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run(body: str) -> str:
    _, stdout = _run_wasm(_compile_tcl(body), capture_stdout=True)
    return stdout.rstrip("\n")


@pytest.mark.parametrize(
    "expr,expected",
    [
        ("string index abcd { 0 }", "a"),
        ("string index abcd { 3 }", "d"),
        ("string index abcd {end-1 }", "c"),
        ("string index abcd { 0x0 }", "a"),
        ("string index abcd { 01 }", "b"),
        ("string index abcd { 0d0 }", "a"),
        ("string index abcdefghijk 0d10", "k"),
        ("string index abcd { -1+2 }", "b"),
        # signed offsets after the operator
        ("string index abcd end+-1", "c"),
        ("string index abcd -1--2", "b"),
        # out-of-range / overflow -> empty string
        ("string index abcd end--1", ""),
        ("string index abcd -0x10000000000000000", ""),
        ("string index abcd end+-0x10000000000000000", ""),
    ],
)
def test_string_index_grammar(expr: str, expected: str) -> None:
    assert _run(f"puts [{expr}]") == expected
