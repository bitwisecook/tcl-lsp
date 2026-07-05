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

"""Runtime tests for multi-value ``linsert`` / ``lreplace``.

Before this work the compiler truncated multi-value forms to the
first value because the runtime signature is fixed-arity.  These
tests pin down the full semantics as compiler-side chaining:

* ``linsert`` with N values chains N inserts at the same index in
  reverse order, so the final layout is ``{before} v1 v2 ... vN
  {after}``.
* ``lreplace`` with N values replaces the range with the *last*
  value, then inserts the earlier values at ``first`` in reverse.

Both shapes exercise compile-time index resolution that works for
``end`` / ``end-N`` as well as plain integers — because the index
literal is emitted once per runtime call, no compile-time
arithmetic is involved.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from tests.test_wasm_real_tcl import _compile_tcl_with_diag, _run_wasm


def _run(src: str) -> str:
    wasm, _ = _compile_tcl_with_diag(src)
    _, stdout = _run_wasm(wasm, capture_stdout=True)
    return stdout


class TestLinsertMultiValue:
    def test_two_values(self) -> None:
        out = _run("puts [linsert {a b c} 1 X Y]\n")
        assert out == "a X Y b c\n"

    def test_three_values(self) -> None:
        out = _run("puts [linsert {a b c d} 2 X Y Z]\n")
        assert out == "a b X Y Z c d\n"

    def test_insert_at_start(self) -> None:
        out = _run("puts [linsert {a b} 0 X Y]\n")
        assert out == "X Y a b\n"

    def test_insert_at_end(self) -> None:
        out = _run("puts [linsert {a b c} end X Y]\n")
        assert out == "a b c X Y\n"

    def test_insert_with_space_values(self) -> None:
        out = _run('puts [linsert {a b} 1 "hi there" Z]\n')
        assert out == "a {hi there} Z b\n"


class TestLreplaceMultiValue:
    def test_replace_with_two_values(self) -> None:
        out = _run("puts [lreplace {a b c d} 1 2 X Y]\n")
        assert out == "a X Y d\n"

    def test_replace_with_three_values(self) -> None:
        out = _run("puts [lreplace {a b c} 0 0 X Y Z]\n")
        assert out == "X Y Z b c\n"

    def test_replace_with_space_values(self) -> None:
        out = _run('puts [lreplace {a b c} 1 1 "x y" Z]\n')
        assert out == "a {x y} Z c\n"

    def test_replace_end_index(self) -> None:
        out = _run("puts [lreplace {a b c d} end end X Y]\n")
        assert out == "a b c X Y\n"


class TestLinsertStatementContext:
    def test_linsert_in_set_uses_all_values(self) -> None:
        # The statement-context emit path mirrors the value-context
        # chain above; verify parity by exercising the ``set x
        # [linsert ...]`` shape.
        out = _run("set x [linsert {a b} 1 X Y Z]\nputs $x\n")
        assert out == "a X Y Z b\n"
