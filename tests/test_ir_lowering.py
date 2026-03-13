"""Tests for Tcl IR lowering."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.compiler.expr_ast import ExprNode, expr_text
from core.compiler.ir import (
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRBarrier,
    IRCall,
    IRCatch,
    IRExprEval,
    IRFor,
    IRForeach,
    IRIf,
    IRIncr,
    IRReturn,
    IRSwitch,
    IRTry,
    IRWhile,
)
from core.compiler.lowering import lower_to_ir


class TestIRLowering:
    def test_simple_set_incr_call(self):
        source = "set a 1\nincr a\nputs $a"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert len(stmts) == 3
        assert isinstance(stmts[0], IRAssignConst)
        assert stmts[0].name == "a"
        assert stmts[0].value == "1"
        assert isinstance(stmts[1], IRIncr)
        assert stmts[1].name == "a"
        assert isinstance(stmts[2], IRCall)
        assert stmts[2].command == "puts"
        assert stmts[2].args == ("${a}",)

    def test_set_expr_assignment(self):
        source = "set result [expr {$x + 1}]"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert len(stmts) == 1
        assert isinstance(stmts[0], IRAssignExpr)
        assert stmts[0].name == "result"
        assert isinstance(stmts[0].expr, ExprNode)
        assert expr_text(stmts[0].expr) == "$x + 1"

    def test_set_non_const_non_expr(self):
        source = 'set name "bob"'
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert len(stmts) == 1
        assert isinstance(stmts[0], IRAssignValue)
        assert stmts[0].name == "name"
        assert stmts[0].value == "bob"

    def test_standalone_expr(self):
        """Standalone ``expr`` with braced body lowers to IRExprEval."""
        source = "expr {$x + 1}"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert len(stmts) == 1
        assert isinstance(stmts[0], IRExprEval)
        assert isinstance(stmts[0].expr, ExprNode)

    def test_standalone_expr_multiarg_fallback(self):
        """Multi-arg ``expr`` falls back to IRCall."""
        source = "expr $x + 1"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert len(stmts) == 1
        assert isinstance(stmts[0], IRCall)
        assert stmts[0].command == "expr"

    def test_proc_definition_lowered(self):
        source = textwrap.dedent("""\
            proc add {a b} {
                set c [expr {$a + $b}]
                return $c
            }
            add 1 2
        """)
        mod = lower_to_ir(source)
        assert "::add" in mod.procedures
        proc = mod.procedures["::add"]
        assert proc.params == ("a", "b")
        body_stmts = proc.body.statements
        assert any(isinstance(s, IRAssignExpr) for s in body_stmts)
        assert any(isinstance(s, IRReturn) for s in body_stmts)
        # Top-level contains both the proc IRCall (runtime definition) and
        # the ``add 1 2`` call.
        assert len(mod.top_level.statements) == 2
        assert isinstance(mod.top_level.statements[0], IRCall)
        assert mod.top_level.statements[0].command == "proc"
        assert isinstance(mod.top_level.statements[1], IRCall)
        assert mod.top_level.statements[1].command == "add"

    def test_if_lowered_to_structured_node(self):
        source = "if {$x > 0} {set y 1} elseif {$x < 0} {set y -1} else {set y 0}"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert len(stmts) == 1
        assert isinstance(stmts[0], IRIf)
        if_stmt = stmts[0]
        assert len(if_stmt.clauses) == 2
        assert isinstance(if_stmt.clauses[0].condition, ExprNode)
        assert expr_text(if_stmt.clauses[0].condition) == "$x > 0"
        assert expr_text(if_stmt.clauses[1].condition) == "$x < 0"
        assert if_stmt.else_body is not None

    def test_switch_lowered(self):
        source = "switch $x {a {set y 1} b - default {set y 0}}"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert len(stmts) == 1
        assert isinstance(stmts[0], IRSwitch)
        sw = stmts[0]
        assert sw.subject == "${x}"
        assert len(sw.arms) == 2
        assert sw.arms[0].pattern == "a"
        assert sw.arms[0].fallthrough is False
        assert sw.arms[1].pattern == "b"
        assert sw.arms[1].fallthrough is True
        assert sw.default_body is not None

    def test_for_lowered_to_structured_node(self):
        source = "for {set i 0} {$i < 5} {incr i} {set total [expr {$total + $i}]}"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert len(stmts) == 1
        assert isinstance(stmts[0], IRFor)
        loop = stmts[0]
        assert isinstance(loop.condition, ExprNode)
        assert expr_text(loop.condition) == "$i < 5"
        assert any(isinstance(s, IRAssignConst) and s.name == "i" for s in loop.init.statements)
        assert any(isinstance(s, IRIncr) and s.name == "i" for s in loop.next.statements)
        assert any(isinstance(s, IRAssignExpr) and s.name == "total" for s in loop.body.statements)

    def test_namespace_proc_qualification(self):
        source = textwrap.dedent("""\
            namespace eval math {
                proc add {a b} { return [expr {$a + $b}] }
            }
        """)
        mod = lower_to_ir(source)
        assert "::math::add" in mod.procedures
        stmts = mod.top_level.statements
        assert len(stmts) == 1
        assert isinstance(stmts[0], IRBarrier)
        assert stmts[0].reason == "namespace eval"

    def test_while_lowered_to_structured_node(self):
        source = "while {$i < 10} {incr i}"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert len(stmts) == 1
        assert isinstance(stmts[0], IRWhile)
        loop = stmts[0]
        assert isinstance(loop.condition, ExprNode)
        assert expr_text(loop.condition) == "$i < 10"
        assert any(isinstance(s, IRIncr) and s.name == "i" for s in loop.body.statements)

    # Phase 1: Variable-defining commands

    def test_append_tracks_var_def(self):
        source = "append result hello"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert len(stmts) == 1
        assert isinstance(stmts[0], IRCall)
        assert stmts[0].command == "append"
        assert stmts[0].defs == ("result",)

    def test_lappend_tracks_var_def(self):
        source = "lappend items a b c"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert len(stmts) == 1
        assert isinstance(stmts[0], IRCall)
        assert stmts[0].command == "lappend"
        assert stmts[0].defs == ("items",)

    def test_unset_tracks_var_defs(self):
        source = "unset -nocomplain -- x y"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert len(stmts) == 1
        assert isinstance(stmts[0], IRCall)
        assert stmts[0].command == "unset"
        assert stmts[0].defs == ("x", "y")

    def test_unset_nocomplain_reads_own_defs_false(self):
        source = "unset -nocomplain x"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert isinstance(stmts[0], IRCall)
        assert stmts[0].reads_own_defs is False

    def test_unset_no_nocomplain_reads_own_defs_true(self):
        source = "unset x"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert isinstance(stmts[0], IRCall)
        assert stmts[0].reads_own_defs is True

    def test_unset_terminator_reads_own_defs_false(self):
        source = "unset -nocomplain -- x"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert isinstance(stmts[0], IRCall)
        assert stmts[0].reads_own_defs is False

    def test_global_tracks_var_defs(self):
        source = "global x y z"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert len(stmts) == 1
        assert isinstance(stmts[0], IRCall)
        assert stmts[0].defs == ("x", "y", "z")

    def test_variable_tracks_var_defs(self):
        source = "variable x 1 y 2"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert len(stmts) == 1
        assert isinstance(stmts[0], IRCall)
        assert stmts[0].defs == ("x", "y")

    def test_upvar_tracks_local_var_defs(self):
        source = "upvar 1 other local"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert len(stmts) == 1
        assert isinstance(stmts[0], IRCall)
        assert stmts[0].defs == ("local",)

    def test_upvar_without_level(self):
        source = "upvar other local"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert len(stmts) == 1
        assert isinstance(stmts[0], IRCall)
        assert stmts[0].defs == ("local",)

    # Phase 3: foreach / lmap

    def test_foreach_lowered(self):
        source = "foreach x $list {puts $x}"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert len(stmts) == 1
        assert isinstance(stmts[0], IRForeach)
        fe = stmts[0]
        assert fe.iterators == ((("x",), "${list}"),)
        assert not fe.is_lmap
        assert any(isinstance(s, IRCall) and s.command == "puts" for s in fe.body.statements)

    def test_foreach_multi_var(self):
        source = "foreach {a b} $pairs {puts $a}"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert isinstance(stmts[0], IRForeach)
        assert stmts[0].iterators[0][0] == ("a", "b")

    def test_foreach_multi_list(self):
        source = "foreach x $xs y $ys {puts $x$y}"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert isinstance(stmts[0], IRForeach)
        fe = stmts[0]
        assert len(fe.iterators) == 2
        assert fe.iterators[0][0] == ("x",)
        assert fe.iterators[1][0] == ("y",)

    def test_lmap_lowered(self):
        source = "lmap x $list {expr {$x * 2}}"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert isinstance(stmts[0], IRForeach)
        assert stmts[0].is_lmap

    # Phase 4: catch / try

    def test_catch_lowered(self):
        source = "catch {expr {1/0}} result opts"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert len(stmts) == 1
        assert isinstance(stmts[0], IRCatch)
        c = stmts[0]
        assert c.result_var == "result"
        assert c.options_var == "opts"
        assert len(c.body.statements) > 0

    def test_catch_no_vars(self):
        source = "catch {error oops}"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert isinstance(stmts[0], IRCatch)
        assert stmts[0].result_var is None
        assert stmts[0].options_var is None

    def test_try_lowered(self):
        source = textwrap.dedent("""\
            try {
                set x 1
            } on error {msg opts} {
                puts $msg
            } finally {
                puts done
            }
        """)
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert len(stmts) == 1
        assert isinstance(stmts[0], IRTry)
        t = stmts[0]
        assert len(t.handlers) == 1
        assert t.handlers[0].kind == "on"
        assert t.handlers[0].match_arg == "error"
        assert t.handlers[0].var_name == "msg"
        assert t.handlers[0].options_var == "opts"
        assert t.finally_body is not None

    def test_try_no_handlers(self):
        source = "try {set x 1} finally {puts done}"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert isinstance(stmts[0], IRTry)
        assert stmts[0].handlers == ()
        assert stmts[0].finally_body is not None

    # Phase 5: dict subcommands

    def test_dict_for_lowered(self):
        source = "dict for {k v} $d {puts $k=$v}"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert len(stmts) == 1
        assert isinstance(stmts[0], IRForeach)
        fe = stmts[0]
        assert fe.iterators[0][0] == ("k", "v")
        assert not fe.is_lmap

    def test_dict_map_lowered(self):
        source = "dict map {k v} $d {list $k $v}"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert isinstance(stmts[0], IRForeach)
        assert stmts[0].is_lmap

    def test_dict_set_tracks_var(self):
        source = "dict set mydict key value"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert isinstance(stmts[0], IRCall)
        assert stmts[0].defs == ("mydict",)

    def test_dict_incr_tracks_var(self):
        source = "dict incr counts key"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert isinstance(stmts[0], IRCall)
        assert stmts[0].defs == ("counts",)

    def test_dict_get_no_defs(self):
        source = "dict get $d key"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert isinstance(stmts[0], IRCall)
        assert stmts[0].defs == ()

    def test_dict_update_barrier(self):
        source = "dict update d key1 v1 key2 v2 {puts $v1}"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert isinstance(stmts[0], IRBarrier)
        assert "dict update" in stmts[0].reason

    def test_unsupported_body_command_barrier(self):
        source = "time {incr i}"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert len(stmts) == 1
        assert isinstance(stmts[0], IRBarrier)
        assert "unsupported body command" in stmts[0].reason


class TestNamespaceArrayScalarVariableForms:
    """Variable form edge cases: namespaced + array/scalar distinctions."""

    def test_lowering_marks_braced_array_like_name_as_scalar_read(self):
        mod = lower_to_ir("set out ${a(1)}")
        stmt = mod.top_level.statements[0]
        assert isinstance(stmt, IRAssignValue)
        assert stmt.value == "$={a(1)}"

    def test_lowering_keeps_unbraced_array_ref_as_array_form(self):
        mod = lower_to_ir("set out $a(1)")
        stmt = mod.top_level.statements[0]
        assert isinstance(stmt, IRAssignValue)
        assert stmt.value == "${a(1)}"

    def test_lowering_preserves_namespaced_braced_scalar_form(self):
        mod = lower_to_ir("set ::ns::out ${::ns::arr(item)}")
        stmt = mod.top_level.statements[0]
        assert isinstance(stmt, IRAssignValue)
        assert stmt.name == "::ns::out"
        assert stmt.value == "$={::ns::arr(item)}"

    def test_lowering_tracks_namespace_array_defs_by_base_name(self):
        mod = lower_to_ir("unset ::ns::arr(item)")
        stmt = mod.top_level.statements[0]
        assert isinstance(stmt, IRCall)
        assert stmt.command == "unset"
        assert stmt.defs == ("::ns::arr",)

    def test_lowering_variable_command_tracks_namespace_scalar_and_array_defs(self):
        mod = lower_to_ir("variable ::ns::x 1 ::ns::arr(item) 2")
        stmt = mod.top_level.statements[0]
        assert isinstance(stmt, IRCall)
        assert stmt.command == "variable"
        assert stmt.defs == ("::ns::x", "::ns::arr")

    def test_lowering_global_tracks_namespace_scalar_and_array_defs(self):
        mod = lower_to_ir("global ::ns::x ::ns::arr(item)")
        stmt = mod.top_level.statements[0]
        assert isinstance(stmt, IRCall)
        assert stmt.command == "global"
        assert stmt.defs == ("::ns::x", "::ns::arr")

    def test_lowering_upvar_keeps_namespace_qualified_target_and_local_def(self):
        mod = lower_to_ir("upvar 0 ::ns::x local")
        stmt = mod.top_level.statements[0]
        assert isinstance(stmt, IRCall)
        assert stmt.command == "upvar"
        assert stmt.args == ("0", "::ns::x", "local")
        assert stmt.defs == ("local",)
