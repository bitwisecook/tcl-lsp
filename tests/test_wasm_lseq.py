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

"""WASM ``lseq`` — double arithmetic series and the ``count`` keyword.

``lseq`` was integer-only.  It now:

* generates a double series when START, END, or STEP is a double literal
  (Tcl ``arithSeriesDouble``), rounding each element to the inputs' max
  decimal precision (Tcl ``ArithRound``) so ``lseq 4 40 0.1`` prints
  ``6.3`` rather than the f64 noise ``6.300000000000001``;
* supports the ``count`` keyword (``lseq START count N ?by? ?STEP?``);
* the ``count`` value's type never forces double output (``lseq 0 count
  3.0`` is integer ``{0 1 2}``; ``lseq 0.0 count 3.0`` is double);
* returns an empty list rather than materialising a multi-million
  element series (there is no lazy ArithSeries rep here).
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run(body: str) -> str:
    _, stdout = _run_wasm(_compile_tcl(body), capture_stdout=True, timeout_s=15.0)
    return stdout.rstrip("\n")


@pytest.mark.parametrize(
    "expr,expected",
    [
        # double series, inferred / explicit / negative step
        ("lseq 5.0 to 15.", "5.0 6.0 7.0 8.0 9.0 10.0 11.0 12.0 13.0 14.0 15.0"),
        ("lseq 5.0 to 25. by 5", "5.0 10.0 15.0 20.0 25.0"),
        ("lseq 0.0 2.0", "0.0 1.0 2.0"),
        # fractional step with consistent rounding (lseq-4.14 head)
        ("lseq 4 4.5 0.1", "4.0 4.1 4.2 4.3 4.4 4.5"),
        # count keyword — integer and double
        ("lseq 5 count 5 2", "5 7 9 11 13"),
        ("lseq 0 count 3", "0 1 2"),
        ("lseq 0 count 3.0", "0 1 2"),
        ("lseq 0.0 count 3.0", "0.0 1.0 2.0"),
        ("lseq 0 count 3 by 1.0", "0.0 1.0 2.0"),
        # integer forms unchanged
        ("lseq 5", "0 1 2 3 4"),
        ("lseq 1 5", "1 2 3 4 5"),
        ("lseq 10 to 2 by -2", "10 8 6 4 2"),
    ],
)
def test_lseq_double_and_count(expr: str, expected: str) -> None:
    assert _run(f"puts [{expr}]") == expected


def test_lseq_oversized_series_is_empty_not_hang() -> None:
    # No lazy series rep — a multi-million element double request returns
    # empty rather than OOM/hang.
    assert _run("puts [llength [lseq 0.0 1e7]]") == "0"


@pytest.mark.parametrize(
    "body,expected",
    [
        # Operands are evaluated as expressions — ``double(...)`` coerces
        # the whole series to floating point (lseq-1.25).
        ("puts [lseq double(0) 2]", "0.0 1.0 2.0"),
        ("puts [lseq 0 double(2)]", "0.0 1.0 2.0"),
        ("puts [lseq 0 count 3 by double(1)]", "0.0 1.0 2.0"),
        # Arithmetic-string operands (lseq-1.26).
        ("puts [lseq 1 {3.0+0}]", "1.0 2.0 3.0"),
        ("puts [lseq 1 [expr {3.0 + 0}] 1]", "1.0 2.0 3.0"),
        # Single-argument count is also an expression (lseq-2.19).
        ("puts [lseq {1+1}]", "0 1"),
        ("puts [lseq {1+1} {5+5} {2+2}]", "2 6 10"),
        ("puts [lseq {1+1} count {2+2} by {2+2}]", "2 6 10 14"),
        # The count expression is evaluated exactly once (lseq-2.20).
        ("set i 1; set r [lseq {[incr i]}]; puts [list $r $i]", "{0 1} 2"),
        ("set i 1; set r [lseq {0 + [incr i]}]; puts [list $r $i]", "{0 1} 2"),
    ],
)
def test_lseq_expression_operands(body: str, expected: str) -> None:
    assert _run(body) == expected
