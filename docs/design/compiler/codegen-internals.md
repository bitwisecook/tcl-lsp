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

`CodegenCtx` retains the full ingress `LexerConfig` when the module supplies
it. Code that decodes literal words, normalised spellings, or expression
source reads the grammar fields from this held configuration; it does not
reconstruct them from a dialect name. If no ingress configuration was
provided, the context derives a grammar from its dialect or registry profile.
The full configuration matters because related grammar facts (numbers,
escapes, braced variable names, and expression parsing) must describe the
same source interpretation.

Keep source interpretation separate from invocation and execution policy.
The context carries an invocation dialect and other selected target policies;
a Native call candidate has further independent gates for original invocation
selection, effective arguments, and currentness. Holding the source lexer
configuration does not establish that a Native operation is admissible or
that a backend implements every dialect feature. Support and conformance are
bounded by the selected operation and the runtime path actually exercised.
See [dialect-profile-model.md](../registry/dialect-profile-model.md) for
context selection.

`braced_var` resolves where a `${…}` variable name ends: `Tcl_ParseVarName`
took the *first* close brace through 8.6 and balances nested braces from 9.0.
The rule matters at codegen time because of the **normalised-word round
trip**: the segmenter re-spells a `Var` token as source-like text and codegen
decodes that spelling back, so encoder and decoder must agree about the
release. The invariant is that every consumer resolves the form through the
single shared owner, `tcl_lexer::braced_var_name_end`, rather than scanning
for `}` itself — two decoders applying two releases' rules to one encoding
give answers inverted at both 8.x and 9.x, not merely wrong. Consumers code
against its `BracedVarEnd` enum so `Unterminated` stays an explicit outcome.

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
