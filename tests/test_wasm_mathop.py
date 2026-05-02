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


def test_mathop_div_unary_returns_floating_reciprocal() -> None:
    # ``mathop.n``: with one argument, ``/`` returns ``1.0/x``.  An
    # integer input still produces a *float* — ``[/ 5]`` is ``0.2``,
    # not ``0`` (integer floor).  Codex P1 review caught the
    # earlier integer-floor implementation.
    assert _run("puts [::tcl::mathop::/ 5]") == "0.2"
    assert _run("puts [::tcl::mathop::/ 4]") == "0.25"
    # Unary on a float input keeps the float path.
    assert _run("puts [::tcl::mathop::/ 2.0]") == "0.5"
    # Multi-arg integer division still floors as before.
    assert _run("puts [::tcl::mathop::/ 100 5 2]") == "10"


def test_mathop_compare_falls_back_to_string_for_non_numeric() -> None:
    # ``mathop`` comparison ops compare numerically when both args
    # are numeric, otherwise lexically.  Without this fallback,
    # ``obj_get_int("a")`` collapsed every non-numeric string to
    # ``0`` and every comparison returned vacuously true.
    assert _run("puts [::tcl::mathop::== a b]") == "0"
    assert _run("puts [::tcl::mathop::== abc abc]") == "1"
    assert _run("puts [::tcl::mathop::!= a b]") == "1"
    assert _run("puts [::tcl::mathop::< a b]") == "1"
    assert _run("puts [::tcl::mathop::> b a]") == "1"
    assert _run("puts [::tcl::mathop::<= ab ac]") == "1"
    assert _run("puts [::tcl::mathop::>= ab ab]") == "1"
    # Mixed numeric / string falls back to string compare too —
    # ``"10"`` lexically precedes ``"9"`` because ``'1' < '9'``.
    assert _run("puts [::tcl::mathop::< 10 9z]") == "1"
    # Pure numeric still goes through the numeric path.
    assert _run("puts [::tcl::mathop::== 1 1.0]") == "1"
    assert _run("puts [::tcl::mathop::< 9 10]") == "1"


def test_mathop_div_negative_truncates_toward_zero() -> None:
    # ``::tcl::mathop::/`` integer division must use truncation
    # toward zero (``@divTrunc``) to match ``tcl_arith_div`` and
    # ``expr {a / b}``.  An earlier ``@divFloor`` produced ``-3``
    # instead of the Tcl-correct ``-2`` (Copilot review).
    assert _run("puts [::tcl::mathop::/ 5 -2]") == "-2"
    assert _run("puts [::tcl::mathop::/ -5 2]") == "-2"
    assert _run("puts [::tcl::mathop::/ -7 -2]") == "3"


def test_mathop_mod_uses_floor_sign_of_divisor() -> None:
    # ``::tcl::mathop::%`` matches Tcl 9 floor-mod semantics
    # (``tclExecute.c`` INST_MOD): the result has the *divisor's*
    # sign.  The pre-bignum ``@rem``-sign-of-dividend path made
    # ``[% -7 3]`` return ``-1``, miscompiled w.r.t. C Tcl which
    # returns ``2``.  ``tcl_arith_mod`` (and thus mathop's ``%``)
    # now apply the ``r += b when signs differ`` fixup upstream
    # uses, so all three of these match the C Tcl reference.
    assert _run("puts [::tcl::mathop::% -7 3]") == "2"
    assert _run("puts [::tcl::mathop::% 7 -3]") == "-2"
    assert _run("puts [::tcl::mathop::% -7 -3]") == "-1"


def test_mathop_pow_negative_exponent_returns_float() -> None:
    # ``::tcl::mathop::** 2 -1`` must produce a fractional ``0.5``
    # rather than the integer-pow ``0``.  Negative exponents force
    # the float pathway (Copilot review).
    out = _run("puts [::tcl::mathop::** 2 -1]")
    assert out == "0.5"
    out = _run("puts [::tcl::mathop::** 4 -2]")
    assert out == "0.0625"
    # Positive integer exponents stay on the integer path.
    assert _run("puts [::tcl::mathop::** 3 4]") == "81"


def test_mathop_logical_boolean_keywords() -> None:
    # ``!`` / ``&&`` / ``||`` must accept Tcl's full boolean
    # keyword set (``true`` / ``false`` / ``yes`` / ``no`` /
    # ``on`` / ``off`` and their non-ambiguous prefixes), not the
    # earlier 1-or-2-character prefix heuristic that misclassified
    # strings like ``tree`` / ``frame`` as boolean (Copilot review).
    assert _run("puts [::tcl::mathop::! true]") == "0"
    assert _run("puts [::tcl::mathop::! yes]") == "0"
    assert _run("puts [::tcl::mathop::! on]") == "0"
    assert _run("puts [::tcl::mathop::! false]") == "1"
    assert _run("puts [::tcl::mathop::! no]") == "1"
    assert _run("puts [::tcl::mathop::! off]") == "1"
    # Ambiguous prefixes that LOOK like a boolean keyword
    # — ``"tree"`` (4 chars starting with ``tr``) used to slip
    # through as truthy.  Now ``try_parse_bool`` rejects it and
    # we fall through to the numeric coerce (which yields 0
    # because the string isn't an integer either, so ``! tree``
    # returns 1).
    assert _run("puts [::tcl::mathop::! tree]") == "1"
    assert _run("puts [::tcl::mathop::! frame]") == "1"
    # Numeric truth still works.
    assert _run("puts [::tcl::mathop::&& 1 1 1]") == "1"
    assert _run("puts [::tcl::mathop::|| 0 0 1]") == "1"
