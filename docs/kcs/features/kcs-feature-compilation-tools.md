# KCS: feature — Compilation Tools

> **Audience:** User
> **Type:** Functionality

## Summary

Disassemble bytecode, syntax-highlight source to ANSI or HTML, and compile to WebAssembly.

## Applies to

tcl-lsp CLI

## Question

How do I view bytecode output, produce highlighted HTML, or compile Tcl to WebAssembly?

## How to use

Three CLI verbs cover compilation inspection and export:

| Verb | What it does |
|------|-------------|
| `tcl dis` | Compile source and emit human-readable bytecode assembly (instruction sequences, jumps, stack operations). |
| `tcl highlight` | Tokenise source and emit syntax-highlighted output in ANSI (terminal) or HTML format. |
| `tcl compwasm` | Compile through IR and CFG lowering, then generate WebAssembly binary with optional WAT text output. |

## Example

```
$ tcl dis my_irule.tcl -o out.asm
$ tcl highlight my_irule.tcl --format html -o highlighted.html
$ tcl compwasm my_irule.tcl -o out.wasm --wat
```

The `dis` output shows the compiled instruction stream — useful for understanding what the compiler produces and verifying that optimisation passes fired correctly.

## Related

- [KCS feature index](README.md)
- [Compiler Explorer](kcs-feature-compiler-explorer.md) — interactive web panel for the same pipeline
- [Optimiser](kcs-feature-optimiser.md) — the passes that transform the IR before codegen
- [Glossary: Codegen](../../GLOSSARY.md#codegen)
