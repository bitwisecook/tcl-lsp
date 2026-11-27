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
    `tcl-binding` 6, `tcl-lowering` 7, `tcl-analysis` 8, the unit crate
    (`tcl-unit`) 9, `tcl-native-plan`, `tcl-diagnostics` and `tcl-optimiser`
    10, and `tcl-db` 11 ([step 13](13-unit-and-db.md)).
  - Binding sits below `tcl-lowering`: item 278 chose inversion
    ([step 7](07-binding-fixpoint.md)). The seeded ledger reports binding →
    lowering and SSA edges as back-edges, which item 279 clears.
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

What the gate reads and writes:

```mermaid
flowchart LR
  tree["tcl-compiler production code"] --> scanner["token scanner: crate:: paths outside cfg(test)"]
  layers["module-layers.tsv: module, crate, rank"] --> gate["module-layers gate"]
  scanner --> gate
  manifests["Cargo.toml dev-dependencies"] --> gate
  gate --> backEdges["module-back-edges.tsv: the work list"]
  gate --> perCrate["per-crate report: back-edges left"]
  perCrate --> extract["a crate at 0 can be extracted"]
```

Its mode changes once, at step 13:

```mermaid
stateDiagram-v2
  [*] --> Reporting: item 247 lands
  Reporting --> Reporting: steps 2 to 12 move modules
  Reporting --> Enforcing: step 13 turns it on
  Enforcing --> [*]
  note right of Reporting
    prints the diff against the tree
    and never fails, except on a
    new upward dev-dependency
  end note
  note right of Enforcing
    a new pair or a higher count fails
  end note
```

A worked example. In the seeded ledger, `command_binding` is rank 6 and
`analyser` is rank 11 (`tcl-diagnostics`, above `tcl-semantic` at 10), so this line in `command_binding` counts as one
production reference on a back-edge:

```rust
use crate::analyser::ResolvedAnalysisInput;
```

The same line inside a `#[cfg(test)] mod tests` block, or in a `tests.rs`
file, counts nothing. The ledgers then hold rows like these (the counts are
today's, from the table below; `analyser::types` shows a split module):

```text
# module-layers.tsv
module	crate	rank
segmenter	tcl-source	1
analyser::types	tcl-model	4
ir	tcl-ir	5
command_binding	tcl-binding	6
signature_scan	tcl-binding	6
analyser	tcl-diagnostics	11

# module-back-edges.tsv
from	to	count
command_binding	analyser	316
ir	command_binding	54
```

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
| `command_binding` → `analyser` | 316 (production; it reads reports, step 7) |
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

## Review fixes (26 November 2026, deep review)

- **O2:** the example ranks `tcl-diagnostics` 11, above `tcl-semantic` (10, decision D-C); `tcl-db` is 12 ([step 13](13-unit-and-db.md)).

## As built (27 November 2026)

Item 247. The gate is `rust/xtask/src/semantic_ownership/module_layers.rs`,
run as `cargo xtask module-layers` by `cargo xtask check` and
`make xtask-gates` (0.5 s, under the test-profile `xtask` alias). It differs
from the design in these points:

- **Re-exports.** A `crate::` path is followed through `pub use` items
  (of any visibility) to the module that defines it before a row is matched.
  `use crate::analyser::ResolvedAnalysisInput` therefore counts against
  `analyser::input`, and `crate::command_binding::SourceNamespaceKey` against
  `command_binding::namespace_context`. Paths into another crate's re-export
  (`expr_ast`, `naming`) match no row and are not counted.
- **Rows.** A row names a top-level module, a submodule, or an item by the
  path that defines it; the longest matching row wins. The gate fails, in
  both modes, on a top-level module without a row, a row naming nothing, a
  crate with two ranks, and the `unassigned` crate that `--write` gives a new
  module.
- **Test code** is a `#[cfg(test)]` item, an item behind `test-support` or
  `any(test, feature = "test-support")`, and a file declared under a test
  gate.
- **Mode switch.** `module-layers.tsv`'s `# mode: reporting` line becomes
  `# mode: enforcing` at step 13. Reporting prints the ledger's diff;
  enforcing fails on any difference, a lower count included, so the ledger is
  lowered with the tree.
- **Dev-dependency order.** A dev-dependency of crate A on B is upward when B
  depends on A, or when A has dependents and B reaches a `deny.toml` layer
  (a ban with wrappers) that A does not. `xtask` and `xtask-backing` are
  tooling and are not checked. A self dev-dependency (`tcl-spectcl` enabling
  its own `programmed-packs`, item 318a; `tcl-vm` its own `test-support`) is
  not an edge. Items 246 and 269–272 left two cases, both `tcl-cshim`'s
  tests: `tcl-engine-tclvm` and `tcl-spec-hooks` (`dev-edges.tsv`).
- **The `test-support` body lint** (deep review W1) reads every workspace
  crate's `src`. A body is a `#[test]` function, or a `pub fn` that an
  enabling crate's test of the same name calls, directly or through a
  `macro_rules!` that declares `#[test]` functions (`tcl-compiler-it`'s
  `spectcl_seeded_tests!`). It finds 12 `tcl-compiler` modules
  (`test-support-bodies.tsv`). Item 267 counts 13: the 13th,
  `command_binding::spectcl_test_bodies`, only re-exports the others.
- **Duplicated crates.** For each crate whose `test-support` feature another
  crate enables, the gate prints how many workspace crates that test build
  compiles a second time beside a `tcl-lsp-server` build, item 296's
  measure. Today: `tcl-compiler`'s from `tcl-codegen` 2 and from
  `tcl-compiler-it` 9; `tcl-vm`'s from `tcl-engine-tclvm` 2, `tcl-irule-test`
  1, `tcl-spectcl` 4 and `tcl-vm-it` 1. The counts come from
  `cargo metadata --no-deps` and ignore optional dependencies.
- **Ranks** follow the crate table as revised (O2): `tcl-diagnostics` 11.
  `tcl-irules`, which takes `irules_checks` and `irules_event_checks`
  (item 285), is ranked 12. No module goes to `tcl-db`.

### Review decisions (item 273)

The seed has 154 rows: 118 top-level modules and 36 split rows. Each module
was checked against the [crate table](../crate-split-design.md#crates-bottom-up)
and steps 2–13. A module whose parts go to different crates is split by
submodule where the parts are files, and by item where they share a file.
An item row decides only where references to the item count; the item's own
code counts under its file's row.

| Crate (rank) | Modules | Split rows | Back-edges out (refs, pairs) |
|---|---:|---:|---|
| `tcl-source` (1) | 6 | 3 | 4, 2 |
| `tcl-names` (2) | 1 | 9 | 25, 11 |
| `tcl-values` (3) | 0 | 7 | none |
| `tcl-model` (4) | 11 | 4 | 229, 17 |
| `tcl-ir` (5) | 3 | 0 | 68, 13 |
| `tcl-binding` (6) | 25 | 1 | 286, 40 |
| `tcl-lowering` (7) | 18 | 0 | 22, 9 |
| `tcl-analysis` (8) | 17 | 2 | 67, 15 |
| `tcl-unit` (9) | 18 | 7 | 30, 8 |
| `tcl-native-plan` (10) | 7 | 0 | none |
| `tcl-optimiser` (10) | 3 | 0 | 2, 1 |
| `tcl-diagnostics` (11) | 8 | 2 | 66, 2 |
| `tcl-irules` (12) | 1 | 1 | none |

The split rows:

- **`analyser::types` and `analyser::input`** go to `tcl-model`.
  `AnalysisResult` stays in `tcl-diagnostics` (the item row
  `analyser::types::AnalysisResult`), so binding's 119 references to it
  (`command_binding` 76, `registry_invocation` 39) are step 7's body-report
  work, and `analyser::types` naming it (5) shows that it must leave
  `types.rs`. `analyser::types` → `signature_scan` (102),
  `command_binding` (51) and `registry_invocation` (37) are step 5's
  receipts work (`ProcDef` and `ClassDef` generic over binding's receipts).
  `analyser::irules_event_checks` goes to `tcl-irules` (item 285).
- **`command_binding`'s key types** go to `tcl-names`: the submodules
  `namespace_context` (`SourceNamespaceKey`, `SourceCommandKey`),
  `command_identity` (`CommandAllocationSite`), `source_origin` and
  `executed_script_source`, and the item `command_binding::BindingKind`,
  which `command_binding.rs` defines. `SourceInvocationBinding` stays with
  binding, so `ir` → `command_binding` (33) is the `Facts` work of step 6.
  Beside them: `var_resolve::cell_key` and the item
  `var_resolve::VariableExecutionFrame`, `signature_scan::scope`
  (`SignatureNamespaceScope`) and the item
  `signature_scan::original_name::SignatureSourceNameKey`.
  `signature_scan::scope::SignatureSourceCommand` goes to `tcl-model`.
- **The value lattice** in `analyses.rs`: the items `LatticeValue`,
  `ConstValue` and `LatticeKind` go to `tcl-values`; the rest of `analyses`
  (the result records) goes to `tcl-model`. With the lattice in its own rows,
  step 4's "no binding module names `analyses`" is read as no back-edge into
  either. `tcl_expr_eval` is ranked with `tcl-analysis`: its kernel moves to
  `tcl-values` under a new owner (D-G), so binding → `tcl_expr_eval` (38
  from `command_binding`, 17 from `native_numeric`, 7 from
  `math_function_binding`) is step 4's work list. The step's other moves
  down are item rows too: `codegen_helpers::split_list_values`,
  `native_lowering::cells::{variable_reference_place, CellPlace}` and
  `shimmer::hints::is_pure_intrep`.
- **Other splits the steps name:** `ir::word_expr`, `ir::provenance` and
  `optimiser::MAX_OPTIMISER_WALK_DEPTH` to `tcl-source`; `ssa::uses` to
  binding; `ssa::source_view`, `value_transfer::queries`,
  `lattice_rebase::rebase_function_unit`, the unit-level
  `type_infer::infer_function_return_type*`, `optimiser::elimination` and
  `optimiser::method_barrier` to `tcl-unit`; the two
  `gvn::resolved_invocation_is_*gvn_candidate` predicates to
  `tcl-analysis`; `taint::is_irules_dialect` to `tcl-model`;
  `source_graph::current_analysis` to `tcl-diagnostics`.
- **Totals.** 799 references in 118 pairs. Back-edges per crate pair, the
  work list for steps 2–12: `tcl-model` → `tcl-binding` 203,
  `tcl-binding` → `tcl-diagnostics` 162, `tcl-binding` → `tcl-analysis` 67,
  `tcl-diagnostics` → `tcl-irules` 66, `tcl-ir` → `tcl-binding` 65,
  `tcl-analysis` → `tcl-unit` 56, `tcl-binding` → `tcl-unit` 31,
  `tcl-binding` → `tcl-lowering` 25, `tcl-unit` → `tcl-diagnostics` 24,
  `tcl-names` → `tcl-binding` 23, and 18 smaller pairs
  (`module-back-edges.tsv`).

Placements the design leaves open, decided by each module's edges (what it
names, who names it):

- `site_claims` (no edges; step 2's open question 2): `tcl-source`, until
  item 261 decides on the registry core.
- `execution_region` and `native_compilation_admission` (annotations that
  step 6 puts into `Facts`): `tcl-binding`.
- `types` (`TypeLattice`, step 5): `tcl-model`. `object_types`:
  `tcl-unit`, as step 13 says unless step 5 finds otherwise.
- Not in the design: `var_scoping` (token helpers for `global` and
  `upvar`, names nothing) to `tcl-binding`; `loops` (names `ssa`, `cfg`) to
  `tcl-lowering`; `expression_rewrite` and `scan_predicate` to
  `tcl-analysis`; `inline_uplevel`, `path_concat`, `uri_split`,
  `value_provenance`, `rendered_properties` and `durable_inventory` (each
  names or is named only by `compilation_unit` and `taint`) to `tcl-unit`;
  `increment_rewrite` and `read_schedule` (used only by `optimiser`) to
  `tcl-optimiser`; `dead_stores`, `regex_source`, `script_arg`,
  `source_command_observation` and `table_value_provenance` (used only by
  `analyser`) to `tcl-diagnostics`.

Open: `body_analysis_capsule` → `optimiser` (4) waits for step 12's
`optimiser_report(&capsule)`; `segmenter` → `analyser` (4) for step 2's
`segment_with_recovery` move; `tcl-binding` → `tcl-diagnostics` (162) is
item 250's done-when. An item row's own code cannot be placed apart from its
file; `BindingKind` and the `analyses` lattice name nothing above them today.
