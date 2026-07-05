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

"""WASM ``regexp`` / ``regsub`` — empty-subject and empty-pattern matching.

Pins two zero-width contracts in ``runtime/zig/valtypes/tcl_regex.zig``:

* ``eval_regexp_cmd``'s ``-all`` loop must attempt a match at ``offset``
  BEFORE testing ``offset >= stringLength`` — mirroring the bottom-of-loop
  break in ``Tcl_RegexpObjCmd``.  An earlier revision hoisted that test to
  the top of the loop, so an empty subject (``offset == length == 0``)
  never tried a single match and every capture / matchVar / ``-inline`` /
  ``-indices`` form returned 0 against ``""`` even when the pattern matches
  the empty string.  Regression guard for regexp-1.5, regexp-23.1, and
  regexpComp-1.5; ``regexp {(x*)y*} {} whole`` even left the matchVar unset
  (surfacing downstream as ``can't read "whole"``).

* ``eval_regsub_cmd``'s suffix loop uses the inclusive ``offset <= wlen``
  bound so anchors like ``$`` and patterns like ``x*`` still match at EOF.
  But ``Tcl_RegsubObjCmd`` routes ``-all`` + an empty pattern + a literal
  subSpec through a string-map fast path that matches strictly *between*
  characters — ``regsub -all {} foo bar`` yields ``barfbarobaro`` (three
  matches, none at the end), and ``regsub -all {} {} bar`` yields zero.
  Regression guard for regexpComp-21.10 / 21.11.
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run(source: str) -> str:
    wasm = _compile_tcl(source)
    _, stdout = _run_wasm(wasm, capture_stdout=True)
    return stdout.rstrip("\n")


@pytest.mark.parametrize(
    "source,expected",
    [
        # regexp-1.5: a capture pattern that matches the empty string,
        # WITH a matchVar, against an empty subject — returns 1, not 0.
        ("puts [regexp {^([^ ]*)[ ]*([^ ]*)} {} a]", "1"),
        # The matchVar must be set to the (empty) whole match rather than
        # left unset — ``$a`` reads back as the empty string.
        ('regexp {^([^ ]*)} {} a; puts "<$a>"', "<>"),
        ('regexp {(x*)y*} {} whole; puts "<$whole>"', "<>"),
        # A bare boolean form already worked; keep it pinned both ways.
        ("puts [regexp {a*} {}]", "1"),
        ("puts [regexp {a*} {} m]", "1"),
        ("puts [regexp {a} {} m]", "0"),
        # regexp-22.1 (Bug 1810038): alternation with anchors on empty.
        ("puts [regexp {($|^X)*} {}]", "1"),
    ],
)
def test_regexp_empty_subject_with_capture(source: str, expected: str) -> None:
    assert _run(source) == expected


@pytest.mark.parametrize(
    "source,expected",
    [
        # regexp-23.1: -all -inline -indices over an empty subject yields a
        # single zero-width match at {0 -1} for ^, ^$, and $.
        ("puts [regexp -all -inline -indices -line -- {^} {}]", "{0 -1}"),
        ("puts [regexp -all -inline -indices -line -- {^$} {}]", "{0 -1}"),
        ("puts [regexp -all -inline -indices -line -- {$} {}]", "{0 -1}"),
        # -inline -indices for an empty pattern / anchor on empty subject.
        ("puts [regexp -inline -indices {} {}]", "{0 -1}"),
        ("puts [regexp -inline -indices {^} {}]", "{0 -1}"),
        # -all -inline empty matches: one per character position (the EOF
        # position is NOT counted — regexp-18.x family stays intact).
        ("puts [regexp -all -inline -indices {} foo]", "{0 -1} {1 0} {2 1}"),
        # ``a*`` over ``a`` must not log a spurious trailing empty match.
        ("puts [regexp -all -inline {a*} a]", "a"),
    ],
)
def test_regexp_empty_inline_indices(source: str, expected: str) -> None:
    assert _run(source) == expected


@pytest.mark.parametrize(
    "source,expected",
    [
        # regexpComp-21.10: empty pattern, literal subSpec — matches
        # between each char (count == length), never at EOF.
        ('set s {}; puts "[regsub -all {} foo bar s]:$s"', "3:barfbarobaro"),
        ('set s {}; puts "[regsub -all {} foo X s]:$s"', "3:XfXoXo"),
        # regexpComp-21.11: empty pattern over an empty subject — zero
        # matches, result unchanged.
        ('set s {}; puts "[regsub -all {} {} bar s]:$s"', "0:"),
        # A single-char subject: one match before the char.
        ('set s {}; puts "[regsub -all {} a X s]:$s"', "1:Xa"),
        # Non -all empty pattern matches once at the start (unchanged).
        ('set s {}; puts "[regsub {} foo X s]:$s"', "1:Xfoo"),
        # A subSpec containing ``&`` disqualifies the fast path — the
        # general loop runs and DOES take the trailing empty match (four).
        ('set s {}; puts "[regsub -all {} foo {&X} s]:$s"', "4:XfXoXoX"),
    ],
)
def test_regsub_empty_pattern(source: str, expected: str) -> None:
    assert _run(source) == expected


@pytest.mark.parametrize(
    "source,expected",
    [
        # Anchors and ``x*`` still match at EOF in regsub (inclusive bound
        # preserved for non-empty-pattern cases).
        ('set s {}; puts "[regsub -all -- {$} foo X s]:$s"', "1:fooX"),
        ('set s {}; puts "[regsub -all -- ^ {} X s]:$s"', "1:X"),
        ('set s {}; puts "[regsub -all -- {x*} foo X s]:$s"', "4:XfXoXoX"),
        # A non-empty literal pattern is unaffected.
        ('set s {}; puts "[regsub -all o foo X s]:$s"', "2:fXX"),
    ],
)
def test_regsub_anchor_at_eof_preserved(source: str, expected: str) -> None:
    assert _run(source) == expected
