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
event's handlers by `(priority, file_index)`. Entries are emitted grouped by
event, in the canonical order `EventRegistry::order_events` gives, and
`multiplicity` is that event's `EventRegistry::event_multiplicity` class
(`init`, `per_request`, …). Only `when` commands whose last word is a braced
block are collected.

`RULE_INIT` variable tracking is per-document only. `Procedure::base_priority`
(above) carries the declared priority for each `when` handler, and
`static::` variables written by `when RULE_INIT` are tracked by
`ConnectionScope` (`rust/tcl-compiler/src/connection_scope.rs`), built by
`build_connection_scope()` from the `::when::*` procedures of a single
`CompilationUnit` — its `racy_static_defs` is the RULE_INIT-aware half, and it
never spans files.

## Extraction paths

There are two independent priority extraction paths:

1. **Compiler path** — `lowering/mod.rs` parses `when EVENT priority N
   { body }` during IR lowering and stores `base_priority` on the
   `Procedure` it registers under `::when::EVENT` (or `::when::EVENT#n` for a
   repeat handler). The priority word is read only when the command has at
   least four words, `args[1]` is literally `priority`, and `args[2]` parses
   as a `u32`; anything else keeps the 500 default. Consumed by
   `rust/tcl-diagram/src/data.rs` for diagram data.

2. **Lightweight segmenter path** — `serialise_event_order`
   (`rust/tcl-explorer/src/serialise.rs`) segments the source directly and
   reads `when EVENT priority N` from the words, without building a
   `CompilationUnit`. It parses the priority as `i64`, so this path accepts
   values the compiler path would reject.

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
