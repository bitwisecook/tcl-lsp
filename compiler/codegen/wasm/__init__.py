"""WebAssembly (WASM) code generator for Tcl.

Translates CFG-based IR into valid WASM binary modules.  Supports two
output modes controlled by the ``optimise`` flag:

- **Non-optimised** (``optimise=False``): straightforward translation
  with stack-based variable access, no instruction folding.
- **Optimised** (``optimise=True``): constant folding, dead-code
  elimination, and local variable promotion.

Public API::

    wasm_codegen_module(cfg_module, ir_module, *, optimise=False) -> WasmModule
    wasm_codegen_function(cfg, params=(), *, optimise=False) -> WasmFunction

The resulting ``WasmModule`` can be serialised to a ``.wasm`` binary
via :meth:`WasmModule.to_bytes` or inspected via :meth:`WasmModule.to_wat`.

This package keeps its implementation in underscore-prefixed siblings
(``_emitter``, ``_encoding``, ``_ir``, ``_imports``, ``_parsing``,
``_scan``) plus the public ``api`` / ``proc_scan`` modules.  Import
those directly for internals; this ``__init__`` re-exports only the
stable surface.
"""

from __future__ import annotations

from ._ir import (
    DiagMap,
    DiagSite,
    SectionId,
    ValType,
    WasmData,
    WasmFunction,
    WasmImport,
    WasmInstruction,
    WasmModule,
    WasmOp,
)
from .api import wasm_codegen_function, wasm_codegen_module

__all__ = [
    "DiagMap",
    "DiagSite",
    "SectionId",
    "ValType",
    "WasmData",
    "WasmFunction",
    "WasmImport",
    "WasmInstruction",
    "WasmModule",
    "WasmOp",
    "wasm_codegen_function",
    "wasm_codegen_module",
]
