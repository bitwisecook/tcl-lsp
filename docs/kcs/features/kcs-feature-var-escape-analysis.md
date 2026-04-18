# KCS: feature — Var-escape analysis

> **Audience:** Contributor
> **Type:** Functionality

## Summary

Compile-time analysis that decides whether each Tcl local variable stays
in a WASM local slot (fast) or spills to the runtime frame (visible by
name to `uplevel`, `upvar`, `eval`, and dynamic `set $name`).

## Applies to

tcl-lsp CLI, codegen, dataflow, analyser

## Question

What does the var-escape analysis do, and when does it fire?

## How to use

The analysis runs automatically inside the WASM code generator
(`core/compiler/codegen/wasm/`). Every time a module is compiled,
`analyse_var_escape` walks the IR for each procedure, tags each
variable as `LOCAL` or `FRAME`, and publishes a
[`ProcEscapeSummary`](../../design/compiler/var-escape-analysis.md) that
the emitter consults at `_intern_local`, `_emit_var_read_obj`,
`_emit_var_write_obj`, and `_emit_frame_sync` sites.

There is no user-facing flag to toggle the analysis — it's always on.
Contributors extending it should read the design doc for the lattice,
the transfer-function table, and the `info` subcommand allow-list.

## Options

- None at the user level.
- Internally, `analyse_var_escape(source, interprocedural=False)`
  returns the raw per-proc summary for tests and debugging.

## Example

### Before — procs paid for sync even when they couldn't escape

```tcl
proc add {a b} {
    set sum [expr {$a + $b}]
    return $sum
}
```

Before this analysis, if any fallback path ran anywhere in the module,
the compiled `add` emitted `tcl_local_set` for `a`, `b`, and `sum`
before every interpreter call. None of those can actually be observed
by name, so the work was wasted.

### After — pristine procs need no frame sync

```
# analyse_var_escape(::add) → ProcEscapeSummary(
#     tags={}, dynamic_barrier=False, frame_needed=False, …)
```

`frame_needed=False` tells the emitter the proc has nothing to sync.
The WASM codegen keeps `a`, `b`, and `sum` in WASM local slots
exclusively.

### Example with escape — only the aliased var spills

```tcl
proc swap {a b} {
    upvar 1 $a la
    upvar 1 $b lb
    set tmp $la
    set la $lb
    set lb $tmp
}
```

- `la` and `lb` are `FRAME` — they're `upvar` targets. Reads and
  writes route through `tcl_local_get` / `tcl_local_set` so the
  interpreter's alias resolution sees the current value.
- `tmp` is `LOCAL` — it never leaves the WASM local slot.

Interprocedurally, a caller whose local name matches a callee's
upvar source set is also escalated to `FRAME`. See
`tests/test_var_escape.py::TestInterproceduralUpvar` for the concrete
test cases.

## Related

- [KCS feature index](README.md)
- [Var-escape analysis (design doc)](../../design/compiler/var-escape-analysis.md)
- [Glossary: escape tag](../../GLOSSARY.md#escape-tag)
- [`tests/test_var_escape.py`](../../../tests/test_var_escape.py) — unit tests for each rule
