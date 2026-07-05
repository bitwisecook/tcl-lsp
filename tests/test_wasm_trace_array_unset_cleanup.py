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

"""Unsetting a whole array drops its element traces.

Tcl's ``TclDeleteVars`` removes traces on an array AND on every element
when the array is unset.  Our ``remove_all_var_traces`` only matched the
exact directory key, so an element ``read``/``write`` trace survived
``unset arr`` and re-fired on the next ``set arr(k)`` — trace.test 12.3
saw the write trace fire three times (once per accumulated leftover from
12.1/12.2), and 14.18/14.19 saw ``trace info variable x(0)`` report a
stale trace after ``unset x``.

These run INTERPRETED (``eval``) so the variable-trace dispatch is
exercised the same way trace.test runs.  Reference: Tcl 9 ``tclVar.c``
(``TclDeleteVars`` / ``Tcl_UnsetVar2``).
"""

from __future__ import annotations

import pytest

pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run_interp(body: str) -> str:
    src = "set __s {" + body + "}\neval $__s\n"
    _, stdout = _run_wasm(_compile_tcl(src), capture_stdout=True)
    return stdout.rstrip("\n")


_TP = "proc traceProc {name1 name2 op} {global info; lappend info $name1 $name2 $op}\n"


def test_element_write_trace_dropped_on_array_unset_no_accumulation() -> None:
    # Mirror trace-12.1 -> 12.2 -> 12.3: each test re-adds an element
    # write trace after ``unset -nocomplain x``.  The leftover traces must
    # not accumulate — the final write fires exactly once.
    body = (
        _TP + "unset -nocomplain x\nset info {}\n"
        "trace add variable x(0) write traceProc\ncatch {set x 22}\n"
        "unset -nocomplain x\nset info {}\n"
        "trace add variable x(0) write traceProc\ncatch {set x(0)}\n"
        "unset -nocomplain x\nset info {}\n"
        "trace add variable x(0) write traceProc\nset x(0) 22\n"
        "puts [list $info]"
    )
    assert _run_interp(body) == "{x 0 write}"


def test_single_write_after_array_unset_then_retrace() -> None:
    body = (
        _TP + "set x(0) 1\ntrace add variable x(0) write traceProc\n"
        "unset x\nset info {}\n"
        "trace add variable x(0) write traceProc\nset x(0) 9\nputs [list $info]"
    )
    assert _run_interp(body) == "{x 0 write}"


def test_trace_info_element_empty_after_array_unset() -> None:
    # trace-14.18 / 14.19: ``trace info variable x(0)`` is empty once the
    # array has been unset, even if a prior element trace existed.
    body = (
        _TP + "set x(0) 1\ntrace add variable x(0) read traceProc\n"
        "unset -nocomplain x\nputs [list [trace info variable x(0)]]"
    )
    assert _run_interp(body) == "{}"


def test_scalar_unset_still_drops_its_trace() -> None:
    # Guard: the array-element sweep must not disturb plain scalar trace
    # removal on unset.
    body = (
        _TP + "set x 1\ntrace add variable x write traceProc\n"
        "unset x\nset info {}\n"
        "trace add variable x write traceProc\nset x 9\nputs [list $info]"
    )
    assert _run_interp(body) == "{x {} write}"


def test_unrelated_array_element_trace_survives() -> None:
    # Guard: unsetting array ``x`` must NOT drop a trace on ``xx(0)`` —
    # the prefix match requires the ``(`` to fall exactly after the array
    # name, so ``xx`` is not an element of ``x``.
    body = (
        _TP + "set xx(0) 1\ntrace add variable xx(0) write traceProc\n"
        "set x(0) 1\nunset x\nset info {}\n"
        "set xx(0) 9\nputs [list $info]"
    )
    assert _run_interp(body) == "{xx 0 write}"
