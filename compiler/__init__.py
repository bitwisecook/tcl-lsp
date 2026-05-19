"""Tcl language pipeline: lexing, parsing, IR, lowering, passes, optimiser, codegen.

`compiler/` owns the path from Tcl source text to executable bytecode (and
WASM). It depends only on `shared/` and on data registered by `dialects/`;
it must not import from `analyser/`, `server/`, `tooling/`, or `ai/`.
"""
