# KCS: Diagnostics calculation — two-phase architecture and scheduling

## Symptom

A contributor needs to understand why some diagnostics appear instantly and
others after a delay, how the DiagnosticScheduler manages cancellation, or
needs to add a new diagnostic to the correct phase.

## Context

The LSP server produces diagnostics in two phases: a fast synchronous phase
for immediate feedback (basic diagnostics) and an expensive asynchronous phase
(deep diagnostics) that runs in a background thread.  The `DiagnosticScheduler`
manages task lifecycle with cancellation and version tracking.

Source: [`lsp/features/diagnostics.py`](../../../lsp/features/diagnostics.py),
[`lsp/async_diagnostics.py`](../../../lsp/async_diagnostics.py)

## Content

### Phase 1 — Basic diagnostics (fast, synchronous)

`get_basic_diagnostics()` runs on every keystroke and returns immediately:

- **Semantic analysis** (`analyse()`):
  - W100: Unbraced expr body
  - W101: Wrong number of arguments
  - W102: Unknown command
  - W103: Variable read before set
  - W104: Unused variable
  - W200+: iRules event/command warnings
  - W300+: Deprecation/style warnings
- **Style checks**:
  - W111: Line exceeds configured length
  - W112: Trailing whitespace
  - W115: Backslash-newline continuation in comment
  - W120: Command used without package require

### Phase 2 — Deep diagnostics (expensive, background thread)

`get_deep_diagnostics()` runs via `asyncio.to_thread` to avoid blocking.
It reuses the `CompilationUnit` from Phase 1:

- **Optimiser** (`find_optimisations`): O100–O126
- **Shimmer detector** (`find_shimmer_warnings`): S100–S102
- **Taint engine** (`find_taint_warnings`): T100–T106, T200–T201, IRULE3001–3004
- **iRules flow checker** (`find_irules_flow_warnings`): IRULE1005–5004
- **GVN/CSE** (`find_redundant_computations`): O105–O106

### Async scheduling and cancellation

```
Document edit (version N)
    │
    ├─► Phase 1: get_basic_diagnostics() → publish immediately
    │
    └─► DiagnosticScheduler.schedule(uri, version=N, ...)
          │
          ├─► Cancel any in-flight deep task for this URI
          │
          └─► asyncio.create_task(_run())
                │
                └─► asyncio.to_thread(deep_fn) ← background thread
                      │
                      ▼
                  publish_fn(uri, basic + deep, version=N)
```

Key properties:
- **Cancellation**: new keystrokes cancel stale deep tasks.
- **Version tracking**: results are discarded if a newer version was scheduled.
- **Merge**: final published diagnostics are `basic + deep`.

### Suppression with `# noqa`

```tcl
set x 42    ;# noqa: O109  — suppress dead store warning
eval $cmd   ;# noqa: *     — suppress ALL warnings on this line
```

The suppression map (`suppressed_lines: dict[int, frozenset[str]]`) is built
during semantic analysis and checked by both phases before emitting.

### Grouped optimisations

Related optimisation edits share a `group` ID.  The diagnostics publisher
emits one primary diagnostic with others as `DiagnosticRelatedInformation`:

```
Primary: O100 "Propagate constant into expression" (+1 dead store eliminated)
  └─ Related: O109 "Dead store: x is set but never read"
```

The LSP client applies all grouped edits atomically via a single code action.

## Decision rule

- Fast diagnostics (W-codes, syntax errors) go in `get_basic_diagnostics()`.
- Expensive diagnostics (optimisations, taint, shimmer) go in
  `get_deep_diagnostics()`.
- If a new diagnostic requires `CompilationUnit` data (CFG, SSA, analysis),
  it belongs in Phase 2.
- If it only needs AST/tokens, it can go in Phase 1.

## Related docs

- [Diagnostics section in walkthroughs](../../example-script-walkthroughs.md#how-diagnostics-are-calculated)
- [kcs-async-diagnostics-tiering.md](kcs-async-diagnostics-tiering.md)
- [kcs-diagnostics-integration.md](kcs-diagnostics-integration.md)
- [kcs-troubleshooting-duplicate-diagnostics.md](kcs-troubleshooting-duplicate-diagnostics.md)
