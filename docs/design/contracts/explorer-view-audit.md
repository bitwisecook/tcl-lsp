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
| `bounds` | 🐞 | `find_interval_bounds` has no `execution_intent` input and emits no divide-by-zero (W233) findings, so `divzero` is always `[]`. Honest about Rust today; the W233/intent analysis is unported. |
| `dataflow` | 🏗 + 🐞 | Functions sorted (Python emits top-level first) — presentation. `aliases` limited because memory-SSA isn't built here — a gap. |
| `interproc` | 🏗 (shape-gated) | Rust detects more (`depends(a,b)`, call-graph purity) where Python leaves `unknown`. Honest improvement; gated by shape. |
| `rendered` (`renderedProperties`) | 🐞 | Rust **over-reports** flags for command-substitution values (e.g. `HAS_DOUBLE_ESCAPE`). A correctness gap in the rendered-properties pass, not a design choice. |
| `opt` (`optimisations`) | 🏗 | The Rust optimiser is the production source of truth (drives `tcl --opt`, LSP code actions, the bytecode-compare gate) and intentionally improves on Python (folds interproc-constant `return $param`; prefers `O101 fold` over `O109`). |
| `gvn` | 🐞 | Rust GVN **under-detects** vs Python. Capability gap. |
| `shimmer` | ✅ | Strict modulo cosmetic message wording (normalised). |
| `taint` (`taintWarnings`) | 🏗 + 🐞 | Range points at the sink command (`eval $x`) — Rust's honest record location (🏗). Proc-order is not yet stable (🐞 non-determinism). |
| `taintTracking` | ✅ | Per-fn tainted-value lattice, strict. |
| `irules` (`irulesFlow`) | 🏗 | Fires only for the iRules dialect; finders are ported. |
| `eventOrder` | ✅ | Byte-for-byte (reuses `EventRegistry::{order_events, event_multiplicity}`). `[]` for plain Tcl. |
| `callouts` (`annotations` / `…ByLine`) | 🏗 + 🐞 | Aggregates the optimiser/shimmer/gvn/taint sources. **Omits dead-store callouts** (same O109 item as `cfgPostSsa`). *Action: add once dead stores are surfaced.* |
| `asm` / `asmOptimised` | 🏗 + 🐞 | Honest Rust bytecode. `sourceLine` is always 0 and the `Instruction` carries no per-op source span (`range` is null) — a codegen-metadata gap. Label numbering differs by design. Pinned by the bytecode-compare gate. |
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

2. **Track the gaps (🐞) as compiler work, not explorer pinning.** `bounds`
   (W233 / execution-intent), `gvn` (under-detection), `renderedProperties`
   (command-subst over-report), `taint` proc-order determinism, `dataflow`
   aliases (memory-SSA), `asm` per-instruction source spans. These stay
   Rust-pinned in the explorer (the view is honest) but belong on the
   analyser/codegen backlog in `docs/rust-rewrite.md`.

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
