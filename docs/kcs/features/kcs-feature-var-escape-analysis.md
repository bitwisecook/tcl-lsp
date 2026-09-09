# KCS: feature — Var-escape analysis

> **Audience:** Contributor
> **Type:** Functionality

## Summary

Proves whether a procedure variable can stay local to compiled code or must stay visible by name in a Tcl runtime frame.

## Applies to

codegen

## How to use

There is nothing to switch on and no CLI verb: the analysis runs inside the
compiler and its results reach you through the optimisations they enable.

A variable is tagged `Local` only when the compiler can prove that `upvar`,
`uplevel`, dynamic `eval`, and dynamic variable names cannot observe it. Any
uncertainty raises the result to `Frame`. Dynamic, malformed, or unmodelled
constructs degrade to `Frame`; they never make an optimisation eligible.

The implementation lives in `rust/tcl-compiler/src/var_escape/`. It produces a
`ProcEscapeSummary` carrying per-name and per-[static single
assignment](../../GLOSSARY.md#ssa) tags, typed barriers, source ranges,
interprocedural `upvar` sources, and the conservative predicates
`safe_to_inline`, `safe_to_dce`, and `safe_for_frame_elision`. The inliner
consumes the registry-aware IR analysis; a separate `CompilationUnit` entry
runs the flow-sensitive control-flow-graph and static-single-assignment
analysis for consumers that need versioned facts.

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
