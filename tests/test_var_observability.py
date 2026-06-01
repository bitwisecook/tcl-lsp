"""Tests for the flow-sensitive alias/observability lattice."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from compiler.compilation_unit import ensure_compilation_unit
from compiler.core_analyses import _escaping_var_names
from compiler.registry.runtime import configure_signatures
from compiler.var_observability import EscapeFlag, analyse_var_observability


def _cfg(src: str, proc: str | None = None):
    configure_signatures(dialect="tcl9.0")
    cu = ensure_compilation_unit(src, logger=None, context="t")
    if proc is None:
        return cu.top_level.cfg
    return cu.procedures[proc].cfg


def _find_stmt(cfg, predicate):
    """Return (block, idx) of the first statement satisfying *predicate*."""
    for bname in cfg.reverse_postorder():
        for idx, stmt in enumerate(cfg.blocks[bname].statements):
            if predicate(stmt):
                return bname, idx
    raise AssertionError("statement not found")


class TestAliasFlags:
    def test_global_decl_flags_following_write(self):
        cfg = _cfg("proc f {} {global g; set g 5}\n", proc="::f")
        obs = analyse_var_observability(cfg)
        # at the `set g 5`, g is GLOBAL-aliased
        b, i = _find_stmt(cfg, lambda s: getattr(s, "name", None) == "g")
        assert obs.flag_at(b, i, "g") & EscapeFlag.GLOBAL
        assert obs.flag_at(b, i, "g").writes_outer_scope

    def test_variable_decl_is_namespace(self):
        cfg = _cfg(
            "namespace eval n {variable v 1}\nproc n::f {} {variable v; set v 5}\n",
            proc="::n::f",
        )
        obs = analyse_var_observability(cfg)
        b, i = _find_stmt(cfg, lambda s: getattr(s, "name", None) == "v")
        assert obs.flag_at(b, i, "v") & EscapeFlag.NAMESPACE

    def test_upvar_caller_frame_is_upvar(self):
        cfg = _cfg("proc f {} {upvar 1 y x; set x 5}\n", proc="::f")
        obs = analyse_var_observability(cfg)
        b, i = _find_stmt(cfg, lambda s: getattr(s, "name", None) == "x")
        flag = obs.flag_at(b, i, "x")
        assert flag & EscapeFlag.UPVAR
        assert not flag.writes_outer_scope  # caller frame, not global

    def test_upvar_zero_is_global(self):
        cfg = _cfg("proc f {} {upvar #0 g x; set x 5}\n", proc="::f")
        obs = analyse_var_observability(cfg)
        b, i = _find_stmt(cfg, lambda s: getattr(s, "name", None) == "x")
        assert obs.flag_at(b, i, "x") & EscapeFlag.GLOBAL


class TestTraceFlowSensitivity:
    SRC = (
        "set x 1\n"
        "set x 2\n"  # before the trace — NOT observed
        "trace add variable x write {apply {{a b c} {}}}\n"
        "set x 3\n"  # after the trace — observed
    )

    def test_trace_is_flow_sensitive(self):
        cfg = _cfg(self.SRC)
        obs = analyse_var_observability(cfg)
        # locate the two `set x` writes by value
        writes = [
            (b, i)
            for b in cfg.reverse_postorder()
            for i, s in enumerate(cfg.blocks[b].statements)
            if getattr(s, "name", None) == "x"
        ]
        assert len(writes) >= 3
        # the first write (value 1) and second (value 2) precede the trace
        b0, i0 = writes[0]
        b2, i2 = writes[-1]
        assert not obs.is_observed_at(b0, i0, "x"), "write before trace must be clean"
        assert obs.is_observed_at(b2, i2, "x"), "write after trace must be observed"

    def test_observed_after_trace_in_branch_merge(self):
        # trace added in one branch -> observed on the merge (union join)
        src = (
            "set x 1\n"
            "if {[clock seconds]} {trace add variable x write {apply {{a b c} {}}}}\n"
            "set x 9\n"
        )
        cfg = _cfg(src)
        obs = analyse_var_observability(cfg)
        # the final `set x 9` is in the merge block; it may be observed.
        last = [
            (b, i)
            for b in cfg.reverse_postorder()
            for i, s in enumerate(cfg.blocks[b].statements)
            if getattr(s, "name", None) == "x"
        ][-1]
        assert obs.is_observed_at(last[0], last[1], "x")


class TestWholeFunctionUnion:
    def test_escaping_names_explicit(self):
        # The whole-function union names every aliased / traced variable.
        cases = [
            ("proc f {} {global g; set g 5}\n", "::f", {"g"}),
            ("proc f {} {upvar 1 y x; set x 5}\n", "::f", {"x"}),
            (
                "namespace eval n {variable v 1}\nproc n::f {} {variable v; set v 1}\n",
                "::n::f",
                {"v"},
            ),
            ("proc f {} {set a 1\nset b 2}\n", "::f", set()),
        ]
        for src, proc, expected in cases:
            cfg = _cfg(src, proc=proc)
            assert set(analyse_var_observability(cfg).escaping_names()) == expected

    def test_union_is_single_source_for_escaping_var_names(self):
        # _escaping_var_names now derives from the same lattice.
        src = "set x 1\ntrace add variable x write {apply {{a b c} {}}}\nset x 2\n"
        cfg = _cfg(src)
        assert set(_escaping_var_names(cfg)) == set(
            analyse_var_observability(cfg).escaping_names()
        )


class TestO104UsesLatticeFlowSensitively:
    """The string-build-chain folder consults the lattice point-wise."""

    def test_chain_before_trace_folds(self):
        from compiler.optimiser import optimise_source

        configure_signatures(dialect="tcl9.0")
        src = (
            "set s a\nappend s b\nappend s c\n"
            "trace add variable s write {apply {{a b c} {}}}\nputs $s"
        )
        out, rewrites = optimise_source(src)
        assert "set s abc" in out
        assert any(r.code == "O104" for r in rewrites)

    def test_chain_after_trace_is_preserved(self):
        from compiler.optimiser import optimise_source

        configure_signatures(dialect="tcl9.0")
        src = (
            "set s a\ntrace add variable s write {apply {{a b c} {}}}\n"
            "append s b\nappend s c\nputs $s"
        )
        out, rewrites = optimise_source(src)
        assert "append s b" in out and "append s c" in out
        assert not any(r.code == "O104" for r in rewrites)
