# Step 1: the module-layers map

Part of [splitting tcl-compiler](../crate-split-design.md). Plan item 247 (the
gate and its tests), then item 273 (seeding and reviewing the ledger).
Builds on [step 0](00-test-dependencies.md). Steps 2–12 are measured against it.

## Problem

The production code of `tcl-compiler` is one cycle: 96 of its 121 modules form
one strongly connected graph. No crate builds on its own while a module of a
lower layer still names one of a higher layer. The branch may stay broken while
modules move, but the moves still need a work list: which edges block which
crate, and which types to move or invert to clear them.

## Design

A new gate in `rust/xtask/src/semantic_ownership.rs` beside `walker-ownership`,
`module-layers`, run by `cargo xtask check` and `make xtask-gates`. Two ledgers
in `docs/work/name-handling/ownership/`:

- `module-layers.tsv` gives each top-level module its target crate and the
  crates' order. It has one row per module, with these columns: `module`,
  `crate`, `rank`.
  - The ranks follow the [design](../crate-split-design.md#crates-bottom-up):
    `tcl-source` 1, `tcl-names` 2, `tcl-values` 3, `tcl-model` 4, `tcl-ir` 5,
    `tcl-binding` 6, `tcl-lowering` 7, `tcl-analysis` 8, `tcl-native-plan` 8,
    `tcl-diagnostics` 9, `tcl-optimiser` 9, and the unit crate 10.
  - Binding's rank against `tcl-lowering` is undecided until item 278. If it
    chooses co-location, ranks 6 and 7 become one crate; the seeded ledger
    reports binding → lowering and SSA edges either way, so 278 has its
    inventory.
  - Equal ranks may not name each other.
  - Splitting a module across two crates is recorded as rows for its
    submodules (`analyser::types` → `tcl-model`).
- `module-back-edges.tsv` is the work list. It has one row per (from-module,
  to-module) pair where the "from" crate's rank is not above the "to" crate's
  rank, with the count of production references.
  - A production reference is a `crate::` path in a `use` item or an expression,
    outside `#[cfg(test)]` items and test files, counted with the token scanner
    the other gates use.
  - During the split the gate only reports: it prints the ledger's diff against
    the tree and never fails.
  - From step 13 on, a new pair or a higher count fails, so the layers stay
    clean.
- Dev-dependency edges are checked from the start, not only after the split.
  A crate's `[dev-dependencies]` may not name a crate of a higher layer in the
  workspace's crate order (`deny.toml`'s wrapper lists). Today's cases (items
  246 and 269–272) are a known list that may only shrink; a new one fails.

The gate also reports the total, the number of pairs still at a nonzero count,
and, per target crate, whether it has no back-edges left. A crate with no
back-edges can be extracted.

## Seeding

The first ledger is generated from the tree as it stands (`cargo xtask
module-layers --write`) and reviewed by hand:

- whether each module's target crate is right;
- the split rows for modules whose parts go to different crates
  (`analyser::types`, `analyser::input`, `command_binding`'s key types).

Today's measurement, with test code excluded, is about 178 two-way pairs. The
largest are:

| Pair | Count |
|---|---|
| `command_binding` → `analyser` | ~280 |
| `command_binding` → `signature_scan` | 317 |
| `registry_invocation` → `command_binding` | 270 |
| `ir` → `command_binding` | 54 |

## Verification

- **Unit tests** in `semantic_ownership`:
  - a new back-edge is reported, and fails in enforcing mode;
  - a new upward dev-dependency fails;
  - test code is not counted;
  - equal-rank crates naming each other are reported.
- **Seed:** the seeded ledger passes on the tree it was generated from.

## What the next steps read from it

| Step | Done when |
|---|---|
| 3, 5 (vocabulary) | every back-edge into the types that move to `tcl-names` and `tcl-model` is gone |
| 4 (value kernel) | no binding module names `tcl_expr_eval` or `analyses` |
| 7 (binding against lowering) | 278's inventory lists every binding → lowering, SSA and value pair; after 250, `tcl-binding` → `tcl-diagnostics` reads 0 |
| 2, 6, 8–12 | each crate is extracted when its row in the per-crate report reads 0 |
