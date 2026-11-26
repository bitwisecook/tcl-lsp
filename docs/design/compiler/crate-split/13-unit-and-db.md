# Step 13: the unit crate, the database and tcl-compiler-it

Part of [splitting tcl-compiler](../crate-split-design.md). Plan items 258
(retire the façade and name the unit crate), 286 (`tcl-lsp-db` becomes
`tcl-db`) and 287 (`tcl-compiler-it`'s dependencies), in that order. Builds
on [step 12](12-diagnostics-optimiser.md). [Step 14](14-caching.md) builds on
it.

**Decisions:**

- The unit crate is named **`tcl-unit`**.
- It sits above `tcl-analysis` and below `tcl-native-plan`,
  `tcl-diagnostics` and `tcl-optimiser`, not at the top of the stack.
- `tcl-db` is the top of the analysis stack.

## Problem

### The unit is not at the top

The overview draws the unit crate (`compilation_unit`, `source_graph`,
`auto_path_eval`) above `tcl-diagnostics` and `tcl-optimiser`. The ledger
(measured as in step 7, 26 November 2026) shows otherwise:

| Edge | Refs | Meaning |
|---|---|---|
| `analyser` → `compilation_unit` | 175 | the analyser builds units and reads `FunctionUnit` |
| `optimiser` → `compilation_unit` | 42 | |
| `common_aot_plan` → `compilation_unit` | 6 | the native plan reads units |
| `irules_checks` → `compilation_unit` | 10 | |
| `compilation_unit` → `analyser` | 40 | all vocabulary for `tcl-model`: `ResolvedAnalysisInput`, settings |
| `compilation_unit` → `optimiser`, `irules_checks`, `compiler_checks` | 0 | |

So the unit is what the analyser, the optimiser and the native plan consume.

- **`source_graph`** (638 lines) mixes two kinds of function. Some are IR
  helpers (`event_body_for_procedure`, `procedure_body_for_declaration`,
  `hosted_procedure_body_for_function`, used by `optimiser` and `analyser`).
  `current_analysis(source, &AnalysisResult)` is an analysis-result query,
  with 12 uses in the analyser.
- **`auto_path_eval`** (2,795 lines) produces an `AnalysisResult` field
  (`PathConstantAssignments`) and reads `AnalysisResult`.

Both of those halves belong to `tcl-diagnostics` (step 12).

### The façade

`tcl-compiler/src/lib.rs` re-exports other crates:

- `tcl_expr::ast as expr_ast` and `tcl_expr::expr::parser as expr_parser`
  (`lib.rs:134–135`);
- `tcl_syntax::naming` (`:167`);
- `tcl_expr::{BinOp, ExprOffset, UnaryOp}` (`:231`);
- `ExprNode`, `parse_expr`, `parse_expr_for_profile` (`:228–229`).

It also re-exports its own items at the root: `rebase_script`, the
`completion` lattice, `ir::{Module, …}` and the `tcl_expr_eval` API.

Outside the crate, `tcl_compiler::<module>` appears 5,678 times.

### tcl-lsp-db

`tcl-lsp-db` is 18,684 lines, with 86 `#[salsa::tracked]` queries and the
inputs `SourceFile`, `AnalyserConfig`, `Project`, `EvaluatorEpoch` and
`OverlayEpoch`. Only `tcl-lsp-server` depends on it today. It names
`tcl-lsp-core` in 7 production files:

| What | Where (`tcl-lsp-db/src`) | Kind |
|---|---|---|
| `semantic_tokens::{full_with_cu_and_analysis, full_with_cu_and_facts, WorkspaceTokenFacts, NamedInstanceMap, SemanticTokens, VarNameArgRoles}` | `display_queries.rs`, `project_token_inputs.rs`, `lib.rs:241` | LSP feature |
| `folding::{folding_ranges_with_analysis, FoldingRange}` | `display_queries.rs:30, 352` | LSP feature |
| `document_symbols::{document_symbols_from_analysis, DocumentSymbol}` | `display_queries.rs:29, 48` | LSP feature |
| `profile_for_dialect`, `stated_profile_for_dialect`, `optional_profile_for_dialect`, `environment_for_dialect`, `registry_for_dialect` | `compilation_units.rs`, `check_solve.rs`, `compiler_diagnostics.rs`, `project_calls.rs`, `lib.rs:656–664` | thin wrappers over `tcl_registry::model` (`tcl-lsp-core/src/lib.rs:167–318`) |
| `source_graph::conditional_project_source_target` | `project_calls.rs:92` | path policy over a `registry_invocation` source load |
| `workspace_index::WorkspaceIndex` | `lib.rs:904` (a doc link only) | |
| `diagnostic_policy::Finding`, `diagnostic_subject` | tests only | |

## Design

### tcl-unit (item 258)

`rust/tcl-unit` is what remains of `tcl-compiler` after steps 2–12, under a
new name. It contains:

- `compilation_unit` (7,866 lines). Its build is the binding and value
  fixpoint driver ([step 7](07-binding-fixpoint.md)). It also builds the
  `FunctionUnit` and `CompilationUnit` records.
- the whole-unit passes ([step 10](10-tcl-analysis.md)): `interprocedural`,
  `taint`, `taint_interproc`, `shimmer`, `gvn` and `var_escape` (about 41k
  lines);
- `unit_scope` (3,361), `ssa::source_view`, `value_transfer::queries`,
  `connection_scope` (893), the `FunctionUnit` part of `lattice_rebase`, and
  the data part of `body_analysis_capsule`;
- `object_types` (4,375), unless step 5 finds it is model vocabulary;
- the IR half of `source_graph`, the optimiser's `elimination` scans and
  `method_barrier` (step 12).

That is about 65k lines.

`lib.rs` declares its modules and an `api` module of its own items. It has
no `pub use` of another workspace crate and no module that only forwards
one. The `tcl_expr` and `tcl_syntax` re-exports above are deleted, and their
users name `tcl-expr` and `tcl-syntax` directly.

**The name: `tcl-unit`, not `tcl-compiler`.** Reasons:

1. **What it holds.** After the split the crate holds the compilation unit,
   not the compiler. A crate called `tcl-compiler` that is the eighth layer of
   twelve would mislead every reader of a `Cargo.toml`.
2. **Old paths fail to compile.** Every `tcl_compiler::` path then fails.
   `retired_api_gate` gets a one-line rule (no `tcl_compiler` anywhere) in
   place of a per-module list, and docs that cite
   `rust/tcl-compiler/src/...` become stale in an obvious way.
3. **No extra cost.** The 5,678 outside references are rewritten in steps
   2–12 anyway. The rename touches only the unit's own consumers: the
   `compilation_unit` users in `tcl-codegen` (27 files), `tcl-lsp-core` (25),
   `tcl-lsp-db` (12), `tcl-lsp-server` (6), and 1–4 each in nine more crates.

**Order.** The steps still run in the plan's order. The unit modules simply
stay in `tcl-compiler` while steps 11 and 12 extract the crates above them,
and those crates depend on the shrinking `tcl-compiler`. Step 13 then renames
what is left. The `module-layers` ranks change: the unit crate is rank 9,
`tcl-native-plan`, `tcl-diagnostics` and `tcl-optimiser` are rank 10, and
`tcl-db` is 11.

### tcl-db (item 286)

`rust/tcl-lsp-db` is renamed `rust/tcl-db`. Its `tcl-lsp-core` dependency is
removed as follows:

- **The profile helpers move down** to the registry core, beside
  `tcl_registry::model::resolve_environment`, and later into `tcl-catalogue`
  (item 303). `tcl-lsp-core` keeps no copy.
- **`conditional_project_source_target` moves down** to `tcl-binding`'s
  `query` module, beside `OriginalProjectSourceLoad`. It is source-path
  policy, not an LSP feature.
- **The feature queries move up.** Semantic tokens, folding and document
  symbols become salsa queries defined in `tcl-lsp-core` over a database
  trait that `tcl-db` exports:

```rust
// tcl-db
#[salsa::db] pub trait Db: salsa::Database {
    fn registry_inputs(&self) -> RegistryInputs;      // step 14
}
#[salsa::tracked] pub fn document_analysis(db: &dyn Db, file: SourceFile) -> Arc<AnalysisResult>;
#[salsa::tracked] pub fn compilation_unit(db: &dyn Db, file: SourceFile) -> Arc<CompilationUnit>;

// tcl-lsp-core
#[salsa::tracked] pub fn semantic_tokens(db: &dyn tcl_db::Db, file: SourceFile) -> Arc<SemanticTokens>;
#[salsa::tracked] pub fn folding_ranges(db: &dyn tcl_db::Db, file: SourceFile) -> Arc<Vec<FoldingRange>>;
#[salsa::tracked] pub fn document_symbols(db: &dyn tcl_db::Db, file: SourceFile) -> Arc<Vec<DocumentSymbol>>;
```

  `NamedInstanceMap`, `WorkspaceTokenFacts` and the project token inputs
  (`project_token_inputs.rs`) move with them. `tcl-lsp-server` holds the
  concrete database, which implements both crates' traits.
- **The test-only uses** (`diagnostic_policy::Finding`, `diagnostic_subject`)
  move with their tests into `tcl-lsp-core`'s, or into item 271's test crate.

`tcl-db` depends on `tcl-diagnostics`, `tcl-optimiser`, `tcl-unit` and
below. It names no native-plan module today (`analyser` 136 references,
`ir` 39, `command_binding` 31, `compilation_unit` 28, `body_analysis_capsule`
22, …), so it does not need `tcl-native-plan`. Its users
are `tcl-lsp-server` (62 files today), `tcl-mcp` and the CLI. MCP and the
CLI analyse through it instead of their own `Analyser` calls, which
[step 14](14-caching.md) needs for "one database".

### tcl-compiler-it (item 287)

Step 0 creates `tcl-compiler-it` with dev-dependencies `tcl-compiler`
(`test-support`), `tcl-spectcl`, `tcl-engine-tclvm`, `tcl-vm` and
`tcl-test-support`. After the split, its tests name these crates:

| Tests | Crates named |
|---|---|
| `it_2/value_transfer_witnesses*` | `tcl-diagnostics`, `tcl-optimiser`, `tcl-unit`, `tcl-analysis`, `tcl-lowering`, `tcl-ir`, `tcl-values`, `tcl-model` |
| `it_3/optimiser/*` | `tcl-optimiser`, `tcl-values` |
| `it_4/analyser/command_resolution.rs` | `tcl-diagnostics`, `tcl-binding` |
| `it_5/cfg.rs` | `tcl-ir`, `tcl-lowering`, `tcl-expr` |
| `benches/value_transfers.rs` | `tcl-unit`, `tcl-lowering`, `tcl-optimiser`, `tcl-analysis` |
| the 12 SpecTcl `src` blocks (`jim_stock_class`, `jim_wrapper_body`, `declaration_preview/static_read_child`, …) | `tcl-binding` (`test-support`) |

Its dev-dependencies become exactly that list plus `tcl-spectcl`,
`tcl-engine-tclvm`, `tcl-vm` and `tcl-test-support`, with `test-support`
enabled only on `tcl-binding` and `tcl-unit`. No test moves back into the
crate it tests yet, because every test here needs `tcl-spectcl` (Jim's core
surface through `core_surfaces::ensure()`) or `tcl-engine-tclvm`, and both sit
above.

The 12 binding blocks can move back into `tcl-binding` once two things hold:

- Jim's surface comes from `tcl-dialect-jim` (registry split, item 264);
- the SpecTcl loader runs on `tcl-source` (item 281).

At that point their only extra dev-dependency, `tcl-dialect-jim`, sits below
`tcl-binding`. Item 287 records that as the follow-up.

## Work list

1. Move `auto_path_eval` and the `AnalysisResult` half of `source_graph` to
   `tcl-diagnostics`, if step 12 has not already done so.
2. Delete the `pub use` re-exports in `lib.rs`, and rewrite their users to
   `tcl-expr` and `tcl-syntax`.
3. Rename `rust/tcl-compiler` to `rust/tcl-unit`. Rewrite the consumers, set
   `retired_api_gate` to forbid `tcl_compiler` entirely, and turn the
   `module-layers` gate to enforcing (step 1's "from step 13 on").
4. Item 286:
   - move the profile helpers down and `conditional_project_source_target`
     to `tcl-binding`;
   - move the feature queries to `tcl-lsp-core` over `tcl_db::Db`;
   - rename the crate to `tcl-db`;
   - point `tcl-mcp` and `tcl-cli` at it.
5. Item 287: rewrite `tcl-compiler-it`'s `Cargo.toml` to the table above.

## Verification

- **Item 258.**
  - `cargo xtask module-layers --check` reports zero back-edges.
  - `lib.rs` has no `pub use` of another workspace crate.
  - Every consumer names only the crates whose types it uses.
  - The `cargo-deny` layer gate passes with a `wrappers` list per crate.
  - The name is recorded here and in the overview.
- **Item 286.** `rust/tcl-db` exists and `rust/tcl-lsp-db` does not.
  `cargo tree -p tcl-db -e normal` lists no `tcl-lsp-core`.
  `tcl-lsp-server`, `tcl-mcp` and `tcl-cli` depend on `tcl-db`.
- **Item 287.** `cargo tree -p tcl-compiler-it -e dev` lists only the crates
  above, and the dev-dependency check passes.
- **Measured.**
  - The cones of `tcl-unit` and `tcl-db`.
  - The crates rebuilt after an LSP-feature edit (expected: `tcl-lsp-core`
    and above, not `tcl-db`) and after a `tcl-db` query edit.
  - Release latency per keystroke on the analysis-performance corpus,
    before and after moving the feature queries.

## Risks

- **Salsa queries in a downstream crate.** Salsa 0.28 supports tracked
  functions over a `dyn` database trait from another crate. The server's
  concrete database must implement both traits, and its `memory_usage()`
  probe must still see every ingredient.
- **MCP and the CLI moving onto `tcl-db`** changes their start-up path. Golden
  diffs of their output must be byte-identical.

## Corrections to the overview

- The unit crate sits below `tcl-native-plan`, `tcl-diagnostics` and
  `tcl-optimiser`. The diagram should show it under them, with `tcl-db` alone
  at the top.
- Item 258's text, "it keeps only `compilation_unit`, `source_graph` and
  `auto_path_eval`", should read: it keeps `compilation_unit`, the whole-unit
  passes, `unit_scope` and the IR half of `source_graph`. `auto_path_eval`
  and the `AnalysisResult` half of `source_graph` go to `tcl-diagnostics`.
- Step 1's rank list changes as given under "Order".
