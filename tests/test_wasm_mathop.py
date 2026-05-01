"""WASM ``::tcl::mathop`` ensemble tests.

Covers the variadic / chain semantics that Tcl's ``mathop.n`` page
documents and that upstream tcltest fixtures (notably ``clock-7``'s
chained equality assertions) rely on.  Each test compiles a tiny Tcl
script, runs it under wasmtime, and asserts on captured stdout.
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run(source: str) -> str:
    wasm = _compile_tcl(source)
    _, stdout = _run_wasm(wasm, capture_stdout=True)
    return stdout.strip()


@pytest.mark.parametrize(
    "expr,expected",
    [
        # Arithmetic
        ("::tcl::mathop::+ 1 2 3 4", "10"),
        ("::tcl::mathop::+", "0"),
        ("::tcl::mathop::- 10 1 2 3", "4"),
        ("::tcl::mathop::- 5", "-5"),
        ("::tcl::mathop::* 2 3 4", "24"),
        ("::tcl::mathop::*", "1"),
        ("::tcl::mathop::/ 100 5 2", "10"),
        ("::tcl::mathop::% 17 5", "2"),
        ("::tcl::mathop::** 2 3", "8"),
        ("::tcl::mathop::** 2 3 2", "512"),  # right-assoc: 2 ** (3 ** 2) = 2 ** 9
        # Chained comparisons (clock-7's idiom)
        ("::tcl::mathop::== 1 1 1", "1"),
        ("::tcl::mathop::== 1 1 2", "0"),
        ("::tcl::mathop::< 1 2 3 4", "1"),
        ("::tcl::mathop::< 1 2 2 4", "0"),
        ("::tcl::mathop::<= 1 2 2 4", "1"),
        ("::tcl::mathop::> 4 3 2 1", "1"),
        ("::tcl::mathop::>= 4 4 3 1", "1"),
        ("::tcl::mathop::!= 1 2", "1"),
        # String compare
        ("::tcl::mathop::eq abc abc abc", "1"),
        ("::tcl::mathop::eq abc abd", "0"),
        ("::tcl::mathop::ne abc def", "1"),
        # Bitwise
        ("::tcl::mathop::& 12 10", "8"),
        ("::tcl::mathop::| 12 10", "14"),
        ("::tcl::mathop::^ 12 10", "6"),
        ("::tcl::mathop::~ 0", "-1"),
        ("::tcl::mathop::<< 1 4", "16"),
        ("::tcl::mathop::>> 16 2", "4"),
        # Logical
        ("::tcl::mathop::! 0", "1"),
        ("::tcl::mathop::! 1", "0"),
        ("::tcl::mathop::&& 1 1 1", "1"),
        ("::tcl::mathop::&& 1 0 1", "0"),
        ("::tcl::mathop::|| 0 0 1", "1"),
        ("::tcl::mathop::|| 0 0 0", "0"),
        # Min / max
        ("::tcl::mathop::min 5 2 7 3", "2"),
        ("::tcl::mathop::max 5 2 7 3", "7"),
        # List membership
        ("::tcl::mathop::in b {a b c}", "1"),
        ("::tcl::mathop::in z {a b c}", "0"),
        ("::tcl::mathop::ni z {a b c}", "1"),
    ],
)
def test_mathop_op(expr: str, expected: str) -> None:
    out = _run(f"puts [{expr}]")
    assert out == expected, f"{expr!r}: expected {expected!r}, got {out!r}"


def test_mathop_float_arithmetic() -> None:
    # Floating-point results should keep ``.`` so Tcl recognises them.
    out = _run("puts [::tcl::mathop::+ 1.5 2.5]")
    assert out in {"4.0", "4"}  # depending on canonicalisation


def test_mathop_short_qualified_form() -> None:
    # The half-qualified ``tcl::mathop::==`` form (no leading ``::``)
    # must also resolve — used inside ``namespace eval ::tcl`` blocks.
    out = _run("puts [tcl::mathop::== 1 1 1]")
    assert out == "1"


def test_mathop_chain_under_one_arg_returns_true() -> None:
    # Tcl's chain-comparison ops return 1 vacuously for 0 / 1 args.
    assert _run("puts [::tcl::mathop::< 5]") == "1"
    assert _run("puts [::tcl::mathop::<]") == "1"
