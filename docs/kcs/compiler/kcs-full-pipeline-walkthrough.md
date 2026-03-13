# KCS: Full pipeline walkthrough — source to bytecode and diagnostics

## Symptom

A contributor needs an end-to-end understanding of how Tcl source is
transformed through every compiler stage into bytecode and diagnostics,
including all specialised analyses (taint, shimmer, side-effects,
connection scope).

## Context

The compiler pipeline has 8 phases, plus two diagnostic tiers.  Each phase
produces reusable facts consumed by later phases.  The entry point is
`compile_source()` in `compilation_unit.py`, which orchestrates parsing,
lowering, CFG/SSA construction, analyses, and codegen.

This KCS ties together all the individual topic KCS files and the worked
examples in the walkthrough document.

## Content

### Pipeline overview

```
Source text
    │
    ▼
Phase 0: Error recovery (recovery.py)
    │  Inject VirtualTokens for unclosed delimiters → E201–E206
    │
    ▼
Phase 1: Lexing (tokens.py)
    │  Token stream: ESC, STR, VAR, CMD, SEP, NL, ...
    │
    ▼
Phase 2: Segmentation (command_segmenter.py)
    │  SegmentedCommand objects with texts[] and single[] arrays
    │
    ▼
Phase 3: IR lowering (lowering.py, lowering_hooks/)
    │  IRModule with IRStatement nodes (IRAssignConst, IRIf, IRCall, ...)
    │  Expression parsing: Pratt parser → ExprNode AST (expr_parser.py)
    │  Namespace resolution: normalise_qualified_name() (naming.py)
    │  Lowering dispatch: hooks → match/case → arg_roles fallthrough
    │
    ▼
Phase 4: CFG construction (cfg.py)
    │  CFGFunction with CFGBlock nodes, terminators (Goto, Branch, Return)
    │
    ▼
Phase 5: SSA construction (ssa.py)
    │  SSAFunction with SSABlock, SSAStmt, SSAPhi nodes
    │  Dominator tree, dominance frontiers
    │
    ▼
Phase 6: Core analyses (core_analyses.py)
    │  SCCP: constant propagation + unreachable-code detection
    │  Liveness: live_in/live_out per block
    │  Type lattice: type inference per SSA value
    │  Execution intent: side-effect/escape classification
    │  Side-effects classification: CommandSideEffects per statement
    │
    ▼
Phase 7: Specialised passes
    │
    ├── Interprocedural analysis (interprocedural.py)
    │   ProcSummary with purity, effects, constant-fold eligibility
    │
    ├── Connection scope (connection_scope.py)  [iRules only]
    │   EventVarSummary per handler, cross-event variable flow
    │
    ├── Optimiser (_manager.py, _propagation.py, _elimination.py, ...)
    │   O100–O126: constant propagation, folding, ICIP, DCE, DSE, ADCE,
    │   InstCombine, GVN/CSE, code sinking, tail-call, unused procs
    │
    ├── Taint engine (taint/)
    │   TaintLattice per SSA value, TaintColour flags, source/sink matching,
    │   interprocedural taint propagation
    │   T100–T106, T200–T201, IRULE3001–3004
    │
    ├── Shimmer detector (shimmer.py)
    │   S100–S102: type coercion warnings
    │
    └── iRules flow checker
        IRULE1005–5004: collect/release, response lifecycle, flow
    │
    ▼
Phase 8: Bytecode codegen (codegen/)
    │  LVT allocation, block linearisation (RPO), bottom-tested loop reorder,
    │  label-based emission, jump size optimisation, peephole passes,
    │  literal table construction
    │
    ▼
FunctionAsm (instructions + LVT + literal table)
```

### Phase-by-phase with examples

**Phase 0 — Error recovery** ([KCS](kcs-error-recovery.md), [Example 20](../../example-script-walkthroughs.md#example-20-error-recovery--unclosed-bracket)):
Virtual token injection for unclosed `[`, `"`, `{`.  Heuristics detect command
breaks to determine where to insert the missing delimiter.

**Phase 1–2 — Lexing and segmentation** ([Examples 1–2](../../example-script-walkthroughs.md#example-1-set-x-42)):
`set x 42` → Token(`ESC`, `"set"`), Token(`ESC`, `"x"`), Token(`ESC`, `"42"`)
→ `SegmentedCommand(texts=["set", "x", "42"])`.

**Phase 3 — IR lowering** ([KCS: lowering](kcs-lowering-dispatch.md), [KCS: expressions](kcs-expression-parsing.md), [KCS: namespaces](kcs-namespace-resolution.md)):
- Dispatch hierarchy: hooks → match/case → arg_roles fallthrough
- Expression parsing: Pratt parser with binding powers
- Namespace resolution: `normalise_qualified_name()` + context propagation
- [Examples 1–11](../../example-script-walkthroughs.md#example-1-set-x-42), [Example 22](../../example-script-walkthroughs.md#example-22-lowering-dispatch--arg_roles-and-command-classification)

**Phase 4 — CFG** ([Examples 5–10](../../example-script-walkthroughs.md#example-5-if-x--set-y-10-)):
`if`/`while`/`for`/`foreach` decompose into basic blocks with Branch/Goto
terminators.

**Phase 5 — SSA** ([Examples 5–10](../../example-script-walkthroughs.md#example-5-if-x--set-y-10-)):
Phi nodes at merge points, dominator tree, versioned definitions.

**Phase 6 — Core analyses** ([KCS: execution intent](kcs-execution-intent-model.md), [KCS: side-effects](kcs-side-effects-system.md)):
- SCCP resolves constants and marks unreachable blocks
- Execution intent classifies command substitutions (PURE/MAY_SIDE_EFFECT)
- Side-effects classification builds `CommandSideEffects` per command
- [Example 23](../../example-script-walkthroughs.md#example-23-execution-intent--command-substitution-classification), [Example 28](../../example-script-walkthroughs.md#example-28-side-effects-classification)

**Phase 7 — Specialised passes**:
- Interprocedural: [KCS](kcs-interprocedural-analysis.md), [Example 24](../../example-script-walkthroughs.md#example-24-interprocedural-analysis--summary-construction)
- Connection scope: [KCS](kcs-connection-scope.md), [Example 25](../../example-script-walkthroughs.md#example-25-connection-scope--cross-event-variable-flow-irules)
- Optimiser: [KCS](kcs-optimisation-passes.md), [Examples 13–19](../../example-script-walkthroughs.md#example-13-icip--interprocedural-constant-propagation-o103)
- Taint: [KCS](kcs-taint-analysis.md), [Example 12](../../example-script-walkthroughs.md#example-12-taint-analysis--httpheader-to-httprespond-subcommand-flow-and-spec)

**Phase 8 — Codegen** ([KCS](kcs-codegen-internals.md), [Example 27](../../example-script-walkthroughs.md#example-27-codegen-internals--labels-lvt-linearisation-peephole)):
LVT allocation → RPO linearisation → label emission → jump optimisation →
peephole → literal table → `FunctionAsm`.

### Diagnostics architecture

([KCS](kcs-diagnostics-calculation.md), [Diagnostics section](../../example-script-walkthroughs.md#how-diagnostics-are-calculated))

Two-phase delivery:
1. **Basic** (synchronous): W-codes from semantic analysis + style checks
2. **Deep** (background thread): O-codes, T-codes, S-codes, IRULE-codes

`DiagnosticScheduler` manages cancellation on new keystrokes and version
tracking to discard stale results.

### Data flow summary

```
Source text  ──────────────────────────────────────────────►  Bytecode
  "set x 42"                                                  push1/storeStk/done
       │                                                           ▲
       ▼                                                           │
  Token stream → SegmentedCommand → IR → CFG → SSA → Analyses → Codegen
                                                  │
                                                  ├─► Optimisations (O100–O126)
                                                  ├─► Taint warnings (T100+)
                                                  ├─► Shimmer warnings (S100+)
                                                  └─► Flow warnings (IRULE+)
```

## Decision rule

- If a fact can improve diagnostics/completions/quick fixes, model it at
  IR/CFG/SSA/pass level — not as codegen-only behaviour.
- New analyses that need SSA data belong in Phase 6 or 7.
- New diagnostics that only need tokens/AST belong in Phase 1 (basic).
- When adding cross-cutting features, trace data flow through the pipeline
  using the examples to verify each phase produces the expected output.

## Related docs

- [Example script walkthroughs](../../example-script-walkthroughs.md) (all 28 examples)
- [GLOSSARY.md](../../GLOSSARY.md)
- [compiler-architecture.md](../../compiler-architecture.md)
- Individual topic KCS files:
  - [kcs-lexing-segmentation.md](kcs-lexing-segmentation.md)
  - [kcs-error-recovery.md](kcs-error-recovery.md)
  - [kcs-expression-parsing.md](kcs-expression-parsing.md)
  - [kcs-ir-types-lowering.md](kcs-ir-types-lowering.md)
  - [kcs-lowering-dispatch.md](kcs-lowering-dispatch.md)
  - [kcs-command-registry.md](kcs-command-registry.md)
  - [kcs-cfg-construction.md](kcs-cfg-construction.md)
  - [kcs-ssa-construction.md](kcs-ssa-construction.md)
  - [kcs-sccp-core-analyses.md](kcs-sccp-core-analyses.md)
  - [kcs-constant-folding-type-inference.md](kcs-constant-folding-type-inference.md)
  - [kcs-control-flow-patterns.md](kcs-control-flow-patterns.md)
  - [kcs-interprocedural-analysis.md](kcs-interprocedural-analysis.md)
  - [kcs-connection-scope.md](kcs-connection-scope.md)
  - [kcs-namespace-resolution.md](kcs-namespace-resolution.md)
  - [kcs-codegen-internals.md](kcs-codegen-internals.md)
  - [kcs-taint-analysis.md](kcs-taint-analysis.md)
  - [kcs-optimisation-passes.md](kcs-optimisation-passes.md)
  - [kcs-diagnostics-calculation.md](kcs-diagnostics-calculation.md)
  - [kcs-dialects-events.md](kcs-dialects-events.md)
  - [kcs-data-structure-reference.md](kcs-data-structure-reference.md)
  - [kcs-execution-intent-model.md](kcs-execution-intent-model.md)
  - [kcs-side-effects-system.md](kcs-side-effects-system.md)
  - [kcs-cfg-ssa-fact-model.md](kcs-cfg-ssa-fact-model.md)
  - [kcs-compilation-unit-contracts.md](kcs-compilation-unit-contracts.md)
