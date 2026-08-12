# Event handler priority model

## Summary

Event handler priority is modelled as two separate values: a **base priority**
(the declared `priority N` from source, default 500) and a **priority offset**
(tie-breaker among handlers sharing the same event and base priority, derived
from file order).

## Problem

iRules allow multiple `when EVENT` handlers for the same event. BIG-IP executes
them in priority order (lowest first). When two handlers share the same
priority, file order breaks the tie. Base priority and offset are separate
values so the declared priority is never conflated with the implicit
file-order tie-break.

## Data model

### `Procedure::base_priority` (`rust/tcl-compiler/src/ir.rs`)

`pub base_priority: u32` — the declared priority from
`when EVENT priority N { body }`, defaulting to 500. Set during lowering
(`rust/tcl-compiler/src/lowering/`). Does **not** carry an offset — a single
`Procedure` does not know about sibling handlers.

### Event-order entries (`rust/tcl-explorer/src/serialise.rs`)

`serialise_event_order` emits one JSON object per handler:

```json
{
  "event": "HTTP_REQUEST",
  "base_priority": 500,
  "priority_offset": 0,
  "multiplicity": "...",
  "range": { "...": "..." }
}
```

`priority_offset` is 0 for the first handler at a given base priority and
increments by 1 for each subsequent tie. It is computed after sorting each
event's handlers by `(priority, file_index)`.

### `RuleInitExport` / `RuleInitVarDef`

Both carry a `base_priority` for cross-file RULE_INIT variable tracking.
No offset — RULE_INIT ordering does not require tie-breaking.

## Extraction paths

There are two independent priority extraction paths:

1. **Compiler path** — `lowering/` parses `when EVENT priority N { body }`
   during IR lowering and stores `base_priority` on `IRProcedure`. Consumed by
   `rust/tcl-diagram/src/data.rs` for diagram data.

2. **Lightweight segmenter path** — `serialise_event_order`
   (`rust/tcl-explorer/src/serialise.rs`) segments the source directly and
   reads `when EVENT priority N` from the words, without building a
   `CompilationUnit`.

## JSON serialisation

- **Diagram data** (`rust/tcl-diagram/src/data.rs`): emits `"priority"` as
  `base_priority` or `null` when equal to 500.
- **Explorer event order** (`rust/tcl-explorer/src/serialise.rs`): emits `"base_priority"`
  and `"priority_offset"`.

## File-path anchors

- `rust/tcl-compiler/src/ir.rs` — `Procedure::base_priority`
- `rust/tcl-compiler/src/lowering/` — priority extraction during lowering
- `rust/tcl-diagram/src/data.rs` — diagram consumer
- `rust/tcl-explorer/src/serialise.rs` — `serialise_event_order`, and its
  unit tests covering priority and offset
- `rust/tcl-explorer/src/view_tree.rs` — the explorer view that sorts on
  `base_priority + priority_offset`
