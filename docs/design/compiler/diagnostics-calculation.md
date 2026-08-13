# Diagnostics calculation — two-phase architecture and scheduling

Why some diagnostics appear instantly and others after a delay, how the
diagnostic scheduler handles cancellation and document versions, and which
phase a new diagnostic belongs in.

The LSP server produces diagnostics in two phases: a fast synchronous phase
for immediate feedback (basic diagnostics) and an expensive asynchronous phase
(deep diagnostics) that runs in a background thread.  The `DiagnosticScheduler`
manages task lifecycle with cancellation and version tracking.

Source: `rust/tcl-lsp-db/src/lib.rs`,
`rust/tcl-lsp-server/src/lib.rs`

### Phase 1 — Basic diagnostics (fast, synchronous)

The basic phase runs on every keystroke and returns immediately:

- **Semantic analysis**:
  - W100: Unbraced expression argument
  - E002 / E003: Too few / too many arguments for command
  - W101: `eval` with string concatenation — code injection risk
  - W102: `subst` on variable input — code injection risk
  - W103: `open` with pipeline `|` — command injection risk
  - W104: String concatenation for list building
  - W123: Unresolved command (default on; `tclLsp.diagnostics.W123 = false` to disable)
  - W210: Variable read before it is set
  - W200+: iRules event/command warnings
  - W300+: Deprecation/style warnings
- **Style checks**:
  - W111: Line exceeds configured length
  - W112: Trailing whitespace
  - W115: Backslash-newline continuation in comment
  - W120: Command used without package require

### Phase 2 — Deep diagnostics (expensive, background thread)

The deep phase runs on a background thread to avoid blocking. It reuses the
`CompilationUnit` from Phase 1:

- **Optimiser** (`find_optimisations`): O100–O130
- **Shimmer detector** (`find_shimmer_warnings`): S100–S102
- **Taint engine** (`find_taint_warnings`): T100–T106, IRULE3001–3004
- **iRules flow checker** (`find_irules_flow_warnings`): IRULE1005–1008, IRULE4002, IRULE5004
- **GVN/CSE** (`find_redundant_computations`): O105–O106

### Async scheduling and cancellation

```
Document edit (version N)
    │
    ├─► Phase 1: basic diagnostics → publish immediately
    │
    └─► DiagnosticScheduler::schedule(uri, version = N, ...)
          │
          ├─► Cancel any in-flight deep task for this URI
          │
          └─► Spawn the deep run on a background thread
                │
                ▼
            publish(uri, basic + deep, version = N)
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

The suppression map (`suppressed_lines: HashMap<i32, HashSet<String>>`,
`rust/tcl-compiler/src/analyser/types.rs`) is built during semantic analysis
and checked by both phases before emitting.

### Grouped optimisations

Related optimisation edits share a `group` ID.  The diagnostics publisher
emits one primary diagnostic with others as `DiagnosticRelatedInformation`:

```
Primary: O100 "Propagate constant into expression" (+1 dead store eliminated)
  └─ Related: O109 "Dead store: x is set but never read"
```

The LSP client applies all grouped edits atomically via a single code action.

## Decision rule

- Fast diagnostics (W-codes, syntax errors) go in the basic phase.
- Expensive diagnostics (optimisations, taint, shimmer) go in the deep phase.
- If a new diagnostic requires `CompilationUnit` data (CFG, SSA, analysis),
  it belongs in Phase 2.
- If it only needs AST/tokens, it can go in Phase 1.

## Related docs

- [Diagnostics section in walkthroughs](../../../docs/design/example-script-walkthroughs.md#how-diagnostics-are-calculated)
- [kcs-async-diagnostics-tiering.md](../../../docs/design/compiler/async-diagnostics-tiering.md)
- [kcs-diagnostics-integration.md](../../../docs/design/compiler/diagnostics-integration.md)
