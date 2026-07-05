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

"""WASM ``return -options DICT`` — order-preserving, recursive merge.

C Tcl's ``TclMergeReturnOptions`` walks the options dict as ordered
key/value *pairs* and recursively flattens a nested ``-options`` key.
``eval_return`` (``runtime/zig/cmds/flow.zig``) previously iterated
``dict_keys`` (which deduplicates), so a dict with repeated/nested
``-options`` keys collapsed to the last value — ``-options {-options
{-foo 1} -options {-bar 2}}`` yielded ``-options {-bar 2}`` instead of a
flattened ``-foo 1 -bar 2``.  Now it applies each pair in order and
recurses on ``-options``.  Regression guard for cmdMZ-return-2.20 / 2.21.

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


@pytest.mark.parametrize(
    "script,expected",
    [
        # cmdMZ-return-2.20: two separate ``-options`` arguments accumulate.
        (
            "return -level 0 -options {-foo 1} -options {-bar 2}",
            "0 {-foo 1 -bar 2 -code 0 -level 0}",
        ),
        # cmdMZ-return-2.21: one ``-options`` whose dict has two nested
        # ``-options`` keys — each is recursively flattened, in order.
        (
            "return -level 0 -options {-options {-foo 1} -options {-bar 2}}",
            "0 {-foo 1 -bar 2 -code 0 -level 0}",
        ),
        # Deeper recursion: ``-options`` nested two levels.
        (
            "return -level 0 -options {-options {-options {-z 9}}}",
            "0 {-z 9 -code 0 -level 0}",
        ),
    ],
)
def test_return_options_recursive_merge(script: str, expected: str) -> None:
    src = f"puts [list [catch {{{script}}} -> f] $f]"
    assert _run_interp(src) == expected
