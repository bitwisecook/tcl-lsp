# KCS: Connection scope — cross-event variable flow (iRules)

## Symptom

A contributor sees false-positive "read before set" (W103) or "dead store"
(O109) warnings on variables that are defined in one iRules `when` event
handler and used in another, or needs to understand how cross-event variable
flow is tracked.

## Context

In iRules, `when` event handlers share a connection-scoped variable stack.
Variables set in `CLIENT_ACCEPTED` persist until the connection closes, so
reads in `HTTP_REQUEST` are legitimate — not read-before-set errors.
`ConnectionScope` analysis tracks this flow to suppress false positives.

Source: [`core/compiler/connection_scope.py`](../../../core/compiler/connection_scope.py)

## Content

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

`CLIENT_ACCEPTED`: defs=`{conn_start, count}`, uses_before_def=`{}`
`HTTP_REQUEST`: defs=`{count}`, uses_before_def=`{count, conn_start}`

**Step 2 — Cross-event set computation:**

`build_connection_scope()` compares every pair of events:
- `CLIENT_ACCEPTED` defines `{conn_start, count}`
- `HTTP_REQUEST` uses-before-def `{count, conn_start}`
- Intersection: `{conn_start, count}` — these flow across events

**Step 3 — Result:**

```python
ConnectionScope(
    cross_event_defs=frozenset({"conn_start", "count"}),
    cross_event_imports=frozenset({"conn_start", "count"}),
)
```

### Effect on diagnostics

The optimiser's `PassContext` receives `cross_event_vars` when processing
each event handler.  Before reporting:
- **O109 (dead store)**: check if the variable is in `cross_event_vars`
- **W103 (read before set)**: check if the variable is in `cross_event_vars`

Both suppress the warning if the variable flows across events.

## Decision rule

- If a new event type is added to iRules, no changes to `connection_scope.py`
  are needed — the analysis is event-name agnostic.
- If a variable is `unset` in one handler, it is removed from `cross_event_defs`
  for downstream events.
- Connection scope only applies to iRules (multi-event scripts).  Standard Tcl
  procedures do not use this analysis.

## Related docs

- [Example 25 in walkthroughs](../../example-script-walkthroughs.md#example-25-connection-scope--cross-event-variable-flow-irules)
- [kcs-compiler-pipeline-overview.md](kcs-compiler-pipeline-overview.md)
- [kcs-side-effects-system.md](kcs-side-effects-system.md)
