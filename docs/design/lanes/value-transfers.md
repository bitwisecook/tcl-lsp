# Lane: value-transfers — slice 1 of the migration plan

The crash-insurance and handover note for the `value-transfers` lane. A
fresh agent resumes from this file and the `wip(value-transfers):` commits.

## Goal

Slice 1 of [value-transfers-migration.md](../compiler/value-transfers-migration.md)
§ *The slices*: the interface shapes in the registry, the value ingress that
preserves exact strings, the corrected derivation, the analysis context in
`FnLatticeKey`, the SCCP dispatcher re-expressed behind the interface
(behaviour-preserving), the ledger of transitional handlers, and the
`cargo xtask value-transfers` gate. Nothing beyond it.

Exit criteria: every existing `evaluate_def_*` and `sccp_with_builtin_folds`
test pins byte-identical results; `sccp.rs` matches no command name; the
inventory shows `incr` with a derived cell-update descriptor and an enabled
direct route, and `append` / `lappend` with the descriptor present and no
route enabled.

## Decisions taken, and why

- **Where the shapes live.** `rust/tcl-registry/src/value_transfer/` owns
  `CommandSemantics`, the declaration states, the answer shapes, the decline
  reasons (the page's list, plus the evaluation page's `NoRouteReason` and
  `Axis` payloads), `AnalysisContext`, `Budget`, and the routes. The
  compiler owns the driver (`rust/tcl-compiler/src/value_transfer.rs`): the
  lattice-backed `AnalysisInputs`, the projection of each typed statement
  to an invocation view, and the application of answers to definitions.
- **`EvalAnswer::Evaluated` boxes its outcome** (pedantic clippy: the
  variant was 648 bytes). Otherwise the page's names are kept.
- **The registry cannot name compiler types**, so the context's binding
  evidence is a registry-side `BindingEvidence` (the fields of the
  compiler's `CommandTrustSnapshot`), the correlating SSA identity is an
  opaque `ValueIdentity`, and the binding identity is a registry-side
  `BindingIdentity` mirroring `tcl_runtime_api::CommandBindingIdentity`.
- **The route is a method of `CommandSemantics`** (`route()`), and the
  catalogue of direct evaluators (`NativeEvalId`) records who implements
  each (`owner()`): the registry, or the compiler's transitional handler
  with the slice that retires it. That is how the driver dispatches to a
  handler it still owns without matching a command name, and how the
  inventory and the ledger read one fact.
- **The `Evaluated` route in value position runs under
  `NestedPolicy::EffectFreeOnly`**: an outcome with stores has no
  definition to land on in a `[…]`, so `set r [incr n]` keeps today's
  `Overdefined` rather than gaining a value the SSA does not model.
- **Derivation.** Only `NativeLowering::CellReadModifyWrite(u)` (a cell
  update, command scope only) and `Traits::DESTROYS_VARIABLE` (an unbind)
  derive. A resolved subcommand never inherits the command's cell-update
  descriptor: it selects an operation of its own. `ElementsOf` and
  `LOOP_LIST_HEADER` derive nothing; `foreach` and `lmap` declare
  `IterationSemantics` explicitly, answering the synthetic loop header (the
  `Call` the CFG builder marks with `foreach_groups`) with a one-list
  `Iterate` plan and declining the source layout until slice 5.
- **The `dict incr` / `dict append` / `dict lappend` subcommands derive
  nothing** (confirmed): a keyed update inside a dict is not the same
  operation as a cell read-modify-write, and an inheriting subcommand never
  falls through to the command's cell-update derivation, so they are gap
  rows in the inventory until slice 2, as the plan says.
- **The increment's slice-one arithmetic** lives in the registry: a
  canonical-integer base and step, `checked_add`, overflow declines. The
  step is read through the same round-trip classification as the base, so
  `+5`, `010`, and a whitespace-padded step decline where today's arm
  parsed them with Rust's `i64` grammar (a delta accepted below).
- **Binding validity is applied uniformly** by the driver: the optimiser's
  re-run (with `ModuleCommandMutations`) declines a renamed head for every
  resolved statement, `incr` and the loop header included; the
  mutation-fact-free shared lattice trusts every binding, as before.
- **The context key** (`AnalysisContextKey`: registry generation, overlay
  generation, the `CommandTrustSnapshot`, tier, evaluator revision) is one
  value per module (`build_with` computes it from the mutation scan) and
  rides `FnLatticeKey`. It could not ride `ModuleTraceFacts`, which
  `rust/tcl-lsp-core/src/graphs.rs` (the diagnostic-policy lane's file)
  also constructs; for this slice it goes through `FunctionBuildInputs` →
  `TraceInputs`, and the memo path's
  `build_with_param_constants_and_classes_under` takes it beside the trace
  facts as `ModuleAnalysisFacts` — the pair that folds into
  `ModuleTraceFacts` once that lane is done. In `tcl-lsp-db` the key is
  interned once per module (`ValueTransferContext`) and `FnLatticeKey`
  carries its id: inlined, the ~120-byte context on every per-edit key
  broke the plateau `tests/memory_growth.rs` pins (4232 bytes over the
  late window against a 4096 budget; the unmodified tree passes). The
  generations are 0 until slice 4 (overlay key) and the evaluator host
  reach the key.
- **Exact ingress.** `ExactValue::from_literal` is the one classification
  rule (decimal integer, round-trip); `sccp::parse_literal_value` is its
  lattice projection and no longer trims; `fold_assign_value` no longer
  trims the RHS; `strip_one_level` keeps the inside of a braced word exact.
- **The gate: clean files and a per-file ratchet.** `cargo xtask
  value-transfers` scans the compiler crate and the analysis and tooling
  tiers (`rust/tcl-lsp-core`, `tcl-mcp`, `tcl-cli`, `tcl-diagram`,
  `tcl-irules`, `tcl-irule-test`, `tcl-bigip`, `tcl-sslictcl`,
  `tcl-syntax`). Every file this slice touched is held *clean* — every
  recogniser-shaped site waived or gone (`CLEAN_FILES`: `sccp.rs`,
  `value_transfer.rs`, `command_binding.rs`, `compilation_unit.rs`,
  `dataflow_graph.rs`, `lib.rs`, `optimiser/propagation.rs`,
  `shimmer/mod.rs`; none needed an annotation). Every other scanned file
  is *ratcheted*: `RATCHET` pins its count of unwaived sites (113 across 44
  files at this commit), `--check` fails when a count rises or a pin is
  stale, and a pin is lowered — never raised, never added — beside the
  review that removes or waives the sites, with the `value-transfer-ok`
  annotations for what it reviews. The ledger in the migration plan
  (§ *The ratchet over unreviewed files*) carries the same table with who
  reviews each file — the slice whose work rewrites it, or the axis
  migration that waives it — and the gate holds the two equal. No site was
  hand-annotated in this slice beyond the transitional table's waiver;
  `native_lowering` joined the waiver axes for the codegen and lowering
  sites. A file whose sites all belong to one axis may carry a
  `value-transfer-ok(file):` header waiver; the inventory lists every
  waiver by axis.
- **Gap classification.** `KNOWN_GAPS` classifies every variable-writing
  command without semantics by the slice that gives it one: slice 2 (`set`,
  `const`, `lset`, `ledit`, `lpop`, the `dict` cell updates and their
  `::tcl::dict::` spellings); slice 5 (`regexp`, `regsub`, `scan`, `binary
  scan`, `lassign`, `dict with` / `update`, `array set`, `file stat` /
  `lstat`, `foreachLine`, the may-write commands `gets`, `chan gets`, `file
  tempfile`, `vwait`, `tk_optionMenu`, and the `trace` subcommands); slice 8
  (`array unset`, `array default`); slice 10 (`catch`); slice 13 (`global`,
  `variable`, `my variable`, `sharedvar`, `info default`); slice 4 (the
  EDA packs' `append_to_collection`, `remove_from_collection`,
  `foreach_in_collection`, declared in `.tclspec` once the spelling lands);
  slice 7 (tcllib and the Tcl-level library procedures). `append` and
  `lappend` need no entry: their derived descriptor already classifies them
  (*descriptor without a route*). An entry no row matches is stale and
  fails the gate.

## Site inventory

Done:

- registry: `value_transfer/` (decline, route, inputs, answers, context,
  declaration, cell_update, unbind, iteration, builtins); the `semantics`
  field on `CommandSpec`, `SubCommand`, `CommandForm`; `InvocationSemantics::value`;
  declarations on `foreach`, `lmap`, `list`, `format`, `llength`, `expr`,
  `string length`; `tests/value_transfers.rs` (the pinned route set).
- compiler: `value_transfer.rs` (driver, transitional handlers,
  `unbound_names`, `AnalysisContextKey`); `sccp.rs` (no command name;
  `evaluate_def_under`, `scan_defined_and_unbound`, exact ingress);
  `command_binding.rs` (`binding_evidence`); `compilation_unit.rs`
  (context threading); `TraceInputs::analysis_context`.
- lsp-db: `FnLatticeKey::analysis_context`; `function_lattice` builds under it.
- spec-studio: the `semantics` field on every studio surface, as an
  unrenderable draft key like `native_lowering`.
- xtask: `value_transfers.rs`, the Makefile target in `xtask-check`, the
  owner-resolution dispatch entry.

Remaining (in this slice): see the commit log and the report.

## Behavioural deltas accepted

- `incr x +5`, `incr x 010`, `incr x " 5"` (a literal step Rust's `i64`
  parser accepted) no longer fold: the step is classified by the one
  round-trip rule. `010` was wrong under 8.x anyway.
- A renamed `incr` / `foreach` (whole-module `rename incr …`) no longer
  folds in the optimiser's re-run (`RebindingSuspected`).
- `string length { a }` folds to 3 (the exact inside), not 1.
- `set p { again}` holds ` again` in the lattice (#2052): O100 propagates
  the exact text.
- `string le "abc"` (a unique-prefix subcommand) now folds through the
  resolver where the exact-name check declined; a command absent from the
  unit's profile (`lmap` under 8.4) no longer folds its loop variable.
- The `unset` fold's name set no longer carries `-nocomplain`, `--`, or a
  computed name's spelling (none of which a parameter can be named).

## Open uncertainties

- Whether the Explorer's `sccp` view should render route and decline reason
  in this slice (the plan's "part of this too if it fits"); the driver does
  not yet record per-statement explanations in `SccpResult`.
- The lint's enforced scope stops at `rust/tcl-compiler/src` while other
  lanes own the tooling crates; the measured counts are in the inventory.

## Status (2026-09-18): slice 1 landed

Committed as `wip(value-transfers): slice 1 — contracts and corrections`,
staged by path. Verified on a clean rebuild (the shared target was wiped
after the ENOSPC that stopped the first attempt):

- `cargo fmt --all`; `cargo clippy --all-targets -- -D warnings` clean for
  `tcl-compiler` with its dependencies, and for `tcl-lsp-db`,
  `tcl-spec-studio`, and `xtask` with `--no-deps`.
  `rust/tcl-lsp-core/src/diagnostic_policy.rs` (the diagnostic-policy
  lane's file) fails pedantic clippy (`struct_excessive_bools` on
  `Policy`), which blocks a dependency-inclusive lint of the three crates
  above it; not this lane's to fix.
- Tests: `tcl-registry` and `tcl-compiler` in full (every `evaluate_def_*`
  and `sccp_with_builtin_folds` test pins the same result under the
  dispatcher), `tcl-lsp-db` in full (`memory_growth` and `interned_gc`
  included), `tcl-spec-studio` in full (`reference_doc` regenerated
  `docs/references/command-spec/fields.md` for the `semantics` field;
  `spectcl_ports` documents the field as unrenderable on the `foreach`
  port), `xtask` in full (the gate's verdict tests included).
- Gates: `cargo xtask value-transfers --check` — OK, 8 files clean, 5 sites
  waived, 113 sites pinned across 44 ratcheted files, 6607 inventory rows;
  `cargo xtask kcs-index-links`; `cargo xtask owner-resolution`.
- The inventory (`docs/generated/value-transfers.md`) shows `incr` with the
  derived cell-update descriptor and the `direct cell-increment` route
  enabled (owner: registry), and `append` / `lappend` with the descriptor
  present and `none (unauthored)` — descriptor availability and enabled
  evaluation as separate columns.

Left to later slices, by name:

- The Explorer's `tcl explore --show sccp` route and decline rendering:
  slice 2, once the driver records per-statement explanations in
  `SccpResult`.
- `docs/design/compiler/diagnostics-calculation.md` is the
  diagnostic-policy lane's file; the row this slice would add is: *value
  transfers* — producer `LatticeDriver`
  (`rust/tcl-compiler/src/value_transfer.rs`) over the registry's
  `CommandSemantics`; consumers: every lattice reader (I230, I231, W124,
  W230–W233, S100–S110, T100–T106 through `FunctionUnit::sccp`; O100–O103,
  O112, O116, O118, O129 through the optimiser's re-run); context
  dependencies: `AnalysisContextKey` (binding evidence, registry and
  overlay generations, tier, evaluator revision); when unavailable, the
  invocation is `Overdefined` with its `DeclineReason` and no consumer
  changes its answer.
- `AnalysisContextKey` moves from `FunctionBuildInputs` / `TraceInputs` /
  `ModuleAnalysisFacts` into `ModuleTraceFacts` once
  `rust/tcl-lsp-core/src/graphs.rs` is free.
- The ratchet pins fall with the slices and axis migrations the ledger
  names; the registry and overlay generations reach the key in slice 4.
