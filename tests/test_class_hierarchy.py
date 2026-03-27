"""Tests for Class Hierarchy Analysis (CHA)."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.analysis.class_hierarchy import ClassHierarchy, build_class_hierarchy
from core.analysis.semantic_model import ClassDef, MethodDef, ParamDef, PropertyDef, Range
from core.parsing.tokens import SourcePosition


def _range(line: int = 0, col: int = 0) -> Range:
    """Helper to create a Range for test data."""
    start = SourcePosition(line=line, character=col, offset=0)
    end = SourcePosition(line=line, character=col + 5, offset=5)
    return Range(start=start, end=end)


def _make_class(
    name: str,
    qname: str | None = None,
    superclasses: list[str] | None = None,
    mixins: list[str] | None = None,
    methods: dict[str, MethodDef] | None = None,
    class_methods: dict[str, MethodDef] | None = None,
) -> ClassDef:
    """Helper to create a ClassDef for tests."""
    if qname is None:
        qname = f"::{name}"
    return ClassDef(
        name=name,
        qualified_name=qname,
        name_range=_range(),
        body_range=_range(),
        superclasses=superclasses or [],
        mixins=mixins or [],
        methods=methods or {},
        class_methods=class_methods or {},
    )


def _make_method(name: str, params: list[str] | None = None) -> MethodDef:
    """Helper to create a MethodDef for tests."""
    param_defs = [ParamDef(name=p, name_range=_range()) for p in (params or [])]
    return MethodDef(
        name=name,
        params=param_defs,
        name_range=_range(),
        body_range=_range(),
    )


class TestBuildClassHierarchy:
    def test_single_class(self):
        classes = {"::Dog": _make_class("Dog")}
        ch = build_class_hierarchy(classes)
        assert "::Dog" in ch.mro_map
        assert ch.mro_map["::Dog"] == ("::Dog",)

    def test_single_inheritance_mro(self):
        classes = {
            "::Animal": _make_class("Animal"),
            "::Dog": _make_class("Dog", superclasses=["::Animal"]),
        }
        ch = build_class_hierarchy(classes)
        assert ch.mro_map["::Dog"] == ("::Dog", "::Animal")

    def test_diamond_mro(self):
        classes = {
            "::A": _make_class("A"),
            "::B": _make_class("B", superclasses=["::A"]),
            "::C": _make_class("C", superclasses=["::A"]),
            "::D": _make_class("D", superclasses=["::B", "::C"]),
        }
        ch = build_class_hierarchy(classes)
        assert ch.mro_map["::D"] == ("::D", "::B", "::C", "::A")

    def test_direct_subclasses(self):
        classes = {
            "::Animal": _make_class("Animal"),
            "::Dog": _make_class("Dog", superclasses=["::Animal"]),
            "::Cat": _make_class("Cat", superclasses=["::Animal"]),
        }
        ch = build_class_hierarchy(classes)
        assert ch.subclasses["::Animal"] == frozenset({"::Dog", "::Cat"})
        assert ch.subclasses["::Dog"] == frozenset()

    def test_transitive_subtypes(self):
        classes = {
            "::A": _make_class("A"),
            "::B": _make_class("B", superclasses=["::A"]),
            "::C": _make_class("C", superclasses=["::B"]),
        }
        ch = build_class_hierarchy(classes)
        assert ch.transitive_subtypes["::A"] == frozenset({"::B", "::C"})
        assert ch.transitive_subtypes["::B"] == frozenset({"::C"})
        assert ch.transitive_subtypes["::C"] == frozenset()

    def test_is_subtype(self):
        classes = {
            "::A": _make_class("A"),
            "::B": _make_class("B", superclasses=["::A"]),
        }
        ch = build_class_hierarchy(classes)
        assert ch.is_subtype("::B", "::A")
        assert ch.is_subtype("::B", "::B")  # self is in MRO
        assert not ch.is_subtype("::A", "::B")

    def test_mixins_in_mro(self):
        classes = {
            "::Serializable": _make_class("Serializable"),
            "::Animal": _make_class("Animal"),
            "::Dog": _make_class("Dog", superclasses=["::Animal"], mixins=["::Serializable"]),
        }
        ch = build_class_hierarchy(classes)
        mro = ch.mro_map["::Dog"]
        assert mro[0] == "::Dog"
        # Mixins come before superclasses in TclOO
        assert mro.index("::Serializable") < mro.index("::Animal")


class TestMethodProviders:
    def test_own_method(self):
        classes = {
            "::Dog": _make_class("Dog", methods={"bark": _make_method("bark")}),
        }
        ch = build_class_hierarchy(classes)
        assert ch.method_target("::Dog", "bark") == "::Dog"

    def test_inherited_method(self):
        classes = {
            "::Animal": _make_class("Animal", methods={"speak": _make_method("speak")}),
            "::Dog": _make_class("Dog", superclasses=["::Animal"]),
        }
        ch = build_class_hierarchy(classes)
        assert ch.method_target("::Dog", "speak") == "::Animal"

    def test_overridden_method(self):
        classes = {
            "::Animal": _make_class("Animal", methods={"speak": _make_method("speak")}),
            "::Dog": _make_class("Dog", superclasses=["::Animal"], methods={"speak": _make_method("speak")}),
        }
        ch = build_class_hierarchy(classes)
        assert ch.method_target("::Dog", "speak") == "::Dog"
        assert ch.method_target("::Animal", "speak") == "::Animal"

    def test_mixin_method_takes_priority(self):
        """Mixin methods should be found before superclass methods."""
        classes = {
            "::Mixin": _make_class("Mixin", methods={"shared": _make_method("shared")}),
            "::Base": _make_class("Base", methods={"shared": _make_method("shared")}),
            "::Child": _make_class("Child", superclasses=["::Base"], mixins=["::Mixin"]),
        }
        ch = build_class_hierarchy(classes)
        # Mixin comes first in MRO, so it provides the method
        assert ch.method_target("::Child", "shared") == "::Mixin"

    def test_class_method_provider(self):
        classes = {
            "::Counter": _make_class("Counter", class_methods={"count": _make_method("count")}),
        }
        ch = build_class_hierarchy(classes)
        assert ch.method_target("::Counter", "count") == "::Counter"

    def test_all_implementations(self):
        classes = {
            "::Animal": _make_class("Animal", methods={"speak": _make_method("speak")}),
            "::Dog": _make_class("Dog", superclasses=["::Animal"], methods={"speak": _make_method("speak")}),
            "::Cat": _make_class("Cat", superclasses=["::Animal"]),
        }
        ch = build_class_hierarchy(classes)
        impls = ch.all_implementations("speak")
        # Dog resolves speak to Dog, Cat resolves speak to Animal, Animal resolves speak to Animal
        impl_dict = dict(impls)
        assert impl_dict["::Dog"] == "::Dog"
        assert impl_dict["::Cat"] == "::Animal"
        assert impl_dict["::Animal"] == "::Animal"

    def test_method_target_unknown_method(self):
        classes = {"::Dog": _make_class("Dog")}
        ch = build_class_hierarchy(classes)
        assert ch.method_target("::Dog", "nonexistent") is None

    def test_method_target_unknown_class(self):
        classes = {"::Dog": _make_class("Dog")}
        ch = build_class_hierarchy(classes)
        assert ch.method_target("::Unknown", "bark") is None


class TestParentNameNormalisation:
    def test_unqualified_parent_normalised(self):
        """Unqualified parent name should be normalised to qualified if found."""
        classes = {
            "::Animal": _make_class("Animal"),
            "::Dog": _make_class("Dog", superclasses=["Animal"]),  # unqualified
        }
        ch = build_class_hierarchy(classes)
        assert ch.is_subtype("::Dog", "::Animal")

    def test_already_qualified_parent(self):
        classes = {
            "::Animal": _make_class("Animal"),
            "::Dog": _make_class("Dog", superclasses=["::Animal"]),
        }
        ch = build_class_hierarchy(classes)
        assert ch.is_subtype("::Dog", "::Animal")


class TestHierarchyErrors:
    def test_cycle_produces_error(self):
        classes = {
            "::A": _make_class("A", superclasses=["::B"]),
            "::B": _make_class("B", superclasses=["::A"]),
        }
        ch = build_class_hierarchy(classes)
        assert len(ch.errors) > 0
        assert any("cycle" in e for e in ch.errors)

    def test_cycle_class_still_has_mro(self):
        """Classes with cycle errors should still get a fallback MRO."""
        classes = {
            "::A": _make_class("A", superclasses=["::B"]),
            "::B": _make_class("B", superclasses=["::A"]),
        }
        ch = build_class_hierarchy(classes)
        # Fallback: just the class itself
        assert "::A" in ch.mro_map
        assert "::B" in ch.mro_map

    def test_empty_hierarchy(self):
        ch = build_class_hierarchy({})
        assert ch.classes == {}
        assert ch.mro_map == {}
        assert ch.errors == ()
