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

`CodegenCtx` carries **one** `LexerGrammar` (`expr_grammar`), resolved once
in `codegen_module` from the unit's dialect name through
`grammar_of_dialect_name`, and every grammar fact it reads — `numbers`
(`NumberSyntax`), `escapes` (`EscapeSyntax`), `braced_var`
(`BracedVarStyle`), `word_rules` (`WordValueRules`), and the grammar
`parse_compile_expr` re-parses expression text under — is a field of that one
value. They exist because codegen decodes text — literal words, normalised
word spellings, expression source — and the grammar that text obeys is
dialect-dependent. That they are one value is the invariant: before it, a
named compile emitted numerals under a grammar resolved from the name and
re-parsed `expr` under the profile's, and for `tk` the two disagreed about
`010` inside a single compile. See `dialect-profile-model.md` §2.5 for how
the name reaches codegen and why it is the document's own.

**Scope, stated deliberately.** The grammar codegen *decodes under* is the
document's resolved dialect today — a JimTcl unit's literals, escapes and
word splitting are read as Jim wrote them. What codegen *emits*, and what the
bytecode VM and `runtime/rust` execute, is **Tcl 9 semantics only**: the
projected profile a non-Tcl dialect resolves to carries
`vm_runtime_version = V9_0`; Jim's `$(…)` lowers to the same `AssignExpr`
a bracketed `expr` does in `set` and `return` (so the emitted bytecode is
Tcl 9's expression evaluation of the body) and stays an opaque dynamic word
elsewhere; and Jim's own list, dict and `expr` semantics are not modelled
by any pass. That is the intended state,
not an omission: the backends target Tcl 9, and dialect-aware emission and
execution is an **eventual** — the readiness requirement met now is that
codegen consumes only the point-derived grammar and profile, never a dialect
name or a per-consumer table, so a future dialect-keyed backend has one value
to key on and nothing to unpick.

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
