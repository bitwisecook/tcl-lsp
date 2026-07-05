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

"""WASM ``concat`` trailing-whitespace trimming contract.

``Tcl_ConcatObj`` (tclUtil.c) trims leading and trailing whitespace
from every element before joining with single spaces.  The trailing
trim removes *all* whitespace with no backslash awareness, after which::

    elemLength += trimr && (element[elemLength - 1] == '\\');

re-exposes exactly one of the trimmed bytes when the trim landed on a
backslash — so a trailing escaped space survives the concat.  The rule
keys off the bare final ``\\``, not the backslash run's parity, so both
``b\\ `` (one backslash) and ``b\\\\   `` (two backslashes) keep the
same single trailing space.

The runtime previously stopped trimming at the first odd-backslash-
escaped space, which is correct for odd runs but trimmed the exposed
space away for even runs (util-4.3: ``concat a {b\\\\   } c`` produced
``a b\\\\ c`` instead of ``a b\\\\  c``).  These cases pin the C rule.
"""

from __future__ import annotations

import pytest

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

from tests.test_wasm_real_tcl import _compile_tcl, _run_wasm  # noqa: E402


def _run(source: str) -> str:
    wasm = _compile_tcl(source)
    _, stdout = _run_wasm(wasm, capture_stdout=True)
    return stdout.rstrip("\n")


@pytest.mark.parametrize(
    "source,expected",
    [
        # One backslash, one trailing space — escaped space kept (util-4.1).
        (r"puts [concat a {b\ } c]", r"a b\  c"),
        # One backslash, run of trailing spaces — collapse to one (util-4.2).
        (r"puts [concat a {b\   } c]", r"a b\  c"),
        # Two backslashes (even run) — the exposed final ``\`` still keeps
        # one trailing space.  This is the case the old parity check broke
        # (util-4.3).
        (r"puts [concat a {b\\   } c]", r"a b\\  c"),
        # Three backslashes (odd run) — same single trailing space.
        (r"puts [concat a {b\\\   } c]", r"a b\\\  c"),
        # No backslash — trailing whitespace fully trimmed (util-4.4).
        (r"puts [concat a {b } c]", "a b c"),
        # An all-whitespace element drops out entirely (util-4.5).
        (r"puts [concat a { } c]", "a c"),
        # Pairwise accumulation keeps the escaped space across joins.
        (r"puts [concat {x\ } {y}]", r"x\  y"),
    ],
)
def test_concat_trailing_backslash_space(source: str, expected: str) -> None:
    assert _run(source) == expected
