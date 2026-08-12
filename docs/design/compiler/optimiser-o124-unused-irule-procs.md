# O124 — Comment out unused procs in iRules

## Summary

O124 detects procs defined in an iRule that are never called (directly or
transitively) from any event handler, and suggests commenting them out.

## Operational context

iRules often accumulate dead proc definitions over time — procs that were
once needed but are no longer called after refactoring.  These procs still
consume TMM memory and add maintenance burden.  O124 builds a call graph
from event handlers through `call` indirection and flags unreachable procs.

### When it fires

- The active dialect is iRules — `is_irules_dialect(ctx.dialect)` accepts
  both `f5-irules` and `irules`.
- The IR module defines at least one procedure that is not a `::when::*`
  event handler.
- The handlers present include at least one non-`RULE_INIT` event (e.g.
  `HTTP_REQUEST`, `CLIENT_ACCEPTED`).
- A `proc` definition is not reachable from any event handler via the
  transitive call graph.

### When it does NOT fire

- **Plain Tcl** — only applies to the iRules dialect.
- **Library iRules** — iRules with only procs and at most a `RULE_INIT`
  event (used for setting static variables) are treated as libraries and
  excluded.  These are designed to be `call`-ed from other iRules.
- **Procs only, no events** — `is_library_irule` answers `true` for an empty
  event set, so this is a library too.
- **Procs called from `RULE_INIT`** — still considered used since
  `RULE_INIT` is a real event that executes the proc.
- **Dynamic dispatch present** — if any *reachable* proc's summary has
  `has_barrier` (`eval`, `uplevel`, and the rest of the dynamic-barrier
  set), the pass returns without reporting anything.  The dynamic code could
  invoke any proc at runtime.

  `has_unknown_calls` is deliberately **not** part of this guard: it is set
  by ordinary impure built-ins (`pool`, `puts`, …) that cannot invoke a user
  proc, so treating it as dynamic dispatch would suppress O124 on almost
  every real iRule.

## Examples

### Unused proc — fires O124

```tcl
proc helper {x} {
    return [expr {$x + 1}]
}

when HTTP_REQUEST {
    pool my_pool
}
```

`helper` is defined but never called from `HTTP_REQUEST`.  O124 suggests
replacing the proc with:

```tcl
# [O124] Unused proc — 'helper' is not called from any event
# proc helper {x} {
#     return [expr {$x + 1}]
# }

when HTTP_REQUEST {
    pool my_pool
}
```

### Used proc — no diagnostic

```tcl
proc helper {} {
    return 1
}

when HTTP_REQUEST {
    set val [call helper]
}
```

`helper` is called from `HTTP_REQUEST` via `call helper`, so no O124.

### Transitively used proc — no diagnostic

```tcl
proc inner {} {
    return 42
}

proc outer {} {
    return [call inner]
}

when HTTP_REQUEST {
    set val [call outer]
}
```

`outer` is called from `HTTP_REQUEST`, and `inner` is called from `outer`.
Both are reachable — no O124 for either.

### Library iRule — excluded

```tcl
proc utility_a {x} {
    return [string toupper $x]
}

proc utility_b {x y} {
    return "$x:$y"
}

when RULE_INIT {
    set ::debug 0
}
```

This iRule has only procs and `RULE_INIT`.  It looks like a library
intended to be `call`-ed from other iRules.  O124 does not fire.

### Multiple unused procs

```tcl
proc used_helper {} {
    return 1
}

proc dead_code_a {} {
    return 2
}

proc dead_code_b {} {
    return 3
}

when HTTP_REQUEST {
    set val [call used_helper]
}
```

O124 fires for `dead_code_a` and `dead_code_b` but not `used_helper`.

## Enabling and disabling O124

O124's category is `OptCategory::Dce`
(`rust/tcl-core-types/src/diag_code.rs`).  The default editor profile is
`readability`, which enables only `OptCategory::Readability` — so **O124 does
not surface under the default configuration**.  It needs
`tclLsp.optimiser.profile` set to `full` or `aggressive`; `profile_to_disabled`
(`rust/tcl-compiler/src/optimiser/profiles.rs`) computes the disabled set from
the profile's enabled categories.

Once the profile enables it, O124 can be turned back off per-editor or via
LSP settings:

| Editor | Setting |
|--------|---------|
| VS Code | `tclLsp.optimiser.O124` → `false` |
| JetBrains | Settings → Tcl LSP → Optimiser → uncheck O124 |
| Any LSP client | Send `workspace/didChangeConfiguration` with `{"optimiser": {"O124": false}}` |
| Inline | `# noqa: O124` comment on the line **before** the `proc` |

The inline form must be a *preceding* comment: `apply_preceding_noqa`
(`rust/tcl-compiler/src/analyser/utils.rs`) attaches a comment's codes to the
command that follows it, so a trailing `;# noqa: O124` suppresses the next
command instead.

The master optimiser toggle (`tclLsp.optimiser.enabled` / `optimiser.enabled`)
also controls O124.  All of these routes converge on the
`disabled_optimisations` set filtered by `lift_compiler_diagnostics`
(`rust/tcl-lsp-server/src/lib.rs`).

## Algorithm

1. Bail unless the active dialect is iRules and the module defines at least
   one procedure.
2. Separate IR module procedures into event handlers (`::when::*`) and
   user procs; bail if there are no user procs.
3. Collect event names via `when_event_name`; if all are `RULE_INIT` (or
   none exist), `is_library_irule` classifies the file as a library and the
   pass returns.
4. `reachable_procs` walks the call graph from every event handler, following
   `InterproceduralAnalysis::procedures[*].calls` (already the transitive
   closure) with a visited set, so cycles terminate.
5. **Dynamic dispatch guard**: if any reachable proc's summary has
   `has_barrier`, return without reporting — dynamic dispatch could target
   any proc, so none can safely be called unused.
6. Sort the unreachable user procs by qualified name for deterministic
   output, and for each emit an `Optimisation` whose span is
   `full_rewrite_span(ctx.source, ir_proc.span)` and whose replacement is
   `comment_out(...)`: the banner line followed by every source line prefixed
   with `# ` (a blank line becomes a bare `#`).

## File-path anchors

- `rust/tcl-compiler/src/optimiser/unused_procs.rs` — pass implementation and unit tests
- `rust/tcl-compiler/src/optimiser/mod.rs` — `PassId::UnusedProcs` dispatch, and
  `opt_priority` (O124 has priority 10, joint highest with O126)
- `rust/tcl-compiler/src/optimiser/manager.rs` — the `optimise_*` façades that run `PassId::all()`
- `rust/tcl-compiler/src/optimiser/profiles.rs` — profile → disabled-code set
- `rust/tcl-core-types/src/diag_code.rs` — the `O124` row and its `Dce` category
- `rust/tcl-compiler/src/interprocedural.rs` — the `ProcSummary` call graph the pass walks
- `rust/tcl-compiler/src/optimiser/helpers/spans.rs` — `full_rewrite_span`
- `rust/tcl-lsp-server/src/lib.rs` — `lift_compiler_diagnostics` (per-code and master toggles)
- `editors/vscode/package.json` — VS Code toggle
- `editors/jetbrains/src/main/kotlin/com/tcllsp/jetbrains/settings/` — JetBrains toggle (`optimiserO124`)

## Tests

- `rust/tcl-compiler/src/optimiser/unused_procs.rs` unit tests

## Failure modes

- False positive on a library iRule that has a non-RULE_INIT event for
  housekeeping (e.g. `CLIENT_ACCEPTED` that only logs).  The proc may be
  `call`-ed from other iRules — user should disable O124 for that file.
- False negative when `eval`/`uplevel` is present in the reachable call
  graph: O124 conservatively suppresses all suggestions even if the
  dynamic dispatch does not actually target any of the unused procs.

## See also

- [Compiler KCS index](README.md)
- [Optimiser feature KCS](../../kcs/features/kcs-feature-optimiser.md)
- [Pass fact ownership matrix](../../../docs/design/compiler/pass-fact-ownership-matrix.md)
