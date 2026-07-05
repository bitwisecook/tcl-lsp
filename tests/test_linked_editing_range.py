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

"""Tests for the linkedEditingRange provider."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from server.features.linked_editing_range import get_linked_editing_ranges

TEST_URI = "file:///test.tcl"  # noqa: F841 (keep parity with other tests)


class TestLinkedEditingRange:
    def test_recursive_self_calls_linked_with_declaration(self):
        source = textwrap.dedent("""\
            proc factorial {n} {
                if {$n <= 1} { return 1 }
                return [expr {$n * [factorial [expr {$n - 1}]]}]
            }
        """)
        # Cursor on the `factorial` declaration at line 0, col 5.
        result = get_linked_editing_ranges(source, 0, 5)
        assert result is not None
        assert len(result.ranges) >= 2
        # Declaration at line 0 is included.
        assert any(r.start.line == 0 for r in result.ranges)
        # The self-call at line 2 is included.
        assert any(r.start.line == 2 for r in result.ranges)
        # Word pattern is sensible.
        assert "[A-Za-z" in (result.word_pattern or "")

    def test_non_recursive_proc_returns_none(self):
        source = "proc greet {} { return hi }\n"
        # Only a declaration, no self-calls.
        result = get_linked_editing_ranges(source, 0, 5)
        assert result is None

    def test_cursor_not_on_proc_returns_none(self):
        source = "proc f {} { return 1 }\n"
        result = get_linked_editing_ranges(source, 0, 0)
        assert result is None

    def test_cursor_on_self_call_site(self):
        source = textwrap.dedent("""\
            proc loop {} {
                loop
            }
        """)
        # Cursor on the `loop` self-call at line 1, col 5.
        result = get_linked_editing_ranges(source, 1, 5)
        assert result is not None
        assert len(result.ranges) >= 2
