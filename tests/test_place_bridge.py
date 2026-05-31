"""Tests for the IR→Place bridge (Phase 8A): context build + def/read places."""

from __future__ import annotations

from compiler.cfg import build_cfg
from compiler.lowering import lower_to_ir
from compiler.place import PlaceKind, overlap
from compiler.place_bridge import build_resolve_context, def_places, read_places


def _fn(src: str, proc: str = "::f"):
    cfg = build_cfg(lower_to_ir(src)).procedures[proc]
    return cfg, build_resolve_context(cfg)


def _stmts(cfg):
    for block in cfg.blocks.values():
        yield from block.statements


class TestResolveContext:
    def test_global_and_variable_and_trace(self):
        cfg, ctx = _fn("proc f {} { global g\n variable v\n trace add variable t write cb\n }")
        assert "g" in ctx.globals
        assert "v" in ctx.ns_vars
        assert "t" in ctx.traced

    def test_upvar_alias_target(self):
        cfg, ctx = _fn("proc f {} { upvar 1 caller local\n }")
        assert ctx.upvar_aliases.get("local") == "caller"

    def test_dynamic_upvar_target_is_blank(self):
        cfg, ctx = _fn("proc f {n} { upvar 1 $n local\n }")
        assert ctx.upvar_aliases.get("local") == ""

    def test_embedded_subst_upvar_target_is_blank(self):
        # A target with an *embedded* substitution (not just a leading $) names a
        # runtime-computed caller var, so it is unresolvable → recorded dynamic.
        cfg, ctx = _fn("proc f {x} { upvar 1 prefix_$x local\n }")
        assert ctx.upvar_aliases.get("local") == ""


class TestDefPlaces:
    def test_scalar_def(self):
        cfg, ctx = _fn("proc f {} { set x 1 }")
        places = [p for s in _stmts(cfg) for p in def_places(s, ctx)]
        assert any(p.kind is PlaceKind.SCALAR and p.name == "x" for p in places)

    def test_array_element_def(self):
        cfg, ctx = _fn("proc f {} { set a(k) 1 }")
        places = [p for s in _stmts(cfg) for p in def_places(s, ctx)]
        assert any(p.kind is PlaceKind.ARRAY_ELEM and p.name == "a" for p in places)

    def test_dynamic_name_def_is_unknown(self):
        cfg, ctx = _fn("proc f {} { set X y\n set $X 1 }")
        kinds = {p.kind for s in _stmts(cfg) for p in def_places(s, ctx)}
        assert PlaceKind.UNKNOWN in kinds


class TestReadPlaces:
    def test_dynamic_name_read_recovered(self):
        # `set $X 1` reads X — the cascade fix, now structural via the Place.
        cfg, ctx = _fn("proc f {} { set X y\n set $X 1 }")
        reads = [p for s in _stmts(cfg) for p in read_places(s, ctx)]
        assert any(p.kind is PlaceKind.SCALAR and p.name == "X" for p in reads)

    def test_array_index_var_read_recovered(self):
        cfg, ctx = _fn("proc f {} { set i 0\n set v $arr($i) }")
        reads = [p for s in _stmts(cfg) for p in read_places(s, ctx)]
        assert any(p.kind is PlaceKind.SCALAR and p.name == "i" for p in reads)

    def test_element_def_overlaps_element_read(self):
        # set a(k) write and $a(k) read overlap (not dead); a(j) would not.
        cfg, ctx = _fn("proc f {} { set a(k) 1\n puts $a(k) }")
        defs = [p for s in _stmts(cfg) for p in def_places(s, ctx)]
        reads = [p for s in _stmts(cfg) for p in read_places(s, ctx)]
        elem_def = next(p for p in defs if p.kind is PlaceKind.ARRAY_ELEM)
        assert any(overlap(elem_def, r) for r in reads)

    def test_distinct_elements_do_not_overlap(self):
        cfg, ctx = _fn("proc f {} { set a(k) 1\n puts $a(j) }")
        defs = [p for s in _stmts(cfg) for p in def_places(s, ctx)]
        reads = [p for s in _stmts(cfg) for p in read_places(s, ctx)]
        elem_def = next(
            p
            for p in defs
            if p.kind is PlaceKind.ARRAY_ELEM and p.index is not None and p.index.value == "k"
        )
        assert not any(overlap(elem_def, r) for r in reads)

    def test_whole_array_read_in_cmdsub_overlaps_element_def(self):
        # `[array get arr]` reads the whole array — observes the arr(x) write.
        cfg, ctx = _fn("proc f {} { set arr(x) 9\n set y [array get arr] }")
        defs = [p for s in _stmts(cfg) for p in def_places(s, ctx)]
        reads = [p for s in _stmts(cfg) for p in read_places(s, ctx)]
        elem = next(p for p in defs if p.kind is PlaceKind.ARRAY_ELEM)
        assert any(overlap(elem, r) for r in reads)

    def test_var_read_role_name_recovered(self):
        # `info exists x` reads x as a name arg (no $), even nested in a cmdsub.
        cfg, ctx = _fn("proc f {} { set x 1\n set y [info exists x] }")
        reads = [p for s in _stmts(cfg) for p in read_places(s, ctx)]
        assert any(p.name == "x" for p in reads)

    def test_dynamic_var_read_name_is_unknown_overlap(self):
        # `array get $arr_name` reads an array whose name is computed — the read
        # must over-approximate (UNKNOWN overlaps everything), so an element
        # write to *any* array is observed and not falsely flagged dead.
        cfg, ctx = _fn("proc f {} { set mydata(k) 1\n set y [array get $arr_name] }")
        defs = [p for s in _stmts(cfg) for p in def_places(s, ctx)]
        reads = [p for s in _stmts(cfg) for p in read_places(s, ctx)]
        elem = next(p for p in defs if p.kind is PlaceKind.ARRAY_ELEM)
        assert any(r.kind is PlaceKind.UNKNOWN for r in reads)
        assert any(overlap(elem, r) for r in reads)


class TestNamespacePrecision:
    """Place resolution must respect the proc's namespace: `variable v` inside
    ::ns::p is ::ns::v, and distinct `namespace upvar` targets must not collapse
    to the same owner."""

    def test_variable_in_namespace_proc_resolves_to_namespace(self):
        from compiler.var_resolve import _bind_scalar

        cfg, ctx = _fn(
            "namespace eval ns { proc p {} { variable v\n set v 1 } }",
            proc="::ns::p",
        )
        assert ctx.namespace == "::ns"
        pl = _bind_scalar("v", ctx)
        assert pl.ns == "::ns"  # not the global "::"
        assert pl.name == "v"

    def test_distinct_namespace_upvar_aliases_do_not_overlap(self):
        from compiler.var_resolve import _bind_scalar

        cfg, ctx = _fn(
            "proc f {} { namespace upvar ::ns1 x a\n namespace upvar ::ns2 x b\n"
            " set a 1\n set b 2 }"
        )
        # Owners are namespace-qualified, so the two aliases are distinct vars.
        assert ctx.upvar_aliases["a"] == "::ns1::x"
        assert ctx.upvar_aliases["b"] == "::ns2::x"
        pa = _bind_scalar("a", ctx)
        pb = _bind_scalar("b", ctx)
        assert pa.owner != pb.owner
        assert not overlap(pa, pb)  # ::ns1::x and ::ns2::x must not falsely overlap

    def test_plain_upvar_owner_unchanged(self):
        # Regression guard: plain upvar (no namespace) keeps the bare caller name.
        cfg, ctx = _fn("proc f {} { upvar 1 caller local\n }")
        assert ctx.upvar_aliases.get("local") == "caller"


class TestRelativeNamespaceUpvar:
    """A relative `namespace upvar` namespace arg must resolve against the
    enclosing proc's namespace, so it matches the absolute spelling of the same
    target.  Regression for relative args being left unqualified (inner::x)."""

    def test_relative_and_absolute_namespace_upvar_overlap(self):
        from compiler.place import overlap
        from compiler.var_resolve import _bind_scalar

        cfg, ctx = _fn(
            "namespace eval outer { proc p {} {\n"
            " namespace upvar inner x a\n"
            " namespace upvar ::outer::inner x b\n"
            " set a 1\n puts $b } }",
            proc="::outer::p",
        )
        # Both resolve to the same fully-qualified owner.
        assert ctx.upvar_aliases["a"] == "::outer::inner::x"
        assert ctx.upvar_aliases["b"] == "::outer::inner::x"
        assert overlap(_bind_scalar("a", ctx), _bind_scalar("b", ctx))

    def test_distinct_relative_namespaces_still_disjoint(self):
        # Guard: two *different* relative namespaces remain distinct owners.
        from compiler.place import overlap
        from compiler.var_resolve import _bind_scalar

        cfg, ctx = _fn(
            "namespace eval outer { proc p {} {\n"
            " namespace upvar ns1 x a\n namespace upvar ns2 x b\n set a 1\n set b 2 } }",
            proc="::outer::p",
        )
        assert ctx.upvar_aliases["a"] == "::outer::ns1::x"
        assert ctx.upvar_aliases["b"] == "::outer::ns2::x"
        assert not overlap(_bind_scalar("a", ctx), _bind_scalar("b", ctx))
