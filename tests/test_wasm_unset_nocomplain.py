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

"""WASM ``unset`` option parsing — restrictive ``-nocomplain`` / ``--``.

``Tcl_UnsetObjCmd`` (generic/tclVar.c) recognises ``-nocomplain`` ONLY as
the leading argument and consumes it *exactly once*; a second
``-nocomplain`` is a variable *name*, not a repeated option.  The
interpreted ``unset`` handler (``runtime/zig/cmds/var.zig``) previously
looped consuming every leading ``-nocomplain``, so
``unset -nocomplain -nocomplain`` ate both and left the variable named
``-nocomplain`` in place.  Regression guard for set-old-7.16 / 7.17.

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


def test_unset_nocomplain_set_old_7_16() -> None:
    # set-old-7.16 verbatim: the option is consumed once; the second
    # ``-nocomplain`` is the name of the variable that gets unset.
    src = (
        "set -nocomplain abc\n"
        "set var abc\n"
        "puts [list [info exists bogus] "
        "[catch {unset -nocomplain bogus var bogus}] "
        "[info exists -nocomplain] [info exists var] "
        "[catch {unset -nocomplain -nocomplain}] [info exists -nocomplain]]"
    )
    assert _run_interp(src) == "0 0 1 0 0 0"


def test_unset_nocomplain_no_abbreviation() -> None:
    # set-old-7.17: ``-nocomp`` is not an abbreviation of the option —
    # it names a variable, which exists and is unset cleanly.
    src = (
        "set -nocomp abc\n"
        "puts [list [info exists -nocomp] [catch {unset -nocomp}] "
        "[info exists -nocomp]]"
    )
    assert _run_interp(src) == "1 0 0"


def test_unset_lone_nocomplain_is_noop() -> None:
    # ``unset -nocomplain`` with no following names is a no-op (returns
    # OK), and does not touch a same-named variable.
    src = "set -nocomplain keep\nputs [list [catch {unset -nocomplain}] [info exists -nocomplain]]"
    assert _run_interp(src) == "0 1"


def test_unset_double_dash_ends_options() -> None:
    # ``unset -- -nocomplain`` treats ``-nocomplain`` as a name to unset.
    src = "set -nocomplain v\nputs [list [catch {unset -- -nocomplain}] [info exists -nocomplain]]"
    assert _run_interp(src) == "0 0"
