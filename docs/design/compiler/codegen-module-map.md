# KCS: codegen module map

## Goal

Keep code generation behaviour stable while reducing review and maintenance cost by splitting mixed concerns into focused modules.  The shared compiler front-end (parse → IR → CFG → SSA → lowering) feeds two back-ends that live side-by-side under `compiler/codegen/`.

## Current split

`compiler/codegen/` is a thin doc-only parent; it re-exports nothing so neither back-end is privileged.  Callers import a back-end explicitly.

### Bytecode back-end — `compiler/codegen/bytecode/`

- `__init__.py` — bytecode public API (`codegen_function`, `codegen_module`, `FunctionAsm`, `Instruction`, `ModuleAsm`, `Op`, `format_*`).
- `_emitter.py` — emission flow + mixin integration.
- `opcodes.py` — opcode enum/metadata and expression op maps.
- `layout.py` — jump-size optimisation and label/offset layout.
- `format.py` — disassembly text rendering.
- `_types.py`, `_statements.py`, `_expressions.py`, `_values.py`, `_control_flow.py`, `_cmd_subst.py`, `_helpers.py`, `_peephole.py` — emitter mixins and shared types.
- `bytecoded/` — per-command bytecode emit hooks.

### WASM back-end — `compiler/codegen/wasm/`

- `__init__.py` — WASM public API (`wasm_codegen_module`, `wasm_codegen_function`, `WasmModule`, `WasmFunction`).
- `link.py` — whole-program linker (`wasm_link`, `wasm_link_sources`, `wasm_link_bundled`, `merge_ir_modules`).
- `_emitter/`, `_ir.py`, `_encoding.py`, `_imports.py`, `_parsing.py`, `_scan.py`, `_bundle.py`, `extensions.py`, `_ownership.py` — WASM emitter internals.

## Migration guidance

1. Prefer adding new opcode metadata in `bytecode/opcodes.py`.
2. Keep offset math and jump shrinking in `bytecode/layout.py`.
3. Keep disassembly string/rendering changes in `bytecode/format.py`.
4. Use each back-end's `__init__.py` for high-level emission flow and public API wiring.
5. Import a back-end explicitly (`from compiler.codegen.bytecode import …` / `from compiler.codegen.wasm import …`); do not add re-exports to the parent `compiler/codegen/__init__.py`.

## Related files

- `compiler/codegen/__init__.py`
- `compiler/codegen/bytecode/__init__.py`
- `compiler/codegen/bytecode/opcodes.py`
- `compiler/codegen/bytecode/layout.py`
- `compiler/codegen/bytecode/format.py`
- `compiler/codegen/wasm/__init__.py`
- `compiler/codegen/wasm/link.py`
