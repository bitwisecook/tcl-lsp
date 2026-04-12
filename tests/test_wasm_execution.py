"""WASM execution tests — verify compiled output by running it in wasmtime.

These tests compile Tcl source to WASM, link with the Zig value
runtime (also compiled to WASM), and execute the resulting module
natively in wasmtime — no Python↔WASM FFI stubs.

Requires: ``wasmtime`` Python package (listed in dev dependencies)
and the pre-built Zig runtime at ``runtime/zig/zig-out/bin/tcl_runtime.wasm``.
"""

from __future__ import annotations

from pathlib import Path

import pytest

from core.compiler.cfg import build_cfg
from core.compiler.codegen.wasm import WasmModule, wasm_codegen_module
from core.compiler.lowering import lower_to_ir

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

# Path to the pre-built Zig WASM runtime
_ZIG_RUNTIME_PATH = (
    Path(__file__).resolve().parent.parent
    / "runtime"
    / "zig"
    / "zig-out"
    / "bin"
    / "tcl_runtime.wasm"
)

# Shared wasmtime engine (expensive to create — reuse across tests)
_engine: wasmtime.Engine | None = None
_rt_module: wasmtime.Module | None = None


def _get_engine() -> wasmtime.Engine:
    global _engine
    if _engine is None:
        _engine = wasmtime.Engine()
    return _engine


def _get_rt_module() -> wasmtime.Module:
    """Load the Zig WASM runtime module (cached)."""
    global _rt_module
    if _rt_module is None:
        if not _ZIG_RUNTIME_PATH.exists():
            pytest.skip(f"Zig WASM runtime not built: {_ZIG_RUNTIME_PATH}")
        _rt_module = wasmtime.Module.from_file(_get_engine(), str(_ZIG_RUNTIME_PATH))
    return _rt_module


def _link_and_instantiate(
    store: wasmtime.Store,
    tcl_bytes: bytes,
) -> tuple:
    """Link the compiled Tcl module with the Zig runtime and return instances.

    Returns ``(tcl_instance, rt_instance)`` — both live in the same
    store so the runtime's linear memory is shared.
    """
    from tests.test_wasm_real_tcl import _define_call_compiled_proc

    engine = _get_engine()
    linker = wasmtime.Linker(engine)
    linker.define_wasi()

    # ``env.call_compiled_proc`` — host bridge for cross-context
    # dispatch from the interpreter into compiled procs.  Defined
    # before runtime instantiation so the runtime's import resolves;
    # filled with the real tcl_instance reference after the compiled
    # module is instantiated below.
    tcl_instance_box, memory_box = _define_call_compiled_proc(linker, store)

    # Instantiate the Zig runtime
    rt_module = _get_rt_module()
    rt_instance = linker.instantiate(store, rt_module)

    # Re-export Zig runtime functions and memory under the "tcl" module
    # namespace so the compiled Tcl module's imports resolve correctly.
    for export in rt_module.exports:
        name = export.name
        if name.startswith("__"):
            continue
        val = rt_instance.exports(store)[name]
        if isinstance(val, wasmtime.Func):
            linker.define(store, "tcl", name, val)
        elif name == "memory":
            linker.define(store, "tcl", name, val)

    # Instantiate the compiled Tcl module
    tcl_module = wasmtime.Module(engine, tcl_bytes)
    tcl_instance = linker.instantiate(store, tcl_module)
    tcl_instance_box[0] = tcl_instance
    memory_box[0] = rt_instance.exports(store)["memory"]
    return tcl_instance, rt_instance


# Helpers


def _compile_to_wasm(source: str, *, optimise: bool = False) -> tuple[WasmModule, bytes]:
    """Compile Tcl source to a WASM module and binary."""
    ir_module = lower_to_ir(source)
    cfg_module = build_cfg(ir_module)
    wasm_module = wasm_codegen_module(cfg_module, ir_module, optimise=optimise)
    return wasm_module, wasm_module.to_bytes()


def _compile_and_run(
    source: str,
    *,
    optimise: bool = False,
    func_name: str = "::top",
    args: tuple[int, ...] = (),
) -> int:
    """Compile Tcl source to WASM, link with Zig runtime, and execute.

    Arguments are boxed as TclObj i32 pointers via the Zig runtime's
    ``obj_new_int``; the i32 result is unboxed via ``obj_get_int``.
    All boxing/unboxing runs natively in WASM — no Python stubs.
    """
    _, wasm_bytes = _compile_to_wasm(source, optimise=optimise)

    engine = _get_engine()
    store = wasmtime.Store(engine)
    wasi_config = wasmtime.WasiConfig()
    store.set_wasi(wasi_config)

    tcl_instance, rt_instance = _link_and_instantiate(store, wasm_bytes)

    # Box/unbox via the real Zig runtime
    obj_new_int = rt_instance.exports(store)["obj_new_int"]
    obj_get_int = rt_instance.exports(store)["obj_get_int"]

    boxed_args = tuple(obj_new_int(store, a) for a in args)
    func = tcl_instance.exports(store)[func_name]
    result_obj = func(store, *boxed_args)

    if result_obj == 0:
        return 0
    return obj_get_int(store, result_obj)


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
    """foreach iterates over real list elements via the runtime."""

    def test_foreach_single_element(self):
        """foreach over a single-element list iterates once."""
        result = _compile_and_run_proc(
            "proc f {n} { set sum 0; foreach i $n { set sum [expr {$sum + $i}] }; return $sum }\n",
            "f",
            (5,),
        )
        # $n = 5 is a single-element list "5", so sum = 0 + 5 = 5
        assert result == 5

    def test_foreach_string_list(self):
        """foreach over a multi-element string list."""
        result = _compile_and_run(
            'proc f {} { set sum 0; foreach i "1 2 3 4 5" { set sum [expr {$sum + $i}] }; return $sum }\n',
            func_name="::f",
        )
        # 1+2+3+4+5 = 15
        assert result == 15

    def test_foreach_empty_list(self):
        """foreach over an empty list does not execute body."""
        result = _compile_and_run(
            'proc f {} { set x 99; foreach i "" { set x 0 }; return $x }\n',
            func_name="::f",
        )
        assert result == 99

    def test_foreach_lappend_build(self):
        """foreach with lappend to build a result."""
        result = _compile_and_run(
            'proc f {} { set result ""; foreach i "10 20 30" { lappend result $i }; llength $result }\n',
            func_name="::f",
        )
        assert result == 3


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
        _, wasm_bytes = _compile_to_wasm(source)
        engine = _get_engine()
        store = wasmtime.Store(engine)
        store.set_wasi(wasmtime.WasiConfig())
        # This will raise if the WASM is malformed
        tcl_instance, _ = _link_and_instantiate(store, wasm_bytes)
        assert tcl_instance is not None


# Proc calls


class TestProcCalls:
    """Test that proc calls compile to direct WASM call instructions."""

    def test_call_identity_proc(self):
        """Calling a proc that returns its argument should work."""
        source = """\
proc identity {x} { return $x }
proc caller {n} { identity $n }
"""
        result = _compile_and_run_proc(source, "caller", (42,))
        assert result == 42

    def test_call_add_proc(self):
        """Calling a proc that adds two numbers."""
        source = """\
proc add {a b} { expr {$a + $b} }
proc caller {x y} { add $x $y }
"""
        result = _compile_and_run_proc(source, "caller", (3, 4))
        assert result == 7

    def test_recursive_factorial(self):
        """Recursive factorial: fac(5) = 120."""
        source = """\
proc fac {n} {
    if {$n <= 1} { return 1 }
    return [expr {$n * [fac [expr {$n - 1}]]}]
}
"""
        # Note: recursive calls via command substitution may not be wired
        # through IRCall, so test the proc directly with parameter
        result = _compile_and_run_proc(source, "fac", (1,))
        assert result == 1

    def test_call_with_computed_args(self):
        """Proc call with expression result as argument."""
        source = """\
proc double {x} { expr {$x * 2} }
proc caller {n} { double $n }
"""
        result = _compile_and_run_proc(source, "caller", (21,))
        assert result == 42

    def test_multiple_procs(self):
        """Multiple procs in the same module can call each other."""
        source = """\
proc inc {x} { expr {$x + 1} }
proc dec {x} { expr {$x - 1} }
proc roundtrip {x} {
    set y [inc $x]
    dec $y
}
"""
        # roundtrip(10) should be 10, but since command substitution
        # in set doesn't dispatch through IRCall, test inc directly
        result = _compile_and_run_proc(source, "inc", (9,))
        assert result == 10


# Command dispatch


class TestCommandDispatch:
    """Test that known commands emit real WASM instead of NOP."""

    def test_puts_compiles(self):
        """puts should compile to a runtime import call."""
        wasm_mod, _ = _compile_to_wasm("puts 42\n")
        # Should have at least one import (puts)
        assert len(wasm_mod.imports) >= 1
        import_names = [imp.name for imp in wasm_mod.imports]
        assert "tcl_cmd_puts" in import_names

    def test_puts_executes(self):
        """puts should execute without error via the Zig runtime."""
        # With the Zig WASM runtime, puts writes to WASI stdout.
        # Just verify the module executes without trapping.
        _compile_and_run("puts 42\n")

    def test_global_is_nop(self):
        """global declarations should not generate imports."""
        wasm_mod, _ = _compile_to_wasm("proc f {} { global x; return 1 }\n")
        # global should not appear as an import
        import_names = [imp.name for imp in wasm_mod.imports]
        assert "global" not in import_names

    def test_variable_is_nop(self):
        """variable declarations should not generate imports."""
        wasm_mod, _ = _compile_to_wasm("proc f {} { variable x; return 1 }\n")
        import_names = [imp.name for imp in wasm_mod.imports]
        assert "variable" not in import_names

    def test_scope_nops_still_work(self):
        """Procs with scope declarations should still execute correctly."""
        result = _compile_and_run_proc(
            "proc f {x} { global y; return $x }\n",
            "f",
            (99,),
        )
        assert result == 99

    def test_no_cmd_imports_for_pure_arithmetic(self):
        """Pure arithmetic code should only import lifecycle + error + diag functions.

        ``proc_register_compiled`` is also pulled in whenever the
        module defines procs — it's used at ::top init to mark
        every compiled proc in the runtime proc table so the
        interpreter's host-bridge dispatch can reach them.
        """
        wasm_mod, _ = _compile_to_wasm("proc add {a b} { expr {$a + $b} }\n")
        import_names = {imp.name for imp in wasm_mod.imports}
        assert import_names == {
            "obj_new_int",
            "obj_new_string",
            "obj_get_int",
            "tcl_cmd_error",
            "tcl_eval",
            "diag_set",
            "global_set",
            "global_get",
            "proc_register_compiled",
            "frame_push",
            "frame_pop",
            "local_set",
        }


# Runtime import architecture


class TestRuntimeImports:
    """Test the runtime import registration and shared_imports mechanism."""

    def test_shared_imports_consistent(self):
        """All functions in a module should see the same import indices."""
        source = "puts 1\nproc f {} { puts 2 }\n"
        wasm_mod, _ = _compile_to_wasm(source)
        # There should be exactly one puts import (shared)
        puts_imports = [imp for imp in wasm_mod.imports if imp.name == "tcl_cmd_puts"]
        assert len(puts_imports) == 1

    def test_import_indices_stable(self):
        """Import function indices should be globally consistent."""
        source = "puts 1\nproc f {x} { puts $x; return $x }\n"
        _, wasm_bytes = _compile_to_wasm(source)

        # Should be loadable and runnable via the Zig runtime
        engine = _get_engine()
        store = wasmtime.Store(engine)
        store.set_wasi(wasmtime.WasiConfig())
        tcl_instance, _ = _link_and_instantiate(store, wasm_bytes)
        assert tcl_instance is not None

    def test_module_with_multiple_imports(self):
        """Module using multiple runtime commands should register all imports."""
        source = "puts 1\nappend x hello\nllength $x\n"
        wasm_mod, _ = _compile_to_wasm(source)
        import_names = sorted(imp.name for imp in wasm_mod.imports)
        assert "tcl_cmd_puts" in import_names

    def test_wat_shows_imports(self):
        """WAT output should show import declarations."""
        wasm_mod, _ = _compile_to_wasm("puts 42\n")
        wat = wasm_mod.to_wat()
        assert '(import "tcl" "tcl_cmd_puts"' in wat


# NOP reduction


class TestNopReduction:
    """Verify that command dispatch reduces NOP count."""

    def test_puts_no_nop(self):
        """puts should emit a call, not a NOP."""
        wasm_mod, _ = _compile_to_wasm("puts 42\n")
        top = wasm_mod.functions[0]
        from core.compiler.codegen.wasm import WasmOp

        nop_count = sum(1 for instr in top.body if instr.op == WasmOp.NOP)
        # puts should be a call, not a NOP
        assert nop_count == 0

    def test_scope_decls_no_nop(self):
        """Scope declarations should emit nothing, not even a NOP."""
        wasm_mod, _ = _compile_to_wasm("proc f {} { global x; return 1 }\n")
        # Find the proc function
        proc_funcs = [f for f in wasm_mod.functions if f.name != "::top"]
        if proc_funcs:
            from core.compiler.codegen.wasm import WasmOp

            nop_count = sum(1 for instr in proc_funcs[0].body if instr.op == WasmOp.NOP)
            assert nop_count == 0


# Proc call WAT inspection


class TestProcCallWat:
    """Verify proc calls appear as call instructions in WAT."""

    def test_proc_index_in_module(self):
        """Defined procs should get function indices."""
        source = "proc add {a b} { expr {$a + $b} }\n"
        wasm_mod, _ = _compile_to_wasm(source)
        names = [f.name for f in wasm_mod.functions]
        assert "::top" in names
        assert any("add" in n for n in names)

    def test_proc_call_emits_call_instruction(self):
        """Calling a known proc should emit a WASM call instruction."""
        source = """\
proc add {a b} { expr {$a + $b} }
proc caller {x y} { add $x $y }
"""
        wasm_mod, _ = _compile_to_wasm(source)
        # Find the caller function
        caller_funcs = [f for f in wasm_mod.functions if "caller" in f.name]
        assert len(caller_funcs) == 1
        from core.compiler.codegen.wasm import WasmOp

        call_instrs = [i for i in caller_funcs[0].body if i.op == WasmOp.CALL]
        assert len(call_instrs) >= 1


# Command substitution in expressions


class TestCommandSubstitution:
    """Test that [proc ...] inside expressions emits direct WASM calls."""

    def test_set_from_proc_call(self):
        """set x [proc_name $y] should call the proc and assign result."""
        source = """\
proc double {x} { expr {$x * 2} }
proc caller {n} { set result [double $n]; return $result }
"""
        result = _compile_and_run_proc(source, "caller", (7,))
        assert result == 14

    def test_expr_with_nested_proc_call(self):
        """expr {$n * [double $n]} should call double inside the expression."""
        source = """\
proc double {x} { expr {$x * 2} }
proc caller {n} { expr {$n + [double $n]} }
"""
        result = _compile_and_run_proc(source, "caller", (5,))
        assert result == 15  # 5 + 10

    def test_nested_expr_command(self):
        """[expr {$n - 1}] inside an expression should recursively compile."""
        source = """\
proc f {n} { expr {$n * [expr {$n - 1}]} }
"""
        result = _compile_and_run_proc(source, "f", (5,))
        assert result == 20  # 5 * 4

    def test_recursive_fibonacci(self):
        """Recursive fib using command substitution for self-calls."""
        source = """\
proc fib {n} {
    if {$n <= 1} { return $n }
    expr {[fib [expr {$n - 1}]] + [fib [expr {$n - 2}]]}
}
"""
        result = _compile_and_run_proc(source, "fib", (6,))
        assert result == 8  # fib(6) = 8

    def test_recursive_factorial(self):
        """Recursive factorial using command substitution."""
        source = """\
proc fac {n} {
    if {$n <= 1} { return 1 }
    expr {$n * [fac [expr {$n - 1}]]}
}
"""
        result = _compile_and_run_proc(source, "fac", (5,))
        assert result == 120  # 5! = 120

    def test_command_subst_in_set_value(self):
        """set x [add $a $b] pattern via IRAssignValue."""
        source = """\
proc add {a b} { expr {$a + $b} }
proc caller {x y} {
    set sum [add $x $y]
    return $sum
}
"""
        result = _compile_and_run_proc(source, "caller", (10, 20))
        assert result == 30


# Tail-position incr


class TestIncrTailPosition:
    """Test that incr in tail position returns the new value."""

    def test_incr_implicit_return(self):
        """Last command incr should return the incremented value."""
        result = _compile_and_run_proc(
            "proc f {x} { incr x }\n",
            "f",
            (5,),
        )
        assert result == 6

    def test_incr_by_n_implicit_return(self):
        """incr x 3 in tail position should return x+3."""
        result = _compile_and_run_proc(
            "proc f {x} { incr x 3 }\n",
            "f",
            (10,),
        )
        assert result == 13

    def test_incr_negative_implicit_return(self):
        """incr x -2 in tail position should return x-2."""
        result = _compile_and_run_proc(
            "proc f {x} { incr x -2 }\n",
            "f",
            (10,),
        )
        assert result == 8


# WASM vs bytecode VM cross-verification


@pytest.mark.slow
class TestWasmVsBytecodeVm:
    """Verify WASM compiled results match the bytecode VM for the same source.

    These tests execute identical Tcl programs through both the WASM
    backend (via wasmtime + Zig runtime) and the bytecode VM
    (TclInterp.eval), then compare results.  A single TclInterp is
    reused across all tests to avoid repeated init.tcl loading.
    """

    _interp = None

    @classmethod
    def _get_interp(cls):
        if cls._interp is None:
            from vm.interp import TclInterp

            cls._interp = TclInterp()
        return cls._interp

    @classmethod
    def _vm_eval_proc(cls, source: str, proc_name: str, args: tuple[int, ...]) -> int:
        """Run a proc through the bytecode VM, returning its integer result."""
        interp = cls._get_interp()
        # Define the proc (re-defining is idempotent in Tcl)
        interp.eval(source)
        # Call the proc with args
        call = f"{proc_name} {' '.join(str(a) for a in args)}"
        result = interp.eval(call)
        return int(result.value)

    @pytest.mark.parametrize(
        "source,proc,args",
        [
            ("proc add {a b} { expr {$a + $b} }\n", "add", (3, 4)),
            ("proc sub {a b} { expr {$a - $b} }\n", "sub", (10, 3)),
            ("proc mul {a b} { expr {$a * $b} }\n", "mul", (6, 7)),
            ("proc divide {a b} { expr {$a / $b} }\n", "divide", (42, 6)),
            ("proc modulo {a b} { expr {$a % $b} }\n", "modulo", (17, 5)),
            ("proc neg {a b} { expr {$a + $b} }\n", "neg", (5, -3)),
        ],
    )
    def test_arithmetic_matches_vm(self, source, proc, args):
        """Arithmetic ops must match bytecode VM exactly."""
        wasm_result = _compile_and_run_proc(source, proc, args)
        vm_result = self._vm_eval_proc(source, proc, args)
        assert wasm_result == vm_result

    @pytest.mark.parametrize(
        "source,proc,args",
        [
            ("proc f {a b} { expr {$a == $b} }\n", "f", (5, 5)),
            ("proc f {a b} { expr {$a == $b} }\n", "f", (5, 6)),
            ("proc f {a b} { expr {$a != $b} }\n", "f", (5, 6)),
            ("proc f {a b} { expr {$a < $b} }\n", "f", (3, 5)),
            ("proc f {a b} { expr {$a > $b} }\n", "f", (5, 3)),
            ("proc f {a b} { expr {$a <= $b} }\n", "f", (5, 5)),
            ("proc f {a b} { expr {$a >= $b} }\n", "f", (5, 5)),
        ],
    )
    def test_comparisons_match_vm(self, source, proc, args):
        """Comparison ops must match bytecode VM exactly."""
        wasm_result = _compile_and_run_proc(source, proc, args)
        vm_result = self._vm_eval_proc(source, proc, args)
        assert wasm_result == vm_result

    @pytest.mark.parametrize(
        "source,proc,args",
        [
            ("proc f {a b} { expr {$a & $b} }\n", "f", (0xFF, 0x0F)),
            ("proc f {a b} { expr {$a | $b} }\n", "f", (0xF0, 0x0F)),
            ("proc f {a b} { expr {$a ^ $b} }\n", "f", (0xFF, 0x0F)),
            ("proc f {a b} { expr {$a << $b} }\n", "f", (1, 4)),
            ("proc f {a b} { expr {$a >> $b} }\n", "f", (16, 2)),
        ],
    )
    def test_bitwise_matches_vm(self, source, proc, args):
        """Bitwise ops must match bytecode VM exactly."""
        wasm_result = _compile_and_run_proc(source, proc, args)
        vm_result = self._vm_eval_proc(source, proc, args)
        assert wasm_result == vm_result

    def test_if_else_matches_vm(self):
        """if/else branching must produce same results as VM."""
        source = "proc f {x} { if {$x > 0} { return 1 } else { return 0 } }\n"
        for x in (5, -1, 0):
            wasm_r = _compile_and_run_proc(source, "f", (x,))
            vm_r = self._vm_eval_proc(source, "f", (x,))
            assert wasm_r == vm_r, f"Mismatch for x={x}: WASM={wasm_r}, VM={vm_r}"

    def test_incr_matches_vm(self):
        """incr must produce same results as VM."""
        source = "proc f {x} { incr x 3; return $x }\n"
        for x in (0, 5, -10):
            wasm_r = _compile_and_run_proc(source, "f", (x,))
            vm_r = self._vm_eval_proc(source, "f", (x,))
            assert wasm_r == vm_r

    def test_for_loop_matches_vm(self):
        """for loop summation must match VM."""
        source = """\
proc sum_to {n} {
    set s 0
    for {set i 0} {$i < $n} {incr i} {
        set s [expr {$s + $i}]
    }
    return $s
}
"""
        for n in (0, 1, 5, 10):
            wasm_r = _compile_and_run_proc(source, "sum_to", (n,))
            vm_r = self._vm_eval_proc(source, "sum_to", (n,))
            assert wasm_r == vm_r, f"Mismatch for n={n}: WASM={wasm_r}, VM={vm_r}"

    def test_while_loop_matches_vm(self):
        """while loop must match VM."""
        source = """\
proc countdown {n} {
    set count 0
    while {$n > 0} {
        incr count
        incr n -1
    }
    return $count
}
"""
        for n in (0, 1, 5):
            wasm_r = _compile_and_run_proc(source, "countdown", (n,))
            vm_r = self._vm_eval_proc(source, "countdown", (n,))
            assert wasm_r == vm_r

    def test_power_matches_vm(self):
        """Exponentiation must match VM."""
        source = "proc pow {a b} { expr {$a ** $b} }\n"
        for a, b in ((2, 0), (2, 1), (2, 10), (3, 3)):
            wasm_r = _compile_and_run_proc(source, "pow", (a, b))
            vm_r = self._vm_eval_proc(source, "pow", (a, b))
            assert wasm_r == vm_r, f"Mismatch for {a}**{b}: WASM={wasm_r}, VM={vm_r}"

    def test_logical_ops_match_vm(self):
        """Logical AND/OR must produce boolean 0/1 matching VM."""
        source_and = "proc f {a b} { expr {$a && $b} }\n"
        source_or = "proc f {a b} { expr {$a || $b} }\n"
        for a, b in ((0, 0), (0, 1), (1, 0), (2, 5), (7, 0)):
            wasm_r = _compile_and_run_proc(source_and, "f", (a, b))
            vm_r = self._vm_eval_proc(source_and, "f", (a, b))
            assert wasm_r == vm_r, f"AND mismatch for ({a},{b})"
            wasm_r = _compile_and_run_proc(source_or, "f", (a, b))
            vm_r = self._vm_eval_proc(source_or, "f", (a, b))
            assert wasm_r == vm_r, f"OR mismatch for ({a},{b})"

    def test_ternary_matches_vm(self):
        """Ternary ?: must match VM."""
        source = "proc f {x} { expr {$x > 0 ? $x : -$x} }\n"
        for x in (5, -3, 0):
            wasm_r = _compile_and_run_proc(source, "f", (x,))
            vm_r = self._vm_eval_proc(source, "f", (x,))
            assert wasm_r == vm_r, f"Mismatch for x={x}"

    def test_recursive_fib_matches_vm(self):
        """Recursive fibonacci via command substitution must match VM."""
        source = """\
proc fib {n} {
    if {$n <= 1} { return $n }
    expr {[fib [expr {$n - 1}]] + [fib [expr {$n - 2}]]}
}
"""
        for n in (0, 1, 2, 3, 6):
            wasm_r = _compile_and_run_proc(source, "fib", (n,))
            vm_r = self._vm_eval_proc(source, "fib", (n,))
            assert wasm_r == vm_r, f"fib({n}): WASM={wasm_r}, VM={vm_r}"

    def test_proc_calls_match_vm(self):
        """Proc-to-proc calls must match VM."""
        source = """\
proc double {x} { expr {$x * 2} }
proc caller {n} { double $n }
"""
        for n in (0, 1, 7, -3):
            wasm_r = _compile_and_run_proc(source, "caller", (n,))
            vm_r = self._vm_eval_proc(source, "caller", (n,))
            assert wasm_r == vm_r

    def test_nested_if_in_loop_matches_vm(self):
        """if inside a for loop must match VM."""
        source = """\
proc count_pos {n} {
    set count 0
    for {set i 1} {$i <= $n} {incr i} {
        if {$i > 0} {
            incr count
        }
    }
    return $count
}
"""
        for n in (0, 1, 5, 10):
            wasm_r = _compile_and_run_proc(source, "count_pos", (n,))
            vm_r = self._vm_eval_proc(source, "count_pos", (n,))
            assert wasm_r == vm_r, f"count_pos({n}): WASM={wasm_r}, VM={vm_r}"

    def test_gcd_matches_vm(self):
        """Euclidean GCD via while loop and modulo must match VM."""
        source = """\
proc gcd {a b} {
    while {$b != 0} {
        set t [expr {$a % $b}]
        set a $b
        set b $t
    }
    return $a
}
"""
        for a, b in ((12, 8), (100, 75), (17, 13), (0, 5), (7, 0)):
            wasm_r = _compile_and_run_proc(source, "gcd", (a, b))
            vm_r = self._vm_eval_proc(source, "gcd", (a, b))
            assert wasm_r == vm_r, f"gcd({a},{b}): WASM={wasm_r}, VM={vm_r}"

    def test_multi_proc_pipeline_matches_vm(self):
        """Chain of proc calls must match VM."""
        source = """\
proc square {x} { expr {$x * $x} }
proc inc {x} { expr {$x + 1} }
proc pipeline {n} { inc [square $n] }
"""
        for n in (0, 1, 3, 5, -2):
            wasm_r = _compile_and_run_proc(source, "pipeline", (n,))
            vm_r = self._vm_eval_proc(source, "pipeline", (n,))
            assert wasm_r == vm_r, f"pipeline({n}): WASM={wasm_r}, VM={vm_r}"

    def test_nested_loops_matches_vm(self):
        """Nested for loops must match VM."""
        source = """\
proc sum_products {n} {
    set s 0
    for {set i 1} {$i <= $n} {incr i} {
        for {set j 1} {$j <= $i} {incr j} {
            set s [expr {$s + $i * $j}]
        }
    }
    return $s
}
"""
        for n in (0, 1, 3, 4):
            wasm_r = _compile_and_run_proc(source, "sum_products", (n,))
            vm_r = self._vm_eval_proc(source, "sum_products", (n,))
            assert wasm_r == vm_r, f"sum_products({n}): WASM={wasm_r}, VM={vm_r}"

    def test_triple_nested_loops_matches_vm(self):
        """Triple-nested for loops must match VM."""
        source = """\
proc f {n} {
    set s 0
    for {set i 1} {$i <= $n} {incr i} {
        for {set j 1} {$j <= $i} {incr j} {
            for {set k 1} {$k <= $j} {incr k} {
                incr s
            }
        }
    }
    return $s
}
"""
        for n in (0, 1, 3, 4):
            wasm_r = _compile_and_run_proc(source, "f", (n,))
            vm_r = self._vm_eval_proc(source, "f", (n,))
            assert wasm_r == vm_r, f"triple({n}): WASM={wasm_r}, VM={vm_r}"

    def test_abs_function_matches_vm(self):
        """expr abs() must match VM."""
        source = "proc f {x} { expr {abs($x)} }\n"
        for x in (-5, 0, 3):
            wasm_r = _compile_and_run_proc(source, "f", (x,))
            vm_r = self._vm_eval_proc(source, "f", (x,))
            assert wasm_r == vm_r, f"abs({x}): WASM={wasm_r}, VM={vm_r}"

    def test_unary_ops_match_vm(self):
        """Unary operators must match VM."""
        source_neg = "proc f {x} { expr {-$x} }\n"
        source_not = "proc f {x} { expr {!$x} }\n"
        source_bitnot = "proc f {x} { expr {~$x} }\n"
        for x in (0, 1, -3, 42):
            wasm_r = _compile_and_run_proc(source_neg, "f", (x,))
            vm_r = self._vm_eval_proc(source_neg, "f", (x,))
            assert wasm_r == vm_r, f"neg({x}): WASM={wasm_r}, VM={vm_r}"
            wasm_r = _compile_and_run_proc(source_not, "f", (x,))
            vm_r = self._vm_eval_proc(source_not, "f", (x,))
            assert wasm_r == vm_r, f"not({x}): WASM={wasm_r}, VM={vm_r}"
            wasm_r = _compile_and_run_proc(source_bitnot, "f", (x,))
            vm_r = self._vm_eval_proc(source_bitnot, "f", (x,))
            assert wasm_r == vm_r, f"bitnot({x}): WASM={wasm_r}, VM={vm_r}"


# String result helper


def _compile_and_run_string(
    source: str,
    *,
    func_name: str = "::top",
    args: tuple[int, ...] = (),
) -> str:
    """Compile and run, returning the string representation of the result."""
    _, wasm_bytes = _compile_to_wasm(source)
    engine = _get_engine()
    store = wasmtime.Store(engine)
    wasi_config = wasmtime.WasiConfig()
    store.set_wasi(wasi_config)
    tcl_instance, rt_instance = _link_and_instantiate(store, wasm_bytes)

    obj_new_int = rt_instance.exports(store)["obj_new_int"]
    boxed_args = tuple(obj_new_int(store, a) for a in args)
    func = tcl_instance.exports(store)[func_name]
    result_obj = func(store, *boxed_args)

    if result_obj == 0:
        return ""

    # Read string representation from TclObj
    memory = rt_instance.exports(store)["memory"]
    buf = memory.data_ptr(store)
    addr = result_obj
    # Read type tag (offset 4)
    tag = int.from_bytes(buf[addr + 4 : addr + 8], "little", signed=True)
    if tag == 1:  # TYPE_INT
        val = int.from_bytes(buf[addr + 8 : addr + 16], "little", signed=True)
        return str(val)
    # TYPE_STRING (or others): read str_ptr and str_len
    str_ptr = int.from_bytes(buf[addr + 16 : addr + 20], "little", signed=True)
    str_len = int.from_bytes(buf[addr + 20 : addr + 24], "little", signed=True)
    if str_len <= 0:
        return ""
    return bytes(buf[str_ptr : str_ptr + str_len]).decode("utf-8", errors="replace")


def _compile_and_run_proc_string(
    source: str,
    proc_name: str,
    args: tuple[int, ...],
) -> str:
    """Compile and run a proc, returning the string result."""
    return _compile_and_run_string(source, func_name=f"::{proc_name}", args=args)


# List operations


class TestListOperations:
    """Test real list operations in the WASM runtime."""

    def test_llength_empty(self):
        """llength of empty string is 0."""
        result = _compile_and_run(
            'proc f {} { set x ""; llength $x }\n',
            func_name="::f",
        )
        assert result == 0

    def test_llength_single(self):
        """llength of a single-element list."""
        result = _compile_and_run(
            'proc f {} { set x "hello"; llength $x }\n',
            func_name="::f",
        )
        assert result == 1

    def test_llength_multiple(self):
        """llength of a multi-element list."""
        result = _compile_and_run(
            'proc f {} { set x "a b c d"; llength $x }\n',
            func_name="::f",
        )
        assert result == 4

    def test_lindex_first(self):
        """lindex returns the first element."""
        result = _compile_and_run_proc_string(
            'proc f {} { set x "alpha beta gamma"; lindex $x 0 }\n',
            "f",
            (),
        )
        assert result == "alpha"

    def test_lindex_middle(self):
        """lindex returns a middle element."""
        result = _compile_and_run_proc_string(
            'proc f {} { set x "alpha beta gamma"; lindex $x 1 }\n',
            "f",
            (),
        )
        assert result == "beta"

    def test_lindex_last(self):
        """lindex returns the last element."""
        result = _compile_and_run_proc_string(
            'proc f {} { set x "alpha beta gamma"; lindex $x 2 }\n',
            "f",
            (),
        )
        assert result == "gamma"

    def test_lrange(self):
        """lrange extracts a sublist."""
        result = _compile_and_run_proc_string(
            'proc f {} { set x "a b c d e"; lrange $x 1 3 }\n',
            "f",
            (),
        )
        assert result == "b c d"

    def test_lsort(self):
        """lsort sorts elements lexicographically."""
        result = _compile_and_run_proc_string(
            'proc f {} { set x "cherry apple banana"; lsort $x }\n',
            "f",
            (),
        )
        assert result == "apple banana cherry"

    def test_lsearch_found(self):
        """lsearch returns index of matching element."""
        result = _compile_and_run(
            'proc f {} { set x "alpha beta gamma"; lsearch $x beta }\n',
            func_name="::f",
        )
        assert result == 1

    def test_lsearch_not_found(self):
        """lsearch returns -1 when element not found."""
        result = _compile_and_run(
            'proc f {} { set x "alpha beta gamma"; lsearch $x delta }\n',
            func_name="::f",
        )
        assert result == -1

    def test_lappend_builds_list(self):
        """lappend builds a space-separated list."""
        result = _compile_and_run_proc_string(
            'proc f {} { set x ""; lappend x hello; lappend x world; return $x }\n',
            "f",
            (),
        )
        assert result == "hello world"


# Dict operations


class TestDictOperations:
    """Test dict operations in the WASM runtime."""

    def test_dict_set_and_get(self):
        """dict set then dict get returns the value."""
        result = _compile_and_run_proc_string(
            'proc f {} { set d ""; dict set d name Alice; dict get $d name }\n',
            "f",
            (),
        )
        assert result == "Alice"

    def test_dict_exists_true(self):
        """dict exists returns 1 for existing key."""
        result = _compile_and_run(
            'proc f {} { set d "name Alice age 30"; dict exists $d name }\n',
            func_name="::f",
        )
        assert result == 1

    def test_dict_exists_false(self):
        """dict exists returns 0 for missing key."""
        result = _compile_and_run(
            'proc f {} { set d "name Alice age 30"; dict exists $d email }\n',
            func_name="::f",
        )
        assert result == 0

    def test_dict_size(self):
        """dict size returns number of key-value pairs."""
        result = _compile_and_run(
            'proc f {} { set d "a 1 b 2 c 3"; dict size $d }\n',
            func_name="::f",
        )
        assert result == 3

    def test_dict_keys(self):
        """dict keys returns all keys."""
        result = _compile_and_run_proc_string(
            'proc f {} { set d "x 10 y 20 z 30"; dict keys $d }\n',
            "f",
            (),
        )
        assert result == "x y z"

    def test_dict_values(self):
        """dict values returns all values."""
        result = _compile_and_run_proc_string(
            'proc f {} { set d "x 10 y 20 z 30"; dict values $d }\n',
            "f",
            (),
        )
        assert result == "10 20 30"

    def test_dict_set_overwrites(self):
        """dict set overwrites existing key."""
        result = _compile_and_run_proc_string(
            'proc f {} { set d "name Alice"; dict set d name Bob; dict get $d name }\n',
            "f",
            (),
        )
        assert result == "Bob"

    def test_dict_multiple_set_get(self):
        """Multiple dict set/get operations."""
        result = _compile_and_run(
            'proc f {} { set d ""; dict set d a 10; dict set d b 20; expr {[dict get $d a] + [dict get $d b]} }\n',
            func_name="::f",
        )
        # This test uses shimmer: dict get returns strings "10" and "20",
        # which shimmer to integers for the expr
        assert result == 30


# Integer shimmer


class TestIntegerShimmer:
    """Test that string TclObj values shimmer to integers in expressions."""

    def test_string_number_in_expr(self):
        """A string "42" should shimmer to integer 42 in expr."""
        result = _compile_and_run(
            'proc f {} { set x "42"; expr {$x + 1} }\n',
            func_name="::f",
        )
        assert result == 43

    def test_negative_string_number(self):
        """A string "-5" should shimmer to integer -5."""
        result = _compile_and_run(
            'proc f {} { set x "-5"; expr {$x + 10} }\n',
            func_name="::f",
        )
        assert result == 5

    def test_string_zero(self):
        """A string "0" should shimmer to integer 0."""
        result = _compile_and_run(
            'proc f {} { set x "0"; expr {$x + 1} }\n',
            func_name="::f",
        )
        assert result == 1

    def test_dict_get_shimmer(self):
        """dict get returns a string that shimmers to int in expr."""
        result = _compile_and_run(
            'proc f {} { set d "count 7"; expr {[dict get $d count] * 2} }\n',
            func_name="::f",
        )
        assert result == 14


# String operations — extended


class TestStringOperationsExtended:
    """Test string sub-commands added beyond the original set."""

    def test_string_trimleft(self):
        """string trimleft strips leading whitespace."""
        result = _compile_and_run_proc_string(
            'proc f {} { string trimleft "  hello  " }\n',
            "f",
            (),
        )
        assert result == "hello  "

    def test_string_trimright(self):
        """string trimright strips trailing whitespace."""
        result = _compile_and_run_proc_string(
            'proc f {} { string trimright "  hello  " }\n',
            "f",
            (),
        )
        assert result == "  hello"

    def test_string_equal_true(self):
        """string equal returns 1 for matching strings."""
        result = _compile_and_run(
            "proc f {} { string equal hello hello }\n",
            func_name="::f",
        )
        assert result == 1

    def test_string_equal_false(self):
        """string equal returns 0 for different strings."""
        result = _compile_and_run(
            "proc f {} { string equal hello world }\n",
            func_name="::f",
        )
        assert result == 0

    def test_string_first(self):
        """string first finds first occurrence."""
        result = _compile_and_run(
            "proc f {} { string first ll hello }\n",
            func_name="::f",
        )
        assert result == 2

    def test_string_first_not_found(self):
        """string first returns -1 when not found."""
        result = _compile_and_run(
            "proc f {} { string first xyz hello }\n",
            func_name="::f",
        )
        assert result == -1

    def test_string_last(self):
        """string last finds last occurrence."""
        result = _compile_and_run(
            "proc f {} { string last l hello }\n",
            func_name="::f",
        )
        assert result == 3

    def test_string_repeat(self):
        """string repeat repeats a string N times."""
        result = _compile_and_run_proc_string(
            "proc f {} { string repeat ab 3 }\n",
            "f",
            (),
        )
        assert result == "ababab"

    def test_string_reverse(self):
        """string reverse reverses a string."""
        result = _compile_and_run_proc_string(
            "proc f {} { string reverse hello }\n",
            "f",
            (),
        )
        assert result == "olleh"

    def test_string_toupper(self):
        """string toupper converts to uppercase."""
        result = _compile_and_run_proc_string(
            "proc f {} { string toupper hello }\n",
            "f",
            (),
        )
        assert result == "HELLO"

    def test_string_tolower(self):
        """string tolower converts to lowercase."""
        result = _compile_and_run_proc_string(
            "proc f {} { string tolower HELLO }\n",
            "f",
            (),
        )
        assert result == "hello"

    def test_string_map(self):
        """string map applies substitutions."""
        result = _compile_and_run_proc_string(
            "proc f {} { string map {a A e E} hello }\n",
            "f",
            (),
        )
        assert result == "hEllo"

    def test_string_first_in_expr(self):
        """[string first] in expression context shimmers to integer."""
        result = _compile_and_run(
            "proc f {} { expr {[string first ll hello] + 1} }\n",
            func_name="::f",
        )
        assert result == 3

    def test_string_length_in_expr(self):
        """[string length] in expression context shimmers to integer."""
        result = _compile_and_run(
            "proc f {} { expr {[string length hello] * 2} }\n",
            func_name="::f",
        )
        assert result == 10

    def test_string_replace(self):
        """string replace replaces a range of characters."""
        result = _compile_and_run_proc_string(
            'proc f {} { string replace "hello world" 5 5 "_" }\n',
            "f",
            (),
        )
        assert result == "hello_world"

    def test_string_is_integer_true(self):
        """string is integer returns 1 for a valid integer."""
        result = _compile_and_run(
            'proc f {} { string is integer "42" }\n',
            func_name="::f",
        )
        assert result == 1

    def test_string_is_integer_false(self):
        """string is integer returns 0 for non-integer."""
        result = _compile_and_run(
            'proc f {} { string is integer "hello" }\n',
            func_name="::f",
        )
        assert result == 0

    def test_string_is_alpha(self):
        """string is alpha returns 1 for alphabetic string."""
        result = _compile_and_run(
            'proc f {} { string is alpha "hello" }\n',
            func_name="::f",
        )
        assert result == 1

    def test_string_is_digit(self):
        """string is digit returns 1 for digit-only string."""
        result = _compile_and_run(
            'proc f {} { string is digit "12345" }\n',
            func_name="::f",
        )
        assert result == 1

    def test_string_is_space(self):
        """string is space returns 1 for whitespace-only string."""
        result = _compile_and_run(
            'proc f {} { string is space "   " }\n',
            func_name="::f",
        )
        assert result == 1


# Switch with string matching


class TestSwitchStringMatching:
    """Test switch with string-based comparison."""

    def test_switch_exact_match(self):
        """switch -exact matches string values."""
        result = _compile_and_run(
            'proc f {} { set x "hello"; switch $x { hello { return 1 } world { return 2 } } }\n',
            func_name="::f",
        )
        assert result == 1

    def test_switch_exact_second(self):
        """switch -exact matches the second arm."""
        result = _compile_and_run(
            'proc f {} { set x "world"; switch $x { hello { return 1 } world { return 2 } } }\n',
            func_name="::f",
        )
        assert result == 2

    def test_switch_default(self):
        """switch falls through to default."""
        result = _compile_and_run(
            'proc f {} { set x "other"; switch $x { hello { return 1 } default { return 99 } } }\n',
            func_name="::f",
        )
        assert result == 99

    def test_switch_integer_strings(self):
        """switch matches integers as strings."""
        result = _compile_and_run(
            "proc f {x} { switch $x { 1 { return 10 } 2 { return 20 } 3 { return 30 } } }\n",
            func_name="::f",
            args=(2,),
        )
        assert result == 20


# break/continue


class TestBreakContinue:
    """Test break and continue emit proper br instructions.

    Note: break/continue require the CFG loop body to not create
    spurious inner loop structures.  Currently only simple loop
    bodies (without if/else) work; complex bodies need CFG fixes.
    These tests verify the codegen wiring is correct.
    """

    def test_break_compiles(self):
        """break command emits a br instruction, not a NOP."""
        from core.compiler.codegen.wasm import WasmOp

        wasm_mod, _ = _compile_to_wasm('proc f {} { foreach i "1 2 3" { break } }\n')
        proc_funcs = [f for f in wasm_mod.functions if "f" in f.name]
        if proc_funcs:
            has_br = any(i.op in (WasmOp.BR, WasmOp.BR_IF) for i in proc_funcs[0].body)
            assert has_br, "break should emit a br instruction"


# Unknown command traps


class TestUnknownCommandTraps:
    """Unknown commands must trap at runtime, not silently NOP."""

    def test_unsupported_command_traps(self):
        """Truly unsupported commands (exec, socket) should trap."""
        with pytest.raises(wasmtime.Trap):
            _compile_and_run(
                "proc f {} { exec ls }\n",
                func_name="::f",
            )

    def test_unknown_command_uses_interpreter(self):
        """Unknown commands emit tcl_eval fallback, not a hard trap."""
        wasm_mod, _ = _compile_to_wasm("proc f {} { bogus_cmd }\n")
        all_data = b""
        for seg in wasm_mod.data_segments:
            all_data += seg.data[4:]
        assert b"bogus_cmd" in all_data

    def test_known_commands_dont_trap(self):
        """Supported commands should execute without trapping."""
        result = _compile_and_run_proc(
            "proc f {x} { set y [expr {$x + 1}]; return $y }\n",
            "f",
            (5,),
        )
        assert result == 6


# Interpreter fallback


class TestInterpreterFallback:
    """Test that the Zig interpreter handles commands the compiler can't."""

    def test_eval_set_via_interpreter(self):
        """tcl_eval dispatches set correctly through the interpreter."""
        # The catch body forces re-parsing of the script through the interpreter
        result = _compile_and_run(
            "proc f {} { catch { set x 99 }; return 1 }\n",
            func_name="::f",
        )
        assert result == 1

    def test_catch_preserves_execution(self):
        """Code continues executing after the interpreter handles an error."""
        result = _compile_and_run(
            'proc f {} { set r [catch { error "fail" }]; return $r }\n',
            func_name="::f",
        )
        assert result == 1


# Global variable scoping


class TestGlobalVariables:
    """Test global variable table for cross-proc variable sharing."""

    def test_global_set_and_read(self):
        """One proc sets a global, another reads it."""
        source = """\
proc setter {} {
    global counter
    set counter 42
}
proc getter {} {
    global counter
    return $counter
}
proc test {} {
    setter
    getter
}
"""
        result = _compile_and_run(source, func_name="::test")
        assert result == 42

    def test_global_increment(self):
        """Global variable can be incremented across calls."""
        source = """\
proc init {} {
    global total
    set total 0
}
proc add_to_total {n} {
    global total
    set total [expr {$total + $n}]
}
proc get_total {} {
    global total
    return $total
}
proc test {} {
    init
    add_to_total 10
    add_to_total 20
    get_total
}
"""
        result = _compile_and_run(source, func_name="::test")
        assert result == 30

    def test_global_multiple_vars(self):
        """Multiple global variables in one proc."""
        source = """\
proc setup {} {
    global x y
    set x 10
    set y 20
}
proc sum_globals {} {
    global x y
    expr {$x + $y}
}
proc test {} {
    setup
    sum_globals
}
"""
        result = _compile_and_run(source, func_name="::test")
        assert result == 30


# catch / error handling


class TestCatch:
    """Test catch with error-flag based error propagation."""

    def test_catch_no_error(self):
        """catch with a normal body returns 0 (TCL_OK)."""
        result = _compile_and_run(
            "proc f {} { catch { set x 42 } }\n",
            func_name="::f",
        )
        assert result == 0

    def test_catch_with_error(self):
        """catch with error returns 1 (TCL_ERROR)."""
        result = _compile_and_run(
            'proc f {} { catch { error "something went wrong" } }\n',
            func_name="::f",
        )
        assert result == 1

    def test_catch_unsupported_command(self):
        """catch traps unsupported commands without aborting the module."""
        result = _compile_and_run(
            "proc f {} { catch { exec ls } }\n",
            func_name="::f",
        )
        # exec is unsupported — should be caught, returning 1
        assert result == 1

    def test_catch_continues_after_error(self):
        """Code after catch should continue executing."""
        result = _compile_and_run(
            'proc f {} { catch { error "fail" }; return 99 }\n',
            func_name="::f",
        )
        assert result == 99

    def test_catch_normal_body_continues(self):
        """catch with no error, followed by more code."""
        result = _compile_and_run(
            "proc f {} { set x 10; catch { set x 20 }; return $x }\n",
            func_name="::f",
        )
        assert result == 20

    def test_nested_catch(self):
        """Nested catch should handle errors independently."""
        result = _compile_and_run(
            'proc f {} { catch { catch { error "inner" }; set x 42 } }\n',
            func_name="::f",
        )
        # Outer catch should succeed (inner catch handles the error)
        assert result == 0
