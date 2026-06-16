# Explorer view audit — represent the Rust compiler, not Python

The compiler explorer's JSON contract was originally fixed by the Python
explorer (`tooling/cli/serialise.py`). As the pipeline is ported to Rust
(`EXP*`), the explorer must show **what the Rust compiler actually computes**,
not contort its output to match Python. This document audits every view and
classifies each divergence so the parity gate, the frontend tabs, and the
backlog all reflect that intent.

## Classification

- **✅ Faithful** — Rust matches Python byte-for-byte (strict parity gate).
- **🏗 By design** — Rust's backend genuinely differs (and is the production
  source of truth). The view honestly shows the Rust result; a Python
  differential is the *wrong* gate, so the key is Rust-pinned (`_NO_PARITY`)
  or shape-gated. **Not a bug — do not "fix" toward Python.**
- **🐞 Gap** — Rust's analysis is *weaker* than Python (under-/over-reports).
  The view is honest about Rust today, but the backend should improve. Tracked
  as a capability gap, separate from 🏗.
- **⛔ Blocked** — depends on an unported backend; degrades to `null`/empty.
- **🗑 Dropped** — the view represents a Python-only structure Rust doesn't
  have; removed from the Rust contract.

## Per-view findings

| View | Status | Notes |
| --- | --- | --- |
| `meta` | ✅ | Tab table, dialects, severities. Lists 23 views (Python's 24 minus `greentree`). |
| `greentree` | 🗑 | **Dropped.** Rust's parser produces one red-green CST (`parsing::syntax`); there is no separate legacy green-token tree. `cst` is the sole parse-tree tab. Removed from `VIEW_META`, `view_tree`, and the shared frontend. |
| `cst` | ✅ | The canonical red-green CST — Rust's actual parse tree. |
| `segments` | ✅ | `SegmentedCommand` list, byte-for-byte. |
| `ir` / `irOptimised` | ✅ / 🏗 | IR is faithful. `irOptimised` re-runs the **Rust** optimiser (source of truth). |
| `cfgPreSsa` / `…Optimised` | ✅ / 🏗 | Pre-SSA CFG faithful (orphan-exit normalised in the harness — a CFG-builder convergence item). |
| `cfgPostSsa` / `…Optimised` | 🏗 + 🐞 | SSA phis/uses/defs/lattice are honest Rust. **`analysis.deadStores` is empty** — Rust has no standalone liveness `FunctionAnalysis`; dead stores are computed by the optimiser as **O109** findings. *Action: populate from O109 (see below).* `constantBranches`/`unreachableBlocks` come from SCCP. |
| `loops` | ✅ | Order normalised (set-derived); content strict. |
| `types` | ✅ | One characterised Overdefined-`*` normalisation; otherwise strict. |
| `intervals` | ✅ | Byte-for-byte over `compute_intervals`. |
| `bounds` | ✅ | **Converged → strict.** Out-of-range index findings (W230/W231/W232) plus the W233 divide-by-zero finder (`interval_bounds::find_divide_by_zero`), which the explorer now populates from the same SCCP-executable fixpoint. (Exposed and fixed a latent SCCP fold bug: `string length $v` was folding on the unresolved `$v` text instead of the lattice constant.) |
| `dataflow` | 🏗 | Functions ordered `::top`-first then alphabetically (matches Python), `typeInfo` projected from the type lattice, and `aliases` surfaced via memory-SSA (`.with_memory_ssa()` is now run in the explorer pipeline). Remaining divergence is Rust's honest def-use representation: version-0 parameter nodes and def-use edge ordering. Rust-pinned. |
| `interproc` | 🏗 (shape-gated) | Rust detects more (`depends(a,b)`, call-graph purity) where Python leaves `unknown`. Honest improvement; gated by shape. |
| `rendered` (`renderedProperties`) | ✅ | **Converged → strict.** Command-substitution values use a minimal `HAS_INTERPOLATION` baseline (plus registry hints) instead of the conservative may-mask that over-reported every flag; `HAS_DOUBLE_ESCAPE` is now detected on rendered literal words, and `analyse_literal` flags `HAS_LITERAL_SPACE` to match the Python const path. |
| `opt` (`optimisations`) | 🏗 | The Rust optimiser is the production source of truth (drives `tcl --opt`, LSP code actions, the bytecode-compare gate) and intentionally improves on Python (folds interproc-constant `return $param`; prefers `O101 fold` over `O109`). |
| `gvn` | 🏗 | **Under-detection fixed:** subcommand-aware purity in `classify_side_effects` lets GVN see redundant ensemble computations (`string length`, `dict get`, …) it previously treated as unknown-writes. Remaining divergence is the finding *range*: Rust records the enclosing statement span for an embedded `[…]` substitution where Python pins the substitution itself (presentation, like the taint sink-range). Expr-statement CSE (`[expr {…}]`) is still Python-only. Rust-pinned. |
| `shimmer` | ✅ | Strict modulo cosmetic message wording (normalised). |
| `taint` (`taintWarnings`) | 🏗 | Range points at the sink command (`eval $x`) — Rust's honest record location. **Proc-order non-determinism fixed:** `CompilationUnit::functions()` now yields procedures in qualified-name order, so cross-function diagnostic order is reproducible. Rust-pinned for the by-design sink-range location. |
| `taintTracking` | ✅ | Per-fn tainted-value lattice, strict. |
| `irules` (`irulesFlow`) | 🏗 | Fires only for the iRules dialect; finders are ported. |
| `eventOrder` | ✅ | Byte-for-byte (reuses `EventRegistry::{order_events, event_multiplicity}`). `[]` for plain Tcl. |
| `callouts` (`annotations` / `…ByLine`) | 🏗 + 🐞 | Aggregates the optimiser/shimmer/gvn/taint sources. **Omits dead-store callouts** (same O109 item as `cfgPostSsa`). *Action: add once dead stores are surfaced.* |
| `asm` / `asmOptimised` | 🏗 | Honest Rust bytecode. **Per-op source spans now plumbed:** the emitter stamps each `Instruction` with the byte `source_span` of the construct it lowered from (statement / branch condition / return value), so the explorer emits a real `range` + 1-based `sourceLine` and the GUI's click-to-source + source-comment grouping light up. Synthetic ops with no source (loop-result pushes, fallthrough jumps, padding NOPs) keep `range: null` / `sourceLine: 0` — honest. Label numbering still differs by design. Pinned by the bytecode-compare gate (the span is metadata; the instruction stream is unchanged). |
| `wasm` / `wasmOptimised` | ⛔ | The WASM emitter is unported (≈14K Python LOC). `null` today; lights up when the emitter chunk lands. |
| `stats` | 🏗 + 🐞 | `deadStores` is `0` (same O109 item); warning counts follow the Rust analyses. |

## Action backlog (derived from this audit)

1. **Surface dead stores honestly.** ✅ **Done.** Rust computes dead stores as
   optimiser **O109** findings (`optimiser/elimination.rs`). Added the public
   `optimiser::DeadStore` type and `find_dead_stores(cu, registry, dialect)`
   (a collector on `PassContext` populated alongside the O109 emit, so it
   reuses the optimiser's full purity / scope-alias / place-model / cross-event
   suppression — not a naive SSA re-derivation). The explorer now populates
   `cfgPostSsa.analysis.deadStores`, `stats.deadStores`, and the `deadStore`
   callouts from it.

2. **Analyser capability gaps (🐞) — all addressed.**
   - ✅ **`asm` per-instruction source spans.** The emitter stamps
     `Instruction::source_span` from the lowered construct's span
     (`CodegenCtx::current_span`, set per statement / terminator and reset
     for synthetic ops); the explorer maps it to the per-op `range` +
     `sourceLine`.
   - ✅ **`bounds` W233 divide-by-zero.** `interval_bounds::find_divide_by_zero`
     mirrors Python (same interval fixpoint + executable-block filter); the
     explorer populates `divzero`. Also fixed a latent SCCP fold bug where
     `string length $v` folded on the unresolved `$v` text. `bounds` is now a
     **strict** parity view.
   - ✅ **`renderedProperties` over-report.** Command-substitution values use a
     minimal `HAS_INTERPOLATION` baseline; `HAS_DOUBLE_ESCAPE` /
     `HAS_LITERAL_SPACE` detection added. Now a **strict** parity view.
   - ✅ **`gvn` under-detection.** Subcommand-aware purity in
     `classify_side_effects` surfaces redundant ensemble computations
     (`string length`, `dict get`, …). Residual: embedded-substitution
     finding *range* is statement-level (presentation), and expr-statement
     CSE is still Python-only.
   - ✅ **`taint` proc-order determinism.** `CompilationUnit::functions()`
     iterates procedures in qualified-name order. (The sink-range location
     stays 🏗 by design, so the view remains Rust-pinned.)
   - ✅ **`dataflow` ordering + aliases.** `::top`-first function order,
     `typeInfo` from the type lattice, and memory-SSA-backed `aliases`
     (the pipeline now runs `.with_memory_ssa()`). Residual divergence is
     Rust's honest def-use shape (v0 parameter nodes, edge ordering).

3. **Add Rust-native views** the Python contract never had. ✅ **Done.** Three
   additive keys (skipped by the parity harness via `_RUST_NATIVE_KEYS`, each
   with a `meta` tab, a `view_tree` builder, and a GUI tab):
   - `optimiserPasses` — each `PassId` in execution order with the rewrites it
     produced (`optimiser::optimise_by_pass`); reveals Rust's multi-pass
     structure.
   - `structuralIndex` — the lexer's structural pre-scan (command boundaries,
     bracket/brace balance, inert literal-delimiter spans) that drives
     incremental reparse (`tcl_lexer::structural_index`).
   - `sourceMap` — the `LineIndex` span model (line-start table) that powers
     O(1) offset↔line:col resolution.

4. **WASM emitter (⛔)** — unblocks `wasm`/`wasmOptimised` and the `run` view;
   tracked under the broader `EXP*` / `tcl-vm` work.

## Parity-gate policy

The differential harness (`tests/test_explorer_rust_parity.py`) keeps a
three-tier gate: strict byte-equality for ✅ views, shape-only for
documented 🏗 shape divergences, and `_NO_PARITY` for 🏗/🐞/⛔ keys (pinned by
Rust unit tests + the backends' own suites). The `_NO_PARITY` comments cite
this audit's classification so a reader can tell "Rust by design" from
"Rust gap to fix".
