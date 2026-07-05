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

"""WASM ``string is`` command tests.

Pins the contracts added for tier-5 ``string is`` parity:

* canonical class disambiguation (``bad class`` / ``ambiguous class``)
* ``-strict`` semantics for empty input
* ``string is dict`` with even-element-count check
* ``string is true`` / ``string is false`` / ``string is wordchar``
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run(source: str) -> str:
    wasm = _compile_tcl(source)
    _, stdout = _run_wasm(wasm, capture_stdout=True)
    return stdout


@pytest.mark.parametrize(
    "setup,expr,expected",
    [
        # Empty + strict — non-strict accepts every class for an
        # empty input, strict rejects.  Both go through the runtime
        # eval path because ``-strict`` is one of the recognised
        # flags.
        ("set v {}", "string is alpha $v", "1"),
        ("set v {}", "string is alpha -strict $v", "0"),
        # ``dict`` class — even-element-count contract.  Inputs are
        # routed through ``$v`` so the codegen's ``_split_command_subst``
        # can't mistake the brace-quoted value for a literal.
        ("set v {a b c d}", "string is dict $v", "1"),
        ("set v {a b c}", "string is dict $v", "0"),
        ("set v {}", "string is dict $v", "1"),
        ("set v a", "string is dict $v", "0"),
        # ``true`` / ``false`` — boolean-prefix matching (Tcl
        # ``Tcl_GetBoolean`` accepts ``tr`` / ``f`` / ``ye`` /
        # ``of`` etc).
        ("set v 1", "string is true $v", "1"),
        ("set v yes", "string is true $v", "1"),
        ("set v ye", "string is true $v", "1"),
        ("set v tr", "string is true $v", "1"),
        ("set v 0", "string is true $v", "0"),
        ("set v 0", "string is false $v", "1"),
        ("set v off", "string is false $v", "1"),
        ("set v N", "string is false $v", "1"),
        ("set v 1", "string is false $v", "0"),
        ("set v f", "string is boolean $v", "1"),
        # ``wordchar`` — alnum + underscore.
        ("set v abc_def", "string is wordchar $v", "1"),
        ("set v abc-def", "string is wordchar $v", "0"),
        # ``entier`` (alias for integer).
        ("set v 42", "string is entier $v", "1"),
        ("set v 4.5", "string is entier $v", "0"),
        # ``list`` accepts well-formed strings.
        ("set v {a b c}", "string is list $v", "1"),
    ],
)
def test_string_is_basic(setup: str, expr: str, expected: str) -> None:
    out = _run(f"{setup}; puts -nonewline [{expr}]")
    assert out == expected, f"{expr!r} → {out!r}, want {expected!r}"


def test_string_is_bad_class() -> None:
    out = _run("puts [catch {string is bogus foo} msg]; puts $msg")
    lines = out.strip().splitlines()
    assert lines[0] == "1"
    assert lines[1].startswith('bad class "bogus":')
    assert "alnum, alpha, ascii" in lines[1]


def test_string_is_ambiguous_class() -> None:
    out = _run("puts [catch {string is al foo} msg]; puts $msg")
    lines = out.strip().splitlines()
    assert lines[0] == "1"
    assert lines[1].startswith('ambiguous class "al":')


def test_string_is_failindex_dict_odd() -> None:
    out = _run("set x {}; puts [string is dict -failindex x {a b c}]; puts $x")
    lines = out.strip().splitlines()
    assert lines[0] == "0"
    assert lines[1] == "-1"
