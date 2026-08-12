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
`priority: int` which conflated the declared value with the implicit ordering.
Splitting into base + offset makes disambiguation explicit.

## Data model

### `IrProcedure::base_priority` (`rust/tcl-compiler/src/ir.rs`)

The declared priority from `when EVENT priority N { body }`. Defaults to 500.
Set during lowering (`rust/tcl-compiler/src/lowering/`). Does **not** carry an
offset — a single `IRProcedure` does not know about sibling handlers.

### `EventOrderEntry` (`rust/tcl-compiler/src/irules_checks.rs`)

```python
@dataclass(frozen=True, slots=True)
class EventOrderEntry:
    event: str
    base_priority: int
    priority_offset: int   # 0 for first handler at this priority, +1 per tie
    multiplicity: str
    range: Range
```

Offset is computed in `extract_event_order()` after sorting handlers by
`(base_priority, file_index)`.

### `RuleInitExport` / `RuleInitVarDef`

Both carry `base_priority: int` for cross-file RULE_INIT variable tracking.
No offset — RULE_INIT ordering does not require tie-breaking.

## Extraction paths

There are two independent priority extraction paths:

1. **Compiler path** — `lowering.py` parses `when EVENT priority N { body }`
   during IR lowering and stores `base_priority` on `IRProcedure`. Consumed by
   `rust/tcl-diagram/src/data.rs` for diagram data.

2. **Lightweight lexer path** — `_find_when_bodies()` in `irules_flow.py`
   re-parses the same syntax directly from source. Consumed by
   `extract_event_order()` and `extract_rule_init_vars()`.

## JSON serialisation

- **Diagram data** (`rust/tcl-diagram/src/data.rs`): emits `"priority"` as
  `base_priority` or `null` when equal to 500.
- **Explorer event order** (`rust/tcl-explorer/src/serialise.rs`): emits `"base_priority"`
  and `"priority_offset"`.

## File-path anchors

- `rust/tcl-compiler/src/ir.rs` — `IrProcedure::base_priority`
- `rust/tcl-compiler/src/lowering/` — priority extraction during lowering
- `rust/tcl-compiler/src/irules_checks.rs` — `EventOrderEntry`, `RuleInitExport`
- `rust/tcl-diagram/src/data.rs` — diagram consumer
- `rust/tcl-explorer/src/serialise.rs` — JSON serialisation
- `rust/tcl-lsp-core/src/workspace_index.rs` — `RuleInitVarDef`
- `rust/tcl-compiler/src/irules_checks.rs` unit tests — priority and offset assertions
- `rust/tcl-lsp-core/src/workspace_index.rs` unit tests — RULE_INIT priority assertions
