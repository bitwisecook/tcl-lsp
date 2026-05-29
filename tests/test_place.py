"""Tests for the unified variable Place model (Phase 8A).

The overlap relation is the soundness keystone: it must over-approximate real
aliasing (any uncertainty ⇒ overlap) so consumers built on it only suppress,
never falsely flag.  These pin the exact table and the over-approximation
property.
"""

from __future__ import annotations

import itertools

from compiler.place import (
    ANY_INDEX,
    LOCAL_NS,
    Place,
    PlaceKind,
    array_elem,
    array_whole,
    dict_path,
    dynamic_index,
    instance_var,
    literal_index,
    overlap,
    places_read_to_form,
    scalar,
    unknown,
    upvar_alias,
)


class TestScalarOverlap:
    def test_same_local_scalar_overlaps(self):
        assert overlap(scalar("x"), scalar("x"))

    def test_distinct_scalars_disjoint(self):
        assert not overlap(scalar("x"), scalar("y"))

    def test_local_vs_global_same_name_disjoint(self):
        # bare local x and ::x are different variables.
        assert not overlap(scalar("x"), scalar("x", ns="::"))

    def test_same_global_overlaps(self):
        assert overlap(scalar("g", ns="::"), scalar("g", ns="::"))


class TestArrayOverlap:
    def test_distinct_literal_elements_disjoint(self):
        # The headline precision win: a(foo) does not clobber a(bar).
        assert not overlap(
            array_elem("a", literal_index("foo")), array_elem("a", literal_index("bar"))
        )

    def test_same_literal_element_overlaps(self):
        assert overlap(array_elem("a", literal_index("k")), array_elem("a", literal_index("k")))

    def test_whole_array_overlaps_any_element(self):
        assert overlap(array_whole("a"), array_elem("a", literal_index("k")))
        assert overlap(array_elem("a", literal_index("k")), array_whole("a"))

    def test_dynamic_index_overlaps_anything_same_array(self):
        assert overlap(array_elem("a", dynamic_index()), array_elem("a", literal_index("k")))

    def test_different_arrays_disjoint(self):
        assert not overlap(array_elem("a", literal_index("k")), array_elem("b", literal_index("k")))

    def test_scalar_and_array_same_name_disjoint(self):
        assert not overlap(scalar("a"), array_elem("a", literal_index("k")))


class TestDictOverlap:
    def test_key_prefix_overlaps(self):
        # write d a b  ↔  read d a   (the subdict contains the deeper write)
        assert overlap(
            dict_path("d", (literal_index("a"), literal_index("b"))),
            dict_path("d", (literal_index("a"),)),
        )

    def test_disjoint_keys_dont_overlap(self):
        assert not overlap(
            dict_path("d", (literal_index("a"),)),
            dict_path("d", (literal_index("x"),)),
        )

    def test_dynamic_key_overlaps(self):
        assert overlap(
            dict_path("d", (dynamic_index(),)),
            dict_path("d", (literal_index("a"),)),
        )


class TestInstanceAndAlias:
    def test_same_instance_var_overlaps(self):
        assert overlap(instance_var("v", owner="::C"), instance_var("v", owner="::C"))

    def test_instance_var_different_owner_disjoint(self):
        assert not overlap(instance_var("v", owner="::C"), instance_var("v", owner="::D"))

    def test_upvar_alias_overlaps_conservatively(self):
        # Dynamic / unresolved alias edges overlap anything (sound).
        assert overlap(upvar_alias("var"), scalar("x"))

    def test_same_resolved_alias_edge_overlaps(self):
        assert overlap(upvar_alias("var", owner="::g"), upvar_alias("var", owner="::g"))

    def test_distinct_aliases_same_owner_overlap(self):
        # `upvar 1 caller_x a; upvar 1 caller_x b` — a and b alias the *same*
        # caller storage, so they must overlap despite distinct local names
        # (must-alias; the model must not under-approximate this).
        assert overlap(upvar_alias("a", owner="::caller::x"), upvar_alias("b", owner="::caller::x"))

    def test_distinct_aliases_different_owner_disjoint(self):
        # Aliases of *different* caller vars are distinct storage.
        assert not overlap(
            upvar_alias("a", owner="::caller::x"), upvar_alias("b", owner="::caller::y")
        )


class TestDynamicAliasVsIndex:
    """A *dynamic* place (``upvar 1 $x date`` — alias target unknown) keeps a
    *known local base name*, so it is NOT a wildcard over all names (Phase 8E).

    This is the precision the dead-store needs: a caller-frame alias ``date``
    must not be treated as observing a differently-named this-frame local
    ``date2`` (the gregorian.tcl over-suppression).  Distinguished from a
    dynamic *index* (``a($i)`` — same base) and a dynamic *name* (``set $X`` →
    ``UNKNOWN`` — a true wildcard)."""

    def _dyn_elem(self, name: str, idx: str) -> Place:
        # An array element whose base bound through an unresolved upvar alias.
        return Place(PlaceKind.ARRAY_ELEM, name=name, index=literal_index(idx), dynamic=True)

    def test_dynamic_alias_disjoint_from_differently_named_local(self):
        # date(k) [dynamic alias] vs date2(ERA) [concrete local] — different
        # frames, different names → disjoint (the gregorian fix).
        assert not overlap(self._dyn_elem("date", "k"), array_elem("date2", literal_index("ERA")))

    def test_two_dynamic_aliases_overlap_conservatively(self):
        # Two upvar aliases may point at the same caller storage → conservative.
        assert overlap(self._dyn_elem("date", "k"), self._dyn_elem("other", "k"))

    def test_dynamic_alias_same_name_concrete_overlaps(self):
        # Same base name (upvar 0 / same-frame alias is possible) → conservative.
        assert overlap(self._dyn_elem("a", "k"), array_elem("a", literal_index("k")))

    def test_dynamic_alias_distinct_elements_disjoint(self):
        # Distinct literal elements of a dynamic-aliased array are still disjoint.
        assert not overlap(self._dyn_elem("a", "k"), self._dyn_elem("a", "j"))

    def test_dynamic_index_is_not_place_dynamic(self):
        # `a($i)` is a dynamic *index*, not a dynamic alias — overlaps any
        # element of the *same* array, but not a different array.
        assert overlap(array_elem("a", dynamic_index()), array_elem("a", literal_index("k")))
        assert not overlap(array_elem("a", dynamic_index()), array_elem("b", literal_index("k")))

    def test_unknown_name_still_overlaps_everything(self):
        # `set $X` → UNKNOWN is a true wildcard (unchanged).
        assert overlap(unknown(), array_elem("date2", literal_index("ERA")))


class TestUnknownTop:
    def test_unknown_overlaps_everything(self):
        samples = [
            scalar("x"),
            array_elem("a", literal_index("k")),
            array_whole("a"),
            dict_path("d", (literal_index("a"),)),
            instance_var("v", owner="::C"),
            upvar_alias("var", owner="::g"),
            unknown(),
        ]
        for s in samples:
            assert overlap(unknown(), s)
            assert overlap(s, unknown())


class TestOverApproximationProperty:
    """No pair may be reported disjoint when either side is uncertain."""

    def _universe(self) -> list[Place]:
        return [
            scalar("x"),
            scalar("x", ns="::"),
            scalar("y"),
            array_whole("x"),
            array_elem("x", literal_index("k")),
            array_elem("x", literal_index("j")),
            array_elem("x", dynamic_index()),
            array_elem("x", ANY_INDEX),
            dict_path("x", (literal_index("k"),)),
            instance_var("x", owner="::C"),
            upvar_alias("x", owner="::g"),
            upvar_alias("x"),  # dynamic
            unknown(),
        ]

    def test_symmetric(self):
        u = self._universe()
        for p, q in itertools.product(u, u):
            assert overlap(p, q) == overlap(q, p), (p, q)

    def test_reflexive(self):
        for p in self._universe():
            assert overlap(p, p)

    def test_top_overlaps_everything(self):
        # UNKNOWN and any dynamic (unresolved alias/name) place are ⊤ — they
        # overlap every place (an opaque write/read may touch anything).
        u = self._universe()
        for p in u:
            if p.kind is PlaceKind.UNKNOWN or p.dynamic:
                for q in u:
                    assert overlap(p, q), (p, q)

    def test_any_or_dynamic_index_overlaps_same_array(self):
        # An ANY/DYNAMIC index is uncertain *within its array*: it overlaps any
        # element or whole of the same (ns,name), but not other variables — a
        # scalar/array share a name yet can't coexist, so stay disjoint.
        any_elem = array_elem("x", ANY_INDEX)
        assert overlap(any_elem, array_elem("x", literal_index("k")))
        assert overlap(any_elem, array_whole("x"))
        assert not overlap(any_elem, array_elem("y", literal_index("k")))
        assert not overlap(any_elem, scalar("x"))  # array x ≠ scalar x


class TestPlacesReadToForm:
    def test_dynamic_index_reads(self):
        i = scalar("i")
        p = array_elem("a", dynamic_index((i,)))
        assert places_read_to_form(p) == frozenset({i})

    def test_dynamic_name_reads(self):
        x = scalar("X")
        p = unknown(name_reads=(x,))
        assert places_read_to_form(p) == frozenset({x})

    def test_dict_key_reads(self):
        k = scalar("k")
        p = dict_path("d", (dynamic_index((k,)), literal_index("lit")))
        assert places_read_to_form(p) == frozenset({k})

    def test_literal_has_no_reads(self):
        assert places_read_to_form(array_elem("a", literal_index("k"))) == frozenset()


def test_local_ns_sentinel_is_local():
    assert scalar("x").ns == LOCAL_NS
    assert not scalar("x").is_global
    assert scalar("g", ns="::").is_global
