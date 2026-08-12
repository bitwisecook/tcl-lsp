# Bytecode boundary (what to lift earlier)

Which knowledge belongs inside codegen and which is better modelled earlier as
a shared fact, so that editor features can use it without depending on opcode
layout.

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

- `compiler/codegen/`
- `rust/tcl-compiler/src/lowering/`
- `rust/tcl-compiler/src/compilation_unit.rs`
