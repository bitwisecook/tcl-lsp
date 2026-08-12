# KCS: Connection scope — cross-event variable flow (iRules)

## Symptom

A contributor sees false-positive "read before set" (W210) or "dead store"
(O109) warnings on variables that are defined in one iRules `when` event
handler and used in another, or needs to understand how cross-event variable
flow is tracked.

## Context

In iRules, `when` event handlers share a connection-scoped variable stack.
Variables set in `CLIENT_ACCEPTED` persist until the connection closes, so
reads in `HTTP_REQUEST` are legitimate — not read-before-set errors.
`ConnectionScope` analysis tracks this flow to suppress false positives.

Source: `rust/tcl-compiler/src/connection_scope.rs`

## Content

### Analysis flow

```
when CLIENT_ACCEPTED { set conn_start [clock seconds]; set count 0 }
when HTTP_REQUEST    { incr count; log ... $conn_start }
```

**Step 1 — `EventVarSummary` per handler:**

For each event, walk SSA statements and record:
- `defs: HashSet<String>` — variables definitely assigned (any SSA version > 0),
  excluding `::`-qualified globals and `static::` names destroyed by `unset`
- `uses_before_def: HashSet<String>` — variables read at SSA version 0 (no
  preceding local assignment)
- `unsets: HashSet<String>` — variables explicitly `unset`

When an iRule defines multiple `when` blocks for the same event (possibly
with different priorities), each handler's summary is merged — defs, uses,
and unsets are unioned — producing a single combined summary per event name.

`CLIENT_ACCEPTED`: defs=`{conn_start, count}`, uses_before_def=`{}`
`HTTP_REQUEST`: defs=`{count}`, uses_before_def=`{count, conn_start}`

**Step 2 — Cross-event set computation:**

`build_connection_scope()` compares every ordered pair of events, skipping
pairs the event registry flags with a `variable_scope_note` (i.e. where the
two events do not actually share a variable stack):
- `CLIENT_ACCEPTED` defines `{conn_start, count}`
- `HTTP_REQUEST` uses-before-def `{count, conn_start}`
- Intersection: `{conn_start, count}` — these flow across events

**Step 3 — Result:**

```rust
pub struct ConnectionScope {
    /// Per-event summaries keyed by event name.
    pub summaries: HashMap<String, EventVarSummary>,
    /// Variables defined in one event AND used-before-def in a
    /// different event.  Suppresses dead-store / unused-var
    /// diagnostics on the producer side.
    pub cross_event_defs: HashSet<String>,
    /// Variables used-before-def in one event AND defined in a
    /// different event.  Suppresses W210 on the consumer side.
    pub cross_event_imports: HashSet<String>,
    /// `static::` vars defined in a non-RULE_INIT event and
    /// used cross-event — feeds the **IRULE4005** racy-static
    /// emitter.
    pub racy_static_defs: HashSet<String>,
}
```

For the example above both `cross_event_defs` and `cross_event_imports` come
out as `{"conn_start", "count"}`.  The result is built once from the
`::when::*` subset of `CompilationUnit::procedures` and cached on
`CompilationUnit::connection_scope`.

### Effect on diagnostics

The optimiser's `PassContext` (`rust/tcl-compiler/src/optimiser/mod.rs`) has a
`cross_event_vars: HashSet<String>` field, populated for `::when::*`
procedures by unioning `cross_event_defs` with `cross_event_imports`.
Before reporting:
- **O109 (dead store) / O126**: `elimination.rs` skips names in `cross_event_vars`
- **Propagation**: `propagation.rs` refuses to forward a def whose name is in
  the set
- **W210 (read before set)** and the unused-variable emitters: the analyser
  threads `cross_event_defs` / `cross_event_imports` through
  `emit_cfg_ssa_diagnostics_for_function`

All suppress the warning if the variable flows across events.

## Decision rule

- If a new event type is added to iRules, no changes to `connection_scope.rs`
  are needed — the analysis is event-name agnostic.
- If a variable is `unset` in one handler, it is removed from `cross_event_defs`
  for downstream events.
- Connection scope only applies to iRules (multi-event scripts).  Standard Tcl
  procedures do not use this analysis.

## Related docs

- [Example 25 in walkthroughs](../../../docs/design/example-script-walkthroughs.md#example-25-connection-scope--cross-event-variable-flow-irules)
- [kcs-compiler-pipeline-overview.md](../../../docs/design/compiler/compiler-pipeline-overview.md)
- [kcs-side-effects-system.md](../../../docs/design/compiler/side-effects-system.md)
