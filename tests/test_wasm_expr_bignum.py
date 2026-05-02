"""WASM bignum end-to-end tests for ``expr``.

Compile small ``expr``-driven snippets through the AOT pipeline,
execute under wasmtime, and compare ``puts`` output against the
reference values produced by C Tcl 9.0.3.  The cases are lifted
directly from ``tmp/tcl9.0.3/tests/expr.test`` and
``tests/expr-old.test``; each comment names the upstream test ID
the case mirrors.

Stage 1 of bignum support promoted overflow as far as i128.
Stage 2 (current) lifts the cap to true arbitrary precision via
``std.math.big.int.Managed`` from Zig's stdlib — see
``runtime/zig/valtypes/tcl_bignum.zig``.  The
``TestArbitraryPrecision`` class below pins the cases that
require >i128 magnitude (upstream ``expr-old-36.{11,14,16}``).
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


# Arbitrary precision (Stage 2, ``std.math.big.int.Managed``).
#
# Stage 1's i128 cap saturated values past 38 decimal digits.  Stage 2
# routes through Zig stdlib's Managed for true arbitrary precision —
# the cases below all need >128 bits and would have produced wrong
# answers under Stage 1.


class TestArbitraryPrecision:
    """Stage 2 — values that exceed i128."""

    def test_expr_old_36_11(self) -> None:
        # Upstream ``expr-old-36.11``:
        #     set x 665802003400000000000000
        #     expr {$x+1}
        # The literal is 80 bits — fits i128 — but the test catches
        # any silent precision loss at the i64→bignum boundary.
        src = (
            "set x 665802003400000000000000\nputs [expr {$x+1}]\n"
        )
        ok, stdout, err = _run_tcl_for_stdout(src)
        if not ok:
            pytest.fail(f"WASM compile/run failed: {err}")
        assert stdout.rstrip("\n") == "665802003400000000000001"

    def test_expr_old_36_14(self) -> None:
        # Upstream ``expr-old-36.14``:
        #     set x "123456789012345678901234567890 "
        #     expr {$x+1}
        # 30 decimal digits — fits i128, again pins the bignum
        # parse + add path.
        src = 'set x "123456789012345678901234567890 "\nputs [expr {$x+1}]\n'
        ok, stdout, err = _run_tcl_for_stdout(src)
        if not ok:
            pytest.fail(f"WASM compile/run failed: {err}")
        assert stdout.rstrip("\n") == "123456789012345678901234567891"

    def test_expr_old_36_16(self) -> None:
        # Upstream ``expr-old-36.16``:
        #     set x " 0xffffffffffffffffffffffffffffffffffffff  "
        #     expr {$x+1}
        # 38 hex digits = 152 bits — exceeds i128.  Stage 1 would
        # have saturated; Stage 2's Managed path produces the exact
        # value ``2^152`` decimal-formatted.
        src = (
            'set x " 0xffffffffffffffffffffffffffffffffffffff  "\n'
            "puts [expr {$x+1}]\n"
        )
        ok, stdout, err = _run_tcl_for_stdout(src)
        if not ok:
            pytest.fail(f"WASM compile/run failed: {err}")
        assert stdout.rstrip("\n") == "5708990770823839524233143877797980545530986496"

    def test_one_shl_200(self) -> None:
        # ``1 << 200`` — Stage 1 would have saturated past 1 << 127.
        src = "puts [expr {1 << 200}]\n"
        ok, stdout, err = _run_tcl_for_stdout(src)
        if not ok:
            pytest.fail(f"WASM compile/run failed: {err}")
        assert stdout.rstrip("\n") == (
            "1606938044258990275541962092341162602522202993782792835301376"
        )

    def test_pow_via_repeated_mul(self) -> None:
        # ``(2^100) * (2^100) * (2^100)`` = ``2^300`` — three
        # multiplications past the i128 boundary.  Stage 2 must
        # carry the precision through the chain.
        src = (
            "set a [expr {1 << 100}]\n"
            "puts [expr {$a * $a * $a}]\n"
        )
        ok, stdout, err = _run_tcl_for_stdout(src)
        if not ok:
            pytest.fail(f"WASM compile/run failed: {err}")
        # Compute expected via Python.
        assert stdout.rstrip("\n") == str((1 << 100) ** 3)

    def test_huge_mod(self) -> None:
        # ``-1 % (1 << 200)`` = ``(1 << 200) - 1`` — Tcl floor-mod
        # plus arbitrary-precision divisor.
        src = "puts [expr {-1 % (1 << 200)}]\n"
        ok, stdout, err = _run_tcl_for_stdout(src)
        if not ok:
            pytest.fail(f"WASM compile/run failed: {err}")
        assert stdout.rstrip("\n") == str((1 << 200) - 1)

    def test_huge_negate(self) -> None:
        # ``-(2^150)`` rendered as a negative 46-digit number.
        src = "puts [expr {-(1 << 150)}]\n"
        ok, stdout, err = _run_tcl_for_stdout(src)
        if not ok:
            pytest.fail(f"WASM compile/run failed: {err}")
        assert stdout.rstrip("\n") == str(-(1 << 150))

    def test_round_trip_via_set(self) -> None:
        # Set a 50-digit literal, multiply by itself, render — the
        # result should be the exact 100-digit decimal product.
        src = (
            "set x 12345678901234567890123456789012345678901234567890\n"
            "puts [expr {$x * $x}]\n"
        )
        ok, stdout, err = _run_tcl_for_stdout(src)
        if not ok:
            pytest.fail(f"WASM compile/run failed: {err}")
        x = 12345678901234567890123456789012345678901234567890
        assert stdout.rstrip("\n") == str(x * x)


# Comparison ops — ``<`` / ``>`` / ``<=`` / ``>=`` / ``==`` / ``!=``
# with bignum operands.  Stage 2's ``tcl_expr_order_cmp`` falls back
# to ``Managed.order`` so e.g. ``(1<<200) > (1<<100)`` returns 1
# rather than the lexicographic-string answer Stage 1 would have
# given.  These cases also pin the routing of ``==`` / ``!=`` through
# the bignum-aware path, since the inline ``i64.eq`` truncation made
# ``(1<<200) == (1<<200)`` equal-by-truncation.


class TestBignumComparison:
    """``<``, ``>``, ``==``, ``!=`` with bignum operands."""

    def test_int_lt_bignum(self) -> None:
        assert _expr_puts("99 < (1<<70)") == "1"

    def test_bignum_gt_int(self) -> None:
        assert _expr_puts("(1<<70) > 99") == "1"

    def test_bignum_lt_bignum_smaller_first(self) -> None:
        assert _expr_puts("(1<<100) < (1<<200)") == "1"

    def test_bignum_eq_bignum_same(self) -> None:
        assert _expr_puts("(1<<200) == (1<<200)") == "1"

    def test_bignum_ne_bignum_different(self) -> None:
        assert _expr_puts("(1<<200) != (1<<100)") == "1"

    def test_int_eq_float(self) -> None:
        # Pre-existing pattern that must keep working — ``1 == 1.0``
        # still returns true through the bignum-aware path (numeric
        # float compare is reached when at least one operand is a
        # float and the other is integer-shaped).
        assert _expr_puts("1 == 1.0") == "1"

    def test_negative_bignum_cmp(self) -> None:
        assert _expr_puts("-(1<<200) < 0") == "1"

    def test_bignum_ge_self(self) -> None:
        assert _expr_puts("(1<<200) >= (1<<200)") == "1"


# Bitwise ops with bignum operands.  Stage 2's ``Managed.bitAnd /
# bitOr / bitXor`` paths preserve precision; Stage 1 truncated each
# operand to its low 64 bits.


class TestBignumBitwise:
    """Bitwise ops on values exceeding i64."""

    def test_bignum_or_small(self) -> None:
        assert _expr_puts("(1<<128) | 7") == str((1 << 128) | 7)

    def test_bignum_and_self(self) -> None:
        assert _expr_puts("(1<<200) & (1<<200)") == str(1 << 200)

    def test_bignum_and_low_mask(self) -> None:
        # Mask off the high bits — Stage 1 returned 0 because the
        # low 64 bits of any large power-of-two are zero.  Stage 2
        # produces the correct ``1<<200 & full-bits-of-2^200`` =
        # ``1<<200``.
        assert _expr_puts("(1<<200) & ((1<<201) - 1)") == str(1 << 200)

    def test_bignum_xor(self) -> None:
        assert _expr_puts("(1<<128) ^ (1<<127)") == str((1 << 128) ^ (1 << 127))

    def test_bignum_bnot(self) -> None:
        # ``~x`` = ``-x - 1`` in two's complement.
        assert _expr_puts("~(1<<200)") == str(-(1 << 200) - 1)

    def test_bignum_rshift_recovers_value(self) -> None:
        # ``(1 << 200) >> 100 == 1 << 100`` — Stage 1 truncated the
        # bignum operand and returned 0.
        assert _expr_puts("(1<<200) >> 100") == str(1 << 100)

    def test_bignum_rshift_to_zero(self) -> None:
        assert _expr_puts("(1<<100) >> 200") == "0"


# Power (``**``) — promotes overflow to bignum via ``Managed.pow``
# instead of the i64 multiply loop the inline emitter used.


class TestBignumPower:
    """``**`` with results exceeding i64 / i128."""

    def test_2_pow_64(self) -> None:
        assert _expr_puts("2 ** 64") == str(1 << 64)

    def test_3_pow_100(self) -> None:
        assert _expr_puts("3 ** 100") == str(3 ** 100)

    def test_2_pow_200(self) -> None:
        assert _expr_puts("2 ** 200") == str(1 << 200)

    def test_neg_one_pow_even(self) -> None:
        assert _expr_puts("(-1) ** 100") == "1"

    def test_neg_one_pow_odd(self) -> None:
        assert _expr_puts("(-1) ** 99") == "-1"

    def test_bignum_base_pow_2(self) -> None:
        # ``(1 << 100) ** 2`` = ``1 << 200``
        assert _expr_puts("(1 << 100) ** 2") == str(1 << 200)


# ``int()`` / ``wide()`` / ``entier()`` — preserve bignum precision
# on the integer path; for floats truncate toward zero with full
# precision (``int(1e30)`` = the exact IEEE-754 integer view).


class TestBignumIntCoercion:
    """``int()`` and friends preserve bignum magnitude."""

    def test_int_of_bignum(self) -> None:
        assert _expr_puts("int(1<<200)") == str(1 << 200)

    def test_wide_of_64bit_hex(self) -> None:
        # Tcl 9 ``wide(0x8000000000000000)`` is the bignum
        # ``9223372036854775808`` — fits a wide on 64-bit Tcl, but
        # the literal value exceeds i64 max so we route through the
        # bignum path.
        assert _expr_puts("wide(0x8000000000000000)") == "9223372036854775808"

    def test_int_of_float_within_i128(self) -> None:
        # ``int(3.7)`` truncates toward zero.
        assert _expr_puts("int(3.7)") == "3"
        assert _expr_puts("int(-3.7)") == "-3"

    def test_int_of_huge_float(self) -> None:
        # Upstream ``expr-old-34.15``: ``round(1.0e30) ==
        # 1000000000000000019884624838656`` (the exact IEEE-754
        # double representation truncated to integer).  ``int`` and
        # ``round`` differ for negative non-integer floats but agree
        # for ``1e30`` (an integral float).
        assert _expr_puts("int(1.0e30)") == "1000000000000000019884624838656"


# ``::tcl::mathop::*`` prefix-form ops with bignum operands.  The
# legacy mathop dispatcher used ``obj_get_int`` for every operand,
# silently truncating bignum values to their low 64 bits.  Now
# routed through Managed.


class TestMathopBignum:
    """``::tcl::mathop::+`` etc. with values exceeding i64."""

    def _run(self, src: str) -> str:
        ok, out, err = _run_tcl_for_stdout(src)
        if not ok:
            pytest.fail(f"WASM compile/run failed: {err}")
        return out.rstrip("\n")

    def test_add_with_literal_bignum(self) -> None:
        # 3 + a 23-digit literal — exceeds i64.
        assert self._run("puts [::tcl::mathop::+ 1 2 99999999999999999999999]") == (
            "100000000000000000000002"
        )

    def test_add_with_var_bignum(self) -> None:
        src = (
            "set x [expr {1<<70}]\n"
            "puts [::tcl::mathop::+ 1 2 $x]\n"
        )
        assert self._run(src) == str(1 + 2 + (1 << 70))

    def test_lt_with_bignum(self) -> None:
        assert self._run("puts [::tcl::mathop::< 99 99999999999999999999999]") == "1"

    def test_eq_with_bignum(self) -> None:
        assert self._run(
            "set x 99999999999999999999999\n"
            "set y 99999999999999999999999\n"
            "puts [::tcl::mathop::== $x $y]"
        ) == "1"

    def test_band_with_bignum(self) -> None:
        # ``& (1<<200) ((1<<201)-1)`` = ``1<<200`` — Stage 1 truncated to 0.
        src = (
            "set hi [expr {1<<200}]\n"
            "set mask [expr {(1<<201)-1}]\n"
            "puts [::tcl::mathop::& $hi $mask]\n"
        )
        assert self._run(src) == str(1 << 200)

    def test_bor_with_bignum(self) -> None:
        src = "set hi [expr {1<<128}]\nputs [::tcl::mathop::| 7 $hi]\n"
        assert self._run(src) == str(7 | (1 << 128))

    def test_bnot_with_bignum(self) -> None:
        src = "set hi [expr {1<<200}]\nputs [::tcl::mathop::~ $hi]\n"
        assert self._run(src) == str(-((1 << 200) + 1))

    def test_pow_promotes_to_bignum(self) -> None:
        assert self._run("puts [::tcl::mathop::** 2 100]") == str(1 << 100)

    def test_lshift_promotes_to_bignum(self) -> None:
        assert self._run("puts [::tcl::mathop::<< 1 200]") == str(1 << 200)


# ``incr`` with bignum increment / variable.  The strict-int check
# now accepts TYPE_BIGNUM and bignum-shaped string literals; the
# add path promotes through Managed when either side is bignum.


class TestIncrBignum:
    """``incr`` with operands exceeding i64."""

    def _run(self, src: str) -> str:
        ok, out, err = _run_tcl_for_stdout(src)
        if not ok:
            pytest.fail(f"WASM compile/run failed: {err}")
        return out.rstrip("\n")

    def test_incr_promotes_on_overflow(self) -> None:
        # ``9223372036854775000 + 9000`` lands at ``i64::MAX + 1``,
        # which the legacy ``incr`` saturated.  Bignum promotion
        # lets the counter cross the boundary cleanly.
        src = "set x 9223372036854775000\nincr x 9000\nputs $x\n"
        assert self._run(src) == "9223372036854784000"

    def test_incr_crosses_i64_boundary(self) -> None:
        # Direct boundary-crossing case: ``i64::MAX + 1`` →
        # ``9223372036854775808`` (just past i64::MAX).
        src = "set x 9223372036854775807\nincr x 1\nputs $x\n"
        assert self._run(src) == "9223372036854775808"

    def test_incr_with_bignum_delta(self) -> None:
        src = "set x 0\nincr x [expr {1<<70}]\nputs $x\n"
        assert self._run(src) == str(1 << 70)

    def test_incr_with_bignum_var(self) -> None:
        # Counter that's already a bignum keeps accumulating.
        src = "set x [expr {1<<200}]\nincr x 5\nputs $x\n"
        assert self._run(src) == str((1 << 200) + 5)


# ``format %d / %x / %o`` with bignum operands.  Previously the
# format helper used ``obj_get_int`` which truncated bignum to i64;
# now routes through ``Managed.toString`` for the requested base.


class TestFormatBignum:
    """``format`` with arbitrary-precision integer operands."""

    def _run(self, src: str) -> str:
        ok, out, err = _run_tcl_for_stdout(src)
        if not ok:
            pytest.fail(f"WASM compile/run failed: {err}")
        return out.rstrip("\n")

    def test_format_d_bignum(self) -> None:
        src = "puts [format %d [expr {1<<200}]]"
        assert self._run(src) == str(1 << 200)

    def test_format_x_bignum(self) -> None:
        src = "puts [format %x [expr {1<<128}]]"
        assert self._run(src) == "100000000000000000000000000000000"

    def test_format_o_bignum(self) -> None:
        src = "puts [format %o [expr {1<<70}]]"
        # 1<<70 in octal: 1 followed by ceil(70/3) = 24 zeros (with adjustment).
        # Verify against Python.
        assert self._run(src) == oct(1 << 70)[2:]

    def test_format_negative_bignum(self) -> None:
        src = "set x [expr {-(1<<128)}]\nputs [format %d $x]\n"
        assert self._run(src) == str(-(1 << 128))


# Runtime ``expr`` evaluator (used by the ``expr`` *command* as
# opposed to the AOT-compiled ``expr {...}``) — Stage 2 swapped the
# legacy i64-only recursive descent (which didn't even recognise
# ``<<``) for a bignum-aware one in
# ``runtime/zig/interp/tcl_expr_eval.zig``.


class TestRuntimeExprEval:
    """``expr`` command (runtime path) with bignum-producing ops."""

    def _run(self, src: str) -> str:
        ok, out, err = _run_tcl_for_stdout(src)
        if not ok:
            pytest.fail(f"WASM compile/run failed: {err}")
        return out.rstrip("\n")

    def test_runtime_expr_shift(self) -> None:
        # ``[expr {1 << 70}]`` from a runtime-eval context.  The
        # legacy evaluator returned 1 (parser stopped at unknown
        # ``<<``); the bignum-aware evaluator gives the full
        # 22-digit value.
        assert self._run("puts [expr {1 << 70}]") == str(1 << 70)

    def test_runtime_expr_pow(self) -> None:
        assert self._run("puts [expr {2 ** 64}]") == str(1 << 64)

    def test_runtime_expr_bitwise_chain(self) -> None:
        # ``(1<<200) & ((1<<201) - 1) == 1<<200`` — bitwise on bignum.
        src = "puts [expr {(1<<200) & ((1<<201) - 1)}]"
        assert self._run(src) == str(1 << 200)

    def test_runtime_expr_compare_bignum(self) -> None:
        # ``99 < (1 << 70)`` — comparison must use numeric not lex.
        assert self._run("puts [expr {99 < (1 << 70)}]") == "1"

    def test_runtime_expr_mixed_arith(self) -> None:
        # Combination: shift, mod, bitwise.
        src = "puts [expr {((1 << 100) + (1 << 50)) % (1 << 64)}]"
        assert self._run(src) == str(((1 << 100) + (1 << 50)) % (1 << 64))


# ``scan %d / %x / %o`` — values that overflow i64 promote to
# TYPE_BIGNUM via the ``alloc_from_string`` path in
# ``runtime/zig/cmds/scan.zig``.  Pre-bignum the parse saturated
# at ``i64::MAX``; now the captured digit slice is fed to
# ``Managed.setString`` for the full magnitude.


class TestScanBignum:
    """``scan`` integer specifiers that overflow i64."""

    def _run(self, src: str) -> str:
        ok, out, err = _run_tcl_for_stdout(src)
        if not ok:
            pytest.fail(f"WASM compile/run failed: {err}")
        return out.rstrip("\n")

    def test_scan_d_bignum(self) -> None:
        src = "scan 99999999999999999999999 %d n\nputs $n\n"
        assert self._run(src) == "99999999999999999999999"

    def test_scan_d_negative_bignum(self) -> None:
        src = "scan -99999999999999999999999 %d n\nputs $n\n"
        assert self._run(src) == "-99999999999999999999999"

    def test_scan_x_bignum(self) -> None:
        src = "scan ffffffffffffffffffffff %x n\nputs $n\n"
        assert self._run(src) == str(int("ffffffffffffffffffffff", 16))

    def test_scan_d_within_i64(self) -> None:
        # Sanity: small values still go through the i64 fast path
        # and produce the correct result.  Verifies the overflow
        # branch isn't accidentally activated for normal values.
        src = "scan 12345 %d n\nputs $n\n"
        assert self._run(src) == "12345"

    def test_scan_no_var_form(self) -> None:
        # ``scan`` without variables returns the matched values
        # as a list; the bignum still survives that path.
        src = "puts [scan 99999999999999999999999 %d]"
        assert self._run(src) == "99999999999999999999999"


# ``string is integer`` / ``string is wideinteger`` — recognise
# bignum-shaped string literals as valid integers.  The pre-bignum
# ``string_is_integer`` path used ``try_parse_int`` which rejects
# values exceeding i64; Stage 2 falls through to ``alloc_from_string``
# so any well-formed integer literal — at any magnitude — passes.


class TestStringIsBignum:
    """``string is integer`` / ``string is wideinteger`` accept bignums."""

    def _run(self, src: str) -> str:
        ok, out, err = _run_tcl_for_stdout(src)
        if not ok:
            pytest.fail(f"WASM compile/run failed: {err}")
        return out.rstrip("\n")

    def test_is_integer_small(self) -> None:
        assert self._run("puts [string is integer 123]") == "1"

    def test_is_integer_negative(self) -> None:
        assert self._run("puts [string is integer -42]") == "1"

    def test_is_integer_bignum(self) -> None:
        assert self._run("puts [string is integer 99999999999999999999999]") == "1"

    def test_is_integer_huge_bignum(self) -> None:
        # 50-digit bignum.
        assert self._run(
            "puts [string is integer 12345678901234567890123456789012345678901234567890]"
        ) == "1"

    def test_is_integer_rejects_float(self) -> None:
        assert self._run("puts [string is integer 12.5]") == "0"

    def test_is_integer_rejects_word(self) -> None:
        assert self._run("puts [string is integer abc]") == "0"

    def test_is_wideinteger_small(self) -> None:
        assert self._run("puts [string is wideinteger 99]") == "1"

    def test_is_wideinteger_bignum(self) -> None:
        # In Tcl 9 ``wideinteger`` and ``integer`` accept the same
        # set once bignum is the runtime representation.  Stage 1
        # would have rejected the i64-overflow value here.
        assert self._run("puts [string is wideinteger 18446744073709551616]") == "1"

    def test_is_wideinteger_negative_bignum(self) -> None:
        assert self._run("puts [string is wideinteger -18446744073709551616]") == "1"

    def test_is_wideinteger_rejects_float(self) -> None:
        assert self._run("puts [string is wideinteger 12.5]") == "0"

    def test_is_integer_on_bignum_obj(self) -> None:
        # When the operand is *already* a TYPE_BIGNUM (produced by
        # an ``expr {...}`` shift), the type-tag fast path skips the
        # string parse entirely.
        src = "set x [expr {1<<200}]\nputs [string is integer $x]\n"
        assert self._run(src) == "1"


# Runtime expr evaluator — string-equality and list-membership ops.
# Stage 2's first cut only handled arithmetic / comparison / shift /
# bitwise / pow.  ``eq`` / ``ne`` / ``in`` / ``ni`` / ``lt`` / ``le``
# / ``gt`` / ``ge`` are now wired in too so the runtime path matches
# the AOT-compiled emitter for these forms.


class TestRuntimeExprStringOps:
    """``expr`` command (runtime path) handles word-form operators."""

    def _run(self, src: str) -> str:
        ok, out, err = _run_tcl_for_stdout(src)
        if not ok:
            pytest.fail(f"WASM compile/run failed: {err}")
        return out.rstrip("\n")

    def test_eq_equal(self) -> None:
        assert self._run('puts [expr {"abc" eq "abc"}]') == "1"

    def test_eq_unequal(self) -> None:
        assert self._run('puts [expr {"abc" eq "def"}]') == "0"

    def test_ne_unequal(self) -> None:
        assert self._run('puts [expr {"abc" ne "def"}]') == "1"

    def test_lt_string(self) -> None:
        assert self._run('puts [expr {"a" lt "b"}]') == "1"

    def test_gt_string(self) -> None:
        assert self._run('puts [expr {"b" gt "a"}]') == "1"

    def test_le_string_equal(self) -> None:
        assert self._run('puts [expr {"a" le "a"}]') == "1"

    def test_ge_string_equal(self) -> None:
        assert self._run('puts [expr {"z" ge "z"}]') == "1"

    def test_in_present(self) -> None:
        assert self._run("puts [expr {2 in {1 2 3}}]") == "1"

    def test_in_absent(self) -> None:
        assert self._run("puts [expr {5 in {1 2 3}}]") == "0"

    def test_ni_absent(self) -> None:
        assert self._run("puts [expr {5 ni {1 2 3}}]") == "1"

    def test_ni_present(self) -> None:
        assert self._run("puts [expr {2 ni {1 2 3}}]") == "0"

    def test_in_with_bignum_value(self) -> None:
        # Bignum strings as list members.
        src = (
            "set big 99999999999999999999999\n"
            "puts [expr {$big in [list 1 2 99999999999999999999999]}]\n"
        )
        assert self._run(src) == "1"
