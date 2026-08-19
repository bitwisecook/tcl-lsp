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

### Grammar facts

`CodegenCtx` also carries the target release's *grammar facts*, each threaded
from `IrModule::dialect`: `numbers` (`NumberSyntax`), `escapes`
(`EscapeSyntax`), `braced_var` (`BracedVarStyle`), and the `expr` half of the
same fact for re-parsed expression text. They exist because codegen decodes
text — literal words, normalised word spellings, expression source — and the
grammar that text obeys is release-dependent.

`braced_var` resolves where a `${…}` variable name ends: `Tcl_ParseVarName`
took the *first* close brace through 8.6 and balances nested braces from 9.0.
The rule matters at codegen time because of the **normalised-word round
trip**: the segmenter re-spells a `Var` token as source-like text and codegen
decodes that spelling back, so encoder and decoder must agree about the
release. The invariant is that every consumer resolves the form through the
single shared owner, `tcl_lexer::braced_var_name_end`, rather than scanning
for `}` itself — two decoders applying two different releases' rules to one
encoding is what made issue #1568 produce answers that were inverted at both
8.x and 9.x rather than merely wrong. Consumers code against its
`BracedVarEnd` enum so `Unterminated` stays an explicit outcome.

CFG lowering runs *before* the target release reaches codegen and so has no
dialect in hand. Where it must classify a `${…}` word anyway — the `switch`
subject fast path — it accepts under the permissive rule; see the header
comment on `cfg_lower::switch_subject_operand` for the program that proves
abstaining and pinning are both wrong there.

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
