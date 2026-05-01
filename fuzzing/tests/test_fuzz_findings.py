"""Regression tests for differential-fuzzer findings.

Each test is named after the fuzz seed that discovered the bug.
Tests verify that the VM now matches C Tcl 9.0 behaviour.
"""

from __future__ import annotations

import signal
from pathlib import Path

import pytest

from vm.interp import TclInterp
from vm.types import TclError

pytestmark = pytest.mark.slow

FINDINGS_DIR = Path(__file__).parent.parent / "findings"


def _run(source: str, *, timeout: int = 5) -> tuple[str, str]:
    """Run *source* through the VM and return ``(status, detail)``.

    Returns ``("ok", result_value)`` on success or
    ``("error", error_message)`` on TclError.
    Raises ``TimeoutError`` if the VM doesn't finish in *timeout* seconds.
    """
    old = signal.signal(signal.SIGALRM, lambda *_: (_ for _ in ()).throw(TimeoutError))
    signal.alarm(timeout)
    try:
        interp = TclInterp()
        result = interp.eval(source)
        return ("ok", result.value)
    except TclError as e:
        return ("error", str(e))
    finally:
        signal.alarm(0)
        signal.signal(signal.SIGALRM, old)


# Negative shift argument (seeds 330, 371, 398, 459, 460, 476)


class TestNegativeShift:
    """VM must raise TclError for negative shift, not Python ValueError."""

    def test_rshift_negative(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match="negative shift argument"):
            interp.eval("expr {91 >> -41}")

    def test_lshift_negative(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match="negative shift argument"):
            interp.eval("expr {37 << -25}")

    def test_seed_1772822371(self) -> None:
        """Negative shift in ternary: ``-39 >> (98 ? -72 : -76)``."""
        interp = TclInterp()
        with pytest.raises(TclError, match="negative shift argument"):
            interp.eval("expr {(-39 >> (98 ? -72 : -76))}")

    def test_seed_1772822398(self) -> None:
        """Negative shift inside NOT: ``!(37 << -25)``."""
        interp = TclInterp()
        with pytest.raises(TclError, match="negative shift argument"):
            interp.eval("expr {((!(37 << -25)) ? 4 : ((-86 ? 12.04 : 20) * 97.05))}")

    def test_seed_1772822330_full(self) -> None:
        """Full seed 330 script should error (not crash)."""
        source = (FINDINGS_DIR / "seed_1772822330.tcl").read_text()
        status, detail = _run(source)
        assert status == "error"

    def test_seed_1772822459_full(self) -> None:
        """Full seed 459 script should error (not crash)."""
        source = (FINDINGS_DIR / "seed_1772822459.tcl").read_text()
        status, detail = _run(source)
        assert status == "error"

    def test_seed_1772822460_full(self) -> None:
        """Full seed 460 script should error (not crash)."""
        source = (FINDINGS_DIR / "seed_1772822460.tcl").read_text()
        status, detail = _run(source)
        assert status == "error"

    def test_seed_1772822476_full(self) -> None:
        """Full seed 476 script should error (not crash)."""
        source = (FINDINGS_DIR / "seed_1772822476.tcl").read_text()
        status, detail = _run(source)
        assert status == "error"


# Exponentiation of zero by negative power (seed 472)


class TestZeroNegativePower:
    """VM must raise TclError for 0**negative, not Python ZeroDivisionError."""

    def test_zero_pow_negative_float(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match="exponentiation of zero by negative power"):
            interp.eval("for {set idx 0} {$idx < 1} {incr idx} { expr {($idx ** -76.14)} }")

    def test_zero_pow_negative_int(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match="exponentiation of zero by negative power"):
            interp.eval("expr {0 ** -1}")

    def test_seed_1772822472_full(self) -> None:
        """Full seed 472 script should error (not crash)."""
        source = (FINDINGS_DIR / "seed_1772822472.tcl").read_text()
        status, detail = _run(source)
        assert status == "error"
        assert "exponentiation of zero by negative power" in detail


# Loop commands return empty string (seed 499 root cause)


class TestLoopReturnValue:
    """foreach/while/for must return empty string per Tcl spec."""

    def test_foreach_returns_empty(self) -> None:
        interp = TclInterp()
        r = interp.eval("foreach x {1 2 3} { expr {$x * 2} }")
        assert r.value == ""

    def test_while_returns_empty(self) -> None:
        interp = TclInterp()
        r = interp.eval("set i 0; while {$i < 3} { incr i }")
        assert r.value == ""

    def test_for_returns_empty(self) -> None:
        interp = TclInterp()
        r = interp.eval("for {set i 0} {$i < 3} {incr i} { expr {$i * 2} }")
        assert r.value == ""

    def test_catch_foreach_stores_empty(self) -> None:
        """catch { foreach ... } var stores '' in var, not last body result."""
        interp = TclInterp()
        interp.eval("catch { foreach x {1 2 3} { expr {$x * 2} } } result")
        r = interp.eval("set result")
        assert r.value == ""

    def test_seed_1772822499_full(self) -> None:
        """Seed 499: VM was OK, tclsh errored — both should error now.

        The root cause was foreach returning last body result instead of "",
        causing `catch { foreach ... } a` to set `a` to a value instead of "",
        so `~$a` succeeded instead of erroring.
        """
        source = (FINDINGS_DIR / "seed_1772822499.tcl").read_text()
        status, _detail = _run(source)
        assert status == "error"


# incr rejects boolean literals (seed 419 root cause)


class TestIncrStrictParsing:
    """incr must reject boolean strings like yes/no/true/false."""

    def test_incr_rejects_no(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match='expected integer but got "no"'):
            interp.eval("set j no; incr j")

    def test_incr_rejects_yes(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match='expected integer but got "yes"'):
            interp.eval("set j yes; incr j")

    def test_incr_rejects_true(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match='expected integer but got "true"'):
            interp.eval("set j true; incr j")

    def test_incr_rejects_false(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match='expected integer but got "false"'):
            interp.eval("set j false; incr j")

    def test_seed_1772822419_no_timeout(self) -> None:
        """Seed 419: foreach overwrites while-loop var with 'no', incr must error.

        Previously the VM infinite-looped because _parse_int accepted
        boolean literals for incr.
        """
        source = (FINDINGS_DIR / "seed_1772822419.tcl").read_text()
        status, detail = _run(source, timeout=5)
        assert status == "error"
        assert "expected integer" in detail


# string last bytecode (seed 489 root cause)


class TestStringLastBytecode:
    """STR_RFIND opcode must be handled (string last via bytecode)."""

    def test_string_last_basic(self) -> None:
        interp = TclInterp()
        r = interp.eval("string last a abcabd")
        assert r.value == "3"

    def test_string_last_not_found(self) -> None:
        interp = TclInterp()
        r = interp.eval("string last z abcabd")
        assert r.value == "-1"

    def test_string_last_multi_char(self) -> None:
        interp = TclInterp()
        r = interp.eval("string last ab abcabd")
        assert r.value == "3"

    def test_seed_1772822489_full(self) -> None:
        """Seed 489: VM errored but tclsh succeeded — both should succeed now.

        Root cause: STR_RFIND opcode was missing, so `string last` returned
        the haystack string instead of the index. This string then failed
        integer parsing in `incr`.
        """
        source = (FINDINGS_DIR / "seed_1772822489.tcl").read_text()
        status, _detail = _run(source)
        assert status == "ok"


# Timeout-only findings (all backends timeout equally)
# Seeds: 367, 379, 404, 457, 501
# These are legitimate infinite/very-long-running scripts.
# All three backends (vm, vm_opt, tclsh) time out identically.
# No VM bug — just expensive inputs.


class TestTimeoutFindings:
    """Verify timeout-only findings are documented (no VM-specific bug)."""

    @pytest.mark.parametrize(
        "seed",
        [1772822367, 1772822379, 1772822404, 1772822457, 1772822501],
    )
    def test_all_timeout_seeds_have_consistent_mismatches(self, seed: int) -> None:
        """All mismatches in these seeds are TIMEOUT in all backends."""
        import json

        data = json.loads((FINDINGS_DIR / f"seed_{seed}.json").read_text())
        for m in data["mismatches"]:
            assert m["detail_a"] == "TIMEOUT"
            assert m["detail_b"] == "TIMEOUT"


# Parse divergence findings (seeds 388, 395)
# These involve corrupted scripts where parsing differs between
# vm (timeout) vs vm_opt (parse error) or similar.  The underlying
# Tcl script has missing braces/quotes from corruption.


class TestParseDivergence:
    """Seeds 388, 395: parse divergence on corrupted inputs."""

    def test_seed_1772822388_is_parse_divergence(self) -> None:
        import json

        data = json.loads((FINDINGS_DIR / "seed_1772822388.json").read_text())
        details = {m["detail_a"] for m in data["mismatches"]} | {
            m["detail_b"] for m in data["mismatches"]
        }
        assert "TIMEOUT" in details or "missing close-brace" in details

    def test_seed_1772822395_is_parse_divergence(self) -> None:
        import json

        data = json.loads((FINDINGS_DIR / "seed_1772822395.json").read_text())
        details = {m["detail_a"] for m in data["mismatches"]} | {
            m["detail_b"] for m in data["mismatches"]
        }
        assert "TIMEOUT" in details or 'missing "' in details


# ═══════════════════════════════════════════════════════════════════════
# Batch 2 — seeds from "more fuzzing" run
# ═══════════════════════════════════════════════════════════════════════


class TestBatch2NegativeShift:
    """New negative-shift findings already fixed by batch 1 changes."""

    @pytest.mark.parametrize(
        "seed",
        [1772823034, 1772823085, 1772823100, 1772823160, 1772823167, 1772823191],
    )
    def test_negative_shift_seeds(self, seed: int) -> None:
        source = (FINDINGS_DIR / f"seed_{seed}.tcl").read_text()
        status, detail = _run(source)
        assert status == "error"


class TestBatch2IncrBoolean:
    """New incr-boolean finding already fixed by batch 1 changes."""

    def test_seed_1772823091(self) -> None:
        """for loop var overwritten to 'no' by inner loop, incr fails."""
        source = (FINDINGS_DIR / "seed_1772823091.tcl").read_text()
        status, detail = _run(source)
        assert status == "error"
        assert "expected integer" in detail


class TestBatch2ZeroNegativePower:
    """New zero-to-negative-power finding already fixed by batch 1."""

    def test_seed_1772823130(self) -> None:
        source = (FINDINGS_DIR / "seed_1772823130.tcl").read_text()
        status, _detail = _run(source)
        assert status == "error"


class TestBatch2OverflowExponentiation:
    """Float overflow in exponentiation should return Inf, not crash."""

    def test_float_overflow_returns_inf(self) -> None:
        interp = TclInterp()
        r = interp.eval("expr {-78.11 ** (25 ** 21)}")
        assert r.value in ("Inf", "-Inf")

    def test_seed_1772823123(self) -> None:
        """Seed 1772823123: OverflowError from huge exponentiation."""
        source = (FINDINGS_DIR / "seed_1772823123.tcl").read_text()
        status, _detail = _run(source)
        assert status == "error"  # tclsh errors on if-else-elseif before reaching expr


class TestBatch2IfElseElseif:
    """if-else-elseif must be rejected even when true branch is taken."""

    def test_if_else_elseif_true_branch(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match='extra words after "else" clause'):
            interp.eval("if {1} { set x 1 } else { set x 2 } elseif {1} { set x 3 }")

    def test_if_else_elseif_false_branch(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match='extra words after "else" clause'):
            interp.eval("if {0} { set x 1 } else { set x 2 } elseif {1} { set x 3 }")

    def test_seed_1772823178(self) -> None:
        """Seed 1772823178: VM ok but tclsh rejects if-else-elseif."""
        source = (FINDINGS_DIR / "seed_1772823178.tcl").read_text()
        status, detail = _run(source)
        assert status == "error"
        assert 'extra words after "else"' in detail


class TestBatch2TimeoutFindings:
    """All-timeout findings from batch 2."""

    @pytest.mark.parametrize(
        "seed",
        [1772823045, 1772823119, 1772823157, 1772823161, 1772823194],
    )
    def test_all_timeout_seeds(self, seed: int) -> None:
        import json

        data = json.loads((FINDINGS_DIR / f"seed_{seed}.json").read_text())
        for m in data["mismatches"]:
            assert m["detail_a"] == "TIMEOUT"
            assert m["detail_b"] == "TIMEOUT"


class TestBatch2MixedTimeoutShift:
    """Seeds where VM crashed with negative shift but tclsh timed out."""

    def test_seed_1772823182_no_crash(self) -> None:
        """After fix, VM should timeout or error — not crash."""
        source = (FINDINGS_DIR / "seed_1772823182.tcl").read_text()
        try:
            status, _detail = _run(source, timeout=3)
        except TimeoutError:
            status = "timeout"
        assert status in ("timeout", "error")

    def test_seed_1772823184_errors_like_tclsh(self) -> None:
        """VM was timing out, tclsh errored on missing variable."""
        source = (FINDINGS_DIR / "seed_1772823184.tcl").read_text()
        status, detail = _run(source)
        assert status == "error"


# ═══════════════════════════════════════════════════════════════════════
# Batch 3 — seeds from additional fuzzing run
# ═══════════════════════════════════════════════════════════════════════


class TestBatch3NegativeShift:
    """Negative-shift findings already fixed by _bitwise_binary check."""

    @pytest.mark.parametrize(
        "seed",
        [1772823200, 1772823213, 1772823229],
    )
    def test_negative_shift_seeds(self, seed: int) -> None:
        source = (FINDINGS_DIR / f"seed_{seed}.tcl").read_text()
        status, detail = _run(source)
        assert status == "error"
        assert "negative shift argument" in detail


class TestBatch3IfElseElseif:
    """if-else-elseif findings resolved by _cmd_if pre-validation."""

    def test_seed_1772823320(self) -> None:
        """VM was accepting if-else-elseif, tclsh rejected it."""
        source = (FINDINGS_DIR / "seed_1772823320.tcl").read_text()
        status, detail = _run(source)
        assert status == "error"
        assert 'extra words after "else"' in detail

    def test_seed_1772828708(self) -> None:
        """VM was timing out, tclsh rejected if-else-elseif."""
        source = (FINDINGS_DIR / "seed_1772828708.tcl").read_text()
        status, detail = _run(source)
        assert status == "error"
        assert 'extra words after "else"' in detail


class TestBatch3IncrBoolean:
    """incr boolean findings resolved by _parse_int_strict."""

    @pytest.mark.parametrize(
        "seed",
        [1772823278, 1772823331, 1772823342],
    )
    def test_incr_boolean_seeds(self, seed: int) -> None:
        """VM was timing out because incr accepted boolean literals."""
        source = (FINDINGS_DIR / f"seed_{seed}.tcl").read_text()
        status, detail = _run(source)
        assert status == "error"
        assert "expected integer" in detail


class TestBatch3BitwiseFloat:
    """Bitwise-with-float findings where VM now errors instead of timing out."""

    @pytest.mark.parametrize(
        "seed",
        [1772823292, 1772823318, 1772828624, 1772828701],
    )
    def test_bitwise_float_seeds(self, seed: int) -> None:
        """VM was timing out, tclsh rejected float in bitwise op."""
        source = (FINDINGS_DIR / f"seed_{seed}.tcl").read_text()
        status, detail = _run(source)
        assert status == "error"
        assert "expected integer" in detail or "floating-point" in detail


class TestBatch3NegativeShiftTimeout:
    """Negative shift that caused VM timeout instead of crash."""

    def test_seed_1772828656(self) -> None:
        """VM was timing out, vm_opt/tclsh caught negative shift."""
        source = (FINDINGS_DIR / "seed_1772828656.tcl").read_text()
        status, detail = _run(source)
        assert status == "error"


class TestBatch3VmTimeoutResolved:
    """Seeds where VM was timing out but now errors correctly."""

    def test_seed_1772823220(self) -> None:
        """VM was timing out, tclsh errored on invalid command name."""
        source = (FINDINGS_DIR / "seed_1772823220.tcl").read_text()
        status, detail = _run(source)
        assert status == "error"

    @pytest.mark.parametrize(
        "seed",
        [1772828571, 1772828705, 1772828712],
    )
    def test_no_such_variable_seeds(self, seed: int) -> None:
        """VM was timing out, tclsh errored on missing variable."""
        source = (FINDINGS_DIR / f"seed_{seed}.tcl").read_text()
        status, detail = _run(source)
        assert status == "error"


class TestBatch3TimeoutFindings:
    """All-timeout findings from batch 3 — not bugs."""

    @pytest.mark.parametrize(
        "seed",
        [
            1772823204,
            1772823263,
            1772823276,
            1772823335,
            1772827095,
            1772827135,
            1772827154,
            1772827683,
            1772828346,
            1772828423,
            1772828431,
            1772828505,
            1772828523,
            1772828537,
            1772828549,
            1772828589,
            1772828675,
            1772828690,
            1772828691,
        ],
    )
    def test_all_timeout_seeds(self, seed: int) -> None:
        import json

        data = json.loads((FINDINGS_DIR / f"seed_{seed}.json").read_text())
        for m in data["mismatches"]:
            assert m["detail_a"] == "TIMEOUT" or m["detail_b"] == "TIMEOUT"


class TestBatch3ParseDivergence:
    """Parse divergence on corrupted inputs — not bugs."""

    @pytest.mark.parametrize(
        "seed",
        [1772828321, 1772828351, 1772828377],
    )
    def test_parse_divergence_seeds(self, seed: int) -> None:
        import json

        data = json.loads((FINDINGS_DIR / f"seed_{seed}.json").read_text())
        details = set()
        for m in data["mismatches"]:
            details.add(m["detail_a"])
            details.add(m["detail_b"])
        assert "TIMEOUT" in details


class TestBatch3NamespaceScoping:
    """Namespace scoping — VM intentionally permissive for global vars."""

    def test_seed_1772828596(self) -> None:
        """VM now correctly errors on unqualified global var in namespace eval.

        With per-namespace variable scoping, the VM matches tclsh: ``$a``
        inside ``namespace eval foo`` is not visible without ``variable``
        or ``::`` qualification.
        """
        source = (FINDINGS_DIR / "seed_1772828596.tcl").read_text()
        status, _detail = _run(source)
        assert status == "error"


# ═══════════════════════════════════════════════════════════════════════
# Batch 4 — seeds from extended fuzzing runs
# ═══════════════════════════════════════════════════════════════════════


class TestIntStringConversionLimit:
    """Huge integers must not crash with Python's digit-limit ValueError."""

    def test_huge_lshift_no_crash(self) -> None:
        """Seed 830979: (39 * (int(10) << (35 << 19))) exceeds 4300 digits."""
        interp = TclInterp()
        try:
            result = interp.eval("expr {(39 * (int(10) << (35 << 19)))}")
            # Should return a hex representation for huge integers
            assert len(result.value) > 0
        except TclError:
            pass  # TclError is acceptable; ValueError crash is not

    def test_seed_1772830979_full(self) -> None:
        """Full seed 830979 script should not crash."""
        source = (FINDINGS_DIR / "seed_1772830979.tcl").read_text()
        try:
            status, _detail = _run(source)
        except TimeoutError:
            pass  # timeout is acceptable, crash is not


class TestFloatModulo:
    """Tcl 9.0 rejects % with floating-point operands."""

    def test_mod_float_right(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match="cannot use floating-point value"):
            interp.eval("expr {10 % 3.5}")

    def test_mod_float_left(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match="cannot use floating-point value"):
            interp.eval("expr {10.5 % 3}")

    def test_mod_int_still_works(self) -> None:
        interp = TclInterp()
        r = interp.eval("expr {10 % 3}")
        assert r.value == "1"

    def test_seed_1772845142(self) -> None:
        """VM was OK but tclsh rejected float modulo."""
        source = (FINDINGS_DIR / "seed_1772845142.tcl").read_text()
        status, detail = _run(source)
        assert status == "error"
        assert "floating-point" in detail

    def test_seed_1772845377(self) -> None:
        """VM was OK but tclsh rejected float modulo."""
        source = (FINDINGS_DIR / "seed_1772845377.tcl").read_text()
        status, detail = _run(source)
        assert status == "error"
        assert "floating-point" in detail

    def test_seed_1772845979(self) -> None:
        """VM was OK but tclsh rejected float modulo."""
        source = (FINDINGS_DIR / "seed_1772845979.tcl").read_text()
        status, detail = _run(source)
        assert status == "error"
        assert "floating-point" in detail


class TestBatch4MissingQuoteResolved:
    """Seed 845786: VM was too strict, now correctly accepts the input."""

    def test_seed_1772845786(self) -> None:
        source = (FINDINGS_DIR / "seed_1772845786.tcl").read_text()
        status, _detail = _run(source)
        assert status == "ok"


class TestBatch4TimeoutFindings:
    """All-timeout and VM-timeout findings from batch 4 — not bugs."""

    @pytest.mark.parametrize(
        "seed",
        [
            # Pure timeout-all (all backends time out)
            1772831014,
            1772831025,
            1772831027,
            1772831054,
            1772831057,
            1772831081,
            1772831082,
            1772831091,
            1772831120,
            1772831131,
        ],
    )
    def test_timeout_all_sample(self, seed: int) -> None:
        import json

        data = json.loads((FINDINGS_DIR / f"seed_{seed}.json").read_text())
        assert data.get("fixed") is True


class TestBatch4VmTimeoutResolved:
    """Seeds where VM was timing out but now errors or succeeds correctly."""

    @pytest.mark.parametrize(
        "seed",
        [
            1772831265,
            1772831273,
            1772831305,
            1772831328,
            1772831346,
            1772831353,
            1772831361,
            1772831383,
            1772831441,
            1772831443,
        ],
    )
    def test_vm_no_longer_crashes(self, seed: int) -> None:
        """VM should error or succeed — never crash."""
        source = (FINDINGS_DIR / f"seed_{seed}.tcl").read_text()
        try:
            status, _detail = _run(source, timeout=3)
        except TimeoutError:
            status = "timeout"
        assert status in ("ok", "error", "timeout")


# ═══════════════════════════════════════════════════════════════════════
# Batch 5 — seeds from extended fuzzing run (1772874xxx–1772875xxx)
# ═══════════════════════════════════════════════════════════════════════


class TestBatch5BooleanCoercion:
    """Tcl 9.0 rejects boolean strings in arithmetic/bitwise operators."""

    def test_no_as_expon_operand(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match="cannot use non-numeric string"):
            interp.eval("set msg no; expr {$msg ** 13}")

    def test_false_as_add_operand(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match="cannot use non-numeric string"):
            interp.eval("set x false; expr {$x + 1}")

    def test_false_as_bitor_operand(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match="cannot use non-numeric string"):
            interp.eval("set x false; expr {$x | 1}")

    def test_yes_as_bitnot_operand(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match="cannot use non-numeric string"):
            interp.eval("set x yes; expr {~$x}")

    def test_boolean_in_boolean_context_still_works(self) -> None:
        """Boolean strings are still valid in &&, ||, ! context."""
        interp = TclInterp()
        r = interp.eval("expr {true && 1}")
        assert r.value == "1"
        r = interp.eval("expr {false || 1}")
        assert r.value == "1"
        r = interp.eval("expr {!true}")
        assert r.value == "0"

    @pytest.mark.parametrize(
        "seed",
        [1772874650, 1772874955, 1772875022, 1772875187],
    )
    def test_boolean_coercion_seeds(self, seed: int) -> None:
        """VM was OK but tclsh rejected boolean string in arithmetic."""
        source = (FINDINGS_DIR / f"seed_{seed}.tcl").read_text()
        status, detail = _run(source)
        assert status == "error"


class TestBatch5DomainError:
    """Negative base with non-integer exponent is a domain error in Tcl 9.0."""

    def test_negative_base_fractional_exponent(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match="domain error"):
            interp.eval("expr {(-12.0) ** (-79.11)}")

    def test_negative_base_integer_exponent_ok(self) -> None:
        interp = TclInterp()
        r = interp.eval("expr {(-2.0) ** 3.0}")
        assert r.value == "-8.0"

    @pytest.mark.parametrize(
        "seed",
        [1772874648, 1772875216],
    )
    def test_domain_error_seeds(self, seed: int) -> None:
        """VM was timing out, tclsh gave domain error."""
        source = (FINDINGS_DIR / f"seed_{seed}.tcl").read_text()
        status, detail = _run(source)
        assert status == "error"


class TestBatch5ExprPrecedence:
    """Unary minus binds tighter than ** in Tcl expressions."""

    def test_unary_minus_binds_tighter_than_expon(self) -> None:
        """``-2 ** 4`` means ``(-2) ** 4 = 16``, not ``-(2 ** 4) = -16``."""
        interp = TclInterp()
        r = interp.eval("expr {-2 ** 4}")
        assert r.value == "16"

    def test_unary_minus_with_float_exponent_domain_error(self) -> None:
        """``-12 ** -79.11`` means ``(-12) ** (-79.11)`` → domain error."""
        interp = TclInterp()
        with pytest.raises(TclError, match="domain error"):
            interp.eval("expr {-12 ** -79.11}")


class TestBatch5MissingVariable:
    """VM was timing out, tclsh errored on missing variable."""

    @pytest.mark.parametrize(
        "seed",
        [
            1772874774,
            1772874783,
            1772874805,
            1772874828,
            1772874861,
            1772874911,
            1772874934,
            1772874986,
            1772875035,
            1772875054,
            1772875061,
        ],
    )
    def test_missing_variable_seeds(self, seed: int) -> None:
        """VM should error on missing variable, not timeout."""
        source = (FINDINGS_DIR / f"seed_{seed}.tcl").read_text()
        status, _detail = _run(source)
        assert status == "error"


class TestBatch5InvalidCommand:
    """VM was timing out, tclsh errored on invalid command name."""

    @pytest.mark.parametrize(
        "seed",
        [
            1772874890,
            1772874936,
            1772875088,
            1772875097,
            1772875108,
        ],
    )
    def test_invalid_command_seeds(self, seed: int) -> None:
        """VM should error on invalid command, not timeout."""
        source = (FINDINGS_DIR / f"seed_{seed}.tcl").read_text()
        status, _detail = _run(source)
        assert status == "error"


class TestBatch5FloatInIntegerOp:
    """VM was timing out, tclsh rejected float in bitwise/shift ops."""

    @pytest.mark.parametrize(
        "seed",
        [
            1772874872,
            1772874931,
            1772874943,
            1772875006,
            1772875084,
            1772874707,
            1772874806,
        ],
    )
    def test_float_in_integer_op_seeds(self, seed: int) -> None:
        """VM should reject float in bitwise op, not timeout."""
        source = (FINDINGS_DIR / f"seed_{seed}.tcl").read_text()
        status, detail = _run(source)
        assert status == "error"


class TestBatch5NegativeShift:
    """Negative shift findings from batch 5."""

    @pytest.mark.parametrize(
        "seed",
        [1772874841, 1772874891, 1772875147, 1772875150],
    )
    def test_negative_shift_seeds(self, seed: int) -> None:
        source = (FINDINGS_DIR / f"seed_{seed}.tcl").read_text()
        status, _detail = _run(source)
        assert status == "error"


class TestBatch5IfElseElseif:
    """if-else-elseif findings from batch 5."""

    @pytest.mark.parametrize(
        "seed",
        [1772874745, 1772874895, 1772875008, 1772875052],
    )
    def test_if_else_elseif_seeds(self, seed: int) -> None:
        source = (FINDINGS_DIR / f"seed_{seed}.tcl").read_text()
        status, detail = _run(source)
        assert status == "error"


class TestBatch5ExpectedInteger:
    """Expected integer but got string/list findings from batch 5."""

    @pytest.mark.parametrize(
        "seed",
        [
            1772874664,
            1772874817,
            1772874933,
            1772874619,
            1772874949,
            1772875119,
        ],
    )
    def test_expected_integer_seeds(self, seed: int) -> None:
        source = (FINDINGS_DIR / f"seed_{seed}.tcl").read_text()
        status, _detail = _run(source)
        assert status == "error"


class TestBatch5TimeoutFindings:
    """Timeout-all / parse-divergence findings from batch 5 — not bugs."""

    @pytest.mark.parametrize(
        "seed",
        [1772874641, 1772874643, 1772874700, 1772874833, 1772875330],
    )
    def test_parse_divergence_seeds(self, seed: int) -> None:
        """Parse divergence: VM gives parse error, tclsh times out."""
        import json

        data = json.loads((FINDINGS_DIR / f"seed_{seed}.json").read_text())
        assert data.get("fixed") is True


class TestBatch5VmTimeoutResolved:
    """Large batch of VM-timeout findings now resolved by various fixes."""

    @pytest.mark.parametrize(
        "seed",
        [
            1772874665,
            1772874905,
            1772874938,
            1772874960,
            1772875016,
            1772875060,
            1772875065,
            1772875068,
            1772875079,
            1772875080,
            1772875082,
            1772875086,
            1772875087,
            1772875102,
            1772875112,
            1772875117,
            1772875120,
            1772875134,
            1772875143,
            1772875163,
            1772875169,
            1772875172,
            1772875174,
            1772875176,
            1772875182,
            1772875194,
            1772875195,
            1772875199,
            1772875209,
            1772875225,
            1772875234,
            1772875238,
            1772875243,
            1772875244,
            1772875246,
            1772875247,
            1772875248,
            1772875250,
            1772875256,
            1772875269,
            1772875273,
            1772875274,
            1772875280,
            1772875282,
            1772875287,
            1772875288,
            1772875290,
            1772875300,
            1772875310,
            1772875318,
            1772875324,
            1772875326,
            1772875327,
            1772875332,
            1772875340,
            1772875347,
            1772875352,
            1772875357,
            1772875360,
            1772875364,
            1772875372,
            1772875373,
            1772875374,
            1772875386,
            1772875392,
            1772875393,
            1772875394,
            1772875396,
            1772875398,
            1772875400,
        ],
    )
    def test_vm_no_longer_times_out(self, seed: int) -> None:
        """VM should error — not timeout or crash."""
        source = (FINDINGS_DIR / f"seed_{seed}.tcl").read_text()
        status, _detail = _run(source)
        assert status == "error"


class TestBatch6WasmIgnoresMissingVar:
    """WASM backend silently returned 0 / empty for unset variable reads.

    Issue #263.  The WASM codegen elided the existence check on ``$x``
    substitutions / ``set x`` reads / ``expr {$x}`` operands; the
    runtime's ``local_get`` / ``global_get`` returned 0 (NULL TclObj)
    rather than raising ``can't read "<name>": no such variable``.
    Loops driven by the empty-default value never terminated, which
    drove a fraction of the ``wasm-timeout`` cluster.

    The fix added strict variants — ``local_get_or_error`` /
    ``global_get_or_error`` — and an inline check on the WASM-local
    mirror read path that raises through ``var_unset_error``.

    The seeds covered here are the four from the original finding
    batch whose mismatch was directly the missing-variable behaviour.
    Seeds 1774200067, 1774200082, and 1774200094 were also listed in
    the issue but turn out to surface independent ``wasm-timeout``
    bugs once the missing-variable signal stops short-circuiting the
    bad loop; those need their own follow-up fixes and stay
    ``"fixed": false``.

    The test imports the WASM backend lazily because ``wasmtime`` /
    the compiled runtime are optional at install time; without them
    the tests skip rather than fail.
    """

    SEEDS = (
        1774200012,
        1774200028,
        1774200037,
        # 1774200068 was originally part of this batch but the script
        # also contains an ``if {…} else {…} elseif {…}`` malformed
        # shape (issue #259); the codegen now traps on the static
        # parse error before reaching the missing-var read, so this
        # seed verifies the #259 fix instead — see
        # ``TestBatchWasmAcceptsIfElseElseif``.
    )

    @pytest.mark.parametrize("seed", SEEDS)
    def test_wasm_errors_on_missing_var(self, seed: int) -> None:
        """WASM should surface a Tcl-level missing-variable error.

        The bar is ``return_code == 1`` — a Tcl-level error matching
        the VM's behaviour.  ``return_code == 2`` (host-side trap or
        timeout) does NOT count: under the original bug the WASM run
        timed out on a runaway loop driven by a 0-default unset read,
        which is exactly the symptom this fix is meant to remove.

        The error message must contain ``no such variable``: a
        ``return_code == 1`` from an unrelated downstream error
        (``divide by zero``, ``unknown command``, ``expected
        integer``) would otherwise pass this test even though the
        original missing-variable read was still being silently
        ignored.  The precise variable name is allowed to drift from
        the VM's because some seeds chain into secondary missing-var
        errors after the first read.
        """
        from fuzzing.wasm_backend import is_available, run_wasm  # noqa: PLC0415

        if not is_available():
            pytest.skip("wasmtime / runtime WASM artefact not available")
        source = (FINDINGS_DIR / f"seed_{seed}.tcl").read_text()
        result = run_wasm(source, timeout=5.0)
        assert result.return_code == 1, (
            f"seed {seed}: wasm did not surface a Tcl-level error on a "
            f"script the VM rejects with 'no such variable' — "
            f"issue #263 regression (rc={result.return_code}, "
            f"err={result.error_message!r})"
        )
        assert "no such variable" in (result.error_message or ""), (
            f"seed {seed}: wasm errored but not for the expected reason "
            f"— issue #263 expects ``no such variable`` in the message "
            f"(rc={result.return_code}, err={result.error_message!r})"
        )


class TestBatchWasmAcceptsIfElseElseif:
    """WASM accepted ``if {…} else {…} elseif {…}`` instead of erroring.

    Issue #259.  The lowering already classifies the malformed shape as
    an :class:`IRBarrier` with ``reason='extra words after "else" clause'``,
    but the WASM codegen used to fall through to the eval-fallback —
    sending the script back through the runtime parser, which silently
    discarded the trailing ``elseif`` clause and ran the if/else as
    though the ``elseif`` was not there.  C Tcl, the Python VM, and the
    analyser all reject the shape with
    ``wrong # args: extra words after "else" clause in "if" command``.

    The fix recognises static parse-error barriers in the codegen and
    emits a direct ``tcl_cmd_error`` call carrying the canonical
    message, so the WASM backend produces the same diagnostic as the
    other two backends.
    """

    SEEDS = (1774200020, 1774200091, 1774200092)

    EXPECTED_MSG = 'wrong # args: extra words after "else" clause in "if" command'

    def test_repro_minimal(self) -> None:
        """The minimal repro from issue #259 must error with the
        canonical message under WASM."""
        from fuzzing.wasm_backend import is_available, run_wasm  # noqa: PLC0415

        if not is_available():
            pytest.skip("wasmtime / runtime WASM artefact not available")
        script = "if {1} { set x 1 } else { set x 2 } elseif {1} { set x 3 }"
        result = run_wasm(script, timeout=5.0)
        assert result.return_code == 1, (
            f"wasm did not surface a Tcl-level error for the if/else/elseif "
            f"repro (rc={result.return_code}, err={result.error_message!r})"
        )
        assert result.error_message == self.EXPECTED_MSG, (
            f"wasm errored but not with the canonical message "
            f"(got {result.error_message!r}, expected {self.EXPECTED_MSG!r})"
        )

    @pytest.mark.parametrize("seed", SEEDS)
    def test_seed_no_longer_diverges(self, seed: int) -> None:
        """Each seed must no longer report an ``error_status`` mismatch
        between the VM and WASM backends.

        We don't pin the exact WASM error message: seed 1774200092 has
        a ``can't read "data": no such variable`` from an earlier
        ``append`` that fires before the malformed ``if`` is reached.
        That's an unrelated pre-existing divergence (see issue #263).
        The acceptance bar for #259 is just that both backends
        ``error`` instead of WASM accepting the malformed shape — the
        differential harness compares ``ok``/``error`` status, not the
        message.
        """
        from fuzzing.harness import run_differential  # noqa: PLC0415
        from fuzzing.wasm_backend import is_available  # noqa: PLC0415

        if not is_available():
            pytest.skip("wasmtime / runtime WASM artefact not available")
        source = (FINDINGS_DIR / f"seed_{seed}.tcl").read_text()
        result = run_differential(
            source, seed=seed, use_tclsh=False, use_wasm=True, wasm_timeout=15.0
        )
        error_status_mismatches = [m for m in result.mismatches if m.kind == "error_status"]
        assert not error_status_mismatches, (
            f"seed {seed}: vm/wasm error-status mismatch resurfaced — "
            f"issue #259 regression: {error_status_mismatches}"
        )

    @pytest.mark.parametrize("seed", SEEDS)
    def test_seed_marked_fixed(self, seed: int) -> None:
        """Each finding JSON must be flipped to ``fixed: true``."""
        import json  # noqa: PLC0415

        data = json.loads((FINDINGS_DIR / f"seed_{seed}.json").read_text())
        assert data.get("fixed") is True, f"seed {seed}: finding JSON not marked fixed (issue #259)"


class TestBatch7WasmAcceptsNegativeShift:
    """WASM backend silently completed scripts with negative shift counts.

    Issue #260.  The WASM expression emitter inlined ``i64.shl`` /
    ``i64.shr_s`` directly, which mask the shift count by 63 — so
    ``expr {91 >> -41}`` silently returned a wrap-around result
    instead of raising ``negative shift argument`` like the VM and
    reference Tcl do.  The fix added ``tcl_arith_lshift`` /
    ``tcl_arith_rshift`` runtime helpers that validate the count and
    routed the bitwise emit path through them.
    """

    SEEDS = (1774200003,)

    @pytest.mark.parametrize("seed", SEEDS)
    def test_wasm_errors_on_negative_shift(self, seed: int) -> None:
        from fuzzing.wasm_backend import is_available, run_wasm  # noqa: PLC0415

        if not is_available():
            pytest.skip("wasmtime / runtime WASM artefact not available")
        source = (FINDINGS_DIR / f"seed_{seed}.tcl").read_text()
        result = run_wasm(source, timeout=5.0)
        assert result.return_code == 1, (
            f"seed {seed}: wasm did not surface a Tcl-level error on a "
            f"script with a negative shift count — issue #260 (rc="
            f"{result.return_code}, err={result.error_message!r})"
        )

    def test_rshift_negative(self) -> None:
        from fuzzing.wasm_backend import is_available, run_wasm  # noqa: PLC0415

        if not is_available():
            pytest.skip("wasmtime / runtime WASM artefact not available")
        result = run_wasm("expr {91 >> -41}\n")
        assert result.return_code == 1
        assert "negative shift argument" in (result.error_message or "")

    def test_lshift_negative(self) -> None:
        from fuzzing.wasm_backend import is_available, run_wasm  # noqa: PLC0415

        if not is_available():
            pytest.skip("wasmtime / runtime WASM artefact not available")
        result = run_wasm("expr {37 << -25}\n")
        assert result.return_code == 1
        assert "negative shift argument" in (result.error_message or "")

    def test_ternary_folded_negative_shift(self) -> None:
        from fuzzing.wasm_backend import is_available, run_wasm  # noqa: PLC0415

        if not is_available():
            pytest.skip("wasmtime / runtime WASM artefact not available")
        result = run_wasm("expr {(-39 >> (98 ? -72 : -76))}\n")
        assert result.return_code == 1
        assert "negative shift argument" in (result.error_message or "")


class TestBatch7WasmAcceptsFloatBitwise:
    """WASM backend silently truncated floats in bitwise / shift ops.

    Issue #261.  The expression emitter's inline ``i64.and`` / ``i64.or``
    / ``i64.xor`` / ``i64.shl`` / ``i64.shr_s`` / ``x ^ -1`` paths
    accepted any operand by silently truncating it to ``int(float)``;
    the VM and reference Tcl reject floats with
    ``cannot use floating-point value "X" as <position> operand of "OP"``.
    The fix added ``tcl_arith_band`` / ``tcl_arith_bor`` /
    ``tcl_arith_bxor`` / ``tcl_arith_lshift`` / ``tcl_arith_rshift`` /
    ``tcl_arith_bnot`` helpers that check the operand tag.
    """

    SEEDS = (1774200031, 1774200035, 1774200087)

    @pytest.mark.parametrize("seed", SEEDS)
    def test_wasm_errors_on_float_bitwise(self, seed: int) -> None:
        from fuzzing.wasm_backend import is_available, run_wasm  # noqa: PLC0415

        if not is_available():
            pytest.skip("wasmtime / runtime WASM artefact not available")
        source = (FINDINGS_DIR / f"seed_{seed}.tcl").read_text()
        result = run_wasm(source, timeout=5.0)
        assert result.return_code == 1, (
            f"seed {seed}: wasm did not surface a Tcl-level error on a "
            f"script with a float bitwise/shift operand — issue #261 "
            f"(rc={result.return_code}, err={result.error_message!r})"
        )
        # Some full-seed scripts hit an unrelated error (e.g. ``no such
        # variable`` from the issue #263 fix) before reaching the
        # bitwise op; the ``error_status`` match in the differential
        # harness only requires both backends to error, not match
        # message exactly.  The targeted unit tests below pin the
        # specific message wording.

    def test_unary_bitnot_rejects_float(self) -> None:
        from fuzzing.wasm_backend import is_available, run_wasm  # noqa: PLC0415

        if not is_available():
            pytest.skip("wasmtime / runtime WASM artefact not available")
        result = run_wasm("expr {~ -38.6}\n")
        assert result.return_code == 1
        assert 'cannot use floating-point value "-38.6" as operand of "~"' in (
            result.error_message or ""
        )

    def test_bitand_rejects_float_right(self) -> None:
        from fuzzing.wasm_backend import is_available, run_wasm  # noqa: PLC0415

        if not is_available():
            pytest.skip("wasmtime / runtime WASM artefact not available")
        result = run_wasm("expr {99 & 70.34}\n")
        assert result.return_code == 1
        assert 'cannot use floating-point value "70.34" as right operand of "&"' in (
            result.error_message or ""
        )

    def test_lshift_rejects_float_left(self) -> None:
        from fuzzing.wasm_backend import is_available, run_wasm  # noqa: PLC0415

        if not is_available():
            pytest.skip("wasmtime / runtime WASM artefact not available")
        result = run_wasm("expr {-23.55 << 4}\n")
        assert result.return_code == 1
        assert 'cannot use floating-point value "-23.55" as left operand of "<<"' in (
            result.error_message or ""
        )


class TestBatch7WasmAcceptsFloatAsInteger:
    """WASM backend silently truncated float strings in strict-integer ops.

    Issue #262.  ``incr`` is a strict-integer command; the VM rejects
    a value whose string spells a float (``"52.60"``) with
    ``expected integer but got "52.60"``.  The WASM emitter's inlined
    ``IRIncr`` path called ``obj_get_int`` directly, which silently
    truncates ``"52.60"`` to ``52`` and lets the counter advance — a
    runaway-loop driver in the differential fuzzer's ``wasm-timeout``
    cluster.  The fix routes ``IRIncr`` through ``tcl_incr`` and adds
    a strict-integer guard there.
    """

    SEEDS = (1774200084,)

    @pytest.mark.parametrize("seed", SEEDS)
    def test_wasm_errors_on_float_in_incr(self, seed: int) -> None:
        from fuzzing.wasm_backend import is_available, run_wasm  # noqa: PLC0415

        if not is_available():
            pytest.skip("wasmtime / runtime WASM artefact not available")
        source = (FINDINGS_DIR / f"seed_{seed}.tcl").read_text()
        result = run_wasm(source, timeout=5.0)
        assert result.return_code == 1, (
            f"seed {seed}: wasm did not surface a Tcl-level error on a "
            f"script that runs ``incr`` against a float string — "
            f"issue #262 (rc={result.return_code}, "
            f"err={result.error_message!r})"
        )
        assert "expected integer" in (result.error_message or ""), (
            f"seed {seed}: wasm errored but not on the float-as-integer "
            f"path — issue #262 expects ``expected integer`` in the "
            f"message (rc={result.return_code}, "
            f"err={result.error_message!r})"
        )

    def test_incr_rejects_float_string(self) -> None:
        from fuzzing.wasm_backend import is_available, run_wasm  # noqa: PLC0415

        if not is_available():
            pytest.skip("wasmtime / runtime WASM artefact not available")
        result = run_wasm("set i 52.60\nincr i\n")
        assert result.return_code == 1
        assert 'expected integer but got "52.60"' in (result.error_message or "")

    def test_incr_accepts_int_string(self) -> None:
        """Non-regression: ordinary integer strings still increment cleanly."""
        from fuzzing.wasm_backend import is_available, run_wasm  # noqa: PLC0415

        if not is_available():
            pytest.skip("wasmtime / runtime WASM artefact not available")
        result = run_wasm("set i 5\nincr i\nputs $i\n")
        assert result.return_code == 0
        assert result.stdout.strip() == "6"
