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

`CompilationUnit::connection_scope` carries the result (`Some` when at least
one `::when::*` procedure exists). Three consumers read it:

- **W210 (read before set)** — the analyser treats
  `cross_event_defs ∪ cross_event_imports` as defined inside a `::when::*`
  procedure (`when_proc_cross_event_names`, `analyser/diagnostics.rs`).
- **O109 (dead store)** and the other optimiser passes — `PassContext::cross_event_vars`
  holds the same names while a handler is processed.
- **IRULE4005** — reads `racy_static_defs`.

## Decision rule

- A new iRules event needs no change to `connection_scope.rs` — the analysis
  is event-name agnostic; only `EventRegistry::variable_scope_note` decides
  whether a pair of events shares scope.
- An `unset` is recorded in the handler's summary; it does not remove the
  name from `cross_event_defs`.
- Connection scope applies only to iRules. Plain Tcl procedures never use it.

## Related docs

- [Example 24 in walkthroughs](../example-script-walkthroughs.md#example-24-connection-scope--cross-event-variable-flow-irules)
- [compiler-pipeline-overview.md](compiler-pipeline-overview.md)
- [side-effects-system.md](side-effects-system.md)
