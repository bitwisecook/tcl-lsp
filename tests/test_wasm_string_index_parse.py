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

"""WASM ``Tcl_GetIntForIndex`` parsing — whitespace, signed offsets, ``0d``.

``string index`` (and every index-taking command, via the shared
``resolve_list_index``) must parse the Tcl 9 index grammar:

* leading / trailing whitespace is skipped (``{ 0 }`` / ``{end-1 }``);
* an offset after ``end±`` / between ``±`` may itself be signed
  (``end+-1`` = end + (-1), ``end--1`` = end - (-1), ``-1--2`` = 1);
* integer literals accept the ``0d`` explicit-decimal radix alongside
  ``0x`` / ``0o`` / ``0b``;
* out-of-range / overflowing indices yield the empty string.

Regression guard for the util-9.* cluster.
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run(body: str) -> str:
    _, stdout = _run_wasm(_compile_tcl(body), capture_stdout=True)
    return stdout.rstrip("\n")


@pytest.mark.parametrize(
    "expr,expected",
    [
        ("string index abcd { 0 }", "a"),
        ("string index abcd { 3 }", "d"),
        ("string index abcd {end-1 }", "c"),
        ("string index abcd { 0x0 }", "a"),
        ("string index abcd { 01 }", "b"),
        ("string index abcd { 0d0 }", "a"),
        ("string index abcdefghijk 0d10", "k"),
        ("string index abcd { -1+2 }", "b"),
        # signed offsets after the operator
        ("string index abcd end+-1", "c"),
        ("string index abcd -1--2", "b"),
        # out-of-range / overflow -> empty string
        ("string index abcd end--1", ""),
        ("string index abcd -0x10000000000000000", ""),
        ("string index abcd end+-0x10000000000000000", ""),
    ],
)
def test_string_index_grammar(expr: str, expected: str) -> None:
    assert _run(f"puts [{expr}]") == expected


@pytest.mark.parametrize(
    "idx,catch_expected",
    [
        # single-operator signed forms remain valid (resolver folds them)
        ("-1+2", "0:b"),
        ("end+-1", "0:c"),
        ("-1--2", "0:b"),
        # chained operators exceed the resolver's single-operator fold, so
        # the validator must reject them as a bad index rather than let
        # them silently resolve to the wrong position (PR #516 review).
        ("end--1+2", '1:bad index "end--1+2": must be integer?[+-]integer? or end?[+-]integer?'),
        ("-1--2+3", '1:bad index "-1--2+3": must be integer?[+-]integer? or end?[+-]integer?'),
    ],
)
def test_string_index_rejects_operator_chains(idx: str, catch_expected: str) -> None:
    # A dynamic subcommand forces the runtime ``string`` command (and its
    # ``is_valid_string_index`` validator) rather than the compiled
    # direct-import path.
    body = f"set sub index\nputs [catch {{string $sub abcd {idx}}} m]:$m"
    assert _run(body) == catch_expected


@pytest.mark.parametrize(
    "expr,expected",
    [
        # list commands validate via is_valid_list_index before the shared
        # resolver; it must accept the same whitespace / 0d forms (PR #516).
        ("lindex {a b c d} { 1 }", "b"),
        ("lindex {a b c d} 0d2", "c"),
        ("lindex {a b c d} {end-1 }", "c"),
        ("lrange {a b c d} { 1 } { 2 }", "b c"),
        ("lindex {a b c d} { 0d10 }", ""),
    ],
)
def test_list_index_grammar(expr: str, expected: str) -> None:
    assert _run(f"puts [{expr}]") == expected
