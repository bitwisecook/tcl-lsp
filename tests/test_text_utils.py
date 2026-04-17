"""Tests for core.common.text — edit distance and suggestion utilities."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.common.text import edit_distance, suggest_similar


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
