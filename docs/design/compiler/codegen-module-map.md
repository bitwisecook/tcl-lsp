# KCS: codegen module map (Phase 3)

## Goal

Keep bytecode generation behaviour stable while reducing review and maintenance cost by splitting mixed concerns into focused modules.

## Current split

- `server/compiler/codegen/__init__.py` — orchestration + emitter integration.
- `server/compiler/codegen/opcodes.py` — opcode enum/metadata and expression op maps.
- `server/compiler/codegen/layout.py` — jump-size optimisation and label/offset layout.
- `server/compiler/codegen/format.py` — disassembly text rendering.

## Migration guidance

1. Prefer adding new opcode metadata in `opcodes.py`.
2. Keep offset math and jump shrinking in `layout.py`.
3. Keep disassembly string/rendering changes in `format.py`.
4. Use `__init__.py` for high-level emission flow and public API wiring.

## Related files

- `server/compiler/codegen/__init__.py`
- `server/compiler/codegen/opcodes.py`
- `server/compiler/codegen/layout.py`
- `server/compiler/codegen/format.py`
