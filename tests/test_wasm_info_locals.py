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

"""WASM ``info locals`` / ``info vars`` declaration-order enumeration.

C Tcl enumerates a frame's compiled locals in *declaration* order —
formal parameters first, in the order they appear in the proc / lambda
arg-spec, then the body's other locals.  Our runtime stores locals in a
hash table, so without help they come back in bucket order.  These tests
pin the declaration-order guarantee across both call paths:

* ``apply`` lambdas dispatch through ``eval_apply`` (``runtime/zig``),
  which stamps the lambda's param spec on the frame (apply-8.*).
* compiled procs dispatch through the codegen prologue
  (``compiler/codegen/wasm``), which stamps the proc's param spec when
  the body references ``info locals`` / ``info vars`` — that reference
  also forces the proc its own runtime frame (the substring backstop in
  ``_proc_wants_frame``), without which a nested ``[… [info locals] …]``
  would read an empty / caller frame.

The runtime emits the formals (in declaration order) ahead of the body's
remaining locals; see ``frame_locals_visit_current`` in
``runtime/zig/interp/tcl_frames.zig``.
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run(source: str) -> str:
    wasm = _compile_tcl(source)
    _, stdout = _run_wasm(wasm, capture_stdout=True)
    return stdout.rstrip("\n")


# The upstream apply.test helper (apply-8.*): walk ``info locals``,
# skipping the accumulator, and pair each remaining local with its value.
# Built as a variable so the lambda body reaches the runtime path.
_APPLY_BODY = (
    "set applyBody {set res {}; foreach v [info locals] "
    '{if {$v eq "res"} continue; lappend res [list $v [set $v]]}; set res}'
)


@pytest.mark.parametrize(
    "source,expected",
    [
        # -- apply / eval_apply path: formals in declaration order --------
        ("puts [apply {{x y z} {info locals}} 1 2 3]", "x y z"),
        ("puts [apply {{x args} {info locals}} 1 2 3]", "x args"),
        ("puts [apply {{x {y 2} z} {info locals}} 1 9 3]", "x y z"),
        # ``args`` keeps its declaration slot even when empty.
        ("puts [apply {{x {y 2} args} {info locals}} 1]", "x y args"),
        # -- compiled-proc / codegen prologue path ------------------------
        ("proc p {a b c} {info locals}; puts [p 1 2 3]", "a b c"),
        ("proc p {a {b 2} c} {info locals}; puts [p 1 9 3]", "a b c"),
        (
            "proc p {first second third fourth} {info locals}; puts [p 1 2 3 4]",
            "first second third fourth",
        ),
        # ``info vars`` orders the formals the same way at proc scope.
        ("proc p {m n} {info vars}; puts [p 1 2]", "m n"),
        # Formals precede the body's own locals (declaration order first).
        ("proc p {a b} {set z 9; set out [info locals]; puts $out}; p 1 2", "a b z"),
        # -- nested [info locals]: the substring backstop forces a frame --
        # so the query sees this proc's formals, not an empty / caller one.
        ("proc p {a b} {puts [lindex [info locals] 0]}; p 1 2", "a"),
        ("proc p {a b} {puts [lrange [info locals] 0 1]}; p 1 2", "a b"),
        ("proc p {a b} {puts [llength [info locals]]}; p 1 2", "2"),
        # -- upstream apply.test 8.* semantics verbatim -------------------
        (_APPLY_BODY + r"; puts [apply [list {x args} $applyBody] 1 2]", "{x 1} {args 2}"),
        (
            _APPLY_BODY + r"; puts [apply [list {x args} $applyBody] 1 2 3]",
            "{x 1} {args {2 3}}",
        ),
        (_APPLY_BODY + r"; puts [apply [list {{x 1} {y 2}} $applyBody]]", "{x 1} {y 2}"),
        (_APPLY_BODY + r"; puts [apply [list {x {y 2}} $applyBody] 1]", "{x 1} {y 2}"),
        (
            _APPLY_BODY + r"; puts [apply [list {x {y 2} args} $applyBody] 1]",
            "{x 1} {y 2} {args {}}",
        ),
        (
            _APPLY_BODY + r"; puts [apply [list {x {y 2} args} $applyBody] 1 3]",
            "{x 1} {y 3} {args {}}",
        ),
    ],
)
def test_info_locals_declaration_order(source: str, expected: str) -> None:
    assert _run(source) == expected
