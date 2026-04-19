# KCS: Bytecode boundary (what to lift earlier)

## Symptom

A useful analysis fact exists only in `codegen.py`, limiting LSP features from using it.

## Context

Bytecode generation must preserve Tcl-compatible output, but editor tooling benefits from semantic facts before formatting/opcode layout concerns.

## Boundary guidance

Keep in codegen:

- opcode selection details,
- jump/label layout,
- disassembly formatting,
- strict reference-identity quirks.

Lift earlier when practical:

- invocation intent (intrinsic candidate vs generic call),
- substitution classification,
- side-effect/escape class hints,
- conversion pressure that informs shimmer/optimisation diagnostics.

## Refactor trigger

If a rule in codegen would improve diagnostics or quick-fix quality, model it as a shared pre-codegen fact and let codegen consume that fact.

## Related files

- `core/compiler/codegen.py`
- `core/compiler/lowering.py`
- `core/compiler/compilation_unit.py`
