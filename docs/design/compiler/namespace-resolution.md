# KCS: Namespace resolution

## Symptom

A contributor needs to understand how qualified names (`::foo::bar`) are
normalised and resolved through the compilation pipeline, or is debugging
why a procedure call is not matched to its definition.

## Context

Tcl uses `::` as the namespace separator with `::` as the global namespace.
`normalise_qualified_name()` canonicalises names, and lowering propagates
namespace context so that `proc` definitions inside `namespace eval` receive
fully qualified names. Same-file call resolution (the analyser's shadow /
arity-suppression checks, the optimiser's interprocedural proc-identity
resolution, and interprocedural/taint analysis's call-graph edges) all
resolve a bareword call the same way, via one shared candidate-list
function.

Source: `rust/tcl-syntax/src/naming.rs`,
`rust/tcl-compiler/src/lowering/mod.rs`,
`rust/tcl-compiler/src/interprocedural.rs`

## Content

### `normalise_qualified_name()`

```rust
normalise_qualified_name("helper")          // → "::helper"
normalise_qualified_name("::helper")        // → "::helper"
normalise_qualified_name("mylib::helper")   // → "::mylib::helper"
normalise_qualified_name("::::foo::::bar")  // → "::foo::bar"
```

Rules: strip trailing `::`, collapse multiple `::` runs, ensure leading `::`.

### Namespace context propagation during lowering

Lowering threads a `namespace` parameter through the walk:

1. Top-level: `namespace = "::"`.
2. `namespace eval mylib { ... }`: joins to `"::mylib"`; the body lowers
   with that namespace.
3. `proc helper` inside `::mylib` is qualified to `"::mylib::helper"`.

### Bareword call resolution: `bareword_resolution_candidates()`

`tcl_syntax::naming::bareword_resolution_candidates(namespace, cmd_name)`
returns the candidate qualified names for a call, in the priority order Tcl
itself uses — current namespace first, then global:

```rust
bareword_resolution_candidates("::mylib", "helper")       // → ["::mylib::helper", "::helper"]
bareword_resolution_candidates("::mylib", "other::helper") // → ["::mylib::other::helper", "::other::helper"]
bareword_resolution_candidates("::mylib", "::helper")      // → ["::helper"]
bareword_resolution_candidates("::", "helper")             // → ["::helper"]
```

The rule is **exactly two levels** — the caller's own namespace, then
global — never every enclosing ancestor namespace. Real Tcl command lookup
does not walk intermediate namespaces absent an explicit `namespace path`,
which none of these consumers model: a `::a::b::c::caller` calling bare
`foo` does **not** reach a `::a::foo` defined in the grandparent namespace,
even though `::a` encloses `::a::b::c`. This also applies to a *relative
dotted* word (`other::helper`, containing `::` but not starting with it) —
it is still resolved against the current namespace first, not rooted
straight at global (confirmed against tclsh 9.0.4: calling `other::helper`
from inside `namespace eval ::mylib { … }` reaches `::mylib::other::helper`
before `::other::helper`, when both exist).

Every same-file resolution consumer builds its own candidates through this
one function and then walks them against its own lookup table (procedures,
aliases, classes, …), so a fix to the rule — or a bug in it — cannot drift
between call sites:

- **Analyser** — `Analyser::resolve_proc_call` (go-to-definition / symbol
  lookup) and `UserResolutionFacts::resolves_to_user` (the W002
  disabled-in-dialect and E002/E003 builtin-arity same-file shadow
  suppression checks) compute the caller's namespace from the live scope
  tree via `Analyser::command_resolution_namespace`.
- **Optimiser** — `resolve_proc_qname` (O103 static-proc-call folding)
  computes it the same way, from the enclosing proc's scope.
- **Interprocedural / taint** — `resolve_internal_call` computes the
  caller's namespace from its own qualified name
  (`namespace_parts_from_proc`, since interprocedural analysis works over
  IR qnames rather than a live scope tree).

### Resulting IR module

```rust
Module {
    procedures: {
        "::mylib::helper": Procedure { name: "helper", .. },
        "::mylib::compute": Procedure { name: "compute", .. },
    },
    ..
}
```

All procedure names in the IR module are fully qualified.

## Decision rule

- Always pass names through `normalise_qualified_name()` before using them
  as map keys or comparison targets.
- Always build same-file call-resolution candidates through
  `bareword_resolution_candidates()` rather than hand-rolling the
  current-namespace/global logic again — every prior hand-rolled copy
  carried at least one of the two bugs above.
- If a procedure call fails to resolve, check that the caller's namespace
  context was propagated correctly through lowering (or, for the analyser,
  through the scope tree).
- `normalise_var_name()` is for variables (handles `::` prefix only);
  `normalise_qualified_name()` is for procedures and commands.

## Related docs

- [Example 26 in walkthroughs](../../../docs/design/example-script-walkthroughs.md#example-26-namespace-resolution)
- [compiler-pipeline-overview.md](../../../docs/design/compiler/compiler-pipeline-overview.md)
- [interprocedural-analysis.md](../../../docs/design/compiler/interprocedural-analysis.md)
