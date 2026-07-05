# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Tests for compiler.scan_format -- the Tcl scan-format static no-match analyser.

Ground truth is real tclsh 9.0.3 behaviour: each ``[scan INPUT FORMAT
varName]`` returns the number of conversions on success and 0 (with
``varName`` unset) on first-conversion failure.
"""

from __future__ import annotations

from compiler.scan_format import scan_provably_no_match


class TestDecimalInt:
    def test_no_digit_in_input(self):
        # tclsh: scan "abc" "%d" n -> 0 (n unset)
        assert scan_provably_no_match("%d", "abc") is True

    def test_leading_letters_then_digit(self):
        # %d doesn't skip non-whitespace; "a1" fails to consume.
        # tclsh: scan "a1" "%d" n -> 0
        assert scan_provably_no_match("%d", "a1") is True

    def test_leading_digit_matches(self):
        # tclsh: scan "42abc" "%d" n -> 1 (n=42)
        assert scan_provably_no_match("%d", "42abc") is False

    def test_signed_integer_matches(self):
        # tclsh: scan "-7" "%d" n -> 1 (n=-7)
        assert scan_provably_no_match("%d", "-7") is False

    def test_leading_whitespace_skipped(self):
        # %d skips leading whitespace; "   42" matches.
        assert scan_provably_no_match("%d", "   42") is False

    def test_only_whitespace(self):
        # No digit reachable after ws skip -> no match.
        assert scan_provably_no_match("%d", "   ") is True

    def test_empty_input(self):
        assert scan_provably_no_match("%d", "") is True


class TestHexInt:
    def test_no_hex_digit(self):
        # tclsh: scan "zzz" "%x" n -> 0
        assert scan_provably_no_match("%x", "zzz") is True

    def test_hex_digit_matches(self):
        # tclsh: scan "ff" "%x" n -> 1 (n=255)
        assert scan_provably_no_match("%x", "ff") is False


class TestString:
    def test_empty_input(self):
        # %s requires at least one non-whitespace char.
        assert scan_provably_no_match("%s", "") is True

    def test_only_whitespace(self):
        # After ws skip, nothing remains -- no match.
        assert scan_provably_no_match("%s", "   ") is True

    def test_non_space_matches(self):
        # tclsh: scan "hello" "%s" w -> 1 (w="hello")
        assert scan_provably_no_match("%s", "hello") is False


class TestChar:
    def test_any_char_matches(self):
        # %c consumes one char regardless of contents.
        assert scan_provably_no_match("%c", "a") is False

    def test_empty_input_fails(self):
        assert scan_provably_no_match("%c", "") is True

    def test_does_not_skip_whitespace(self):
        # %c does NOT skip leading whitespace; it matches the first
        # character even if that's a space.
        assert scan_provably_no_match("%c", " a") is False


class TestFloat:
    def test_digit_matches(self):
        # tclsh: scan "3.14" "%f" f -> 1 (f=3.14)
        assert scan_provably_no_match("%f", "3.14") is False

    def test_leading_dot_with_digit(self):
        # tclsh: scan ".5" "%f" f -> 1 (f=0.5)
        assert scan_provably_no_match("%f", ".5") is False

    def test_non_numeric(self):
        # tclsh: scan "abc" "%f" f -> 0
        assert scan_provably_no_match("%f", "abc") is True

    def test_signed_matches(self):
        # tclsh: scan "-3.14" "%f" f -> 1 (f=-3.14)
        assert scan_provably_no_match("%f", "-3.14") is False


class TestLiteralInFormat:
    def test_literal_match(self):
        # "X42" matches format "X%d" because the X is consumed literally
        # then %d consumes 42.
        assert scan_provably_no_match("X%d", "X42") is False

    def test_literal_mismatch(self):
        # tclsh: scan "Y42" "X%d" n -> 0 (the X doesn't match Y).
        # First conversion never reached -- the literal failed.
        assert scan_provably_no_match("X%d", "Y42") is True


class TestSuppressedAndWidth:
    def test_suppressed_conversion_still_modeled(self):
        # ``%*d`` still has to consume input -- we still need a digit.
        assert scan_provably_no_match("%*d", "abc") is True

    def test_width_modifier_skipped(self):
        # ``%5d`` still requires a leading digit (or sign).
        assert scan_provably_no_match("%5d", "abc") is True
        assert scan_provably_no_match("%5d", "42") is False


class TestUncertainCases:
    def test_character_set_returns_false_conservatively(self):
        # ``%[abc]`` is a character set; we don't parse it, so we
        # return False (no proof of no-match).  This avoids false
        # W210 firings.
        assert scan_provably_no_match("%[abc]", "xyz") is False

    def test_format_with_no_conversion(self):
        # Pure literal format -- no varName is bound, so there's
        # no var to be "unset".  We return False.
        assert scan_provably_no_match("hello", "hello") is False

    def test_unknown_conversion(self):
        # Unknown letter -- conservative False.
        assert scan_provably_no_match("%z", "abc") is False
