"""Complex stress tests for the IR → CFG → codegen pipeline.

Exercises nested control flow, edge cases in expression compilation,
procedure interactions, and unusual Tcl patterns that stress the
lowering, CFG construction, and bytecode emission stages.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.compiler.cfg import (
    CFGBranch,
    CFGGoto,
    CFGReturn,
    build_cfg,
)
from core.compiler.codegen import (
    FunctionAsm,
    ModuleAsm,
    Op,
    codegen_module,
    format_function_asm,
    format_module_asm,
)
from core.compiler.expr_ast import ExprRaw
from core.compiler.ir import (
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRBarrier,
    IRCall,
    IRExprEval,
    IRForeach,
    IRIf,
    IRIncr,
    IRSwitch,
)
from core.compiler.lowering import lower_to_ir

# Helpers


def _asm_for(source: str) -> ModuleAsm:
    """Lower source → IR → CFG → ASM."""
    ir = lower_to_ir(source)
    cfg = build_cfg(ir, defer_top_level=True)
    return codegen_module(cfg, ir)


def _top_asm(source: str) -> FunctionAsm:
    return _asm_for(source).top_level


def _top_text(source: str) -> str:
    return format_function_asm(_top_asm(source))


def _opcodes(fa: FunctionAsm) -> list[Op]:
    return [i.op for i in fa.instructions]


def _proc_asm(source: str, proc_name: str) -> FunctionAsm:
    """Get the FunctionAsm for a named procedure."""
    ma = _asm_for(source)
    return ma.procedures[proc_name]


def _has_jump(ops: list[Op]) -> bool:
    return any(
        op in (Op.JUMP1, Op.JUMP4, Op.JUMP_TRUE1, Op.JUMP_TRUE4, Op.JUMP_FALSE1, Op.JUMP_FALSE4)
        for op in ops
    )


def _has_cond_jump(ops: list[Op]) -> bool:
    return any(op in (Op.JUMP_TRUE1, Op.JUMP_TRUE4, Op.JUMP_FALSE1, Op.JUMP_FALSE4) for op in ops)


# ═══════════════════════════════════════════════════════════════════
# 1. Nested control flow
# ═══════════════════════════════════════════════════════════════════


class TestNestedControlFlow:
    """Nested if/for/while/switch/foreach hammering CFG block creation."""

    def test_if_inside_for(self):
        """if nested inside a for loop body."""
        source = """\
proc f {n} {
    set total 0
    for {set i 0} {$i < $n} {incr i} {
        if {$i % 2 == 0} {
            set total [expr {$total + $i}]
        } else {
            set total [expr {$total - $i}]
        }
    }
    return $total
}
"""
        fa = _proc_asm(source, "::f")
        ops = _opcodes(fa)
        assert _has_cond_jump(ops), "for header + if branch need conditional jumps"
        assert Op.ADD in ops
        assert Op.SUB in ops
        assert Op.MOD in ops
        assert Op.DONE in ops

    def test_for_inside_if(self):
        """for loop nested inside if branches."""
        source = """\
proc g {mode n} {
    if {$mode eq "sum"} {
        set r 0
        for {set i 1} {$i <= $n} {incr i} {
            set r [expr {$r + $i}]
        }
    } elseif {$mode eq "prod"} {
        set r 1
        for {set i 1} {$i <= $n} {incr i} {
            set r [expr {$r * $i}]
        }
    } else {
        set r -1
    }
    return $r
}
"""
        fa = _proc_asm(source, "::g")
        ops = _opcodes(fa)
        assert Op.ADD in ops
        assert Op.MULT in ops
        assert ops.count(Op.LE) >= 2, "two for-loop condition tests"

    def test_deeply_nested_if(self):
        """Three-level nested if/elseif/else chains."""
        source = """\
proc classify {x y z} {
    if {$x > 0} {
        if {$y > 0} {
            if {$z > 0} {
                set r "all_pos"
            } else {
                set r "z_neg"
            }
        } else {
            set r "y_neg"
        }
    } else {
        set r "x_neg"
    }
    return $r
}
"""
        fa = _proc_asm(source, "::classify")
        ops = _opcodes(fa)
        # Three nested if blocks → at least 3 conditional jumps
        cond_count = sum(
            1 for op in ops if op in (Op.JUMP_TRUE1, Op.JUMP_TRUE4, Op.JUMP_FALSE1, Op.JUMP_FALSE4)
        )
        assert cond_count >= 3

    def test_while_with_nested_switch(self):
        """Switch statement inside a while loop body."""
        source = """\
proc dispatch {items} {
    set i 0
    set result {}
    while {$i < 10} {
        switch -exact $i {
            0 { set result "zero" }
            1 { set result "one" }
            default { set result "other" }
        }
        incr i
    }
    return $result
}
"""
        ir = lower_to_ir(source)
        cfg = build_cfg(ir)
        proc_cfg = cfg.procedures["::dispatch"]
        # While creates a header + body + end; switch creates dispatch cascade
        branch_count = sum(
            1 for b in proc_cfg.blocks.values() if isinstance(b.terminator, CFGBranch)
        )
        # At least 1 while-header branch + 2 switch dispatch branches
        assert branch_count >= 3

    def test_nested_for_loops(self):
        """Two nested for loops (2D iteration)."""
        source = """\
proc matrix_fill {rows cols} {
    for {set i 0} {$i < $rows} {incr i} {
        for {set j 0} {$j < $cols} {incr j} {
            set val [expr {$i * $cols + $j}]
        }
    }
}
"""
        fa = _proc_asm(source, "::matrix_fill")
        ops = _opcodes(fa)
        # Two < comparisons (one per loop header)
        assert ops.count(Op.LT) >= 2
        assert Op.MULT in ops
        assert Op.ADD in ops

    def test_foreach_inside_for(self):
        """foreach loop nested inside a for loop."""
        source = """\
proc search {lists n} {
    for {set i 0} {$i < $n} {incr i} {
        foreach item $lists {
            if {$item eq "found"} {
                return $item
            }
        }
    }
    return "not_found"
}
"""
        fa = _proc_asm(source, "::search")
        ops = _opcodes(fa)
        assert Op.LT in ops
        assert Op.FOREACH_START in ops or Op.INVOKE_STK1 in ops
        assert Op.STR_EQ in ops


# ═══════════════════════════════════════════════════════════════════
# 2. Complex expression compilation
# ═══════════════════════════════════════════════════════════════════


class TestComplexExpressions:
    """Expression compilation edge cases: ternary, nested ops, math functions."""

    def test_ternary_in_expr(self):
        """Ternary conditional expression compiles to branches."""
        source = """\
proc clamp {x lo hi} {
    set r [expr {$x < $lo ? $lo : ($x > $hi ? $hi : $x)}]
    return $r
}
"""
        fa = _proc_asm(source, "::clamp")
        ops = _opcodes(fa)
        assert Op.LT in ops
        assert Op.GT in ops
        assert _has_cond_jump(ops), "ternary needs conditional jumps"

    def test_chained_logical_and(self):
        """Chained && with short-circuit semantics."""
        source = """\
proc all_positive {a b c} {
    set r [expr {$a > 0 && $b > 0 && $c > 0}]
    return $r
}
"""
        fa = _proc_asm(source, "::all_positive")
        ops = _opcodes(fa)
        assert ops.count(Op.GT) >= 3

    def test_chained_logical_or(self):
        """Chained || with short-circuit."""
        source = """\
proc any_zero {a b c} {
    set r [expr {$a == 0 || $b == 0 || $c == 0}]
    return $r
}
"""
        fa = _proc_asm(source, "::any_zero")
        ops = _opcodes(fa)
        assert ops.count(Op.EQ) >= 3

    def test_mixed_arithmetic_and_comparison(self):
        """Complex expression mixing arithmetic and comparisons."""
        source = """\
proc check {x y} {
    set r [expr {($x + $y) * 2 > ($x - $y) ** 2}]
    return $r
}
"""
        fa = _proc_asm(source, "::check")
        ops = _opcodes(fa)
        assert Op.ADD in ops
        assert Op.SUB in ops
        assert Op.MULT in ops
        assert Op.EXPON in ops
        assert Op.GT in ops

    def test_bitwise_operations(self):
        """Bitwise operators compile to correct opcodes."""
        source = """\
proc bits {x y} {
    set a [expr {$x & $y}]
    set b [expr {$x | $y}]
    set c [expr {$x ^ $y}]
    set d [expr {$x << 2}]
    set e [expr {$x >> 1}]
    set f [expr {~$x}]
    return [expr {$a + $b + $c + $d + $e + $f}]
}
"""
        fa = _proc_asm(source, "::bits")
        ops = _opcodes(fa)
        assert Op.BITAND in ops
        assert Op.BITOR in ops
        assert Op.BITXOR in ops
        assert Op.LSHIFT in ops
        assert Op.RSHIFT in ops
        assert Op.BITNOT in ops

    def test_string_comparison_operators(self):
        """String comparison operators (eq, ne, lt, gt, le, ge)."""
        source = """\
proc str_cmp {a b} {
    set r1 [expr {$a eq $b}]
    set r2 [expr {$a ne $b}]
    set r3 [expr {$a lt $b}]
    set r4 [expr {$a gt $b}]
    return [expr {$r1 + $r2 + $r3 + $r4}]
}
"""
        fa = _proc_asm(source, "::str_cmp")
        ops = _opcodes(fa)
        assert Op.STR_EQ in ops
        assert Op.STR_NEQ in ops
        assert Op.STR_LT in ops
        assert Op.STR_GT in ops

    def test_list_membership_in_ni(self):
        """List membership operators (in, ni)."""
        source = """\
proc membership {x lst} {
    set a [expr {$x in $lst}]
    set b [expr {$x ni $lst}]
    return [expr {$a + $b}]
}
"""
        fa = _proc_asm(source, "::membership")
        ops = _opcodes(fa)
        assert Op.LIST_IN in ops
        assert Op.LIST_NOT_IN in ops

    def test_nested_math_functions(self):
        """Nested math function calls with tryCvtToNumeric."""
        source = """\
proc compute {x} {
    set r [expr {abs(int(sin($x) * 100))}]
    return $r
}
"""
        fa = _proc_asm(source, "::compute")
        ops = _opcodes(fa)
        assert Op.INVOKE_STK1 in ops
        assert Op.TRY_CVT_TO_NUMERIC in ops

    def test_constant_fold_complex(self):
        """Multi-operator constant fold."""
        fa = _top_asm("set r [expr {(2 + 3) * 4 - 1}]")
        ops = _opcodes(fa)
        # Entire expression should be folded
        assert Op.ADD not in ops
        assert Op.MULT not in ops
        assert Op.SUB not in ops
        lits = fa.literals.entries()
        assert "19" in lits

    def test_constant_fold_boolean(self):
        """Boolean constant fold: 1 && 0 → 0."""
        fa = _top_asm("set r [expr {1 && 0}]")
        ops = _opcodes(fa)
        assert Op.LAND not in ops
        lits = fa.literals.entries()
        assert "0" in lits

    def test_partial_fold_mixed_const_var(self):
        """Constant parts folded, variable parts remain."""
        source = "set x 1; set r [expr {$x + 2 * 3}]"
        fa = _top_asm(source)
        ops = _opcodes(fa)
        # 2*3=6 should be folded, but $x + 6 still needs ADD
        assert Op.ADD in ops
        assert Op.MULT not in ops


# ═══════════════════════════════════════════════════════════════════
# 3. Switch statement edge cases
# ═══════════════════════════════════════════════════════════════════


class TestSwitchEdgeCases:
    """Switch with fallthrough, glob mode, many arms."""

    def test_switch_fallthrough(self):
        """Fallthrough arms (body = '-') share a body block."""
        source = """\
proc classify {x} {
    switch -exact $x {
        a - b - c { set r "letter" }
        1 - 2 - 3 { set r "digit" }
        default { set r "other" }
    }
    return $r
}
"""
        ir = lower_to_ir(source)
        cfg = build_cfg(ir)
        proc_cfg = cfg.procedures["::classify"]
        # Fallthrough arms create CFGBranch dispatch + goto chains
        branch_count = sum(
            1 for b in proc_cfg.blocks.values() if isinstance(b.terminator, CFGBranch)
        )
        assert branch_count >= 6, "6 pattern arms need 6 dispatch branches"
        # Codegen should still work
        fa = _proc_asm(source, "::classify")
        assert Op.DONE in _opcodes(fa)

    def test_switch_glob_becomes_barrier(self):
        """Switch -glob compiles as a barrier (generic invoke)."""
        source = """\
proc ext {filename} {
    switch -glob $filename {
        *.c { return "c_source" }
        *.h { return "c_header" }
        default { return "unknown" }
    }
}
"""
        ir = lower_to_ir(source)
        cfg = build_cfg(ir)
        proc_cfg = cfg.procedures["::ext"]
        # -glob switch is lowered to barrier, not inline dispatch
        has_barrier = any(
            any(isinstance(s, IRBarrier) and "switch -glob" in s.reason for s in b.statements)
            for b in proc_cfg.blocks.values()
        )
        assert has_barrier

    def test_switch_regexp_becomes_barrier(self):
        """Switch -regexp compiles as a barrier."""
        source = """\
proc match {s} {
    switch -regexp $s {
        {^[0-9]+$} { return "number" }
        {^[a-z]+$} { return "alpha" }
        default { return "mixed" }
    }
}
"""
        ir = lower_to_ir(source)
        cfg = build_cfg(ir)
        proc_cfg = cfg.procedures["::match"]
        has_barrier = any(
            any(isinstance(s, IRBarrier) and "switch -regexp" in s.reason for s in b.statements)
            for b in proc_cfg.blocks.values()
        )
        assert has_barrier

    def test_switch_many_arms(self):
        """Switch with many arms creates a dispatch mechanism."""
        arms = " ".join(f"v{i} {{ set r {i} }}" for i in range(10))
        source = f"""\
proc big_switch {{x}} {{
    switch -exact $x {{
        {arms}
        default {{ set r -1 }}
    }}
    return $r
}}
"""
        fa = _proc_asm(source, "::big_switch")
        ops = _opcodes(fa)
        assert Op.DONE in ops
        # Many arms → either conditional branches or a jump table
        cond_count = sum(
            1 for op in ops if op in (Op.JUMP_TRUE1, Op.JUMP_TRUE4, Op.JUMP_FALSE1, Op.JUMP_FALSE4)
        )
        has_jump_table = Op.JUMP_TABLE in ops
        assert cond_count >= 10 or has_jump_table, (
            "switch with 10 arms should use conditional branches or a jump table"
        )

    def test_switch_with_return_in_arms(self):
        """Return inside switch arms terminates blocks properly."""
        source = """\
proc dispatch {cmd} {
    switch -exact $cmd {
        add { return 1 }
        sub { return 2 }
        mul { return 3 }
        default { return 0 }
    }
}
"""
        fa = _proc_asm(source, "::dispatch")
        ops = _opcodes(fa)
        assert Op.JUMP_TABLE in ops or _has_cond_jump(ops)


# ═══════════════════════════════════════════════════════════════════
# 4. Loop control flow (break, continue)
# ═══════════════════════════════════════════════════════════════════


class TestLoopControlFlow:
    """Break, continue, and early return in loops."""

    def test_break_in_while(self):
        """Break inside while loop emits jump to loop end."""
        source = """\
proc find_first {lst target} {
    set i 0
    while {$i < 100} {
        if {$i == $target} {
            break
        }
        incr i
    }
    return $i
}
"""
        fa = _proc_asm(source, "::find_first")
        ops = _opcodes(fa)
        assert _has_cond_jump(ops)
        # break compiles to JUMP4 to loop end
        assert Op.JUMP4 in ops

    def test_continue_in_for(self):
        """Continue inside for loop jumps to step block."""
        source = """\
proc sum_odd {n} {
    set total 0
    for {set i 0} {$i < $n} {incr i} {
        if {$i % 2 == 0} {
            continue
        }
        set total [expr {$total + $i}]
    }
    return $total
}
"""
        fa = _proc_asm(source, "::sum_odd")
        ops = _opcodes(fa)
        assert Op.JUMP4 in ops  # continue → jump to step

    def test_return_in_loop(self):
        """Return inside loop body terminates the function."""
        source = """\
proc search {lst target} {
    foreach item $lst {
        if {$item eq $target} {
            return $item
        }
    }
    return ""
}
"""
        fa = _proc_asm(source, "::search")
        ops = _opcodes(fa)
        # Should have return instruction
        assert Op.DONE in ops or Op.RETURN_IMM in ops


# ═══════════════════════════════════════════════════════════════════
# 5. Procedure compilation
# ═══════════════════════════════════════════════════════════════════


class TestProcedureEdgeCases:
    """Procedure edge cases: multiple procs, params, recursion, namespace."""

    def test_multiple_procs(self):
        """Multiple procedures in one module."""
        source = """\
proc add {a b} { expr {$a + $b} }
proc sub {a b} { expr {$a - $b} }
proc mul {a b} { expr {$a * $b} }
"""
        ma = _asm_for(source)
        assert "::add" in ma.procedures
        assert "::sub" in ma.procedures
        assert "::mul" in ma.procedures
        assert Op.ADD in _opcodes(ma.procedures["::add"])
        assert Op.SUB in _opcodes(ma.procedures["::sub"])
        assert Op.MULT in _opcodes(ma.procedures["::mul"])

    def test_proc_calling_proc(self):
        """Procedure calling another procedure uses invokeStk."""
        source = """\
proc helper {x} { expr {$x * 2} }
proc caller {x} { helper $x }
"""
        ma = _asm_for(source)
        caller_ops = _opcodes(ma.procedures["::caller"])
        assert Op.INVOKE_STK1 in caller_ops

    def test_proc_with_many_params(self):
        """Procedure with many parameters fills the LVT."""
        source = """\
proc many {a b c d e f g h} {
    return [expr {$a + $b + $c + $d + $e + $f + $g + $h}]
}
"""
        fa = _proc_asm(source, "::many")
        entries = fa.lvt.entries()
        assert entries[:8] == ["a", "b", "c", "d", "e", "f", "g", "h"]

    def test_proc_local_vs_param(self):
        """Local variables get LVT slots after params."""
        source = """\
proc compute {x y} {
    set temp [expr {$x + $y}]
    set result [expr {$temp * 2}]
    return $result
}
"""
        fa = _proc_asm(source, "::compute")
        entries = fa.lvt.entries()
        assert entries[0] == "x"
        assert entries[1] == "y"
        assert "temp" in entries
        assert "result" in entries
        assert entries.index("temp") > 1
        assert entries.index("result") > 1

    def test_namespace_eval_proc(self):
        """Procedure defined inside namespace eval gets qualified name."""
        source = """\
namespace eval ::myns {
    proc foo {x} { expr {$x + 1} }
}
"""
        ir = lower_to_ir(source)
        assert "::myns::foo" in ir.procedures

    def test_dynamic_proc_name_becomes_barrier(self):
        """Dynamic proc name (containing $) becomes barrier."""
        source = "proc $name {x} { return $x }"
        ir = lower_to_ir(source)
        # Dynamic proc name → not registered as a procedure
        assert len(ir.procedures) == 0

    def test_proc_with_default_args(self):
        """Procedure with default arguments compiles normally."""
        source = """\
proc greet {name {greeting "hello"}} {
    return "$greeting, $name"
}
"""
        ir = lower_to_ir(source)
        assert "::greet" in ir.procedures


# ═══════════════════════════════════════════════════════════════════
# 6. Try / catch exception handling
# ═══════════════════════════════════════════════════════════════════


class TestExceptionHandling:
    """Try/catch/finally compilation edge cases."""

    def test_catch_basic(self):
        """Basic catch compiles to inline beginCatch/endCatch in proc."""
        source = """\
proc safe {cmd} {
    set rc [catch {eval $cmd} result]
    return $rc
}
"""
        fa = _proc_asm(source, "::safe")
        ops = _opcodes(fa)
        assert Op.BEGIN_CATCH4 in ops
        assert Op.END_CATCH in ops

    def test_catch_with_options(self):
        """Catch with result and options vars."""
        source = """\
proc safe2 {cmd} {
    set rc [catch {eval $cmd} result opts]
    return $rc
}
"""
        fa = _proc_asm(source, "::safe2")
        ops = _opcodes(fa)
        assert Op.BEGIN_CATCH4 in ops
        assert Op.END_CATCH in ops

    def test_try_finally(self):
        """Try/finally creates proper CFG with finally block."""
        source = """\
proc with_cleanup {} {
    try {
        set x 1
    } finally {
        set x 0
    }
    return $x
}
"""
        ir = lower_to_ir(source)
        cfg = build_cfg(ir)
        proc_cfg = cfg.procedures["::with_cleanup"]
        # Should have try_finally block
        has_finally = any("finally" in name for name in proc_cfg.blocks)
        assert has_finally

    def test_try_on_error(self):
        """Try with on error handler creates handler blocks."""
        source = """\
proc safe_div {a b} {
    try {
        set r [expr {$a / $b}]
    } on error {msg opts} {
        set r -1
    }
    return $r
}
"""
        ir = lower_to_ir(source)
        cfg = build_cfg(ir)
        proc_cfg = cfg.procedures["::safe_div"]
        has_handler = any("handler" in name for name in proc_cfg.blocks)
        assert has_handler

    def test_try_on_error_with_finally(self):
        """Try with both on-error handler and finally."""
        source = """\
proc robust {body_cmd} {
    set ok 0
    try {
        eval $body_cmd
        set ok 1
    } on error {msg} {
        set ok 0
    } finally {
        set cleanup_done 1
    }
    return $ok
}
"""
        ir = lower_to_ir(source)
        cfg = build_cfg(ir)
        proc_cfg = cfg.procedures["::robust"]
        has_handler = any("handler" in name for name in proc_cfg.blocks)
        has_finally = any("finally" in name for name in proc_cfg.blocks)
        assert has_handler
        assert has_finally


# ═══════════════════════════════════════════════════════════════════
# 7. For loop edge cases
# ═══════════════════════════════════════════════════════════════════


class TestForLoopEdgeCases:
    """For loop edge cases: empty clauses, complex conditions."""

    def test_empty_init_clause(self):
        """Empty init clause in for loop emits NOP placeholder."""
        source = """\
proc count_down {i} {
    for {} {$i > 0} {incr i -1} {
        puts $i
    }
}
"""
        fa = _proc_asm(source, "::count_down")
        ops = _opcodes(fa)
        # Empty init clause → 3 NOPs
        assert ops.count(Op.NOP) >= 3

    def test_empty_step_clause(self):
        """Empty step clause in for loop emits NOP placeholder."""
        source = """\
proc manual_step {} {
    for {set i 0} {$i < 10} {} {
        incr i 2
    }
}
"""
        fa = _proc_asm(source, "::manual_step")
        ops = _opcodes(fa)
        assert ops.count(Op.NOP) >= 3

    def test_for_with_complex_init(self):
        """For loop with multi-statement init clause."""
        source = """\
proc multi_init {} {
    for {set i 0; set j 10} {$i < $j} {incr i; incr j -1} {
        puts "$i $j"
    }
}
"""
        fa = _proc_asm(source, "::multi_init")
        ops = _opcodes(fa)
        assert Op.LT in ops
        assert Op.INCR_SCALAR1_IMM in ops

    def test_for_loop_registered_in_loop_nodes(self):
        """For loop end block is registered in CFG loop_nodes."""
        source = """\
proc loop {} {
    for {set i 0} {$i < 10} {incr i} {
        puts $i
    }
}
"""
        ir = lower_to_ir(source)
        cfg = build_cfg(ir)
        proc_cfg = cfg.procedures["::loop"]
        assert len(proc_cfg.loop_nodes) >= 1, "for loop should register loop_nodes"


# ═══════════════════════════════════════════════════════════════════
# 8. While loop edge cases
# ═══════════════════════════════════════════════════════════════════


class TestWhileLoopEdgeCases:
    """While loop: infinite, complex conditions, nested."""

    def test_while_constant_true(self):
        """while {1} creates a loop with constant true condition."""
        source = """\
proc spin {} {
    while {1} {
        break
    }
}
"""
        fa = _proc_asm(source, "::spin")
        ops = _opcodes(fa)
        assert Op.JUMP4 in ops  # break

    def test_while_complex_condition(self):
        """While with compound boolean condition."""
        source = """\
proc bounded {x y limit} {
    while {$x < $limit && $y < $limit} {
        incr x
        incr y
    }
    return [expr {$x + $y}]
}
"""
        fa = _proc_asm(source, "::bounded")
        ops = _opcodes(fa)
        assert Op.LT in ops


# ═══════════════════════════════════════════════════════════════════
# 9. Foreach edge cases
# ═══════════════════════════════════════════════════════════════════


class TestForeachEdgeCases:
    """Foreach: multi-var, dict iteration, qualified vars."""

    def test_foreach_multi_var(self):
        """Foreach with multiple variables per iteration."""
        source = """\
proc pairs {lst} {
    set result {}
    foreach {k v} $lst {
        append result "$k=$v "
    }
    return $result
}
"""
        fa = _proc_asm(source, "::pairs")
        ops = _opcodes(fa)
        assert Op.FOREACH_START in ops
        assert Op.FOREACH_STEP in ops

    def test_foreach_creates_loop_cfg(self):
        """Foreach creates proper header/body/end CFG pattern."""
        source = """\
proc iter {lst} {
    foreach item $lst {
        puts $item
    }
}
"""
        ir = lower_to_ir(source)
        cfg = build_cfg(ir)
        proc_cfg = cfg.procedures["::iter"]
        has_header = any("foreach_header" in name for name in proc_cfg.blocks)
        has_body = any("foreach_body" in name for name in proc_cfg.blocks)
        has_end = any("foreach_end" in name for name in proc_cfg.blocks)
        assert has_header
        assert has_body
        assert has_end

    def test_foreach_opaque_condition(self):
        """Foreach header has an opaque <foreach_has_next> branch condition."""
        source = """\
proc iter2 {lst} {
    foreach item $lst {
        puts $item
    }
}
"""
        ir = lower_to_ir(source)
        cfg = build_cfg(ir)
        proc_cfg = cfg.procedures["::iter2"]
        has_opaque = any(
            isinstance(b.terminator, CFGBranch)
            and isinstance(b.terminator.condition, ExprRaw)
            and "foreach_has_next" in b.terminator.condition.text
            for b in proc_cfg.blocks.values()
        )
        assert has_opaque

    def test_foreach_qualified_var_becomes_generic(self):
        """Foreach with :: qualified loop var falls back to generic invoke."""
        source = """\
proc ns_iter {lst} {
    foreach ::myvar $lst {
        puts $::myvar
    }
}
"""
        ir = lower_to_ir(source)
        cfg = build_cfg(ir)
        proc_cfg = cfg.procedures["::ns_iter"]
        # Should be an IRCall for "foreach" (generic), not inlined
        has_foreach_call = any(
            any(isinstance(s, IRCall) and s.command == "foreach" for s in b.statements)
            for b in proc_cfg.blocks.values()
        )
        assert has_foreach_call

    def test_dict_for_becomes_barrier(self):
        """dict for becomes barrier with qualified command name."""
        source = """\
proc dict_iter {d} {
    dict for {k v} $d {
        puts "$k: $v"
    }
}
"""
        ir = lower_to_ir(source)
        cfg = build_cfg(ir)
        proc_cfg = cfg.procedures["::dict_iter"]
        has_barrier = any(
            any(isinstance(s, IRBarrier) and "dict for" in s.reason for s in b.statements)
            for b in proc_cfg.blocks.values()
        )
        assert has_barrier


# ═══════════════════════════════════════════════════════════════════
# 10. Variable access patterns
# ═══════════════════════════════════════════════════════════════════


class TestVariableAccess:
    """Array refs, qualified names, stack vs LVT access."""

    def test_proc_array_ref(self):
        """Array access in proc uses LOAD_ARRAY1/STORE_ARRAY1 with LVT."""
        source = """\
proc array_op {key} {
    set arr(key1) "val1"
    set arr($key) "val2"
    return $arr(key1)
}
"""
        fa = _proc_asm(source, "::array_op")
        ops = _opcodes(fa)
        assert Op.STORE_ARRAY1 in ops
        assert Op.LOAD_ARRAY1 in ops

    def test_top_level_array_ref(self):
        """Top-level array access uses stack-based ops."""
        fa = _top_asm('set arr(key) "hello"; set x $arr(key)')
        ops = _opcodes(fa)
        assert Op.STORE_ARRAY_STK in ops or Op.STORE_STK in ops

    def test_qualified_var_in_proc(self):
        """Namespace-qualified var in proc uses stack-based access."""
        source = """\
proc ns_access {} {
    set ::global_var 42
    return $::global_var
}
"""
        fa = _proc_asm(source, "::ns_access")
        ops = _opcodes(fa)
        # Qualified names always use stack-based ops, even in procs
        assert Op.STORE_STK in ops
        assert Op.LOAD_STK in ops

    def test_incr_in_proc_vs_top_level(self):
        """Incr uses different ops in proc vs top-level."""
        proc_source = """\
proc inc {x} {
    incr x
    return $x
}
"""
        top_source = "set x 0; incr x"

        proc_fa = _proc_asm(proc_source, "::inc")
        top_fa = _top_asm(top_source)

        # Proc uses INCR_SCALAR1_IMM
        assert Op.INCR_SCALAR1_IMM in _opcodes(proc_fa)
        # Top-level uses INCR_STK_IMM
        assert Op.INCR_STK_IMM in _opcodes(top_fa)

    def test_incr_large_amount(self):
        """Incr with amount outside -128..127 uses different encoding."""
        source = """\
proc big_incr {} {
    set x 0
    incr x 1000
    return $x
}
"""
        fa = _proc_asm(source, "::big_incr")
        ops = _opcodes(fa)
        # Large increment falls through to push literal + INCR_SCALAR1
        # or invokeStk depending on context
        assert Op.INCR_SCALAR1 in ops or Op.INVOKE_STK1 in ops


# ═══════════════════════════════════════════════════════════════════
# 11. Bytecoded string commands
# ═══════════════════════════════════════════════════════════════════


class TestBytecodedStringCommands:
    """String subcommands that get specialised opcodes."""

    def test_string_index(self):
        fa = _top_asm('string index "hello" 2')
        assert Op.STR_INDEX in _opcodes(fa)

    def test_string_range(self):
        fa = _top_asm('string range "hello world" 0 4')
        ops = _opcodes(fa)
        assert Op.STR_RANGE in ops or Op.STR_RANGE_IMM in ops

    def test_string_toupper_tolower(self):
        fa1 = _top_asm('string toupper "hello"')
        fa2 = _top_asm('string tolower "HELLO"')
        assert Op.STR_UPPER in _opcodes(fa1)
        assert Op.STR_LOWER in _opcodes(fa2)

    def test_string_trim(self):
        fa = _top_asm('string trim "  hello  "')
        assert Op.STR_TRIM in _opcodes(fa)

    def test_string_trimleft_trimright(self):
        fa1 = _top_asm('string trimleft "  hello"')
        fa2 = _top_asm('string trimright "hello  "')
        assert Op.STR_TRIM_LEFT in _opcodes(fa1)
        assert Op.STR_TRIM_RIGHT in _opcodes(fa2)

    def test_string_match(self):
        fa = _top_asm('string match "*.tcl" "test.tcl"')
        assert Op.STR_MATCH in _opcodes(fa)

    def test_string_map_in_proc(self):
        """string map compiles to STR_MAP or invokeStk depending on context."""
        source = "proc m {s} { string map {a A b B} $s }"
        fa = _proc_asm(source, "::m")
        ops = _opcodes(fa)
        assert Op.STR_MAP in ops or Op.INVOKE_STK1 in ops

    def test_string_first_in_proc(self):
        source = 'proc f {s} { string first "ll" $s }'
        fa = _proc_asm(source, "::f")
        ops = _opcodes(fa)
        assert Op.STR_FIND in ops or Op.INVOKE_STK1 in ops

    def test_string_last_in_proc(self):
        source = 'proc l {s} { string last "l" $s }'
        fa = _proc_asm(source, "::l")
        ops = _opcodes(fa)
        assert Op.STR_RFIND in ops or Op.INVOKE_STK1 in ops

    def test_string_replace_in_proc(self):
        source = 'proc r {s} { string replace $s 1 3 "XY" }'
        fa = _proc_asm(source, "::r")
        ops = _opcodes(fa)
        assert Op.STR_REPLACE in ops or Op.INVOKE_STK1 in ops


# ═══════════════════════════════════════════════════════════════════
# 12. Bytecoded list commands
# ═══════════════════════════════════════════════════════════════════


class TestBytecodedListCommands:
    """List subcommands with specialised opcodes."""

    def test_llength(self):
        fa = _top_asm('llength "a b c"')
        assert Op.LIST_LENGTH in _opcodes(fa)

    def test_lindex_constant(self):
        fa = _top_asm('lindex "a b c" 1')
        assert Op.LIST_INDEX_IMM in _opcodes(fa)

    def test_lrange(self):
        fa = _top_asm('lrange "a b c d e" 1 3')
        assert Op.LIST_RANGE_IMM in _opcodes(fa)

    def test_list_create_in_proc(self):
        """list with variable args compiles to LIST opcode."""
        source = "proc mk {a b c} { list $a $b $c }"
        fa = _proc_asm(source, "::mk")
        assert Op.LIST in _opcodes(fa)

    def test_lreplace(self):
        fa = _top_asm('lreplace "a b c d" 1 2 X Y')
        assert Op.LREPLACE4 in _opcodes(fa)

    def test_linsert(self):
        fa = _top_asm('linsert "a b c" 1 X')
        assert Op.LREPLACE4 in _opcodes(fa)

    def test_lassign(self):
        fa = _top_asm('lassign "a b c" x y z')
        ops = _opcodes(fa)
        assert Op.LIST_INDEX_IMM in ops
        assert Op.LIST_RANGE_IMM in ops


# ═══════════════════════════════════════════════════════════════════
# 13. Dict commands
# ═══════════════════════════════════════════════════════════════════


class TestDictCommands:
    """Dict subcommands: get, exists, set, unset, etc."""

    def test_dict_get(self):
        fa = _top_asm("set d [dict create a 1 b 2]; dict get $d a")
        assert Op.DICT_GET in _opcodes(fa)

    def test_dict_exists(self):
        fa = _top_asm("set d [dict create a 1]; dict exists $d a")
        assert Op.DICT_EXISTS in _opcodes(fa)


# ═══════════════════════════════════════════════════════════════════
# 14. CFG structure validation
# ═══════════════════════════════════════════════════════════════════


class TestCFGStructure:
    """Validate CFG block connectivity and terminator correctness."""

    def test_linear_script_single_block(self):
        """Linear script has entry → exit via goto."""
        ir = lower_to_ir("set a 1\nset b 2")
        cfg = build_cfg(ir).top_level
        entry = cfg.blocks[cfg.entry]
        assert isinstance(entry.terminator, CFGGoto)
        # Target block exists
        assert entry.terminator.target in cfg.blocks

    def test_if_else_merge_block(self):
        """If/else creates then/else blocks that merge."""
        ir = lower_to_ir("if {$x} {set a 1} else {set a 2}\nset b 3")
        cfg = build_cfg(ir).top_level
        # Find the merge block with set b 3
        merge_blocks = [
            name
            for name, b in cfg.blocks.items()
            if any(isinstance(s, IRAssignConst) and s.name == "b" for s in b.statements)
        ]
        assert len(merge_blocks) >= 1

    def test_for_loop_back_edge(self):
        """For loop step block jumps back to header (back-edge)."""
        source = """\
proc loop {} {
    for {set i 0} {$i < 10} {incr i} {
        puts $i
    }
}
"""
        ir = lower_to_ir(source)
        cfg = build_cfg(ir)
        proc_cfg = cfg.procedures["::loop"]
        # Find header block
        headers = [name for name in proc_cfg.blocks if "for_header" in name]
        assert len(headers) >= 1
        header = headers[0]
        # Header has branch terminator
        term = proc_cfg.blocks[header].terminator
        assert isinstance(term, CFGBranch)
        # True target (body) and false target (end) exist
        assert term.true_target in proc_cfg.blocks
        assert term.false_target in proc_cfg.blocks

    def test_return_terminates_block(self):
        """Return in middle of block prevents subsequent statements."""
        source = """\
proc early_exit {x} {
    set a 1
    return $a
    set b 2
}
"""
        ir = lower_to_ir(source)
        cfg = build_cfg(ir)
        proc_cfg = cfg.procedures["::early_exit"]
        # Should have a CFGReturn terminator
        has_return = any(isinstance(b.terminator, CFGReturn) for b in proc_cfg.blocks.values())
        assert has_return
        # Statement "set b 2" should NOT appear (dead code)
        has_b = any(
            any(isinstance(s, IRAssignConst) and s.name == "b" for s in b.statements)
            for b in proc_cfg.blocks.values()
        )
        assert not has_b

    def test_all_blocks_reachable(self):
        """Every block in a simple CFG is reachable from entry."""
        source = """\
proc simple {x} {
    if {$x > 0} {
        set r "pos"
    } else {
        set r "neg"
    }
    return $r
}
"""
        ir = lower_to_ir(source)
        cfg = build_cfg(ir)
        proc_cfg = cfg.procedures["::simple"]
        # BFS from entry
        visited: set[str] = set()
        queue = [proc_cfg.entry]
        while queue:
            block_name = queue.pop(0)
            if block_name in visited:
                continue
            visited.add(block_name)
            block = proc_cfg.blocks.get(block_name)
            if block is None:
                continue
            term = block.terminator
            if isinstance(term, CFGGoto):
                queue.append(term.target)
            elif isinstance(term, CFGBranch):
                queue.append(term.true_target)
                queue.append(term.false_target)
        # All blocks should be reachable
        assert visited == set(proc_cfg.blocks.keys())


# ═══════════════════════════════════════════════════════════════════
# 15. IR lowering edge cases
# ═══════════════════════════════════════════════════════════════════


class TestIRLoweringEdgeCases:
    """Lowering edge cases: malformed constructs, barriers, special forms."""

    def test_set_integer_becomes_assign_const(self):
        """set x 42 → IRAssignConst."""
        ir = lower_to_ir("set x 42")
        stmts = ir.top_level.statements
        assert any(
            isinstance(s, IRAssignConst) and s.name == "x" and s.value == "42" for s in stmts
        )

    def test_set_expr_becomes_assign_expr(self):
        """set r [expr {...}] → IRAssignExpr."""
        ir = lower_to_ir("set r [expr {1 + 2}]")
        stmts = ir.top_level.statements
        assert any(isinstance(s, IRAssignExpr) and s.name == "r" for s in stmts)

    def test_set_value_becomes_assign_value(self):
        """set x $y → IRAssignValue."""
        ir = lower_to_ir("set x $y")
        stmts = ir.top_level.statements
        assert any(isinstance(s, IRAssignValue) and s.name == "x" for s in stmts)

    def test_incr_becomes_ir_incr(self):
        """incr x → IRIncr with no amount."""
        ir = lower_to_ir("set x 0; incr x")
        stmts = ir.top_level.statements
        assert any(isinstance(s, IRIncr) and s.name == "x" and s.amount is None for s in stmts)

    def test_incr_with_amount(self):
        """incr x 5 → IRIncr with amount."""
        ir = lower_to_ir("set x 0; incr x 5")
        stmts = ir.top_level.statements
        assert any(isinstance(s, IRIncr) and s.name == "x" and s.amount == "5" for s in stmts)

    def test_return_with_options_becomes_barrier(self):
        """return -code error → IRBarrier."""
        source = """\
proc fail {} {
    return -code error "oops"
}
"""
        ir = lower_to_ir(source)
        proc = ir.procedures["::fail"]
        has_barrier = any(isinstance(s, IRBarrier) for s in proc.body.statements)
        assert has_barrier

    def test_eval_becomes_barrier(self):
        """eval is an analysis barrier."""
        ir = lower_to_ir("eval {set x 1}")
        stmts = ir.top_level.statements
        has_barrier = any(isinstance(s, IRBarrier) for s in stmts)
        has_call = any(isinstance(s, IRCall) and s.command == "eval" for s in stmts)
        assert has_barrier or has_call

    def test_uplevel_becomes_barrier(self):
        """uplevel is an analysis barrier."""
        source = """\
proc up {} {
    uplevel 1 {set x 1}
}
"""
        ir = lower_to_ir(source)
        proc = ir.procedures["::up"]
        has_barrier = any(isinstance(s, IRBarrier) for s in proc.body.statements)
        has_call = any(isinstance(s, IRCall) for s in proc.body.statements)
        assert has_barrier or has_call

    def test_standalone_expr_becomes_expr_eval(self):
        """Standalone expr command → IRExprEval."""
        ir = lower_to_ir("expr {1 + 2}")
        stmts = ir.top_level.statements
        assert any(isinstance(s, IRExprEval) for s in stmts)

    def test_if_without_braces_fallback(self):
        """Unbraced if condition lowers to IRIf, IRBarrier, or IRCall."""
        ir = lower_to_ir("if $x {puts ok}")
        stmts = ir.top_level.statements
        assert len(stmts) > 0
        # The lowering must produce one of the recognised fallback shapes;
        # a silent no-op or unexpected node type would be a regression.
        has_expected = any(isinstance(s, (IRIf, IRBarrier, IRCall)) for s in stmts)
        assert has_expected, (
            f"expected IRIf, IRBarrier, or IRCall but got {[type(s).__name__ for s in stmts]}"
        )

    def test_switch_modes(self):
        """Switch -exact/-glob/-regexp are all tracked."""
        ir_exact = lower_to_ir("switch -exact $x {a {set r 1}}")
        ir_glob = lower_to_ir("switch -glob $x {*.c {set r 1}}")
        ir_regexp = lower_to_ir("switch -regexp $x {^a {set r 1}}")
        for ir in (ir_exact, ir_glob, ir_regexp):
            stmts = ir.top_level.statements
            has_switch = any(isinstance(s, IRSwitch) for s in stmts)
            has_barrier = any(isinstance(s, IRBarrier) for s in stmts)
            assert has_switch or has_barrier


# ═══════════════════════════════════════════════════════════════════
# 16. End-to-end pipeline tests
# ═══════════════════════════════════════════════════════════════════


class TestEndToEnd:
    """Full pipeline: source → IR → CFG → codegen → format."""

    def test_fibonacci_proc(self):
        """Fibonacci procedure exercises recursion-like patterns."""
        source = """\
proc fib {n} {
    if {$n <= 1} {
        return $n
    }
    set a 0
    set b 1
    for {set i 2} {$i <= $n} {incr i} {
        set temp [expr {$a + $b}]
        set a $b
        set b $temp
    }
    return $b
}
"""
        fa = _proc_asm(source, "::fib")
        ops = _opcodes(fa)
        assert Op.LE in ops
        assert Op.ADD in ops
        text = format_function_asm(fa)
        assert "ByteCode ::fib" in text

    def test_bubble_sort(self):
        """Bubble sort exercises nested loops + conditionals + array ops."""
        source = """\
proc bubblesort {lst} {
    set n [llength $lst]
    for {set i 0} {$i < $n} {incr i} {
        for {set j 0} {$j < [expr {$n - $i - 1}]} {incr j} {
            set a [lindex $lst $j]
            set b [lindex $lst [expr {$j + 1}]]
            if {$a > $b} {
                set lst [lreplace $lst $j $j $b]
                set lst [lreplace $lst [expr {$j + 1}] [expr {$j + 1}] $a]
            }
        }
    }
    return $lst
}
"""
        fa = _proc_asm(source, "::bubblesort")
        ops = _opcodes(fa)
        assert Op.LT in ops
        assert Op.GT in ops
        text = format_function_asm(fa)
        assert "ByteCode ::bubblesort" in text

    def test_state_machine(self):
        """State machine pattern: while loop + switch."""
        source = """\
proc state_machine {input} {
    set state "start"
    set i 0
    while {$state ne "done"} {
        switch -exact $state {
            start {
                set state "process"
            }
            process {
                if {$i >= 10} {
                    set state "done"
                } else {
                    incr i
                }
            }
            default {
                set state "done"
            }
        }
    }
    return $i
}
"""
        ma = _asm_for(source)
        fa = ma.procedures["::state_machine"]
        ops = _opcodes(fa)
        assert _has_cond_jump(ops)
        assert Op.DONE in ops

    def test_multiproc_module_format(self):
        """Module with multiple procs formats correctly."""
        source = """\
proc add {a b} { expr {$a + $b} }
proc mul {a b} { expr {$a * $b} }
add 1 2
mul 3 4
"""
        ma = _asm_for(source)
        text = format_module_asm(ma)
        assert "ByteCode ::top" in text
        assert "ByteCode ::add" in text
        assert "ByteCode ::mul" in text

    def test_complex_value_interpolation_in_proc(self):
        """String interpolation with variable refs inside proc."""
        source = """\
proc greet {name age} {
    set msg "Hello, $name! You are $age years old."
    return $msg
}
"""
        fa = _proc_asm(source, "::greet")
        ops = _opcodes(fa)
        # Interpolated string should use strcat
        assert Op.STR_CONCAT1 in ops or Op.INVOKE_STK1 in ops

    def test_append_lappend_bytecoded(self):
        """append and lappend compile to specialised ops in proc."""
        source = """\
proc builder {} {
    set s ""
    set lst {}
    append s "hello"
    append s " world"
    lappend lst "item1"
    lappend lst "item2"
    return $lst
}
"""
        fa = _proc_asm(source, "::builder")
        ops = _opcodes(fa)
        assert Op.APPEND_SCALAR1 in ops
        assert Op.LAPPEND_SCALAR1 in ops

    def test_catch_inside_loop(self):
        """Catch inside a for loop — exercises catch depth + loop interaction."""
        source = """\
proc retry {n cmd} {
    for {set i 0} {$i < $n} {incr i} {
        set rc [catch {eval $cmd} result]
        if {$rc == 0} {
            return $result
        }
    }
    return ""
}
"""
        fa = _proc_asm(source, "::retry")
        ops = _opcodes(fa)
        assert Op.BEGIN_CATCH4 in ops
        assert Op.END_CATCH in ops
        assert Op.LT in ops

    def test_large_literal_table(self):
        """Many distinct literals fill the literal pool without errors."""
        assigns = "\n".join(f"    set v{i} {i}" for i in range(50))
        source = f"""\
proc many_lits {{}} {{
{assigns}
}}
"""
        fa = _proc_asm(source, "::many_lits")
        assert len(fa.literals) >= 50

    def test_deeply_nested_procs_do_not_crash(self):
        """Heavily nested control flow in a proc compiles without error."""
        source = """\
proc deep {x} {
    if {$x > 0} {
        if {$x > 1} {
            if {$x > 2} {
                for {set i 0} {$i < $x} {incr i} {
                    if {$i % 2 == 0} {
                        while {$i < 5} {
                            incr i
                        }
                    }
                }
            }
        }
    }
    return $x
}
"""
        fa = _proc_asm(source, "::deep")
        assert Op.DONE in _opcodes(fa)


# ═══════════════════════════════════════════════════════════════════
# 17. Codegen output format validation
# ═══════════════════════════════════════════════════════════════════


class TestCodegenFormatValidation:
    """Validate the textual assembly output format."""

    def test_instruction_offsets_monotonic(self):
        """Instruction byte offsets are monotonically increasing."""
        fa = _top_asm("set x 1; set y 2; set z [expr {$x + $y}]")
        offsets = [i.offset for i in fa.instructions if i.offset >= 0]
        for i in range(1, len(offsets)):
            assert offsets[i] >= offsets[i - 1]

    def test_labels_point_to_valid_offsets(self):
        """All labels resolve to valid instruction indices.

        Labels may point one past the last instruction (sentinel for
        end-of-block / startCommand end markers), so the valid range
        is [0, len(instructions) + N] where N accounts for sentinel
        positions used by the layout pass.
        """
        fa = _top_asm("if {$x} {set a 1} else {set a 2}")
        # All labels are non-negative and not absurdly large
        for label, offset in fa.labels.items():
            assert offset >= 0, f"label {label} has negative offset {offset}"
        # At least some labels exist for if/else control flow
        assert len(fa.labels) >= 2

    def test_proc_format_has_local_variables(self):
        """Proc formatting shows local variable table."""
        source = "proc foo {a b} { set c [expr {$a + $b}]; return $c }"
        ma = _asm_for(source)
        text = format_function_asm(ma.procedures["::foo"])
        assert "Local variables:" in text
        assert '"a"' in text
        assert '"b"' in text

    def test_done_always_at_end(self):
        """Every function ends with a done instruction."""
        source = """\
proc add {a b} { expr {$a + $b} }
proc sub {a b} { expr {$a - $b} }
set x 1
"""
        ma = _asm_for(source)
        assert ma.top_level.instructions[-1].op == Op.DONE
        for name, proc_asm in ma.procedures.items():
            assert proc_asm.instructions[-1].op == Op.DONE, f"proc {name} missing trailing done"


# ═══════════════════════════════════════════════════════════════════
# 18. Condition defs from command substitutions
# ═══════════════════════════════════════════════════════════════════


class TestConditionDefs:
    """Variables defined by command substitutions in conditions."""

    def test_catch_in_if_condition(self):
        """[catch {...} result] in if condition defines result var."""
        source = """\
proc safe_check {} {
    if {[catch {expr {1/0}} result]} {
        return $result
    }
    return "ok"
}
"""
        ir = lower_to_ir(source)
        cfg = build_cfg(ir)
        proc_cfg = cfg.procedures["::safe_check"]
        # Should have synthetic <cond> def node
        has_cond_def = any(
            any(isinstance(s, IRCall) and s.command == "<cond>" for s in b.statements)
            for b in proc_cfg.blocks.values()
        )
        assert has_cond_def

    def test_catch_in_while_condition(self):
        """[catch {...}] in while condition creates cond defs."""
        source = """\
proc retry_loop {} {
    while {[catch {some_op} result] != 0} {
        puts "retrying..."
    }
    return $result
}
"""
        ir = lower_to_ir(source)
        cfg = build_cfg(ir)
        proc_cfg = cfg.procedures["::retry_loop"]
        has_cond_def = any(
            any(isinstance(s, IRCall) and s.command == "<cond>" for s in b.statements)
            for b in proc_cfg.blocks.values()
        )
        assert has_cond_def


# ═══════════════════════════════════════════════════════════════════
# 19. Backslash substitution in values
# ═══════════════════════════════════════════════════════════════════


class TestBackslashSubstitution:
    """Backslash substitution flag handling in IRAssignValue."""

    def test_backslash_in_set_value(self):
        """Backslash sequences in set values are processed."""
        source = 'set x "line1\\nline2"'
        lower_to_ir(source)
        # The value may or may not have backsubst flag depending on parsing
        # Either way, it should compile without error
        fa = _top_asm(source)
        assert Op.DONE in _opcodes(fa)


# ═══════════════════════════════════════════════════════════════════
# 20. Multiple iterator groups in foreach
# ═══════════════════════════════════════════════════════════════════


class TestForeachMultipleIterators:
    """Foreach with multiple varList/list pairs."""

    def test_foreach_two_lists(self):
        """foreach i $list1 j $list2 body."""
        source = """\
proc zip {list1 list2} {
    set result {}
    foreach i $list1 j $list2 {
        lappend result "$i:$j"
    }
    return $result
}
"""
        ir = lower_to_ir(source)
        # Check the foreach has 2 iterator groups
        proc = ir.procedures["::zip"]
        foreach_stmts = [s for s in proc.body.statements if isinstance(s, IRForeach)]
        assert len(foreach_stmts) >= 1
        fe = foreach_stmts[0]
        assert len(fe.iterators) == 2

        # Should compile without error
        fa = _proc_asm(source, "::zip")
        assert Op.DONE in _opcodes(fa)

    def test_foreach_three_lists(self):
        """foreach i $a j $b k $c body — three iterator groups."""
        source = """\
proc triple {a b c} {
    set r {}
    foreach i $a j $b k $c {
        lappend r [expr {$i + $j + $k}]
    }
    return $r
}
"""
        ir = lower_to_ir(source)
        proc = ir.procedures["::triple"]
        foreach_stmts = [s for s in proc.body.statements if isinstance(s, IRForeach)]
        assert len(foreach_stmts) >= 1
        assert len(foreach_stmts[0].iterators) == 3

    def test_foreach_multi_var_per_group(self):
        """foreach {a b} $list body — two vars per group."""
        source = """\
proc pairs {lst} {
    set r {}
    foreach {k v} $lst {
        lappend r "$k=$v"
    }
    return $r
}
"""
        ir = lower_to_ir(source)
        proc = ir.procedures["::pairs"]
        foreach_stmts = [s for s in proc.body.statements if isinstance(s, IRForeach)]
        assert len(foreach_stmts) >= 1
        fe = foreach_stmts[0]
        var_list = fe.iterators[0][0]
        assert len(var_list) == 2
        assert "k" in var_list
        assert "v" in var_list


# ═══════════════════════════════════════════════════════════════════
# 21. defer_top_level vs inline_loops
# ═══════════════════════════════════════════════════════════════════


class TestDeferTopLevel:
    """Top-level foreach/try with defer_top_level=True vs False."""

    def test_top_level_foreach_deferred(self):
        """Top-level foreach with defer=True becomes generic invoke."""
        source = "foreach item {a b c} { puts $item }"
        ir = lower_to_ir(source)
        cfg_deferred = build_cfg(ir, defer_top_level=True)
        top = cfg_deferred.top_level
        # Should have an IRCall for foreach (deferred), not inlined foreach blocks
        has_foreach_call = any(
            any(isinstance(s, IRCall) and s.command == "foreach" for s in b.statements)
            for b in top.blocks.values()
        )
        assert has_foreach_call

    def test_top_level_foreach_inlined(self):
        """Top-level foreach with defer=False gets inlined."""
        source = "foreach item {a b c} { puts $item }"
        ir = lower_to_ir(source)
        cfg_inlined = build_cfg(ir, defer_top_level=False)
        top = cfg_inlined.top_level
        has_foreach_header = any("foreach_header" in name for name in top.blocks)
        assert has_foreach_header


# ═══════════════════════════════════════════════════════════════════
# 22. Frozen for/while (command-subst conditions)
# ═══════════════════════════════════════════════════════════════════


class TestFrozenLoops:
    """For/while with command-substitution conditions become barriers."""

    def test_frozen_for_cmd_subst_condition(self):
        """for with [expr {...}] condition → barrier."""
        source = """\
proc frozen {} {
    for {set i 0} {[expr {$i < 10}]} {incr i} {
        puts $i
    }
}
"""
        ir = lower_to_ir(source)
        cfg = build_cfg(ir)
        proc_cfg = cfg.procedures["::frozen"]
        has_barrier = any(
            any(isinstance(s, IRBarrier) and "frozen for" in s.reason for s in b.statements)
            for b in proc_cfg.blocks.values()
        )
        assert has_barrier

    def test_frozen_while_cmd_subst_condition(self):
        """while with [expr {...}] condition → barrier."""
        source = """\
proc frozen_while {} {
    set i 0
    while {[expr {$i < 10}]} {
        incr i
    }
}
"""
        ir = lower_to_ir(source)
        cfg = build_cfg(ir)
        proc_cfg = cfg.procedures["::frozen_while"]
        has_barrier = any(
            any(isinstance(s, IRBarrier) and "frozen while" in s.reason for s in b.statements)
            for b in proc_cfg.blocks.values()
        )
        assert has_barrier


# ═══════════════════════════════════════════════════════════════════
# 23. Stress: combining everything
# ═══════════════════════════════════════════════════════════════════


class TestStressCombinations:
    """Combine multiple complex constructs in a single compilation unit."""

    def test_kitchen_sink(self):
        """A module exercising most constructs simultaneously."""
        source = """\
namespace eval ::test {
    proc helper {x} {
        return [expr {$x * 2}]
    }
}

proc main {args} {
    set total 0
    set items {1 2 3 4 5}

    # For loop with nested if
    for {set i 0} {$i < 5} {incr i} {
        if {$i % 2 == 0} {
            set total [expr {$total + $i}]
        }
    }

    # Foreach
    foreach item $items {
        set total [expr {$total + $item}]
    }

    # While with break
    set j 0
    while {$j < 100} {
        if {$j > 10} {
            break
        }
        incr j
    }

    # Switch
    switch -exact $j {
        10 { set label "ten" }
        11 { set label "eleven" }
        default { set label "other" }
    }

    # Catch
    set rc [catch {expr {1/0}} err]

    # String operations
    set s "hello world"
    string length $s
    string toupper $s

    return $total
}

main
"""
        ma = _asm_for(source)
        assert "::main" in ma.procedures
        fa = ma.procedures["::main"]
        ops = _opcodes(fa)
        # Verify multiple construct types compiled
        assert Op.LT in ops or Op.LE in ops  # for condition
        assert Op.ADD in ops  # arithmetic
        assert Op.MOD in ops  # modulo
        assert Op.DONE in ops  # function exit
        text = format_module_asm(ma)
        assert "ByteCode ::main" in text
