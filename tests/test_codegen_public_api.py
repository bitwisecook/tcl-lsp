"""Public API compatibility checks for the codegen package split."""

from __future__ import annotations

from core.compiler.cfg import build_cfg
from core.compiler.codegen import (
    __all__ as codegen_all,
)
from core.compiler.codegen import (
    codegen_module,
    format_module_asm,
)
from core.compiler.lowering import lower_to_ir


def test_codegen_public_api_exports_expected_symbols():
    expected = {
        "Op",
        "Instruction",
        "FunctionAsm",
        "ModuleAsm",
        "codegen_function",
        "codegen_module",
        "format_function_asm",
        "format_module_asm",
    }
    assert expected.issubset(set(codegen_all))


def test_codegen_public_api_still_generates_disassembly_text():
    source = "set x 1\n"
    ir = lower_to_ir(source)
    cfg = build_cfg(ir)

    module = codegen_module(cfg, ir)
    text = format_module_asm(module)

    assert "ByteCode ::top" in text
    assert "done" in text
