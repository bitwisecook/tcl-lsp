"""Tests for TclOO method resolution order algorithm.

Verifies the DFS + late-placement dedup algorithm matches Tcl 9.0's
actual dispatch order as documented in the oo.test suite.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import pytest

from core.analysis.mro import MROError, build_mro_map, tcloo_linearise


class TestTclOOMRO:
    def test_single_class_no_parents(self):
        result = tcloo_linearise("A", {})
        assert result == ["A"]

    def test_single_inheritance(self):
        supers = {"A": [], "B": ["A"]}
        result = tcloo_linearise("B", supers)
        assert result == ["B", "A"]

    def test_chain_of_three(self):
        supers = {"A": [], "B": ["A"], "C": ["B"]}
        result = tcloo_linearise("C", supers)
        assert result == ["C", "B", "A"]

    def test_diamond_inheritance(self):
        """oo-9.1: D(B,C), B(A), C(A) -> D B C A"""
        supers = {
            "A": [],
            "B": ["A"],
            "C": ["A"],
            "D": ["B", "C"],
        }
        result = tcloo_linearise("D", supers)
        assert result == ["D", "B", "C", "A"]

    def test_multiple_parents_no_diamond(self):
        supers = {"A": [], "B": [], "C": ["A", "B"]}
        result = tcloo_linearise("C", supers)
        assert result == ["C", "A", "B"]

    def test_cycle_raises_c3error(self):
        supers = {"A": ["B"], "B": ["A"]}
        with pytest.raises(MROError, match="cycle"):
            tcloo_linearise("A", supers)

    def test_self_cycle_raises_c3error(self):
        supers = {"A": ["A"]}
        with pytest.raises(MROError, match="cycle"):
            tcloo_linearise("A", supers)

    def test_unknown_parent_treated_as_leaf(self):
        supers = {"B": ["UnknownBase"]}
        result = tcloo_linearise("B", supers)
        assert result == ["B", "UnknownBase"]

    def test_tcloo_style_with_oo_object(self):
        supers = {
            "::oo::object": [],
            "::Animal": ["::oo::object"],
            "::Dog": ["::Animal"],
        }
        result = tcloo_linearise("::Dog", supers)
        assert result == ["::Dog", "::Animal", "::oo::object"]

    def test_deep_hierarchy(self):
        supers = {str(i): [str(i - 1)] for i in range(1, 10)}
        supers["0"] = []
        result = tcloo_linearise("9", supers)
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
        result = tcloo_linearise("cls", supers, mixins_map=mixins)
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
        result = tcloo_linearise("::Dog", supers, mixins_map=mixins)
        # Mixin comes before the class itself in TclOO dispatch order
        assert result.index("::Serializable") < result.index("::Dog")
        assert result.index("::Dog") < result.index("::Animal")

    def test_no_mixins_same_as_superclass_only(self):
        """Without mixins, result is pure DFS on superclasses."""
        supers = {"A": [], "B": ["A"]}
        result = tcloo_linearise("B", supers, mixins_map={})
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
        result = tcloo_linearise("C", supers, mixins_map=mixins)
        # B is a mixin of C, A is a mixin of B
        # All mixin-path classes (A, B, parent-via-mixin) come before non-mixin (C)
        assert "A" in result
        assert "B" in result
        assert result.index("A") < result.index("B")
        assert result.index("B") < result.index("C")

    def test_two_pass_nested_mixin_ordering(self):
        """Two-pass mechanism: nested mixin at depth must precede non-mixin class.

        D has mixin M1, superclass C. C has mixin M2, superclass A.
        Without two passes: M1, D, M2, C, A (M2 between D and C).
        With two passes:    M1, M2, D, C, A (all mixins before non-mixins).
        """
        supers = {
            "A": [],
            "C": ["A"],
            "D": ["C"],
            "M1": [],
            "M2": [],
        }
        mixins = {
            "D": ["M1"],
            "C": ["M2"],
        }
        result = tcloo_linearise("D", supers, mixins_map=mixins)
        # Both mixins must come before both non-mixin classes
        assert result.index("M1") < result.index("D")
        assert result.index("M2") < result.index("D")
        assert result.index("M2") < result.index("C")
        # Non-mixin ordering preserved
        assert result.index("D") < result.index("C")
        assert result.index("C") < result.index("A")

    def test_mixin_shared_superclass_dedup(self):
        """Late-placement dedup across passes for shared base.

        cls has mixin mix, both extend base. base appears via mixin path
        (pass 1) and non-mixin path (pass 2) — dedup moves it to the end.
        """
        supers = {
            "base": [],
            "mix": ["base"],
            "cls": ["base"],
        }
        mixins = {
            "cls": ["mix"],
        }
        result = tcloo_linearise("cls", supers, mixins_map=mixins)
        # mix (mixin-path) before cls (non-mixin)
        assert result.index("mix") < result.index("cls")
        # base deduped to end (appears via both paths)
        assert result.index("cls") < result.index("base")


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
