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

## Slice 2 — the direct vertical slice

[value-transfers-migration.md](../compiler/value-transfers-migration.md)
§ *The slices*, item 2: `string range` and `incr` over `ConstOps` and its
admissibility adapters, then `append` / `lappend`; the consumers that
carried their own `incr` models read the resolved semantics; the memoised
path runs the same context; the correlated finite-set limit is in force.

### Decisions taken, and why

- **`ConstOps` lives in the registry**
  (`rust/tcl-registry/src/value_transfer/const_ops.rs`): the one live
  `ValueOps` implementation, byte-exact (`ConstValue { bytes: Rc<[u8]>,
  repr }`), poisoned by the first fault so `take` declines whatever a core
  returned, charging the per-evaluation budget (`Budget::evaluation()`:
  a million work units, 16 MiB of allocation and result, the page's
  per-evaluation defaults; the request and iteration ceilings stay
  unbounded until slices 4 and 12). The registry already depends on
  `tcl-cmd-core`, so every shipped evaluator is a core call from there.
- **Admission checks per operation, not per axis.** The page's `admit`
  step 2 reads "a bit whose field is `None` is a decline". Applied
  literally that declines every `incr` under a profile naming no release
  — every vendor dialect, iRules included — because its numeral field is
  `None`, and it would lose the O100 chain the exit criteria name on every
  iRule. The page's own per-axis rules are per operation
  (`NumberSyntax::unanimous`, `StringCharacterModel::count_for(None, …)`),
  so `admit` declines at admission only the two axes no operation can ever
  answer (`PLATFORM`, `WALL_CLOCK`), and each other axis answers with the
  value every modelled release gives — `incr x 5` folds under iRules; `incr
  x` over `010` declines with `ReleaseAmbiguous(NumeralGrammar)`; an
  overflow declines with `ReleaseAmbiguous(IntTower)`. A named release
  answers under that release. The KCS note
  `kcs-qa-why-does-a-constant-fold-depend-on-the-dialect.md` says this to
  a contributor.
- **`SOURCE_ENCODING` subsumes `CHAR_INDEXING` for the string routes.** A
  non-ASCII operand is admissible only where the target decodes source as
  UTF-8 (9.0 onward); under 8.x or no release it declines with
  `ReleaseAmbiguous(SourceEncoding)`. Every ASCII string counts and indexes
  identically under both character models, so the addressing-unit axis
  never bites separately, and `string::range`'s scalar vector is the right
  unit wherever the route answers.
- **`fold_range` is the route.** The subcommand's shipped folder became a
  `const_fold_versioned` that runs the declared evaluator over literal
  words (`value_transfer::evaluate_literal`), so the codegen and O129 paths
  and the lattice share one implementation; with no version it answers
  under unanimity, which is the ASCII-only, index-unanimous behaviour the
  old folder had, and under a version it answers that release's.
- **An error is a decline before slice 10.** A core's `CmdError` or a seam
  `ValueError` — `bad index`, `unmatched open brace in list`, the 8.4
  overflow — maps to `WrongRepresentation` unless the run recorded a fault
  first; the completion slice turns it into `CompletionOutcome::Error`.
- **The lift is the driver's, stated once in the registry**
  (`value_transfer::lift`): `finite_inputs` collects every operand's and
  every plan target's finite view, deduplicated by SSA identity; one pins
  each member in turn through `PinnedInputs` and joins the outcomes into a
  `ConstSet`; two decline with `CorrelatedSets`; more members than
  `MAX_CONSTSET_SIZE` decline with `TooManyMembers`. An evaluator that still
  sees a finite view (a detached caller) declines as correlated.
- **`append` / `lappend` calls reach the lattice as calls.** The driver's
  `evaluate_call` resolves a call with no loop-header marker and, when its
  plan is a cell read-modify-write, applies the registry's evaluated store
  to the target's def; a call with no plan keeps `Overdefined`. The typed
  `Incr` node and the `Call` shape now run the same function.
- **The consumers read the semantics, not a name.** `chain_fold`
  classifies a write by the resolved cell update and reads a `$var` piece
  from the function's lattice at that statement, so the non-consecutive
  chain folds through the lattice value; `fold_tail_statement_under_lattice`
  returns a trailing cell update's written value through the plan's
  target; `static_loops` runs the typed `Incr` through the registry's
  route over its environment (`exec_cell_update_in_env`) under a
  `LoopSemantics { policy, registry }` bundle, and its literal ingress is
  the one round-trip rule; `intervals` interprets the descriptor's
  `RangeModel::IntegerAdd` over the cell and the amount, a `$var` amount
  included — from the descriptor the typed node encodes, so no registry
  is threaded through five public callers for a fact lowering already
  established.
- **A nested cell update's read is an SSA use by construction (#2050).**
  The CFG builder's embedded-substitution scan now carries the names a
  `READS_BEFORE_WRITE` command reads first (`VariableWriteEffects::
  read_before_written`) as `reads` on the host call or on the synthetic
  `<upvar-invalidate>` call, so `set n 1` ahead of `set result [incr n]`
  has a use and O109, O126, and W220 keep it. The `Statement::Incr`
  deletion predicate is unchanged.
- **W231's length is the value after the cell updates (#2054).** The
  lexical walk carries the last literal assignment's *value* and applies
  every later cell update on the variable whose route evaluates over
  literal words; a write it cannot evaluate forgets the value rather than
  reporting against a stale one.
- **The route explanations ride `SccpResult`.** The driver records the
  last pass's route and answer per statement span
  (`RouteExplanation`); the Explorer's `sccp` payload gains `routes`, the
  tree view renders them, and `tcl explore --show sccp --text` prints
  them.
- **`intervals`' model comes from the descriptor, not a registry.** The
  typed `Incr` node is the registry's `CellReadModifyWrite(Increment)` by
  construction, so `cell_update_range_model` builds the derived
  specialisation and asks it for the range model; threading a registry
  through the five public callers of `compute_intervals_with` would have
  re-derived what lowering established.

### Issues this slice's witnesses close

- #2050 — `set n 1; set result [incr n]; puts $n`: O109 keeps `set n 1`
  (`store_read_by_a_nested_cell_update_is_not_dead`) — **not yet true at
  the hand-off**: the test fails; see "Status at hand-off".
- #2052 — `set p { again}; append s $p`: the chain fold reads ` again`
  exactly (`var_piece_proven_by_the_lattice_folds_the_chain`,
  `evaluate_def_append_var_piece_reads_the_lattice_exactly`).
- #2054 — `lappend xs a b c; lset xs 2 X`: W231 reads three elements
  (`w231_length_follows_the_cell_updates`).

### Behavioural deltas accepted

- `incr x " 5"` folds again (6 in every release): the step is read under
  the numeral grammar, which admits whitespace, where slice 1's
  round-trip classification declined it.
- `incr` over a leading-zero base folds under a named release (9 up to
  8.6, 11 from 9.0) and declines under an unnamed one; the same for an
  overflow (a bignum from 8.5, declined under 8.4 and an unnamed release).
- `string range abcdefghijkl 010 end` folds under a named release (`ijkl`
  up to 8.6, `kl` from 9.0); a non-ASCII subject folds under 9.x.
- `append` / `lappend` statements and `[append …]`-free chains reach the
  lattice, so consumers that read it see more constants; a `$var` piece a
  chain fold can prove folds the chain.
- The simulator's literal ingress no longer trims and keeps `010` / `+5`
  as text, so a bounded loop over such a counter simulates only under a
  named release.

## Status at hand-off (2026-09-18): slice 2 checkpoint, not complete

Implementation moved to other agents at the owner's request. This section
is what a fresh agent needs to resume from cold. The slice-2 sections
above describe the design as intended; where they claim a witness, this
section says whether it holds. The checkpoint commit is
`wip(value-transfers): slice 2 checkpoint — hand-off`, on top of
`5bc40e95` (the diagnostic-policy lane's own checkpoint), and the tree it
leaves is the whole lane state — nothing is parked outside git. The
crates the lane touched are `tcl-registry`, `tcl-compiler`,
`tcl-explorer`, `tcl-lsp-db` and `xtask`; the other lane owns
`tcl-lsp-core`, `tcl-lsp-server` and `tcl-mcp`.

**VT2.0 (2026-09-22), in progress:** the seven failing tests, the gate's
six waivers and pedantic clippy over the five crates are fixed in the
first `wip(value-transfers): slice 2 checkpoint green` commit; the unrun
gates and the full suites follow in the next, which rewrites this section
with the results.

### What compiles and what was run

- `cargo check --workspace --all-targets`: green.
- `cargo fmt -p tcl-registry -p tcl-compiler -p tcl-explorer -p tcl-lsp-db
  -p xtask`: applied at the hand-off (it changed no code; it moved six
  waiver comments, see below).
- `cargo xtask pack-goldens`: 0 snapshots rewritten, 24 packs scanned. The
  `string range` declaration changes no shipped-pack snapshot; `56320895`
  already carries the `semantics` column.
- `bash scripts/dev/test-nextest-binary-shards.sh`: ok. The slice adds no
  test binary, so the shard manifest needs no row.
- `cargo xtask value-transfers` (write mode): **exit 1** — "6 site(s)
  recognise a command by name in a file the gate holds clean" (the sites
  are listed under "Where each unfinished piece stops").
  `docs/generated/value-transfers.md` is what that run wrote before it
  failed and is committed as written; regenerate it once the gate passes.
- `cargo test -p tcl-compiler -p tcl-explorer -p tcl-registry` (run just
  before the final format): every test binary of the three crates passes
  except these five, which fail exactly as described below:
  - `tcl-compiler` lib: 6427 passed, 1 failed —
    `optimiser::elimination::tests::store_read_by_a_nested_cell_update_is_not_dead`.
  - `tcl-compiler` `tests/compiler_analysis_residual.rs`: 73 passed, 2
    failed — `rebase_switch_and_while_shift`,
    `rebase_shifted_unit_spans_match_fresh`.
  - `tcl-registry` lib: 894 passed, 1 failed —
    `commands::tcl::string_::tests::string_index_comparison_folds_match_tcl`.
  - `tcl-registry` `tests/differential_fold.rs`: 3 passed, 1 failed —
    `storage_outcome_witnesses_match_every_release_on_path`.
  - `tcl-registry` `tests/value_transfers.rs`: 11 passed, 2 failed —
    `the_increment_route_runs_the_shared_core_under_the_target_semantics`,
    `append_and_list_append_run_the_shared_cores`.
  The witnesses for #2052 and #2054 pass
  (`optimiser::chain_fold::tests::var_piece_proven_by_the_lattice_folds_the_chain`,
  `sccp::tests::evaluate_def_append_var_piece_reads_the_lattice_exactly`,
  `analyser::bounds_checks::tests::w231_length_follows_the_cell_updates`);
  the witness for #2050 is the failing elimination test.
- Not run at all: `cargo test -p tcl-lsp-db` (its new test
  `direct_and_memoised_lattices_agree_on_the_cell_update_witnesses`
  type-checks under the workspace check and has never executed),
  `cargo test -p xtask`, clippy over any slice-2 file, and
  `tcl explore --show sccp --text` on a cell-update fixture.

### What landed (type-checked; the tests above qualify it)

- `rust/tcl-registry/src/value_transfer/const_ops.rs` (new): `Needs`
  (the admissibility bitset), `Representation`, `ConstValue`,
  `TargetSemantics::of(profile)` (via `TclVersion::from_profile`; a
  profile naming no release gets per-operation unanimity through
  `NumberSyntax::unanimous` and `StringCharacterModel::count_for(None, …)`),
  and `ConstOps` — `admit(ctx, budget, needs)`, `index(spec, len)`,
  `admissible_text(value)`, `take(value)`, plus the `ValueOps`
  implementation the runtime adapters' cores run over. Byte-exact,
  poisoned by the first fault, charging `Budget`.
- `rust/tcl-registry/src/value_transfer/lift.rs` (new): `PinnedInputs`,
  `finite_inputs`, `LiftedAnswer`, `evaluate_lifted(semantics, inputs,
  budget, cap)` — one distinct finite SSA identity evaluates per member,
  two decline `CorrelatedSets`, over the cap declines `TooManyMembers`.
- `rust/tcl-registry/src/value_transfer/literal.rs` (new):
  `LiteralInputs::new(command, subcommand, args, profile)`,
  `.with_prior(name, ExactValue)`, `evaluate_literal(semantics, command,
  sub, args, version) -> Option<String>` — the literal-word ingress the
  shipped folders and the lexical consumers (W231) use.
- `cell_update.rs` (rewritten, REVISION 2): `incr` / `append` / `lappend`
  over `ConstOps` through `ValueOps::int_add`,
  `tcl_cmd_core::var::append_bytes`, `tcl_cmd_core::var::lappend_value`;
  `needs()`, `evaluator()`; `exact_input` maps a `Pending` prior to
  `EvalAnswer::Pending`, a finite one to `CorrelatedSets`.
- `route.rs`: `NativeEvalId::{CellAppend, CellListAppend, StringRange}`
  registry-owned. `ListOfArgs`, `ListLength`, `StringLength`,
  `FormatTemplate` are still `EvaluatorOwner::Transitional
  { retires_in_slice: 2 }` — nothing was started on retiring them.
- `builtins.rs`: `StringRangeSemantics` / `STRING_RANGE` (NEEDS =
  `INDEX_GRAMMAR | CHAR_INDEXING | SOURCE_ENCODING`; `evaluate_range`
  resolves both indices through `ops.index` and runs
  `tcl_cmd_core::string::range`). `commands/tcl/string_.rs`: `range`
  declares `semantics: Declared(&STRING_RANGE)` and `const_fold_versioned:
  Some(fold_range)` (`fold_range` at the top of the file calls
  `evaluate_literal`); its unversioned `const_fold` is gone.
- `context.rs`: `Budget::evaluation()`, `charge_work`,
  `charge_allocation`, `charge_result`. `decline.rs`: `Axis::as_str()`.
  `mod.rs`: the three new modules and their re-exports.
- `rust/tcl-compiler/src/value_transfer.rs`: `RouteExplanation`
  recording on the `LatticeDriver` (`explaining(span)`),
  `cell_update_def`, lifted evaluation through `evaluate_lifted`,
  `evaluate_call` applying a cell-update call's store to its target,
  `exec_cell_update_in_env` / `EnvInputs` for the loop simulator,
  `resolved_cell_update`, `cell_update_range_model`;
  `fold_cmd_subst_routes` (the value-position folder) as before.
- `sccp.rs`: `SccpResult.explanations`, `evaluate_def_dispatch`,
  `BranchFold<'a> { policy, grammar, registry }`,
  `TerminatorInputs.registry`, `collect_constant_branches(…, fold)`;
  tests with the `cell_update_call` fixture and `folds_for(dialect)`.
  `static_loops.rs`: `LoopSemantics<'a> { policy, registry }` through
  `exec_*` / `summarise_*`; `parse_literal_value` is
  `ExactValue::from_literal`. `intervals.rs`: the `Incr` arm reads
  `cell_update_range_model`; `compute_intervals_with` keeps its four
  arguments. `optimiser/chain_fold.rs`, `optimiser/propagation.rs`,
  `cfg_builder/mod.rs`, `ir_helpers.rs`, `analyser/bounds_checks.rs`,
  `analyser/diagnostics/usage.rs` read the resolved semantics as the
  slice-2 decisions above describe; `optimiser/elimination.rs`,
  `taint.rs`, `type_infer.rs`, `optimiser/branch_folding.rs` changed in
  tests or literals only.
- `rust/tcl-explorer/src/serialise.rs`: `serialise_sccp` emits `routes`
  (tested at the JSON level); `view_tree.rs` renders a node per route.
- `rust/tcl-lsp-db/src/lib.rs`: the parity test named above.
- `rust/xtask/src/value_transfers.rs`: `CLEAN_FILES` gained
  `analyser/bounds_checks.rs`, `analyser/diagnostics/usage.rs`,
  `cfg_builder/mod.rs`, `intervals.rs`, `ir_helpers.rs`,
  `optimiser/chain_fold.rs`, `static_loops.rs`; their `RATCHET` rows are
  gone. The migration doc's ledger and ratchet rows moved with them.
- Docs: the slice-2 sections above; the KCS pages
  `docs/kcs/compiler/kcs-qa-why-does-a-constant-fold-depend-on-the-dialect.md`
  (new) and `kcs-qa-what-does-a-value-transfer-declaration-say.md`, and
  the two KCS indexes.

### Where each unfinished piece stops

1. **The gate.** The six sites the gate reports —
   `analyser/bounds_checks.rs`: `if cmd_name == "lindex" {`, `let verb =
   if cmd_name == "lrange" {`, `if sub == "index" || sub == "insert" {`,
   `let verb = if sub == "range" {`; `analyser/diagnostics/usage.rs`:
   `let opt_start = if cmd_name == "fconfigure" {`, `} else if cmd_name ==
   "chan" && args.first()… == Some("configure") {` — carried their
   `// value-transfer-ok: <axis> — <reason>` waivers as trailing comments
   on those lines, which `site_waiver` (xtask `value_transfers.rs`) did
   not recognise; the hand-off format then moved each comment onto the
   line *below* its site (the first line inside the `if`), which the gate
   reads even less. The neighbouring sites in the same files
   (`bounds_checks.rs` waivers at the comment lines above their `if`s)
   are recognised, so the fix is to move each of the six comments to its
   own line directly above the site. The axis and reason texts are
   already right: `arg_roles — the W230–W232 index positions await an
   index-argument role on the registry` and `options — W311 reads the
   encoding option's position, which `option_placement` on the registry
   will carry`.
2. **Rebase of the route explanations.**
   `lattice_rebase::rebase_function_unit` shifts
   `fu.sccp.constant_branches[*].span` but not
   `fu.sccp.explanations[*].span` (`RouteExplanation.span` is a plain
   `Span`), so a cache-hit rebased unit compares unequal to a fresh build
   in `compiler_analysis_residual.rs` (`assert_span_carrying_eq`). One
   loop beside the `constant_branches` one fixes both tests.
3. **#2050, the nested cell update's read.** `cfg_builder/mod.rs`'s
   embedded-substitution scan puts the names a `READS_BEFORE_WRITE`
   command reads first (`VariableWriteEffects::read_before_written`) on
   the host call's `reads`, or on the synthetic `<upvar-invalidate>`
   `Statement::Call` it prepends before a non-Call host (the `set result
   [incr n]` fixture is that case). O109 still fires on `set n 1`
   (`elimination.rs`, the test at the end of the `tests` module): the
   `reads` on the synthetic call do not become a use in the def-use chain
   `emit_dead_stores_and_unused` / `dead_chain_code` consult. Not yet
   established whether the SSA builder ignores `reads` on that synthetic
   command, or whether the fixture never reaches the synthetic-call path.
   Trace where `Statement::Call.reads` becomes an SSA use before changing
   anything else.
4. **`string_index_comparison_folds_match_tcl`** (registry lib,
   `string_.rs`). The test's helper unwraps `subcommand("range")
   .const_fold`, which is now `None`. The standing rule was to keep every
   existing test byte-identical unless a witness proves today's result
   wrong, and no witness does here, so restore `const_fold: Some(|args|
   fold_range(args, None))` on `range` — the unversioned form is the
   unanimous answer — unless the `const_fold_versioned` consumers
   (`tcl-compiler/src/const_subst.rs`, `codegen/values.rs`) forbid both
   fields at once; check before choosing.
5. **`storage_outcome_witnesses_match_every_release_on_path`**
   (`differential_fold.rs`). The case `("lappend", "a", &["{", "b"])`:
   the route answers `a \{ b`, which is what every `tclsh` gives for
   `set v a; lappend v \{ b`; the test's oracle brace-quotes each value,
   so its script contains `{{}` — unbalanced — and `tclsh` reading it
   ends with no output and exit 0, which the oracle reports as
   `Some("")`. Fix the oracle's quoting (backslash-escape `{`, `}`, `\`,
   `"`, `$`, `[` inside double quotes, or build the value with
   `[format %c 123]`), not the route.
6. **`the_increment_route_runs_the_shared_core_under_the_target_semantics`**
   (`value_transfers.rs`). The route now answers with
   `RepresentationEvidence::Constructed(TclType::Int)` — `ConstOps`
   records the representation it built — while `ExactValue::int(9)` in
   the expectations carries `Unknown`. The evidence is intended (slice 1
   added it for exactly this), so move the test: compare `bytes` and
   `numeric`, or build the expectations with the constructed evidence.
   Every `Ok(ExactValue::int(…))` expectation in that test is affected.
7. **`append_and_list_append_run_the_shared_cores`**
   (`value_transfers.rs`). The `assert!(matches!(evaluate("lappend",
   FactView::Pending, &["v"]), Err(_) | Ok(_)))` block routes a pending
   prior through the `evaluated` helper, which panics on
   `EvalAnswer::Pending`; `Pending` is the right answer (the route's
   `exact_input`, and the direct assertion the test makes right after).
   Delete that block.
8. **Escapes on the value-position route.** `fold_cmd_subst_routes`
   (`tcl-compiler/src/value_transfer.rs`) hands the raw segment texts to
   the `string range` route, while the const-fold engine
   (`const_subst.rs::literal_words_at_depth`) cooks `Esc` tokens with
   `tcl_lexer::backslash_subst_in(text, config.escapes)` and braced
   `Str` tokens with `WordValueRules::from_config(&config)
   .collapse_braced_word(text)`. IR texts keep escapes raw (`a\tb` is
   four characters), so `[string range "a\tb" 0 1]` reached from value
   position is computed over the raw text. No test pins it. Mirror the
   cooking in `fold_cmd_subst_routes` and add the witness (`tclsh` gives
   `a` followed by a tab).
9. **The transitional handlers.** `ListOfArgs`, `ListLength`,
   `StringLength` say `retires_in_slice: 2` in `route.rs` and in the
   migration doc's ledger; `FormatTemplate` was to move to slice 3 (its
   `format` core is not a `ConstOps` matter yet) but the ledger row and
   the `retires_in_slice` value were not changed. Either retire the three
   list/length handlers onto registry-owned routes over `ConstOps` (the
   `string range` route is the pattern) or re-ledger all four to slice 3
   with the doc rows; the `value_transfers.rs` pinned-route test and
   `docs/generated/value-transfers.md` follow.
10. **Unverified.** The `tcl-lsp-db` parity test; `tcl explore --show sccp
    --text` printing the routes (the tree view has the nodes; nothing
    checks the text); clippy over the slice-2 files.

### Remaining steps, in order

1. Move the six waivers above their sites; `cargo xtask value-transfers`
   in write mode regenerates `docs/generated/value-transfers.md`.
2. Shift `explanations` spans in `rebase_function_unit`; run
   `cargo test -p tcl-compiler --test compiler_analysis_residual`.
3. Fix the two `value_transfers.rs` tests (items 6 and 7) and the
   `differential_fold.rs` oracle quoting (item 5); run
   `cargo test -p tcl-registry`.
4. Restore `range`'s `const_fold` shim (item 4) or move that test; run
   the registry lib tests.
5. #2050 (item 3): trace `Statement::Call.reads` into SSA; make
   `store_read_by_a_nested_cell_update_is_not_dead` pass without changing
   the `Statement::Incr` deletion predicate.
6. `cargo test -p tcl-lsp-db` for the parity test; fix what it finds.
7. Cook escapes in `fold_cmd_subst_routes` (item 8) with its witness.
8. Retire or re-ledger the transitional handlers (item 9).
9. Add a `tcl-explorer` test that `--show sccp --text` prints a route
   line for the `cell_update_call` fixture.
10. Clippy over the five crates (`cargo clippy -p <crate> --no-deps
    --all-targets`), the full `cargo test` of the five crates, then the
    standing gates before the commit: `cargo xtask value-transfers`,
    `cargo xtask pack-goldens` (stage the 24 snapshots if any change),
    `bash scripts/dev/test-nextest-binary-shards.sh` (a row per new test
    binary). Squash onto the checkpoint as
    `wip(value-transfers): slice 2 — the direct vertical slice`; the
    checkpoint commit's body is the draft of that message.

### Decisions this checkpoint takes that the design pages do not state

The slice-2 "Decisions taken, and why" list above stands (per-operation
admission under an unnamed release, `SOURCE_ENCODING` subsuming
`CHAR_INDEXING`, errors as `WrongRepresentation` before slice 10, the
lift's one-finite-identity rule, the interval model from the
descriptor). In addition:

- A route's `ExactValue` carries the representation `ConstOps` built
  (`Constructed(Int)` for an increment); tests that compare whole
  `ExactValue`s must state it. Recorded here because the test that
  fails on it predates the evidence.
- `EvalAnswer::Pending` is an answer a test helper must pass through,
  never a failure.
- `string range` is folded by its route through `evaluate_literal` in
  both the versioned and the unversioned folder; the unversioned answer
  is the unanimous one.
- `docs/generated/value-transfers.md` is committed as the failing
  write-mode run produced it, so the ratchet section already lists the
  slice's clean files; it is regenerated in step 1.
- The lane keeps `FormatTemplate` transitional; slice 3 is its intended
  retirement slice (not yet ledgered).

### Environment a fresh agent needs

- The reference interpreters are `/root/.local/bin/tclsh8.4`, `tclsh8.5`,
  `tclsh8.6`, `tclsh9.0`, `tclsh9.1` (`differential_fold.rs`'s
  `find_tclsh` looks them up on `PATH`); the witness results the slice-2
  sections quote came from them.
- `cargo check --workspace --all-targets` takes about 1.5 min warm; the
  `tcl-compiler` lib tests about 55 s; the full three-crate test run
  about 5 min. Check `df -h /` before a heavy build (8.5 GB free at the
  hand-off; the lane stopped below 5 GB).
- `scripts/dev/test-nextest-binary-shards.sh` and `cargo xtask
  pack-goldens` are the coordinator's pre-commit gates for every lane;
  `cargo xtask value-transfers` is this lane's.
