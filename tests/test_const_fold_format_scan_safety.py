"""Regression guards for ``format`` / ``scan`` constant folding.

The optimiser surfaces ``const_fold`` results as O129 ("Fold constant command
substitution") replacements that *rewrite user source*.  A wrong fold here
silently corrupts code, so the folder must bail (return ``None``) whenever the
true result is version-dependent (32-bit Tcl 9 vs 64-bit Tcl 8 word width) or
otherwise not reproducible without running the interpreter.

These were real bugs: ``[format %x -1]`` folded to ``-1`` (tclsh gives
``ffffffff`` on 9 / ``ffffffffffffffff`` on 8), ``[scan 0xff %x]`` folded to
``0``, ``[format %#o 8]`` folded to ``0o10`` (wrong on 8.x where it is ``010``).

Unlike :mod:`tests.test_const_fold_vs_tcl`, this module has **no** ``tclsh``
dependency — the expected values below were pinned against real ``tclsh8.6``
and ``tclsh9.0`` and are asserted unconditionally so the guard survives in
environments without a Tcl interpreter.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

# Importing ``compiler.core_analyses`` first fully initialises the command
# registry, which otherwise races the ``dialects.tcl`` package import below and
# raises a partially-initialised-module ImportError when this file is collected
# in isolation.
import compiler.core_analyses  # noqa: E402, F401
from dialects.tcl.const_fold import fold_format, fold_scan  # noqa: E402


class TestFormatNeverFoldsWrong:
    """``format`` folds must be byte-identical across Tcl 8.4-9.0 or not fold."""

    @pytest.mark.parametrize(
        ("spec", "arg"),
        [
            # negative %x/%X/%o/%u render as two's-complement at a word width
            # that differs by version; Python's % keeps the sign -> never fold.
            ("%x", "-1"),
            ("%X", "-1"),
            ("%o", "-8"),
            ("%u", "-1"),
            ("%08x", "-1"),
            # the ``#`` alternate-form prefix diverges by version:
            # ``%#o`` 010 (8.x) vs 0o10 (9); ``%#X`` 0XFF (8.x) vs 0xFF (9).
            ("%#o", "8"),
            ("%#X", "255"),
        ],
    )
    def test_version_dependent_specs_bail(self, spec: str, arg: str) -> None:
        assert fold_format((spec, arg)) is None

    @pytest.mark.parametrize(
        ("spec", "arg", "expected"),
        [
            ("%d", "-5", "-5"),  # signed decimal is identical everywhere
            ("%x", "255", "ff"),
            ("%X", "255", "FF"),
            ("%o", "8", "10"),
            ("%u", "5", "5"),  # %u of a non-negative value == %d
            ("%#x", "255", "0xff"),  # 0x prefix is identical across versions
            ("%05d", "7", "00007"),
            ("%+d", "42", "+42"),
            ("%5.2f", "3.14159", " 3.14"),
        ],
    )
    def test_safe_specs_still_fold(self, spec: str, arg: str, expected: str) -> None:
        assert fold_format((spec, arg)) == expected


class TestScanNeverFoldsWrong:
    """``scan`` folds must match the interpreter or bail."""

    def test_unsigned_conversion_bails(self) -> None:
        # %u has version-specific unsigned-width semantics -> never fold.
        assert fold_scan(("-5", "%u")) is None
        assert fold_scan(("5", "%u")) is None

    @pytest.mark.parametrize(
        ("string", "spec", "expected"),
        [
            ("0xff", "%x", "255"),  # optional 0x prefix honoured
            ("0X1f", "%x", "31"),  # uppercase 0X prefix too
            ("0xff", "%X", "255"),
            ("ff", "%x", "255"),  # bare hex still works
            ("0", "%x", "0"),  # a lone 0 is not a truncated prefix
            ("+ff", "%x", "255"),  # leading sign accepted
            ("010", "%o", "8"),  # octal, leading 0 is a digit not a prefix
            ("42", "%d", "42"),
            ("-5", "%d", "-5"),
            ("A", "%c", "65"),
        ],
    )
    def test_integer_conversions(self, string: str, spec: str, expected: str) -> None:
        assert fold_scan((string, spec)) == expected
