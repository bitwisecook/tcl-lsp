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

"""Tests for shared.text — edit distance and suggestion utilities."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from shared.text import edit_distance, suggest_similar


class TestEditDistance:
    def test_identical(self):
        assert edit_distance("puts", "puts") == 0

    def test_single_substitution(self):
        assert edit_distance("puts", "putz") == 1

    def test_transposition(self):
        # Levenshtein counts transpositions as 2 operations
        assert edit_distance("puts", "pust") == 2

    def test_single_insertion(self):
        assert edit_distance("set", "sett") == 1

    def test_single_deletion(self):
        assert edit_distance("string", "strig") == 1

    def test_empty(self):
        assert edit_distance("", "") == 0
        assert edit_distance("abc", "") == 3
        assert edit_distance("", "abc") == 3

    def test_completely_different(self):
        assert edit_distance("abc", "xyz") == 3


class TestSuggestSimilar:
    def test_exact_match_first(self):
        result = suggest_similar("puts", ["puts", "set", "string"])
        assert result[0] == "puts"

    def test_close_match(self):
        result = suggest_similar("pust", ["puts", "set", "string"])
        assert "puts" in result

    def test_no_match_beyond_max_distance(self):
        result = suggest_similar("xyzzy", ["puts", "set", "string"], max_distance=2)
        assert result == []

    def test_max_suggestions(self):
        candidates = ["aa", "ab", "ac", "ad"]
        result = suggest_similar("aa", candidates, max_suggestions=2, max_distance=3)
        assert len(result) <= 2

    def test_empty_candidates(self):
        assert suggest_similar("foo", []) == []
