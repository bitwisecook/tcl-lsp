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
        # Static ``namespace eval`` lowers to ``IRBlock`` now — the
        # WASM codegen inlines the body, while the VM codegen falls
        # back to dispatching the original ``namespace eval`` call
        # via ``source_args``.  The canonical shape has the nested
        # proc lifted (checked above) and the block namespace set.
        from core.compiler.ir import IRBlock

        assert isinstance(stmts[0], IRBlock)
        assert stmts[0].namespace == "::math"
        assert stmts[0].source_args[:2] == ("eval", "math")

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


class TestProcDynamicNameConstMap:
    """P6.1 — ``proc $var body`` resolves to a static name when
    ``$var`` is in the lowering const-map.  Tcltest's ``Option``
    factory is the driving shape:

        proc Configure {name ...} {
            proc $name {{value {}}} [subst -nocommands {body}]
        }
        Configure verbose 0

    Inside ``Configure``'s body, ``$name`` has a const value (``verbose``)
    — the new lowering path substitutes it, creating a real
    ``::verbose`` ``IRProcedure`` instead of an ``IRBarrier`` +
    runtime dispatch.
    """

    def test_proc_dollar_name_bails_when_var_not_const_mapped(self):
        # No preceding literal ``set`` — ``$name`` stays dynamic.
        source = textwrap.dedent("""\
            proc Factory {name body} {
                proc $name {x} $body
            }
        """)
        mod = lower_to_ir(source)
        assert "::Factory" in mod.procedures
        body = mod.procedures["::Factory"].body.statements
        # Without const-tracking, the inner ``proc $name …`` must
        # fall through to a runtime dispatch barrier.
        assert any(isinstance(s, IRBarrier) and "dynamic proc name" in s.reason for s in body)

    def test_proc_dollar_name_resolves_when_var_is_literal(self):
        # The const-map only tracks ``set var {braced-literal}`` shapes
        # (see ``_set_literal_body``) — a bare ESC ``set name Verbose``
        # would NOT be tracked because decimal / bareword values are
        # out of scope for the const-map gate by design.
        source = textwrap.dedent("""\
            proc Factory {body} {
                set name {Verbose}
                proc $name {x} $body
            }
        """)
        mod = lower_to_ir(source)
        # The const-map tracks ``set name {Verbose}`` inside the
        # proc body; the inner ``proc $name`` gets substituted so
        # we end up with a real ``::Verbose`` IRProcedure.
        assert "::Verbose" in mod.procedures
        factory_body = mod.procedures["::Factory"].body.statements
        # The factory's body contains an IRCall for the proc
        # command (with the substituted name).
        proc_calls = [s for s in factory_body if isinstance(s, IRCall) and s.command == "proc"]
        assert len(proc_calls) == 1
        # args[0] should be the resolved name, not ``$name``.
        assert proc_calls[0].args[0] == "Verbose"

    def test_proc_dollar_name_rejects_command_substitution(self):
        # ``$name`` mixed with a ``[…]`` command substitution still
        # bails — command subs could introduce side effects that the
        # compiler can't reason about at lowering time.
        source = textwrap.dedent("""\
            proc Factory {body} {
                set name {Verbose}
                proc $name[suffix] {x} $body
            }
        """)
        mod = lower_to_ir(source)
        body = mod.procedures["::Factory"].body.statements
        assert any(isinstance(s, IRBarrier) and "dynamic proc name" in s.reason for s in body)
        # And no synthetic proc was registered.
        assert not any(name.endswith("::Verbose") for name in mod.procedures)

    def test_proc_dollar_name_respects_reassignment(self):
        # A re-assignment before the ``proc $name`` swaps the literal —
        # the substitution should pick up the latest value.
        source = textwrap.dedent("""\
            proc Factory {body} {
                set name {First}
                set name {Second}
                proc $name {x} $body
            }
        """)
        mod = lower_to_ir(source)
        assert "::Second" in mod.procedures
        assert "::First" not in mod.procedures

    def test_nested_proc_body_does_not_inherit_outer_const_map(self):
        # Regression: an outer proc's ``set body {literal}`` must
        # not be visible to a NESTED ``proc inner``'s barrier-
        # relaxation gate.  At runtime ``inner`` has its own frame
        # with no ``body`` local, so lowering must not bake the
        # outer literal into ``inner``'s body.
        source = textwrap.dedent("""\
            proc Outer {} {
                set body {set z 99}
                proc inner {} {
                    uplevel 1 $body
                }
            }
        """)
        mod = lower_to_ir(source)
        assert "::inner" in mod.procedures
        inner = mod.procedures["::inner"]
        # ``uplevel 1 $body`` inside ``inner`` must remain an
        # IRBarrier (dynamic body) — not a relaxed IRBlock /
        # IRUpFrame containing the outer's ``set z 99`` literal.
        from core.compiler.ir import IRBarrier, IRBlock, IRUpFrame

        for s in inner.body.statements:
            assert not isinstance(s, IRUpFrame), (
                "inner's uplevel must stay dynamic (outer's const-map leaked)"
            )
            if isinstance(s, IRBlock):
                # Guard against a relaxed inline too — IRBlock is
                # the inline form used by ``_relax_eval`` / uplevel
                # inlining.
                for inner_s in s.body.statements:
                    assert "z" not in getattr(inner_s, "name", ""), (
                        "inner's uplevel body was inlined from outer's const literal"
                    )
        # The barrier is the correct shape.
        assert any(
            isinstance(s, IRBarrier) and s.command == "uplevel" for s in inner.body.statements
        )


class TestProcDynamicBodySubstNocommands:
    """P7.3 — ``proc $var [subst -nocommands {template}]`` materialises
    the template at compile time when every template ``$var`` is
    const-tracked in the enclosing proc.  This is the full tcltest
    ``Option`` factory shape."""

    def test_subst_nocommands_body_materialises_with_const_vars(self):
        # ``Factory`` has two const-tracked locals (``name``,
        # ``default``); ``proc`` uses both in the name AND in the
        # body-template.  Both substitutions happen at compile time,
        # producing a real ``::Verbose`` IRProcedure whose body
        # references the materialised literal.
        source = textwrap.dedent("""\
            proc Factory {} {
                set name {Verbose}
                set default {0}
                proc $name {x} [subst -nocommands {return $default}]
            }
        """)
        mod = lower_to_ir(source)
        assert "::Verbose" in mod.procedures
        verbose = mod.procedures["::Verbose"]
        # The body should now contain a ``return 0`` (the template's
        # ``$default`` was substituted to ``0``).
        from core.compiler.ir import IRReturn

        body_stmts = verbose.body.statements
        assert any(isinstance(s, IRReturn) for s in body_stmts)

    def test_subst_nocommands_body_refuses_missing_var(self):
        # Template has ``$unbound`` which isn't in the const-map.
        # The lowering should refuse the materialisation and fall
        # through to the normal path — which in turn bails since
        # the body is a command sub, not a braced literal.
        source = textwrap.dedent("""\
            proc Factory {} {
                set name {Verbose}
                proc $name {x} [subst -nocommands {return $unbound}]
            }
        """)
        mod = lower_to_ir(source)
        # The outer ``proc $name ...`` registers ``::Verbose``, but its body
        # could not be materialised (``$unbound`` isn't constant), so the body
        # must NOT contain a materialised IRReturn — the template stayed
        # dynamic.
        from core.compiler.ir import IRReturn

        assert "::Verbose" in mod.procedures, list(mod.procedures)
        verbose = mod.procedures["::Verbose"]
        assert not any(isinstance(s, IRReturn) for s in verbose.body.statements)

    def test_subst_nocommands_body_refuses_nobackslashes_flag(self):
        # ``-nobackslashes`` changes semantics (no backslash
        # processing).  Our evaluator always decodes backslashes
        # (matching Tcl's default), so we refuse materialisation
        # when the source opts out.
        source = textwrap.dedent("""\
            proc Factory {} {
                set name {Verbose}
                set default {0}
                proc $name {x} [subst -nobackslashes -nocommands {return $default}]
            }
        """)
        mod = lower_to_ir(source)
        # ``-nobackslashes`` opts out of our evaluator's semantics, so the
        # body is not materialised: ``::Verbose`` registers but carries no
        # IRReturn.
        from core.compiler.ir import IRReturn

        assert "::Verbose" in mod.procedures, list(mod.procedures)
        verbose = mod.procedures["::Verbose"]
        assert not any(isinstance(s, IRReturn) for s in verbose.body.statements)


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


class TestAliasLowering:
    """IR-level tests for interp alias handling."""

    def test_set_expr_alias_produces_assign_expr(self):
        """``set x [= {$a + 1}]`` with ``=`` alias for ``expr`` -> IRAssignExpr."""
        source = textwrap.dedent("""\
            interp alias {} = {} expr
            set x [= {$a + 1}]
        """)
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assign_stmts = [s for s in stmts if isinstance(s, IRAssignExpr)]
        assert len(assign_stmts) == 1
        assert assign_stmts[0].name == "x"

    def test_alias_body_command_produces_barrier(self):
        """Alias for eval -> IRBarrier (unsupported body command)."""
        source = textwrap.dedent("""\
            interp alias {} myeval {} eval
            myeval { set x 1 }
        """)
        mod = lower_to_ir(source)
        barriers = [s for s in mod.top_level.statements if isinstance(s, IRBarrier)]
        assert any(b.reason == "unsupported body command" for b in barriers)

    def test_alias_var_defs_correctly_adjusted(self):
        """Alias for scan — var defs indices adjusted for the alias."""
        source = textwrap.dedent("""\
            interp alias {} myscan {} scan
            myscan {123} {%d} myvar
        """)
        mod = lower_to_ir(source)
        calls = [s for s in mod.top_level.statements if isinstance(s, IRCall)]
        var_calls = [c for c in calls if c.defs]
        assert len(var_calls) >= 1
        assert any("myvar" in c.defs for c in var_calls)

    def test_real_world_suchenwirth_equals(self):
        """Suchenwirth's classic ``=`` alias: set var [= {expr}] -> IRAssignExpr."""
        source = textwrap.dedent("""\
            interp alias {} = {} expr
            proc hypot {a b} {
                set c2 [= {$a*$a + $b*$b}]
                set c [= {sqrt($c2)}]
                return $c
            }
        """)
        mod = lower_to_ir(source)
        proc = mod.procedures.get("::hypot")
        assert proc is not None
        assign_exprs = [s for s in proc.body.statements if isinstance(s, IRAssignExpr)]
        assert len(assign_exprs) == 2
        assert assign_exprs[0].name == "c2"
        assert assign_exprs[1].name == "c"


class TestNamespaceDirectiveDeadCode:
    """``namespace import`` / ``namespace export`` inside statically-
    dead ``if`` branches must not register with ``IRModule``'s
    import / export tables — the codegen turns those into
    compile-time dispatch rewrites and a dead-code directive can
    silently globalise an unqualified call to a namespace that was
    never actually populated at runtime.

    See https://github.com/bitwisecook/tcl-lsp/issues/189.
    """

    def test_namespace_import_in_if_false_not_collected(self):
        """``if {0} { namespace import ... }`` leaves the module's
        ``namespace_imports`` table empty."""
        source = textwrap.dedent("""\
            namespace eval ::other {
                namespace export evil
                proc evil {} { return bad }
            }
            if {0} {
                namespace import ::other::evil
            }
        """)
        mod = lower_to_ir(source)
        assert all(pat != "::other::evil" for _, pat in mod.namespace_imports)

    def test_namespace_export_in_if_false_not_collected(self):
        """``if {0} { namespace export ... }`` leaves the module's
        ``namespace_exports`` table free of the dead directive."""
        source = textwrap.dedent("""\
            namespace eval ::other {
                if {0} {
                    namespace export evil
                }
            }
        """)
        mod = lower_to_ir(source)
        assert all(not (ns == "::other" and pat == "evil") for ns, pat in mod.namespace_exports)

    def test_issue_189_shape(self):
        """The exact shape from issue #189: BOTH the export and the
        matching import live in the same dead block, so the
        export-pattern filter alone wouldn't catch the bad rewrite.
        Dead-code suppression prevents either from registering."""
        source = textwrap.dedent("""\
            if {0} {
                namespace eval ::other {
                    namespace export evil
                }
                namespace import ::other::evil
            }
            evil
        """)
        mod = lower_to_ir(source)
        assert mod.namespace_imports == ()
        # An ``evil`` export collected from dead code would still let
        # the codegen try to dispatch ``evil`` directly if some other
        # path added the matching import — so the export must stay
        # uncollected too.
        assert all(pat != "evil" for _, pat in mod.namespace_exports)

    def test_if_true_else_branch_is_dead(self):
        """``if {1} { live } else { namespace import X }`` — the
        else branch is dead, so its directive isn't collected."""
        source = textwrap.dedent("""\
            if {1} {
                set x 1
            } else {
                namespace import ::other::evil
            }
        """)
        mod = lower_to_ir(source)
        assert all(pat != "::other::evil" for _, pat in mod.namespace_imports)

    def test_if_true_elseif_branch_is_dead(self):
        """``if {1} { live } elseif {…} { dead }`` — once the first
        clause is statically true, every later clause is dead."""
        source = textwrap.dedent("""\
            if {1} {
                set x 1
            } elseif {$y == 1} {
                namespace import ::other::evil
            }
        """)
        mod = lower_to_ir(source)
        assert all(pat != "::other::evil" for _, pat in mod.namespace_imports)

    def test_if_false_false_alternative_boolean_literals(self):
        """Tcl accepts ``no``/``off`` as false booleans — the dead-
        code gate respects them too (case-insensitive)."""
        source = textwrap.dedent("""\
            if {No} {
                namespace import ::other::x
            }
            if {OFF} {
                namespace import ::other::y
            }
        """)
        mod = lower_to_ir(source)
        pats = {pat for _, pat in mod.namespace_imports}
        assert "::other::x" not in pats
        assert "::other::y" not in pats

    def test_live_if_still_collects(self):
        """Regression: directives in genuinely-live ``if`` branches
        are still captured (the existing code path stays working)."""
        source = textwrap.dedent("""\
            namespace eval ::other {
                namespace export ok
                proc ok {} { return good }
            }
            if {1} {
                namespace import ::other::ok
            }
        """)
        mod = lower_to_ir(source)
        pats = {pat for _, pat in mod.namespace_imports}
        assert "::other::ok" in pats

    def test_unqualified_unconditional_still_collects(self):
        """Regression: top-level directives (no ``if`` wrapper) are
        captured — dead-code depth stays at 0 for the default path."""
        source = textwrap.dedent("""\
            namespace eval ::other {
                namespace export ok
                proc ok {} { return good }
            }
            namespace import ::other::ok
        """)
        mod = lower_to_ir(source)
        pats = {pat for _, pat in mod.namespace_imports}
        assert "::other::ok" in pats

    def test_nested_dead_inside_live_if(self):
        """``if {1} { if {0} { namespace import … } }`` — the inner
        branch is dead because the inner condition is false, even
        though the outer condition is live."""
        source = textwrap.dedent("""\
            if {1} {
                if {0} {
                    namespace import ::other::evil
                }
            }
        """)
        mod = lower_to_ir(source)
        assert all(pat != "::other::evil" for _, pat in mod.namespace_imports)

    def test_dynamic_condition_still_collects(self):
        """Dynamic conditions (a variable / command substitution)
        can't be folded at compile time — we pessimistically treat
        them as live so we never *lose* a legitimate directive."""
        source = textwrap.dedent("""\
            set flag 1
            if {$flag} {
                namespace import ::other::ok
            }
        """)
        mod = lower_to_ir(source)
        pats = {pat for _, pat in mod.namespace_imports}
        assert "::other::ok" in pats
