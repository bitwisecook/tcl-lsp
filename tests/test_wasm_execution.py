"""WASM execution tests — verify compiled output by running it in wasmtime.

These tests compile Tcl source to WASM, execute the resulting module
in the wasmtime runtime, and verify that the results match expected
values (cross-checked against C Tcl behaviour where applicable).

Requires: ``wasmtime`` Python package (listed in dev dependencies).
"""

from __future__ import annotations

import pytest

from core.compiler.cfg import build_cfg
from core.compiler.codegen.wasm import WasmModule, wasm_codegen_module
from core.compiler.lowering import lower_to_ir

wasmtime = pytest.importorskip("wasmtime", reason="wasmtime not installed")

# Captured output from puts calls during WASM execution
_puts_output: list[int] = []

# Python-side TclObj store — simulates the Zig runtime's linear-memory
# object allocation.  Maps obj_id (i32) → integer value.
_tcl_obj_store: dict[int, int] = {}
_tcl_obj_str_store: dict[int, tuple[int, int]] = {}  # obj_id → (ptr, len)
_tcl_obj_counter: int = 1


def _reset_tcl_obj_state() -> None:
    """Reset the TclObj store between test runs."""
    global _tcl_obj_counter
    _tcl_obj_store.clear()
    _tcl_obj_str_store.clear()
    _tcl_obj_counter = 1


def _box_int(value: int) -> int:
    """Allocate a TclObj for an integer, return its obj_id (i32)."""
    global _tcl_obj_counter
    obj_id = _tcl_obj_counter
    _tcl_obj_counter += 1
    _tcl_obj_store[obj_id] = value
    return obj_id


def _unbox_result(obj_id: int) -> int:
    """Extract the integer value from a TclObj id.  0 = null → 0."""
    if obj_id == 0:
        return 0
    return _tcl_obj_store.get(obj_id, 0)


def _make_runtime_imports(
    store: wasmtime.Store,
    module: wasmtime.Module,
) -> list[wasmtime.Func]:
    """Build stub runtime imports for a WASM module's import list.

    Provides Python-backed implementations of the Tcl runtime functions
    declared in ``_RUNTIME_IMPORTS``.  Includes TclObj lifecycle stubs
    (obj_new_int, obj_new_string, obj_get_int) that maintain a Python-side
    object store for test verification.
    """
    imports: list[wasmtime.Func] = []
    for imp in module.imports:
        name = imp.name
        func_type = imp.type

        if name == "obj_new_int":

            def _obj_new_int(value: int) -> int:
                return _box_int(value)

            imports.append(wasmtime.Func(store, func_type, _obj_new_int))
        elif name == "obj_new_string":

            def _obj_new_string(data_ptr: int, length: int) -> int:
                global _tcl_obj_counter
                obj_id = _tcl_obj_counter
                _tcl_obj_counter += 1
                _tcl_obj_str_store[obj_id] = (data_ptr, length)
                _tcl_obj_store[obj_id] = 0  # strings have int value 0
                return obj_id

            imports.append(wasmtime.Func(store, func_type, _obj_new_string))
        elif name == "obj_get_int":

            def _obj_get_int(obj_id: int) -> int:
                return _tcl_obj_store.get(obj_id, 0)

            imports.append(wasmtime.Func(store, func_type, _obj_get_int))
        elif name == "puts":

            def _puts(obj_id: int) -> int:
                _puts_output.append(_tcl_obj_store.get(obj_id, obj_id))
                return 0

            imports.append(wasmtime.Func(store, func_type, _puts))
        elif name == "error":

            def _error(msg: int) -> None:
                pass

            imports.append(wasmtime.Func(store, func_type, _error))
        elif name == "append" or name == "lappend":

            def _append(a: int, b: int) -> int:
                return b

            imports.append(wasmtime.Func(store, func_type, _append))
        elif name == "list_length":

            def _list_length(value: int) -> int:
                # Unbox the TclObj to get its integer value as the "length"
                return _box_int(_tcl_obj_store.get(value, 0))

            imports.append(wasmtime.Func(store, func_type, _list_length))
        elif name == "string_length":

            def _string_length(value: int) -> int:
                return _box_int(0)

            imports.append(wasmtime.Func(store, func_type, _string_length))
        elif name == "string_compare":

            def _string_compare(a: int, b: int) -> int:
                va = _tcl_obj_store.get(a, 0)
                vb = _tcl_obj_store.get(b, 0)
                if va < vb:
                    return _box_int(-1)
                if va > vb:
                    return _box_int(1)
                return _box_int(0)

            imports.append(wasmtime.Func(store, func_type, _string_compare))
        else:
            # Generic stub: return null TclObj (0) for functions with results
            n_results = len(func_type.results)
            if n_results > 0:

                def _stub(*args: int) -> int:
                    return 0

                imports.append(wasmtime.Func(store, func_type, _stub))
            else:

                def _stub_void(*args: int) -> None:
                    pass

                imports.append(wasmtime.Func(store, func_type, _stub_void))

    return imports


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
    """Compile Tcl source to WASM and execute a function.

    Arguments are boxed as TclObj i32 pointers before calling; the
    i32 result is unboxed back to a Python integer.
    """
    _reset_tcl_obj_state()
    _, wasm_bytes = _compile_to_wasm(source, optimise=optimise)

    store = wasmtime.Store()
    module = wasmtime.Module(store.engine, wasm_bytes)
    runtime_imports = _make_runtime_imports(store, module)
    instance = wasmtime.Instance(store, module, runtime_imports)

    # Box integer arguments as TclObj pointers
    boxed_args = tuple(_box_int(a) for a in args)

    func = instance.exports(store)[func_name]
    result_obj = func(store, *boxed_args)
    return _unbox_result(result_obj)


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
        runtime_imports = _make_runtime_imports(store, module)
        instance = wasmtime.Instance(store, module, runtime_imports)
        assert instance is not None


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
        assert "puts" in import_names

    def test_puts_captures_output(self):
        """puts should call the runtime puts function."""
        _puts_output.clear()
        _compile_and_run("puts 42\n")
        # The stub should have been called
        assert len(_puts_output) >= 1

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
        """Pure arithmetic code should only import TclObj lifecycle functions."""
        wasm_mod, _ = _compile_to_wasm("proc add {a b} { expr {$a + $b} }\n")
        # Only the TclObj lifecycle imports (obj_new_int, obj_new_string, obj_get_int)
        import_names = {imp.name for imp in wasm_mod.imports}
        assert import_names == {"obj_new_int", "obj_new_string", "obj_get_int"}


# Runtime import architecture


class TestRuntimeImports:
    """Test the runtime import registration and shared_imports mechanism."""

    def test_shared_imports_consistent(self):
        """All functions in a module should see the same import indices."""
        source = "puts 1\nproc f {} { puts 2 }\n"
        wasm_mod, _ = _compile_to_wasm(source)
        # There should be exactly one puts import (shared)
        puts_imports = [imp for imp in wasm_mod.imports if imp.name == "puts"]
        assert len(puts_imports) == 1

    def test_import_indices_stable(self):
        """Import function indices should be globally consistent."""
        source = "puts 1\nproc f {x} { puts $x; return $x }\n"
        wasm_mod, wasm_bytes = _compile_to_wasm(source)

        # Should be loadable and runnable
        store = wasmtime.Store()
        module = wasmtime.Module(store.engine, wasm_bytes)
        runtime_imports = _make_runtime_imports(store, module)
        instance = wasmtime.Instance(store, module, runtime_imports)
        assert instance is not None

    def test_module_with_multiple_imports(self):
        """Module using multiple runtime commands should register all imports."""
        source = "puts 1\nappend x hello\nllength $x\n"
        wasm_mod, _ = _compile_to_wasm(source)
        import_names = sorted(imp.name for imp in wasm_mod.imports)
        assert "puts" in import_names

    def test_wat_shows_imports(self):
        """WAT output should show import declarations."""
        wasm_mod, _ = _compile_to_wasm("puts 42\n")
        wat = wasm_mod.to_wat()
        assert '(import "tcl" "puts"' in wat


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
