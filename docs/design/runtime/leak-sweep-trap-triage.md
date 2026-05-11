# Leak-sweep trap triage (29/96 files)

The committed leak baseline (`tests/baselines/wasm_leak_baseline.json`)
records 29 of 96 in-scope tcltest files trapping during sweep
execution.  All 29 are *correctness* gaps in the runtime / codegen,
not leaks: every trapping test exits cleanly when re-run with the
production (non-leakcheck) build, but inside the sweep harness the
test hits an `unreachable` instruction during evaluation.

This document clusters the 29 files by subsystem so future fixes
can target the highest-leverage categories first.

## Method

Each entry below is a stem (Tcl 9 test name) plus the subsystem it
exercises.  All 29 traps land in the runtime's `eval_script`,
`execute_parsed_command`, or `eval_proc_call_bucket` paths — i.e.
the *compiled* wasm dispatched to runtime eval, and the runtime
hit a path it can't currently handle.

The sweep harness (`scripts/dev/leak_sweep.py`) captures the wasmtime
backtrace as `trap_message` but does not currently decode the
runtime's `tcl trap: site=<id>` markers against the per-bundle
`DiagMap`.  Adding that decoding would let triage drill straight to
the failing source line; until then the categorisation below relies
on subsystem grouping.

## Clusters

### Parsing (3)

* `parse` — full Tcl-token parser; tests cover comment / brace /
  bracket edge cases the runtime parser likely doesn't model.
* `parseExpr` — expression parser corner cases.
* `subst` — `[subst]` builtin with `-novariables` / `-nocommands`
  / `-nobackslashes` flag combinations.

These tests exercise the parser through public-API entry points
(`Tcl_ParseCommand`, `Tcl_ParseBraces`, etc.) which the runtime
likely doesn't expose.  Lower priority — the compiler doesn't need
runtime parse APIs.

### List operations (3)

* `lseq` — Tcl 9's list-generating builtin.  Likely missing.
* `lrepeat` — list construction with repetition.
* `foreach` — only the multi-var-list edge cases trap; the bulk of
  `foreach` works in the compiled path.

`lseq` and `lrepeat` are concrete missing builtins — clear paths
forward.

### Strings + regex (4)

* `string` — `string is` / `string match` / `string compare`
  edge cases.
* `regexp`, `regexpComp`, `reg` — three flavours of regex tests.
  Runtime ships a vendored `tcl-regex` C lib; some advanced
  features (back-references, certain flags) likely unimplemented
  at the bridge layer.

### Expressions (3)

* `expr` — wide coverage of operators / type promotion.
* `expr-old` — legacy `expr` parser corner cases.  Specifically
  traps inside `obj_ensure_string` while reached from
  `tcl_cmd_lappend` — the only trap in the sweep that calls out
  a runtime function rather than a generic eval path.  Re-tested
  after the lappend rc fix (commit d4663566) and the dict cache
  (commit 892e6e02): trap **still present** with the same
  signature.  Likely a NULL / immediate-handle reaching
  `tcl_cmd_lappend` that `obj_ensure_string` can't handle.
  Tractable fix: add an immediate-handle / null guard at the
  top of `tcl_cmd_lappend`.
* `mathop` — `::tcl::mathop::*` namespaced operators.

### Control flow (3)

* `for` — likely a `[break]`/`[continue]` interaction with
  body-substitution.
* `switch` — pattern-list edge cases.
* `error` — `[error]` with `-errorcode` / `-errorinfo` chaining.

### Variables (4)

* `var` — array element ops, traces.
* `uplevel` — frame-walking by absolute / relative level.
* `namespace`, `namespace-old` — ns command with deep nesting.

`uplevel` and `namespace` likely need broader frame-traversal
support that the compiler currently approximates.

### Procs (3)

* `proc-old` — legacy `proc` argument-parsing corner cases.
* `info` — `info args`, `info body`, `info vars` family.
* `rename` — `[rename]` interaction with namespaces.

### Eval / dispatch (4)

* `compile` — Tcl 8 bytecode-compilation tests.  **Out of scope**:
  we don't compile to bytecode.
* `basic` — generic interp dispatch.
* `cmdAH` — letters A–H of legacy command coverage.
* `opt` — `[opt]` package shim (deprecated in Tcl 9).

### Object system (1)

* `ooNext2` — TclOO's `[next]` keyword in nested method dispatch.

### Interp (1)

* `safe-stock` — safe interpreter creation with stock policy.

## Suggested order of attack

1. **`expr-old`** — re-run the sweep first; the lappend rc fix may
   have already cleared this one.
2. **`lseq` / `lrepeat`** — concrete missing builtins, well-bounded
   fix per command.
3. **`subst`** flag combinations — the runtime already has a
   substitution engine; expand its flag support.
4. **`info`** family — straightforward getters; well-defined Tcl 9
   spec.
5. Parsing / regex / object / safe-interp clusters are larger
   projects; defer until the simpler clusters land.

## Prerequisite for deeper triage

Enrich `scripts/dev/leak_sweep.py` to:

1. Capture the trap site's WASI stderr alongside the wasmtime
   backtrace.
2. Resolve `tcl trap: site=<id>` markers against the bundle's
   `DiagMap` to produce `(test-file, line, command)` tuples.
3. Surface those in `tests/baselines/wasm_leak_baseline.json` as
   a structured `trap_origin` field.

With that in place each entry becomes actionable without re-
running the bundle by hand.
