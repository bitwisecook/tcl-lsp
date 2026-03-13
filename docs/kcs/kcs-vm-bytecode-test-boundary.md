# KCS: VM/bytecode test boundary and identity expectations

## Symptom

Bytecode identity tests fail across Tcl versions, or disassembly comparisons appear nondeterministic after compiler/codegen changes.

## Operational context

The VM test boundary validates emitted assembly/disassembly against Tcl reference behaviour and versioned fixtures.

## Decision rules / contracts

1. Keep bytecode identity checks version-aware (8.4/8.5/8.6/9.x differences).
2. Codegen correctness changes must update expected fixtures explicitly.
3. Diagnostics/analysis contracts should not depend on backend formatting quirks.

## File-path anchors

- `core/compiler/codegen/` (package: `__init__.py`, `opcodes.py`, `layout.py`, `format.py`)
- `tests/bytecode_reference/`
- `scripts/capture_reference_bytecode.sh`

## Failure modes

- Opcode layout drift causing fixture mismatch without semantic breakage context.
- Reference fixture refresh mixing intended and unintended behaviour changes.
- Cross-version assumptions leaking into single-version assertions.

## Test anchors

- `tests/test_bytecode_identity.py`
- `tests/test_upstream_compiler.py`
- `tests/test_vm_basic_test.py`

## Discoverability

- [KCS index](README.md)
- [compiler bytecode boundary](compiler/kcs-bytecode-boundary.md)
- [compiler pipeline overview](compiler/kcs-compiler-pipeline-overview.md)
