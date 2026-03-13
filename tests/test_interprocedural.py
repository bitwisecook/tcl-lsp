"""Tests for Phase 5 interprocedural summaries."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.compiler.cfg import build_cfg
from core.compiler.interprocedural import (
    analyse_interprocedural_source,
    evaluate_proc_with_constants,
    fold_static_proc_call,
)
from core.compiler.lowering import lower_to_ir


class TestInterproceduralSummaries:
    def test_constant_and_passthrough_summaries(self):
        source = textwrap.dedent("""\
            proc one {} { return 1 }
            proc id {x} { return $x }
            proc noisy {} { puts hi; return 1 }
        """)
        summaries = analyse_interprocedural_source(source).procedures

        one = summaries["::one"]
        assert one.pure is True
        assert one.returns_constant is True
        assert one.constant_return == 1
        assert one.can_fold_static_calls is True

        ident = summaries["::id"]
        assert ident.pure is True
        assert ident.returns_constant is False
        assert ident.return_depends_on_params == ("x",)
        assert ident.return_passthrough_param == "x"
        assert ident.can_fold_static_calls is True

        noisy = summaries["::noisy"]
        assert noisy.has_unknown_calls is True
        assert noisy.pure is False
        assert noisy.can_fold_static_calls is False

    def test_internal_call_graph_and_purity_propagation(self):
        source = textwrap.dedent("""\
            proc leaf {} { return 5 }
            proc mid {} { leaf; return 5 }
            proc impure {} { puts hi; return 1 }
            proc caller {} { impure; return 1 }
        """)
        summaries = analyse_interprocedural_source(source).procedures

        leaf = summaries["::leaf"]
        mid = summaries["::mid"]
        impure = summaries["::impure"]
        caller = summaries["::caller"]

        assert leaf.pure is True
        assert mid.calls == ("::leaf",)
        assert mid.pure is True
        assert impure.pure is False
        assert caller.calls == ("::impure",)
        assert caller.pure is False

    def test_namespace_call_resolution(self):
        source = textwrap.dedent("""\
            namespace eval math {
                proc helper {} { return 1 }
                proc wrapper {} { helper; return 1 }
            }
        """)
        summaries = analyse_interprocedural_source(source).procedures
        wrapper = summaries["::math::wrapper"]
        assert wrapper.calls == ("::math::helper",)
        assert wrapper.pure is True

    def test_builtin_pure_calls_keep_proc_pure(self):
        source = textwrap.dedent("""\
            proc norm {x} {
                return [string tolower $x]
            }
        """)
        summaries = analyse_interprocedural_source(source).procedures
        norm = summaries["::norm"]
        assert norm.pure is True

    def test_global_writes_and_dynamic_barriers_not_pure(self):
        source = textwrap.dedent("""\
            proc setg {} { set ::x 1; return 1 }
            proc dyn {} { eval {set y 1}; return 0 }
        """)
        summaries = analyse_interprocedural_source(source).procedures

        setg = summaries["::setg"]
        dyn = summaries["::dyn"]
        assert setg.writes_global is True
        assert setg.pure is False
        assert dyn.has_barrier is True
        assert dyn.pure is False

    def test_variadic_arity_and_static_folding(self):
        source = textwrap.dedent("""\
            proc const {} { return 7 }
            proc pick {first args} { return $first }
        """)
        analysis = analyse_interprocedural_source(source)
        summaries = analysis.procedures

        pick = summaries["::pick"]
        assert pick.arity.min == 1
        assert pick.arity.is_unlimited
        assert pick.return_passthrough_param == "first"

        assert fold_static_proc_call(analysis, "::const", ()) == 7
        assert fold_static_proc_call(analysis, "::pick", ("a", "b", "c")) == "a"
        assert fold_static_proc_call(analysis, "::pick", ()) is None

    def test_constant_return_via_local_assignment(self):
        source = "proc c {} { set x 2; return $x }"
        summary = analyse_interprocedural_source(source).procedures["::c"]
        assert summary.returns_constant is True
        assert summary.constant_return == 2


class TestEvaluateProcWithConstants:
    """Tests for evaluate_proc_with_constants (re-analysis with seeded params)."""

    @staticmethod
    def _eval(source, proc_name, args):
        ir = lower_to_ir(source)
        cfg_mod = build_cfg(ir)
        proc = ir.procedures[proc_name]
        cfg = cfg_mod.procedures[proc_name]
        return evaluate_proc_with_constants(cfg, proc.params, args)

    def test_add_via_set_and_return(self):
        source = "proc add {a b} { set r [expr {$a + $b}]; return $r }"
        assert self._eval(source, "::add", (3, 4)) == 7

    def test_add_via_return_expr(self):
        source = "proc add {a b} { return [expr {$a + $b}] }"
        assert self._eval(source, "::add", (3, 4)) == 7

    def test_max_with_branching(self):
        source = textwrap.dedent("""\
            proc max {a b} {
                if {$a > $b} { return $a } else { return $b }
            }
        """)
        assert self._eval(source, "::max", (3, 7)) == 7
        assert self._eval(source, "::max", (10, 2)) == 10

    def test_double_via_local_variable(self):
        source = "proc double {x} { set y [expr {$x * 2}]; return $y }"
        assert self._eval(source, "::double", (5,)) == 10

    def test_incr_by_literal(self):
        source = "proc inc3 {x} { incr x 3; return $x }"
        assert self._eval(source, "::inc3", (10,)) == 13

    def test_incr_by_param(self):
        source = "proc add_incr {a b} { incr a $b; return $a }"
        assert self._eval(source, "::add_incr", (5, 3)) == 8

    def test_arity_mismatch_returns_none(self):
        source = "proc add {a b} { return [expr {$a + $b}] }"
        assert self._eval(source, "::add", (1,)) is None
