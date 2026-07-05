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

"""Scope declarations survive builtin distrust (global/variable/upvar).

A dynamic ``rename $x $y`` distrusts every builtin command in the unit
(soundly — the rename *could* hit any of them), which routes their
statement emission through the interpreter eval-fallback instead of the
compiled fast path.  Scope declarations (``global`` / ``variable`` /
``upvar``) must be **exempt**: their compiled hooks don't fold a value,
they record the variable's out-of-frame aliasing in the codegen's
var-model (``emitter._globals`` / ``emitter._aliases``) — the same fact
the flow-sensitive ``var_observability`` lattice marks syntactically for
``::global`` / ``::variable`` / ``::upvar``.

When the declaration was wrongly skipped, the variable was mistaken for a
frame-local: its frame alias was never installed and a later compiled
write landed in a local slot instead of the global.  The symptom was a
*dropped global write* after a distrusted / ``{*}`` statement — e.g.
trace.test's ``traceDelete`` losing its ``set info [...]`` and the whole
trace stem trapping.

See ``_SCOPE_DECL_COMMANDS`` in
``compiler/codegen/wasm/_emitter/_statements.py``.
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402

# A dynamic rename in the unit distrusts every builtin from here on.
_DISTRUST = "set q junk; catch {rename $q other}\n"


def _run(source: str) -> str:
    wasm = _compile_tcl(_DISTRUST + source)
    _, stdout = _run_wasm(wasm, capture_stdout=True)
    return stdout.rstrip("\n")


@pytest.mark.parametrize(
    "body,expected",
    [
        # A {*}-expansion statement before the global write must not
        # demote ``info`` to a frame-local.
        (
            "set x {*}[lindex [list a b] 0]; global info; set info HELLO",
            "HELLO",
        ),
        # Declaration order (global first) is equally affected.
        (
            "global info; set x {*}[lindex [list a b] 0]; set info HELLO",
            "HELLO",
        ),
        # Plain distrusted body with a global write still reaches the global.
        ("global info; set info HELLO", "HELLO"),
        # ``variable`` declarations alias an enclosing namespace the same way.
        ("variable info; set info HELLO", "HELLO"),
    ],
)
def test_global_write_survives_distrust(body: str, expected: str) -> None:
    src = f"set info {{}}\nproc cb {{}} {{ {body} }}\ncb\nputs <$info>\n"
    assert _run(src) == f"<{expected}>"


def test_array_element_unset_survives_distrust() -> None:
    # ``set a(k) v`` compiles to an unconditional array write; ``unset``
    # must stay on the compiled path too, or the interpreter fallback
    # can't see the element to remove it (set-old-2.9).
    src = "set a(1) x; set a(2) y; set a(3) z\nunset a(2)\nputs <[lsort [array names a]]>\n"
    assert _run(src) == "<1 3>"


def test_scalar_unset_survives_distrust() -> None:
    src = "set z keep; set d drop\nunset d\nputs <[info exists z][info exists d]>\n"
    assert _run(src) == "<10>"


def test_upvar_alias_survives_distrust() -> None:
    # ``upvar`` into the caller frame must still route the write to the
    # caller's variable under distrust.
    src = (
        "proc setter {name val} { upvar 1 $name v; set v $val }\n"
        "proc outer {} { set target START; setter target DONE; return $target }\n"
        "puts <[outer]>\n"
    )
    assert _run(src) == "<DONE>"
