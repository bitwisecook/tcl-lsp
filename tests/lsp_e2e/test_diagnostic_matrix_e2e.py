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

"""Data-driven diagnostic matrix: a must-fire / must-stay-silent pair per code.

The goal (tracked in ``docs/testing/lsp-e2e-hardening.md``) is solid positive
*and* negative coverage for every diagnostic / warning / info / optimisation /
shimmer / taint code, on the wire, on both backends.  This file is the
extensible home for that matrix: add a ``_Case`` row and you get a positive test
(the code fires for ``fire``) and a negative test (it stays silent for
``silent``) for free, each locked to the server's published diagnostics.

Every pair here is ground-truthed against the Python reference server.  The
``silent`` snippet is the *minimal* edit that removes the defect, so the pair
also proves the diagnostic is specific (it doesn't fire on the corrected form).

Codes that need dialect / sink context to fire — the taint (T1xx) and shimmer
(S1xx) families (iRules dialect + specific sources/sinks) and the optimiser
O-codes (surfaced through ``optimiseDocument``, not default diagnostics) — are
covered separately (see ``TestOptimisationMatrix`` and the iRules suite) and
tracked as follow-ups in the hardening doc.
"""

from __future__ import annotations

from dataclasses import dataclass

import pytest


@dataclass(frozen=True)
class _Case:
    code: str
    fire: str  # source that MUST raise ``code``
    silent: str  # the corrected source that MUST NOT raise ``code``
    note: str = ""


# One row per code.  Bodies are wrapped in a proc where a bare top-level ``set``
# would otherwise add incidental W211/W220 noise.
_MATRIX: list[_Case] = [
    _Case(
        "E002",
        "set\n",
        "set x 1\nputs $x\n",
        "wrong arg count (too few)",
    ),
    _Case(
        "E003",
        "string length a b c\n",
        "string length abc\n",
        "too many arguments to a subcommand",
    ),
    _Case(
        "W100",
        "proc p {a} {\n    if $a { puts x }\n}\n",
        "proc p {a} {\n    if {$a} { puts x }\n}\n",
        "unbraced expr",
    ),
    _Case(
        "W110",
        'proc p {x} {\n    if {$x == "foo"} { return 1 }\n    return 0\n}\n',
        'proc p {x} {\n    if {$x eq "foo"} { return 1 }\n    return 0\n}\n',
        "== string comparison (use eq)",
    ),
    _Case(
        "W211",
        "proc p {} {\n    set x 1\n    return 0\n}\n",
        "proc p {} {\n    set x 1\n    return $x\n}\n",
        "variable set but never read",
    ),
    _Case(
        "W220",
        "proc p {} {\n    set x 1\n    set x 2\n    return $x\n}\n",
        "proc p {} {\n    set x 1\n    puts $x\n    set x 2\n    return $x\n}\n",
        "dead store (overwritten before read)",
    ),
    _Case(
        "W210",
        "proc p {x} {\n    if {$x} {\n        set y 1\n    }\n    return $y\n}\n",
        "proc p {x} {\n    if {$x} {\n        set y 1\n    } else {\n        set y 2\n    }\n    return $y\n}\n",
        "read of a possibly-unset variable",
    ),
    _Case(
        "W302",
        "catch {error e}\n",
        "catch {error e} result\n",
        "catch without a result variable",
    ),
    _Case(
        "I230",
        "proc p {} {\n    if {1} { return 1 }\n    return 0\n}\n",
        "proc p {cond} {\n    if {$cond} { return 1 }\n    return 0\n}\n",
        "constant condition",
    ),
]


def _diags_for(lsp_server, uri_factory, source):
    return lsp_server.open_ready(uri_factory(), source)


def _codes(diags) -> set[str]:
    return {str(d.get("code")) for d in diags}


@pytest.mark.parametrize("case", _MATRIX, ids=lambda c: c.code)
def test_code_fires_on_defect(case: _Case, lsp_server, uri_factory):
    diags = _diags_for(lsp_server, uri_factory, case.fire)
    matching = [d for d in diags if str(d.get("code")) == case.code]
    assert matching, f"{case.code} ({case.note}) must fire on:\n{case.fire}\ngot {_codes(diags)}"
    # Severity is a valid LSP value, and an ``E``-prefixed code is an Error.
    sev = matching[0].get("severity")
    assert sev in (1, 2, 3, 4), f"{case.code}: invalid severity {sev}"
    if case.code.startswith("E"):
        assert sev == 1, f"{case.code} is an error code but severity={sev}"


@pytest.mark.parametrize("case", _MATRIX, ids=lambda c: c.code)
def test_code_silent_on_corrected_form(case: _Case, lsp_server, uri_factory):
    diags = _diags_for(lsp_server, uri_factory, case.silent)
    assert case.code not in _codes(diags), (
        f"{case.code} ({case.note}) must NOT fire on the corrected form:\n"
        f"{case.silent}\ngot {_codes(diags)}"
    )


class TestOptimisationMatrix:
    """Optimiser offers (O-codes) surface through ``optimiseDocument``.

    Positive: a foldable construct yields the folded source.  Negative: code
    with nothing to fold is returned unchanged.
    """

    def test_constant_llength_folds(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "puts [llength [list a b c]]\n")
        result = lsp_server.execute_command("tcl-lsp.optimiseDocument", [uri, "full"])
        assert "puts 3" in result["source"], result["source"]

    def test_constant_expr_folds(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc p {} { return [expr {1 + 2}] }\n")
        result = lsp_server.execute_command("tcl-lsp.optimiseDocument", [uri, "full"])
        assert "return 3" in result["source"], result["source"]

    def test_nothing_to_fold_is_unchanged(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = "proc p {a b} { return [expr {$a + $b}] }\n"
        lsp_server.open_ready(uri, src)
        result = lsp_server.execute_command("tcl-lsp.optimiseDocument", [uri, "full"])
        # A dynamic expression has no safe constant fold — source is preserved.
        assert "expr {$a + $b}" in result["source"], result["source"]
