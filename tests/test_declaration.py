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

"""White-box declaration-collection internals.

``get_declaration``'s request/response behaviour is exercised end-to-end against
the packaged server in ``tests/lsp_e2e/test_navigation_e2e.py``.  What remains
here is ``_collect_declaration_ranges``, an internal helper with no JSON-RPC
surface — it underpins the declaration provider but is unit-tested directly for
its handling of ``global`` / ``variable`` / ``upvar`` declaration forms.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from server.features.declaration import _collect_declaration_ranges


class TestCollectDeclarationRanges:
    def test_global_declaration(self):
        ranges = _collect_declaration_ranges("global foo\nset foo 1\n", "foo")
        assert len(ranges) == 1
        assert ranges[0].start.line == 0

    def test_variable_declaration(self):
        ranges = _collect_declaration_ranges("variable bar\nset bar 2\n", "bar")
        assert len(ranges) == 1
        assert ranges[0].start.line == 0

    def test_upvar_integer_level(self):
        ranges = _collect_declaration_ranges("upvar 1 caller_var local_var\n", "local_var")
        assert len(ranges) == 1
        assert ranges[0].start.line == 0

    def test_upvar_hash_level_arbitrary(self):
        """`upvar #3 other local` should recognise the level and declare `local`."""
        ranges = _collect_declaration_ranges("upvar #3 other local\n", "local")
        assert len(ranges) == 1
        assert ranges[0].start.line == 0

    def test_upvar_no_level_defaults_to_pair(self):
        ranges = _collect_declaration_ranges("upvar other my_var\n", "my_var")
        assert len(ranges) == 1
        assert ranges[0].start.line == 0

    def test_variable_with_value_pairs(self):
        """`variable a 1 b 2` declares `a` and `b`, not `1`/`2`."""
        source = "namespace eval foo { variable a 1 b 2 }\n"
        assert len(_collect_declaration_ranges(source, "a")) == 1
        assert len(_collect_declaration_ranges(source, "b")) == 1
        assert len(_collect_declaration_ranges(source, "1")) == 0

    def test_no_declaration(self):
        assert _collect_declaration_ranges("set x 42\n", "x") == []
