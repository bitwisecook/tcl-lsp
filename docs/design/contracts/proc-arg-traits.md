# KCS: Proc argument trait inference

## Purpose

Proc argument trait inference determines how each parameter of a user-defined
proc is used inside its body.  This gives navigation, highlighting, spec
authoring, and interprocedural summaries structured knowledge about parameter
flow.

There are **two** distinct trait vocabularies, and they are not the same
enum.  The analyser-level `ProcArgTrait` describes *how a parameter is used
syntactically* inside one body; the interprocedural-summary `ProcArgTrait`
describes *what a call site can conclude* about the parameter.  Both live in
`tcl-compiler` but in different modules, and neither converts to the other.

## Analyser traits

`tcl_compiler::analyser::types::ProcArgTrait`:

| Trait | Meaning | Example pattern |
|-------|---------|-----------------|
| `Eval` | Argument is eval'd as a script | `eval $script`, `uplevel 1 $body` |
| `Body` | Argument is used as a loop/control body | `foreach item $list $body` |
| `VarWrite` | Argument names a caller-frame variable the proc writes | `upvar 1 $varName local; set local 42` |
| `VarRead` | Argument names a caller-frame variable the proc reads via `upvar` without writing | `upvar 1 $varName local; return $local` |
| `Expr` | Argument is evaluated as an expression | `if {$cond} {...}` |
| `LoopList` | Argument is used as the list in foreach/lmap | `foreach item $collection {...}` |
| `DynamicNameLocal` | The parameter's *value* names a variable in the proc's **own** scope | `set $p 1`, `lassign $l $p` |
| `Command` | The parameter's *value* is used as a command name or command prefix | `$cmd arg1 arg2` |

`DynamicNameLocal` is deliberately narrower than `VarWrite` / `VarRead`:
those imply the parameter aliases a *caller-frame* variable through `upvar`,
so passing a literal name at the call site consumes the caller's variable.
`DynamicNameLocal` is callee-local — `f x` does not touch the caller's `x`.
It is always emitted alongside `VarRead` (the parameter's string value *is*
read), so a consumer asking only "is this parameter used at all" still sees
it.

Each trait has a stable lower-case wire form (`"eval"`, `"var_write"`,
`"dynamic_name_local"`, …) that serialising consumers use rather than
re-implementing the mapping.

### `upvar` level is a separate fact

`VarWrite` / `VarRead` say the parameter names a variable through an
`upvar`, but not *which* frame the alias lands in — only `upvar 1` lands in
the caller's.  `ProcDef::caller_frame_params` carries that level fact, and
consumers that navigate to the caller's variable (the caller-frame bindings
behind go-to-definition and find-references) must check both.

## Interprocedural traits

`tcl_compiler::interprocedural::ProcArgTrait`, stored on `ProcSummary`:

| Trait | Meaning |
|-------|---------|
| `Passthrough` | The parameter's text is substituted into the return value unchanged |
| `UsedInCondition` | The parameter participates in a comparison that gates control flow |
| `ForwardedToCallee` | The parameter is forwarded to another procedure |
| `VarRead` | The parameter names a variable the proc reads via `upvar` (call-by-name) |
| `VarWrite` | The parameter names a variable the proc writes via `upvar` (call-by-name) |
| `Unused` | The parameter is never read |

Every parameter gets an entry: `Passthrough` is added when the proc's
return-passthrough parameter matches, and `Unused` is the fallback when no
observation fired.

## Analysis tiers

Both entry points take the parameter names, the body source, and a
`TraitScanEnv` bundling the dialect-aware `CommandRegistry`, the document's
`# tcl-lsp: stub` overlay, the dialect `LexerConfig`, and the document's
proven command-identity facts.  The registry and the overlay are what make
the scan generic: a body argument is recognised because the command's spec
(or the user's stub) declares that argument's role, never because the
scanner knows the command's name.

### Shallow (synchronous)

`infer_param_traits(params, body_source, env)` scans top-level commands only.
Fast enough for real-time analysis during typing.  Detects direct patterns
where `$param` appears as an argument to a known command at the outermost
level of the proc body.

### Deep (asynchronous)

`infer_param_traits_deep(params, body_source, env)` recursively descends into
braced body arguments (foreach bodies, if bodies, etc.) to find traits
that the shallow pass misses.  For example:

```tcl
proc iterate {items body} {
    foreach item $items {
        uplevel 1 $body   ;# deep pass catches Eval trait on $body
    }
}
```

The shallow pass only sees `$items` as `LoopList`.  The deep pass also
detects `$body` as `Eval` inside the braced foreach body.

Recursion is bounded by the public `MAX_DEPTH` (8) to prevent runaway
analysis on pathological input.  A lambda body reached through `apply`
continues at `depth + 1` rather than restarting at 0, so alternating
`if {…} { apply {x {…}} … }` nesting cannot reset the counter while the
native stack keeps growing.

Only *braced* body arguments are entered: a `$var` or `[cmd]` head is opaque,
and its `Eval` trait is already recorded by the top-level role scan.

### Merging results

`merge_traits(shallow, deep)` unions the results from both passes.  The
analyser runs the deep pass only when `deep_param_traits` is enabled — off
by default, because the shallow pass is fast enough for synchronous analysis
and catches the common patterns.  The merged map is stored on
`ProcDef::param_traits`.

## Integration points

| Consumer | How traits are used |
|----------|-------------------|
| Analyser proc handler | Runs the shallow pass (plus the deep pass when enabled) and stores the result on `ProcDef::param_traits` |
| `interprocedural.rs` | Builds its own `ProcSummary::param_traits` from the summary-level observations, not from the analyser enum |
| Semantic tokens | Highlights a literal at a call site as a variable write, a variable read, or a command, based on the callee's traits |
| Caller-frame bindings | `VarWrite` / `VarRead` plus `caller_frame_params` drive go-to-definition and find-references across an `upvar` boundary, including `my`-headed TclOO self-dispatch |
| Spec studio importer | Infers argument roles and command traits for a draft `CommandSpec` from how a real proc uses each parameter, recording the evidence |
| MCP tools | Serialise the analyser trait sets into the proc-analysis payload |
| LSP database | Persists the interprocedural `ProcSummary::param_traits` across sessions |

The interprocedural traits are computed for every summary but are currently
read only by the persistence layer and by tooling — the optimiser passes,
including unused-proc removal, deliberately ignore them.  Treat the table
above as the live consumer set rather than assuming any pass consumes a
trait it does not appear against.

## Files

| File | Purpose |
|------|---------|
| `rust/tcl-compiler/src/analyser/param_traits.rs` | Trait inference (shallow + deep), `TraitScanEnv`, `merge_traits`, `MAX_DEPTH`, caller-frame helpers |
| `rust/tcl-compiler/src/analyser/types.rs` | Analyser `ProcArgTrait`, `ProcDef::param_traits`, `ProcDef::caller_frame_params` |
| `rust/tcl-compiler/src/analyser/handlers.rs` | Proc handler — where the passes are invoked and merged |
| `rust/tcl-compiler/src/interprocedural.rs` | Summary-level `ProcArgTrait` and `ProcSummary::param_traits` |
| `rust/tcl-lsp-core/src/semantic_tokens.rs` | Call-site highlighting driven by callee traits |
| `rust/tcl-lsp-core/src/caller_frame.rs` | Caller-frame binding resolution |
| `rust/tcl-spec-studio/src/infer.rs` | Spec inference from an observed proc |
