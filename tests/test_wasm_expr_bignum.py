"""WASM bignum (i128) end-to-end tests for ``expr``.

Compile small ``expr``-driven snippets through the AOT pipeline,
execute under wasmtime, and compare ``puts`` output against the
reference values produced by C Tcl 9.0.3.  The cases are lifted
directly from ``tmp/tcl9.0.3/tests/expr.test`` and
``tests/expr-old.test``; each comment names the upstream test ID
the case mirrors.

Stage 1 of bignum support promotes overflow only as far as i128
(see ``runtime/zig/valtypes/tcl_bignum.zig``).  Cases that need
more than 128 bits (e.g. upstream ``expr-old-36.16`` with its
152-bit hex literal) are deliberately *not* listed here — Stage 2
will lift the cap via libtommath and grow this file to match.
"""

from __future__ import annotations

import pytest

from tests.test_wasm_real_tcl import _run_tcl_for_stdout


def _expr_puts(expr_body: str) -> str:
    """Return the stdout produced by ``puts [expr {<expr_body>}]``."""
    src = f"puts [expr {{{expr_body}}}]\n"
    ok, stdout, err = _run_tcl_for_stdout(src)
    if not ok:
        pytest.fail(f"WASM compile/run failed for {expr_body!r}: {err}")
    return stdout.rstrip("\n")


# expr-32.{3..9} — Bug 1585704 + bignum regression cluster.
# Tests that ``1 << 63`` (which overflows i64) is treated as the
# mathematically correct ``9223372036854775808`` rather than
# silently wrapping to ``-9223372036854775808``.


class TestBug1585704:
    """Upstream ``expr-32.{3..9}``: ``1 % (1<<63)`` and friends."""

    def test_expr_32_3(self) -> None:
        # expr 1%(1<<63) -> 1
        assert _expr_puts("1%(1<<63)") == "1"

    def test_expr_32_4(self) -> None:
        # expr -1%(1<<63) -> (1<<63)-1 = 9223372036854775807
        assert _expr_puts("-1%(1<<63)") == "9223372036854775807"

    def test_expr_32_5(self) -> None:
        # expr (1<<32)%(1<<63) -> 1<<32 = 4294967296
        assert _expr_puts("(1<<32)%(1<<63)") == "4294967296"

    def test_expr_32_6(self) -> None:
        # expr -(1<<32)%(1<<63) -> (1<<63)-(1<<32) = 9223372032559808512
        assert _expr_puts("-(1<<32)%(1<<63)") == "9223372032559808512"

    def test_expr_32_7(self) -> None:
        # expr {0%(1<<63)} -> 0
        assert _expr_puts("0%(1<<63)") == "0"

    def test_expr_32_8(self) -> None:
        # expr {0%-(1<<63)} -> 0
        assert _expr_puts("0%-(1<<63)") == "0"

    def test_expr_32_9(self) -> None:
        # expr {0%-(1+(1<<63))} -> 0
        assert _expr_puts("0%-(1+(1<<63))") == "0"


# Direct shift-and-render checks.  These don't appear verbatim in
# expr.test but are the trigger that drives 32.3-9 — pinning them
# separately means a regression in shifting reports first.
class TestShiftPromotion:
    """Bit-shift overflow promotes to bignum and renders correctly."""

    def test_one_shl_63(self) -> None:
        assert _expr_puts("1<<63") == "9223372036854775808"

    def test_one_shl_64(self) -> None:
        assert _expr_puts("1<<64") == "18446744073709551616"

    def test_one_shl_100(self) -> None:
        assert _expr_puts("1<<100") == "1267650600228229401496703205376"

    def test_neg_one_shl_30(self) -> None:
        # ``-1 << 30`` fits in i64 and stays an int.
        assert _expr_puts("(-1)<<30") == "-1073741824"


# i64 boundary arithmetic — covers the +1 promotion at the i64::MAX
# / i64::MIN edges that drive ``expr-old-36.11`` and the
# ``Tcl_NewWideIntObj`` round-trip cases in expr-33.
class TestI64BoundaryArithmetic:
    """``maxInt(i64) + 1`` and ``minInt(i64) - 1`` promote correctly."""

    def test_max_wide_plus_one(self) -> None:
        # ``9223372036854775807 + 1`` overflows i64; bignum gives
        # the next integer.  Mirrors expr-33.3's ``wide(maxwide+1) < 0``
        # check (Tcl 9.0 returns 0 because the result is positive
        # bignum, not a negative wrap).
        assert _expr_puts("9223372036854775807 + 1") == "9223372036854775808"

    def test_min_wide_minus_one(self) -> None:
        assert _expr_puts("-9223372036854775808 - 1") == "-9223372036854775809"

    def test_neg_min_wide(self) -> None:
        # ``-(-9223372036854775808)`` = ``9223372036854775808``;
        # the i64 negate would overflow.
        assert _expr_puts("-(-9223372036854775808)") == "9223372036854775808"


# Multiplication-driven overflow.  These cover the ``expr-23.x``
# INST_EXPON / repeated-multiply cluster at the i64 boundary.
class TestMultiplicationPromotion:
    """``2^32 * 2^32`` etc. promote to bignum."""

    def test_2pow32_squared(self) -> None:
        # ``4294967296 * 4294967296`` = ``18446744073709551616``
        assert _expr_puts("4294967296 * 4294967296") == "18446744073709551616"

    def test_3_times_max_wide(self) -> None:
        assert _expr_puts("3 * 9223372036854775807") == "27670116110564327421"

    def test_minus_one_times_min_wide(self) -> None:
        assert _expr_puts("-1 * -9223372036854775808") == "9223372036854775808"


# Round-trip through arithmetic and stringification.  The most
# important guarantee: a bignum value written by one expression
# parses back as a bignum when re-read by the next expression.
class TestRoundTrip:
    """A bignum written by ``set`` round-trips through ``expr``."""

    def test_set_and_re_use(self) -> None:
        # Cumulative shift then mod — both sides go through string
        # rep at the ``set``/``$x`` boundary.
        src = "set x [expr {1 << 65}]\nputs [expr {$x % 7}]\n"
        ok, stdout, err = _run_tcl_for_stdout(src)
        if not ok:
            pytest.fail(f"WASM compile/run failed: {err}")
        # ``2^65 % 7`` = ``32768 * 1125899906842624 % 7``.
        # Compute it via Python for the expectation.
        expected = (1 << 65) % 7
        assert stdout.rstrip("\n") == str(expected)

    def test_set_explicit_bignum(self) -> None:
        # The literal exceeds i64 — promotes via the string-parse path.
        src = "set x 18446744073709551616\nputs [expr {$x + 1}]\n"
        ok, stdout, err = _run_tcl_for_stdout(src)
        if not ok:
            pytest.fail(f"WASM compile/run failed: {err}")
        assert stdout.rstrip("\n") == "18446744073709551617"
