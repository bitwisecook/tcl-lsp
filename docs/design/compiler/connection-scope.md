# Connection scope — cross-event variable flow (iRules)

How variable flow between iRules `when` event handlers is tracked, so that a
variable set in one handler and read in another is not reported as a
read-before-set (W210) or a dead store (O109).

In iRules, `when` event handlers share a connection-scoped variable stack.
Variables set in `CLIENT_ACCEPTED` persist until the connection closes, so
reads in `HTTP_REQUEST` are legitimate — not read-before-set errors.
`ConnectionScope` analysis tracks this flow to suppress false positives.

Source: `rust/tcl-compiler/src/connection_scope.rs`

### Analysis flow

```
when CLIENT_ACCEPTED { set conn_start [clock seconds]; set count 0 }
when HTTP_REQUEST    { incr count; log ... $conn_start }
```

**Step 1 — `EventVarSummary` per handler:**

For each event, walk SSA blocks and record:
- `defs`: variables definitely assigned (any SSA version > 0)
- `uses_before_def`: variables read at version 0 (no preceding assignment)
- `unsets`: variables explicitly unset

When an iRule defines multiple `when` blocks for the same event (possibly
with different priorities), each handler's summary is merged — defs, uses,
and unsets are unioned — producing a single combined summary per event name.

`CLIENT_ACCEPTED`: defs=`{conn_start, count}`, uses_before_def=`{}`
`HTTP_REQUEST`: defs=`{count}`, uses_before_def=`{count, conn_start}`

**Step 2 — Cross-event set computation:**

`build_connection_scope()` compares every pair of events:
- `CLIENT_ACCEPTED` defines `{conn_start, count}`
- `HTTP_REQUEST` uses-before-def `{count, conn_start}`
- Intersection: `{conn_start, count}` — these flow across events

**Step 3 — Result:**

```rust
ConnectionScope {
    summaries: /* per-event EventVarSummary */,
    cross_event_defs: HashSet::from(["conn_start".into(), "count".into()]),
    cross_event_imports: HashSet::from(["conn_start".into(), "count".into()]),
    racy_static_defs: HashSet::new(),
}
```

### Effect on diagnostics

The optimiser's `PassContext` receives `cross_event_vars` when processing
each event handler.  Before reporting:
- **O109 (dead store)**: check if the variable is in `cross_event_vars`
- **W210 (read before set)**: check if the variable is in `cross_event_vars`

Both suppress the warning if the variable flows across events.

## Decision rule

- If a new event type is added to iRules, no changes to `connection_scope.rs`
  are needed — the analysis is event-name agnostic.
- If a variable is `unset` in one handler, it is removed from `cross_event_defs`
  for downstream events.
- Connection scope only applies to iRules (multi-event scripts).  Standard Tcl
  procedures do not use this analysis.

## Related docs

- [Example 24 in walkthroughs](../../../docs/design/example-script-walkthroughs.md#example-24-connection-scope--cross-event-variable-flow-irules)
- [kcs-compiler-pipeline-overview.md](../../../docs/design/compiler/compiler-pipeline-overview.md)
- [kcs-side-effects-system.md](../../../docs/design/compiler/side-effects-system.md)
