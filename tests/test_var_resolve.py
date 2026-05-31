"""Tests for variable resolution: syntactic ref + scope context -> Place (8A)."""

from __future__ import annotations

from compiler.place import LOCAL_NS, IndexKind, PlaceKind, places_read_to_form
from compiler.var_resolve import ResolveContext, resolve_dict_path, resolve_place


def _reads(p) -> set[str]:
    return {x.name for x in places_read_to_form(p)}


class TestBasicForms:
    def test_local_scalar(self):
        p = resolve_place("x")
        assert p.kind is PlaceKind.SCALAR and p.ns == LOCAL_NS and p.name == "x"

    def test_literal_array_element(self):
        p = resolve_place("a(k)")
        assert p.kind is PlaceKind.ARRAY_ELEM and p.name == "a"
        assert p.index is not None and p.index.kind is IndexKind.LITERAL and p.index.value == "k"

    def test_dynamic_array_index_reads_index_var(self):
        p = resolve_place("a($i)")
        assert p.kind is PlaceKind.ARRAY_ELEM
        assert p.index is not None and p.index.kind is IndexKind.DYNAMIC
        assert _reads(p) == {"i"}

    def test_array_index_var_in_read_ref(self):
        # `$state($whom)` arrives de-sigilled as `state($whom)` from the scanner.
        p = resolve_place("state($whom)")
        assert p.kind is PlaceKind.ARRAY_ELEM and p.name == "state"
        assert _reads(p) == {"whom"}

    def test_qualified_scalar(self):
        p = resolve_place("::g::v")
        assert p.kind is PlaceKind.SCALAR and p.ns == "::g" and p.name == "v"

    def test_whole_array(self):
        p = resolve_place("a", whole_array=True)
        assert p.kind is PlaceKind.ARRAY_WHOLE and p.name == "a"


class TestDynamicNames:
    def test_dollar_name_is_unknown_reads_name_var(self):
        # `set $X v` (IR name `${X}`) writes a runtime-named var, reading X.
        p = resolve_place("${X}")
        assert p.kind is PlaceKind.UNKNOWN
        assert _reads(p) == {"X"}

    def test_dynamic_array_name_reads_name_var(self):
        # `set ${tok}(k) v` — value of tok forms the array name; reads tok.
        p = resolve_place("${tok}(k)")
        assert p.kind is PlaceKind.UNKNOWN
        assert _reads(p) == {"tok"}


class TestScopeBinding:
    def _ctx(self) -> ResolveContext:
        return ResolveContext(
            namespace="::ns",
            globals=frozenset({"cfg"}),
            ns_vars=frozenset({"state"}),
            upvar_aliases={"var": "caller", "dyn": ""},
            traced=frozenset({"t"}),
            instance_vars=frozenset({"iv"}),
            instance_owner="::C",
        )

    def test_global_declaration_binds_global_ns(self):
        p = resolve_place("cfg", self._ctx())
        assert p.kind is PlaceKind.SCALAR and p.ns == "::"

    def test_variable_declaration_binds_current_ns(self):
        p = resolve_place("state", self._ctx())
        assert p.ns == "::ns"

    def test_upvar_alias(self):
        p = resolve_place("var", self._ctx())
        assert p.kind is PlaceKind.UPVAR_ALIAS and p.owner == "caller" and not p.dynamic

    def test_dynamic_upvar_alias(self):
        p = resolve_place("dyn", self._ctx())
        assert p.kind is PlaceKind.UPVAR_ALIAS and p.dynamic

    def test_trace_marks_observed(self):
        assert resolve_place("t", self._ctx()).observed

    def test_instance_var(self):
        p = resolve_place("iv", self._ctx())
        assert p.kind is PlaceKind.INSTANCE_VAR and p.owner == "::C"

    def test_plain_local_unaffected(self):
        p = resolve_place("local", self._ctx())
        assert p.kind is PlaceKind.SCALAR and p.ns == LOCAL_NS


class TestDictPath:
    def test_literal_key_path(self):
        p = resolve_dict_path("$d", ["a", "b"])
        assert p.kind is PlaceKind.DICT_PATH and p.name == "d" and len(p.keys) == 2
        assert p.keys[0].kind is IndexKind.LITERAL and p.keys[0].value == "a"

    def test_dynamic_key_reads(self):
        p = resolve_dict_path("$d", ["$k"])
        assert _reads(p) == {"k"}
