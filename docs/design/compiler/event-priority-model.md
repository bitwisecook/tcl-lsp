# KCS: compiler — event handler priority model

## Summary

Event handler priority is modelled as two separate values: a **base priority**
(the declared `priority N` from source, default 500) and a **priority offset**
(tie-breaker among handlers sharing the same event and base priority, derived
from file order).

## Problem

iRules allow multiple `when EVENT` handlers for the same event. BigIP executes
them in priority order (lowest first). When two handlers share the same
priority, file order breaks the tie. Previously the codebase stored a single
`priority` integer, which conflated the declared value with the implicit ordering.
Splitting into base + offset makes disambiguation explicit.

## Data model

### `Procedure::base_priority` (`rust/tcl-compiler/src/ir.rs`)

```rust
/// BIG-IP handler priority (0..2^32-1, default 500).
pub base_priority: u32,
```

The declared priority from `when EVENT priority N { body }`, set during
lowering (`rust/tcl-compiler/src/lowering/mod.rs`). Does **not** carry an
offset — a single `Procedure` does not know about sibling handlers. Sibling
handlers for the same event are instead disambiguated by the qualified name:
the first is `::when::EVENT`, subsequent ones are `::when::EVENT#1`,
`::when::EVENT#2`, … (`when_event_name` strips the `#N` suffix back off).

### Explorer `eventOrder` entries (`rust/tcl-explorer/src/serialise.rs`)

There is no standalone entry struct — `serialise_event_order` builds the JSON
rows directly. Each row carries `event`, `base_priority`, `priority_offset`,
`multiplicity`, and `range`. The offset is computed after sorting each event's
handlers by `(priority, file_index)`: it resets to 0 whenever the priority
changes and increments by 1 for each further handler at the same priority.

## Extraction paths

There are two independent priority extraction paths:

1. **Compiler path** — `Lowerer::lower_when` parses `when EVENT priority N
   { body }` during IR lowering and stores `base_priority` on `Procedure`.
   A missing or non-integer priority word keeps the 500 default. Consumed by
   `rust/tcl-diagram/src/data.rs` for diagram data.

2. **Lightweight segmenter path** — `serialise_event_order` re-scans the same
   syntax straight from source with `segment_commands`, requiring the body word
   to be a braced block (`TokenType::Str`). It never builds IR.

Both paths order events through `EventRegistry::order_events` and label them
with `EventRegistry::event_multiplicity`
(`rust/tcl-registry/src/events.rs`).

## JSON serialisation

- **Diagram data** (`rust/tcl-diagram/src/data.rs`): emits `"priority"` as
  `base_priority`, or `null` when it equals the 500 default.
- **Explorer event order** (`rust/tcl-explorer/src/serialise.rs`): emits
  `"base_priority"` and `"priority_offset"` as separate fields; the explorer's
  tree view (`rust/tcl-explorer/src/view_tree.rs`) sorts on their sum.

## File-path anchors

- `rust/tcl-compiler/src/ir.rs` — `Procedure::base_priority`, `when_event_name`
- `rust/tcl-compiler/src/lowering/mod.rs` — priority extraction during lowering
- `rust/tcl-registry/src/events.rs` — `EventRegistry::order_events`, `event_multiplicity`
- `rust/tcl-diagram/src/data.rs` — diagram consumer
- `rust/tcl-explorer/src/serialise.rs` — `serialise_event_order` JSON serialisation
- `rust/tcl-explorer/src/view_tree.rs` — explorer tree consumer
- `rust/tcl-explorer/src/serialise.rs` — `mod tests`, the `event_order_*` cases
  (`event_order_orders_when_handlers_by_firing_order`,
  `event_order_tie_break_increments_offset`)
