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

"""WASM ``array set`` through a top-level ``upvar 0`` alias.

At the script top level (no proc frame) ``upvar 0 target alias`` records
the alias as a ``VAR_LINK`` in the root namespace's Var table rather than
as a frame ALIAS_EXT descriptor.  ``frame_resolve_array_name``
(``runtime/zig/interp/tcl_frames.zig``) followed frame descriptors but
not these root links, so ``array set alias …`` minted a *separate* array
under the alias name instead of populating the target.  The read-error
suffix picker (``pick_var_read_suffix`` in
``runtime/zig/interp/tcl_catch.zig``) likewise probed the raw alias name,
so a whole-array read of the alias reported "no such variable" instead of
"variable is array".  Both now follow the root link.  Regression guard
for set-old-8.38.2.

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


def test_array_set_through_upvar0_alias_set_old_8_38_2() -> None:
    # set-old-8.38.2 verbatim: the array lands on the target, and a
    # whole-array read of the alias reports "variable is array".
    src = (
        "catch {unset aVaRnAmE}\n"
        "upvar 0 aVaRnAmE anAliAs\n"
        "array set anAliAs {}\n"
        "puts [list [array exists aVaRnAmE] [catch {set anAliAs} msg] $msg]"
    )
    assert _run_interp(src) == '1 1 {can\'t read "anAliAs": variable is array}'


def test_array_populated_through_upvar0_alias() -> None:
    # Populating through the alias is visible under BOTH names.
    src = (
        "catch {unset av}\n"
        "upvar 0 av al\n"
        "array set al {a 1 b 2}\n"
        "puts [list [array exists av] [lsort [array names av]] "
        "[lsort [array names al]] $av(a) $al(b)]"
    )
    assert _run_interp(src) == "1 {a b} {a b} 1 2"


def test_plain_toplevel_array_unaffected() -> None:
    # A plain (un-aliased) top-level array must still resolve normally.
    src = (
        "catch {unset arr}\n"
        "array set arr {x 1 y 2}\n"
        "puts [list [array exists arr] [lsort [array names arr]]]"
    )
    assert _run_interp(src) == "1 {x y}"
