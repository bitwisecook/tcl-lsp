"""Tests for the bytecode assembly codegen backend."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.compiler.cfg import build_cfg
from core.compiler.codegen import (
    FunctionAsm,
    LiteralTable,
    LocalVarTable,
    ModuleAsm,
    Op,
    codegen_function,
    codegen_module,
    format_function_asm,
    format_module_asm,
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


# LiteralTable


class TestLiteralTable:
    def test_intern_new(self):
        lt = LiteralTable()
        assert lt.intern("hello") == 0
        assert lt.intern("world") == 1
        assert len(lt) == 2

    def test_intern_dedup(self):
        lt = LiteralTable()
        assert lt.intern("x") == 0
        assert lt.intern("x") == 0
        assert len(lt) == 1

    def test_entries(self):
        lt = LiteralTable()
        lt.intern("a")
        lt.intern("b")
        assert lt.entries() == ["a", "b"]


# LocalVarTable


class TestLocalVarTable:
    def test_params_first(self):
        lvt = LocalVarTable(("x", "y"))
        assert lvt.intern("x") == 0
        assert lvt.intern("y") == 1
        assert lvt.intern("z") == 2
        assert len(lvt) == 3

    def test_entries(self):
        lvt = LocalVarTable(("a",))
        lvt.intern("b")
        assert lvt.entries() == ["a", "b"]


# Simple codegen


class TestSimpleCodegen:
    def test_set_const(self):
        fa = _top_asm("set x 1")
        ops = _opcodes(fa)
        assert Op.PUSH1 in ops
        # Top-level uses stack-based ops; procs use STORE_SCALAR1
        assert Op.STORE_STK in ops
        # No trailing pop: last command's result stays on TOS for done
        assert ops[-1] == Op.DONE

    def test_puts(self):
        fa = _top_asm("puts hello")
        ops = _opcodes(fa)
        assert Op.INVOKE_STK1 in ops
        assert fa.literals.entries()[0] == "puts" or "puts" in fa.literals.entries()

    def test_incr_default(self):
        fa = _top_asm("set i 0; incr i")
        ops = _opcodes(fa)
        # Top-level uses stack-based incr
        assert Op.INCR_STK_IMM in ops

    def test_incr_immediate(self):
        fa = _top_asm("set i 0; incr i 5")
        ops = _opcodes(fa)
        # Top-level uses stack-based incr
        assert Op.INCR_STK_IMM in ops

    def test_return(self):
        ir = lower_to_ir("proc foo {} { return 42 }")
        cfg = build_cfg(ir)
        module = codegen_module(cfg, ir)
        assert "::foo" in module.procedures
        proc_asm = module.procedures["::foo"]
        ops = _opcodes(proc_asm)
        # Tail-position return is folded to done (matching tclsh).
        assert Op.DONE in ops

    def test_done_at_end(self):
        fa = _top_asm("set x 1")
        assert fa.instructions[-1].op == Op.DONE


# Expression compilation


class TestExprCodegen:
    def test_binary_add(self):
        fa = _top_asm("set x 1; set r [expr {$x + 2}]")
        ops = _opcodes(fa)
        assert Op.ADD in ops

    def test_binary_eq(self):
        fa = _top_asm("set x 1; set r [expr {$x == 2}]")
        ops = _opcodes(fa)
        assert Op.EQ in ops

    def test_binary_lt(self):
        fa = _top_asm("set x 1; set r [expr {$x < 2}]")
        ops = _opcodes(fa)
        assert Op.LT in ops

    def test_constant_fold(self):
        """All-constant expressions are folded at compile time."""
        fa = _top_asm("set r [expr {1 + 2}]")
        ops = _opcodes(fa)
        assert Op.ADD not in ops  # folded away
        lits = fa.literals.entries()
        assert "3" in lits  # result of constant fold

    def test_unary_neg(self):
        fa = _top_asm("set r [expr {-1}]")
        ops = _opcodes(fa)
        assert Op.UMINUS not in ops  # constant folded
        lits = fa.literals.entries()
        assert "-1" in lits

    def test_var_load(self):
        fa = _top_asm("set x 1; set r [expr {$x + 1}]")
        ops = _opcodes(fa)
        # Top-level uses stack-based load
        assert Op.LOAD_STK in ops
        assert Op.ADD in ops

    def test_expr_math_func_inline(self):
        """Math function calls are compiled inline as invokeStk."""
        fa = _top_asm("set x 1; set r [expr {sin($x)}]")
        ops = _opcodes(fa)
        assert Op.INVOKE_STK1 in ops
        assert Op.TRY_CVT_TO_NUMERIC in ops

    def test_standalone_expr(self):
        """Standalone ``expr`` is compiled inline, not as invokeStk."""
        fa = _top_asm("set x 1; expr {$x + 2}")
        ops = _opcodes(fa)
        assert Op.ADD in ops
        # Top-level uses stack-based load
        assert Op.LOAD_STK in ops
        assert Op.INVOKE_STK1 not in ops

    def test_standalone_expr_constant_fold(self):
        """Standalone ``expr`` with constants is folded."""
        fa = _top_asm("expr {2 + 3}")
        ops = _opcodes(fa)
        assert Op.ADD not in ops
        lits = fa.literals.entries()
        assert "5" in lits


# iRules expression operators


class TestIRulesExprOps:
    """iRules engine adds extra expr operators that get dedicated opcodes."""

    def _emit_binop(self, binop):
        """Build a minimal CFG with a single branch using the given BinOp."""
        from core.compiler.cfg import CFGBlock, CFGFunction, CFGReturn
        from core.compiler.expr_ast import ExprBinary, ExprVar

        expr = ExprBinary(op=binop, left=ExprVar("$a", "a", 0, 2), right=ExprVar("$b", "b", 0, 2))
        blk = CFGBlock(
            name="entry",
            statements=(),
            terminator=CFGReturn(value=None),
        )
        _cfg = CFGFunction(name="test", entry="entry", blocks={"entry": blk})  # noqa: F841
        # Hack: inject an IRAssignExpr to exercise the expression emitter
        from core.analysis.semantic_model import Range
        from core.compiler.ir import IRAssignExpr
        from core.parsing.tokens import SourcePosition

        r = Range(SourcePosition(0, 0, 0), SourcePosition(0, 0, 0))
        stmt = IRAssignExpr(range=r, name="r", expr=expr)
        blk2 = CFGBlock(name="entry", statements=(stmt,), terminator=CFGReturn(value=None))
        cfg2 = CFGFunction(name="test", entry="entry", blocks={"entry": blk2})
        return codegen_function(cfg2)

    def _emit_unaryop(self, unaryop):
        from core.analysis.semantic_model import Range
        from core.compiler.cfg import CFGBlock, CFGFunction, CFGReturn
        from core.compiler.expr_ast import ExprUnary, ExprVar
        from core.compiler.ir import IRAssignExpr
        from core.parsing.tokens import SourcePosition

        expr = ExprUnary(op=unaryop, operand=ExprVar("$a", "a", 0, 2))
        r = Range(SourcePosition(0, 0, 0), SourcePosition(0, 0, 0))
        stmt = IRAssignExpr(range=r, name="r", expr=expr)
        blk = CFGBlock(name="entry", statements=(stmt,), terminator=CFGReturn(value=None))
        cfg = CFGFunction(name="test", entry="entry", blocks={"entry": blk})
        return codegen_function(cfg)

    def test_contains(self):
        from core.compiler.expr_ast import BinOp

        fa = self._emit_binop(BinOp.CONTAINS)
        assert Op.IRULE_CONTAINS in _opcodes(fa)

    def test_starts_with(self):
        from core.compiler.expr_ast import BinOp

        fa = self._emit_binop(BinOp.STARTS_WITH)
        assert Op.IRULE_STARTS_WITH in _opcodes(fa)

    def test_ends_with(self):
        from core.compiler.expr_ast import BinOp

        fa = self._emit_binop(BinOp.ENDS_WITH)
        assert Op.IRULE_ENDS_WITH in _opcodes(fa)

    def test_equals(self):
        from core.compiler.expr_ast import BinOp

        fa = self._emit_binop(BinOp.STR_EQUALS)
        assert Op.IRULE_EQUALS in _opcodes(fa)

    def test_matches_glob(self):
        from core.compiler.expr_ast import BinOp

        fa = self._emit_binop(BinOp.MATCHES_GLOB)
        assert Op.IRULE_MATCHES_GLOB in _opcodes(fa)

    def test_matches_regex(self):
        from core.compiler.expr_ast import BinOp

        fa = self._emit_binop(BinOp.MATCHES_REGEX)
        assert Op.IRULE_MATCHES_REGEX in _opcodes(fa)

    def test_word_and(self):
        from core.compiler.expr_ast import BinOp

        fa = self._emit_binop(BinOp.WORD_AND)
        assert Op.IRULE_WORD_AND in _opcodes(fa)

    def test_word_or(self):
        from core.compiler.expr_ast import BinOp

        fa = self._emit_binop(BinOp.WORD_OR)
        assert Op.IRULE_WORD_OR in _opcodes(fa)

    def test_word_not(self):
        from core.compiler.expr_ast import UnaryOp

        fa = self._emit_unaryop(UnaryOp.WORD_NOT)
        assert Op.IRULE_WORD_NOT in _opcodes(fa)


# CFG terminators


class TestTerminators:
    def test_if_branch(self):
        fa = _top_asm("if {$x} {set a 1} else {set a 2}")
        ops = _opcodes(fa)
        # Should have at least one conditional jump
        has_cond_jump = (
            Op.JUMP_TRUE4 in ops
            or Op.JUMP_FALSE4 in ops
            or Op.JUMP_TRUE1 in ops
            or Op.JUMP_FALSE1 in ops
        )
        assert has_cond_jump

    def test_fallthrough_elimination(self):
        """Goto to the very next block should be omitted (no jump emitted)."""
        fa = _top_asm("set x 1")
        # A simple set shouldn't need any jumps at all
        ops = _opcodes(fa)
        jump_ops = [op for op in ops if op in (Op.JUMP1, Op.JUMP4)]
        assert len(jump_ops) == 0


# Bytecoded commands


class TestBytecodedCommands:
    def test_append(self):
        fa = _top_asm("set x hello; append x world")
        ops = _opcodes(fa)
        # Top-level uses stack-based append
        assert Op.APPEND_STK in ops

    def test_lappend(self):
        fa = _top_asm("set x {}; lappend x foo")
        ops = _opcodes(fa)
        # Top-level uses stack-based lappend
        assert Op.LAPPEND_STK in ops

    def test_string_length(self):
        fa = _top_asm('string length "hello"')
        ops = _opcodes(fa)
        assert Op.STR_LEN in ops

    def test_string_equal(self):
        fa = _top_asm('string equal "a" "b"')
        ops = _opcodes(fa)
        assert Op.STR_EQ in ops

    def test_string_compare(self):
        fa = _top_asm('string compare "a" "b"')
        ops = _opcodes(fa)
        assert Op.STR_CMP in ops


# Formatting


class TestFormatting:
    def test_header(self):
        text = _top_text("set x 1")
        assert text.startswith("ByteCode ::top")
        assert "instructions" in text
        assert "literals" in text
        assert "variables" in text

    def test_literals_section(self):
        text = _top_text("set x hello")
        assert "Literals:" in text
        assert '"hello"' in text

    def test_variables_section(self):
        # Top-level scripts use stack-based var access, so no LVT
        text = _top_text("set x 1")
        assert "0 variables" in text
        # Proc bodies still use LVT
        source = "proc foo {x} { set x 1 }"
        ma = _asm_for(source)
        proc_text = format_function_asm(ma.procedures["::foo"])
        assert "Local variables:" in proc_text
        assert '%v0: "x"' in proc_text

    def test_instruction_offsets(self):
        text = _top_text("set x 1")
        assert "(0) push1" in text

    def test_block_labels(self):
        text = _top_text("set x 1")
        assert "# " in text  # block label comment

    def test_module_format(self):
        source = "proc foo {x} { return $x }\nfoo 1"
        ma = _asm_for(source)
        text = format_module_asm(ma)
        assert "ByteCode ::top" in text
        assert "ByteCode ::foo" in text


# Procedures


class TestProcedures:
    def test_proc_params_in_lvt(self):
        ir = lower_to_ir("proc add {a b} { expr {$a + $b} }")
        cfg = build_cfg(ir)
        module = codegen_module(cfg, ir)
        proc = module.procedures["::add"]
        # params should be first slots
        entries = proc.lvt.entries()
        assert entries[0] == "a"
        assert entries[1] == "b"

    def test_proc_uses_invoke(self):
        ir = lower_to_ir("proc greet {name} { puts $name }")
        cfg = build_cfg(ir)
        module = codegen_module(cfg, ir)
        proc = module.procedures["::greet"]
        ops = _opcodes(proc)
        # Proc compiles puts $name → push "puts", push "$name", invokeStk
        assert Op.INVOKE_STK1 in ops
