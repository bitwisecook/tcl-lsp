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

"""WASM ``lset`` — out-of-range index errors and append semantics.

Pins the index-bounds contract in ``runtime/zig/valtypes/tcl_list.zig``'s
``lset_recurse`` (and the error propagation in ``runtime/zig/cmds/
list.zig``'s ``eval_lset``), mirroring C Tcl's ``TclLsetFlat``:

* ``idx == length`` *appends* a fresh trailing element / sublist
  (``lset {a b} 2 c`` -> ``a b c``; ``lset {} 0 c`` -> ``c``;
  ``lset x {2 0} v`` grows ``x`` by a new ``{v}`` sublist).
* ``idx < 0`` or ``idx > length`` raises ``index "<word>" out of range``
  using the user's *original* index word (not the resolved integer) and
  leaves the target variable untouched.  An earlier revision silently
  returned the source list unchanged — neither appending nor erroring —
  so ``lset x {1 5} 5`` was a no-op (regexpComp lsetComp-2.8 / 2.9 /
  3.8 / 3.9).
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
        # lsetComp-2.8: list-of-indices form, inner index out of range.
        (
            "set x { {1 2} {3 4} }; puts [list [catch {lset x {1 5} 5} m] $m]",
            '1 {index "5" out of range}',
        ),
        # lsetComp-3.8: flat-args form, same out-of-range index.
        (
            "set x { {1 2} {3 4} }; puts [list [catch {lset x 1 5 5} m] $m]",
            '1 {index "5" out of range}',
        ),
        # Single index past the end errors with that index's word.
        (
            "set x {a b}; puts [list [catch {lset x 5 Z} m] $m]",
            '1 {index "5" out of range}',
        ),
        # Negative index errors.
        (
            "set x {a b}; puts [list [catch {lset x -1 Z} m] $m]",
            '1 {index "-1" out of range}',
        ),
        # ``end`` on an empty list resolves below zero -> out of range,
        # reported with the literal ``end`` word.
        (
            "set x {}; puts [list [catch {lset x end Z} m] $m]",
            '1 {index "end" out of range}',
        ),
    ],
)
def test_lset_out_of_range(source: str, expected: str) -> None:
    assert _run(source) == expected


@pytest.mark.parametrize(
    "source,expected",
    [
        # idx == length appends a fresh element.
        ("set x {a b}; lset x 2 c; puts $x", "a b c"),
        ("set x {}; lset x 0 c; puts <$x>", "<c>"),
        ("set x {a b}; lset x end+1 Z; puts $x", "a b Z"),
        # Nested append grows the outer list by a brand-new sublist.
        ("set x {{a b} {c d}}; lset x {2 0} Z; puts $x", "{a b} {c d} Z"),
        ("set x {a b}; lset x 2 0 Z; puts $x", "a b Z"),
    ],
)
def test_lset_append_at_length(source: str, expected: str) -> None:
    assert _run(source) == expected


@pytest.mark.parametrize(
    "source,expected",
    [
        # Existing in-range writes still work (no regression).
        ("set x {a b c}; lset x 1 Z; puts $x", "a Z c"),
        ("set x {a b c}; lset x end Z; puts $x", "a b Z"),
        ("set x {{1 2} {3 4}}; lset x 1 0 Z; puts $x", "{1 2} {Z 4}"),
        ("set x {{1 2} {3 4}}; lset x {1 0} Z; puts $x", "{1 2} {Z 4}"),
    ],
)
def test_lset_in_range_still_works(source: str, expected: str) -> None:
    assert _run(source) == expected


@pytest.mark.parametrize(
    "source,expected",
    [
        # lsetComp-2.9 / 3.9: on an out-of-range error the variable is
        # left untouched — its exact string representation (spacing and
        # all) survives, and a later read of the second element matches.
        (
            "set ::x { { 1 2 } { 3 4 } }; proc p {} {lset ::x { 1 5 } 5};"
            " catch {p}; puts [list $::x [lindex $::x 1]]",
            "{ { 1 2 } { 3 4 } } { 3 4 }",
        ),
        (
            "set ::x { { 1 2 } { 3 4 } }; proc p {} {lset ::x 1 5 5};"
            " catch {p}; puts [list $::x [lindex $::x 1]]",
            "{ { 1 2 } { 3 4 } } { 3 4 }",
        ),
    ],
)
def test_lset_error_preserves_variable(source: str, expected: str) -> None:
    assert _run(source) == expected
