"""Tests for C3 linearisation (MRO) algorithm."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import pytest

from core.analysis.mro import C3Error, build_mro_map, c3_linearise


class TestC3Linearise:
    def test_single_class_no_parents(self):
        result = c3_linearise("A", {})
        assert result == ["A"]

    def test_single_inheritance(self):
        supers = {"A": [], "B": ["A"]}
        result = c3_linearise("B", supers)
        assert result == ["B", "A"]

    def test_chain_of_three(self):
        supers = {"A": [], "B": ["A"], "C": ["B"]}
        result = c3_linearise("C", supers)
        assert result == ["C", "B", "A"]

    def test_diamond_inheritance(self):
        supers = {
            "A": [],
            "B": ["A"],
            "C": ["A"],
            "D": ["B", "C"],
        }
        result = c3_linearise("D", supers)
        # D -> B -> C -> A (C3 linearisation of classic diamond)
        assert result == ["D", "B", "C", "A"]

    def test_multiple_parents_no_diamond(self):
        supers = {"A": [], "B": [], "C": ["A", "B"]}
        result = c3_linearise("C", supers)
        assert result == ["C", "A", "B"]

    def test_cycle_raises_c3error(self):
        supers = {"A": ["B"], "B": ["A"]}
        with pytest.raises(C3Error, match="cycle"):
            c3_linearise("A", supers)

    def test_self_cycle_raises_c3error(self):
        supers = {"A": ["A"]}
        with pytest.raises(C3Error, match="cycle"):
            c3_linearise("A", supers)

    def test_inconsistent_hierarchy(self):
        # Inconsistent: B(A, C) and D(C, A) where order conflicts
        supers = {
            "A": [],
            "C": [],
            "B": ["A", "C"],
            "D": ["C", "A"],
            "E": ["B", "D"],
        }
        with pytest.raises(C3Error, match="inconsistent"):
            c3_linearise("E", supers)

    def test_unknown_parent_treated_as_leaf(self):
        supers = {"B": ["UnknownBase"]}
        result = c3_linearise("B", supers)
        assert result == ["B", "UnknownBase"]

    def test_tcloo_style_with_oo_object(self):
        supers = {
            "::oo::object": [],
            "::Animal": ["::oo::object"],
            "::Dog": ["::Animal"],
        }
        result = c3_linearise("::Dog", supers)
        assert result == ["::Dog", "::Animal", "::oo::object"]

    def test_mixins_before_superclasses(self):
        """TclOO puts mixins before superclasses in the MRO."""
        supers = {
            "::oo::object": [],
            "::Serializable": ["::oo::object"],
            "::Animal": ["::oo::object"],
            "::Dog": ["::Serializable", "::Animal"],  # mixin first
        }
        result = c3_linearise("::Dog", supers)
        assert result[0] == "::Dog"
        assert result.index("::Serializable") < result.index("::Animal")

    def test_deep_hierarchy(self):
        supers = {str(i): [str(i - 1)] for i in range(1, 10)}
        supers["0"] = []
        result = c3_linearise("9", supers)
        assert result == [str(i) for i in range(9, -1, -1)]

    def test_memoisation_shares_results(self):
        """Computing MRO for a child should also cache parent results."""
        supers = {"A": [], "B": ["A"], "C": ["B"]}
        # Just verify no errors and correct result
        result = c3_linearise("C", supers)
        assert result == ["C", "B", "A"]


class TestBuildMroMap:
    def test_all_classes_get_mro(self):
        supers = {"A": [], "B": ["A"], "C": ["A"]}
        mro_map, errors = build_mro_map(supers)
        assert not errors
        assert "A" in mro_map
        assert "B" in mro_map
        assert "C" in mro_map

    def test_errors_collected_not_raised(self):
        supers = {"A": ["B"], "B": ["A"]}
        mro_map, errors = build_mro_map(supers)
        assert len(errors) > 0
        assert "cycle" in errors[0]

    def test_empty_input(self):
        mro_map, errors = build_mro_map({})
        assert mro_map == {}
        assert errors == []
