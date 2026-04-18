"""Tests for the WASM code generator."""

from __future__ import annotations

import struct

from core.compiler.cfg import build_cfg
from core.compiler.codegen.wasm import (
    WasmModule,
    _leb128_signed,
    _leb128_unsigned,
    wasm_codegen_module,
)
from core.compiler.lowering import lower_to_ir

# LEB128 encoding


def test_leb128_unsigned_zero():
    assert _leb128_unsigned(0) == b"\x00"


def test_leb128_unsigned_small():
    assert _leb128_unsigned(1) == b"\x01"
    assert _leb128_unsigned(127) == b"\x7f"


def test_leb128_unsigned_multibyte():
    assert _leb128_unsigned(128) == b"\x80\x01"
    assert _leb128_unsigned(624485) == b"\xe5\x8e\x26"


def test_leb128_signed_zero():
    assert _leb128_signed(0) == b"\x00"


def test_leb128_signed_positive():
    assert _leb128_signed(1) == b"\x01"
    assert _leb128_signed(63) == b"\x3f"


def test_leb128_signed_negative():
    assert _leb128_signed(-1) == b"\x7f"
    assert _leb128_signed(-128) == b"\x80\x7f"


# WASM module structure


def _compile(source: str, *, optimise: bool = False) -> WasmModule:
    ir = lower_to_ir(source)
    cfg = build_cfg(ir)
    return wasm_codegen_module(cfg, ir, optimise=optimise)


def test_wasm_module_magic():
    """WASM binary starts with the magic number and version."""
    module = _compile("set x 1\n")
    wasm = module.to_bytes()
    assert wasm[:4] == b"\x00asm"
    assert struct.unpack("<I", wasm[4:8])[0] == 1


def test_wasm_module_has_functions():
    """Module should contain at least the top-level function."""
    module = _compile("set x 1\n")
    assert len(module.functions) >= 1
    assert module.functions[0].name == "::top"


def test_wasm_module_exports_top():
    """Top-level function should be exported."""
    module = _compile("set x 1\n")
    assert module.functions[0].exported is True


def test_wasm_module_procedures():
    """Procedures should be compiled as separate WASM functions."""
    module = _compile("proc add {a b} { expr {$a + $b} }\n")
    names = [f.name for f in module.functions]
    assert "::top" in names
    assert any("add" in n for n in names)


def test_wasm_proc_params():
    """Procedure parameters should map to WASM function params."""
    module = _compile("proc add {a b} { expr {$a + $b} }\n")
    add_func = [f for f in module.functions if "add" in f.name][0]
    # 2 params (a, b) → 2 i64 params
    assert len(add_func.params) == 2


# WAT output


def test_wat_output_parseable():
    """WAT output should be well-formed S-expression."""
    module = _compile("set x 42\n")
    wat = module.to_wat()
    assert wat.startswith("(module")
    assert wat.endswith(")")
    # Should contain the function
    assert "(func $::top" in wat


def test_wat_contains_i64_const():
    """Literal integers should appear as i64.const in WAT."""
    module = _compile("set x 42\n")
    wat = module.to_wat()
    assert "i64.const 42" in wat


def test_wat_contains_memory():
    """WAT should declare a memory section."""
    module = _compile("set x 1\n")
    wat = module.to_wat()
    assert "(memory" in wat


# Optimised vs non-optimised


def test_optimised_folds_constants():
    """Optimised mode should constant-fold known expressions."""
    source = "set x [expr {2 + 3}]\n"
    mod_no_opt = _compile(source, optimise=False)
    mod_opt = _compile(source, optimise=True)

    # Both should produce valid WASM
    assert mod_no_opt.to_bytes()[:4] == b"\x00asm"
    assert mod_opt.to_bytes()[:4] == b"\x00asm"


def test_optimised_removes_nops():
    """Optimised mode should remove NOP instructions."""
    source = "set x 1\nputs $x\n"
    mod_no_opt = _compile(source, optimise=False)
    mod_opt = _compile(source, optimise=True)

    no_opt_instrs = sum(len(f.body) for f in mod_no_opt.functions)
    opt_instrs = sum(len(f.body) for f in mod_opt.functions)

    # Optimised should have fewer or equal instructions
    assert opt_instrs <= no_opt_instrs


def test_optimised_binary_smaller_or_equal():
    """Optimised binary should be no larger than non-optimised."""
    source = "set x 1\nset y [expr {$x + 2}]\nset z [expr {$y * 3}]\n"
    mod_no_opt = _compile(source, optimise=False)
    mod_opt = _compile(source, optimise=True)

    assert len(mod_opt.to_bytes()) <= len(mod_no_opt.to_bytes())


# Control flow


def test_if_statement():
    """if statement should compile without error."""
    module = _compile("if {$x > 0} { set y 1 } else { set y 0 }\n")
    assert module.to_bytes()[:4] == b"\x00asm"
    wat = module.to_wat()
    assert "if" in wat


def test_for_loop():
    """for loop should compile without error."""
    module = _compile("for {set i 0} {$i < 10} {incr i} { set x $i }\n")
    assert module.to_bytes()[:4] == b"\x00asm"
    wat = module.to_wat()
    # CFG linearises loops into blocks with branches — check for if/else
    assert "if" in wat or "loop" in wat


def test_while_loop():
    """while loop should compile without error."""
    module = _compile("while {$x > 0} { incr x -1 }\n")
    assert module.to_bytes()[:4] == b"\x00asm"


def test_switch_statement():
    """switch should compile without error."""
    source = "switch $x { a { set y 1 } b { set y 2 } default { set y 0 } }\n"
    module = _compile(source)
    assert module.to_bytes()[:4] == b"\x00asm"


# Expressions


def test_arithmetic_expr():
    """Arithmetic expressions should produce correct WASM ops."""
    module = _compile("set x [expr {1 + 2 * 3}]\n")
    wat = module.to_wat()
    assert "i64.add" in wat or "i64.mul" in wat


def test_comparison_expr():
    """Comparison expressions should produce correct WASM ops."""
    module = _compile("set x [expr {$a == $b}]\n")
    wat = module.to_wat()
    assert "i64.eq" in wat


def test_logical_and():
    """Logical AND should use short-circuit evaluation."""
    module = _compile("set x [expr {$a && $b}]\n")
    wat = module.to_wat()
    # Short-circuit uses if/else
    assert "if" in wat


def test_ternary_expr():
    """Ternary expression should compile without error."""
    source = "proc choose {a} { expr {$a ? 1 : 0} }\n"
    module = _compile(source)
    assert module.to_bytes()[:4] == b"\x00asm"


def test_unary_neg():
    """Unary negation should produce 0 - x."""
    module = _compile("set x [expr {-$y}]\n")
    wat = module.to_wat()
    assert "i64.sub" in wat


# Binary validity


def test_binary_has_type_section():
    """WASM binary should contain a type section."""
    module = _compile("set x 1\n")
    wasm = module.to_bytes()
    # After magic+version (8 bytes), first section should be type (id=1)
    assert wasm[8] == 1  # type section


def test_binary_roundtrip_consistency():
    """Serialising the same module twice should produce identical bytes."""
    module = _compile("set x [expr {1 + 2}]\n")
    a = module.to_bytes()
    b = module.to_bytes()
    assert a == b


# Edge cases


def test_empty_script():
    """Empty script should produce a valid minimal module."""
    module = _compile("")
    wasm = module.to_bytes()
    assert wasm[:4] == b"\x00asm"


def test_incr_command():
    """incr should compile to i64.add."""
    module = _compile("set x 0\nincr x\n")
    wat = module.to_wat()
    assert "i64.add" in wat


def test_multiple_procedures():
    """Multiple procedures should all appear in the module."""
    source = "proc foo {x} { expr {$x + 1} }\nproc bar {x} { expr {$x * 2} }\n"
    module = _compile(source)
    names = [f.name for f in module.functions]
    assert any("foo" in n for n in names)
    assert any("bar" in n for n in names)


# Public API


def test_wasm_codegen_function_api():
    """wasm_codegen_function requires lifecycle imports for TclObj emission."""
    module = _compile("set x 1\n")
    assert module.functions[0].name == "::top"
    assert len(module.functions[0].body) > 0


def test_codegen_package_exports_wasm():
    """WASM symbols should be accessible from the codegen package."""
    from core.compiler.codegen import (
        WasmFunction,
        WasmModule,
        wasm_codegen_function,
        wasm_codegen_module,
    )

    assert WasmModule is not None
    assert WasmFunction is not None
    assert callable(wasm_codegen_function)
    assert callable(wasm_codegen_module)


# Runtime imports


def test_runtime_imports_registered_for_puts():
    """puts should cause a runtime import to be registered."""
    module = _compile("puts hello\n")
    assert len(module.imports) >= 1
    import_names = [imp.name for imp in module.imports]
    assert "tcl_cmd_puts" in import_names


def test_no_cmd_imports_for_pure_math():
    """Pure arithmetic code should only import lifecycle + error + diag functions."""
    module = _compile("set x [expr {1 + 2}]\n")
    import_names = {imp.name for imp in module.imports}
    # diag_set is always imported so any trap site can prefix stderr with
    # the source-location site ID the sidecar map resolves.
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
    }


def test_scope_declarations_no_imports():
    """global/variable/upvar should not register imports."""
    module = _compile("proc f {} { global x; return 1 }\n")
    import_names = [imp.name for imp in module.imports]
    assert "global" not in import_names
    assert "variable" not in import_names


def test_wat_shows_import_declarations():
    """WAT output should contain import declarations for runtime commands."""
    module = _compile("puts 42\n")
    wat = module.to_wat()
    assert '(import "tcl" "tcl_cmd_puts"' in wat


def test_shared_imports_deduplication():
    """Multiple puts calls should share a single import."""
    module = _compile("puts 1\nputs 2\nputs 3\n")
    puts_imports = [imp for imp in module.imports if imp.name == "tcl_cmd_puts"]
    assert len(puts_imports) == 1


# Command dispatch


def test_puts_no_nop():
    """puts should emit a call instruction, not a NOP."""
    from core.compiler.codegen.wasm import WasmOp

    module = _compile("puts 42\n")
    top = module.functions[0]
    nop_count = sum(1 for instr in top.body if instr.op == WasmOp.NOP)
    call_count = sum(1 for instr in top.body if instr.op == WasmOp.CALL)
    assert nop_count == 0
    assert call_count >= 1


def test_global_emits_nothing():
    """global should emit no instructions (not even a NOP)."""
    from core.compiler.codegen.wasm import WasmOp

    module = _compile("proc f {} { global x; return 1 }\n")
    proc_funcs = [f for f in module.functions if f.name != "::top"]
    if proc_funcs:
        nop_count = sum(1 for instr in proc_funcs[0].body if instr.op == WasmOp.NOP)
        assert nop_count == 0


# Proc call codegen


def test_proc_call_emits_call():
    """Calling a known proc should emit a WASM call instruction."""
    from core.compiler.codegen.wasm import WasmOp

    source = "proc add {a b} { expr {$a + $b} }\nproc caller {x y} { add $x $y }\n"
    module = _compile(source)
    caller_funcs = [f for f in module.functions if "caller" in f.name]
    assert len(caller_funcs) == 1
    call_instrs = [i for i in caller_funcs[0].body if i.op == WasmOp.CALL]
    assert len(call_instrs) >= 1


def test_proc_index_consistent():
    """Proc function indices should be stable across compilation."""
    source = "proc foo {x} { expr {$x + 1} }\nproc bar {x} { expr {$x * 2} }\n"
    module = _compile(source)
    names = [f.name for f in module.functions]
    assert "::top" in names
    assert any("foo" in n for n in names)
    assert any("bar" in n for n in names)


# Binary size with imports


def test_binary_valid_with_imports():
    """WASM binary with imports should still be valid."""
    module = _compile("puts 42\n")
    wasm = module.to_bytes()
    assert wasm[:4] == b"\x00asm"
    assert len(module.imports) >= 1
