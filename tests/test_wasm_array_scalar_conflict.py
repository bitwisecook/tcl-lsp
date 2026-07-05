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

"""WASM array-vs-scalar conflict detection (read + write sides).

Two related fixes:

* Empty ``array set X {}`` must materialise the (empty) array so
  ``info exists X`` / ``array exists X`` are true and a later scalar
  read of ``X`` reports ``can't read "X": variable is array`` rather
  than ``no such variable``.  ``array_set_list`` (runtime/zig/valtypes/
  tcl_array.zig) validated the scalar conflict but never created the
  table on the empty-list path (set-old-8.38).

* Writing an element of a name that already exists as a SCALAR raises
  ``can't set "<arr(key)>": variable isn't array`` with the user's full
  spelling.  ``var_set`` (runtime/zig/interp/tcl_frames.zig) routed the
  write straight to ``array_set`` (which only produced a truncated
  ``can't set: variable isn't array`` for the global case and silently
  succeeded for a proc-local scalar).  The probe is scope-correct: a
  same-named GLOBAL scalar must NOT block ``set x(1)`` inside a proc,
  which creates a fresh local array (set-old-6.2, set-old-12.2,
  regexpComp-6.8/11.7).

The slice runs test bodies INTERPRETED (``eval_script``), so these force
that path via ``set s {…}; eval $s``.
"""

from __future__ import annotations

import pytest

pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run_interp(source: str) -> str:
    wrapped = "set __s {" + source + "}\neval $__s\n"
    _, stdout = _run_wasm(_compile_tcl(wrapped), capture_stdout=True)
    return stdout.rstrip("\n")


# --- read side: empty array set creates the array -------------------------


def test_empty_array_set_creates_array() -> None:
    src = 'array set arr {}\nputs "[info exists arr] [array exists arr] [catch {set arr} m]:$m"'
    assert _run_interp(src) == '1 1 1:can\'t read "arr": variable is array'


def test_array_set_on_scalar_still_errors() -> None:
    src = 'set s 5\nputs "[catch {array set s {}} m]:$m"'
    assert _run_interp(src) == "1:can't array set \"s\": variable isn't array"


# --- write side: element write on a scalar --------------------------------


def test_set_element_on_global_scalar() -> None:
    # set-old-6.2 — full-name message, not the truncated form.
    src = 'set a xxx\nputs "[catch {set a(14) 186} m]:$m"'
    assert _run_interp(src) == "1:can't set \"a(14)\": variable isn't array"


def test_set_element_on_proc_local_scalar() -> None:
    # regexpComp-6.8/11.7 — a proc-local scalar must be detected too.
    src = "puts [apply {{} {set f1 44; catch {set f1(f2) 1} m; return $m}}]"
    assert _run_interp(src) == "can't set \"f1(f2)\": variable isn't array"


def test_global_scalar_does_not_block_local_array() -> None:
    # set-old-12.2 — `set x(1)` in a proc creates a fresh LOCAL array even
    # when a global scalar `x` exists; it must not raise.
    src = "set x global-scalar\nproc foo {} {set x(1) 23456}\nputs [foo]"
    assert _run_interp(src) == "23456"


def test_fresh_array_writes_unaffected() -> None:
    src = (
        "catch {unset n}\nset n(k) v\n"
        "proc bar {} {set y(1) 99; return $y(1)}\n"
        'puts "[array get n] [bar]"'
    )
    assert _run_interp(src) == "k v 99"
