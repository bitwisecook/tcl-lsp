# Proc argument trait inference

Trait inference decides how each parameter of a user-defined `proc` is used
inside its body. The result feeds the optimiser, shimmer analysis, taint
propagation, diagnostics, the LSP's hover and semantic tokens, and the spec
studio's registry importer ([command-spec-studio.md](command-spec-studio.md)).

## Traits

`tcl_compiler::analyser::types::ProcArgTrait`:

| Trait | Meaning | Example |
|---|---|---|
| `Eval` | evaluated as a script (`eval` / `uplevel` / `subst`) | `uplevel 1 $body` |
| `Body` | used as a loop or control body | `foreach item $list $body` |
| `VarWrite` | names a caller-frame variable the proc **writes** via `upvar` | `upvar 1 $varName l; set l 42` |
| `VarRead` | names a caller-frame variable the proc reads via `upvar` without writing | `upvar 1 $varName l; return $l` |
| `Expr` | evaluated as an expression | `if {$cond} {…}` |
| `LoopList` | the list argument of a `foreach` / `lmap` | `foreach i $collection {…}` |
| `DynamicNameLocal` | the parameter's **value** names a variable in the proc's **own** scope | `set $p 1`, `lassign $l $p` |
| `Command` | the parameter's **value** names a command — the command word of an invocation, or a registry/stub `CommandPrefix` callback argument | `$cmd a b` |

Two of these carry a distinction that is easy to get wrong and that consumers
depend on:

* **`DynamicNameLocal` is callee-local.** `VarWrite`/`VarRead` mean the
  parameter *aliases* a caller-frame variable, so passing a literal name at
  the call site consumes the caller's variable. `DynamicNameLocal` does not —
  `f x` does not consume the caller's `x`; the callee merely uses the string
  `x` to name one of its own locals. It is always emitted alongside `VarRead`
  (the parameter's string value *is* read), so a consumer asking "is this
  parameter used at all?" still sees it; the refinement matters only for
  caller-side dead-store and unused-variable suppression, which must skip a
  parameter that is `DynamicNameLocal` without also being a genuine
  `VarWrite`.
* **`Command` makes a literal at the call site a command reference**, which
  the call graph and semantic tokens resolve.

`ProcArgTrait::as_str` is the stable lower-case serialisation used on the
wire (MCP, explorer payloads); it is the only place the spellings are fixed.

## Two passes

**Shallow** — `infer_param_traits(params, body_source, env)` scans top-level
commands only. It is fast enough to run synchronously during typing, and
catches direct patterns where `$param` is an argument to a known command at
the outermost level of the body.

**Deep** — `infer_param_traits_deep(…)` additionally descends into braced body
arguments (`foreach` bodies, `if` bodies, `apply` lambdas, …). Given

```tcl
proc iterate {items body} {
    foreach item $items {
        uplevel 1 $body   ;# only the deep pass sees this
    }
}
```

the shallow pass reports `$items` as `LoopList`; the deep pass additionally
reports `$body` as `Eval`.

Recursion is bounded by `MAX_DEPTH` (8). The depth counter is threaded through
`infer_param_traits_deep_at_depth` rather than reset per entry point,
specifically so that an `apply` inside a body cannot restart the counter at
zero while the native call stack keeps growing.

**The deep pass is off by default.** `Analyser::deep_param_traits` is `false`
on a fresh analyser, and the proc handler runs the shallow pass alone unless
it is set; when it *is* set, the two results are unioned via `merge_traits`.
The shallow pass is fast enough for synchronous analysis and catches the
common patterns; the deep pass is intended for asynchronous use behind the
call-graph / symbol-graph / dataflow-graph / semantic-graph builders. Either
way the resulting map is stored on `ProcDef::param_traits`.

## The interprocedural sibling

`tcl_compiler::interprocedural` declares its **own** `ProcArgTrait` enum for a
different question — how a parameter's *value* flows (`Passthrough`,
`UsedInCondition`, `ForwardedToCallee`, `VarRead`, `VarWrite`, `Unused`). The
two are deliberately separate: the analyser's traits describe how an argument
is *interpreted* (script, expression, variable name), the interprocedural ones
describe *dataflow* through the body. Do not collapse them.

`ProcSummary::param_traits` holds the **interprocedural** enum, not this
module's. It is built by `interprocedural::finalise_param_traits` from
observations the summary builder makes while walking the lowered
`ir::Procedure` body — `infer_param_traits` is never called from
`interprocedural.rs`. `finalise_param_traits` adds `Passthrough` when the
proc's `return_passthrough_param` names the parameter, and `Unused` when no
observation fired at all, so a parameter's trait set is never empty.

## Consumers

| Consumer | Use |
|---|---|
| `Analyser` proc handling | shallow pass (plus the deep pass when `deep_param_traits` is set), stored on `ProcDef::param_traits` |
| `tcl-lsp-core`'s `caller_frame.rs` | shallow pass, to resolve caller-frame `upvar` targets |
| `interprocedural.rs` | its **own** dataflow trait enum over lowered `ir::Procedure` bodies, stored on `ProcSummary::param_traits` — not this module's traits |
| LSP hover / semantic tokens | annotate parameters; `Command`-trait arguments highlight as commands |
| Optimiser | whether a proc can be inlined or constant-folded |
| Taint engine | `Eval` and `Body` mark taint-sensitive parameters |
| Shimmer analysis | `LoopList` and `VarWrite` inform shimmer tracking |
| Spec studio importer | `arg_roles` and `traits` on an inferred draft ([command-spec-studio.md](command-spec-studio.md)) |

## Key files

| File | Role |
|---|---|
| `rust/tcl-compiler/src/analyser/param_traits.rs` | shallow + deep inference |
| `rust/tcl-compiler/src/analyser/types.rs` | `ProcArgTrait`, `ProcDef::param_traits` |
| `rust/tcl-compiler/src/interprocedural.rs` | the dataflow-side trait enum and `ProcSummary::param_traits` |
