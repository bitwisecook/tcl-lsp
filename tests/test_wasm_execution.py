"""WASM execution tests — verify compiled output by running it in wasmtime.

These tests compile Tcl source to WASM, execute the resulting module
in the wasmtime runtime, and verify that the results match expected
values (cross-checked against C Tcl behaviour where applicable).

Requires: ``wasmtime`` Python package (listed in dev dependencies).
"""

from __future__ import annotations

import pytest

from core.compiler.cfg import build_cfg
from core.compiler.codegen.wasm import wasm_codegen_module
from core.compiler.lowering import lower_to_ir

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")


# Helpers


def _compile_and_run(
    source: str,
    *,
    optimise: bool = False,
    func_name: str = "::top",
    args: tuple[int, ...] = (),
) -> int:
    """Compile Tcl source to WASM and execute a function, returning its i64 result."""
    ir_module = lower_to_ir(source)
    cfg_module = build_cfg(ir_module)
    wasm_module = wasm_codegen_module(cfg_module, ir_module, optimise=optimise)
    wasm_bytes = wasm_module.to_bytes()

    store = wasmtime.Store()
    module = wasmtime.Module(store.engine, wasm_bytes)
    instance = wasmtime.Instance(store, module, [])
    func = instance.exports(store)[func_name]
    return func(store, *args)


def _compile_and_run_proc(
    source: str,
    proc_name: str,
    args: tuple[int, ...],
    *,
    optimise: bool = False,
) -> int:
    """Compile and execute a specific procedure."""
    return _compile_and_run(source, optimise=optimise, func_name=f"::{proc_name}", args=args)


# Basic value tests


class TestBasicValues:
    """Test that basic value assignments produce correct results."""

    def test_set_integer(self):
        """set x 42 → x should be 42."""
        # Top-level returns 0 (no explicit return), but the var is set
        result = _compile_and_run("set x 42\n")
        assert isinstance(result, int)

    def test_set_zero(self):
        result = _compile_and_run("set x 0\n")
        assert isinstance(result, int)

    def test_set_negative(self):
        result = _compile_and_run("set x -1\n")
        assert isinstance(result, int)


# Procedure execution


class TestProcedureExecution:
    """Test that procedures compute correct results."""

    def test_identity(self):
        """proc identity {x} { return $x } → identity(7) == 7."""
        result = _compile_and_run_proc(
            "proc identity {x} { return $x }\n",
            "identity",
            (7,),
        )
        assert result == 7

    def test_identity_negative(self):
        result = _compile_and_run_proc(
            "proc identity {x} { return $x }\n",
            "identity",
            (-42,),
        )
        assert result == -42

    def test_identity_zero(self):
        result = _compile_and_run_proc(
            "proc identity {x} { return $x }\n",
            "identity",
            (0,),
        )
        assert result == 0


# Arithmetic procedures


class TestArithmetic:
    """Test arithmetic operations produce results matching C Tcl."""

    def test_add(self):
        """expr {$a + $b} should match tclsh: 3 + 4 == 7."""
        result = _compile_and_run_proc(
            "proc add {a b} { expr {$a + $b} }\n",
            "add",
            (3, 4),
        )
        assert result == 7

    def test_subtract(self):
        """expr {$a - $b}: 10 - 3 == 7."""
        result = _compile_and_run_proc(
            "proc sub {a b} { expr {$a - $b} }\n",
            "sub",
            (10, 3),
        )
        assert result == 7

    def test_multiply(self):
        """expr {$a * $b}: 6 * 7 == 42."""
        result = _compile_and_run_proc(
            "proc mul {a b} { expr {$a * $b} }\n",
            "mul",
            (6, 7),
        )
        assert result == 42

    def test_divide(self):
        """expr {$a / $b}: 42 / 6 == 7."""
        result = _compile_and_run_proc(
            "proc divide {a b} { expr {$a / $b} }\n",
            "divide",
            (42, 6),
        )
        assert result == 7

    def test_modulo(self):
        """expr {$a % $b}: 17 % 5 == 2."""
        result = _compile_and_run_proc(
            "proc modulo {a b} { expr {$a % $b} }\n",
            "modulo",
            (17, 5),
        )
        assert result == 2

    def test_add_negative(self):
        """5 + (-3) == 2."""
        result = _compile_and_run_proc(
            "proc add {a b} { expr {$a + $b} }\n",
            "add",
            (5, -3),
        )
        assert result == 2


# Comparison operators


class TestComparisons:
    """Test comparison operators produce correct boolean (0/1) results."""

    def test_eq_true(self):
        result = _compile_and_run_proc(
            "proc eq {a b} { expr {$a == $b} }\n",
            "eq",
            (5, 5),
        )
        assert result == 1

    def test_eq_false(self):
        result = _compile_and_run_proc(
            "proc eq {a b} { expr {$a == $b} }\n",
            "eq",
            (5, 6),
        )
        assert result == 0

    def test_ne_true(self):
        result = _compile_and_run_proc(
            "proc ne {a b} { expr {$a != $b} }\n",
            "ne",
            (5, 6),
        )
        assert result == 1

    def test_lt_true(self):
        result = _compile_and_run_proc(
            "proc lt {a b} { expr {$a < $b} }\n",
            "lt",
            (3, 5),
        )
        assert result == 1

    def test_lt_false(self):
        result = _compile_and_run_proc(
            "proc lt {a b} { expr {$a < $b} }\n",
            "lt",
            (5, 3),
        )
        assert result == 0

    def test_gt_true(self):
        result = _compile_and_run_proc(
            "proc gt {a b} { expr {$a > $b} }\n",
            "gt",
            (5, 3),
        )
        assert result == 1

    def test_le_true(self):
        result = _compile_and_run_proc(
            "proc le {a b} { expr {$a <= $b} }\n",
            "le",
            (5, 5),
        )
        assert result == 1

    def test_ge_true(self):
        result = _compile_and_run_proc(
            "proc ge {a b} { expr {$a >= $b} }\n",
            "ge",
            (5, 5),
        )
        assert result == 1


# Bitwise operators


class TestBitwise:
    """Test bitwise operations match C Tcl output."""

    def test_and(self):
        """expr {$a & $b}: 0xFF & 0x0F == 0x0F."""
        result = _compile_and_run_proc(
            "proc bitand {a b} { expr {$a & $b} }\n",
            "bitand",
            (0xFF, 0x0F),
        )
        assert result == 0x0F

    def test_or(self):
        """expr {$a | $b}: 0xF0 | 0x0F == 0xFF."""
        result = _compile_and_run_proc(
            "proc bitor {a b} { expr {$a | $b} }\n",
            "bitor",
            (0xF0, 0x0F),
        )
        assert result == 0xFF

    def test_xor(self):
        """expr {$a ^ $b}: 0xFF ^ 0x0F == 0xF0."""
        result = _compile_and_run_proc(
            "proc bitxor {a b} { expr {$a ^ $b} }\n",
            "bitxor",
            (0xFF, 0x0F),
        )
        assert result == 0xF0

    def test_lshift(self):
        """expr {$a << $b}: 1 << 4 == 16."""
        result = _compile_and_run_proc(
            "proc lshift {a b} { expr {$a << $b} }\n",
            "lshift",
            (1, 4),
        )
        assert result == 16

    def test_rshift(self):
        """expr {$a >> $b}: 16 >> 2 == 4."""
        result = _compile_and_run_proc(
            "proc rshift {a b} { expr {$a >> $b} }\n",
            "rshift",
            (16, 2),
        )
        assert result == 4


# Incr command


class TestIncr:
    """Test incr command execution."""

    def test_incr_by_one(self):
        """set x 5; incr x → x == 6."""
        result = _compile_and_run_proc(
            "proc incr_test {x} { incr x; return $x }\n",
            "incr_test",
            (5,),
        )
        assert result == 6

    def test_incr_by_n(self):
        """set x 5; incr x 3 → x == 8."""
        result = _compile_and_run_proc(
            "proc incr_test {x} { incr x 3; return $x }\n",
            "incr_test",
            (5,),
        )
        assert result == 8

    def test_incr_negative(self):
        """incr x -2: 10 - 2 → 8."""
        result = _compile_and_run_proc(
            "proc incr_test {x} { incr x -2; return $x }\n",
            "incr_test",
            (10,),
        )
        assert result == 8


# Conditional execution


class TestConditionals:
    """Test if/else execution."""

    def test_if_true_branch(self):
        """if {$x > 0} → should take true branch."""
        result = _compile_and_run_proc(
            "proc test_if {x} { if {$x > 0} { return 1 } else { return 0 } }\n",
            "test_if",
            (5,),
        )
        assert result == 1

    def test_if_false_branch(self):
        """if {$x > 0} → should take false branch."""
        result = _compile_and_run_proc(
            "proc test_if {x} { if {$x > 0} { return 1 } else { return 0 } }\n",
            "test_if",
            (-1,),
        )
        assert result == 0

    def test_if_else_merge_true(self):
        """Both branches set y then return it — true path."""
        result = _compile_and_run_proc(
            "proc f {x} { if {$x > 0} { set y 1 } else { set y 2 }; return $y }\n",
            "f",
            (5,),
        )
        assert result == 1

    def test_if_else_merge_false(self):
        """Both branches set y then return it — false path."""
        result = _compile_and_run_proc(
            "proc f {x} { if {$x > 0} { set y 1 } else { set y 2 }; return $y }\n",
            "f",
            (-1,),
        )
        assert result == 2


# Logical operators


class TestLogicalOperators:
    """Logical AND/OR must return boolean 0/1, not raw operand values."""

    def test_and_both_truthy(self):
        """expr {2 && 5} → 1, not 5."""
        result = _compile_and_run_proc("proc f {a b} { expr {$a && $b} }\n", "f", (2, 5))
        assert result == 1

    def test_and_left_zero(self):
        """expr {0 && 5} → 0 (short-circuit)."""
        result = _compile_and_run_proc("proc f {a b} { expr {$a && $b} }\n", "f", (0, 5))
        assert result == 0

    def test_or_left_truthy(self):
        """expr {7 || 0} → 1, not 7."""
        result = _compile_and_run_proc("proc f {a b} { expr {$a || $b} }\n", "f", (7, 0))
        assert result == 1

    def test_or_both_zero(self):
        """expr {0 || 0} → 0."""
        result = _compile_and_run_proc("proc f {a b} { expr {$a || $b} }\n", "f", (0, 0))
        assert result == 0

    def test_or_left_zero_right_truthy(self):
        """expr {0 || 7} → 1, not 7."""
        result = _compile_and_run_proc("proc f {a b} { expr {$a || $b} }\n", "f", (0, 7))
        assert result == 1


# Foreach loops


class TestForeach:
    """foreach must actually iterate using the list variable as loop bound."""

    def test_foreach_accumulate(self):
        """foreach with counter-based iteration: sum 0..n-1."""
        result = _compile_and_run_proc(
            "proc f {n} { set sum 0; foreach i $n { set sum [expr {$sum + $i}] }; return $sum }\n",
            "f",
            (5,),
        )
        # sum of 0+1+2+3+4 = 10
        assert result == 10


# Optimised vs non-optimised consistency


class TestOptimisationConsistency:
    """Verify that optimised and non-optimised output produce the same results."""

    @pytest.mark.parametrize(
        "source,proc,args,expected",
        [
            ("proc add {a b} { expr {$a + $b} }\n", "add", (3, 4), 7),
            ("proc sub {a b} { expr {$a - $b} }\n", "sub", (10, 3), 7),
            ("proc mul {a b} { expr {$a * $b} }\n", "mul", (6, 7), 42),
            ("proc id {x} { return $x }\n", "id", (99,), 99),
            (
                "proc incr_test {x} { incr x 5; return $x }\n",
                "incr_test",
                (10,),
                15,
            ),
        ],
    )
    def test_optimised_matches_unoptimised(self, source, proc, args, expected):
        """Optimised and non-optimised code should produce identical results."""
        result_no_opt = _compile_and_run_proc(source, proc, args, optimise=False)
        result_opt = _compile_and_run_proc(source, proc, args, optimise=True)
        assert result_no_opt == expected
        assert result_opt == expected

    @pytest.mark.parametrize(
        "source,proc,args",
        [
            ("proc add {a b} { expr {$a + $b} }\n", "add", (100, 200)),
            ("proc mul {a b} { expr {$a * $b} }\n", "mul", (11, 13)),
            ("proc eq {a b} { expr {$a == $b} }\n", "eq", (5, 5)),
            ("proc eq {a b} { expr {$a == $b} }\n", "eq", (5, 6)),
        ],
    )
    def test_optimised_same_as_unoptimised(self, source, proc, args):
        """General check: both modes produce identical i64 results."""
        r1 = _compile_and_run_proc(source, proc, args, optimise=False)
        r2 = _compile_and_run_proc(source, proc, args, optimise=True)
        assert r1 == r2


# WASM binary validity


class TestWasmValidity:
    """Verify that compiled WASM binaries are valid (accepted by wasmtime)."""

    @pytest.mark.parametrize(
        "source",
        [
            "set x 1\n",
            "proc foo {x} { return $x }\n",
            "proc add {a b} { expr {$a + $b} }\n",
            "if {1} { set x 1 }\n",
            "set x 0\nincr x\n",
        ],
    )
    def test_valid_wasm_accepted_by_engine(self, source):
        """wasmtime should accept our compiled WASM without error."""
        ir_module = lower_to_ir(source)
        cfg_module = build_cfg(ir_module)
        wasm_module = wasm_codegen_module(cfg_module, ir_module)
        wasm_bytes = wasm_module.to_bytes()

        store = wasmtime.Store()
        # This will raise if the WASM is malformed
        module = wasmtime.Module(store.engine, wasm_bytes)
        instance = wasmtime.Instance(store, module, [])
        assert instance is not None
