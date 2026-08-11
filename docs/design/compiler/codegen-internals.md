# Bytecode code-generation internals

This note describes the current Rust bytecode emitter. The public compiler
front end produces a `CompilationUnit`; bytecode consumers use the emitter in
`rust/tcl-compiler/src/codegen/` and the artefact types in `rust/tcl-bytecode/`.

## Pipeline

1. `rust/tcl-compiler/src/codegen/emitter/generate.rs` drives module and
   function emission from the retained CFG/SSA facts.
2. `emitter/ordering.rs` linearises reachable CFG blocks and handles the
   loop-specific ordering required by the bytecode layout.
3. `statements.rs`, `expressions.rs`, `values.rs`, and `control_flow.rs` lower
   statements, expressions, values, and terminators through `CodegenCtx`.
4. `emitter/terminator.rs`, `loop_blocks.rs`, and `try_blocks.rs` emit branch,
   loop, and exception-region structure.
5. `tcl-bytecode::layout::resolve_layout` resolves symbolic labels and jump
   offsets. The resulting `FunctionAsm` retains instructions, literals, local
   variables, source spans, and error regions.
6. `rust/tcl-compiler/src/codegen/peephole.rs` applies bytecode-local rewrites
   after emission. Formatting and disassembly are provided by
   `rust/tcl-bytecode/src/format.rs`.

`CodegenCtx` owns the `LocalVarTable`, literal table, source-span context, and
the instruction buffer. It records labels symbolically until the shared
layout pass can compute final offsets. Synthetic instructions may have no
source span; this is represented explicitly rather than guessed from nearby
source.

## Boundaries

The bytecode VM executes the resulting `ModuleAsm`; it does not select a
compiler backend. Tcl command semantics remain registry-owned, and generic
invocation is emitted when a command shape has no safe bytecode hook. WASM is a
separate target under `rust/tcl-compiler/src/codegen/wasm/` and has its sole
public entry point in `compile_wasm`.

## Related

- [Code-generation module map](codegen-module-map.md)
- [Bytecode boundary](bytecode-boundary.md)
- [Compiler pipeline overview](compiler-pipeline-overview.md)
