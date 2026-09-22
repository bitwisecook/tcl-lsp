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
  `<upvar-invalidate>` call, and the SSA builder reads a `reads` name its
  call also defines as a read of the prior value (`uses_in_call`):
  lowering flags that overlap `reads_own_defs`, the CFG builder's calls
  leave it unflagged because their other definitions are not read, and
  the use filter dropped the name. So `set n 1` ahead of `set result
  [incr n]` has a use and O109, O126, and W220 keep it. The
  `Statement::Incr` deletion predicate is unchanged.
- **Read-before-set does not claim an embedded cell update's read.** The
  substitution scan recovers every `[…]` in a statement's words, braced
  ones included, so the read may not run where it is recorded: the
  top-level `proc f {} { puts [incr n] }` statement records a read of
  `n`. W210 (`embedded_cell_update_read` in `dataflow.rs`) skips a named
  read of a cell the same call defines on a call not flagged
  `reads_own_defs`, as it skips a quoted mention; the use keeps the
  `Name` class, so value forwarding never targets it. W210 reports what
  it reported before: an embedded `lset`, `lpop` or `ledit`, or an 8.4
  `incr`, over an unset variable stays unreported.
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
  (`store_read_by_a_nested_cell_update_is_not_dead`), and keeps `set s x`
  ahead of `puts [append s y]; puts $s`, where the read rides the host
  call.
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

## Status (2026-09-22): slice 2 checkpoint green, not complete

Implementation moved to other agents at the owner's request. This section
is what a fresh agent needs to resume from cold. The slice-2 sections
above describe the design as intended; where they claim a witness, this
section says whether it holds. The hand-off checkpoint is `f3f9390f`
(`wip(value-transfers): slice 2 checkpoint — hand-off`, 2026-09-18, on
top of `5bc40e95`); it left seven tests and the lane's gate failing and
four checks never run. VT2.0 (2026-09-22) made it green without
starting the slice's remaining steps, in `206f6eee` (the tests, the gate
and clippy) and the commit that carries this section (the unrun checks
and the full suites). The tree is the whole lane state — nothing is
parked outside git. The crates the lane touched are `tcl-registry`,
`tcl-compiler`, `tcl-explorer`, `tcl-lsp-db` and `xtask` (and, for one
port-table entry, `tcl-spec-studio`); the other lane owns
`tcl-lsp-core`, `tcl-lsp-server` and `tcl-mcp`.

### What compiles and what was run (VT2.0)

- `cargo check --workspace --all-targets`: green.
- `cargo clippy -p tcl-registry -p tcl-compiler -p tcl-explorer
  -p tcl-lsp-db -p xtask --all-targets -- -D warnings`: clean, with no
  `#[allow]` added. Its first run found pedantic lints in slice-2 code,
  each fixed at its cause: single-arm `match`es in `const_ops.rs` and
  `context.rs`; `cell_update_assignment`'s `Option<Option<String>>`
  (now `Option<CellUpdateWrite>`); a `?`-shaped block in
  `chain_fold.rs`; `fold_cmd_subst_routes` over 100 lines (the two
  identical explanations became `explain_fold`); and two registry tests
  over 100 lines (helpers, and the increment test split in two).
- `cargo fmt --all -- --check`: clean.
- `cargo xtask value-transfers --check`: OK — 15 files clean, 16 sites
  waived, 100 sites pinned across 41 ratcheted files, 6607 inventory
  rows; `docs/generated/value-transfers.md` is the write-mode output.
- `cargo xtask pack-goldens`: 0 snapshots rewritten, 24 packs scanned.
- `bash scripts/dev/test-nextest-binary-shards.sh`: ok; VT2.0 adds no
  test binary.
- `cargo xtask kcs-index-links`: passed; `cargo xtask owner-resolution`:
  OK, 43 owner rows.
- `cargo test -p tcl-compiler -p tcl-registry -p tcl-explorer
  -p tcl-lsp-db -p xtask --no-fail-fast`: every test binary passes.
  `tcl-compiler`: lib 6428 passed (2 ignored), 65 integration binaries
  3172 passed (4 ignored; `compiler_analysis_residual` 75), 7 doc-tests;
  `tcl-registry`: lib 895, 19 integration binaries 275
  (`differential_fold` 4, `value_transfers` 14), 1 doc-test;
  `tcl-explorer`: lib 99; `tcl-lsp-db`: lib 92 (the parity test
  included), 9 integration binaries 26 (5 ignored); `xtask`: 223. In
  all 11218 passed, 0 failed, 11 ignored.
- `cargo test -p tcl-spec-studio`: 282 passed, once `spectcl_ports`
  documents `string range`'s new unrenderable fields (item 4).
- `tcl explore --show sccp --text` (`target/debug/tcl` from
  `cargo build -p tcl-cli`) over `set n 1; set result [incr n]; append s
  x; append s y; lappend xs a b; set r [string range abcdef 1 3]; set q
  [string range abc 010 end]` prints a `route <command>: <route>
  (<owner>)` node per resolved statement with its `answer` and `line`:
  `direct cell-increment (registry)` answering `pending` (item 11),
  `direct cell-append` and `direct cell-list-append` answering
  `declined: not-exact` over the unset cells, and `direct string-range`
  answering `evaluated` (`r#1 = const('bcd')`, `q#1 = const('')`).

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

Items 1–7 and 10 are done; each says what the cause was and what fixed
it. Items 8 and 9 stand as the hand-off wrote them; item 11 is new.

1. **The gate — done.** The six waivers were comments on the line below
   each site; each is now on its own line directly above it (the
   `chan configure` one inside the first arm, directly above its
   `} else if`), the form `site_waiver` reads.
2. **Rebase of the route explanations — done.** `rebase_function_unit`
   shifts `explanations[*].span` beside the constant-branch spans.
3. **#2050, the nested cell update's read — done.** The fixture does
   take the synthetic-call path. The SSA builder's use filter
   (`uses_of_classified`) keeps a name the statement also defines only as
   a read-before-write, and the synthetic `<upvar-invalidate>` call is
   not flagged `reads_own_defs`, so its read of `n` was dropped. The rule
   that fixes it is in the slice-2 decisions above, with the
   read-before-set rule that keeps W210 where it was.
4. **`string_index_comparison_folds_match_tcl` — done.** `range` carries
   `const_fold: Some(fold_range_unanimous)` beside
   `const_fold_versioned: Some(fold_range)`, both the route through
   `evaluate_literal`. The consumers allow both: `run_const_fold` tries
   the versioned folder first, so the unanimous one answers only a caller
   that reads `const_fold` itself. The subcommand's new `semantics` and
   `const_fold_versioned` also failed `tcl-spec-studio`'s `spectcl_ports`
   (never run on the checkpoint): its port table now documents them as
   unrenderable on `range`, as slice 1 did for `foreach`.
5. **`storage_outcome_witnesses_match_every_release_on_path` — done.**
   The oracle writes each value as a double-quoted word with `\`, `"`,
   `$`, `[`, `]`, `{` and `}` escaped (`tcl_quoted_word`). Seen and left
   alone: `tclsh` reading a script on stdin exits 0 after printing an
   error, so `run_tcl` never reports a raise and the oracle reads one as
   empty output; no case in the matrix has a route that could answer the
   empty string.
6. **The increment's representation evidence — done.** The expectations
   are `built_int(i)` (`Constructed(TclType::Int)`); the release rules
   moved into
   `the_increment_route_reads_numerals_under_the_target_release` to keep
   each test under the line limit.
7. **The pending `lappend` — done.** The block is gone; the direct
   `EvalAnswer::Pending` assertion stays.
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
10. **Verification — done.** The `tcl-lsp-db` parity test's first run
    failed on its own loop: it asserted that `n` folds to 4 in both `p`
    and `q`, and `q` has no `n`. The assertion now covers `p` alone; the
    parity assertion covers both. Clippy and the explorer text: above.
11. **Found by VT2.0, not fixed: a value-position cell update launders a
    constant through a join.** In `proc f {cond} { set n 1; if {$cond} {
    set x [incr n] } else { set x 5 }; puts $x }`, `tcl opt` rewrites
    `puts $x` to `puts 5` (O100), and `f 1` prints 2 on `tclsh8.6`. The
    `[incr n]` route runs in `fold_cmd_subst_routes` with the host
    `AssignValue`'s uses, which do not hold `n` (its read is on the
    synthetic call before the host), so `LatticeInputs::prior_store`
    reads version 0 (`unwrap_or(0)`), finds no value and answers
    `Pending`; `LiftedAnswer::Pending` becomes `LatticeValue::Unknown`,
    which survives the fixed point, and the phi takes the other arm's
    `Const(5)`. The explorer shows it as `answer: pending`. The smallest
    fix is the rule `place` already states for a dynamic key: a missing
    use is a permanent miss, so `prior_store` declines rather than
    reading version 0, and `set x [incr n]` keeps slice 1's
    `Overdefined`; this program is its witness. Not bisected: the
    fallback and the pending mapping are in slice 1's code as well.

### Remaining steps, in order

1. Item 11: `prior_store` declines on a use the statement does not hold,
   with the join witness.
2. Cook escapes in `fold_cmd_subst_routes` (item 8) with its witness.
3. Retire or re-ledger the transitional handlers (item 9).
4. Add a `tcl-explorer` test that `--show sccp --text` prints a route
   line for the `cell_update_call` fixture.
5. Clippy over the five crates, the full `cargo test` of the five
   crates, then the standing gates before the commit: `cargo xtask
   value-transfers`, `cargo xtask pack-goldens` (stage the 24 snapshots
   if any change), `bash scripts/dev/test-nextest-binary-shards.sh` (a
   row per new test binary). Squash onto the checkpoint as
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
- `docs/generated/value-transfers.md` is the gate's write-mode output;
  VT2.0 regenerated it once the gate passed.
- VT2.0's two rules — the SSA reads a `reads` name its call also defines
  as a read of the prior value, and read-before-set does not claim an
  embedded cell update's read — are in the slice-2 decisions above.
- The lane keeps `FormatTemplate` transitional; slice 3 is its intended
  retirement slice (not yet ledgered).

### Environment a fresh agent needs

- The reference interpreters are `/root/.local/bin/tclsh8.4`, `tclsh8.5`,
  `tclsh8.6`, `tclsh9.0`, `tclsh9.1` (`differential_fold.rs`'s
  `find_tclsh` looks them up on `PATH`); the witness results the slice-2
  sections quote came from them.
- `cargo check --workspace --all-targets` takes about 1.5 min warm; the
  `tcl-compiler` lib tests about 55 s; the five-crate test run about
  6 min. Check `df -h /` before a heavy build: each distinct `-p` set
  builds its own variant of every test binary (feature unification
  differs), about 2 GB for `tcl-compiler`'s alone. VT2.0 deleted the lane
  crates' stale test executables to stay above 4 GB.
- The worktree and the target directory are shared with another
  implementer working in `tcl-lsp-core`, `tcl-lsp-server`, `tcl-cli` and
  `tcl-mcp`: stage by explicit path, and never `git stash` or
  `git checkout .`.
- `scripts/dev/test-nextest-binary-shards.sh` and `cargo xtask
  pack-goldens` are the coordinator's pre-commit gates for every lane;
  `cargo xtask value-transfers` is this lane's.
