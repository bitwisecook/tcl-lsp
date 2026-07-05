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

"""WASM ``regexp`` — ``-nocase`` option abbreviation.

C Tcl's ``TclCompileRegexpCmd`` prefix-matches a single option,
``-nocase`` (``strncmp(str, "-nocase", len)``), so ``-n`` / ``-no`` /
``-noc`` all mean ``-nocase`` — regexpComp-21.6/21.7/24.6/24.7.  Every
*other* option goes through the interpreted ``Tcl_RegexpObjCmd``, which
uses an exact (``TCL_EXACT``) lookup, so ``-al`` / ``-li`` / ``-l`` are
plain ``bad option`` errors, not abbreviations.

``runtime/zig/valtypes/tcl_regex.zig`` previously matched ``-nocase``
exactly, rejecting its abbreviations.  The matcher now prefix-matches
``-nocase`` only and exact-matches the rest.
"""

from __future__ import annotations

import pytest

pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run(source: str) -> str:
    _, stdout = _run_wasm(_compile_tcl(source), capture_stdout=True)
    return stdout.rstrip("\n")


@pytest.mark.parametrize(
    "source,expected",
    [
        # -n / -no / -noc abbreviate -nocase (regexpComp-21.6/.7/24.6/.7).
        ("puts [regexp -n foo dogfoOd]", "1"),
        ("puts [regexp -no -- FoO dogfood]", "1"),
        ("puts [regexp -noc FOO dogFOOd]", "1"),
        # Exact options still resolve.
        ("puts [regexp -nocase FOO dogfood]", "1"),
        ("puts [regexp -- -x -x]", "1"),
        ("puts [regexp -all foo foofoo]", "2"),
        ('puts [regexp -line {^b} "a\\nb"]', "1"),
    ],
)
def test_regexp_option_accepted(source: str, expected: str) -> None:
    assert _run(source) == expected


@pytest.mark.parametrize(
    "opt",
    [
        # Abbreviations of options OTHER than -nocase are NOT accepted —
        # C Tcl reports plain "bad option" (no prefix matching, no
        # "ambiguous option").
        "-al",
        "-li",
        "-l",
        "-i",
        "-a",
        "-ind",
        "-gorp",
    ],
)
def test_regexp_non_nocase_abbrev_rejected(opt: str) -> None:
    out = _run(f"puts [catch {{regexp {opt} foo foo}} m]:$m")
    assert out.startswith(f'1:bad option "{opt}"'), out
