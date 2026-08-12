# KCS: Var-escape analysis

> **Audience:** Contributor
> **Type:** Functionality

## Applies to

tcl-lsp CLI, compiler Explorer, analyser, codegen

## What does var-escape analysis prove?

Var-escape analysis decides whether a procedure variable can remain local to
compiled code or must be visible by name in a Tcl runtime frame. A variable is
tagged `Local` only when the compiler can prove that features such as `upvar`,
`uplevel`, dynamic `eval`, and dynamic variable names cannot observe it. Any
uncertainty raises the result to `Frame`.

The Rust implementation lives in `rust/tcl-compiler/src/var_escape/`. It
produces a `ProcEscapeSummary` containing per-name and per-[static single
assignment](../../GLOSSARY.md#ssa) tags, typed barriers, source ranges,
interprocedural `upvar` sources, and conservative predicates such as
`safe_to_inline`, `safe_to_dce`, and `safe_for_frame_elision`.

The production inliner consumes the registry-aware IR analysis. A separate
`CompilationUnit` entry runs the flow-sensitive control-flow-graph and static
single-assignment analysis for consumers that need versioned facts. Neither is
an alternative WebAssembly compiler: `compile_wasm` remains the sole public
code-generation entry, and any future frame plan must consume these common
facts through that pipeline.

There is no user-facing switch. Dynamic, malformed, or unmodelled constructs
degrade to `Frame`; they never make an optimisation eligible by default.

## Example

```tcl
proc add {a b} {
    set sum [expr {$a + $b}]
    return $sum
}

proc copy_from_caller {name} {
    upvar 1 $name value
    return $value
}
```

`add` can be a pure leaf, so its variables remain `Local`. In
`copy_from_caller`, `value` is a frame-visible alias and the caller-side name
is propagated interprocedurally. If the source name or frame level is dynamic,
the analysis records a typed barrier and abstains from the narrower proof.

## Related

- [Var-escape analysis design](../../design/compiler/var-escape-analysis.md)
- [WASM code generation](../../design/compiler/wasm-codegen.md)
- [Glossary: escape tag](../../GLOSSARY.md#escape-tag)
