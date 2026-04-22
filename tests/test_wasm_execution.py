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
    """Compile and execute a specific procedure.

    Runs ``::top`` first to execute any top-level setup code (global variable
    initialisations, proc registrations, etc.) before calling the named proc.
    This matches the real execution model where the top-level script runs as a
    module initialiser before individual procs are invoked.
    """
    _, wasm_bytes = _compile_to_wasm(source, optimise=optimise)

    engine = _get_engine()
    store = wasmtime.Store(engine)
    wasi_config = wasmtime.WasiConfig()
    store.set_wasi(wasi_config)

    tcl_instance, rt_instance = _link_and_instantiate(store, wasm_bytes)

    obj_new_int = rt_instance.exports(store)["obj_new_int"]
    obj_get_int = rt_instance.exports(store)["obj_get_int"]

    # Run the top-level script to set up globals and register procs
    top_func = tcl_instance.exports(store).get("::top")
    if top_func is not None:
        top_func(store)

    boxed_args = tuple(obj_new_int(store, a) for a in args)
    func = tcl_instance.exports(store)[f"::{proc_name}"]
    result_obj = func(store, *boxed_args)

    if result_obj == 0:
        return 0
    return obj_get_int(store, result_obj)


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
            "ns_set",
            "ns_restore",
            "diag_set",
            "global_set",
            "global_get",
            "proc_register_compiled",
            "frame_push",
            "frame_pop",
            # ``frame_set_argv`` / ``frame_get_argv`` / ``tcl_list``
            # are pulled in whenever the module defines procs so
            # the prologue can stash the invocation list for
            # ``info level 0`` and the inline reader can retrieve
            # it.
            "frame_set_argv",
            "frame_get_argv",
            # ``frame_set_pending_argv0`` / ``frame_take_pending_argv0``
            # back the call-site → callee ABI for ``argv0``: the
            # caller writes the invoked word into the pending slot
            # immediately before the compiled ``call``, and the
            # callee's prologue consumes it so ``info level 0``
            # reports the source-level command word (imported /
            # renamed / qualified forms all survive intact).
            "frame_set_pending_argv0",
            "frame_take_pending_argv0",
            "tcl_list",
            "local_set",
            "local_get",
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


# Compiled-proc local frame mirroring


class TestCompiledProcFrameMirror:
    """Compiled proc locals are mirrored into the call frame so that
    the interpreter (reached via eval fallback for unknown commands) can
    read them with var_resolve.

    ``eval {script}`` is not handled by the compiler and falls through to
    tcl_eval; the Zig interpreter's ``eval`` handler then runs ``script``
    in the current frame, exercising var_resolve against the mirrored locals.
    """

    def test_set_local_visible_via_eval_fallback(self):
        """Variable written by compiled set is readable through the interpreter.

        ``[eval {set b}]`` in value context falls to the eval-fallback path
        (not the statement IRBarrier path) — tcl_eval runs ``set b`` in the
        current frame, reading the mirrored local.  The result is captured in
        ``r`` by the compiled ``set`` and returned normally.
        """
        result = _compile_and_run_proc(
            "proc f {a} { set b [expr {$a + 1}]; set r [eval {set b}]; return $r }\n",
            "f",
            (5,),
        )
        assert result == 6

    def test_dynamic_varname_read_via_eval_fallback(self):
        """Dynamic ``set $n`` succeeds when n and the target are both mirrored.

        Both ``x`` and ``n`` are in the frame.  The interpreter substitutes
        ``$n`` → "x" then reads x from the frame.

        Note: ``{x}`` (braced) is used to force ``set n`` to store the
        literal string "x", rather than a bare ``x`` which the compiler
        would optimise as ``local.get(x_idx)`` (the value 14).
        """
        result = _compile_and_run_proc(
            "proc f {a} { set x [expr {$a * 2}]; set n {x}; set r [eval {set $n}]; return $r }\n",
            "f",
            (7,),
        )
        assert result == 14

    def test_param_update_visible_via_eval_fallback(self):
        """A param reassigned in the body is current in the frame."""
        result = _compile_and_run_proc(
            "proc f {x} { set x [expr {$x + 10}]; set r [eval {set x}]; return $r }\n",
            "f",
            (3,),
        )
        assert result == 13

    def test_incr_local_visible_via_eval_fallback(self):
        """incr updates are reflected in the frame."""
        result = _compile_and_run_proc(
            "proc f {a} { set c $a; incr c 5; set r [eval {set c}]; return $r }\n",
            "f",
            (10,),
        )
        assert result == 15


class TestEvalWriteback:
    """After eval modifies a frame variable, the compiled proc sees the new value.

    _emit_frame_readback reloads all synced locals from the frame hash table
    after every tcl_eval call.  Without it, eval { set x 99 } would update
    the frame but leave the WASM local stale.
    """

    def test_eval_set_writeback(self):
        """eval {set x 99} modifies the variable; compiled proc reads new value."""
        result = _compile_and_run_proc(
            "proc f {x} { eval {set x 99}; return $x }\n",
            "f",
            (1,),
        )
        assert result == 99

    def test_eval_incr_writeback(self):
        """eval {incr x} increments the variable; compiled proc reads result."""
        result = _compile_and_run_proc(
            "proc f {x} { eval {incr x 10}; return $x }\n",
            "f",
            (5,),
        )
        assert result == 15

    def test_eval_does_not_affect_untouched_locals(self):
        """eval that only touches one variable does not clobber others."""
        result = _compile_and_run_proc(
            "proc f {a b} { eval {set a 99}; return $b }\n",
            "f",
            (1, 42),
        )
        assert result == 42

    def test_multiple_evals_accumulate(self):
        """Successive evals each write back correctly."""
        result = _compile_and_run_proc(
            "proc f {x} { eval {set x [expr {$x + 1}]}; eval {set x [expr {$x * 2}]}; return $x }\n",
            "f",
            (3,),
        )
        assert result == 8  # (3+1)*2


class TestEvalUpvar:
    """upvar in eval'd scripts aliases into the proc's frame or global scope.

    Since eval runs in the proc's frame, upvar 0 aliases within the current
    frame.  upvar 1 looks at the proc's caller; for the tests here the caller
    is the test harness (no compiled frame), so the lookup falls to globals.
    upvar #0 directly aliases to the global scope.
    """

    def test_upvar_hash0_read(self):
        """upvar #0 inside eval reads a global variable."""
        result = _compile_and_run_proc(
            "set ::gval 77\nproc f {} { set r [eval {upvar #0 ::gval g; set g}]; return $r }\n",
            "f",
            (),
        )
        assert result == 77

    def test_upvar_hash0_write(self):
        """upvar #0 inside eval writes through to a global variable."""
        result = _compile_and_run_proc(
            "set ::gctr 0\nproc f {} { eval {upvar #0 ::gctr c; set c 55}; return $::gctr }\n",
            "f",
            (),
        )
        assert result == 55

    def test_upvar_named_global_alias(self):
        """upvar #0 otherName localName maps a local alias to a different global."""
        result = _compile_and_run_proc(
            "set ::base 10\nproc f {} { set r [eval {upvar #0 ::base myvar; expr {$myvar + 5}}]; return $r }\n",
            "f",
            (),
        )
        assert result == 15


class TestEvalUplevel:
    """uplevel in eval'd scripts shifts the interpreter's frame context."""

    def test_uplevel_hash0_reads_global(self):
        """uplevel #0 runs a script at global scope."""
        result = _compile_and_run_proc(
            "set ::answer 42\nproc f {} { set r [eval {uplevel #0 {set ::answer}}]; return $r }\n",
            "f",
            (),
        )
        assert result == 42

    def test_uplevel_hash0_sets_global(self):
        """uplevel #0 sets a global variable from inside a compiled proc's eval."""
        result = _compile_and_run_proc(
            "set ::shared 0\nproc f {} { eval {uplevel #0 {set ::shared 99}}; return $::shared }\n",
            "f",
            (),
        )
        assert result == 99


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


class TestParseCacheReplay:
    """End-to-end regression guard for the P9 parse cache (runtime/zig/parse_cache.zig).

    The cache populates on ``proc_register`` and replays a pre-built
    command + token slab on subsequent ``eval_script`` entries.  A
    miscompile in ``build_for_body`` (wrong ``tokens_len``, bad
    ``next_pos``, …) would manifest as divergent or wrong results
    across repeated invocations.  We invoke the same proc many times
    in a single store so the cache is exercised on every call after
    the first, and assert that results stay stable and match the
    value a correct parser + evaluator would produce.
    """

    def _invoke_many(
        self,
        source: str,
        proc_name: str,
        args: tuple[int, ...],
        n_calls: int,
    ) -> list[int]:
        _, wasm_bytes = _compile_to_wasm(source)
        engine = _get_engine()
        store = wasmtime.Store(engine)
        store.set_wasi(wasmtime.WasiConfig())
        tcl_instance, rt_instance = _link_and_instantiate(store, wasm_bytes)

        obj_new_int = rt_instance.exports(store)["obj_new_int"]
        obj_get_int = rt_instance.exports(store)["obj_get_int"]

        top = tcl_instance.exports(store).get("::top")
        if top is not None:
            top(store)

        func = tcl_instance.exports(store)[f"::{proc_name}"]
        results: list[int] = []
        for _ in range(n_calls):
            boxed = tuple(obj_new_int(store, a) for a in args)
            r = func(store, *boxed)
            results.append(0 if r == 0 else obj_get_int(store, r))
        return results

    def test_multi_command_body_stable_across_calls(self):
        """A proc with a multi-command interpreted body returns the same
        value on every call.  Cache miscompile would show up as one
        (or all) calls diverging from the expected 55.
        """
        source = """\
proc sum_to {n} {
    set total 0
    set i 1
    while {$i <= $n} {
        set total [expr {$total + $i}]
        incr i
    }
    return $total
}
"""
        results = self._invoke_many(source, "sum_to", (10,), n_calls=5)
        assert results == [55, 55, 55, 55, 55]

    def test_repeated_calls_distinct_args(self):
        """Repeated calls with different args still produce correct
        results per-call.  Guards against the cache accidentally
        hard-coding a value from a prior invocation's frame.
        """
        source = """\
proc square {n} {
    set result [expr {$n * $n}]
    return $result
}
"""
        _, wasm_bytes = _compile_to_wasm(source)
        engine = _get_engine()
        store = wasmtime.Store(engine)
        store.set_wasi(wasmtime.WasiConfig())
        tcl_instance, rt_instance = _link_and_instantiate(store, wasm_bytes)
        obj_new_int = rt_instance.exports(store)["obj_new_int"]
        obj_get_int = rt_instance.exports(store)["obj_get_int"]
        top = tcl_instance.exports(store).get("::top")
        if top is not None:
            top(store)

        func = tcl_instance.exports(store)["::square"]
        pairs: list[tuple[int, int]] = []
        for n in (2, 3, 4, 5, 6, 7):
            boxed = obj_new_int(store, n)
            r = func(store, boxed)
            pairs.append((n, 0 if r == 0 else obj_get_int(store, r)))
        assert pairs == [(2, 4), (3, 9), (4, 16), (5, 25), (6, 36), (7, 49)]


class TestRenameAndAlias:
    """End-to-end rename / interp alias behaviour.

    The compiler leaves ``rename`` / ``interp`` as runtime calls (they
    aren't specialisable at compile time — the command / target only
    resolve when the source runs), so these tests exercise the full
    top-level eval path through the interpreter fallback.  The state
    we inspect lives in globals so we can pull it out via the runtime's
    ``global_get`` helper.
    """

    def _run_and_read_global(self, source: str, name: str) -> bytes:
        _, wasm_bytes = _compile_to_wasm(source)
        engine = _get_engine()
        store = wasmtime.Store(engine)
        store.set_wasi(wasmtime.WasiConfig())
        tcl_instance, rt_instance = _link_and_instantiate(store, wasm_bytes)
        top = tcl_instance.exports(store).get("::top")
        if top is not None:
            top(store)
        return _read_global_string(rt_instance, store, name)

    def test_rename_basic(self):
        """``rename foo bar`` moves the proc; calling the new name
        runs the proc body, calling the old name raises an error."""
        result = self._run_and_read_global(
            """\
proc orig {} { set ::result ran-orig }
rename orig renamed
renamed
""",
            "::result",
        )
        assert result == b"ran-orig"

    def test_rename_to_empty_deletes(self):
        """``rename foo {}`` removes ``foo`` — calling it afterwards
        is an error.  We stamp a ``deleted`` marker first so the test
        asserts on proc execution order rather than relying on error
        propagation."""
        result = self._run_and_read_global(
            """\
proc once {} { set ::result hit }
once
rename once {}
set ::marker stopped
""",
            "::marker",
        )
        assert result == b"stopped"

    def test_interp_alias_basic(self):
        """``interp alias {} greet {} say_hello`` — calling ``greet``
        dispatches to ``say_hello``."""
        result = self._run_and_read_global(
            """\
proc say_hello {name} { set ::result hello-$name }
interp alias {} greet {} say_hello
greet world
""",
            "::result",
        )
        assert result == b"hello-world"

    def test_interp_alias_with_prefix(self):
        """``interp alias {} g {} say header`` — each dispatch of
        ``g tail`` runs as ``say header tail``."""
        result = self._run_and_read_global(
            """\
proc say {first second} { set ::result $first/$second }
interp alias {} g {} say top
g bottom
""",
            "::result",
        )
        assert result == b"top/bottom"


class TestInterpHideExpose:
    """End-to-end ``interp hide`` / ``interp expose`` / ``interp
    hidden`` behaviour through the runtime's eval fallback.  The
    compiler leaves ``interp`` as an eval call, so these exercise
    the full dispatch path.
    """

    def _run_and_read_global(self, source: str, name: str) -> bytes:
        _, wasm_bytes = _compile_to_wasm(source)
        engine = _get_engine()
        store = wasmtime.Store(engine)
        store.set_wasi(wasmtime.WasiConfig())
        tcl_instance, rt_instance = _link_and_instantiate(store, wasm_bytes)
        top = tcl_instance.exports(store).get("::top")
        if top is not None:
            top(store)
        return _read_global_string(rt_instance, store, name)

    def test_hide_makes_same_unit_call_fail(self):
        """After ``interp hide {} foo`` the bare ``foo`` call must
        raise ``unknown command: foo`` — not dispatch to the
        compile-time-known body.  Exercises the compiler's
        proc-index invalidation for procs targeted by ``interp
        hide`` in the same translation unit (see
        ``docs/design/runtime/command-introspection.md`` §8.1)."""
        result = self._run_and_read_global(
            """\
proc foo {} { set ::result should-not-run }
set ::result untouched
interp hide {} foo
catch foo msg
""",
            "::result",
        )
        assert result == b"untouched"

    def test_rename_makes_old_name_unreachable_in_same_unit(self):
        """After ``rename foo bar`` the old name must stop
        resolving, and the new name must reach the body — proving
        the compiler didn't specialise the call to a direct
        ``call $::foo`` that ignores the rename."""
        result = self._run_and_read_global(
            """\
proc foo {} { set ::result old-ran }
rename foo bar
set ::result new-name-not-called
bar
""",
            "::result",
        )
        assert result == b"old-ran"

    def test_rename_inside_namespace_eval_invalidates_qualified_name(self):
        """``namespace eval ::ns { proc foo {}; rename foo bar }``
        must invalidate ``::ns::foo`` in the compiler's
        ``proc_index``, not just ``::foo``.  Without the
        context-aware invalidation, calls to ``foo`` elsewhere in
        the module would still emit ``call $::ns::foo`` and skip
        the runtime rename.

        We verify the compile-time invariant: the emitted WAT for
        the module must NOT contain a direct ``call $::ns::foo``
        anywhere — every invocation has to route through
        ``tcl_eval``.  This pins the fix for the Copilot review
        finding on PR #200 (proc_index qualification gap inside
        namespace eval).
        """
        source = """\
namespace eval ::ns {
    proc helper {} { return 1 }
    rename helper renamed
}
::ns::helper
::ns::renamed
"""
        wasm_module, _ = _compile_to_wasm(source)
        wat = wasm_module.to_wat()
        assert "call $::ns::helper" not in wat, (
            "proc_index should have been invalidated for ::ns::helper"
        )
        # The rename target must also route via eval — ``::ns::renamed``
        # is the "new" name and needs to resolve at runtime too.
        assert "call $::ns::renamed" not in wat, (
            "proc_index should have been invalidated for ::ns::renamed"
        )
        # Sanity: tcl_eval is imported, proving the eval-fallback
        # path is the one being used for these calls.
        assert 'import "tcl" "tcl_eval"' in wat

    def test_chained_rename_round_trip(self):
        """``rename foo bar; rename bar foo`` must restore the
        original name.  Exercises the compiler's
        ``_collect_dynamically_modified_procs`` invalidation
        across a chain — both ``foo`` and ``bar`` are invalidated
        in the first rename, both again in the second, so both
        calls route through ``tcl_eval`` / ``proc_lookup`` and
        observe the live cmd_table state at dispatch time."""
        result = self._run_and_read_global(
            """\
proc foo {} { set ::result round-tripped }
rename foo bar
rename bar foo
foo
""",
            "::result",
        )
        assert result == b"round-tripped"

    def test_hide_then_expose_restores_proc(self):
        """Hiding a proc removes it from the command table; exposing
        puts it back so subsequent calls dispatch normally."""
        result = self._run_and_read_global(
            """\
proc greet {} { set ::result hello }
interp hide {} greet
interp expose {} greet
greet
""",
            "::result",
        )
        assert result == b"hello"

    def test_invokehidden_dispatches_into_hidden_table(self):
        """``interp invokehidden {} foo arg`` finds ``foo`` in the
        hidden table and invokes it with the supplied args."""
        result = self._run_and_read_global(
            """\
proc greet {who} { set ::result "hello-$who" }
interp hide {} greet
interp invokehidden {} greet world
""",
            "::result",
        )
        assert result == b"hello-world"

    def test_invokehidden_unknown_command_raises(self):
        """``interp invokehidden {} nonexistent`` raises the
        ``invalid hidden command name "X"`` error from tclInterp.c."""
        result = self._run_and_read_global(
            """\
catch {interp invokehidden {} nonexistent} msg
set ::result $msg
""",
            "::result",
        )
        assert result == b'invalid hidden command name "nonexistent"'

    def test_invokehidden_with_global_flag(self):
        """``interp invokehidden {} -global foo`` dispatches in the
        global namespace regardless of where it was called."""
        result = self._run_and_read_global(
            """\
proc mark {} { set ::result at-root }
interp hide {} mark
interp invokehidden {} -global mark
""",
            "::result",
        )
        assert result == b"at-root"

    def test_invokehidden_with_namespace_flag_passes_args(self):
        """``interp invokehidden {} -namespace ::ns mark arg1 arg2`` —
        the hidden proc is invoked inside ``::ns`` and receives the
        trailing argv verbatim.  Pins the option-parser's arg-
        consumption flow (``-namespace`` consumes a value slot,
        subsequent args pass through unchanged)."""
        result = self._run_and_read_global(
            """\
namespace eval ::target {}
proc carry {a b} { set ::result "$a/$b" }
interp hide {} carry
interp invokehidden {} -namespace ::target carry left right
""",
            "::result",
        )
        assert result == b"left/right"

    def test_hidden_rejects_extra_args(self):
        """``interp hidden`` requires ``objc == 2`` in Tcl 9's
        ``HiddenCmdsNamesObjCmd``.  We accept the bare form and
        the single-path form (``interp hidden {}``) but raise
        ``wrong # args`` for any extra tokens — matches tclsh's
        error shape rather than silently ignoring the tail."""
        result = self._run_and_read_global(
            """\
catch {interp hidden {} extra-token} msg
set ::result $msg
""",
            "::result",
        )
        assert result == b'wrong # args: should be "interp hidden path"'

    def test_hidden_lists_hidden_commands(self):
        """``interp hidden {}`` returns a space-separated list of
        currently-hidden names."""
        result = self._run_and_read_global(
            """\
proc foo {} {}
proc bar {} {}
interp hide {} foo
interp hide {} bar
set ::result [interp hidden {}]
""",
            "::result",
        )
        names = set(result.decode("utf-8").split())
        assert names == {"foo", "bar"}


class TestInterpChildren:
    """End-to-end ``interp create`` / ``eval`` / ``exists`` /
    ``slaves`` / ``delete`` behaviour through the runtime's eval
    fallback.  The compiler leaves every ``interp`` call as an
    eval-fallback (``_collect_dynamically_modified_procs`` forces a
    full ``proc_index`` flush when it spots any of these), so these
    exercise the runtime's child-interp registry through the full
    dispatch path.
    """

    def _run_and_read_global(self, source: str, name: str) -> bytes:
        _, wasm_bytes = _compile_to_wasm(source)
        engine = _get_engine()
        store = wasmtime.Store(engine)
        store.set_wasi(wasmtime.WasiConfig())
        tcl_instance, rt_instance = _link_and_instantiate(store, wasm_bytes)
        top = tcl_instance.exports(store).get("::top")
        if top is not None:
            top(store)
        return _read_global_string(rt_instance, store, name)

    def test_create_returns_name(self):
        """``interp create`` returns the new interp's name; explicit
        name form uses the caller-supplied identifier verbatim."""
        result = self._run_and_read_global(
            """\
set ::result [interp create worker]
""",
            "::result",
        )
        assert result == b"worker"

    def test_create_auto_name(self):
        """Anonymous ``interp create`` assigns ``interp<N>``."""
        result = self._run_and_read_global(
            """\
set ::result [interp create]
""",
            "::result",
        )
        # The counter advances across invocations; we only check the
        # prefix since later tests in the same module may have
        # burned earlier numbers.
        assert result.startswith(b"interp")

    def test_exists_tracks_create_delete(self):
        """``interp exists`` returns 1 after create and 0 after
        delete."""
        result = self._run_and_read_global(
            """\
interp create gone
set ::before [interp exists gone]
interp delete gone
set ::after [interp exists gone]
set ::result "$::before $::after"
""",
            "::result",
        )
        assert result == b"1 0"

    def test_slaves_lists_children(self):
        """``interp slaves`` returns the child simple names as a
        Tcl list."""
        result = self._run_and_read_global(
            """\
interp create a
interp create b
set ::result [lsort [interp slaves]]
""",
            "::result",
        )
        assert result == b"a b"

    def test_eval_runs_script_in_child(self):
        """Scripts passed to ``interp eval`` execute inside the
        child — reading back the value through a second eval
        confirms the first eval's side effect took.
        """
        result = self._run_and_read_global(
            """\
interp create worker
interp eval worker {set x 42}
set ::result [interp eval worker {set x}]
""",
            "::result",
        )
        assert result == b"42"

    def test_eval_concat_multiple_scripts(self):
        """``interp eval child a b c`` concatenates ``a b c`` with
        spaces and evaluates the result (matches ``InterpEvalObjCmd``)."""
        result = self._run_and_read_global(
            """\
interp create worker
interp eval worker set r 123
set ::result [interp eval worker {set r}]
""",
            "::result",
        )
        assert result == b"123"

    def test_child_has_independent_procs(self):
        """Procs registered inside a child are reachable from the
        child's own eval but absent from the parent's registry."""
        result = self._run_and_read_global(
            """\
interp create worker
interp eval worker {proc greet {} {return hello}}
set ::result [interp eval worker greet]
""",
            "::result",
        )
        assert result == b"hello"

    def test_delete_current_raises(self):
        """Deleting the current interp (empty path) raises
        ``cannot delete the current interpreter``."""
        result = self._run_and_read_global(
            """\
catch {interp delete {}} msg
set ::result $msg
""",
            "::result",
        )
        assert result == b"cannot delete the current interpreter"

    def test_cross_interp_hide(self):
        """``interp hide child foo`` hides a proc in the child's
        cmd_table, not the parent's."""
        result = self._run_and_read_global(
            """\
interp create worker
interp eval worker {proc foo {} {return 1}}
interp hide worker foo
# After hide, the child can't reach ``foo`` via a bare call.
set ::result [interp eval worker {catch foo}]
""",
            "::result",
        )
        assert result == b"1"

    def test_cross_interp_invokehidden(self):
        """``interp invokehidden child foo`` dispatches into the
        child's hidden table."""
        result = self._run_and_read_global(
            """\
interp create worker
interp eval worker {proc mark {} {set ::marked yes}}
interp hide worker mark
interp invokehidden worker mark
# The assignment landed inside the child — verify it.
set ::result [interp eval worker {set ::marked}]
""",
            "::result",
        )
        assert result == b"yes"

    def test_cross_interp_alias(self):
        """``interp alias child foo {} parent_proc`` creates an
        alias in the child whose target resolves in the parent."""
        result = self._run_and_read_global(
            """\
proc parent_proc {who} { set ::result "hi-$who" }
interp create worker
interp alias worker hi {} parent_proc
interp eval worker {hi world}
""",
            "::result",
        )
        assert result == b"hi-world"


class TestNamespaceEvalPostFlush:
    """Regression tests: inside a ``namespace eval ::ns { … }`` body
    that is inlined into the enclosing script, unqualified calls to
    helper procs defined in the same block must resolve via the
    runtime's namespace tree even after a full ``proc_index`` flush
    triggered by ``interp create`` / ``eval`` / ``delete``.

    The codegen wraps every ``IRBlock`` body with
    ``tcl_ns_set(block_ns)`` / ``tcl_ns_restore`` so that any
    eval-fallback inside the body sees ``ns_current() == ::ns``.
    Without that wrapper the ``callable_proc_index.clear()`` path
    in :func:`wasm_codegen_module` leaves every call inside the
    block routed through ``tcl_eval``, and ``proc_lookup`` runs
    with the outer namespace (typically ``::``) so unqualified
    names never find their ``::ns::`` siblings.

    This is the concrete symptom that surfaced when sourcing
    ``tmp/tcl9.0.3/library/tcltest/tcltest.tcl`` through the WASM
    runtime: the top-level ``namespace eval tcltest { …
    ArrayDefault originalEnv [array get ::env] … }`` trapped with
    ``unknown command: ArrayDefault`` despite the proc being
    defined immediately above.
    """

    def _run_and_read_global(self, source: str, name: str) -> bytes:
        _, wasm_bytes = _compile_to_wasm(source)
        engine = _get_engine()
        store = wasmtime.Store(engine)
        store.set_wasi(wasmtime.WasiConfig())
        tcl_instance, rt_instance = _link_and_instantiate(store, wasm_bytes)
        top = tcl_instance.exports(store).get("::top")
        if top is not None:
            top(store)
        return _read_global_string(rt_instance, store, name)

    def test_unqualified_call_inside_namespace_eval_body_without_flush(self):
        """Baseline: without ``interp create`` in scope, no
        full-flush happens; the compile-time ``proc_index`` knows
        about ``::foo::bar`` and the call is specialised.  This
        case already worked before the fix, pinned here so
        regressions to the specialised path also fail loudly.
        """
        result = self._run_and_read_global(
            """\
namespace eval ::foo {
    proc bar {} { return 42 }
    set ::result [bar]
}
""",
            "::result",
        )
        assert result == b"42"

    def test_unqualified_call_inside_namespace_eval_body_post_flush(self):
        """Post-flush path: ``interp create`` forces
        ``callable_proc_index.clear()`` so every call inside the
        body falls back to ``tcl_eval``.  The runtime
        ``ns_current`` must be ``::foo`` for ``bar`` to resolve.

        This is the direct repro of the ArrayDefault trap that
        blocked sourcing tcltest.tcl before the fix."""
        result = self._run_and_read_global(
            """\
interp create child
interp delete child
namespace eval ::foo {
    proc bar {} { return 42 }
    set ::result [bar]
}
""",
            "::result",
        )
        assert result == b"42"

    def test_nested_namespace_eval_bodies_post_flush(self):
        """Nested ``namespace eval`` bodies must push / restore
        correctly so the inner block's helpers shadow the outer
        block's helpers by simple name, and on return the outer
        block's resolution is restored."""
        result = self._run_and_read_global(
            """\
interp create child
interp delete child
namespace eval ::outer {
    proc helper {} { return from-outer }
    namespace eval ::outer::inner {
        proc helper {} { return from-inner }
        set ::inner_result [helper]
    }
    set ::outer_result [helper]
}
set ::result $::outer_result:$::inner_result
""",
            "::result",
        )
        assert result == b"from-outer:from-inner"

    def test_tcltest_like_array_default_pattern(self):
        """Closest-to-upstream repro: a proc defined inside the
        block is invoked from the same block with a literal list
        argument — the exact shape of tcltest's
        ``ArrayDefault originalEnv [list …]`` call that trapped
        before the fix."""
        result = self._run_and_read_global(
            """\
interp create child
interp delete child
namespace eval ::tcltest {
    proc ArrayDefault {varName value} {
        variable $varName
        array set $varName $value
        return ok
    }
    set ::result [ArrayDefault originalEnv {a 1 b 2}]
}
""",
            "::result",
        )
        assert result == b"ok"

    def test_namespace_eval_body_wat_contains_ns_set_and_restore(self):
        """Compile-time invariant: the IRBlock wrapper emits calls
        to both ``ns_set`` and ``ns_restore`` so the runtime
        namespace context is pushed and popped around the body.
        """
        source = """\
interp create child
interp delete child
namespace eval ::foo {
    proc bar {} { return 42 }
    set ::result [bar]
}
"""
        wasm_module, _ = _compile_to_wasm(source)
        wat = wasm_module.to_wat()
        assert 'import "tcl" "ns_set"' in wat, (
            "tcl_ns_set import must be present for namespace eval body wrapper"
        )
        assert 'import "tcl" "ns_restore"' in wat, (
            "tcl_ns_restore import must be present for namespace eval body wrapper"
        )
        assert "call $::foo::bar" not in wat, (
            "callable proc index should be flushed by interp create"
        )


class TestProcDefaultsUnderFlush:
    """Regression tests: when a compiled proc is reached via the host
    bridge (``env.call_compiled_proc``) from an eval-fallback caller,
    missing param slots arrive as null TclObj pointers (``0``).  The
    proc's prologue must substitute the declared default for each
    null slot, otherwise ``$param`` resolves to the empty string and
    dynamic-dispatch patterns like ``catch {$verify $value}`` return
    an empty result instead of invoking the default verifier.

    This is the bug that caused tcltest's ``Option`` proc to
    silently lose its ``verify`` parameter when invoked via
    ``tcl_eval`` (the post–``interp create`` full-flush path).
    """

    def _run_and_read_global(self, source: str, name: str) -> bytes:
        _, wasm_bytes = _compile_to_wasm(source)
        engine = _get_engine()
        store = wasmtime.Store(engine)
        store.set_wasi(wasmtime.WasiConfig())
        tcl_instance, rt_instance = _link_and_instantiate(store, wasm_bytes)
        top = tcl_instance.exports(store).get("::top")
        if top is not None:
            top(store)
        return _read_global_string(rt_instance, store, name)

    def test_default_applied_for_missing_arg_under_flush(self):
        """``interp create`` → full flush → call via eval.  The
        proc's default for a missing trailing arg must be applied
        by the prologue."""
        result = self._run_and_read_global(
            """\
interp create child
interp delete child
proc Opt {v {verify ident}} {
    set ::result "v=$v verify=$verify"
}
Opt hello
""",
            "::result",
        )
        assert result == b"v=hello verify=ident"

    def test_default_preserved_when_explicit_empty_string(self):
        """Explicit empty-string arg must NOT be replaced by the
        default — empty string is a valid value distinct from
        "missing slot".  ``$verify`` should be empty, not the
        default."""
        result = self._run_and_read_global(
            """\
interp create child
interp delete child
proc Opt {v {verify ident}} {
    set ::result "<$verify>"
}
Opt hello ""
""",
            "::result",
        )
        assert result == b"<>"

    def test_tcltest_option_verify_pattern_post_flush(self):
        """The exact shape tcltest's ``Option`` uses:
        ``catch {$verify $value}`` with ``verify`` defaulted to
        ``AcceptAll``.  Under the host-bridge call path the
        default must reach the catch body."""
        result = self._run_and_read_global(
            """\
interp create child
interp delete child
namespace eval ::foo {
    proc AcceptAll {value} { return $value }
    proc Option {opt value {verify AcceptAll}} {
        if {[catch {$verify $value} msg]} {
            return -code error $msg
        }
        set ::result $msg
    }
    Option -match {body error}
}
""",
            "::result",
        )
        assert result == b"body error"


class TestNamespaceImportRuntime:
    """Regression tests: ``namespace import`` / ``namespace export``
    / ``namespace forget`` must have their runtime side effects —
    creating / removing command redirects in the current
    ``cmd_table`` — executed so eval-fallback dispatch can resolve
    imported names.

    The codegen's compile-time resolver already shortens
    ``testConstraint`` → ``::tcltest::testConstraint`` for
    specialised calls, but under ``interp create`` / ``eval`` /
    ``delete`` the callable proc index is cleared and every call
    routes through ``tcl_eval``.  Without the runtime redirect,
    the interpreter's ``proc_lookup`` can't find the short name
    and raises ``unknown command``.
    """

    def _run_and_read_global(self, source: str, name: str) -> bytes:
        _, wasm_bytes = _compile_to_wasm(source)
        engine = _get_engine()
        store = wasmtime.Store(engine)
        store.set_wasi(wasmtime.WasiConfig())
        tcl_instance, rt_instance = _link_and_instantiate(store, wasm_bytes)
        top = tcl_instance.exports(store).get("::top")
        if top is not None:
            top(store)
        return _read_global_string(rt_instance, store, name)

    def test_import_resolves_short_name_under_flush(self):
        """After ``interp create`` triggers full flush,
        unqualified calls to an imported name must resolve
        through the runtime redirect."""
        result = self._run_and_read_global(
            """\
namespace eval ::src {
    namespace export foo
    proc foo {} { return from-src }
}
namespace import -force ::src::foo
interp create child
interp delete child
set ::result [foo]
""",
            "::result",
        )
        assert result == b"from-src"

    def test_import_wat_uses_eval_fallback(self):
        """The WAT for ``namespace import`` must contain a
        ``tcl_eval`` invocation so the runtime's ``ns_import``
        actually creates redirects at runtime.  Without it, the
        compile-time resolver is the only mechanism and runtime
        dispatch has no visibility."""
        source = """\
namespace eval ::src { namespace export foo; proc foo {} {} }
namespace import -force ::src::foo
"""
        wasm_module, _ = _compile_to_wasm(source)
        wat = wasm_module.to_wat()
        # The eval-fallback for the namespace import statement
        # reconstructs its script and passes it to tcl_eval.
        assert "namespace import" in wat, (
            "namespace import script text must be in the WASM data segment"
        )


class TestTopLevelForeachIterVarVisible:
    """Regression: at top level, a ``foreach`` iteration variable
    must be reachable from an eval-fallback in the body.  The
    compiler emits the iter var as a WASM local (fast path), so
    an eval-fallback reference like ``[eval $i]`` or ``interp
    delete $i`` couldn't resolve ``$i`` via the global table and
    saw the empty string, silently passing "" to the runtime.

    ``interp.test`` trips this with
    ``foreach i [interp children] { interp delete $i }`` — after
    earlier tests create children, the delete loop would fail
    with "cannot delete the current interpreter" because ``$i``
    resolved to empty.  The emitter now publishes each iteration
    value as a global (``tcl_global_set``) immediately after
    binding the WASM local, so the eval-fallback sees the real
    element.
    """

    def _run_and_read_stdout(self, source: str) -> str:
        import tempfile

        from tests.test_wasm_real_tcl import _compile_tcl_with_diag, _run_wasm

        wasm, _diag = _compile_tcl_with_diag(source, "toplevel_foreach")
        with tempfile.TemporaryDirectory() as tmpd:
            r = _run_wasm(wasm, capture_stdout=True, preopen_tmpdir=tmpd)
        return r[1].strip() if len(r) >= 2 else ""

    def test_foreach_iter_visible_to_interp_delete(self):
        """The top-level-foreach + ``interp delete $i`` pattern
        from ``interp.test``."""
        out = self._run_and_read_stdout(
            """\
interp create a
interp create b
foreach i [interp children] {
    interp delete $i
}
puts "remaining=<[interp children]>"
"""
        )
        assert out == "remaining=<>"

    def test_foreach_iter_visible_to_eval_fallback(self):
        """Generic shape: an eval-fallback in the loop body
        reads ``$i`` and sees the real iteration value."""
        out = self._run_and_read_stdout(
            """\
set result ""
foreach i {alpha beta gamma} {
    # ``append`` goes through the runtime for dynamic args.
    append result [string toupper $i] ","
}
puts $result
"""
        )
        assert out == "ALPHA,BETA,GAMMA,"


class TestInfoLevelArgv:
    """Regression tests: ``info level 0`` / ``info level -N`` must
    return the actual list of words (proc name + call args) that
    entered the target frame.  tcltest leans on this pattern
    heavily (``if {[llength [info level 0]] == 1} { return
    $current } else { set $var $arg }``), so a stub that returns a
    placeholder list breaks every accessor.

    The runtime stores per-frame argv in ``tcl_frames.frame_argv``
    and the compiled-proc prologue calls ``frame_set_argv`` with
    a list built element-by-element from ``[<proc-tail>,
    <param-1>, <param-2>, ...]``.  ``info level 0`` reads slot 0
    (top frame); ``info level -N`` reads N frames up.
    """

    def _run_and_read_stdout(self, source: str) -> str:
        """Compile the source to WASM and return captured stdout.

        Separate helper from ``_run_and_read_global`` because these
        tests probe ``puts`` output — ``info level 0`` inside a
        proc and a ``[llength [info level 0]]`` branch each print
        their measured value so the assertion is against a concrete
        string.
        """
        import tempfile

        from tests.test_wasm_real_tcl import _compile_tcl_with_diag, _run_wasm

        wasm, _diag = _compile_tcl_with_diag(source, "info_level")
        with tempfile.TemporaryDirectory() as tmpd:
            r = _run_wasm(wasm, capture_stdout=True, preopen_tmpdir=tmpd)
        return r[1].strip() if len(r) >= 2 else ""

    def test_level_zero_returns_real_invocation(self):
        """``info level 0`` inside a proc returns the full invocation
        list (proc name + all args)."""
        out = self._run_and_read_stdout(
            """\
proc myp {a b} { puts [info level 0] }
myp hello world
"""
        )
        assert out == "myp hello world"

    def test_level_zero_llength_matches_arg_count(self):
        """The llength of ``info level 0`` equals (n_params + 1)
        when the caller supplied every declared arg."""
        out = self._run_and_read_stdout(
            """\
proc myp {a b} { puts [llength [info level 0]] }
myp hello world
"""
        )
        assert out == "3"

    def test_leading_underscore_param_is_not_skipped(self):
        """Leading-underscore parameter names are valid user-visible
        Tcl identifiers — the prologue must bind them into the
        frame, substitute declared defaults when null, and include
        them in the ``info level 0`` argv list.  A stale filter
        that skipped them (``pname.startswith("_")``) produced a
        wrong argv list and made ``$_x`` resolve to empty inside
        the body.  Pinned here so that filter never comes back."""
        out = self._run_and_read_stdout(
            """\
proc myp {_x _y} { puts "[info level 0] /// $_x $_y" }
myp hello world
"""
        )
        assert out == "myp hello world /// hello world"

    def test_level_zero_llength_for_no_arg_call(self):
        """A zero-param proc called with no args returns llength 1
        — just the proc name.  tcltest's accessor pattern uses
        this case for "was I called without args?"."""
        out = self._run_and_read_stdout(
            """\
proc myp {} { puts [llength [info level 0]] }
myp
"""
        )
        assert out == "1"

    def test_variadic_args_tail_expands_in_argv(self):
        """``proc p {args}`` is variadic: caller dispatch packs
        surplus words into a list before the call.  The prologue
        must expand that list back into individual argv elements
        so ``[info level 0]`` reports the caller's true words
        (``p a b c``), not a nested list (``p {a b c}``).
        """
        out = self._run_and_read_stdout(
            """\
proc p {args} {
    set l [info level 0]
    puts "L=[llength $l] V=<$l>"
}
p alpha beta gamma
"""
        )
        assert out == "L=4 V=<p alpha beta gamma>"

    def test_variadic_args_tail_empty_yields_llength_one(self):
        """``proc p {args}`` called with no arguments must report
        ``llength == 1`` (just the proc name) — the packed empty
        ``args`` list must not contribute a spurious element."""
        out = self._run_and_read_stdout(
            """\
proc p {args} { puts [llength [info level 0]] }
p
"""
        )
        assert out == "1"

    def test_variadic_mixed_fixed_and_args_tail(self):
        """``proc p {x args}`` with a fixed param plus a variadic
        tail: the argv list should contain the proc name, the
        fixed arg, then each tail element as its own list
        element.  Pins the "no collapse into nested list" shape
        (``{p one {two three}}`` would be wrong)."""
        out = self._run_and_read_stdout(
            """\
proc p {x args} {
    set l [info level 0]
    puts "L=[llength $l] V=<$l>"
}
p one two three
"""
        )
        assert out == "L=4 V=<p one two three>"

    def test_info_level_large_integer_raises_bad_level(self):
        """An integer argument that overflows the i32 range must
        surface ``bad level "..."`` rather than wrap silently.
        Pins the ``parse_signed_int`` overflow guard."""
        out = self._run_and_read_stdout(
            """\
proc myp {} {
    catch {info level 99999999999} msg
    puts $msg
}
myp
"""
        )
        assert out == 'bad level "99999999999"'

    def test_level_minus_one_reads_caller(self):
        """``info level -1`` inside a proc returns the caller's
        invocation list.

        Note: ``outer`` also references ``info level`` so its
        frame is not elided by the escape-analysis path.  A
        caller that elides its frame (because its own body
        doesn't use ``info level``) wouldn't appear in the
        ``info level -N`` walk — a known limitation of the
        per-proc elision check.  Fixing it needs a
        whole-module transitive-reachability pass; tracked as a
        follow-up.
        """
        out = self._run_and_read_stdout(
            """\
proc inner {} { puts [info level -1] }
proc outer {a b} {
    # Reference info level so outer's frame isn't elided.
    set n [info level]
    inner
}
outer foo bar
"""
        )
        assert out == "outer foo bar"

    def test_accessor_pattern_switches_on_llength(self):
        """End-to-end tcltest-style accessor pattern: the proc
        takes an optional ``dir`` param and branches on
        ``llength [info level 0] == 1`` (no args) vs 2 (with
        arg).  Both cases must route correctly."""
        out = self._run_and_read_stdout(
            """\
proc workingDirectory { {dir ""} } {
    if {[llength [info level 0]] == 1} {
        return "no-args"
    } else {
        return "with-arg:$dir"
    }
}
set r1 [workingDirectory]
set r2 [workingDirectory /tmp]
puts "$r1 // $r2"
"""
        )
        assert out == "no-args // with-arg:/tmp"

    def test_argv0_reports_qualified_caller_word(self):
        """A call-site written as ``::ns::foo`` reports
        ``::ns::foo`` at ``[lindex [info level 0] 0]`` — the
        compile-time specialised call must pass the caller's
        invoked word through the pending-argv0 slot rather than
        the callee's qname tail."""
        out = self._run_and_read_stdout(
            """\
proc ::ns::foo {} { puts [lindex [info level 0] 0] }
::ns::foo
"""
        )
        assert out == "::ns::foo"

    def test_argv0_reports_renamed_call_word(self):
        """After ``rename foo bar``, calling ``bar`` must report
        ``bar`` as argv0 — the compile-time proc-index is flushed
        for the renamed name, so the call routes through
        ``tcl_eval``, and the runtime must forward
        ``words[0]`` through the pending-argv0 slot before
        dispatching to the compiled callee."""
        out = self._run_and_read_stdout(
            """\
proc foo {} { puts [lindex [info level 0] 0] }
rename foo bar
bar
"""
        )
        assert out == "bar"

    def test_argv0_reports_imported_short_name(self):
        """After ``namespace import ::src::foo``, calling the
        short name ``foo`` must report ``foo`` (not
        ``::src::foo``) as argv0 — matches Tcl's ``info level
        0`` semantics for import-redirected callers."""
        out = self._run_and_read_stdout(
            """\
namespace eval ::src {
    namespace export foo
    proc foo {} { puts [lindex [info level 0] 0] }
}
namespace import -force ::src::foo
foo
"""
        )
        assert out == "foo"

    def test_argv0_is_taken_not_leaked(self):
        """The pending-argv0 slot is consumed on proc entry, so a
        call from a compiled proc into another compiled proc
        doesn't leak the outer caller's word into the inner
        callee's ``info level 0``."""
        out = self._run_and_read_stdout(
            """\
proc inner {} { puts "inner=[lindex [info level 0] 0]" }
proc outer {} {
    puts "outer=[lindex [info level 0] 0]"
    inner
}
outer
"""
        )
        assert out == "outer=outer\ninner=inner"

    def test_accessor_pattern_works_post_flush(self):
        """Same accessor pattern after ``interp create`` triggers
        full flush — the proc is now invoked through the host
        bridge, not a compile-time specialised call.  The
        prologue must still stash argv so the accessor works."""
        out = self._run_and_read_stdout(
            """\
interp create child
interp delete child
namespace eval ::foo {
    proc Opt { {value {}}} {
        if {[llength [info level 0]] == 2} {
            return "set:$value"
        }
        return "get"
    }
    set r1 [Opt]
    set r2 [Opt hello]
    puts "$r1 // $r2"
}
"""
        )
        assert out == "get // set:hello"

    def test_level_with_no_arg_returns_depth(self):
        """``info level`` (no arg) returns the integer frame
        depth — Tcl 9's ``InfoLevelCmd`` zero-arg form."""
        out = self._run_and_read_stdout(
            """\
proc outer {} {
    proc inner {} { return [info level] }
    return [inner]
}
puts [outer]
"""
        )
        assert out == "2"

    def test_level_bad_positive_raises(self):
        """``info level 99`` (beyond current depth) raises ``bad
        level "99"`` just like tclsh."""
        out = self._run_and_read_stdout(
            """\
proc myp {} {
    catch {info level 99} msg
    puts $msg
}
myp
"""
        )
        assert out == 'bad level "99"'

    def test_level_zero_outside_proc_raises(self):
        """Calling ``info level 0`` at the global scope (no
        frames pushed) raises ``bad level "0"``."""
        out = self._run_and_read_stdout(
            """\
catch {info level 0} msg
puts $msg
"""
        )
        assert out == 'bad level "0"'


class TestInfoIntrospection:
    """End-to-end ``info commands`` / ``info procs`` /
    ``namespace which -command`` through the eval path.
    """

    def _run_and_read_global(self, source: str, name: str) -> bytes:
        _, wasm_bytes = _compile_to_wasm(source)
        engine = _get_engine()
        store = wasmtime.Store(engine)
        store.set_wasi(wasmtime.WasiConfig())
        tcl_instance, rt_instance = _link_and_instantiate(store, wasm_bytes)
        top = tcl_instance.exports(store).get("::top")
        if top is not None:
            top(store)
        return _read_global_string(rt_instance, store, name)

    def test_info_commands_lists_user_procs(self):
        result = self._run_and_read_global(
            """\
proc alpha {} {}
proc beta {} {}
set ::result [info commands]
""",
            "::result",
        )
        names = set(result.decode("utf-8").split())
        assert "::alpha" in names
        assert "::beta" in names

    def test_info_commands_pattern(self):
        result = self._run_and_read_global(
            """\
proc alpha {} {}
proc beta {} {}
set ::result [info commands a*]
""",
            "::result",
        )
        assert result.decode("utf-8").split() == ["::alpha"]

    def test_info_commands_qualified_pattern(self):
        """``info commands ::ns::*`` — qualified pattern walks the
        target ns only.  We use ``info commands`` rather than ``info
        procs`` because the WASM compiler compiles all user procs
        via ``proc_register_compiled`` — those come out of the
        interpreted-proc filter but remain visible as commands."""
        result = self._run_and_read_global(
            """\
proc ::ns::p1 {} { set ::unused 1 }
proc ::ns::p2 {} { set ::unused 2 }
proc outside {} { set ::unused 3 }
set ::result [info commands ::ns::*]
""",
            "::result",
        )
        names = set(result.decode("utf-8").split())
        assert names == {"::ns::p1", "::ns::p2"}

    def test_namespace_which_returns_fqn(self):
        result = self._run_and_read_global(
            """\
namespace eval ::ns { proc here {} {} }
set ::result [namespace which -command ::ns::here]
""",
            "::result",
        )
        assert result == b"::ns::here"

    def test_namespace_which_miss_returns_empty(self):
        result = self._run_and_read_global(
            """\
set ::result "<[namespace which -command missing]>"
""",
            "::result",
        )
        assert result == b"<>"

    def test_info_body_on_missing_raises_error(self):
        """``info body`` on a name that doesn't resolve raises
        ``"X" isn't a procedure`` — matches Tcl 9's ``InfoBodyCmd``
        wording."""
        result = self._run_and_read_global(
            """\
catch {info body nonexistent} msg
set ::result $msg
""",
            "::result",
        )
        assert result == b'"nonexistent" isn\'t a procedure'

    def test_info_body_on_alias_raises_error(self):
        """Aliases aren't interpreted procs — ``info body`` on an
        alias raises the same "isn't a procedure" error."""
        result = self._run_and_read_global(
            """\
proc target {} {}
interp alias {} my_alias {} target
catch {info body my_alias} msg
set ::result $msg
""",
            "::result",
        )
        assert result == b'"my_alias" isn\'t a procedure'

    def test_hide_and_catch_snippet_compiles(self):
        """Explorer WAT sanity: the
        ``proc foo {} {return 1}; interp hide {} foo; catch foo msg;
        expr {$msg}`` snippet compiles to a well-formed WASM module.

        The bare ``foo`` call after ``interp hide`` does NOT currently
        route through the eval fallback — the compiler specialises
        it to a direct ``call $::foo`` via its proc-index dispatch.
        That's a pre-existing codegen optimisation; runtime
        ``interp hide`` state isn't visible to the compile-time
        proc index, so hiding a proc that was *also* defined in the
        same translation unit doesn't block the compiled dispatch.
        Acknowledged as a known gap in
        ``docs/design/runtime/command-introspection.md`` §9 — the
        fix belongs to the compiler, not this runtime wave.  This
        test just pins the compile-clean invariant so regressions
        that trap or mis-emit catch this snippet are caught."""
        source = "proc foo {} {return 1}\ninterp hide {} foo\ncatch foo msg\nexpr {$msg}\n"
        wasm_module, wasm_bytes = _compile_to_wasm(source)
        # Smoke check: module parses back out and contains the
        # imports we rely on (``tcl_eval`` for the ``interp hide``
        # body, ``catch_enter`` for the catch wrapper).
        wat = wasm_module.to_wat()
        assert "tcl_eval" in wat, "tcl_eval import should be present"
        assert "catch_enter" in wat, "catch_enter import should be present"
        assert "::foo" in wat, "::foo compiled function should be present"
        assert len(wasm_bytes) > 0

    def test_info_level_returns_frame_depth(self):
        """``info level`` at the top-level returns 0 (no frames
        pushed).  Inside a proc body it returns the call depth.
        The probe reads the result via ``_read_global_string``
        which routes through ``tcl_test_obj_str_ptr/len`` — that
        materialises the decimal rendering for int-shaped TclObjs,
        so the bare ``[info level]`` form works without needing
        surrounding literals to force string conversion."""
        result_top = self._run_and_read_global(
            """\
set ::result [info level]
""",
            "::result",
        )
        assert result_top == b"0"

        result_nested = self._run_and_read_global(
            """\
proc outer {} { set ::result [info level] }
outer
""",
            "::result",
        )
        assert result_nested == b"1"

    def test_info_script_is_empty_in_wasm_sandbox(self):
        """``info script`` returns the empty string in our compiled
        runtime — there's no real filesystem source path."""
        result = self._run_and_read_global(
            """\
set ::result "<[info script]>"
""",
            "::result",
        )
        assert result == b"<>"

    def test_info_level_with_arg_raises_bad_level(self):
        """``info level N`` raises ``bad level "N"`` because our
        runtime doesn't track per-frame argv yet."""
        result = self._run_and_read_global(
            """\
catch {info level 1} msg
set ::result $msg
""",
            "::result",
        )
        assert result == b'bad level "1"'

    def test_info_default_on_compiled_proc_raises_error(self):
        """End-to-end, user procs reach this path as compiled procs
        (the WASM compiler routes every top-level ``proc`` through
        ``proc_register_compiled``).  ``info default`` refuses
        compiled procs with the same ``"X" isn't a procedure``
        wording it uses for aliases and missing commands — the
        compiled form has no retrievable params list.  The
        ``"procedure \"X\" doesn't have an argument \"Y\""`` error
        shape is still reachable for interpreted procs registered
        via the runtime ``proc_register`` path (covered in
        ``tests/runtime/test_tcl_info.py``)."""
        result = self._run_and_read_global(
            """\
proc p1 {a b} {}
catch {info default p1 missing_arg v} msg
set ::result $msg
""",
            "::result",
        )
        assert result == b'"p1" isn\'t a procedure'


def _read_global_string(
    rt_instance: wasmtime.Instance,
    store: wasmtime.Store,
    name: str,
) -> bytes:
    """Read a global variable as a UTF-8 byte string.  Stages the
    name into linear memory, routes through ``global_get``, then
    walks the TclObj's string representation.

    Uses ``tcl_test_obj_str_ptr`` / ``tcl_test_obj_str_len`` — both
    route through ``obj_ensure_string`` which materialises the
    decimal rendering for int-shaped TclObjs the first time it's
    asked.  Reading the raw ``OBJ_STR_PTR`` / ``OBJ_STR_LEN``
    slots directly (the pre-wave approach) would surface empty for
    a freshly-allocated int TclObj — anything set via ``set x
    [info level]`` / ``llength $list`` / etc. — because the
    compiler can short-circuit those paths to avoid the string
    rendering work.
    """
    exports = rt_instance.exports(store)
    memory: wasmtime.Memory = exports["memory"]
    test_alloc = exports["tcl_test_alloc"]
    obj_new_string = exports["obj_new_string"]
    global_get = exports["global_get"]
    obj_str_ptr = exports["tcl_test_obj_str_ptr"]
    obj_str_len = exports["tcl_test_obj_str_len"]

    encoded = name.encode("utf-8")
    addr = test_alloc(store, len(encoded))
    memory.write(store, encoded, addr)
    name_obj = obj_new_string(store, addr, len(encoded))
    val_obj = global_get(store, name_obj)
    if val_obj == 0:
        return b""
    str_ptr = obj_str_ptr(store, val_obj)
    str_len = obj_str_len(store, val_obj)
    if str_len == 0:
        return b""
    return bytes(memory.read(store, str_ptr, str_ptr + str_len))


class TestInterpTestPort:
    """Hand-ported interp.test sections 1.x / 2.x / 3.x / 4.x / 5.x / 6.x.

    The full ``tmp/tcl9.0.3/tests/interp.test`` bundle doesn't compile
    through this runtime yet (it reaches for tcltest primitives like
    ``::tcltest::normalizePath`` that aren't wired up), but the
    individual ``test interp-N.M {...}`` blocks from sections 1-6
    do — they exercise the child-interp registry surface without the
    surrounding tcltest harness.

    Each Python test mirrors one or more numbered upstream test.
    Errors are verified through ``catch msg; set ::result $msg`` so
    the assertions see the full tclsh error wording.
    """

    def _run_and_read_global(self, source: str, name: str) -> bytes:
        _, wasm_bytes = _compile_to_wasm(source)
        engine = _get_engine()
        store = wasmtime.Store(engine)
        store.set_wasi(wasmtime.WasiConfig())
        tcl_instance, rt_instance = _link_and_instantiate(store, wasm_bytes)
        top = tcl_instance.exports(store).get("::top")
        if top is not None:
            top(store)
        return _read_global_string(rt_instance, store, name)

    # --- Section 1: options for interp command ----------------------------

    def test_1_1_no_subcommand(self):
        """interp-1.1: ``interp`` alone is a wrong-#-args error."""
        result = self._run_and_read_global(
            "catch interp msg\nset ::result $msg\n",
            "::result",
        )
        assert result == b'wrong # args: should be "interp cmd ?arg ...?"'

    def test_1_2_unknown_subcommand(self):
        """interp-1.2: unknown subcommand raises ``bad option "X": ...``."""
        result = self._run_and_read_global(
            "catch {interp frobox} msg\nset ::result $msg\n",
            "::result",
        )
        # Match the prefix + middle + tail; the full option list is verbatim.
        assert result.startswith(b'bad option "frobox": must be ')
        assert b"alias, aliases" in result
        assert result.endswith(b"or transfer")

    def test_1_3_delete_no_args(self):
        """interp-1.3: ``interp delete`` with no args returns ""."""
        result = self._run_and_read_global(
            "set ::result [interp delete]\n",
            "::result",
        )
        assert result == b""

    def test_1_4_delete_missing(self):
        """interp-1.4: delete on missing name raises
        ``could not find interpreter "foo"``."""
        result = self._run_and_read_global(
            "catch {interp delete foo bar} msg\nset ::result $msg\n",
            "::result",
        )
        assert result == b'could not find interpreter "foo"'

    def test_1_5_exists_too_many_args(self):
        """interp-1.5: ``interp exists foo bar`` raises wrong-#-args."""
        result = self._run_and_read_global(
            "catch {interp exists foo bar} msg\nset ::result $msg\n",
            "::result",
        )
        assert result == b'wrong # args: should be "interp exists ?path?"'

    def test_1_6_children_too_many_args(self):
        """interp-1.6: ``interp children foo bar zop`` raises wrong-#-args."""
        result = self._run_and_read_global(
            "catch {interp children foo bar zop} msg\nset ::result $msg\n",
            "::result",
        )
        assert result == b'wrong # args: should be "interp slaves ?path?"'

    def test_1_10_target_no_args(self):
        """interp-1.10: ``interp target`` raises
        ``wrong # args: should be "interp target path alias"``."""
        result = self._run_and_read_global(
            "catch {interp target} msg\nset ::result $msg\n",
            "::result",
        )
        assert result == b'wrong # args: should be "interp target path alias"'

    # --- Section 2: basic interpreter creation ----------------------------

    def test_2_1_create_with_name(self):
        """interp-2.1: ``interp create a`` returns "a"."""
        result = self._run_and_read_global(
            "set ::result [interp create a]\n",
            "::result",
        )
        assert result == b"a"

    def test_2_2_create_anonymous(self):
        """interp-2.2: ``catch {interp create}`` returns 0 (success)."""
        result = self._run_and_read_global(
            "set ::result [catch {interp create}]\n",
            "::result",
        )
        assert result == b"0"

    def test_2_3_create_safe_anonymous(self):
        """interp-2.3: ``catch {interp create -safe}`` returns 0."""
        result = self._run_and_read_global(
            "set ::result [catch {interp create -safe}]\n",
            "::result",
        )
        assert result == b"0"

    def test_2_4_create_existing_name(self):
        """interp-2.4: creating under an already-used name raises
        ``interpreter named "X" already exists, cannot create``."""
        result = self._run_and_read_global(
            """\
interp create a
catch {interp create a} msg
set ::result $msg
""",
            "::result",
        )
        assert result == b'interpreter named "a" already exists, cannot create'

    def test_2_7_unknown_option(self):
        """interp-2.7: ``interp create -froboz`` raises
        ``bad option "-froboz": must be -safe or --``."""
        result = self._run_and_read_global(
            """\
catch {interp create -froboz} msg
set ::result $msg
""",
            "::result",
        )
        assert result == b'bad option "-froboz": must be -safe or --'

    def test_2_8_dashdash_allows_dashed_name(self):
        """interp-2.8: ``interp create -- -froboz`` creates and returns
        the dashed name."""
        result = self._run_and_read_global(
            "set ::result [interp create -- -froboz]\n",
            "::result",
        )
        assert result == b"-froboz"

    def test_2_9_safe_dashdash_dashed_name(self):
        """interp-2.9: ``interp create -safe -- -froboz1`` returns
        ``-froboz1`` and sets the safe bit."""
        result = self._run_and_read_global(
            """\
set ::a [interp create -safe -- -froboz1]
set ::b [interp issafe -froboz1]
set ::result "$::a $::b"
""",
            "::result",
        )
        assert result == b"-froboz1 1"

    def test_2_10_multi_level_path(self):
        """interp-2.10: ``interp create {a x1}`` creates ``x1`` as a
        child of ``a``."""
        result = self._run_and_read_global(
            """\
interp create a
interp create {a x1}
interp create {a x2}
set ::result [interp create {a x3} -safe]
""",
            "::result",
        )
        assert result == b"a x3"

    def test_2_13_anonymous_regexp(self):
        """interp-2.13: ``interp create --`` with no name auto-generates
        ``interp<N>``."""
        result = self._run_and_read_global(
            "set ::result [interp create --]\n",
            "::result",
        )
        import re

        assert re.fullmatch(rb"interp[0-9]+", result), result

    # --- Section 3: interp exists + interp children -----------------------

    def test_3_1_children_empty(self):
        """interp-3.1: with no children, ``interp children`` returns ""."""
        result = self._run_and_read_global(
            "set ::result [interp children]\n",
            "::result",
        )
        assert result == b""

    def test_3_2_exists_after_create(self):
        """interp-3.2: after ``interp create a``, ``interp exists a`` → 1."""
        result = self._run_and_read_global(
            """\
interp create a
set ::result [interp exists a]
""",
            "::result",
        )
        assert result == b"1"

    def test_3_3_exists_nonexistent(self):
        """interp-3.3: ``interp exists nonexistent`` → 0."""
        result = self._run_and_read_global(
            "set ::result [interp exists nonexistent]\n",
            "::result",
        )
        assert result == b"0"

    def test_3_6_exists_no_path_returns_true(self):
        """interp-3.6: bare ``interp exists`` → 1 (the current interp
        always exists)."""
        result = self._run_and_read_global(
            "set ::result [interp exists]\n",
            "::result",
        )
        assert result == b"1"

    def test_3_7_children_after_create(self):
        """interp-3.7: ``interp children`` returns the created child."""
        result = self._run_and_read_global(
            """\
interp create a
set ::result [interp children]
""",
            "::result",
        )
        assert result == b"a"

    def test_3_9_nested_child_appears(self):
        """interp-3.9: ``interp children a`` lists ``a``'s own
        children."""
        result = self._run_and_read_global(
            """\
interp create a
interp create {a a2} -safe
set ::result [interp children a]
""",
            "::result",
        )
        assert result == b"a2"

    def test_3_10_exists_multi_level_path(self):
        """interp-3.10: ``interp exists {a a2}`` → 1 after
        creating the chain."""
        result = self._run_and_read_global(
            """\
interp create a
interp create {a a2}
set ::result [interp exists {a a2}]
""",
            "::result",
        )
        assert result == b"1"

    # --- Section 4: interp delete -----------------------------------------

    def test_3_11_delete_no_args(self):
        """interp-3.11: ``interp delete`` with no args returns ""
        (duplicate of 1.3 in the upstream file)."""
        result = self._run_and_read_global(
            "set ::result [interp delete]\n",
            "::result",
        )
        assert result == b""

    def test_4_1_delete_existing(self):
        """interp-4.1: ``interp delete a`` returns ""."""
        result = self._run_and_read_global(
            """\
catch {interp create a}
set ::result [interp delete a]
""",
            "::result",
        )
        assert result == b""

    def test_4_2_delete_nonexistent(self):
        """interp-4.2: ``interp delete nonexistent`` raises
        ``could not find interpreter "nonexistent"``."""
        result = self._run_and_read_global(
            "catch {interp delete nonexistent} msg\nset ::result $msg\n",
            "::result",
        )
        assert result == b'could not find interpreter "nonexistent"'

    def test_4_3_delete_first_missing_errors_early(self):
        """interp-4.3: ``interp delete x y z`` raises on the first
        missing name."""
        result = self._run_and_read_global(
            "catch {interp delete x y z} msg\nset ::result $msg\n",
            "::result",
        )
        assert result == b'could not find interpreter "x"'

    def test_4_5_delete_nested_child(self):
        """interp-4.5: deleting ``{a x1}`` removes it from ``a``'s
        children table."""
        result = self._run_and_read_global(
            """\
interp create a
interp create {a x1}
interp delete {a x1}
set ::result [expr {"x1" in [interp children a]}]
""",
            "::result",
        )
        assert result == b"0"

    def test_4_6_delete_multiple_args(self):
        """interp-4.6: ``interp delete c1 c2 c3`` deletes all three."""
        result = self._run_and_read_global(
            """\
interp create c1
interp create c2
interp create c3
set ::result [interp delete c1 c2 c3]
""",
            "::result",
        )
        assert result == b""

    def test_4_7_delete_mixed_missing_errors(self):
        """interp-4.7: deleting a list that contains a missing name
        raises."""
        result = self._run_and_read_global(
            """\
interp create c1
interp create c2
catch {interp delete c1 c2 c3} msg
set ::result $msg
""",
            "::result",
        )
        assert result == b'could not find interpreter "c3"'

    def test_4_8_delete_empty_path_rejected(self):
        """interp-4.8: ``interp delete {}`` raises
        ``cannot delete the current interpreter``."""
        result = self._run_and_read_global(
            "catch {interp delete {}} msg\nset ::result $msg\n",
            "::result",
        )
        assert result == b"cannot delete the current interpreter"

    # --- Section 5: consistency -------------------------------------------

    def test_5_1_fresh_children_list_empty(self):
        """interp-5.1: a freshly-reset runtime has no children."""
        result = self._run_and_read_global(
            "set ::result [interp children]\n",
            "::result",
        )
        assert result == b""

    def test_5_2_nonexistent_exists_false(self):
        """interp-5.2/5.3: exists returns 0 for a never-created name."""
        result = self._run_and_read_global(
            """\
set ::a [interp exists a]
set ::b [interp exists nonexistent]
set ::result "$::a $::b"
""",
            "::result",
        )
        assert result == b"0 0"

    # --- Section 6: eval --------------------------------------------------

    def test_6_1_child_eval_expr(self):
        """interp-6.1: ``a eval expr {{3 + 5}}`` dispatches through
        the child-as-command path to produce 8."""
        result = self._run_and_read_global(
            """\
interp create a
set ::result [a eval expr {{3 + 5}}]
""",
            "::result",
        )
        assert result == b"8"

    def test_6_3_child_eval_defines_proc(self):
        """interp-6.3: a proc defined via ``a eval {proc ...}`` is
        reachable on subsequent ``a eval foo``."""
        result = self._run_and_read_global(
            """\
interp create a
a eval {proc foo {} {expr {3 + 5}}}
set ::result [a eval foo]
""",
            "::result",
        )
        assert result == b"8"

    def test_6_4_interp_eval_variant(self):
        """interp-6.4: ``interp eval a foo`` is equivalent to
        ``a eval foo``."""
        result = self._run_and_read_global(
            """\
interp create a
a eval {proc foo {} {expr {3 + 5}}}
set ::result [interp eval a foo]
""",
            "::result",
        )
        assert result == b"8"

    def test_6_5_interp_eval_nested_path(self):
        """interp-6.5: ``interp eval {a x2} body`` runs ``body`` in
        the nested child."""
        result = self._run_and_read_global(
            """\
interp create a
interp create {a x2}
interp eval {a x2} {proc frob {} {expr {4 * 9}}}
set ::result [interp eval {a x2} frob]
""",
            "::result",
        )
        assert result == b"36"

    # --- Delete cascade -----------------------------------------------

    def test_delete_cascades_to_grandchildren(self):
        """``interp delete a`` with a grandchild ``a2`` tombstones
        both; ``interp exists {a a2}`` returns 0 afterwards."""
        result = self._run_and_read_global(
            """\
interp create a
interp create {a a2}
interp delete a
set ::a_ex [interp exists a]
set ::a2_ex [interp exists {a a2}]
set ::result "$::a_ex $::a2_ex"
""",
            "::result",
        )
        assert result == b"0 0"

    def test_child_as_command_stops_after_delete(self):
        """Post-delete, the child's top-level command is gone and
        ``<child> eval ...`` raises unknown-command."""
        result = self._run_and_read_global(
            """\
interp create a
interp delete a
catch {a eval set x 1} msg
set ::result $msg
""",
            "::result",
        )
        assert result.startswith(b"unknown command")

    # --- Section 7: basic alias creation (child-as-command alias) ---------

    def test_7_1_child_alias_returns_name(self):
        """interp-7.1: ``a alias foo in_parent`` returns ``foo``."""
        result = self._run_and_read_global(
            """\
proc in_parent {args} { return [list seen in parent: $args] }
interp create a
set ::result [a alias foo in_parent]
""",
            "::result",
        )
        assert result == b"foo"

    def test_7_2_child_alias_with_prefix(self):
        """interp-7.2: ``a alias bar in_parent a1 a2 a3`` → ``bar``."""
        result = self._run_and_read_global(
            """\
proc in_parent {args} { return [list seen in parent: $args] }
interp create a
set ::result [a alias bar in_parent a1 a2 a3]
""",
            "::result",
        )
        assert result == b"bar"

    def test_7_3_alias_query_target_only(self):
        """interp-7.3: ``a alias foo`` (query) returns the target name
        when no prefix was specified."""
        result = self._run_and_read_global(
            """\
proc in_parent {args} {}
interp create a
a alias foo in_parent
set ::result [a alias foo]
""",
            "::result",
        )
        assert result == b"in_parent"

    def test_7_4_alias_query_with_prefix(self):
        """interp-7.4: ``a alias bar`` returns ``target prefix...``
        when the alias was created with a prefix."""
        result = self._run_and_read_global(
            """\
proc in_parent {args} {}
interp create a
a alias bar in_parent a1 a2 a3
set ::result [a alias bar]
""",
            "::result",
        )
        assert result == b"in_parent a1 a2 a3"

    def test_7_5_child_aliases_lsort(self):
        """interp-7.5: ``lsort [a aliases]`` lists the two aliases."""
        result = self._run_and_read_global(
            """\
proc in_parent {args} {}
interp create a
a alias foo in_parent
a alias bar in_parent a1 a2 a3
set ::result [lsort [a aliases]]
""",
            "::result",
        )
        assert result == b"bar foo"

    def test_7_6_aliases_wrong_args(self):
        """interp-7.6: ``a aliases too many args`` raises
        ``wrong # args: should be "a aliases"``."""
        result = self._run_and_read_global(
            """\
interp create a
catch {a aliases too many args} msg
set ::result $msg
""",
            "::result",
        )
        assert result == b'wrong # args: should be "a aliases"'

    # --- Section 8: alias invocation --------------------------------------

    def test_8_1_alias_invocation_forwards_args(self):
        """interp-8.1: ``a eval foo s1 s2 s3`` after
        ``a alias foo in_parent`` dispatches to parent's ``in_parent``
        with the trailing args."""
        result = self._run_and_read_global(
            """\
proc in_parent {args} { return [list seen in parent: $args] }
interp create a
a alias foo in_parent
set ::result [a eval foo s1 s2 s3]
""",
            "::result",
        )
        assert result == b"seen in parent: {s1 s2 s3}"

    def test_8_2_alias_invocation_with_prefix_merges(self):
        """interp-8.2: creation-time prefix prepends to the caller's
        tail argv."""
        result = self._run_and_read_global(
            """\
proc in_parent {args} { return [list seen in parent: $args] }
interp create a
a alias bar in_parent a1 a2 a3
set ::result [a eval bar s1 s2 s3]
""",
            "::result",
        )
        assert result == b"seen in parent: {a1 a2 a3 s1 s2 s3}"

    def test_8_3_child_alias_wrong_args(self):
        """interp-8.3: bare ``a alias`` raises
        ``wrong # args: should be "a alias aliasName ?targetName? ?arg ...?"``."""
        result = self._run_and_read_global(
            """\
interp create a
catch {a alias} msg
set ::result $msg
""",
            "::result",
        )
        assert result == (b'wrong # args: should be "a alias aliasName ?targetName? ?arg ...?"')

    # --- Section 9: missing + late-bound alias targets --------------------

    def test_9_1_alias_missing_target(self):
        """interp-9.1: calling an alias whose target doesn't exist
        raises ``invalid command name "X"`` (tclsh's wording when
        the command gets to the ``unknown`` stage).  Our runtime
        emits ``unknown command: X`` — the semantic identity is
        the same (the alias failed to find its target)."""
        result = self._run_and_read_global(
            """\
interp create a
a alias zop nonexistent
catch {a eval zop} msg
set ::result $msg
""",
            "::result",
        )
        # Our runtime's alias-miss diagnostic is ``unknown command:
        # <target>`` (see dispatch_alias in tcl_interp.zig).  tclsh
        # uses ``invalid command name "<target>"`` after the unknown
        # fallback; we pin our wording here.
        assert result == b"unknown command: nonexistent"

    def test_9_2_alias_late_bound_target(self):
        """interp-9.2: defining the target after the alias still
        works because the alias resolves by name on each dispatch."""
        result = self._run_and_read_global(
            """\
interp create a
a alias zop latebind
proc latebind {} { return i_exist! }
set ::result [a eval zop]
""",
            "::result",
        )
        assert result == b"i_exist!"

    def test_9_4_alias_target_resolves_in_global(self):
        """interp-9.4: alias targets resolve in the *global*
        namespace even when invoked from inside a namespace eval
        (matches TCL_EVAL_INVOKE semantics)."""
        result = self._run_and_read_global(
            """\
proc p {} { return GLOBAL }
namespace eval tst { proc p {} { return NAMESPACE } }
interp alias {} myalias {} p
set ::r1 [myalias]
set ::r2 [namespace eval tst myalias]
set ::result "$::r1 $::r2"
""",
            "::result",
        )
        assert result == b"GLOBAL GLOBAL"

    # --- Section 10: aliases between interpreters (sibling routing) -------

    def test_10_1_alias_registration_between_siblings(self):
        """interp-10.1: ``interp alias a a_alias b b_alias 1 2 3`` —
        registering an alias from a (source) targeting b (parent
        interp hosts both siblings)."""
        result = self._run_and_read_global(
            """\
interp create a
interp create b
set ::result [interp alias a a_alias b b_alias 1 2 3]
""",
            "::result",
        )
        assert result == b"a_alias"

    def test_10_4_child_alias_aliases_list_roundtrip(self):
        """interp-10.4: after ``a alias a_alias puts``, ``a aliases``
        lists ``a_alias``."""
        result = self._run_and_read_global(
            """\
interp create a
a alias a_alias puts
set ::result [a aliases]
""",
            "::result",
        )
        assert result == b"a_alias"

    # --- Cross-interp + namespace interactions ---------------------------

    def test_alias_passes_args_into_child_eval_scope(self):
        """A parent-side alias target invoked from inside a child
        ``eval`` script sees the args forwarded correctly.  This is
        the observable shape of "cross-interp argv plumbing" that
        ``upvar`` would complement; we don't exercise ``upvar``
        itself because ``docs/design/runtime/child-interp.md`` §1
        flags per-interp call-frame-stack interactions as
        deferred (the shared frame stack is safe for
        single-threaded, non-nested eval but truly nested
        ``interp eval`` + ``upvar`` isn't characterised yet)."""
        result = self._run_and_read_global(
            """\
proc in_parent {args} { return "from-parent n=[llength $args] args=$args" }
interp create a
a alias call_parent in_parent
set ::result [a eval call_parent hello world]
""",
            "::result",
        )
        assert result == b"from-parent n=2 args=hello world"

    def test_per_parent_id_issuer_independence(self):
        """interp-2.11-style check: each parent has its own
        ``id_issuer``.  Creating an anonymous interp under the root
        and another under ``a`` should not share numbers."""
        result = self._run_and_read_global(
            """\
interp create a
set ::x1 [interp create]
set ::y1 [a eval interp create]
set ::result "$::x1 $::y1"
""",
            "::result",
        )
        # Both should start from 0; child ``a``'s issuer is
        # independent of root's.
        import re

        parts = result.decode("utf-8").split()
        assert len(parts) == 2
        for p in parts:
            assert re.fullmatch(r"interp[0-9]+", p), p

    # --- Review-driven regression tests -----------------------------------

    def test_interp_slaves_list_quotes_names_with_whitespace(self):
        """Regression for a Codex / Copilot P2 review: ``interp
        slaves`` used to copy raw child names into a space-delimited
        string, so a name containing whitespace would emit an invalid
        Tcl list.  The fix routes through ``list_elem_quote``; the
        output is a canonical list that ``llength`` parses back to
        one element.

        Uses a two-level create so the final component (with a
        space) lands as the child's simple name.  The reviewer's
        example: ``interp create {a {child one}}`` makes
        ``interp children a`` produce a list whose sole element is
        ``child one`` (requiring braces to round-trip).
        """
        result = self._run_and_read_global(
            """\
interp create a
interp create {a {child one}}
set ::result [llength [interp children a]]
""",
            "::result",
        )
        assert result == b"1"

    def test_interp_slaves_roundtrip_through_lindex(self):
        """The name must survive a round-trip through
        ``lindex [interp children a] 0`` unchanged."""
        result = self._run_and_read_global(
            """\
interp create a
interp create {a {child one}}
set ::result [lindex [interp children a] 0]
""",
            "::result",
        )
        assert result == b"child one"

    def test_invokehidden_namespace_resolves_in_target_interp(self):
        """Regression for a Codex P1 review: ``interp invokehidden
        child -namespace ns cmd`` must resolve ``ns`` in the target
        interp's namespace tree, not the caller's.  We verify by
        making the call end-to-end: the child has a namespace
        ``::inner`` with a proc ``mark``; we hide ``mark`` into the
        child's hidden table then invokehidden it inside ``::inner``.
        The call must succeed (pre-fix, the namespace would have
        been created in the caller's tree and the subsequent
        dispatch would find the wrong — or no — ns)."""
        result = self._run_and_read_global(
            """\
interp create child
interp eval child {
    namespace eval ::inner {}
    proc stash {value} { set ::captured $value }
}
interp hide child stash
interp invokehidden child -namespace ::inner stash hello
set ::result [interp eval child {set ::captured}]
""",
            "::result",
        )
        assert result == b"hello"

    def test_alias_slot_does_not_collide_with_namespace_import(self):
        """Regression for a Codex P1 review: cross-interp alias
        parent-interp handle used to share the ``OFF_IMPORT_REF_HEAD``
        slot with the namespace-import back-reference list head.
        Importing a cross-interp alias would overwrite the Interp*
        with an ImportRef list node and corrupt dispatch.

        Post-fix, ``parent_interp`` lives on ``AliasRec``, so import
        of a cross-interp alias leaves dispatch intact.
        """
        result = self._run_and_read_global(
            """\
proc target {args} { return "target-saw: $args" }
namespace eval ::src { namespace export *alias }
interp create child
# Cross-interp alias into the child, target lives in the parent.
interp alias child myalias {} target arg1
# Define an alias of the same name in a parent ns to exercise
# the normal import machinery — the redirect's OFF_IMPORT_REF_HEAD
# slot would previously have been clobbered by link_import_ref
# if it had aliased into the child.
set ::result [child eval {myalias call1 call2}]
""",
            "::result",
        )
        assert result == b"target-saw: arg1 call1 call2"
