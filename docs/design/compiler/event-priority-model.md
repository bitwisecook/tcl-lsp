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

### `IRProcedure.base_priority` (`compiler/ir.py`)

The declared priority from `when EVENT priority N { body }`. Defaults to 500.
Set during lowering (`compiler/lowering.py`). Does **not** carry an
offset — a single `IRProcedure` does not know about sibling handlers.

### `EventOrderEntry` (`compiler/irules_flow.py`)

```python
@dataclass(frozen=True, slots=True)
class EventOrderEntry:
    event: str
    base_priority: int
    priority_offset: int  # 0 for first handler at this priority, +1 per tie
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
   `tooling/diagram/extract.py` for diagram data.

2. **Lightweight lexer path** — `_find_when_bodies()` in `irules_flow.py`
   re-parses the same syntax directly from source. Consumed by
   `extract_event_order()` and `extract_rule_init_vars()`.

## JSON serialisation

- **Diagram data** (`tooling/diagram/extract.py`): emits `"priority"` as
  `base_priority` or `null` when equal to 500.
- **Explorer event order** (`tooling/cli/serialise.py`): emits `"base_priority"`
  and `"priority_offset"`.

## File-path anchors

- `compiler/ir.py` — `IRProcedure.base_priority`
- `compiler/lowering.py` — priority extraction during lowering
- `compiler/irules_flow.py` — `EventOrderEntry`, `RuleInitExport`
- `tooling/diagram/extract.py` — diagram consumer
- `tooling/cli/serialise.py` — JSON serialisation
- `tooling/explorer/static/index.html` — explorer HTML consumer
- `server/workspace/workspace_index.py` — `RuleInitVarDef`
- `tests/test_irules_checks.py` — priority and offset assertions
- `tests/test_rule_init_vars.py` — RULE_INIT priority assertions
