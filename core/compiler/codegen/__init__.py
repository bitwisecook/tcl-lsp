"""Tcl VM bytecode assembly backend.

Takes a pre-SSA :class:`CFGModule` and emits assembly text matching the
format produced by ``tcl::unsupported::disassemble`` in Tcl 9.0.2.

Two-phase approach:
  1. Walk CFG blocks, emit :class:`Instruction` nodes with symbolic labels.
  2. Layout pass resolves labels → byte offsets, then format to text.

Public API::

    codegen_function(cfg, params=()) -> FunctionAsm
    codegen_module(cfg_module, ir_module) -> ModuleAsm
    format_function_asm(asm) -> str
    format_module_asm(module) -> str
"""

from __future__ import annotations

from ._emitter import codegen_function, codegen_module
from ._types import FunctionAsm, Instruction, LiteralTable, LocalVarTable, ModuleAsm
from .format import _esc, format_function_asm, format_module_asm
from .opcodes import Op
from .wasm import (  # noqa: F401
    WasmFunction,
    WasmModule,
    wasm_codegen_function,
    wasm_codegen_module,
)

__all__ = [
    "Op",
    "Instruction",
    "FunctionAsm",
    "ModuleAsm",
    "codegen_function",
    "codegen_module",
    "format_function_asm",
    "format_module_asm",
]
