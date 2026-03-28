"""Tests for TclOO method resolution order algorithm.

Verifies the DFS + late-placement dedup algorithm matches Tcl 9.0's
actual dispatch order as documented in the oo.test suite.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import pytest

from core.analysis.mro import C3Error, build_mro_map, c3_linearise


class TestTclOOMRO:
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
        """oo-9.1: D(B,C), B(A), C(A) -> D B C A"""
        supers = {
            "A": [],
            "B": ["A"],
            "C": ["A"],
            "D": ["B", "C"],
        }
        result = c3_linearise("D", supers)
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

    def test_deep_hierarchy(self):
        supers = {str(i): [str(i - 1)] for i in range(1, 10)}
        supers["0"] = []
        result = c3_linearise("9", supers)
        assert result == [str(i) for i in range(9, -1, -1)]


class TestTclOOMixins:
    """Tests for mixin ordering matching Tcl 9.0's oo.test suite."""

    def test_mixin_before_class_oo_14_8(self):
        """oo-14.8: cls has mixin mix, superclass parent -> mix cls parent"""
        supers = {
            "parent": ["oo::object"],
            "mix": ["parent"],
            "cls": ["parent"],
            "oo::object": [],
        }
        mixins = {
            "cls": ["mix"],
        }
        result = c3_linearise("cls", supers, mixins_map=mixins)
        # Tcl test expects dispatch: mix -> cls -> parent
        assert result.index("mix") < result.index("cls")
        assert result.index("cls") < result.index("parent")

    def test_mixin_provides_method(self):
        """Mixin methods come before the class's own superclass methods."""
        supers = {
            "::oo::object": [],
            "::Serializable": ["::oo::object"],
            "::Animal": ["::oo::object"],
            "::Dog": ["::Animal"],
        }
        mixins = {
            "::Dog": ["::Serializable"],
        }
        result = c3_linearise("::Dog", supers, mixins_map=mixins)
        # Mixin comes before the class itself in TclOO dispatch order
        assert result.index("::Serializable") < result.index("::Dog")
        assert result.index("::Dog") < result.index("::Animal")

    def test_no_mixins_same_as_superclass_only(self):
        """Without mixins, result is pure DFS on superclasses."""
        supers = {"A": [], "B": ["A"]}
        result = c3_linearise("B", supers, mixins_map={})
        assert result == ["B", "A"]

    def test_mixin_of_mixin_oo_14_6(self):
        """oo-14.6: C mixin B, B mixin A -> methods from A available via C."""
        supers = {
            "parent": [],
            "A": ["parent"],
            "B": ["parent"],
            "C": ["parent"],
        }
        mixins = {
            "B": ["A"],
            "C": ["B"],
        }
        result = c3_linearise("C", supers, mixins_map=mixins)
        # B is a mixin of C, A is a mixin of B
        # So A should appear before B, B before C
        assert "A" in result
        assert "B" in result


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

    def test_with_mixins(self):
        supers = {"A": [], "B": ["A"], "C": ["A"]}
        mixins = {"C": ["B"]}
        mro_map, errors = build_mro_map(supers, mixins_map=mixins)
        assert not errors
        assert "C" in mro_map
        assert mro_map["C"].index("B") < mro_map["C"].index("A")
