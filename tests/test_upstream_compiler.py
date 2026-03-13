"""Tests for compiler infrastructure derived from upstream Tcl test patterns.

These supplement the existing test_ir_lowering.py, test_cfg.py, test_ssa.py,
and test_type_propagation.py with additional coverage for patterns found in
the upstream Tcl test suite.

Areas covered:
- IR lowering for control flow: if/else, switch, for, foreach, while, catch,
  try/on/finally
- CFG construction: linear code, if branching, for loops, while loops,
  foreach loops, return termination
- SSA construction: version numbering, phi functions at merge points,
  dominator trees
- Type propagation: integer/float/string/boolean through assignments, expr
  results, math functions, comparisons
"""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.compiler.cfg import CFGBranch, CFGReturn, build_cfg
from core.compiler.expr_ast import ExprNode, expr_text
from core.compiler.ir import (
    IRAssignConst,
    IRCall,
    IRCatch,
    IRFor,
    IRForeach,
    IRIf,
    IRIncr,
    IRSwitch,
    IRTry,
    IRWhile,
)
from core.compiler.lowering import lower_to_ir
from core.compiler.ssa import build_ssa
from core.compiler.types import TclType, TypeKind

from .helpers import analyse_types as _analyse
from .helpers import var_type as _var_type

# IR control-flow lowering


class TestIRControlFlow:
    """Verify that Tcl control-flow commands lower to the correct IR nodes."""

    def test_if_else_lowering(self):
        """``if {$x > 0} {set y 1} else {set y 0}`` produces an IRIf with
        one clause and an else_body."""
        source = "if {$x > 0} {set y 1} else {set y 0}"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert len(stmts) == 1
        assert isinstance(stmts[0], IRIf)
        if_stmt = stmts[0]
        assert len(if_stmt.clauses) == 1
        assert isinstance(if_stmt.clauses[0].condition, ExprNode)
        assert expr_text(if_stmt.clauses[0].condition) == "$x > 0"
        assert if_stmt.else_body is not None
        # The then branch should contain ``set y 1``
        then_stmts = if_stmt.clauses[0].body.statements
        assert any(
            isinstance(s, IRAssignConst) and s.name == "y" and s.value == "1" for s in then_stmts
        )
        # The else branch should contain ``set y 0``
        else_stmts = if_stmt.else_body.statements
        assert any(
            isinstance(s, IRAssignConst) and s.name == "y" and s.value == "0" for s in else_stmts
        )

    def test_if_elseif_else_lowering(self):
        """``if {$a} {set r 1} elseif {$b} {set r 2} else {set r 3}``
        produces an IRIf with 2 clauses and an else_body."""
        source = "if {$a} {set r 1} elseif {$b} {set r 2} else {set r 3}"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert len(stmts) == 1
        assert isinstance(stmts[0], IRIf)
        if_stmt = stmts[0]
        assert len(if_stmt.clauses) == 2
        assert expr_text(if_stmt.clauses[0].condition) == "$a"
        assert expr_text(if_stmt.clauses[1].condition) == "$b"
        assert if_stmt.else_body is not None
        # Verify the else body contains ``set r 3``
        else_stmts = if_stmt.else_body.statements
        assert any(
            isinstance(s, IRAssignConst) and s.name == "r" and s.value == "3" for s in else_stmts
        )

    def test_switch_lowering(self):
        """``switch $x { a {set r 1} b {set r 2} default {set r 0} }``
        produces an IRSwitch with arms and a default_body."""
        source = "switch $x { a {set r 1} b {set r 2} default {set r 0} }"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert len(stmts) == 1
        assert isinstance(stmts[0], IRSwitch)
        sw = stmts[0]
        # Two non-default arms: "a" and "b"
        assert len(sw.arms) == 2
        assert sw.arms[0].pattern == "a"
        assert sw.arms[1].pattern == "b"
        assert sw.default_body is not None
        # The default body should contain ``set r 0``
        default_stmts = sw.default_body.statements
        assert any(
            isinstance(s, IRAssignConst) and s.name == "r" and s.value == "0" for s in default_stmts
        )

    def test_for_lowering(self):
        """``for {set i 0} {$i < 10} {incr i} {puts $i}`` produces an IRFor
        with init, condition, next, and body scripts."""
        source = "for {set i 0} {$i < 10} {incr i} {puts $i}"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert len(stmts) == 1
        assert isinstance(stmts[0], IRFor)
        loop = stmts[0]
        assert isinstance(loop.condition, ExprNode)
        assert expr_text(loop.condition) == "$i < 10"
        # Init should contain ``set i 0``
        assert any(
            isinstance(s, IRAssignConst) and s.name == "i" and s.value == "0"
            for s in loop.init.statements
        )
        # Next should contain ``incr i``
        assert any(isinstance(s, IRIncr) and s.name == "i" for s in loop.next.statements)
        # Body should contain ``puts $i``
        assert any(isinstance(s, IRCall) and s.command == "puts" for s in loop.body.statements)

    def test_foreach_lowering(self):
        """``foreach item $list {puts $item}`` produces an IRForeach with
        the correct iterator specification."""
        source = "foreach item $list {puts $item}"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert len(stmts) == 1
        assert isinstance(stmts[0], IRForeach)
        fe = stmts[0]
        assert len(fe.iterators) == 1
        assert fe.iterators[0][0] == ("item",)
        # The list argument should reference $list
        assert "list" in fe.iterators[0][1]
        assert not fe.is_lmap
        # Body should contain ``puts $item``
        assert any(isinstance(s, IRCall) and s.command == "puts" for s in fe.body.statements)

    def test_while_lowering(self):
        """``while {$x > 0} {incr x -1}`` produces an IRWhile with the
        correct condition and body."""
        source = "while {$x > 0} {incr x -1}"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert len(stmts) == 1
        assert isinstance(stmts[0], IRWhile)
        loop = stmts[0]
        assert isinstance(loop.condition, ExprNode)
        assert expr_text(loop.condition) == "$x > 0"
        # Body should contain ``incr x -1``
        assert any(
            isinstance(s, IRIncr) and s.name == "x" and s.amount == "-1"
            for s in loop.body.statements
        )

    def test_catch_lowering(self):
        """``catch {expr {1/0}} result`` produces an IRCatch with a
        result_var."""
        source = "catch {expr {1/0}} result"
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert len(stmts) == 1
        assert isinstance(stmts[0], IRCatch)
        c = stmts[0]
        assert c.result_var == "result"
        assert c.options_var is None
        assert len(c.body.statements) > 0

    def test_try_finally_lowering(self):
        """``try { set f [open file.txt] } finally { close $f }`` produces
        an IRTry with a finally_body and no handlers."""
        source = textwrap.dedent("""\
            try {
                set f [open file.txt]
            } finally {
                close $f
            }
        """)
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert len(stmts) == 1
        assert isinstance(stmts[0], IRTry)
        t = stmts[0]
        assert t.handlers == ()
        assert t.finally_body is not None
        # The finally body should contain ``close $f``
        finally_stmts = t.finally_body.statements
        assert any(isinstance(s, IRCall) and s.command == "close" for s in finally_stmts)

    def test_try_on_error_lowering(self):
        """``try { expr {1/0} } on error {msg opts} { puts $msg }``
        produces an IRTry with one handler."""
        source = textwrap.dedent("""\
            try {
                expr {1/0}
            } on error {msg opts} {
                puts $msg
            }
        """)
        mod = lower_to_ir(source)
        stmts = mod.top_level.statements
        assert len(stmts) == 1
        assert isinstance(stmts[0], IRTry)
        t = stmts[0]
        assert len(t.handlers) == 1
        handler = t.handlers[0]
        assert handler.kind == "on"
        assert handler.match_arg == "error"
        assert handler.var_name == "msg"
        assert handler.options_var == "opts"
        # The handler body should contain ``puts $msg``
        assert any(isinstance(s, IRCall) and s.command == "puts" for s in handler.body.statements)


# CFG construction


class TestCFGControlFlow:
    """Verify that structured IR lowers to the expected CFG shapes."""

    def test_while_creates_loop_cfg(self):
        """A ``while`` loop should produce a CFGBranch at the loop header
        block, modelling the conditional back-edge."""
        source = textwrap.dedent("""\
            while {$x > 0} {
                incr x -1
            }
        """)
        mod = lower_to_ir(source)
        cfg = build_cfg(mod).top_level
        # There should be at least one CFGBranch (the while header check)
        branch_blocks = [
            name
            for name, block in cfg.blocks.items()
            if isinstance(block.terminator, CFGBranch) and "while_header" in name
        ]
        assert branch_blocks, "Expected a while_header block with CFGBranch terminator"
        # Verify the branch condition references the original expression
        header_name = branch_blocks[0]
        term = cfg.blocks[header_name].terminator
        assert isinstance(term, CFGBranch)
        assert "$x > 0" in expr_text(term.condition)

    def test_foreach_creates_loop_cfg(self):
        """A ``foreach`` loop should produce a CFGBranch or CFGGoto
        establishing the loop structure."""
        source = textwrap.dedent("""\
            foreach item $list {
                puts $item
            }
        """)
        mod = lower_to_ir(source)
        cfg = build_cfg(mod).top_level
        # The foreach should produce a header block with a CFGBranch
        # (the synthetic "has next element?" condition)
        branch_blocks = [
            name
            for name, block in cfg.blocks.items()
            if isinstance(block.terminator, CFGBranch) and "foreach_header" in name
        ]
        assert branch_blocks, "Expected a foreach_header block with CFGBranch terminator"
        # The foreach body block should exist and be reachable from the header
        header_name = branch_blocks[0]
        term = cfg.blocks[header_name].terminator
        assert isinstance(term, CFGBranch)
        assert term.true_target in cfg.blocks
        assert term.false_target in cfg.blocks

    def test_nested_if_cfg(self):
        """Nested ``if`` statements should produce multiple CFGBranch
        terminators in the CFG."""
        source = textwrap.dedent("""\
            if {$a} {
                if {$b} {set r 1} else {set r 2}
            } else {
                set r 3
            }
        """)
        mod = lower_to_ir(source)
        cfg = build_cfg(mod).top_level
        branch_count = sum(1 for b in cfg.blocks.values() if isinstance(b.terminator, CFGBranch))
        # At least two branches: outer if and inner if
        assert branch_count >= 2, f"Expected >= 2 CFGBranch terminators, got {branch_count}"

    def test_proc_cfg_separate(self):
        """A ``proc`` definition should produce a separate CFG entry in the
        procedures dict, and its body should have its own CFGReturn."""
        source = textwrap.dedent("""\
            proc foo {x} {
                if {$x > 0} { return 1 }
                return 0
            }
        """)
        mod = lower_to_ir(source)
        cfgm = build_cfg(mod)
        assert "::foo" in cfgm.procedures
        proc_cfg = cfgm.procedures["::foo"]
        # The procedure CFG should contain at least one CFGReturn
        has_return = any(isinstance(b.terminator, CFGReturn) for b in proc_cfg.blocks.values())
        assert has_return, "Expected CFGReturn in proc foo"
        # The procedure should also have a CFGBranch for the if statement
        has_branch = any(isinstance(b.terminator, CFGBranch) for b in proc_cfg.blocks.values())
        assert has_branch, "Expected CFGBranch in proc foo for the if condition"


# SSA construction


class TestSSAControlFlow:
    """Verify phi placement and SSA versioning for control-flow constructs."""

    def test_while_loop_phi(self):
        """A variable defined before a ``while`` loop and modified inside
        it should have a phi node at the loop header."""
        source = textwrap.dedent("""\
            set x 10
            while {$x > 0} {
                incr x -1
            }
        """)
        mod = lower_to_ir(source)
        cfg = build_cfg(mod).top_level
        ssa = build_ssa(cfg)
        # There should be a phi for "x" at the while_header block
        phi_for_x = any(
            any(phi.name == "x" for phi in block.phis)
            for block in ssa.blocks.values()
            if "while_header" in block.name
        )
        assert phi_for_x, "Expected phi node for 'x' at the while loop header"

    def test_foreach_phi(self):
        """A variable modified inside a ``foreach`` body should have a
        phi node at the foreach header (loop entry)."""
        source = textwrap.dedent("""\
            set total 0
            foreach item {1 2 3} {
                incr total $item
            }
        """)
        mod = lower_to_ir(source)
        cfg = build_cfg(mod).top_level
        ssa = build_ssa(cfg)
        # There should be a phi for "total" at the foreach header block
        phi_for_total = any(
            any(phi.name == "total" for phi in block.phis)
            for block in ssa.blocks.values()
            if "foreach_header" in block.name
        )
        assert phi_for_total, "Expected phi node for 'total' at the foreach header"

    def test_nested_if_phi(self):
        """Variables assigned in nested ``if`` branches should have phi
        nodes at the merge points after the control flow reconverges."""
        source = textwrap.dedent("""\
            if {$cond1} {
                set r 1
            } else {
                if {$cond2} {
                    set r 2
                } else {
                    set r 3
                }
            }
            set x $r
        """)
        mod = lower_to_ir(source)
        cfg = build_cfg(mod).top_level
        ssa = build_ssa(cfg)
        # There should be at least one phi for "r" at a merge point
        phi_for_r = any(any(phi.name == "r" for phi in block.phis) for block in ssa.blocks.values())
        assert phi_for_r, "Expected phi node for 'r' at an if merge point"

    def test_multiple_variables_versioned(self):
        """Multiple assignments to the same variable should produce
        incrementing SSA version numbers in the entry block."""
        source = textwrap.dedent("""\
            set a 1
            set b 2
            set a 3
            set b 4
        """)
        mod = lower_to_ir(source)
        cfg = build_cfg(mod).top_level
        ssa = build_ssa(cfg)
        entry_block = ssa.blocks[ssa.entry]
        # 'a' should have versions 1 and 2 (two definitions)
        a_versions = [stmt.defs["a"] for stmt in entry_block.statements if "a" in stmt.defs]
        assert a_versions == [1, 2], f"Expected a versions [1, 2], got {a_versions}"
        # 'b' should have versions 1 and 2 (two definitions)
        b_versions = [stmt.defs["b"] for stmt in entry_block.statements if "b" in stmt.defs]
        assert b_versions == [1, 2], f"Expected b versions [1, 2], got {b_versions}"
        # The exit versions should reflect the latest definitions
        assert entry_block.exit_versions.get("a") == 2
        assert entry_block.exit_versions.get("b") == 2


# Type inference


class TestTypeInference:
    """Type propagation tests covering upstream Tcl value semantics."""

    def test_integer_assignment(self):
        """``set x 42`` should infer x as TclType.INT."""
        analysis = _analyse("set x 42")
        t = _var_type(analysis, "x")
        assert t is not None
        assert t.kind is TypeKind.KNOWN
        assert t.tcl_type is TclType.INT

    def test_float_assignment(self):
        """``set x 3.14`` should infer x as TclType.DOUBLE."""
        analysis = _analyse("set x 3.14")
        t = _var_type(analysis, "x")
        assert t is not None
        assert t.kind is TypeKind.KNOWN
        assert t.tcl_type is TclType.DOUBLE

    def test_string_assignment(self):
        """``set x hello`` should infer x as TclType.STRING."""
        analysis = _analyse("set x hello")
        t = _var_type(analysis, "x")
        assert t is not None
        assert t.kind is TypeKind.KNOWN
        assert t.tcl_type is TclType.STRING

    def test_boolean_from_comparison(self):
        """``set x [expr {$a > 0}]`` should infer x as TclType.BOOLEAN
        because comparisons always yield a boolean result."""
        analysis = _analyse("set a 1\nset x [expr {$a > 0}]")
        t = _var_type(analysis, "x")
        assert t is not None
        assert t.tcl_type is TclType.BOOLEAN

    def test_float_from_math_function(self):
        """``set x [expr {sin(1.0)}]`` should infer x as TclType.DOUBLE
        because ``sin`` always returns a floating-point value."""
        analysis = _analyse("set x [expr {sin(1.0)}]")
        t = _var_type(analysis, "x")
        assert t is not None
        assert t.tcl_type is TclType.DOUBLE

    def test_integer_arithmetic(self):
        """``set x [expr {1 + 2}]`` should infer x as TclType.INT
        because both operands are integers."""
        analysis = _analyse("set x [expr {1 + 2}]")
        t = _var_type(analysis, "x")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_float_arithmetic(self):
        """``set x [expr {1.0 + 2.0}]`` should infer x as TclType.DOUBLE
        because both operands are floating-point."""
        analysis = _analyse("set x [expr {1.0 + 2.0}]")
        t = _var_type(analysis, "x")
        assert t is not None
        assert t.tcl_type is TclType.DOUBLE

    def test_incr_is_integer(self):
        """``incr`` always produces an integer, regardless of the initial
        value type."""
        analysis = _analyse("set x 0\nincr x")
        t = _var_type(analysis, "x")
        assert t is not None
        assert t.tcl_type is TclType.INT

    def test_type_narrows_through_reassignment(self):
        """Reassigning a variable should produce different types for
        different SSA versions: version 1 is STRING, version 2 is INT."""
        analysis = _analyse("set x hello\nset x 42")
        t_v1 = _var_type(analysis, "x", version=1)
        t_v2 = _var_type(analysis, "x", version=2)
        assert t_v1 is not None
        assert t_v1.tcl_type is TclType.STRING
        assert t_v2 is not None
        assert t_v2.tcl_type is TclType.INT
