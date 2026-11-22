# Variable lookup is not centralised

The companion to [command lookup is not centralised](command-lookup-centralisation.md). The [naming API guide](naming-api-guide.md) gives variables a shared binding and place owner (`SsaSourceView::read_reference`, `PlaceKind`/`PlaceRef`, "variable binding and place owners"). As with commands, the low-level geometry is shared and the rest is not: the frame a name is looked up in is chosen many times, several consumers keep their own name-keyed tables, and the conformance vectors drive none of the analyser or editor consumers.

## What consumers actually answer

Each fixture in [`oo-dispatch-probe/variables-lookup/`](handover/oo-dispatch-probe/variables-lookup/) was run under tclsh 8.4.20, 8.6.18 and 9.0.4, then given to `tcl-lsp-core` definition, references, rename and hover at the marked read, and to the analyser's diagnostics, on `d8add62f` (`variables-lookup.out`). Rows that agree with Tcl and with each other are summarised at the end.

| Read | Tcl | Definition | References | Rename | Hover | Diagnostics |
| --- | --- | --- | --- | --- | --- | --- |
| `return $c$m` after `catch {set c …}` and `regexp … -> m` | both set | finds the `m` written by `regexp` | the write and the read | renames both | variable `m` | **W210 on `c` and `m`** |
| `$x` after `setit x`, where `setit` does `upvar 1 $name v; set v …` | set by the callee | the `setit x` call | call and read | refused | "caller-frame variable" | **W210** |
| `$alias` after `upvar #0 g alias` | `::g` | `set g` and the `upvar` word | mixes `g` and `alias` | **renames `g`, leaves `alias`** | variable `alias` | — |
| `$l` after `namespace upvar ::cfg limit l` | `::cfg::limit` | the `variable` and the `upvar` word | mixes `limit` and `l` | **renames `limit`, leaves `l`** | variable `l` | — |
| `$made` after a called `uplevel 1 {set made …}` | set | none | none | none | none | — |
| `$count` from a TclOO class `variable count` | instance variable | none | none | none | none | — |
| `$x` after `my variable x` | instance variable | none | none | none | none | W123 on `my` |
| `$t` after 9.0 `classvariable t` | class variable | none | none | none | none | — |
| `$x` after `variable x` in `apply {…} ::lib` | `::lib::x` | none | none | none | none | — |
| `$arr(k)` after `upvar 1 a arr` | caller's array | none | none | none | none | — |
| `$e` after `upvar 1 a(k) e` | caller's element | none | none | none | none | — |
| `$k` after `dict with d {}` | key as local | none | none | none | none | — |
| `namespace which -variable v` | `::a::v` | none | none | none | none | — |
| `$x` in `namespace eval ::n` after `namespace eval ::n {set x …}` with a global `x` | 8.4–8.6: `::x`; 9.0: `::n::x` | per release, correct | per release, correct | refused on 8.6 | variable `x` | — |

Also measured (`variables-flow/`): in `proc p {c} { if {$c} {set x 1} else {global x; set x 2} }` the first store is a dead local, but no W211 is reported, while an unused local store elsewhere is; and a read of a never-set variable in `namespace eval` is not reported.

Agreeing and correct: plain locals, `global x`, `global ::ns::v`, `variable` in namespace and global procedures, `$::x`, `${a b}`, `$a(x y)`, `foreach`/`lassign` targets (rename refuses them, while it renames a `regexp` target), and reads after `unset` through `info exists`.

## Why: the census

About **20 resolvers** answer "which variable is this", sharing only the `tcl-syntax`/`tcl-lexer` geometry (`variable_namespace_lookup_bases`, `variable_root_geometry`, `split_array_name*`, `global_local_name_bytes`). Only the VM, the Rust runtime and `tcl-cmd-core` delegate scoping to it completely.

| Resolver | Consumers | Note |
| --- | --- | --- |
| Place binder `resolve_place`/`bind_scalar_receiver` (`tcl-compiler/src/var_resolve/`) | SSA, memory SSA, liveness dead stores | the richest context; decides the 8.x global fallback itself |
| `SsaSourceView::read_reference` | W210, taint | delegates to the place binder |
| `VarObservability` (`var_observability.rs`) | SCCP, shimmer, load forwarding | keyed by name string; flags with no target identity |
| `var_escape`/`var_scoping` (`var_scoping.rs`) | codegen inlining, type inference | **uses the default registry regardless of dialect** (`var_scoping.rs:61`, *read*); detects dynamic names by a leading `$` in text |
| Diagnostic name sets (`optimiser/elimination.rs` alias scans, `analyser/diagnostics/helpers/original_cells.rs`) | W210, W211, W220, declared reads | whole-function, flow-insensitive `HashSet<String>` of names |
| Analyser lexical recorder (`analyser/scope/variable_reads.rs`, `variable_definitions.rs`) | `VarDef.references`, link targets | a legacy path with its own 8.x fallback |
| `logical_variable_scope_advice` (`analyser/scope/logical_scope_advice.rs`) | definition, completion, rename | re-derives the frame on every query by walking scopes; not program-point sensitive |
| Analyser qualified helpers (`namespace_variables.rs`) | tests only | reuses the *command* namespace walker; two key conventions |
| Signature-scan symbols (`signature_scan/variable_symbol.rs`) | rename, highlights, original completion | re-decides the 8.x fallback and abstains in non-root namespaces |
| LSP `select_navigation`, `VarDef` fallback, caller-frame templates (`tcl-lsp-core`) | definition, references, hover, highlights | three composed routes; the caller-frame route treats only procedure and method spans as frames |
| Registry `VariableScope::Global` rooting | write projections, taint, SSA defs, instance options | re-implemented at about six sites with different predicates |
| VM, Rust runtime, `tcl-cmd-core` | execution, `namespace which -variable` | delegate |
| MCP `def_use_chains`, minifier, graphs | tools | filter by display name |

**The 8.x "creative writing" fallback is decided in at least seven places** (`var_resolve/scalar_binding.rs`, `var_resolve/original_bytes.rs`, `signature_scan/variable_symbol.rs`, `analyser/scope/variable_reads.rs`, `analyser/scope/logical_scope_advice.rs`, `auto_path_eval/path_constants.rs`).

**No single frame type.** `ResolveContext` (`var_resolve.rs:585`) is the richest (namespace and its identity, frame kind, activation, globals, namespace variables, upvar and namespace-upvar aliases, instance variables, traces, unknowns, dialect and name policy). Beside it sit `VariableFrameKind`, `VariableExecutionFrame` (eight variants), `SourceOriginalVariableFrame`, the analyser's `Scope`/`ScopeKind`, `LogicalVariableScopeAdvice`, the diagnostics' `BodyFrame`, the registry's `VariableAliasFrame`, a bare span in `tcl-lsp-core`, and the VM's and codegen's own tables. They disagree on what a frame is: `BodyFrame` documents a `namespace eval` body as "a fresh frame whose bound-at-entry names are its formal parameters" (`analyser/diagnostics.rs:342`, *read*), the place binder treats it as a namespace frame, and the name-derived fallback `ResolveContext::for_function` makes every function other than `::top` a local frame using the namespace parsed from its name (`var_resolve/frame_selection.rs:55`, *read*).

**The frame for a body is chosen at least seven times:** the command-binding walk's recorded variable context passed to SSA; the name-derived fallback used in about nine places when point contexts are missing (dead stores, memory facts, elimination, phi placement, place bridge, cell-state SSA); the diagnostics' `BodyFrame`; the analyser scope tree; the logical advice per query; the `tcl-lsp-core` span per request; and the VM and runtime at execution. About twenty files construct `VariableExecutionFrame` values directly. The branch's own [LSP consumer notes](lsp-consumers.md) already say scope-chain and `uplevel` rules "require a reusable source-frame policy owner".

**Disagreements the code makes likely**, beyond those measured above: references follow caller-frame templates while document highlight does not; hover's reference count comes from the analyser recorder while find-references comes from signature-scan symbols, which abstain in non-root 8.x namespaces; completion offers names set later in the body while W210 is flow-sensitive.

**The conformance vectors drive nothing we ship.** `tcl-syntax/tests/data/variable_resolution_vectors.txt` (frames: global, `namespace eval`, procedure; probes: read, set, `namespace which`, `info exists`, `info vars`; columns 8.4–9.1) is run only by `tcl-syntax/tests/variable_resolution_conformance.rs` against tclsh (*read*: no other file names it). The resolution rewrite fixtures check optimiser output against tclsh; nothing checks W210, W211, definition, references, rename, hover, highlights or completion.

## Plan

The shape is the same as for commands, and the two should share the frame value.

1. **One frame value** for both commands and variables, built where a body starts: current namespace (or unknown), frame kind (global, namespace, procedure activation, method activation with receiver, level-selected, unknown), the alias table (`global`, `variable`, `upvar`, `namespace upvar`, `my variable`, `classvariable`, `link`) with alias identity kept separate from target identity, declared instance variables from the object-system profile, traces, and the selected dialect rules including the 8.x fallback. `ResolveContext` is the natural starting point; the other frame types become projections of it or are deleted.
2. **Registry-declared variable effects.** Every command that creates, links or removes a variable declares it as data: `catch` result and option variables, `regexp`/`regsub` match variables, `scan` targets, `foreach`/`lassign`/`dict with`/`dict update` targets, `upvar`/`namespace upvar`/`global`/`variable` links with their frame selection, `uplevel` writes into the selected frame, Tk `-textvariable` as a global link. The analyser and SSA consume one description instead of hand-written cases (several of these are behind the false W210s).
3. **One lookup owner and one answer type**, point-sensitive (the shared `Answer<T>`, see [validation](name-resolution-validation.md)): `variable(frame, name bytes, point) -> Cell(place) | Alias(alias, target place) | MayBe(places) | Unset | Unknown`. W210, W211, W214, W220, taint, SCCP, escape and inlining, definition, references, highlights, rename, hover and completion all ask it. Rename asks which identity the cursor names (the alias or its target) instead of guessing.
4. **Retire the parallel resolvers**: the name-keyed diagnostic sets, `VarObservability`'s string keys, `var_scoping`'s default-registry lookup, the legacy recorder path, the duplicated 8.x fallback and global-rooting sites, and the per-consumer frame choosers, behind an xtask ratchet like the command one.
5. **Consumer conformance.** Run the existing variable vectors, extended with procedure-call, `uplevel`, `upvar`, `apply`, TclOO and `dict with` frames, through every consumer: the place each one names must match tclsh's answer, and set/unset must match W210.
