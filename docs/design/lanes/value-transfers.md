# Lane: value-transfers — the slices of the migration plan

The crash-insurance and handover note for the `value-transfers` lane. A
fresh agent resumes from this file and the `wip(value-transfers):` commits.
Slices 1 to 4 have landed, slice 4 with its review's fixes (§ *Slice 4* ›
*Record (2026-09-23): the review of slice 4*); § *Plan for slices 2–13* is
the plan for the rest. Slice 5 is in progress: § *Slice 5* › *Record
(2026-09-23): the opus items of slice 5* has each checkpoint so far and is
where to start.

## Goal

The lane began as slice 1 of [value-transfers-migration.md](../compiler/value-transfers-migration.md)
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
  `::tcl::dict::` spellings — slice 2 closed `set` and the `dict` rows, and
  VT2.8 moved `const` to slice 8 and `lset`, `ledit`, `lpop` to slice 7);
  slice 5 (`regexp`, `regsub`, `scan`, `binary
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

Done in slice 2 (landed 2026-09-23):

- registry: `const_ops.rs` (`ConstOps`, `Needs`, `TargetSemantics`),
  `lift.rs`, `literal.rs`, `cell_write.rs` (`set`), `keyed_update.rs` (the
  five `dict` keyed updates, both spellings); `incr`, `append`,
  `lappend`, `string range`, `list`, `llength` and `string length` run
  registry-owned direct routes; `CommandSemantics::incoming_targets`.
- compiler: the driver's lift, explanations, `store_def`, the cooked
  literal words (`literal_token_value`), `call_arguments`; the consumers
  that read the resolved cell update (`chain_fold`, the tail fold,
  `static_loops`, `intervals`, W231); the qualified global write and its
  caller-side observation (#2214); `ModuleAnalysisFacts::command_trust`.
- lsp-db: `ValueTransferContext::mutations`; `value_transfer_parity.rs`.
- explorer: the `sccp` view's route rows, and their text test.
- the witness binary `rust/tcl-compiler/tests/value_transfer_witnesses.rs`
  and its shard row.
- the review fixes: `FactView::exact`, `TargetSemantics::render_list`,
  `Statement::Incr::amount_braced`, the typed assignment's cooking and
  trust, and `READS_BEFORE_WRITE` on the `dict` keyed updates.

Remaining: slices 3 to 13. Slice 2's deferred CLI witness binary (D35)
landed with VT3.10 (D65).

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
  not yet record per-statement explanations in `SccpResult`. Resolved in
  slice 2: `SccpResult::explanations` records each statement's route and
  answer, and the `sccp` view prints them
  (`sccp_text_prints_the_route_of_each_cell_update`).
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

## Status (2026-09-23): slice 2 landed

The hand-off checkpoint `f3f9390f` (2026-09-18) left seven tests and the
lane's gate failing and four checks unrun; VT2.0 made it green in
`206f6eee` and `bdfe3a96` (2026-09-22), and its open items became plan
items — the escapes (VT2.2), the transitional handlers (VT2.7), the
laundered join VT2.0 found (VT2.1), the explorer test (VT2.10) — with the
squash superseded by D1. The coordinator then merged `origin/rust` at
`08bceb36` (VT2.M). Slice 2 landed on 2026-09-23 in `6d0de164`,
`4b8a6f6f` and the landing commit `wip(value-transfers): slice 2 — the
direct vertical slice`; § *Plan for slices 2–13* › *Slice 2* › *Record
(2026-09-23): slice 2 landed* has what each holds, the gates, the deltas
and what was left. The detailed hand-off notes this section used to carry
are in `bdfe3a96`'s tree. The review of the landing returned "land
after fixes"; `wip(value-transfers): review fixes for slice 2` holds them
(§ *Slice 2* › *Record (2026-09-23): review fixes for slice 2*). Slice 3
has since landed; § *Status (2026-09-23): slice 3 landed* below picks up
from here.

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

## Status (2026-09-23): slice 3 landed

The opus items (VT3.1–VT3.5, VT3.7, VT3.8, VT3.12) landed in three
checkpoints the same day (§ *Plan for slices 2–13* › *Slice 3* › *Record
(2026-09-23): the opus items of slice 3*, D47–D61). A later session in
the same day ran the sonnet items and the deferred slice-2 CLI witness
binary one at a time, each its own commit: `636f9e2f` (VT3.6),
`11e7cda7` (VT3.9), `7f0b3d20` (VT3.10), `cb0fe443` (VT2.10's deferred
CLI half, D35/D65), and the landing commit `wip(value-transfers): slice
3 — the expression slice` (VT3.11). The same section has the table with
what each commit landed and its tests; D62–D65 are the decisions the
plan did not state.

Green at the landing:

- tests: `cargo test -p tcl-compiler -p tcl-registry -p tcl-explorer -p
  xtask`: 11226 passed, 0 failed, 6 ignored; `tcl-cli`: 124 passed, 0
  failed, `samples_optimiser_profiles_are_regenerated` included (no
  sample moved);
- pedantic clippy on `tcl-compiler`, `tcl-registry`, `tcl-explorer`,
  `tcl-cli` and `xtask`, with no `#[allow]` added; `cargo fmt` clean on
  the same crates;
- `cargo xtask value-transfers --check` OK: 17 files clean (`word_subst.rs`
  and `shimmer/commit.rs` joined since the opus checkpoint), 13 sites
  waived, 98 pinned across 39 ratcheted files, 6607 inventory rows;
- `bash scripts/dev/test-nextest-binary-shards.sh` OK (the new
  `tcl-cli::value_transfers_cli` row); `cargo xtask kcs-index-links`;
  `cargo xtask owner-resolution` (43 rows); `cargo check --workspace
  --all-targets` clean.

Left to later slices, by name: everything slice 4 onward names in § *Plan
for slices 2–13*; the two found-and-left items the opus record lists
(8.4's double rendering, and a finite input read only inside a nested
command declining as correlated rather than per-member) stand unchanged;
#2118's `o122_sees_a_self_call_inside_a_braced_expr` still waits on
`rust` pull request #2226 reaching this branch.

The review of the landing returned "land after fixes";
`wip(value-transfers): review fixes for slice 3` holds them (§ *Slice 3*
› *Record (2026-09-23): review fixes for slice 3*, D66–D71).

## Status (2026-09-23): slice 4 landed

The opus items (VT4.1, VT4.2, then VT4.3 to VT4.9, D92 folding VT4.6 into
the second checkpoint) landed in three checkpoints, followed by a fourth
closing the host-environment gap (D101) found while running them; the
sonnet items (VT4.10, VT4.11, VT4.12, VT4.14) landed one commit each; and
the opus implementer then landed VT4.13 and the two follow-ups the
coordinator named (Q12, answered as D104; Q13, accepted as built) — one
commit per item, `7fb8efd6` through `c945a41e`. § *Plan for slices 2–13*
› *Slice 4* › the three "Record" subsections has what each commit held,
its tests, and the deltas found beyond the plan's list (D72–D104); §
*Checkpoints and landing* has the full commit list against the plan's
anticipated four checkpoints. VT4.15 (docs and the landing) closed the
slice: every design page D72–D104 touch names now describes the built
mechanism, cited by decision number, rather than the interface pages'
original proposal; the docs commit is `wip(value-transfers): slice 4 —
docs`, and the landing commit is `wip(value-transfers): slice 4 — a
private SpecTcl command` (§ *Checkpoints and landing* has its full text).
No filed issue is pinned or closed by this slice — the findings table
assigns none of #2050–#2144 to slice 4.

Green at the landing (VT4.15 changes no Rust; the smoke run below is the
gate, and every crate suite's own count is unchanged from the last
checkpoint's "Green at …" note above):

- `cargo xtask kcs-index-links`: "KCS docs checks passed" (the new howto
  indexed, every link resolves);
- `cargo xtask owner-resolution`: OK, 44 owner rows (unchanged);
- `UPDATE_REFERENCE=1 cargo test -p tcl-spec-studio --test reference_doc`
  not run — the studio schema has not changed since VT4.11 — and
  `cargo test -p tcl-spec-studio --test reference_doc` (no variable)
  passes unmodified, confirming it;
- `cargo check --workspace --all-targets`: clean;
- `cargo test -p tcl-registry --lib` (the smoke run): 905 passed, 0
  failed, 0 ignored — byte-identical to the count at Q12's checkpoint,
  confirming the docs-only change moved nothing.

Left to later slices, by name: `RegexpPrecision`, `PrecisionDecline`, and
the typed regexp/scan result (slice 5); migrating a shipped builtin
(`incr`, `expr`, `regexp`, …) onto the `semantics` / `evaluate` DSL itself
— slice 4 built the vocabulary and proved it on a private command, but no
shipped command was moved onto it, by mandate ("shipped builtins stay on
the direct route"); the studio's own "try it" box over
`HookHost::install_pack_hooks`, named on the evaluation page but not
scoped into any VT4.x item and not built; `direct_route_needs_match_their_cores`
and the remaining core-function witnesses the evaluation page's own test
anchors still list as "to add".

The review of the landing returned "land after fixes"; its nine findings
landed as one commit, `wip(value-transfers): review fixes for slice 4`
(§ *Plan for slices 2–13* › *Slice 4* › *Record (2026-09-23): the review
of slice 4*, D110 to D114). After it, `-native ID` resolves for the two
`const_fold` families, whose tables hold every shipped folder (47 and 3
ids); the other twelve body families' tables — `ARG_ROLE_RESOLVER`,
`COMMAND_PREFIX_RESOLVER`, `SCRIPT_TIMING_RESOLVER`, `TAINT_SINK_GATE`,
`CONTEXT_GATE`, `LITERAL_ARGUMENT_VALIDATOR`, `CLAUSE_SHAPE_CHECK`,
`OPTION_ARITY`, `CONSTRAINTS`, `SEMANTICS`, `EVALUATE` and `FACTS` — hold
no entries yet: a `-native` id for one of the nine hook families installs
that family's abstention and nothing else, and one on a `semantics`,
`evaluate` or `facts` statement is a notice that nothing this build ships
holds it.

## Status (2026-09-24): slice 5 landed

The opus items (VT5.1 to VT5.12, VT5.15, VT5.16, VT5.18) landed across
fifteen commits, `316ee045` to `3c6714b4`, each its own checkpoint (§
*Plan for slices 2–13* › *Slice 5* › *Record (2026-09-23): the opus items
of slice 5*, D141–D155). A second implementer then ran the five sonnet
items the coordinator held back, one commit each, in the given order:
`674d2910` (VT5.13), `c2b8c741` (VT5.14), `50ce88a6` (VT5.17), `4b676994`
(VT5.19),
and this docs commit, `wip(value-transfers): slice 5 — destructuring and
structured bodies` (VT5.20, the landing) (§ *Slice 5* › *Record
(2026-09-24): the sonnet items of slice 5*, D156). Between VT5.17 and
VT5.19 the coordinator fast-forwarded the branch over the
consumer-contracts lane's `cc-step2` merge (`50ce88a6` → `d9d9868b`, 96
files, green under `make rust-check`): `AnalysisContext::surface_query`
landed in `value_transfer/context.rs`, and that lane's own edit to
`rust/xtask/src/value_transfers.rs` lowered `analyser/commands.rs` and
`analyser/oo.rs` off the ratchet and `lowering/mod.rs` to 1, with the
ledger's row in `value-transfers-migration.md` moving in the same merge —
VT5.19 and VT5.20 ran their gates against that merged baseline, not
VT5.17's. § *Checkpoints and landing* has the plan's anticipated
checkpoint grouping; every item landed as its own commit instead, which
is the more granular, equally valid form the protocol's "commit at each
coherent point" rule allows.

Green at the landing, the full R6 suite plus every standing gate, run
after VT5.20's docs (§ *Slice 5* › *Record (2026-09-24)*'s "Green at
VT5.20" paragraph has the complete counts per crate):

- `cargo check --workspace --all-targets`: clean;
- `cargo test -p tcl-regex -p tcl-cmd-core -p tcl-vm -p tcl-registry -p
  tcl-compiler -p tcl-explorer -p tcl-lsp-core -p tcl-lsp-db -p tcl-cli -p
  xtask`: every crate green (tcl-compiler alone: 9757 passed across 68
  binaries) except one pre-existing, unrelated failure — `tcl-vm`'s
  `builtins_e2e::ensemble_subcommand_words_resolve_like_tclsh` expects
  `encoding system` to answer `utf-8` and gets `iso8859-1` because this
  container's locale is `POSIX`, not because of anything this lane or
  slice 5 touches (`tcl-vm` is not a file any VT5.x item names);
- pedantic clippy and `cargo fmt -- --check` clean on every crate the
  slice's sonnet items touched (`tcl-registry`, `tcl-lsp-core`,
  `tcl-compiler`, `tcl-cli`, `tcl-lsp-db`, `xtask`), no `#[allow]` added
  anywhere in the slice;
- `cargo xtask value-transfers --check`: 20 files clean, 16 sites waived,
  83 pinned across 34 ratcheted files, 6607 inventory rows;
- `cargo xtask registry-axes --check`: 7831 vocabulary words, 6 files
  clean, 5 waived, 956 pinned across 157 ratcheted files;
- `cargo xtask pack-goldens`: 24 packs, 0 rewritten;
- `bash scripts/dev/test-nextest-binary-shards.sh`: ok (no new test
  binary this slice needed a shard row);
- `cargo xtask kcs-index-links`: "KCS docs checks passed";
- `cargo xtask owner-resolution`: OK, 45 owner rows.

Left to later slices: slice 8, the existence rung, is next (§ *Plan for
slices 2–13* below); slices 6 and 7 are not this lane's numbering to
land. The findings table's #2056, #2057, #2118, #2132–#2134, #2141,
#2142 and #2144 stay assigned to the slices that already name them.

Closes #2055. Pins #2051 (closed on rust by #2225). Pins the span half of
#2143 (Q3: the W123 half was not assigned here, so this landing does not
claim it).

## Status (2026-09-24): slice 8 landed

The opus items (VT8.1 to VT8.9) landed across nine commits, `f73ffe63` to
`c387a2ce`, each its own checkpoint — a finer grain than the plan's own
four-checkpoint grouping, which "commit at each coherent point" also
allows (§ *Plan for slices 2–13* › *Slice 8* › *Record (2026-09-24): the
opus items of slice 8*, D157–D172). Two commits from elsewhere landed in
the same sequence: `4c9dc675` (the slice 5 review's fixes, after VT8.1
and before any slice 8 item reads a preserve outcome — D162–D164, D156's
amendment) and `b39e012b` (the consumer-contracts lane's step 2 merge,
over VT8.2). A second implementer then ran the two sonnet items:
`dbd545a1` (VT8.10, the slice's witnesses) and this docs commit,
`wip(value-transfers): slice 8 — the existence rung` (VT8.11, the
landing).

VT8.10 found two gaps the plan's own witnesses did not cover and fixed
them as small, local changes rather than leaving them red: a dead `incr`
on a place the existence rung proves absent was removable even under a
profile spanning 8.4, where the release rule means it may raise —
`assignment_safe_to_delete_with_effect` (`optimiser/elimination.rs`) now
refuses when the statement's own route explanation declined
`unbound-place`; and `chain_fold.rs`'s O104 / O130 pass never anchored a
chain at anything but a literal `set`, so an absent-start `lappend` /
`append` chain (the O104 / O130 row's own claim) did not fold — the
`FunctionLattice` gains an `existence_before` query and the chain may now
anchor at the absent cell's own first write. Both are recorded in
VT8.10's commit and neither moved an existing test's expectation. The
anchor took `try_fold_chain_at` to 101 lines, past pedantic clippy's
limit, which the per-crate `--no-deps` lint missed and `make rust-check`'s
workspace lint caught: `wip(value-transfers): slice 8 — chain_fold under
the line limit` moves it into its own `chain_anchor`, behaviour unchanged.

Green at the landing, the review checklist's suite plus every standing
gate, run after VT8.11's docs:

- `cargo check --workspace --all-targets`: clean;
- `cargo test -p tcl-registry -p tcl-compiler -p tcl-explorer -p
  tcl-lsp-db -p tcl-cli -p xtask`: every crate green — `tcl-compiler`
  alone 9784 passed across 67 binaries, 6 ignored, and 7 doctests;
  `tcl-registry` 1247 and a doctest; `tcl-explorer` 103; `tcl-lsp-db` 129,
  5 ignored; `tcl-cli` 129; `xtask` 237 — no failure;
- `cargo test -p tcl-lsp-server` (not in the review checklist's own
  suite, run in addition since this slice's production fixes touch the
  optimiser core every diagnostic and code action goes through): 1598
  passed, 1 failed, 5 ignored in its e2e binary, plus 593 passed in its
  other binaries; the one failure,
  `rename_safety::fp_namespace_variable_rename_refuses_beside_a_computed_alias_cell`,
  is pre-existing and the consumer-contracts lane's (its CC2.13 row
  records it, fixed on that lane's branch, awaiting merge) — not this
  slice's, and unchanged from VT8.9's own count of it;
- pedantic clippy (`--all-targets --no-deps -D warnings`) on every crate
  VT8.10 touched (`tcl-compiler`, `tcl-cli`, `tcl-lsp-db`), no `#[allow]`
  added anywhere in the slice; `cargo fmt` on the three;
- `cargo xtask value-transfers --check`: 22 files clean, 19 sites waived,
  83 pinned across 34 ratcheted files, 6607 inventory rows, unchanged
  since VT8.9;
- `cargo xtask registry-axes --check`: 7831 vocabulary words, 16 files
  clean, 36 sites waived, 893 pinned across 147 ratcheted files, unchanged
  from VT8.9's baseline — `LANDED` gaining `"slice 5"` and `"slice 8"`
  expires no `until slice N` waiver, so the counts do not move;
- `cargo xtask pack-goldens`: 25 packs, 0 rewritten;
- `cargo xtask kcs-index-links`: "KCS docs checks passed";
- `cargo xtask owner-resolution`: OK, 45 owner rows;
- `cargo xtask retired-api-gate`: OK;
- `cargo xtask dialect-drift`: 8 sites, the eight pre-existing upstream
  ones, none new.

R1's shortfall: none — `dataflow.rs` and `commands.rs` are in
`CLEAN_FILES`, `helpers.rs` is pinned at 4, and `sccp.rs` recognises no
command by spelling (all landed at VT8.4's and VT8.9's own checkpoints).
R6: every existing existence test is byte-identical; the only
expectations that moved are the ones the slice's own Record table names,
each against a witness. R7: checked directly against D166 (the guard
refines every place with no exclusion list, sound because a barrier or
up-frame resets the place first) and D165 (a run without the rung, or a
query the rung leaves undecided, never reads as `Unbound`); no
manufactured value and no re-narrowed widened place in any of this
slice's own tests.

Left to later slices: slice 6 (branch integration) is next in delivery
order, now that the existence branch fact it consumes is stored once;
slice 7 is not this lane's numbering to land yet. The read-recording gaps
VT8.5 and VT8.10 found and left, outside this slice's own scope, are
recorded in
[precision-limitations.md](../compiler/precision-limitations.md): a
nested substitution body and an `uplevel 0` body record no read or write
of the outer frame at all, and a `foreach` list word's own substitution
is not materialised as a use of what it reads (one "Open" entry; the
nested-substitution case is #2231, slice 9's, and the other two are not
yet assigned to a slice); a nested unbind's kill is deliberately left
unmodelled (D167, its own "Accepted" entry); and a procedure's implicit
return value is not a recorded SSA use at all — unrelated to existence,
but found by the same audit (its own "Open" entry).

Closes #2133. Pins #2132 (closed on rust by #2220).

## Plan for slices 2–13

The delivery plan for the rest of
[value-transfers-migration.md](../compiler/value-transfers-migration.md)
§ *The slices*, written against HEAD `bdfe3a96` — the lane's slice-2
checkpoint `f3f9390f` made green by VT2.0 — and against `origin/rust` at
`08bceb36`, which the coordinator merges next (VT2.M; each later slice
names the upstream changes to its files). The planner plans; implementers execute one item
at a time; a reviewer checks each landed item against its entry here. The
design pages stay the specification: this section says where the tree
already differs from them and which wins, orders the work, names the
files, items, tests and gates, and records every choice the pages leave
open. Nothing here changes a ruling of
[value-transfers.md](../compiler/value-transfers.md) § *Rulings*.

### How to read the plan

Every work item has an id (`VT<slice>.<n>`; `VT2.M` absorbs the merge of
`origin/rust`) and the same fields:

- **Files** — every path the item edits or adds.
- **Items** — the Rust items it adds or changes, with signatures. A shape
  the interface page gives verbatim is copied verbatim unless § *Where the
  tree differs from the pages* says the tree's shape wins.
- **Preserves** — behaviour that stays byte-for-byte, with the tests that
  pin it.
- **Changes** — behaviour that changes, each tied to the page sentence,
  validation-matrix row (value-transfers-migration.md § *Validation*) or
  issue that mandates it. A change nothing mandates is not in this plan.
- **Tests** — file, name, what it pins, positive and negative, and the
  oracle releases where the matrix asks for them.
- **Gates** — the standing gates the item runs, by number (below).
- **Docs** — design-page rows, KCS notes, the lane doc.
- **Model** — `opus` for semantics, the solver, the lift, evaluators,
  cores and the CFG; `sonnet` for mechanical rewiring from a given shape,
  waiver annotations, deletions, regeneration, test scaffolding from a
  given witness list, and doc rows.
- **Size** — S (up to about 200 changed lines in at most three files), M
  (up to about 800 lines, or one layer of a subsystem), L (more, or a new
  analysis).
- **After** — the items that land first.

The standing gates:

| Gate | Command | When |
|---|---|---|
| G1 | `cargo xtask value-transfers` (write mode, regenerates `docs/generated/value-transfers.md`), then `cargo xtask value-transfers --check` (`make xtask-value-transfers`) | every item that touches a scanned file, a declaration, `KNOWN_GAPS`, `RATCHET`, `CLEAN_FILES` or the ledger |
| G2 | `cargo xtask pack-goldens` (`make xtask-pack-goldens`); stage every rewritten snapshot | a `CommandSpec` shape change, or a change in a shipped pack's meaning (a new `semantics` declaration is one) |
| G3 | `bash scripts/dev/test-nextest-binary-shards.sh`, after a row in `scripts/dev/rust-test-binary-shards.tsv` for each new test binary (`<shard>\t<crate>::<test>\ttest\t<crate>\t<test>`; new rows go to shard 2, the lightest at 75 rows) | a new integration-test binary |
| G4 | `make codegen` | a generated catalogue changes (a diagnostic or optimisation code's text, `docs/references/command-spec/fields.md`) |
| G5 | `cargo xtask kcs-index-links` | a KCS note is added, moved or relinked |
| G6 | `cargo xtask owner-resolution` | the owner manifest in `docs/design/contracts/shared-utility-contracts-rust.md` or a page's file-path anchors change |
| G7 | `cargo clippy -p <crate> --no-deps --all-targets -- -D warnings` | every crate the item edits |
| G8 | `cargo fmt -p <crate>` | every crate the item edits |
| G9 | `cargo test -p <crate>` for every crate the item edits, plus the suites the slice names | every item |

The ratchet rule every slice applies: a pin is lowered in the commit whose
review removes or waives the sites, with the file's row in the migration
plan's ratchet table in the same commit (the gate holds the two equal); a
pin that reaches 0 leaves `RATCHET` and the table, and the file joins
`CLEAN_FILES`; a file an item touches for other reasons keeps its pin and
may not gain a site. `KNOWN_GAPS` loses a row in the commit that declares
the command's semantics; a stale row fails G1.

The oracle is `/root/.local/bin/tclsh8.4`, `tclsh8.5`, `tclsh8.6`,
`tclsh9.0` and `tclsh9.1`. A witness is run as
`for v in 8.4 8.5 8.6 9.0 9.1; do /root/.local/bin/tclsh$v witness.tcl; done`
(`scripts/dev/tclsh_check.sh -f witness.tcl` covers 8.4 to 9.0 only). A
Rust witness finds the interpreters on `PATH` with the `find_tclsh(series)`
shape of `rust/tcl-registry/tests/differential_fold.rs`, records the
release it ran, and skips a release that is not installed; it never skips
silently when none is.

**Green** at a checkpoint means: `cargo check --workspace --all-targets`;
G7 and G8 for every crate the checkpoint touches; G9 for those crates; G1
with `--check`; G3; and G2 with zero rewrites or its rewrites staged.

The common review checklist every slice's own checklist extends:

- **R1 — the registry rule.** Per-command knowledge lands in
  `tcl-registry` (a declaration, a descriptor, a registry-owned evaluator
  over a shared core); a consumer acquires no command-name or
  command-ID arm (AGENTS.md § *The registry is the source of truth*;
  ruling 1). The files the slice makes clean are in `CLEAN_FILES`, and G1
  reports them clean.
- **R2 — no new `#[allow]`.** Fix the cause.
- **R3 — UK spelling** in identifiers, comments, test names and docs.
- **R4 — the AGPL header** on every new source file (the
  `rust/tcl-registry/src/value_transfer/lift.rs` header); never on
  generated files or fixtures.
- **R5 — identifiers verbatim** from this plan and the pages, or the
  deviation recorded in the lane doc's decisions.
- **R6 — byte-identical tests.** An existing test's expectation changes
  only where a witness (an oracle run or a mandate cited under *Changes*)
  proves today's result wrong, and the commit message names the witness.
- **R7 — soundness.** No manufactured value (an error, a no-match or an
  unbound place is never a value; a decline widens); no re-narrowed
  widened place (a widened value narrows only through a new definition,
  and an externally mutable place — escaping, traced, aliased or
  dynamically named — is never refined); the prefix rule (an
  error completion publishes only the stores that ran); the correlated
  limit (at most one distinct finite SSA value per evaluation; two decline
  with `CorrelatedSets`).

### Where the tree differs from the pages

The code wins wherever its shape differs from a page's, unless it
contradicts a ruling; no row below does. In two rows neither wins: the
page's mechanism cannot be built on the tree, and the plan names what
replaces it. Later items build on the shapes in the *Wins* column.

| Page shape | Tree at HEAD | Wins | Why |
|---|---|---|---|
| `AnalysisInputs::prior_store(&self, place: PlaceRef, …)` | `place: &PlaceRef` (`value_transfer/inputs.rs`) | code | a borrow; no semantic difference |
| `AnalysisInputs::word_structure(…) -> WordStructure` | `-> Result<WordStructure, DeclineReason>` | code | a word the driver cannot read declines rather than inventing a structure |
| `math_function(…) -> Result<CommandBindingIdentity, …>` | `-> Result<BindingIdentity, …>` (`context.rs`) | code | the registry cannot name `tcl_runtime_api` types (slice-1 decision) |
| `FactView::Exact(ExactValue, Option<ValueKey>)` | `Option<ValueIdentity>` | code | `ValueKey` is the compiler's; `ValueIdentity(u64)` is its opaque projection |
| `ExactValue::numeric: Option<ConstValue>` | `Option<NumericValue>` (`answers.rs`) | code | `ConstValue` names the `ConstOps` value in the registry |
| `EvalAnswer::Evaluated(InvocationOutcome)` | `Evaluated(Box<InvocationOutcome>)` | code | pedantic clippy (`large_enum_variant`) |
| `CommandSemantics` with `structure`, `transfer`, `evaluate` | adds `identity()` and `route()`; default bodies (`mod.rs`) | code | the route is a method (slice-1 decision); VT2.4 adds `incoming_targets` |
| `Binder { name: OperandId, … }` | `name: BinderName { Operand(OperandId), Declared(String) }` | code | `try`'s and `catch`'s declared binders have no operand |
| `IterationPlan::body: BodyPlan` | `body: Option<BodyPlan>` | code | the synthetic loop header carries no body operand |
| `DependencyEvidence::route: RouteIdentity` | `Option<RouteIdentity>` | code | `Default` for a detached evaluation |
| `SelectionFact` (fields unstated) | `SelectionFact { selected: Vec<Option<usize>> }` | code | one selected arm (or none) per subject member |
| `EvaluatorCapability { identity, host, target, inputs, depends, budget: tcl_engine_api::Budget, completion }` | `EvaluatorCapability { identity: &'static str }` (`route.rs`) | page, extended in VT4.3 with `&'static` slices and a registry-side `ImplementationBudget` | `EvalRoute` is `Copy` and `tcl-registry` does not depend on `tcl-engine-api` |
| `Interp::set_dialect_profile` | `Vm::set_dialect_profile` (`rust/tcl-vm/src/interp.rs:1656`) | code | naming only |
| `Engine::set_release(&mut self, profile: &'static DialectProfile)` | `tcl-engine-api` is "deliberately dependency-free" (its `Cargo.toml`) | code: `set_release(&mut self, profile: &str)`, the profile's name, resolved by the engine through the registry's dialect ingress (`resolve_known_environment(…).catalogue_profile()`, VT4.1, D76) | a `tcl-dialect` dependency would break the crate's stated design |
| `ActivationStore`: host commands replacing `set` / `incr` / `lappend` / `lassign` "with the whitelisted command's exact semantics" | `HostCommand::invoke(&self, &[Value])` is engine-blind by contract (`rust/tcl-engine-api/src/lib.rs`: "cannot reach the interpreter") | neither: VT4.2 confines stores in the engine | a host command cannot read or write the calling frame, so the page's mechanism cannot keep a body's locals; § *Decisions taken* D10 |
| one interface `Budget`; the evaluation page's `RequestBudget` / `IterationBudget` / `EvaluationBudget` | one `Budget` (`context.rs`) with `request_remaining` and `cancelled` | code: the three levels are three `Budget` values charging through one another (VT4.9) | one type, three owners |
| `scan_defined_and_unset` | `scan_defined_and_unbound` (`sccp.rs:962`) | code | renamed in slice 1 |
| `ConstantBranch` "stored once with its kind" | no kind field (`sccp.rs:175`) | page, added by VT5.12 | |
| `EdgeRefinement { key: ValueKey, … }`, `LoopEnumeration`, `TransferSummary`, `ParamRole` | absent | page; they live in `tcl-compiler` (`sccp.rs`, `static_loops.rs`, `interprocedural.rs`), whose `ValueKey`, `PlaceRef` projection and `ReturnKind` they name | the registry never names SSA identities |
| `ReturnKind` (`Literal`, `Passthrough(param)`, computed) | private `enum ReturnKind { Literal, Passthrough, UsesParam, Other }` (`interprocedural.rs:1650`) | code, made `pub(crate)` in VT13.1 | |
| `AnalysisContext` (interface page) | two values: the registry's `AnalysisContext` (`context.rs`) and the compiler's memo component `AnalysisContextKey` (`value_transfer.rs`) | code | the key is hashable; the context is what evaluators read |
| the loop simulator's `Incr` arm (ledger: retires in slice 12) | runs the registry's route through `exec_cell_update_in_env` already | code | slice 12 deletes the arm's remaining shape, not arithmetic |

### Order of delivery

2, 3, 4, 5, 8, 6, 9, 10, 11, 12, 7a, 13, 7 — the plan's *After* clauses,
with one split. Slice 13's *After* names slice 7, and the only part of
slice 7 that slice 13 consumes is "`summarise_returns` consults a seedless
lattice" (its `TransferSummary::result` is "the shape `summarise_returns`
derives, over the seedless lattice"). That part is **7a** and lands before
13; the rest of slice 7 — the catalogue growth, which no later slice
consumes — lands last as **7**. Slice 8 precedes 6 because the existence
branch fact is what slice 6 stores once (the plan's own note). Every other
slice keeps its number's position relative to its dependencies:

| Slice | After | Before |
|---|---|---|
| 2 (landed 2026-09-23) | slice 1 | everything |
| 3 | 2 | 9, 12 |
| 4 | 2, 3 (the declared route reaches `evaluate_branch` through slice 3's `command` resolver) | 5 |
| 5 | 2, 4 (from slice 4 the four surfaces carry every new declaration, and the parity tests fail closed) | 8, 9, 10 |
| 8 | 5 | 6, 10, 11, 13 |
| 6 | 8 | 11, 12 |
| 9 | 3, 5 | 10 |
| 10 | 5, 8, 9 | — |
| 11 | 6, 8 | — |
| 12 | 2, 3, 6; and 10, whose completion protocol an enumerated loop's error path reads | — |
| 7a | 3 | 13 |
| 13 | 7a, 8 | 7 |
| 7 | 13 | — |

### Slice 2 — the direct vertical slice

**Landed 2026-09-23**, every item but the CLI witness binary (D35),
landed later the same day once VT3.10 opened `rust/tcl-cli/tests/`
(D65); § *Record (2026-09-23): slice 2 landed* at the end of this slice
has the commits, the gates and the deltas.

#### Goal and exit

In the plan's words: "`string range` and `incr` over `ConstOps` and its
admissibility adapters, preserving exact values and target semantics; then
`append` / `lappend`. Standalone analysis and the memoised editor path run
through the same immutable context. Stateful producers are kept when their
results propagate; the `Statement::Incr` deletion predicate is not
generalised; `fold_tail_statement_under_lattice`, `chain_fold`,
`static_loops`, and `intervals` consume the resolved semantics." The
correlated finite-set limit is in force from this slice. The lane's own
classification (`KNOWN_GAPS`, slice 1) adds `set` and the `dict` keyed
updates to this slice, and the ledger adds the retirement of the three
transitional list and length handlers.

*Exit*, in the plan's words: "program (3) of the interface contract folds
in every consumer; the three `incr` models agree on `incr x $n`, leading
zeros, and overflow; `set result [incr n]` keeps its increment; O104 /
O130 fold a non-consecutive chain; direct and memoised paths agree."

Exit evidence:

- Tests: `program_three_folds_in_every_consumer`,
  `the_three_incr_models_agree`, `set_result_incr_keeps_its_increment`,
  `o104_and_o130_fold_a_chain_through_a_lattice_operand` (all in
  `rust/tcl-compiler/tests/value_transfer_witnesses.rs`, VT2.10);
  `optimiser::elimination::tests::store_read_by_a_nested_cell_update_is_not_dead`;
  `value_transfer_parity::direct_and_memoised_lattices_agree_on_the_cell_update_witnesses`
  (`rust/tcl-lsp-db`, VT2.11); `explore_sccp_prints_the_route_of_each_statement`
  (`rust/tcl-cli/tests/value_transfers_cli.rs`, VT2.10).
- Gate: `cargo xtask value-transfers --check` prints `OK` with no
  unrecognised waiver, `KNOWN_GAPS` without `set`, `dict set` / `unset` /
  `incr` / `append` / `lappend` or their `::tcl::dict::` spellings, and
  `const`, `lset`, `ledit`, `lpop` reclassified (VT2.8).
- Inventory rows in `docs/generated/value-transfers.md`: `llength`,
  `list` and `string length` with *Owner* `registry` (today
  `compiler, transitional until slice 2`); `format` with
  `compiler, transitional until slice 3`; `set` as
  ``declared (command) · `cell-write` | direct `cell-write` | registry | yes``;
  `dict incr` as ``declared (subcommand) · `keyed-update:incr` `` and
  `::tcl::dict::incr` as ``declared (command) · `keyed-update:incr` ``,
  both with route ``direct `dict-incr` ``, and the same for `set`,
  `unset`, `append`, `lappend`.
- `tcl explore --source 'proc p {} {set n 1; incr n; incr n 2; return $n}' --show sccp --text --no-colour`
  prints `n#3 = const(4)`, `route incr: direct cell-increment (registry)`
  and `· answer: evaluated` twice; under `--dialect f5-irules`,
  `proc p {} {set z 010; incr z}` prints
  `· answer: declined: release-ambiguous: numeral-grammar`, and
  `set r [llength {a b}]` prints `route llength: direct list-length (registry)`.

#### Work items

##### VT2.0 — the checkpoint made green (landed)

The hand-off's first item: `f3f9390f` made green from the hand-off
section alone — the five failing tests and the two route tests, the
failing `cargo xtask value-transfers` run, the unrun suites and clippy —
without starting the slice's remaining steps. Landed in `206f6eee`
(`wip(value-transfers): slice 2 checkpoint green — tests, gate and
clippy`) and `bdfe3a96` (`wip(value-transfers): slice 2 checkpoint
green`), on top of HEAD rather than squashed onto `f3f9390f` (D1). The
lane doc's "Status (2026-09-22)" section records it; in the hand-off's
terms:

- **Files**: `rust/tcl-compiler/src/analyser/bounds_checks.rs`,
  `analyser/diagnostics/usage.rs`, `analyser/diagnostics/dataflow.rs`,
  `cfg_builder/mod.rs`, `lattice_rebase.rs`, `optimiser/chain_fold.rs`,
  `ssa.rs`, `value_transfer.rs`; `rust/tcl-registry/src/commands/tcl/string_.rs`,
  `value_transfer/const_ops.rs`, `value_transfer/context.rs`,
  `tests/differential_fold.rs`, `tests/value_transfers.rs`;
  `rust/tcl-lsp-db/src/lib.rs`; `rust/tcl-spec-studio/tests/spectcl_ports.rs`;
  `docs/generated/value-transfers.md`; the lane doc.
- **What it did**, hand-off item by item:
  1. The six waivers sit on their own comment line directly above each
     site, the form `site_waiver` reads; G1 regenerated the inventory
     from a passing run.
  2. `rebase_function_unit` shifts `SccpResult::explanations` spans beside
     the constant-branch spans (`rebase_switch_and_while_shift`,
     `rebase_shifted_unit_spans_match_fresh`).
  3. #2050: `uses_in_call` makes a `reads` name the call also defines a
     read of the prior value — the rule `rust` landed in #2215 — so the
     synthetic `<upvar-invalidate>` call's read of `n` is a use and O109
     keeps `set n 1` (`store_read_by_a_nested_cell_update_is_not_dead`);
     `embedded_cell_update_read` keeps W210 where it was, since the
     substitution scan reads braced words too.
  4. `range` carries `const_fold: Some(fold_range_unanimous)` beside
     `const_fold_versioned: Some(fold_range)` (`string_index_comparison_folds_match_tcl`);
     `spectcl_ports`' table documents the subcommand's new fields as
     unrenderable, as slice 1 did for `foreach`.
  5. The storage oracle writes each value as a double-quoted word with
     `\`, `"`, `$`, `[`, `]`, `{` and `}` escaped (`tcl_quoted_word`), so
     `set v a; lappend v \{ b` answers `a \{ b` in every release.
  6. The increment expectations are `built_int(i)`
     (`Constructed(TclType::Int)`), the release rules moved to
     `the_increment_route_reads_numerals_under_the_target_release`; the
     pending `lappend` block is gone.
  7. The `tcl-lsp-db` parity test's `n`-folds-to-4 assertion covers `::p`
     alone.
  8. Pedantic clippy over the five crates, each lint fixed at its cause
     (`CellUpdateWrite`, `explain_fold`, split tests); the five crates'
     suites pass (11218 tests), and `tcl-spec-studio`'s 282.
- **Left for the rest of slice 2**, in the hand-off's own numbering: item
  8 (escapes on the value-position route, VT2.2), item 9 (the
  transitional handlers, VT2.7), item 11 (a value-position cell update
  launders a constant through a join, found by VT2.0, VT2.1), remaining
  step 4 (the `tcl-explorer` text test, VT2.10), and the checkpoint's
  `#[allow(clippy::cast_precision_loss)]` in `ConstOps::as_double`, which
  predates VT2.0 (VT2.2). The lane doc's § *Remaining steps, in order*
  maps onto this plan as 1 → VT2.1, 2 → VT2.2, 3 → VT2.7, 4 → VT2.10 and
  5 → VT2.13, without the squash (D1).
- **Gate output at `bdfe3a96`**: `cargo xtask value-transfers --check` —
  OK, 15 files clean, 16 sites waived, 100 sites pinned across 41
  ratcheted files, 6607 inventory rows; `cargo xtask pack-goldens` — 0
  snapshots rewritten; the shard script, `kcs-index-links` and
  `owner-resolution` pass.
- **Model**: opus. **Size**: L. **After**: —.

##### VT2.M — absorb the upstream merge

The coordinator merges `origin/rust` (at `08bceb36`, 107 commits past the
merge base `3b5eba8a`) into the branch as soon as VT2.0 and DP4.0 are
green, before any further slice work; this item is the lane's half of
that merge. `git diff --stat 3b5eba8a origin/rust` touches, among the
lane's files, `sccp.rs` (+370), `ssa.rs` (+86), `cfg_builder/mod.rs`
(+423), `ir_helpers.rs` (+373), `compilation_unit.rs` (±199),
`command_binding.rs` (+374), `optimiser/elimination.rs` (+60),
`optimiser/chain_fold.rs` (±66), `optimiser/propagation.rs` (±80),
`analyser/diagnostics/dataflow.rs` (±61), `analyser/bounds_checks.rs`
(+64), `lattice_rebase.rs` (±56) and `rust/tcl-lsp-db/src/lib.rs` (+2).
Several upstream commits fix the defect shapes the lane's witnesses pin
(#2050, #2051, #2052, #2054, #2132, #2134, #2141, #2144, #2164) in
upstream's pre-interface structure.

- **Files**: every lane file the merge conflicts in, and
  `docs/generated/value-transfers.md`, `rust/xtask/src/value_transfers.rs`
  (`RATCHET`, `CLEAN_FILES`), `docs/design/compiler/value-transfers-migration.md`
  (the ratchet table), the shipped pack snapshots,
  `scripts/dev/rust-test-binary-shards.tsv`, the lane doc.
- **Rule 1 — one implementation per fact.** Where slice 1 or 2 moved
  logic behind the registry interface and upstream fixed the same logic
  in place, the merged tree keeps the lane's structure and re-expresses
  upstream's semantics through it:
  - `sccp.rs` (#2164): upstream gates every builtin fold on
    `BuiltinFoldInputs { registry_engine, trust: FoldTrust }` and folds
    nothing when `folds` is `None`. `LatticeDriver::trusted` honours the
    stance — `FoldTrust::WholeModule` asks `mutations.trusts(head)`,
    `FoldTrust::ObservedBindings` asks
    `mutations.observed_binding_is_the_builtin(head)` — and declines every
    route when `folds` is `None`; `registry_engine` gates the `const_fold`
    engine (`fold_cmd_subst`'s `const_subst` path) and never a route.
    Upstream's per-command arms in `try_fold_cmd_subst` do not come back.
    The lane's slice-1 decision "the mutation-fact-free shared lattice
    trusts every binding" is withdrawn: upstream's
    `both_stances_decline_a_builtin_a_proc_shadows` is the witness that it
    was wrong. Upstream's test helpers `evaluate_pristine` and
    `evaluate_under_lattice_stance` call the lane's driver with the stance
    they state.
  - `compilation_unit.rs`: upstream bypasses the per-procedure lattice
    memo when the module's trust forbids it, because its key cannot carry
    the fact; the lane's `FnLatticeKey` carries `AnalysisContextKey`, whose
    `CommandTrustSnapshot` is that fact, so the memo stays on and keyed.
    Upstream's `FunctionBuildInputs::command_trust` feeds the driver.
  - `command_binding.rs`: `CommandTrustSnapshot` gains every field
    `ModuleCommandMutations` gained upstream (`RebindingSubjects`, the
    qualified-shadow and unnameable-subject facts), so `to_mutations()`
    round-trips and `observed_binding_is_the_builtin` answers alike from
    the snapshot and from the scan.
  - `optimiser/chain_fold.rs`: upstream's `ChainHeadTrust { set, append,
    lappend }` names three commands in a clean file; the lane's classifier
    already resolves the cell update, so the gate is
    `observed_binding_is_the_builtin` on the statement's own resolved
    head, with no literal name. Upstream's `proc append {varName args}
    {return ZZZ}` witness stays.
  - `ssa.rs`, `cfg_builder/mod.rs`, `ir_helpers.rs` (#2215, #2220):
    upstream's `EmbeddedSubstExtras { defs, read_before_write,
    opaque_global }`, `VariableWriteProjection::read_before_write_names`,
    the `uses_in_call` rule (identical to VT2.0's, kept once),
    `variable_read_projection` / `ConditionEffects`, the in-frame
    expression descent (`in_frame_expression_commands`, with the recovered
    head resolved through the module's bindings) and the
    `SyntheticMarker::UpvarInvalidate` marker are taken; the lane's
    `EmbeddedExtras` and `read_before_written` go. Upstream's #2051 rule
    (a `CONDITIONAL_VARIABLE_WRITE` call reads its targets' prior
    versions) is taken as is; slice 5 derives it from preserve outcomes
    (VT5.11).
  - `analyser/diagnostics/dataflow.rs`: upstream's
    `statement_is_synthetic_effect` (W210 skips a synthetic statement's
    reads) and `script_binds_name` are taken; the lane's
    `embedded_cell_update_read` stays only for the host-`Call` case
    upstream's rule does not reach, as one predicate beside it.
  - `analyser/bounds_checks.rs` (#2054): one path — the lane's
    cell-update evaluation where it computes the new length, upstream's
    `writes_the_name` abstention where it cannot;
    `w231_length_follows_the_cell_updates`,
    `w231_abstains_after_a_write_it_cannot_measure` and
    `w231_still_reports_a_real_overrun` all pass.
  - `parse_literal_value` (#2052): the lane's projection of
    `ExactValue::from_literal` gives upstream's answers (`" 5"` stays a
    string, `5` is `Int`); upstream's `a_literal_keeps_its_surrounding_whitespace`
    joins the lane's tests.
  - `lattice_rebase.rs`: upstream shifts `Return::value_word`, the lane
    shifts `explanations`; both.
  - `value_transfer/const_ops.rs` (#2157, #2151): `TargetSemantics::of`
    reads the three-valued `StringCharacterModel` (8.4 and 8.5, 8.6,
    9.x), keeping the unanimity answer for an unnamed release.
  - `optimiser/elimination.rs` (#2144), `lowering/` (`command_is_unavailable_here`),
    `unit_scope.rs` (#2134), `global_write_info.rs` (in-frame expression
    writes), `var_observability.rs`, `realm.rs`, `script_binds.rs`, the
    codegen and `tcl-cmd-core` changes: taken as they are.
- **Rule 2 — the witnesses stay.** Every lane test upstream now also
  satisfies stays a test; its "today" comment goes, and the examples
  page's `;# today:` comments on the programs upstream fixed (#2050,
  #2051, #2052, #2054, #2132, #2134's single-call-site rewrite, #2141,
  #2144) are restated as the merged behaviour in VT2.12.
- **Rule 3 — the ratchet is re-measured.** G1 over the merged tree: a
  recogniser site upstream added to a clean file is re-expressed (as
  `chain_fold.rs` above) or waived by axis; a ratcheted file whose count
  rose is reviewed and the rise waived by axis — a pin is never raised; a
  new upstream file with a site is reviewed and clean. The migration
  plan's ratchet table is rewritten from the measured `RATCHET`.
- **Preserves**: every test of both sides.
- **Changes**: those upstream's commits make; beyond them, none.
- **Tests**: `the_trust_snapshot_round_trips_every_mutation_field`
  (`command_binding.rs`); G9 for `tcl-registry`, `tcl-compiler`,
  `tcl-explorer`, `tcl-lsp-db`, `xtask` and every crate the merge touches.
- **Gates**: G1 (write mode, then `--check`), G2, G3, G7, G8, G9.
- **Docs**: the lane doc records the merge commit, each resolution above,
  and the re-measured pins.
- **Model**: opus. **Size**: L. **After**: VT2.0 (landed); DP4.0
  (landed, `23eec80f`); the coordinator's merge.
- **Record** (the coordinator's merge of `origin/rust` at `08bceb36`;
  the merge commit's body has every resolution). Where the merged tree
  differs from the item above:
  - `compilation_unit.rs`: `build_with_param_constants_and_classes_under`
    — the one path the memo (`function_lattice`) and the fresh procedure
    build both take — folds under the key's own snapshot
    (`analysis_context.bindings.to_mutations()`, `ObservedBindings`). That
    is VT2.3's *Items*, carried by the merge because the driver now
    declines every route without the fact, so a fact-free memoised unit
    would fold nothing; VT2.3 keeps its test, VT2.11's memoised twin, and
    caching the rebuilt summary in the interned `ValueTransferContext`
    (today it is rebuilt per procedure). Upstream's memo bypass goes with
    `build_procedure_unit_fresh` and `agrees_with_untouched_bindings` (its
    only caller); its `a_shadowing_module_refuses_the_memoised_lattice`
    is rewritten onto the keyed memo as
    `a_shadowing_module_is_served_a_lattice_keyed_by_its_trust`.
  - `chain_fold.rs`: a typed assignment keeps no head, so it is gated over
    every registry spelling of its lowering operation
    (`command_names_for_semantic_operation(StructuredLowering(Set))`); a
    call is gated on its own resolved head.
  - `command_binding.rs`: upstream's snapshot already carries
    `rebinding_subjects`; the round-trip test pins all seven fields.
  - `sccp.rs` tests that expected a route to fold under `evaluate_def` now
    state the stance (`evaluate_pristine`, `sccp_pristine`);
    `evaluate_def_without_the_trust_fact_answers_for_no_command` pins the
    contract. `optimiser.rs`'s
    `a_direct_read_before_write_still_forwards_its_literal` expects `set x
    {1 1}`: the chain folds through the lattice's proven `$x` (8.4.20–9.1b0
    print `1 1` for both programs).
  - G1: upstream's `codegen/emitter/try_blocks.rs` (#2207) added two
    `command == "catch"` sites on `lower_catch`'s defs-only marker, waived
    `irreducible` (a `SyntheticMarker` would retire them; eight consumers
    read `tokens.synthetic`, so not in the merge). Re-measured: 15 files
    clean, 18 sites waived, 100 sites pinned across 41 files, each count
    equal to its pin — no pin moved.

##### VT2.1 — a use the statement does not hold is a permanent miss

The miscompile VT2.0 found (the lane doc's item 11): in `proc f {cond}
{set n 1; if {$cond} {set x [incr n]} else {set x 5}; puts $x}`, `tcl
opt` rewrites `puts $x` to `puts 5` (O100), where `f 1` prints 2 under
every release. The value-position `[incr n]` runs with the host
`AssignValue`'s uses, which do not hold `n` (its read sits on the
synthetic call before the host), so `LatticeInputs::prior_store` reads
version 0 (`unwrap_or(0)`), finds no value and answers `Pending`;
`LiftedAnswer::Pending` becomes `LatticeValue::Unknown`, which survives
the fixed point, and the phi takes the other arm's `Const(5)`.

- **Files**: `rust/tcl-compiler/src/value_transfer.rs`
  (`LatticeInputs::prior_store`), `rust/tcl-compiler/src/sccp.rs` (tests),
  `rust/tcl-compiler/tests/value_transfer_witnesses.rs`.
- **Items**: `prior_store` answers `FactView::Top(DeclineReason::NotExact)`
  for a place whose symbol the statement's uses do not hold — the rule
  `place` already states for a dynamic key: a permanent miss — instead of
  reading version 0 (`unwrap_or(0)`), so the route declines and the
  definition is `Overdefined` rather than a `Pending` that never
  resolves. This is the lane doc's smallest fix; the optimistic
  `Pending` of a use the statement does hold is unchanged.
- **Preserves**: every statement-position cell update, whose uses hold
  its target; `set x [incr n]` keeps slice 1's `Overdefined`.
- **Changes**: `puts $x` after the branch keeps its variable. Mandate:
  R7 (a pending input is never a value, the interface page's "Pending is
  not unbound … no old value is manufactured from bottom"); the lane
  doc's item 11 witness.
- **Tests**: `a_value_position_increment_never_launders_a_join`
  (compiler witnesses: the program above keeps `puts $x` under O100, and
  the optimised program prints 2 for `f 1` and 5 for `f 0` under 8.4 to
  9.1); `sccp::tests::a_missing_use_declines_the_prior_store`.
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: S. **After**: VT2.M.

##### VT2.2 — the hand-off's two leftovers: escapes and the `#[allow]`

- **Files**: `rust/tcl-compiler/src/value_transfer.rs`
  (`fold_cmd_subst_routes`), `rust/tcl-compiler/src/sccp.rs` (tests),
  `rust/tcl-registry/src/value_transfer/const_ops.rs` (`as_double`).
- **Items**: `fold_cmd_subst_routes` cooks each segment text as
  `const_subst.rs::literal_words_at_depth` does — an `Esc` token through
  `tcl_lexer::backslash_subst_in(text, config.escapes)`, a braced `Str`
  token through `WordValueRules::from_config(&config).collapse_braced_word(text)`
  (the lane doc's item 8). `ConstOps::as_double` loses the checkpoint's
  `#[allow(clippy::cast_precision_loss)]` and answers an integer through
  `i.to_string().parse::<f64>()`, the correctly rounded value `i as f64`
  gives (R2).
- **Changes**: `[string range "a\tb" 0 1]` in value position folds to
  `a` followed by a tab. Mandate: the Values row ("whitespace, NUL, and
  backslash preservation").
- **Tests**: `sccp::tests::string_range_in_value_position_reads_cooked_escapes`
  (positive: `set r [string range "a\tb" 0 1]` gives `a\t`; negative:
  `set r [string range {a\tb} 0 1]` gives `a\`; oracle: `puts [string
  range "a\tb" 0 1]` prints `a` and a tab, `puts [string range {a\tb} 0
  1]` prints `a\`, under 8.4 to 9.1).
- **Gates**: G7 (no `#[allow]` left in `value_transfer/`), G8, G9.
- **Model**: sonnet. **Size**: S. **After**: VT2.M.


##### VT2.3 — both paths take the lattice's trust stance

After VT2.M the shared lattice folds under upstream's
`FoldTrust::ObservedBindings` and the optimiser's re-run under
`FoldTrust::WholeModule`, both through `LatticeDriver::trusted`. This item
makes the memoised editor path take the same stance under the same key,
which is the half of "the same immutable context" the merge does not
carry.

- **Files**: `rust/tcl-lsp-db/src/lib.rs` (`function_lattice`,
  `ValueTransferContext`), `rust/tcl-compiler/src/compilation_unit.rs`
  (`build_with_param_constants_and_classes_under`),
  `rust/tcl-compiler/src/value_transfer.rs`,
  `rust/tcl-compiler/tests/value_transfer_witnesses.rs`.
- **Items**: `function_lattice` passes the interned context's
  `CommandTrustSnapshot` as `BuiltinFoldInputs { mutations:
  &snapshot.to_mutations(), registry_engine: false, trust:
  FoldTrust::ObservedBindings, … }`, built once per module (the interned
  `ValueTransferContext` holds the rebuilt `ModuleCommandMutations`), so
  the memoised lattice and `CompilationUnit::build`'s answer every head
  alike.
- **Preserves**: every answer of the direct path after VT2.M.
- **Changes**: the memoised lattice stops folding a head the module
  shadows or renames, as the direct one already does after the merge.
  Mandate: the Consumer parity row ("direct and memoised fact and finding
  equivalence under one context") and the ledger paragraph.
- **Tests**: `the_shared_lattice_declines_a_renamed_head`
  (`value_transfer_witnesses.rs`: with `proc incr {v args} {return 99}`
  at the top level, `proc p {} {set n 1; incr n; return $n}` has `n#2`
  `Overdefined` and the explanation `declined: rebinding-suspected`;
  without the shadow, `Int(2)`); VT2.11 adds its memoised twin.
- **Gates**: G7, G8, G9 (`tcl-compiler`, `tcl-lsp-db`); `memory_growth`
  and `interned_gc` stay green.
- **Docs**: the migration plan's ledger paragraph states the two stances
  and that both paths take them.
- **Model**: opus. **Size**: S. **After**: VT2.M.

##### VT2.4 — one store, applied generically

- **Files**: `rust/tcl-registry/src/value_transfer/mod.rs`,
  `rust/tcl-registry/src/value_transfer/lift.rs`,
  `rust/tcl-compiler/src/value_transfer.rs`,
  `rust/tcl-registry/tests/value_transfers.rs`.
- **Items**:
  ```rust
  // value_transfer/mod.rs — on `CommandSemantics`
  /// The targets whose incoming value and existence this invocation's
  /// evaluation reads. The derived cell updates need no override.
  fn incoming_targets(&self, input: &dyn AnalysisInputs) -> Vec<TargetId> {
      match self.structure(input) {
          PlanAnswer::CellReadModifyWrite { target, .. } => vec![target],
          _ => Vec::new(),
      }
  }
  ```
  `finite_inputs` reads `semantics.incoming_targets(inputs)` in place of
  its `PlanAnswer::CellReadModifyWrite` match. In the driver,
  `cell_update_def` becomes
  `fn store_def(&self, outcome: &InvocationOutcome, def: &str, input: &dyn AnalysisInputs) -> Option<LatticeValue>`:
  an outcome whose only non-`Preserve` store is one `Write { target, value }`
  whose place (`input.place(target.0)`) is the call's single definition
  gives that definition the value; every other shape — two stores, an
  `Unbind`, a `MayWrite`, a target that is not the definition — leaves
  `Overdefined`. `evaluate_call` applies it to every resolved call with an
  evaluated outcome, not only to a cell read-modify-write plan.
- **Preserves**: `append` / `lappend` calls and the typed `Incr` node give
  the values they give at VT2.0.
- **Changes**: none by itself; VT2.5 and VT2.6 reach the lattice through
  it. Mandate for the shape: "Write, preserve, unbind, and may-write
  outcomes" land with slice 5, so this slice applies exactly one write.
- **Tests**: `incoming_targets_default_to_the_cell_update_target`
  (`value_transfers.rs`: `incr` answers `[TargetId(OperandId(0))]`,
  `string range` answers `[]`).
- **Gates**: G1, G7, G8, G9 (`tcl-registry`, `tcl-compiler`).
- **Docs**: the interface page's `CommandSemantics` listing gains
  `incoming_targets` with one line (the evaluation page's `EvalMemoKey`
  already names incoming targets).
- **Model**: opus. **Size**: M. **After**: VT2.M.

##### VT2.5 — `set`, the exact-value write

- **Files**: `rust/tcl-registry/src/value_transfer/cell_write.rs` (new),
  `rust/tcl-registry/src/value_transfer/mod.rs`,
  `rust/tcl-registry/src/value_transfer/route.rs`,
  `rust/tcl-registry/src/commands/tcl/set_.rs`,
  `rust/tcl-registry/tests/value_transfers.rs`,
  `rust/tcl-registry/tests/differential_fold.rs`,
  `rust/tcl-compiler/src/sccp.rs` (tests), `rust/xtask/src/value_transfers.rs`
  (`KNOWN_GAPS` loses `set`).
- **Items**:
  ```rust
  /// `set name ?value?`: the write of an exact value, or the read of the
  /// prior one.
  pub struct CellWriteSemantics;
  pub static CELL_WRITE: CellWriteSemantics = CellWriteSemantics;
  impl CommandSemantics for CellWriteSemantics {
      fn identity(&self) -> &'static str { "cell-write" }
      fn route(&self) -> EvalRoute { EvalRoute::Direct { id: NativeEvalId::CellWrite } }
      fn incoming_targets(&self, input: &dyn AnalysisInputs) -> Vec<TargetId>; // the read form's target
      fn transfer(&self, domain: FactDomain, input: &dyn AnalysisInputs, budget: &mut Budget) -> TransferAnswer;
      fn evaluate(&self, input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer;
  }
  // route.rs
  NativeEvalId::CellWrite // owner: Registry; as_str: "cell-write"
  ```
  The target is the operand with the declared `VarWrite` role
  (`ResolvedInvocationView::operands_with_role`). The write form answers
  the value as the result with one `Write`; the read form answers the
  prior value (`prior_store`; `Pending` passes through; an unbound or
  unknown prior declines, since the read of an absent variable is an
  error) with no store. `transfer(FactDomain::Existence, …)` answers
  `Bind(Scalar)` for the write form; `Type` answers the value's type.
  `set_.rs` declares `semantics: SemanticsDeclaration::Declared(&CELL_WRITE)`.
- **Preserves**: `Statement::Assign` stays the typed node the solver
  transfers natively (`fold_assign_value`); no statement-position `set`
  changes its lattice value.
- **Changes**: `[set x]` in value position folds to `x`'s value
  (`set r [set x]`). Mandate: the Tier-1 list in the migration plan's
  § *Third-party commands* ("`dict incr` / `append` / `lappend` / `set` /
  `unset`") and the slice-1 classification. `[set x 10]` in value position
  stays `Overdefined` (`EffectFreeOnly`; slice 9 admits it).
- **Tests**: `the_cell_write_route_writes_and_reads_the_exact_value`
  (`value_transfers.rs`: `set v { a }` answers the result ` a ` and
  `Write { target 0, " a " }`; `set v` over a prior `Exact("7")` answers
  `7` with no store; over `Pending` answers `Pending`; over
  `Top(UnboundPlace)` declines); `sccp::tests::evaluate_def_set_read_in_value_position_folds`
  (`set x hello; set r [set x]` gives `r` `hello`; negative:
  `set r [set x 10]` leaves `r` `Overdefined`);
  `storage_outcome_witnesses_match_every_release_on_path` gains the write
  and read forms (`set v " a "`, `set v`), under 8.4 to 9.1.
- **Gates**: G1, G2 (the `tcl` pack's meaning changes), G7, G8, G9.
- **Docs**: the interface page's § *Storage-writing commands* names `set`
  as the direct route's one-target write.
- **Model**: opus. **Size**: S. **After**: VT2.4.

##### VT2.6 — the keyed updates of `dict`

- **Files**: `rust/tcl-registry/src/value_transfer/keyed_update.rs`
  (new), `mod.rs`, `route.rs`, `const_ops.rs`,
  `rust/tcl-registry/src/commands/tcl/dict.rs`,
  `rust/tcl-registry/tests/value_transfers.rs`,
  `rust/tcl-registry/tests/differential_fold.rs`,
  `rust/tcl-compiler/src/sccp.rs` (tests), `rust/xtask/src/value_transfers.rs`.
- **Items**:
  ```rust
  /// A keyed update of the dictionary one variable holds, over
  /// `tcl_cmd_core::dict`.
  #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
  pub enum KeyedUpdate { Set, Unset, Increment, Append, ListAppend }
  pub struct KeyedUpdateSemantics { operation: KeyedUpdate }
  pub static DICT_SET: KeyedUpdateSemantics;       // also DICT_UNSET, DICT_INCR,
                                                    // DICT_APPEND, DICT_LAPPEND
  impl KeyedUpdateSemantics {
      pub const fn needs(self) -> Needs;             // DICT_ORDER | LIST_RENDERING, plus
                                                     // NUMERAL_GRAMMAR | INT_TOWER for Increment
      pub const fn evaluator(self) -> NativeEvalId;
  }
  // route.rs: NativeEvalId::{DictSet, DictUnset, DictIncr, DictAppend,
  // DictListAppend}, owner Registry, as_str "dict-set" … "dict-lappend"
  ```
  The dictionary operand is the one with the declared `VarWrite` role, so
  `dict set d k v` (operand 0 is the subcommand word) and
  `::tcl::dict::set d k v` evaluate through one declaration; the key path
  and the value are the operands after it. An absent prior is the empty
  dictionary (every one of the five creates its variable:
  `safe_on_uninit: SpecSurface::ALL_TCL` within `dict`'s surface). A key
  path runs `tcl_cmd_core::dict::lookup` / `upsert` / `remove` level by
  level; `Increment` adds through `ValueOps::int_add` under the numeral
  grammar; `Append` and `ListAppend` run `var::append_bytes` and
  `var::lappend_value` on the keyed value. The answer is the new
  dictionary as the result and one `Write`. `ConstOps` overrides
  `ValueOps::dict_pairs` and `new_dict` to `require` `DICT_ORDER` and
  `LIST_RENDERING` and to charge per element. `qualified_specs()` copies
  `semantics: sub.semantics`.
- **Preserves**: the `dict get`, `exists`, `create`, `keys`, `values`,
  `size`, `merge` folders.
- **Changes**: the five subcommands and their `::tcl::dict::` spellings
  evaluate, so the variable holds the exact dictionary after the call.
  Mandate: § *The drift gate* ("The `append`, `lappend`, `dict incr` gap
  the issue names is a row in that file until slice 2 closes it") and the
  Tier-1 list.
- **Tests**: `keyed_updates_run_the_shared_dict_cores`
  (`value_transfers.rs`), each case an oracle answer under 8.5 to 9.1:
  `dict set d a 1; dict set d b 2; dict set d a 3` gives `a 3 b 2`;
  `dict incr` of an absent variable gives `k 1`; `dict append` twice gives
  `k foobar`; `dict lappend d k a {b c}` gives `k {a {b c}}`; `dict unset`
  of `a` from `a 1 b 2` gives `b 2`; `dict unset` of an absent variable
  binds it to the empty string; `dict set d a y 2` over `a {x 1}` gives
  `a {x 1 y 2}`; `dict set` over `b 2 a 1` keeps the order
  (`b 2 a 1 c 3`); over ` a  1 ` it canonicalises (`a 1 b 2`); over
  `a 1 b` it declines `WrongRepresentation` ("missing value to go with
  key"); `dict incr` over `010` gives 9 under 8.6, 11 under 9.0, and
  declines `ReleaseAmbiguous(NumeralGrammar)` under an unnamed release;
  `the_qualified_dict_spellings_share_the_declaration`
  (`::tcl::dict::incr d k` answers as `dict incr d k`);
  `keyed_update_witnesses_match_every_release_on_path`
  (`differential_fold.rs`, 8.5 to 9.1; under `tcl8.4`, which has no
  `dict`, the resolver finds no route); `dict.rs`'s
  `qualified_specs_carry_the_subcommand_analysis_contract` asserts the
  `semantics` field; `sccp::tests::evaluate_def_dict_incr_writes_the_dictionary`.
- **Gates**: G1 (`KNOWN_GAPS` loses ten rows), G2, G7, G8, G9.
- **Docs**: the interface page's Tier-1 sentence; the KCS declaration note
  (VT2.12).
- **Model**: opus. **Size**: M. **After**: VT2.4.

##### VT2.7 — the three transitional handlers retire

- **Files**: `rust/tcl-registry/src/value_transfer/builtins.rs`,
  `route.rs`, `rust/tcl-compiler/src/value_transfer.rs`
  (`transitional_direct` keeps only `FormatTemplate`),
  `rust/tcl-registry/tests/value_transfers.rs`,
  `docs/design/compiler/value-transfers-migration.md` (three ledger rows go).
- **Items**: the lane doc's item 9. `NativeEvalId::FormatTemplate.owner()`
  answers `Transitional { retires_in_slice: 3 }` and its ledger row reads
  "slice 3 — `format_cmd_with_syntax` under the profile's `NumberSyntax`"
  (VT3.8 retires it). `ListOfArgsSemantics`, `ListLengthSemantics`,
  `StringLengthSemantics` in `builtins.rs`, each overriding `evaluate` over
  `ConstOps`: `tcl_cmd_core::list::list` (`NEEDS = Needs::LIST_RENDERING`),
  `tcl_cmd_core::list::llength` over the list parse, charged per element
  (`NEEDS = Needs::NONE`), `tcl_cmd_core::string::length`
  (`NEEDS = Needs::CHAR_MODEL.union(Needs::SOURCE_ENCODING)`). The statics
  `LIST_OF_ARGS`, `LIST_LENGTH`, `STRING_LENGTH` keep their names and
  identities; `NativeEvalId::{ListOfArgs, ListLength, StringLength}.owner()`
  answers `EvaluatorOwner::Registry`.
- **Preserves**: every existing `evaluate_def_*` answer over `list`,
  `llength` (a literal list and a lattice list) and `string length`; the
  unanimity rule for an unnamed release. A difference between
  `extract_foreach_elements` and the list core on an existing test is
  settled by the oracle (R6).
- **Changes**: the inventory's *Owner* column and the Explorer's route
  line read `registry` for the three, and `compiler, transitional until
  slice 3` for `format`; `string length` over a non-ASCII word read from a
  UTF-8 source declines under 8.x and an unnamed release
  (`ReleaseAmbiguous(SourceEncoding)`) and answers under 9.x — the oracle:
  `string length héllo` from a UTF-8 file prints 6 under 8.4 to 8.6 and 5
  under 9.0 and 9.1. Mandate: the ledger rows ("slice 2 — `ConstOps`
  over the list core", "`tcl_syntax::list::split_list` through
  `ConstOps`", "`ConstOps::char_len` with the `CHAR_MODEL` admissibility
  bit") and the slice-2 decision that `SOURCE_ENCODING` subsumes
  `CHAR_INDEXING` for the string routes.
- **Tests**: `list_and_length_routes_run_the_shared_cores`
  (`value_transfers.rs`: `list a {b c} ""` gives `a {b c} {}`;
  `llength {a {b c}}` gives 2; `llength "a {b"` declines
  `WrongRepresentation`; `string length héllo` gives 5 under 9.0 and
  declines under 8.6); `route_stamps_match_the_pinned_set` with the three
  owners moved.
- **Gates**: G1, G2, G7, G8, G9.
- **Docs**: the migration plan's ledger; the KCS declaration note.
- **Model**: opus. **Size**: M. **After**: VT2.M.

##### VT2.8 — the gap rows the slice does not close

- **Files**: `rust/xtask/src/value_transfers.rs` (`KNOWN_GAPS`),
  `docs/design/lanes/value-transfers.md`.
- **Items**: `const` reads "slice 8 — the write that fails on an existing
  variable reads the existence rung"; `lset`, `ledit`, `lpop` read "slice
  7 — the list cell updates over new shared cores" (D4, D5).
- **Preserves**: every other row.
- **Changes**: the inventory's *Classified* column for four rows.
- **Tests**: `every_file_is_clean_or_at_its_pin` and the gate's verdict
  tests (`cargo test -p xtask`).
- **Gates**: G1, G9 (`xtask`).
- **Docs**: the lane doc's slice-2 decisions.
- **Model**: sonnet. **Size**: S. **After**: VT2.5, VT2.6, VT2.7.

##### VT2.9 — a qualified global written by a nested cell update (#2214)

- **Files**: `rust/tcl-compiler/src/cfg_builder/global_write_info.rs`,
  `rust/tcl-compiler/tests/value_transfer_witnesses.rs`,
  `rust/tcl-cli/tests/value_transfers_cli.rs`.
- **Items**: `collect_write_targets` publishes a `::`-qualified target
  into `GlobalWriteInfo::names` without a `global`, `variable` or `upvar`
  declaration in `state`: the spelling alone makes it outer-scope. A
  renamed alias or an opaque local keeps today's exclusion.
- **Preserves**: every unqualified local target's classification.
- **Changes**: after `bump` (whose body is `set y [incr ::hits]`), O102
  no longer forwards `0` into `puts $hits` and O109 no longer deletes
  `set hits 0`. Mandate: #2214.
- **Tests**: `a_qualified_nested_cell_update_is_a_global_write`
  (compiler: both spellings, `set y [incr ::hits]` and
  `set y [expr {[incr ::hits] + 1}]`, name `::hits` in `bump`'s
  `GlobalWriteInfo`; negative: `set y [incr hits]` with no declaration
  stays local); `opt_keeps_a_global_a_nested_increment_writes` (CLI:
  `tcl opt --profile full` on `set hits 0; proc bump {} {set y [incr
  ::hits]; return $y}; bump; puts $hits` keeps `puts $hits`, and the
  output prints `1` under 8.4 to 9.1, as the original does).
- **Gates**: G1, G7, G8, G9.
- **Docs**: none beyond the lane doc. If a `rust` pull request closes
  #2214 before this item, VT2.M brings the fix and this item keeps only
  the two tests.
- **Model**: opus. **Size**: S. **After**: VT2.M.

##### VT2.10 — the witness binaries

- **Files**: `rust/tcl-compiler/tests/value_transfer_witnesses.rs` (new),
  `rust/tcl-cli/tests/value_transfers_cli.rs` (new),
  `rust/tcl-explorer/src/render.rs` (tests),
  `scripts/dev/rust-test-binary-shards.tsv`.
- **Items**: two integration-test binaries every later slice extends. The
  compiler binary drives `CompilationUnit::build`,
  `optimise_with_dialect` / `apply_optimisations` /
  `optimise_source_multipass` (`tcl_compiler::optimiser::manager`) and a
  local `find_tclsh(series)`; the CLI binary drives
  `env!("CARGO_BIN_EXE_tcl")`. Shard rows:
  `2\ttcl-compiler::value_transfer_witnesses\ttest\ttcl-compiler\tvalue_transfer_witnesses`
  and `2\ttcl-cli::value_transfers_cli\ttest\ttcl-cli\tvalue_transfers_cli`.
  The CLI binary is separate from `rust/tcl-cli/tests/cli.rs`, which the
  diagnostic-policy lane is editing.
- **Tests** (compiler binary):
  - `program_three_folds_in_every_consumer` — `proc p {} {set n 1; incr n;
    incr n 2; puts $n}` under `tcl8.4`, `tcl8.6`, `tcl9.0`, `f5-irules`:
    `n#3` is `Int(4)` in the shared lattice, and the optimiser forwards
    `4` into `puts $n` (O100).
  - `the_three_incr_models_agree` — for `incr x $n` (`x` 5, `n` 3),
    `incr x` over `010`, and `incr x` over `9223372036854775807`, under
    the same four dialects: the lattice value at the definition, the value
    `static_loops::summarise_for_statement` computes for
    `for {set i 0} {$i < 1} {incr i} {<the statement>}`, and the interval
    at the definition agree — equal values, or a decline in every model;
    an interval never excludes the lattice's value. Oracle: 8, then 9
    (8.4 to 8.6) or 11 (9.0, 9.1), then `9223372036854775808` from 8.5
    and `-8` under 8.4, so `tcl8.4` and `f5-irules` decline the overflow.
  - `set_result_incr_keeps_its_increment` — `proc p {} {set n 1; set
    result [incr n]; puts $result; puts $n}; p`: no O109 or O126 on
    `set n 1`, the `incr` stays, and the optimised program prints `2` and
    `2` under 8.4 to 9.1.
  - `o104_and_o130_fold_a_chain_through_a_lattice_operand` —
    `set s hello; set p again; append s $p; puts $s` folds to
    `set s helloagain`; with `set p { again}` to `set s {hello again}`
    (#2052); `set l {}; set x b; lappend l a $x` folds to `set l {a b}`.
  - `deleting_a_quoted_store_leaves_no_quote` — the #2053 program,
    `set s hello; set p "again"; append s $p; puts $s`: the optimised
    source has no line holding only `"` and prints `helloagain` under 8.4
    to 9.1.
  - `lassign_is_not_a_write_under_a_profile_without_it` — under `tcl8.4`,
    `set a old; catch {lassign {new second} a b}; puts $a` keeps
    `set a old`; `tclsh8.4` prints `old` (#2144).
  - VT2.3's and VT2.9's tests.
- **Tests** (`tcl-explorer`, the hand-off's remaining step 4):
  `render::tests::sccp_text_prints_the_route_of_each_cell_update`
  (`render_all` over `serialise_result(&run_pipeline("proc p {} {set n 1;
  incr n; incr n 2; return $n}", "tcl8.6"))` with `["sccp"]` and no colour
  contains `route incr: direct cell-increment (registry)` and
  `· answer: evaluated`; under `"f5-irules"`, `proc p {} {set z 010;
  incr z}` contains `· answer: declined: release-ambiguous:
  numeral-grammar`).
- **Tests** (CLI binary): `explore_sccp_prints_the_route_of_each_statement`
  (the exit evidence lines above); `opt_forwards_program_three`
  (`tcl opt --profile full --dialect tcl8.6` output contains `puts 4`);
  `opt_keeps_the_store_behind_a_nested_increment` (the output keeps
  `set n 1`); VT2.9's test.
- **Gates**: G3, G7, G8, G9 (`tcl-compiler`, `tcl-cli`, `tcl-explorer`).
- **Docs**: the lane doc lists the two binaries as the slices' witness
  homes.
- **Model**: sonnet. **Size**: M. **After**: VT2.1 to VT2.9.

##### VT2.11 — the parity tests in a lane-owned module

- **Files**: `rust/tcl-lsp-db/src/value_transfer_parity.rs` (new,
  `#[cfg(test)]`), `rust/tcl-lsp-db/src/lib.rs` (the test moves out;
  `#[cfg(test)] mod value_transfer_parity;` stays).
- **Items**: the module `use`s `super::*` for `compilation_unit`,
  `unit_build_options`, `lexer_cfg_key` and `declared_command_surface`.
- **Tests**: `direct_and_memoised_lattices_agree_on_the_cell_update_witnesses`
  (moved, with VT2.0's fix); `the_memoised_lattice_declines_a_renamed_head`
  (VT2.3 on the memoised path); `keyed_updates_agree_on_both_paths`
  (`proc p {} {dict set d a 1; dict incr d a 2; return $d}` answers
  `a 3` on both paths under `tcl8.6` and `tcl9.0`);
  `a_trace_installation_invalidates_the_lattice` (adding `trace add
  variable n write …` to the file drops `n`'s constant on the memoised
  path as on the direct one); `file_token_facts_never_evaluates`
  (`file_token_facts` stays structure-only: no route entry is recorded).
  The last two are the Incrementality row's "trace installation" and the
  Consumer parity row's "lightweight tokens and symbols never trigger
  evaluation".
- **Preserves**: `memory_growth` and `interned_gc`.
- **Gates**: G7, G8, G9 (`tcl-lsp-db`).
- **Docs**: none. The lane's footprint in the shared `lib.rs` shrinks to
  `FnLatticeKey`, `ValueTransferContext`, `function_lattice` and the
  module line (B-DP2).
- **Model**: sonnet. **Size**: S. **After**: VT2.3, VT2.6.

##### VT2.12 — the slice's docs

- **Files**: `docs/design/compiler/pass-fact-ownership-matrix.md`,
  `docs/design/compiler/downstream-pass-contracts.md`,
  `docs/design/compiler/sccp-core-analyses.md`,
  `docs/design/compiler/constant-folding-type-inference.md`,
  `docs/design/compiler/optimisation-passes.md`,
  `docs/design/compiler/value-transfers-migration.md`,
  `docs/design/compiler/value-transfers.md`,
  `docs/design/contracts/shared-utility-contracts-rust.md` (the owner row
  only), `docs/design/GLOSSARY.md`,
  `docs/kcs/compiler/kcs-qa-what-does-a-value-transfer-declaration-say.md`,
  `docs/kcs/compiler/kcs-qa-why-does-a-constant-fold-depend-on-the-dialect.md`,
  `docs/kcs/compiler/kcs-qa-why-does-a-renamed-command-stop-folding.md`
  (new) and its index, `docs/design/lanes/README.md`.
- **Items**: the value-transfer fact row (producer `LatticeDriver`;
  consumers; context `AnalysisContextKey`; unavailable is `Overdefined`
  with its `DeclineReason`) with `set` and the keyed updates as producers;
  the O100, O104, O109, O130 rows; the ledger; the Tier-1 sentence;
  *keyed update* and *cell write* in the glossary; the renamed-head KCS
  note (VT2.3's user-visible change); the lanes README's in-flight entry
  names slices 2–13. The rows for `diagnostics-calculation.md` and
  `diagnostics-integration.md` are drafted in the lane doc (B-DP4).
  The owner row is edited only after the diagnostic-policy lane has
  committed its own edit to the same file; another lane's hunk is never
  staged.
- **Gates**: G5, G6.
- **Model**: sonnet. **Size**: M. **After**: VT2.10.

##### VT2.13 — the slice lands

- **Files**: `docs/design/lanes/value-transfers.md` (the slice-2 sections
  read as landed; the hand-off status section becomes a short record).
- **Items**: every gate green over the merged tree; the landing commit.
- **Gates**: G1 to G9 over `tcl-registry`, `tcl-compiler`,
  `tcl-explorer`, `tcl-lsp-db`, `tcl-cli`, `xtask`.
- **Model**: sonnet. **Size**: S. **After**: VT2.0, VT2.M, VT2.1 to VT2.12.

#### Checkpoints and landing

| Checkpoint | Holds | Green means |
|---|---|---|
| `206f6eee`, `bdfe3a96` (`wip(value-transfers): slice 2 checkpoint green`, landed) | VT2.0 | the seven tests pass; G1 `--check` prints `OK`; the five crates' suites and `tcl-spec-studio`'s pass; G7 clean |
| the coordinator's merge commit of `origin/rust`, then `wip(value-transfers): slice 2 — the upstream merge absorbed` | VT2.M | every test of both sides over the merged tree; G1 with the re-measured pins; G2; G3 |
| `6d0de164` (`wip(value-transfers): slice 2 — the join, the escapes, the context and the stores`, landed) | VT2.1, VT2.2, VT2.3, VT2.4, VT2.9 | the join, escape, renamed-head and #2214 tests pass on both paths; every other lattice test of the merged tree byte-identical |
| `4b8a6f6f` (`wip(value-transfers): slice 2 — set, dict and the retired handlers`, landed) | VT2.5, VT2.6, VT2.7, VT2.8 | the route tests and the differential rows pass under every release on `PATH`; G1 without the closed gap rows; G2 with the rewritten snapshots staged |
| `wip(value-transfers): slice 2 — the direct vertical slice` (the landing, landed) | VT2.10 to VT2.13 | every exit test; G1 to G9 |

The landing message, from the checkpoint's draft body:

```text
wip(value-transfers): slice 2 — the direct vertical slice

`ConstOps` is the one live compile-time value model
(`tcl_registry::value_transfer::const_ops`): byte-exact, bound to the
target's release semantics through the admissibility set (`Needs`),
charging the per-evaluation budget, poisoned by the first fault. The cell
updates behind `incr`, `append` and `lappend` are the runtime adapters'
own value computations over it — `ValueOps::int_add`,
`var::append_bytes`, `var::lappend_value` — and `string range` runs the
shared string core with its index numerals resolved under the target's
grammar; the subcommand's shipped folder is that same route. `set` writes
and reads the exact value, the five keyed updates of `dict` run the dict
core for both spellings, and `list`, `llength` and `string length` leave
the compiler's transitional table for registry-owned routes; `format`
stays transitional until slice 3. A profile naming no release gets the
answer every modelled release gives and a recorded decline where they
differ. The lift over finite sets is stated once
(`value_transfer::lift`): one distinct finite input evaluates per member,
two decline as correlated. The driver applies one evaluated write to its
target, records each statement's route and answer for the Explorer's
`sccp` view, and runs the shared lattice under the module's binding
trust, so the memoised editor path and standalone analysis answer alike.
The consumers that carried their own `incr` models read the resolved
semantics: `chain_fold` classifies by the resolved cell update and folds
a `$var` piece through the lattice, the tail fold returns a cell
update's written value, the loop simulator runs the registry's route,
the interval domain interprets the descriptor's `IntegerAdd`, and W231's
length follows the cell updates. A nested cell update's read is an SSA
use, and a `::`-qualified global a nested update writes is in its
procedure's global-write summary.

Behaviour changes: O109, O126 and W220 keep the store a nested cell
update reads (`set n 1` ahead of `set result [incr n]`); a
value-position cell update whose read the host does not hold declines
instead of laundering a join's other constant; `incr` folds
under the target's numeral grammar, so a leading-zero base or an
overflow folds per named release and declines under an unnamed one, and
`incr x " 5"` folds; `string range` folds per release, a non-ASCII
subject under 9.x only, and in value position it reads cooked escapes;
`append`, `lappend`, `set` and the `dict` keyed updates give their
variable an exact value; `string length` of a non-ASCII word declines
under 8.x; the shared lattice declines a head the module renames; the
simulator's literal ingress keeps `010` and `+5` as text; a procedure's
global-write summary names a `::`-qualified global a nested update
writes.

Pins #2050 (closed on rust by #2215), #2052 and #2054 (closed on rust by
#2211), #2053 (closed by 33be5cef), #2144 (closed on rust by #2211),
#2164 (closed on rust by #2169). Closes #2214.
```

The trailer lines are the committing session's. "Closes #2214" stays only
if #2214 is still open when the slice lands; otherwise it reads "Pins
#2214 (closed on rust by #NNNN)".

#### Review checklist

- R1: the files the slice makes clean — the fifteen in `CLEAN_FILES` at
  HEAD stay clean over the merged tree (upstream's named `set` /
  `append` / `lappend` gate in `chain_fold.rs` re-expressed, VT2.M);
  `ssa.rs` (pin 1) and
  `analyser/diagnostics/dataflow.rs` (pin 4) keep their pins and gain no
  site; no consumer matches `"set"`, `"dict"` or a subcommand spelling —
  the keyed updates are reached through the resolver and
  `operands_with_role`, never through `view.subcommand == Some("incr")`.
- R2 to R5 as stated; the new files are `cell_write.rs`,
  `keyed_update.rs`, `value_transfer_witnesses.rs`,
  `value_transfers_cli.rs`, `value_transfer_parity.rs`.
- R6: the only expectations that move are the two route tests (the
  constructed-representation evidence and the pending case), the
  differential oracle's quoting, and the owner fields of three pinned
  routes; each commit names its witness.
- Suites: `cargo test -p tcl-registry -p tcl-compiler -p tcl-explorer -p
  tcl-lsp-db -p tcl-cli -p xtask`.
- R7: a keyed update over a malformed dictionary declines, never writes a
  repaired one; the shared lattice's trust never re-trusts a head the
  module rebinds; a nested cell update's read keeps its store; a finite
  prior of a keyed update's dictionary is one identity, so two finite
  inputs decline `CorrelatedSets`.

#### Behavioural deltas

Accepted in the hand-off and standing (§ *Decisions taken* D2): `incr x
" 5"` folds again; a leading-zero base or an overflow folds under a named
release and declines under an unnamed one; `string range abcdefghijkl
010 end` folds per release; a non-ASCII subject folds under 9.x;
`append` / `lappend` reach the lattice; the simulator's ingress keeps
`010` and `+5` as text. Added by this plan:

| Delta | Mandate |
|---|---|
| O109, O126 and W220 keep `set n 1` ahead of `set result [incr n]` | #2050; the findings table |
| a value-position cell update whose read the host statement does not hold declines, so `puts $x` after `if {$c} {set x [incr n]} else {set x 5}` is no longer rewritten to `puts 5` | R7; the lane doc's item 11 and its `tclsh` witness |
| `[string range "a\tb" 0 1]` in value position folds to `a` and a tab | Values row: "whitespace, NUL, and backslash preservation" |
| the memoised lattice declines a head the module shadows or renames, as the merged direct lattice does | the Consumer parity row; #2164 on the direct path |
| `[set x]` in value position folds to `x`'s value | the Tier-1 list |
| `dict set` / `unset` / `incr` / `append` / `lappend` and their `::tcl::dict::` spellings give their variable an exact value | § *The drift gate*; the Tier-1 list |
| `string length` over a non-ASCII word declines under 8.x and an unnamed release | the ledger row (`CHAR_MODEL`); the slice-2 `SOURCE_ENCODING` decision; the oracle |
| `puts $hits` after a procedure whose nested `[incr ::hits]` writes the global is not forwarded, and `set hits 0` is not deleted | #2214 |

#### Record (2026-09-23): slice 2 landed

Executed by one implementer from HEAD `8b5a8c88`. The session offered no
subagent tool, so the `sonnet` items are the implementer's own work too.
Three commits carry the slice, on top of VT2.0 and the merge: `6d0de164`
(VT2.1–VT2.4, VT2.9), `4b8a6f6f` (VT2.5–VT2.8), and the landing commit
`wip(value-transfers): slice 2 — the direct vertical slice` (VT2.10–VT2.13).
Every item landed except the CLI witness binary of VT2.10 (below), which
landed in its own commit once VT3.10 opened `rust/tcl-cli/tests/` (D65);
§ *Record (2026-09-23): the opus items of slice 3* has its commit and
tests. The decisions the plan did not state are D24–D37 in § *Decisions
taken*; the checkpoint notes below are their first record.

- **Checkpoint `wip(value-transfers): slice 2 — the join, the escapes, the
  context and the stores`** holds VT2.1, VT2.2, VT2.3, VT2.4 and VT2.9,
  with `rust/tcl-compiler/tests/value_transfer_witnesses.rs` created early
  (VT2.1 is its first test) and its shard row. Green: the
  `tcl-compiler`, `tcl-registry` and `tcl-lsp-db` suites (10984 passed, 0
  failed, 11 ignored; `memory_growth` and `interned_gc` included); pedantic
  clippy on the three crates; `cargo xtask value-transfers --check` OK (15
  clean, 18 waived, 100 pinned across 41 files; only waiver line numbers
  moved); `pack-goldens` 0 rewritten; the shard script; `cargo check
  --workspace --all-targets`.

Decisions taken at this checkpoint that the plan does not state (the
landing folds them into § *Decisions taken*):

- **The cooking reaches every reader of a literal word the slice
  evaluates** (VT2.2, R7). Item 8 named the value position only; the same
  raw spelling reached three more readers, each with a witness: a
  statement-position `append s {$x}` then `set t $s; puts $t` was
  rewritten to `puts a1` (the braced `$x` read as a variable) and `append
  s "a\tb"` forwarded `xa\tb` with a literal backslash; the chain fold
  rewrote `lappend l "a\tb"` to `set l {{a\tb}}`; W231 read `append xs
  "\tc"` raw and reported a raise at `lset xs 3 X`, which 8.6 to 9.1
  extend to `a b c X`. One rule, `value_transfer::literal_token_value` (a
  single-token `Esc` word through `backslash_subst_in`, a `Str` word through
  `collapse_braced_word`), is now what `const_subst.rs` calls too; the
  driver reads a call's arguments through its token snapshot
  (`call_arguments`, an argument respelled after lowering is dynamic, and
  with no snapshot a spelling holding a backslash is not a value);
  `LatticeInputs::operand` reads a literal operand as its value and never
  re-reads it as a substitution; the chain fold ends a run at a bare or
  quoted piece that needs backslash substitution, as its `set` anchor does,
  and collapses a braced piece; W231's walk cooks its words.
- **VT2.3's cache is a field of the module facts.** `ModuleAnalysisFacts`
  gains `command_trust: &ModuleCommandMutations`; the interned
  `ValueTransferContext` holds `mutations` beside `key`, built by
  `ValueTransferContext::of`, which needs `ModuleCommandMutations: Hash`
  (implemented over its complete snapshot, so it agrees with `Eq`); a
  whole-module build passes the scan its snapshot was taken from, which
  the snapshot round-trips to
  (`the_trust_snapshot_round_trips_every_mutation_field`). The typed
  `incr` records `declined: rebinding-suspected` as a call does.
- **`store_def` takes no receiver** (pedantic `unused_self`); `call_def`
  wraps it for the typed `incr` and for every other call, and the
  source-layout call path is `evaluate_source_call` (pedantic
  `too_many_lines`).
- **#2214 needed two more pieces than VT2.9's item.** A call site applies
  a callee's summary names as definitions in its own frame, so a
  `::`-qualified name also records the spelling a global-frame caller
  reads (`::hits` is `hits` there; `insert_outer_name`), which fixes the
  sibling `upvar #0 ::hits h; incr h` too; and O109 read no callee reads,
  so it still deleted `set hits 0`: the call site now records the
  callee's summary names as observed (`record_alias_observed`,
  `alias_observed_vars`), as it does for an `upvar` callee. The summary
  records writes, not reads, so every name counts as observed: a store
  ahead of a callee that only writes the global is kept where O109 used to
  delete it (conservative), and `set hits 10; proc bump {} {global hits;
  incr hits 5}` is no longer rewritten to print 5 (tclsh prints 15).

Corrected by the review of the landing: this paragraph said that O109
deletes `set hits 0` ahead of a callee that only *reads* the global (`set
hits 0; proc show {} {global hits; puts $hits}; show; set hits 1`). It
does not reproduce: the rewrite keeps both stores, and tclsh 8.4 to 9.1
print 0 then 1 for the program and its rewrite alike
(`a_store_a_global_reading_callee_observes_is_kept`).

- **Checkpoint `wip(value-transfers): slice 2 — set, dict and the retired
  handlers`** holds VT2.5, VT2.6, VT2.7 and VT2.8. Green: the
  `tcl-compiler`, `tcl-registry`, `tcl-lsp-db` and `xtask` suites (11218
  passed, 0 failed, 11 ignored); pedantic clippy on `tcl-compiler`,
  `tcl-registry` and `xtask`; `cargo xtask value-transfers --check` OK (15
  clean, 15 waived — the three retired arms' waivers went with them — 100
  pinned across 41 files); `pack-goldens` 0 rewritten; the shard script;
  `cargo check --workspace --all-targets`. The keyed-update differential
  agrees with `tclsh` on at least 18 of its 26 cases in each of 8.5 to
  9.1 and declines the rest; under 8.4 the resolver finds no route.

Decisions taken at this checkpoint that the plan does not state:

- **`set` finds both forms by role.** The write form is one `VarWrite`
  operand with the value after it and last; the read form is one
  `VarRead` operand alone. The registry tests give `LiteralInputs` the
  roles the resolver gives the same words (`LiteralInputs::with_role`,
  from `arg_indices_for_role`), so the storage differential runs `set`
  as the lattice does.
- **Three keyed-update answers the plan's list does not name**, each an
  oracle answer under 8.5 to 9.1: `dict incr d k 010` over an absent key
  stores `010` as written once it proves to be an integer (`k 010`); a
  missing intermediate key of `dict unset` is the program's error (`key
  "x" not known in dictionary`) and declines `WrongRepresentation`; an
  absent dictionary is empty only when the inputs prove the variable
  unbound — an unknown prior declines, it is never taken for an absent
  one.
- **The keyed-update differential asks the resolver**, not
  `reg.get("dict")`: the 8.4 registry still answers a `dict` record, and
  `tclsh8.4` prints nothing for the script, so the 8.4 row asserts that
  the resolver finds no route.
- **`llength` reads no release axis.** `ConstOps::list_len` runs the list
  parse alone, charged per element, so `NEEDS = Needs::NONE` holds; the
  default `list_len` goes through `list_elements`, which requires
  `LIST_RENDERING`.
- **`route_stamps_match_the_pinned_set` pins the owner beside the route**,
  so a route that changes owner is a pinned change, not a silent one.
- **G2 rewrote nothing.** The `tcl` commands are the registry's own Rust
  specs, not a shipped `.tclspec`; none of the 24 packs `pack-goldens`
  scans declares `set` or `dict`, so no snapshot moves.

Deltas observed beyond the plan's list, each restated from its oracle:

- `set x puts; set y hello; set cmd [list $x $y]` folds `cmd` to `puts
  hello`: the registry's `list` route reads its operands from the
  lattice, where the transitional `fold_list_cmd` folded literal words
  only. `core_analyses::variable_shape::list_cmd_with_const_vars_diverges_overdefined`
  becomes `list_cmd_with_const_vars_folds_through_the_lattice` (8.4 to
  9.1 print `puts hello`).
- `sccp::tests::string_length_fold_counts_in_the_selected_dialects_character_model`
  is restated under each dialect's own folds: a supplementary character
  gives 1 under 9.0 and declines under 8.6 and an unnamed release (from
  a UTF-8 file 8.4 to 8.6 print 4 and 9.0 and 9.1 print 1), where the
  compiler's model answered 2 under 8.6 — the plan's `string length`
  delta; ASCII still folds under an unnamed release.

- **The landing commit** holds VT2.10 to VT2.13.
  - VT2.10: the compiler binary gained
    `program_three_folds_in_every_consumer`,
    `the_three_incr_models_agree`, `set_result_incr_keeps_its_increment`,
    `o104_and_o130_fold_a_chain_through_a_lattice_operand`,
    `deleting_a_quoted_store_leaves_no_quote` and
    `lassign_is_not_a_write_under_a_profile_without_it` (14 tests, each
    output witness run under 8.4 to 9.1); `tcl-explorer` gained
    `render::tests::sccp_text_prints_the_route_of_each_cell_update`. The
    CLI binary `rust/tcl-cli/tests/value_transfers_cli.rs` was not
    created (D35); its evidence was run by hand with `target/debug/tcl`
    built from this tree: `tcl explore --source 'proc p {} {set n 1; incr
    n; incr n 2; return $n}' --show sccp --text --no-colour` prints `n#3 =
    const(4)`, `route incr: direct cell-increment (registry)` and `·
    answer: evaluated` twice; under `--dialect f5-irules`, `proc p {} {set
    z 010; incr z}` prints `· answer: declined: release-ambiguous:
    numeral-grammar`; `set r [llength {a b}]` prints `route llength:
    direct list-length (registry)`; `tcl opt --profile full --dialect
    tcl8.6` rewrites program (3) to `puts 4` (O100) and keeps `set n 1`
    ahead of `set result [incr n]`; `tcl opt --profile full` keeps `set
    hits 0` and `puts $hits` around `bump`'s `[incr ::hits]`, and the
    output prints 1 under 8.4 to 9.1.
  - VT2.11: `rust/tcl-lsp-db/src/value_transfer_parity.rs` holds the
    moved parity test and `the_memoised_lattice_declines_a_renamed_head`,
    `keyed_updates_agree_on_both_paths`,
    `a_trace_installation_invalidates_the_lattice` and
    `file_token_facts_never_evaluates`; `lib.rs` keeps the `mod` line.
  - VT2.12: the ownership matrix's value-transfer row; the
    downstream-contracts anchor; the SCCP page's decision rule and `incr`
    example; the constant-folding page's routes section; the
    optimisation page's O100–O103, O104/O130 and O107–O109 rows; the
    migration plan's Tier-1 sentence, its `intervals` and `static_loops`
    rows and its O100, O104/O130 and O109/O126 rows; the interface page's
    keyed updates; the glossary's *cell write* and *keyed update* (and
    *value transfer* no longer "proposed; nothing implements it"); the
    two KCS notes, the new
    `kcs-qa-why-does-a-renamed-command-stop-folding.md` and both indexes;
    the owner row (`ConstOps`, `CellWriteSemantics`,
    `KeyedUpdateSemantics` and their files — the diagnostic-policy lane's
    edit to the file was committed in `23eec80f`); the lanes README's
    in-flight entry; and VT2.M's rule 2, the examples page's `today:`
    lines on the programs upstream fixed, restated as `merged:` lines
    re-run on this tree (D37).
  - VT2.13: `a_keyed_update_lifts_its_dictionary_as_one_finite_input`
    pins R7's last clause (a finite dictionary evaluates per member, a
    finite key beside it declines `CorrelatedSets`).
- **Green at the landing**: `cargo test -p tcl-compiler -p tcl-registry
  -p tcl-explorer -p tcl-lsp-db -p xtask` 11330 passed, 0 failed, 11
  ignored; `tcl-cli` and `tcl-spec-studio` 393 passed; `tcl-lsp-core` and
  `tcl-mcp` 3639 passed; pedantic clippy on the five touched crates with
  no `#[allow]` added (VT2.2 removed the one the checkpoint carried);
  `cargo fmt` clean; `cargo xtask value-transfers --check` OK (15 files
  clean, 15 sites waived, 100 pinned across 41 files, 6607 rows);
  `pack-goldens` 0 rewritten; the shard script; `kcs-index-links`;
  `owner-resolution` (43 rows); `make rust-check`.

Rows for the diagnostic-policy lane's owner documents (B-DP4), drafted
here and committed after that lane's slice 10:

- `diagnostics-calculation.md`, § *Deep tier*, the compiler-checks row:
  the lattice those checks and the optimiser read carries the values the
  registry's routes compute (`incr`, `append`, `lappend`, `set`, the
  `dict` keyed updates, `string range`, `list`, `llength`, `string
  length`); a route's decline is recorded in `SccpResult::explanations`
  and is never a diagnostic.
- `diagnostics-calculation.md`, § *Deep tier*, the compiler-checks row
  (slice 8): the same lattice's existence rung (`SccpResult::existence`,
  a flow-sensitive bound / unbound / may-bound fact per place and per SSA
  version) is what W210, W211, W213, W214, O108, O109, and S100 read
  alongside the values the row already names, and what the SCCP
  constant-branch row's I230 / O101 decide an existence-tested condition
  from, one path with every other decided branch; a fast-tier request or
  a unit built below the deep tier never sees it — `Unavailable`, never
  `Unbound`.
- `diagnostics-integration.md`, § *Failure modes*: "the memoised and the
  direct lattice answering differently for one analysis context —
  `rust/tcl-lsp-db/src/value_transfer_parity.rs` pins their agreement";
  § *Anchors*: that module. Since slice 8 the same module's
  `existence_agrees_on_both_paths` pins the existence rung's parity too,
  no separate row needed.

Found and left, outside the plan (recorded for the slices that own them):

- **`set x 1; puts [expr {$x + [set x 10] + $x}]; puts $x`**: O109 now
  deletes `set x 1`, and the optimised program raises `can't read "x"`
  in every release (at `3b5eba8a` O102 forwarded 1 instead). The host
  statement's uses drop a name its nested `[set x 10]` defines, so the
  first `$x` is no use of `set x 1`. #2215's in-frame descent records
  the nested write but not the read before it; VT9.3 ("a read inside a
  braced `expr` is a use … fills any gap the ordered state exposes") is
  where it belongs, and the examples page's ordered-state program now
  says so.
- The `regexp` no-match through `catch` (the examples page's completion
  program) still loses `set a before` to O109 and W220: #2225 fixed the
  direct `regexp` statement only. Slice 10's.
- **#2231 — a `catch` body's reads are invisible** (slice 10, VT10.3).
  The statement form's opaque catch records the body's definitions and no
  reads, and the embedded-substitution scan does not enter a nested
  `[catch {…}]` body at all (`walk_braced_expr_words` descends `Expr`,
  never `Body`), so neither its writes nor its reads reach the host. `set
  x 1; set c [catch {incr x}]; if {$x == 2} {puts two} else {puts "not
  two: $x"}` is reported always false (I230), O112 drops the branch, and
  the rewrite prints `not two: 2`; `set x 5; catch {incr x} m; puts "$x
  $m"` loses `set x 5` to O109; `set x 1; set c [catch {append x y}];
  puts $x` is rewritten to `puts 1`. Recording the reads needs a descent
  that orders a read before a write inside the body — VT10.3's body plan,
  not a contained edit (D44).
- **#2232 — a tab on the CLI's stdout.** O100 forwards `{a<TAB>b}` with
  the tab verbatim (`o100_forwards_a_tab_verbatim`); the space the issue
  shows is the CLI's: `tcl-cli-support`'s `write_highlighted_output`
  expands every tab to spaces whenever the target is stdout, terminal or
  not, for the program-emitting verbs (`rust/tcl-cli/src/commands/transform.rs`
  passes `DEFAULT_TAB_WIDTH`), so `tcl opt f.tcl > g.tcl` changes any tab
  inside any string of the program — O100's word or the source's own —
  while `-o g.tcl` writes it. The fix is the CLI owner's (D43); the issue
  stays open.
- **A relative spelling of a builtin is an unknown command.** `proc p {}
  {set d {a 1}; tcl::dict::set d k v; return $d}` is still rewritten to
  `return {a 1}` (the rewrite prints `a 1`; tclsh 8.5 to 9.1 print `a 1 k
  v`): `CommandRegistry::get` falls back from `::a::b` to `a::b`, never
  from `a::b` to `::a::b`, and an unknown command is taken not to write
  its caller's variables. A registry lookup change with editor-wide
  reach; not this lane's to make in a review fix.
- **The compiled `incr x {$n}` reads a variable named `$n`.** The codegen
  loads the amount through `load_var`, so the bytecode raises `can't read
  "$n": no such variable` where tclsh raises `expected integer but got
  "$n"`; `Statement::Incr::amount_braced` now carries what its fix needs.

#### Record (2026-09-23): review fixes for slice 2

The fable review of `60db3875` returned "land after fixes". The commit
`wip(value-transfers): review fixes for slice 2` holds every fix, each
pinned by a test whose expected output was run under tclsh 8.4 to 9.1.
The decisions are D38–D46 in § *Decisions taken*.

- **The typed assignment reads its word as Tcl substitutes it.** The
  typed nodes carry the word's spelling and the lattice stored it raw:
  `set s "a\tb"; puts [string length $s]` folded to 4 (tclsh: 3), and so
  did `set f [set e]` over `"p\tq"`; `set b "\t"; append b q` held a
  backslash. `AssignConst` is now cooked as a braced word and a marked
  `AssignValue` as an escaped one, through `literal_token_value`
  (`LatticeDriver::literal_value`); an unmarked spelling holding a
  backslash is no value (D38).
  `a_typed_assignment_reads_its_word_as_tcl_substitutes_it` runs five
  programs (3, 3, `<tab>q`, `{x<TAB>y}` and a braced backslash-newline) in
  four dialects and every release.
- **A braced `incr` amount is its text.** `proc p {} {set n 3; set x 1;
  incr x {$n}; return $x}` had `x#2 = const(4)`; every release raises
  `expected integer but got "$n"`. `Statement::Incr::amount_braced`
  (D41); `x#2` is overdefined with `declined: wrong-representation`, and
  `n` has no use (`a_braced_increment_amount_is_its_text`).
- **A leading `#` is quoted per release.** `puts [list # a]` and `set l
  {}; lappend l # b; puts $l` print `# a` / `# b` under tclsh 8.4 and
  `{#} a` / `{#} b` from 8.5; the routes rendered the 8.5 form everywhere,
  and a profile with no release folded it. `TargetSemantics::render_list`
  and `ConstOps::new_list` read the target, and a release-less profile
  declines `ReleaseAmbiguous(ListRendering)` for a first element starting
  with `#` (D42). Two more consumers rendered with the fixed rule and
  miscompiled under 8.4: O130's chain fold (the `lappend` program became
  `{{#} b}`) and O103's variadic `args` seeding (`proc f {args} {return
  $args}; puts [f # a]` became `puts {{#} a}`). Pinned by
  `list_and_length_routes_run_the_shared_cores` (8.4, 8.5, 9.1, no
  release, iRules), three `lappend` rows of
  `storage_outcome_witnesses_match_every_release_on_path`, and
  `a_leading_hash_is_quoted_per_release` (the lattice per dialect; both
  programs' output per release, before and after the rewrite).
- **A rebound `set` stops the typed assignment.** `proc set {name value}
  {return ZZZ}; set s hello; append s world; puts $s` was rewritten to
  `puts helloworld`; tclsh prints `world`. The three typed assignments
  are overdefined unless every command lowered to them is still its
  builtin (D39; `a_typed_assignment_declines_once_set_is_rebound`).
- **#2232** is the CLI's stdout writer, not O100 (D43): the found-and-left
  list above has the cause. `o100_forwards_a_tab_verbatim` pins the
  compiler's half with the review's two programs. The commit does not
  close the issue.
- **#2231** is left to slice 10 (D44), in the found-and-left list above
  with its three programs, and in VT10.3.
- **The nits.** `FactView::exact` replaces the four `exact_input` copies
  (D40). The typed `incr` and the loop simulator's
  `exec_cell_update_in_env`, which lost its `command` parameter, take
  their head from `typed_node_commands` (D39).
  `value_position_routes_agree_on_both_paths` adds `[set x]`, `list`,
  `llength`, `string length` and `::tcl::dict::set` to the parity module
  (the iRules profile has no `dict`, so `d` holds no value there on either
  path). The `hits` record above is corrected, and
  `a_store_a_global_reading_callee_observes_is_kept` pins the correction.
  `ValueTransferContext::of`'s interning of `mutations` is left as it is:
  a later tidy (D46).
- **Found through the parity nit: a keyed update's read** (D45). The
  `::tcl::dict::` spelling in the parity test exposed a wrong rewrite
  older than the lane. The `dict` ensemble's lowering marks its
  sub-mutators as reading their target, but the registry declared no
  `READS_BEFORE_WRITE` on the keyed updates, so every other spelling
  recorded the write without the read and O109 deleted the store it read.
  `proc p {} {set d {a 1}; ::tcl::dict::set d k v; return $d}; puts [p]`
  printed `k v` (tclsh 8.5 to 9.1: `a 1 k v`), and so did a nested `puts
  [dict set d k v]`, the qualified `unset`, `incr`, `append` and
  `lappend`, and `interp alias {} ds {} dict set`.
  `a_keyed_update_reads_its_dictionary_under_every_spelling` runs the
  eight programs from 8.5 (under 8.4, which has no `dict`, each fails
  alike before and after), and `a_route_that_reads_its_target_declares_the_read`
  holds every cell and keyed update in the registry to the trait. The
  relative spelling `tcl::dict::set` is left (above).
- **Green**: `cargo test -p tcl-registry -p tcl-compiler -p tcl-lsp-db`
  11012 passed, 0 failed, 11 ignored; `-p xtask -p tcl-explorer` 327
  passed; pedantic clippy on the three touched crates (`--all-targets`),
  with no `#[allow]` added; `cargo fmt` clean; `cargo xtask
  value-transfers --check` OK (15 files clean, 15 sites waived, 100
  pinned across 41 files, 6607 rows; only two waiver line numbers moved);
  `pack-goldens` OK (24 packs; the `dict` specs are not a shipped pack);
  `cargo check --workspace --all-targets`.

### Slice 3 — the expression slice

#### Goal and exit

In the plan's words: "Registry-owned argument assembly over the shared
expression engine with lazy input services and transitive binding
evidence; the full value result; the nested-substitution policy at
`EffectFreeOnly`; the `command` resolver in `evaluate_branch`; per-member
evaluation of a finite-set condition." The evaluation page adds
`MathFuncSpec::result_class`, and the ledger moves `FormatTemplate` here.

*Exit*: "the `expr` acceptance list in the interface contract; `expr
{"x"}` folds; `abs` rebinding declines; the finite-set condition tests pin
the correlated limit, including the `{1 10 2 20}` and `{1 20 2 10}` mirror
witnesses; the direct-expression and engine entry counts are reported
separately." The acceptance list: "multi-argument forms, braced versus
quoted arguments, short-circuit operators and ternaries, strings that look
like code, math-function rebinding, nested pure substitutions, errors,
bignums, and target release ambiguity".

Exit evidence: `expr_acceptance_list`, `abs_rebinding_declines`,
`a_finite_condition_decides_when_every_member_agrees`,
`the_square_of_one_finite_input_stays_correlated`,
`the_mirror_pairs_decline_as_correlated`,
`route_entries_are_counted_per_family` (compiler witnesses);
`expression_witnesses_match_every_release_on_path`
(`differential_fold.rs`); G1 with `word_subst.rs` and `shimmer/commit.rs`
in `CLEAN_FILES` and `format` owned by the registry; `tcl explore --source
'proc p {} {set r [expr {"x"}]; set n [expr {[string length abcdef] * 2}]}'
--show sccp --text --no-colour` prints `r#1 = const('x')`, `n#1 =
const(12)` and `routes entered: direct 1 · expression 2 · implementation 0`.

#### Upstream starting point

From `git diff 3b5eba8a origin/rust -- rust/tcl-syntax/src/expr/mathfunc.rs
rust/tcl-compiler/src/optimiser/expr_simplify.rs rust/tcl-cmd-core/src/format.rs
rust/tcl-syntax/src/format.rs rust/tcl-compiler/src/word_subst.rs
rust/tcl-compiler/src/tcl_expr_eval.rs`, as merged by VT2.M:

- `mathfunc.rs` (+104): the math-function seam follows the release
  (#2146, from #1944 and #1936), and `IntegerConversion` states what
  `int`, `wide` and `entier` keep of an operand that is already an
  integer (`::tcl::mathfunc::entier 0x10` is `0x10` from 8.5). VT3.3's
  `call` service and VT3.5's `result_class` build on both.
- `expr_simplify.rs` (+170): `try_rewrite_return_expr` gives `return
  [expr {…}]` the rewrites `set v [expr {…}]` has (#1962); VT3.7's type
  guard covers both statement shapes.
- `tcl-cmd-core/src/format.rs` (+198) and `tcl-syntax/src/format.rs`
  (+243): integer conversions honour the size modifier and the release
  width, and Tcl 9 rendering is portable (#2190); VT3.8's route answers
  per release through them, so its `NEEDS` carries `INT_TOWER`.
- `word_subst.rs` (+88): `whole_word_command_tokens` for the value
  emitter; VT3.6 keeps it free of name checks.
- `sccp.rs`: a nested head inside an expression is gated under the trust
  stance of the run that evaluates it (VT2.M).
- `tcl_expr_eval.rs`: unchanged upstream.

#### Work items

##### VT3.1 — the registry assembles the expression

- **Files**: `rust/tcl-registry/src/value_transfer/builtins.rs`,
  `rust/tcl-compiler/src/value_transfer.rs` (`expression_route`,
  `LatticeInputs::word_structure`, `LatticeInputs::operand` for a
  substituted word).
- **Items**:
  ```rust
  /// What `expr`'s argument words assemble to, as the command specifies:
  /// one braced word is the expression text, whose `$name` reads the
  /// engine performs through `variable`; any other word reaches the
  /// engine already substituted; several words join with one space.
  pub enum ExpressionSource {
      Braced { text: String, base: usize },
      Substituted(ExactValue),
  }
  impl ExpressionRoute {
      pub fn assemble(&self, input: &dyn AnalysisInputs) -> Result<ExpressionSource, DeclineReason>;
  }
  ```
  `LatticeInputs::word_structure` answers the word's parts (today
  `Unsupported`), and `operand` of a substituted word concatenates the
  parts' exact values: `Pending` when a part is pending, `Top` with the
  part's reason when one is unknown.
- **Preserves**: every braced `expr` answer; `env_from_uses_numeric` goes
  with the old path, and every existing quoted-form answer that was
  numeric stays.
- **Changes**: a quoted or bare argument is substituted as text before
  parsing, so `set a {1 + 1}; expr "$a * 2"` folds to 3, and `set a
  alpha; set b beta; expr "$a == $b"` declines (the program errors:
  "invalid bareword" from 8.5, a syntax error under 8.4). Mandate:
  "registry-owned argument assembly"; the Expressions row "quoted versus
  braced arguments; multiple arguments".
- **Tests**: `expression_assembly_follows_the_word_kinds`
  (`value_transfers.rs`: braced, quoted, bare, multi-word).
- **Gates**: G1, G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: slice 2.

##### VT3.2 — the full value

- **Files**: `rust/tcl-compiler/src/tcl_expr_eval.rs`,
  `rust/tcl-compiler/src/value_transfer.rs`.
- **Items**:
  ```rust
  /// The shared engine's answer for one expression under the analysis
  /// services.
  pub enum ExprAnswer {
      Value(ExactValue),
      Pending,
      Declined(DeclineReason),
  }
  pub fn evaluate_expression(
      node: &ExprNode,
      services: &mut ExprServices<'_>,
      policy: FoldPolicy,
  ) -> ExprAnswer;
  ```
  A `FoldValue::Str` whose text is a number under the release's grammar
  normalises as Tcl does; any other string is the result. The lattice
  holds a string result as `ConstValue::String`, a numeric one as today.
  `eval_tcl_expr_with_policy` and `TclValue` keep their shapes for their
  six other callers.
- **Preserves**: every numeric answer, `command_substitution_is_none`
  (`[clock seconds]` reads the wall clock and declines under every
  policy), `short_circuit_logical`.
- **Changes**: `expr {"x"}` is `x`; `expr {1 ? "yes" : "no"}` is `yes`.
  Oracle (8.4 to 9.1): `x`, `yes`; `expr {"010"}` is 8 up to 8.6 and 10
  from 9.0, so it declines under an unnamed release; `expr {" 5 "}` is 5;
  `expr {"1.50"}` is 1.5. Mandate: the exit ("`expr {"x"}` folds"); the
  evaluation page's slice-3 row ("the full value").
- **Tests**: in `expr_acceptance_list` (VT3.10).
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: VT3.1.

##### VT3.3 — the lazy services and the binding evidence

- **Files**: `rust/tcl-compiler/src/tcl_expr_eval.rs`,
  `rust/tcl-compiler/src/value_transfer.rs` (`LatticeInputs::nested`,
  `LatticeInputs::math_function`, `LatticeInputs::variable`).
- **Items**:
  ```rust
  /// The engine's `ExprOps` over the analysis inputs: `var` reads through
  /// `variable`, `command` through `nested`, `call` through
  /// `math_function` and the shared dispatcher.
  pub(crate) struct ExprServices<'a> {
      inputs: &'a dyn AnalysisInputs,
      state: &'a mut EvaluationState,
      budget: &'a mut Budget,
  }
  impl tcl_syntax::expr::ExprOps for ExprServices<'_> { /* var, command, call, … */ }
  ```
  `nested` segments one command, resolves it, runs its route through
  `evaluate_lifted`, and admits only an outcome with no store under
  `NestedPolicy::EffectFreeOnly`; anything else is `StatefulNested`.
  `math_function` answers the binding of `::tcl::mathfunc::NAME`: a module
  that renames or defines it (`ModuleCommandMutations`) is
  `RebindingSuspected`; availability follows the release through
  `tcl_registry::mathfunc::available_in_expr`. Every answer's
  `DependencyEvidence::bindings` names `expr`, each nested head and each
  math function. `rand` and `srand` stay the one name check
  (the expected waiver). Each service call checks `Budget::is_cancelled`
  (a cancellation point), and the evaluation charges one `WorkUnits` per
  node of the parsed expression before it runs.
- **Preserves**: `0 && [error never]` never reaches the substitution.
- **Changes**: `expr {[string length abcdef] * 2}` folds to 12 (oracle,
  every release); a nested head the module renames declines the whole
  expression. Mandate: "lazy input services and transitive binding
  evidence"; the Expressions row "nested commands; rebound math functions".
- **Tests**: `abs_rebinding_declines` (compiler witnesses: with `rename
  ::tcl::mathfunc::abs ::tcl::mathfunc::saved_abs; proc
  ::tcl::mathfunc::abs {x} {return 99}` at the top level, `set r [expr
  {abs(-2)}]` declines under `tcl8.6` and `tcl9.0`, and folds to 2 under
  `tcl8.4`; oracle: 99 from 8.5, 2 under 8.4).
- **Gates**: G1 (the `rand` / `srand` waiver), G7, G8, G9.
- **Model**: opus. **Size**: L. **After**: VT3.2.

##### VT3.4 — `evaluate_branch` resolves commands and finite sets

- **Files**: `rust/tcl-compiler/src/sccp.rs` (`evaluate_branch`,
  `branch_decision`, `BranchFold`).
- **Items**: `evaluate_branch` takes the driver's services in place of
  `policy` and `grammar`:
  ```rust
  pub fn evaluate_branch<S: std::hash::BuildHasher>(
      ssa_block: &crate::ssa::SsaBlock,
      condition: &ExprNode,
      values: &HashMap<ValueKey, LatticeValue, S>,
      ssa: &SsaFunction,
      fold: BranchFold<'_>,
  ) -> Option<bool>;
  ```
  `BranchFold` gains `driver: &'a LatticeDriver<'a>`. The condition runs
  through `evaluate_expression` with `EffectFreeOnly`; with exactly one
  distinct finite SSA identity among its reads it runs once per member
  (`PinnedInputs`): every member true is `Some(true)`, every member false
  `Some(false)`, mixed `None`; two finite identities are `None`, recorded
  as `CorrelatedSets`. Its one caller is `branch_decision`.
- **Preserves**: every decided branch today; the version-0 live-in rule.
- **Changes**: `set acc foobar; if {[string length $acc] == 6} {…}`
  decides (I230, O101, O107). Mandate: § *Branch facts* ("Command
  substitutions inside a condition", "Finite-set operands").
- **Tests**: `a_finite_condition_decides_when_every_member_agrees`
  (`foreach a {1 2} {if {$a > 0} {…}}` decides true; `$a > 1` stays
  open); `sccp::tests::evaluate_branch_resolves_a_nested_command`
  (positive with `acc` constant, negative with `acc` a parameter).
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: VT3.3.

##### VT3.5 — `MathFuncSpec::result_class`

- **Files**: `rust/tcl-syntax/src/expr/mathfunc.rs`,
  `rust/tcl-compiler/src/type_infer.rs` (`expr_call_type`).
- **Items**: `MathFuncSpec::result_class(&self) -> MathResultClass`
  (`Int`, `Float`, `Numeric`, `Bool`, `Any`), the one table;
  `expr_call_type` reads it and its own table goes.
- **Preserves**: every `type_infer` test's type for every math function.
- **Changes**: none. Mandate: the analysis table's `type_infer.rs` row
  and the evaluation page's slice-3 row.
- **Gates**: G7, G8, G9 (`tcl-syntax`, `tcl-compiler`).
- **Model**: opus. **Size**: S. **After**: —.

##### VT3.6 — the two lifted-`expr` sites read the registry

- **Files**: `rust/tcl-compiler/src/word_subst.rs` (`lifted_exprs`),
  `rust/tcl-compiler/src/shimmer/commit.rs` (`expr_substitution_body`),
  their callers, `rust/xtask/src/value_transfers.rs`,
  `docs/design/compiler/value-transfers-migration.md`.
- **Items**: `lifted_exprs(tokens, profile, registry: &CommandRegistry)`
  and `expr_substitution_body(lifted, registry)` keep a lifted call whose
  resolved head carries `Traits::EXPR_CONCATENATES_ARGS`, in place of
  `lifted.command != "expr" && lifted.command != "::expr"`.
- **Preserves**: every lifted expression they find today.
- **Changes**: none. Pins: `word_subst.rs` 1 → 0 and `shimmer/commit.rs`
  1 → 0; both join `CLEAN_FILES`; their ratchet rows go.
- **Gates**: G1, G7, G8, G9.
- **Model**: sonnet. **Size**: S. **After**: —.

##### VT3.7 — regrouping consumes the type proof

- **Files**: `rust/tcl-compiler/src/optimiser/helpers/expr_simplify.rs`.
- **Items**: `reassociate_node(node: &ExprNode, types: &OperandTypes) -> Option<ExprNode>`
  regroups an additive or multiplicative chain only when every variable
  term is proven integer in `types`; `instcombine_expr_typed` passes its
  context and `instcombine_expr` an empty one.
- **Preserves**: the closed-subtree folds (`expr {2 + 3 + $x}` to `expr
  {5 + $x}`), and every regrouping over proven integers.
- **Changes**: `set x 10000000000000000.0; expr {$x + 1 + 2}` is not
  rewritten to `$x + 3`. Oracle: 8.5 to 9.1 print `10000000000000002.0`
  for the first and `10000000000000004.0` for the second (8.4 prints
  `1e+16` for both at its default precision). An existing test that
  regroups an untyped term changes, with this witness named (R6).
  Mandate: the O110 row; the Partial knowledge row ("the floating-point
  counterexample").
- **Tests**: `reassociation_refuses_an_unproven_float_term` (positive:
  `$i + 1 + 2` with `i` proven `Int` regroups; negative: `$x` unproven).
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: S. **After**: —.

##### VT3.8 — `format` leaves the transitional table

- **Files**: `rust/tcl-registry/src/value_transfer/builtins.rs`,
  `route.rs`, `rust/tcl-compiler/src/value_transfer.rs`
  (`transitional_direct` and its waiver are deleted),
  `rust/tcl-registry/tests/value_transfers.rs`,
  `rust/tcl-registry/tests/differential_fold.rs`,
  `docs/design/compiler/value-transfers-migration.md` (the row goes).
- **Items**: `FormatTemplateSemantics` over
  `tcl_cmd_core::format::format_cmd_with_syntax(ops, args, syntax)` under
  the profile's `NumberSyntax`
  (`NEEDS = FORMAT_VERBS | NUMERAL_GRAMMAR | INT_TOWER`);
  `NativeEvalId::FormatTemplate.owner()` answers `Registry`.
- **Preserves**: every `%s` and `%d` answer.
- **Changes**: every verb the core supports folds (`format %5.2f
  3.14159` is ` 3.14`, `format %x 255` is `ff`), each an oracle row.
  Mandate: the ledger row.
- **Tests**: `format_runs_the_shared_core` (`value_transfers.rs`);
  `format_witnesses_match_every_release_on_path` (`differential_fold.rs`).
- **Gates**: G1, G2, G7, G8, G9.
- **Model**: opus. **Size**: S. **After**: —.

##### VT3.9 — the entry counts

- **Files**: `rust/tcl-compiler/src/sccp.rs` (`SccpResult`),
  `rust/tcl-compiler/src/value_transfer.rs`,
  `rust/tcl-explorer/src/serialise.rs`, `rust/tcl-explorer/src/view_tree.rs`.
- **Items**: `pub struct RouteTally { pub direct: u32, pub expression:
  u32, pub implementation: u32 }` and `SccpResult::route_tally`, counted
  by the driver at every route entry, nested ones included; `serialise_sccp`
  emits `routeTally`; `build_sccp` renders `routes entered: direct N ·
  expression M · implementation K`. The tally carries no span, so
  `lattice_rebase.rs` is untouched.
- **Tests**: `route_entries_are_counted_per_family` (compiler witnesses);
  `serialise::tests::sccp_reports_the_route_tally`.
- **Gates**: G7, G8, G9 (`tcl-compiler`, `tcl-explorer`).
- **Model**: sonnet. **Size**: S. **After**: VT3.3.

##### VT3.10 — the slice's witnesses

- **Files**: `rust/tcl-compiler/tests/value_transfer_witnesses.rs`,
  `rust/tcl-cli/tests/value_transfers_cli.rs`,
  `rust/tcl-registry/tests/differential_fold.rs`.
- **Tests**: `expr_acceptance_list` — `expr 1 + 2` (3); `set a {1 + 1};
  expr "$a * 2"` (3); `expr {0 && [error never]}` (0); `expr {1 ? "yes"
  : "no"}` (`yes`); `set a {[exit]}; expr {$a eq {[exit]}}` (1: a value
  that looks like code is never re-substituted); `expr {[string length
  abcdef] * 2}` (12); `expr {1/0}` (declines); `expr {2**64}`
  (`18446744073709551616` from 8.5); `expr {1 << 70}` (0 under 8.4, a
  bignum from 8.5, so it declines under an unnamed release); `expr
  {"010" + 0}` (8 against 10: declines unnamed). Also
  `the_square_of_one_finite_input_stays_correlated` (`foreach a {1 2}
  {set r [expr {$a * $a}]}` gives the in-loop `r` the set `{1, 4}`, never
  `{1, 2, 4}`) and `the_mirror_pairs_decline_as_correlated` (both
  `foreach {a b}` loops of the interface page: the `expr` declines
  `CorrelatedSets`, and neither post-loop branch decides until slice 12).
  `expression_witnesses_match_every_release_on_path` runs every program
  above under 8.4 to 9.1. CLI: `explore_sccp_prints_the_route_tally`.
  #2118: `o122_sees_a_self_call_inside_a_braced_expr` is added once
  `rust`'s pull request #2226 is merged into the branch, and not before.
- **Gates**: G7, G8, G9.
- **Model**: sonnet. **Size**: M. **After**: VT3.1 to VT3.9.

##### VT3.11 — docs and the landing

- **Files**: `docs/design/compiler/sccp-core-analyses.md`,
  `constant-folding-type-inference.md`, `optimisation-passes.md` (O101,
  O110), `value-transfers-migration.md` (the `tcl_expr_eval.rs`,
  `word_subst.rs`, `type_infer.rs` rows; the ledger's `expr` assembly and
  `FormatTemplate` rows go), `value-transfers.md` (§ `expr` names
  `ExpressionSource`), `pass-fact-ownership-matrix.md`,
  `docs/kcs/compiler/kcs-qa-why-does-a-constant-fold-depend-on-the-dialect.md`
  (`expr {"010"}`), the lane doc.
- **Gates**: G5, G6, and the slice's green.
- **Model**: sonnet. **Size**: S. **After**: VT3.10.

##### VT3.12 — a BPF expression never takes the Tcl answer

- **Files**: `rust/tcl-registry/src/value_transfer/builtins.rs`
  (`ExpressionRoute`), `rust/tcl-compiler/src/value_transfer.rs`.
- **Items**: the expression route evaluates only under
  `LanguageProfileId::TclExpr`; under `LanguageProfileId::BpfExpr` it
  declines `Unsupported`, because signed division truncates towards zero
  in BPF-Tcl (`-7 / 2` is `-3`) where Tcl floors (`-4`), and the BPF
  arithmetic adapter is the BPF frontend's (`docs/design/compiler/ebpf-backend.md`).
- **Tests**: `a_bpf_expression_never_takes_the_tcl_answer` (negative
  only: `-7 / 2` in a BPF-Tcl program yields no constant); the Tcl
  profile's `-7 / 2` is `-4` under 8.4 to 9.1.
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: S. **After**: VT3.1.

#### Checkpoints and landing

| Checkpoint | Holds | Green means |
|---|---|---|
| `6161601f` (`wip(value-transfers): slice 3 — assembly and the full value`, landed) | VT3.1, VT3.2, VT3.5, VT3.12 | every existing `expr` and `sccp` test byte-identical except the named quoted-form witnesses |
| `e29ba422` (`wip(value-transfers): slice 3 — the lazy services and the branch resolver`, landed without VT3.9) | VT3.3, VT3.4, VT3.9 | the nested-command, rebinding and finite-condition tests pass |
| `wip(value-transfers): slice 3 — format, the expr sites and regrouping` (landed without VT3.6) | VT3.6, VT3.7, VT3.8 | G1 with the two files clean and `transitional_direct` gone; the float witness |
| `wip(value-transfers): slice 3 — the expression slice` (landing) | VT3.10, VT3.11 | every exit test; G1 to G9 |

```text
wip(value-transfers): slice 3 — the expression slice

`expr` is the first client that is not a suffix of literal operands. The
registry assembles its arguments as the command specifies — one braced
word is the expression, any other word is substituted first, several
words join — and the shared engine evaluates them lazily through the
analysis services: `var` reads the proven value, `command` resolves a
nested invocation through the registry under `EffectFreeOnly`, and `call`
proves the math function's binding. The answer is the engine's full
value, so a string result folds, and every answer carries the binding
evidence for its head, its nested commands and its math functions. Branch
conditions resolve nested commands and evaluate once per member of one
finite input; two finite inputs decline as correlated. `format` runs the
shared format core under the profile's numerals, which empties the
compiler's transitional table; the math-function result classes are one
table in `tcl-syntax`; the two lifted-`expr` sites read the registry's
trait; regrouping consumes the type proof, so the floating-point
counterexample is refused; the Explorer reports the direct, expression
and implementation entry counts.

Behaviour changes: `expr {"x"}` folds; a quoted `expr` word is
substituted as text; a nested pure command inside `expr` or a condition
folds; a condition over one finite operand decides when every member
agrees; a rebound math function declines; O110 no longer regroups an
unproven float term; `format` folds every supported verb.
```

#### Review checklist

- R1: `CLEAN_FILES` gains `word_subst.rs` and `shimmer/commit.rs`;
  `tcl_expr_eval.rs` keeps exactly one name check (`rand` / `srand`,
  waived); no consumer matches `"expr"` or `"format"`.
- R2 to R5; the new file is none beyond test additions.
- R6: only the quoted-form witnesses and the untyped regrouping tests
  move, each naming its oracle.
- Suites: `cargo test -p tcl-syntax -p tcl-registry -p tcl-compiler -p
  tcl-explorer -p tcl-cli -p xtask`.
- R7: an `expr` error is a decline, never a value; a nested command with
  a store is `StatefulNested`; the correlated limit holds in
  `evaluate_branch` (two finite identities never decide); a string that
  looks like code is never re-substituted; a release-dependent numeric
  string declines under an unnamed release.

#### Behavioural deltas

| Delta | Mandate |
|---|---|
| `expr {"x"}` and a string-valued ternary fold | exit; the evaluation page's slice-3 row |
| a quoted `expr` word is substituted as text before parsing | "registry-owned argument assembly"; Expressions row |
| nested pure commands fold inside `expr` and conditions | § *Branch facts*; acceptance list ("nested pure substitutions") |
| one-finite-operand conditions decide when every member agrees | "per-member evaluation of a finite-set condition" |
| a rebound `::tcl::mathfunc` function declines | exit ("`abs` rebinding declines") |
| O110 refuses an unproven float regrouping | O110 row; Partial knowledge row |
| `format` folds every verb the core supports | the ledger row |

#### Record (2026-09-23): the opus items of slice 3

One implementer ran the opus items: VT3.1 to VT3.5, VT3.7, VT3.8 and VT3.12.
The sonnet items (VT3.6, VT3.9, VT3.10, VT3.11) are the landing's. The
decisions are D47–D61 in § *Decisions taken*.

A later session runs the sonnet items one at a time, each its own commit;
this table gains a row per item as it lands, and § *Decisions taken*
gains a D-number for any deviation the item needs.

| Item | Commit | What landed | Its tests |
|---|---|---|---|
| VT3.1 | `6161601f` | `ExpressionSource` and `ExpressionRoute::assemble` (registry); the lattice inputs' `word_structure` from each operand's source shape (`OperandSource`) and a substituted operand's value, its parts concatenated; `CommandSemantics::variable_reads`, which the lift counts | `expression_assembly_follows_the_word_kinds` (registry); `sccp::tests::a_quoted_expression_word_is_substituted_before_it_is_parsed` |
| VT3.2 | `6161601f` | `ExprServices`, `ExprAnswer` and `evaluate_expression` in `tcl_expr_eval.rs` (D47–D49); the fused assignment and a value-position `[expr …]` run the engine through the lift (`ExpressionEvaluation`); the full value | `sccp::tests::a_fused_expression_answers_its_full_value` (17 rows) |
| VT3.5 | `6161601f` | `MathResultClass` and `result_class` in `tcl-syntax`; `expr_call_type` reads it and its table went | `mathfunc::tests::every_function_has_a_result_class`; the `type_infer` classification assertions |
| VT3.12 | `6161601f` | `BPF_EXPR`; `assemble` declines `Unsupported` for any language but `TclExpr` | `a_bpf_expression_never_takes_the_tcl_answer` (compiler witnesses; a synthetic command declares the BPF language, D22); the assembly test's BPF row |
| VT3.3 | `6161601f` (the services), `e29ba422` (the evidence and rebinding) | the nested service (`run_script`, `nested_answer`), the math-function service, the binding evidence; the binding scan counts a wrapper command as a builtin (D51) | `abs_rebinding_declines`, `a_rebound_nested_head_declines_the_expression` (witnesses); `value_transfer::tests::an_expression_answer_names_every_binding_it_rests_on`; the `0 && [error never]` row |
| VT3.4 | `e29ba422` | `evaluate_branch` takes `BranchFold` with the driver and evaluates through `evaluate_condition`, per member of one finite input | `a_finite_condition_decides_when_every_member_agrees` (witnesses); `sccp::tests::evaluate_branch_resolves_a_nested_command` |
| VT3.7 | the third checkpoint | `reassociate_node(node, types)`; a closed left operand still folds (`fold_closed_left`) | `reassociation_refuses_an_unproven_float_term`; `o110_reassociates_constant_chains`, `instcombine_reassociation_and_identity_annihilator` and `o110_reassociation` restated over proven integers |
| VT3.8 | the third checkpoint | `FormatTemplateSemantics` over `format_cmd_with_syntax`; `NativeEvalId::FormatTemplate.owner()` is `Registry`; `transitional_direct`, its waiver and `try_format_fold` went; the ledger row went | `format_runs_the_shared_core`, `format_answers_per_release` (registry); `format_witnesses_match_every_release_on_path` (`differential_fold.rs`); `format_folds_through_the_shared_core` (witnesses) |
| VT3.6 | `636f9e2f` | `lifted_exprs` and `expr_substitution_body` resolve `expr` through the registry's `Traits::EXPR_CONCATENATES_ARGS`, not the spelling, matching `optimiser::tail_call` and `optimiser::end_offset`'s existing resolution; `word_subst.rs` and `shimmer/commit.rs` join `CLEAN_FILES`, their ratchet rows go | no new test (R6, no behaviour change): the crate's existing `word_subst` and `shimmer` suites stay green; G1's fall from one pinned site each to zero is the evidence |
| VT3.9 | `11e7cda7` | `RouteTally { direct, expression, implementation }` on `SccpResult`; `call_def` counts `direct`, `run_script` counts `direct` or `expression` per its resolved route (`Implementation` counts too, currently always 0 pre-slice-4), `evaluate_assign_expr` and `evaluate_condition` count `expression`; `LatticeDriver::reset_tally_for_sweep` keeps the fixed point's re-evaluation from over-counting (D62); the Explorer's `sccp` view renders `routes entered: direct N · expression M · implementation K` beside the executable-blocks summary | `route_entries_are_counted_per_family` (compiler witnesses); `serialise::tests::sccp_reports_the_route_tally` |
| VT3.10 | `7f0b3d20` | the slice's remaining witnesses: the acceptance list, the two correlated-limit programs, the release-oracle differential (D63), the CLI witness (`rust/tcl-cli/tests/value_transfers_cli.rs`, new — D65 covers its VT2.10 half) | `expr_acceptance_list`, `the_square_of_one_finite_input_stays_correlated` (D64), `the_mirror_pairs_decline_as_correlated` (D64), `expression_witnesses_match_every_release_on_path` (compiler witnesses); `explore_sccp_prints_the_route_tally` (CLI) |
| VT2.10 (deferred, D35/D65) | the VT2.10 CLI commit | the four CLI tests slice 2 deferred, added to VT3.10's new file: program (3)'s route lines, the `f5-irules` decline, `llength`'s route, `tcl opt`'s forwarding and its kept nested-increment store, and #2214's kept global | `explore_sccp_prints_the_route_of_each_statement`, `opt_forwards_program_three`, `opt_keeps_the_store_behind_a_nested_increment`, `opt_keeps_a_global_a_nested_increment_writes` (CLI) |
| VT3.11 | the landing commit | docs: the ledger's `expr` assembly row goes (`FormatTemplate`'s went with VT3.8); the `tcl_expr_eval.rs` / `word_subst.rs` / `type_infer.rs` rows in "Every analysis, and what changes for it" move their slice-3 content into "today" and close "under this design" to `none`; the driver's file-list line drops "the transitional handlers"; `value-evaluation.md` drops `try_format_fold`; the SCCP, constant-folding, optimisation-passes and ownership-matrix pages gain `expr` / `evaluate_branch` / `RouteTally` rows; `value-transfers.md` names `ExpressionSource`; the leading-zero KCS note gains `expr {"010"}`; § *Status (2026-09-23): slice 3 landed* records the suite and gate counts | none (docs only); G5, G6 green |

Deltas observed beyond the plan's list, each with its oracle:

- The fused assignment's answer is the full value everywhere, so `set r
  [expr {$s}]` with `s` holding `abc` folds to `abc`. A beyond-wide
  result declines under an 8.4 runtime (tcl8.4, the F5 dialects) and under
  a profile with no runtime (f5-bigip). A math function the target's
  grammar lacks declines: `min` under 8.4, and `ABS` anywhere, since the
  wrappers are case-sensitive. *Corrected by the review fixes:* those
  declines were the lattice's only. O101 and the propagation folds
  re-folded the same expressions through the old constant folder, which
  had none of the gates, so `tcl opt --profile full` still rewrote them;
  § *Record (2026-09-23): review fixes for slice 3* has the programs and
  the fix (D68).
- A substituted operand folds for every route, not only for `expr`: the
  lattice inputs concatenate its parts, so `lappend l "$a b"` over a
  constant `a` folds.
- A pending `[expr …]` in value position is `Unknown` (optimistic), as a
  direct route's has been since slice 2; it used to be `Overdefined`.
- `isfinite`, `isnormal`, `issubnormal` and `isunordered` infer `Boolean`;
  they inferred `Numeric`. tclsh 9.0 and 9.1: `isfinite(1.0)` is 1.
- `rch_conditional_break_keeps_loop_tail_reachable` now uses `$x > 1`.
  Over `{1, 2}` the old `$x > 5` is always false: tclsh prints `inner` twice,
  so O107 removing the break is right.
- The two iRules glob tests write a bracket class braced (`{a[bxy]c}`). A
  quoted `"a[bxy]c"` runs the command `bxy`; tclsh 8.4 to 9.1 raise
  `invalid command name "bxy"`.
- O103's call-site fold renders a double as Tcl does (D58).
- The shared format core's zero precision follows the release (D57).
- The optimiser samples moved with VT3.7 (D61).

Found and left:

- **8.4's double rendering.** O100 and O103 spell a double in the 8.5+
  shortest form under a tcl8.4 target, where tclsh 8.4 prints 12
  significant digits: `10000000000000002.0` against `1e+16`. The rendering
  is an axis no route models.
- **A finite input read only inside a nested command** declines as
  correlated rather than evaluating per member. `PinnedInputs` passes
  `nested` through to the unpinned inputs, which is sound but imprecise.
- **VT3.11's pages**:
  - the migration plan's `expr` assembly ledger row, and its
    `tcl_expr_eval.rs`, `word_subst.rs` and `type_infer.rs` rows;
  - the driver's description in its file list, which still names "the
    transitional handlers";
  - `value-evaluation.md`'s `try_format_fold` sentence.

The state the sonnet items start from:

- **VT3.11**: the pages listed under "Found and left" above.

Green at the third checkpoint:

- tests:
  - `cargo test -p tcl-cmd-core -p tcl-syntax -p tcl-registry -p
    tcl-compiler -p tcl-lsp-db -p tcl-explorer -p xtask`: 12030 passed,
    0 failed, 11 ignored;
  - `tcl-cli`: 116 passed, `samples_optimiser_profiles_are_regenerated`
    included;
  - `tcl-lsp-core`: 3560 passed;
- pedantic clippy on `tcl-cmd-core`, `tcl-syntax`, `tcl-registry` and
  `tcl-compiler`, with no `#[allow]` added;
- `cargo fmt` clean on those crates;
- `cargo xtask value-transfers --check` OK: 15 files clean, 13 sites waived
  (the two transitional sites went), 100 pinned across 41 files, 6607 rows;
- `pack-goldens` OK.

`tcl-vm`'s `encoding_command` and `ensemble_subcommand_words_resolve_like_tclsh`
fail in this container either way. They read the system encoding
(`iso8859-1` here), not the format core.

#### Record (2026-09-23): review fixes for slice 3

The fable review of `497f47cc` returned "land after fixes". The commit
`wip(value-transfers): review fixes for slice 3` holds every fix, each
pinned by a test whose expected output was run under tclsh 8.4 to 9.1.
The decisions are D66–D71 in § *Decisions taken*.

- **A namespace-local math function is a rebinding.** From 8.5 `expr`
  resolves `tcl::mathfunc::NAME` relative to the namespace it runs in
  before the global one, so in `namespace eval ns { namespace eval
  tcl::mathfunc { proc abs {x} {return 99} }; proc p {} {set r [expr
  {abs(-2)}]; return $r} }; puts [ns::p]` the call runs
  `::ns::tcl::mathfunc::abs`: tclsh 8.5 to 9.1 print 99, 8.4 prints 2.
  The math-function service proved only the global wrapper, so the
  lattice held `r` at 2 and `tcl opt` printed 2 on every release. The
  binding scan now counts a definition in any `tcl::mathfunc` namespace
  as a rebinding of the global wrapper it shadows (D66);
  `abs_rebinding_declines` runs the program beside the global one, the
  lattice under three dialects and both programs optimised under every
  release.
- **A condition over an invalid octal decides nothing.** `truth_of` read
  any digit string as a beyond-wide integer's canonical spelling, so `set
  x 08; if {$x} …` was decided true (I230) and rewritten to `if {1}`
  under tcl8.6, where tclsh 8.5 and 8.6 raise `expected boolean value but
  got "08" (looks like invalid octal number)`; 8.4, 9.0 and 9.1 print
  `yes`. Only the canonical spelling is a number now (D67).
  `an_invalid_octal_condition_decides_nothing` runs `08`, `-08` and the
  quoted `"08"` under tcl8.6 (undecided) and tcl9.0 (decided), and each
  program optimised under every release;
  `value_transfer::tests::a_condition_reads_only_canonical_digits_as_a_number`
  pins the reader.
- **The rewrites fold on the route.** O101 re-folded through the old
  constant folder, which had none of the route's gates: `tcl opt
  --profile full` rewrote `expr {1 << 70}`, `expr {1e308 * 10}` and `expr
  {min(1,2)}` under tcl8.4 (tclsh 8.4: `0`, `floating-point value too
  large to represent`, `unknown math function "min"`) and `expr
  {ABS(-2)}` under every dialect (every release raises). O101's two
  sites, the branch folds and the propagation folds now ask the route
  (D68). `o101_rewrites_only_what_the_route_proves` pins the rewritten
  output of those programs under the four dialects, and
  `expression_witnesses_match_every_release_on_path` runs each of its 13
  programs optimised under every release beside the lattice check. Two
  more of the same class turned up while fixing it:
  - the route's own tower checked an infinity only at the result, so
    `expr {(1e308 * 10) > 0}` folded to 1 under tcl8.4, where tclsh 8.4
    raises at the product (D69; a row of the O101 witness);
  - O112 decided a condition with the old folder, with neither tower nor
    binding. Under tcl8.4 `if {(1 << 70) == 0} {puts zero} else {puts
    big}` became `puts big` (tclsh 8.4 prints `zero`). With `proc
    ::tcl::mathfunc::abs {x} {return 99}`, `if {abs(-2) == 2} {puts two}
    else {puts other}` became `puts two` and `while {abs(-2) == 99} {puts
    loop; break}` went, where tclsh 8.5 to 9.1 print `other` and `loop`.
    O112 decides on the route (`decide_condition_detached`), and the old
    folder keeps the tower for its remaining callers
    (`a_structure_fold_stays_within_the_targets_tower`;
    `tcl_expr_eval::tests::the_old_folder_stays_within_the_targets_tower`).
- **D64's deferral is in the plan.** VT5.7's tests and slice 5's R7 say
  the mirror-pairs witness gains its `CorrelatedSets` reason once the
  two-binder `foreach` source is lowered. The interface page's
  § *The correlated finite-set limit* no longer says slice 3's finite-set
  tests pin the reason: the witness pins the outcome, and the reason
  comes with slice 5.
- **The pages.** On the examples page, O101's `n` and `t` lines, the
  correlated rung and both I230 lines are `merged:` observations (D37),
  re-run through `tcl opt --profile full`, `tcl diag --json` and tclsh
  8.4 to 9.1, and the status note says so. The O101 prose's `n` becomes
  12, not 6 (every release prints `12 x 0`). The KCS note on dialect
  folds says the expression route reads the profile's runtime and states
  the iRules disagreement below. `value-evaluation.md` § *The expression
  route* describes the built route: D47's crate-private adapter
  (`ExprServices`, `evaluate_expression`, `ExprAnswer`), `expr {"x"}`
  folding, `MathFuncSpec::result_class` and `expr_call_type` reading it,
  the old folder's remaining callers, and the rewrites asking the route.
- **The pins.** D60:
  `value_transfer::tests::a_simple_reference_is_one_reference_only`
  (`${a} + ${b}` is no single reference). D57: `format_answers_per_release`
  gains the `%.0d 0` row (tclsh 8.4 prints the empty string, 8.5 to 9.1
  `0`).
- **A record error, not amended.** `cb0fe443`'s trailer reads "Pins
  #2214 (closed on rust by #NNNN)", the plan's placeholder left unfilled.
  #2214 was closed by this lane's `60db3875` ("Closes #2214"), so the line
  should have read "Pins #2214 (closed by 60db3875)".
- **The route tally counts an entry once the route is entered** (D70).
  With the module's own `proc expr {args} {return 99}`, `set r [expr {1 +
  1}]` declines at the trust check and counted one expression entry; it
  counts none now (`route_entries_are_counted_per_family`'s fourth
  program).
- **The nits** (D71): `ExprServices::quoted_string` assembles an element
  name through `value_transfer::variable_name`, and `fold_expr_under_lattice`
  builds its constants once (`const_to_exact`) and asks the route, so
  `const_to_env_value` went, and `sccp::tcl_value_to_const` with its last
  caller.
- **The two routes disagree under iRules today.** With `set z 010`,
  `incr z` declines `release-ambiguous: numeral-grammar` under
  f5-irules, because the direct routes ask every release, while `expr {$z
  + 1}` folds to 9, because the expression route reads the runtime's 8.4
  grammar (`tcl explore --show sccp --dialect f5-irules`). tclsh 8.4
  prints 9 for both, so the decline is the imprecise half. Ruling 8
  resolves it: VT4.1 makes `TargetSemantics::of` read a declared base
  release, and `incr z` then folds to 9 there as well.

Found and left:

- **A read inside a command nested in a fused assignment's braced `expr`
  is no use of the statement.** `set n [expr {[string length $s] * 2}]`
  lowers with no uses, where `puts [expr {[string length $s] * 2}]` and an
  `if` condition over the same expression record `s`, so the nested
  `string length` declines `not-exact` and `n` stays unproven (the
  examples page's O101 line). The store it reads is kept: no O109, no
  W211. VT9.3 is the nearest item.
- **The static loop simulator reads no math-function binding.** With
  `proc ::tcl::mathfunc::abs {x} {return 99}`, `proc p {} {for {set i 0}
  {$i < abs(-3)} {incr i} {}; if {$i == 3} {puts three} else {puts
  other}}` prints `other` under tclsh 8.5 to 9.1; the simulator ends the
  loop at `i` 3, so `tcl opt` rewrites the condition to `1` and drops the
  `else` arm. The simulator is VT12.3's, which now names the program. The
  tower half is fixed here: the old folder the simulator calls declines
  past an 8.4 target's tower.
- **Code generation's constant operands** fold a `Binary` over a math
  function call through the same folder, so a rebound function would be
  baked into generated code as well. Noticed in passing and not checked
  end to end.

Green:

- tests: `cargo test -p tcl-compiler` 9715 passed, 0 failed, 6 ignored;
  `tcl-registry`'s `value_transfers` 24 passed; `-p tcl-explorer -p
  tcl-lsp-db -p xtask` 451 passed, 0 failed, 5 ignored; `tcl-cli` 124
  passed, `samples_optimiser_profiles_are_regenerated` included (no
  sample moved); `tcl-lsp-core` 3562 passed;
- pedantic clippy on `tcl-compiler` and `tcl-registry` (`--all-targets`),
  with no `#[allow]` added; `cargo fmt` clean on both;
- `cargo xtask value-transfers --check` OK and unchanged: 17 files clean,
  13 sites waived, 98 pinned across 39 ratcheted files, 6607 rows;
  `pack-goldens` OK (24 packs, none rewritten);
  `scripts/dev/test-nextest-binary-shards.sh` OK; `cargo xtask
  kcs-index-links`; `cargo check --workspace --all-targets` clean.

### Slice 4 — a private SpecTcl command through the same interface

#### Goal and exit

In the plan's words: "The loader, renderer, studio, cache inputs (target
values), overlay invalidation (`spec_pack_key` reaching
`compilation_unit`), per-evaluation state isolation in the host,
`-native` resolution for every family, and `Engine::set_release`,
delivered together on one small executable example before any catalogue
migration; shipped builtins stay on the direct route." The evaluation
page's slice-4 rows add `ActivationStore`, the two policies,
`EvaluatorCapability`, the provisioning path, `EvalMemoKey`'s
incoming-target and dependency components, `EvaluatorGeneration`, the
`semantics` / `evaluate` / `facts` statements, the body verbs, the
`SCOPE::FIELD` id rule, the per-family tables, the four surfaces and the
`spectcl_check` findings. The request and iteration budgets the hand-off
deferred land here (D3).

*Exit*: "step 1 of the completion test — a rename and a subcommand form
with different operand positions need no consumer edit; a workspace
pack's evaluator reaches a diagnostic on the memoised path; a body with a
global counter answers identically on every call."

Exit evidence: `the_completion_test_needs_no_consumer_edit` (compiler
witnesses), `a_workspace_pack_evaluator_reaches_i230_on_the_memoised_path`
(`value_transfer_parity.rs`),
`a_body_with_a_global_counter_answers_identically_on_every_call`
(`rust/tcl-spec-hooks/tests/containment_e2e.rs`),
`shipped_builtins_stay_on_the_direct_route` (`value_transfers.rs`);
`spectcl_roundtrip.rs`, `native_hook_tables_cover_their_catalogues`,
`value_tables_cover_their_catalogues` and `reference_doc.rs` green; the
generated inventory unchanged (the fixture is not a shipped pack); and
`tcl explore --show sccp --text` over `set r [tenant::label acme]`, run
in a scratch workspace whose discovered pack is the fixture, prints
`r#1 = const('tenant:acme')` and a `route tenant::label: implementation`
line with `· answer: evaluated`.

#### Upstream starting point

From `git diff 3b5eba8a origin/rust -- rust/tcl-spectcl/src
rust/tcl-vm/src/interp.rs rust/tcl-lsp-db/src/lib.rs rust/tcl-spec-hooks
rust/tcl-engine-api rust/tcl-engine-tclvm rust/tcl-spec-studio`:

- `tcl-spectcl/src/loader.rs` (+162) and the new `loader/dialect_block.rs`
  (+244): dialect blocks, `case_list` rows for every descriptor field, and
  `state_transition` / `world_effect` blocks (#2140). VT4.4's statements
  load inside a dialect block like any other row.
- `loader/eval.rs` (±53): a tier's provenance decides E-R2's untrusted
  rules (#2139: `Tier::StudioOverride` untrusted, `Tier::Workspace`
  trusted until Workspace Trust reaches discovery). An untrusted pack
  cannot register a reserved compiled name, so it cannot move a shipped
  builtin off the direct route; under ruling 3 its evaluator for its own
  command loads.
- `tcl-vm/src/interp.rs` (+204) moved under VT4.2's store path (package
  validation, parse-error seams, TclOO private variables); VT4.2 locates
  every name-resolving store in the merged file.
- `tcl-lsp-db/src/lib.rs` (+2, `TopLevelKind::Script`): VT4.8 unaffected.
- `tcl-spec-hooks`, `tcl-engine-api`, `tcl-engine-tclvm`: unchanged;
  `tcl-spec-studio`: a test only.

#### Work items

##### VT4.1 — `Engine::set_release`

- **Files**: `rust/tcl-engine-api/src/lib.rs`,
  `rust/tcl-engine-tclvm/src/lib.rs`, `rust/tcl-spec-hooks/src/host.rs`.
- **Items**:
  ```rust
  trait Engine {
      /// Pin every later compilation and invocation to the named dialect
      /// profile. Called once per (pack, profile), after the engine is
      /// built and before `compile`.
      fn set_release(&mut self, profile: &str) -> Result<(), EngineError> {
          let _ = profile;
          Err(EngineError::Unsupported("pinning a release"))
      }
  }
  ```
  The argument is the profile's name, not the page's
  `&'static DialectProfile`: `tcl-engine-api` is dependency-free by
  design (its `Cargo.toml`), so the engine resolves the name
  (`DialectProfile::find`). `TclVmEngine` calls `Vm::set_dialect_profile`.
  The host keys its engines by (pack, profile, thread); a second profile
  is a second engine and a recompile, never a per-call setter; a pack
  whose capability names a release the engine cannot pin gets a load
  notice and no route.
- **Tests**: `tcl-engine-tclvm`: `set_release_pins_the_numeral_grammar`
  (a body `fold [expr {010 + 0}]` answers 8 under `tcl8.6` and 10 under
  `tcl9.0`; an unknown profile name is `Unsupported`).
- **Gates**: G7, G8, G9 (`tcl-engine-api`, `tcl-engine-tclvm`,
  `tcl-spec-hooks`).
- **Model**: opus. **Size**: S. **After**: slice 3.

##### VT4.2 — writes outside the activation are refused

- **Files**: `rust/tcl-engine-api/src/lib.rs`,
  `rust/tcl-engine-tclvm/src/lib.rs`, `rust/tcl-vm/src/interp.rs`
  (`set_var`, `store_var_result` and the name-resolving stores),
  `rust/tcl-spec-hooks/src/host.rs`, `rust/tcl-spec-hooks/src/sandbox.rs`,
  `rust/tcl-spec-hooks/tests/containment_e2e.rs`.
- **Items**:
  ```rust
  trait Engine {
      /// Refuse, as a Tcl error, every store whose name resolves outside
      /// the running procedure's own frame: a `::`-qualified name, a
      /// namespace variable, a linked variable. Reads are unaffected.
      fn confine_stores(&mut self) -> Result<(), EngineError> {
          Err(EngineError::Unsupported("confining stores to the activation"))
      }
  }
  ```
  The VM keeps one `confined_stores: bool`; every store that resolves a
  name rather than a local slot checks it. The host calls it once per
  engine, after `restrict_commands`. `SANDBOX_COMMANDS` is unchanged:
  `set`, `incr`, `lappend` and `lassign` keep their exact semantics on
  the activation's locals, and `upvar`, `global`, `variable`,
  `namespace`, `trace`, `uplevel` and `info` stay off it, so a qualified
  name is the only way out and it is closed. The page's host-command
  `ActivationStore` is replaced by this (D10).
- **Tests**: `a_body_with_a_global_counter_answers_identically_on_every_call`
  (the body `fold [incr ::counter]` raises, the evaluator declines, and
  the first and the thousandth answers are the same decline; a body that
  reads `::counter` reads nothing ever written); `a_local_accumulator_is_unaffected`
  (`set acc {}; foreach x {a b} {lappend acc $x}; fold $acc` answers
  `a b`).
- **Gates**: G7, G8, G9 (`tcl-vm`, `tcl-engine-api`, `tcl-engine-tclvm`,
  `tcl-spec-hooks`).
- **Model**: opus. **Size**: M. **After**: VT4.1.

##### VT4.3 — the capability declaration

- **Files**: `rust/tcl-registry/src/value_transfer/route.rs`,
  `rust/tcl-registry/src/value_transfer/mod.rs`.
- **Items**, the page's shape with the tree's constraints (`EvalRoute:
  Copy`, no engine-api dependency):
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
  pub struct EvaluatorCapability {
      pub identity: ImplementationIdentity,
      pub host: HostKind,
      pub target: Needs,
      pub inputs: &'static [DeclaredInput],
      pub depends: &'static [ContextDependency],
      pub budget: ImplementationBudget,
      pub completion: CompletionSupport,
  }
  pub struct ImplementationIdentity { pub pack: &'static str, pub id: &'static str, pub content_hash: u64 }
  pub enum HostKind { BoundedTcl }
  pub enum DeclaredInput { Operand { index: usize, exactness: Exactness }, IncomingTarget { index: usize }, OptionValue { name: &'static str } }
  pub enum ContextDependency { TclProfile, ImplementationIdentity, RegistryGeneration, EvaluatorGeneration, Binding(BindingIdentity) }
  /// The registry-side mirror of `tcl_engine_api::Budget`; the host
  /// converts it and caps it by its own.
  pub struct ImplementationBudget { pub commands: Option<u64>, pub wall_clock_ms: Option<u64>, pub value_bytes: Option<u64> }
  pub enum CompletionSupport { NormalOnly }
  ```
  The loader leaks the slices as it leaks every other pack field.
- **Tests**: `the_capability_is_part_of_the_route_identity`
  (`value_transfers.rs`).
- **Gates**: G1, G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: —.

##### VT4.4 — the `semantics`, `evaluate` and `facts` statements

- **Files**: `rust/tcl-spectcl/src/loader.rs` (`hook_source`,
  `HookSource`), `rust/tcl-spectcl/src/loader/` (the statement parsers),
  `rust/tcl-spectcl/src/catalogue.rs`, `rust/tcl-spectcl/tests/`.
- **Items**: the statements of the evaluation page's § *The `semantics`,
  `evaluate`, and `facts` rows*, at `command`, `subcommand` and `refine`
  scope, innermost winning: `semantics -native ID`, `semantics { … }`
  (`effects`, `result -semantic`, `stores -targets {…} -outcome O`,
  `iterate {…}`), `semantics none`; `evaluate -direct ID`,
  `evaluate -expression ID`, `evaluate -implementation ID -host
  bounded_tcl { inputs … depends … budget … body {params} {…} }`,
  `evaluate -native ID`, `evaluate none`; `facts -native ID`,
  `facts { … }`, `facts none`; the option flags `-evaluate none` and
  `-evaluate-reason WORD`. Each maps to `SemanticsDeclaration` (the three
  states) and, for `-implementation`, to a registry-owned
  `DeclaredImplementation` specialisation whose route is
  `EvalRoute::Implementation(capability)`.
- **Tests**: `semantics_statements_load_at_every_scope`,
  `a_short_native_id_is_a_load_notice` (tcl-spectcl).
- **Gates**: G2, G7, G8, G9 (`tcl-spectcl`).
- **Model**: opus. **Size**: L. **After**: VT4.3; the consumer-contracts
  lane's step 2 where it has landed (B-CC3).

##### VT4.5 — the body verbs

- **Files**: `rust/tcl-spec-hooks/src/emit.rs` (`answer_of`,
  `verbs_for`), `rust/tcl-registry/src/pack_hooks.rs` (`HookFamily`).
- **Items**: three `Emission` variants — `Fold(Value)`,
  `Write { target: usize, value: Value }`, `Preserve { target: usize }` —
  and their arms; the three protocol rules: silence is a decline, a
  `write` to a non-target raises, a declared target with no verb declines
  the whole answer. The `evaluate` family joins `HOOK_FAMILIES`.
- **Tests**: `a_silent_target_declines_the_whole_answer`,
  `a_write_to_a_non_target_raises` (tcl-spec-hooks).
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: VT4.4.

##### VT4.6 — the declared implementation runs through the driver

- **Files**: `rust/tcl-registry/src/pack_hooks.rs`,
  `rust/tcl-registry/src/value_transfer/declared.rs` (new),
  `rust/tcl-compiler/src/value_transfer.rs`.
- **Items**: `DeclaredImplementation` (`CommandSemantics`) whose
  `evaluate` resolves every `DeclaredInput` to an exact value (an input
  that is not exact is `Pending` or `NotExact`, never a placeholder),
  invokes the body through the per-thread host with the host's budget
  narrowed by `ImplementationBudget`, and maps the verbs to an
  `InvocationOutcome` (`fold` the result, `write` / `preserve` the ordered
  stores); an error or a budget overrun is a decline (`Transient` for a
  host that is absent or quarantined, never cached as `Unsupported`). The
  eligibility rule of the evaluation page's § *Two policies* is the
  driver's: a declared route at the resolved form, a valid binding, every
  input exact, every target axis satisfiable.
- **Tests**: `a_declared_implementation_folds_through_the_driver`
  (compiler witnesses: `tenant::label acme` folds to `tenant:acme`; an
  unknown argument declines `NotExact`; a host-absent worker declines
  `Transient`).
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: VT4.2, VT4.5.

##### VT4.7 — the memo key: incoming targets, dependencies, generation

- **Files**: `rust/tcl-registry/src/pack_hooks.rs` (`ShapeKey`,
  `content_hash`, `clear_cache`, `install_host`, `clear_host`, the
  quarantine path), `rust/tcl-registry/src/value_transfer/context.rs`,
  `rust/tcl-compiler/src/value_transfer.rs` (`AnalysisContextKey::evaluator_revision`).
- **Items**:
  ```rust
  #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
  pub struct EvaluatorGeneration(pub u32);
  ```
  bumped at `install_host`, `clear_host` and quarantine, carried as the
  context's `evaluator_revision`; the cache entry holds the exact inputs,
  each incoming target's value and existence, the `TargetDigest` and the
  canonical dependency list, and a hit compares them (the hash is the
  bucket, not the proof). `CacheMode::of(inputs)` keeps deciding
  eligibility.
- **Tests**: `a_changed_incoming_target_misses_the_cache`,
  `a_hash_collision_is_not_a_hit`, `host_install_and_quarantine_bump_the_generation`
  (`rust/tcl-registry` lib tests).
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: VT4.6.

##### VT4.8 — the overlay reaches the unit; the generations reach the key

- **Files**: `rust/tcl-lsp-db/src/lib.rs` (`compilation_unit`,
  `function_lattice`, `ValueTransferContext`, `spec_pack_key`,
  `registry_with_overlay`), `rust/tcl-compiler/src/value_transfer.rs`
  (`AnalysisContextKey::for_module`),
  `rust/tcl-lsp-db/src/value_transfer_parity.rs`.
- **Items**: `compilation_unit` and `function_lattice` build under
  `registry_with_overlay` (today `db.registry(&dialect)`), and the
  context key carries `registry_generation` and `overlay_generation`
  from the registry and the overlay (0 today).
- **Changes**: a workspace pack's declarations reach the memoised
  lattice, and a pack edit invalidates every lattice of the file. Mandate:
  "overlay invalidation (`spec_pack_key` reaching `compilation_unit`)".
- **Tests**: `a_workspace_pack_evaluator_reaches_i230_on_the_memoised_path`
  (`if {[tenant::label x] eq "tenant:x"} {puts yes} else {puts no}` gives
  I230 through the database, and not after the pack's `evaluate` row is
  removed); `a_pack_edit_invalidates_the_lattice`. `memory_growth` and
  `interned_gc` stay green.
- **Gates**: G7, G8, G9 (`tcl-lsp-db`, `tcl-compiler`).
- **Model**: opus. **Size**: M. **After**: VT4.6; the diagnostic-policy
  lane's `file_analysis` commit (B-DP2).

##### VT4.9 — the request and iteration budgets

- **Files**: `rust/tcl-registry/src/value_transfer/context.rs`,
  `rust/tcl-compiler/src/value_transfer.rs`, `rust/tcl-compiler/src/sccp.rs`.
- **Items**: `Budget::request()` (200 ms of evaluation work as
  `WorkUnits`, 64 MiB retained) and `Budget::iteration(&mut self) ->
  Budget` (one tenth of the request's remaining work), with every
  evaluation's `Budget::evaluation()` charging through its iteration and
  request (`charge_work` propagates). The driver opens one request per
  `sccp` run and one iteration per worklist pass; the host's command count
  converts one-to-one (`Engine::commands_spent`).
- **Changes**: a function whose evaluations exhaust the request declines
  the rest with `Budget(Request)`, published as `Overdefined`. Mandate:
  the evaluation page's § *The three nested budgets* and its slice-2 row
  (D3).
- **Tests**: `an_exhausted_request_declines_the_rest` (compiler), with a
  test budget small enough to exhaust.
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: VT4.6.

##### VT4.10 — `-native ID` for every family

- **Files**: `rust/tcl-spectcl/src/loader.rs`
  (`native_hook_tables_cover_their_catalogues`,
  `value_tables_cover_their_catalogues`),
  `rust/tcl-registry/src/pack_hooks.rs`,
  `rust/tcl-registry/src/const_fold.rs`, the shipped folders' modules.
- **Items**: `NativeEvalTables` with one `&[(&'static str, FnPtr)]` per
  `HookFamily` (the eleven) plus `semantics`, `evaluate` and `facts`,
  keyed by `SCOPE::FIELD` (`string::range::const_fold`,
  `string::is::const_fold_versioned`, `format::const_fold_versioned`,
  `lindex::const_fold`, …); a short id is a load notice naming the full
  spelling; `-direct` and `-expression` resolve through the `evaluate`
  table.
- **Tests**: the two table tests gain a row per family and per new
  vocabulary.
- **Gates**: G2, G7, G8, G9.
- **Model**: sonnet. **Size**: M. **After**: VT4.4.

##### VT4.11 — the four surfaces

- **Files**: `rust/tcl-spec-studio/src/render_spectcl.rs` (the `GAPS` row
  `semantics` goes), `coverage.rs`, `draft.rs`, `schema.rs`, `help.rs`,
  `rust/tcl-spectcl/src/export.rs`, `docs/references/command-spec/fields.md`
  (regenerated).
- **Items**: the three statements render, export and round-trip verbatim;
  the studio's "Purity and folding" cluster gains the route picker and
  the body box; a subcommand's body survives a form edit.
- **Tests**: `spectcl_roundtrip.rs` (a rendered-then-reloaded draft
  differs only on `GAPS` keys); `reference_doc.rs`.
- **Gates**: G4, G7, G8, G9 (`tcl-spec-studio`, `tcl-spectcl`).
- **Model**: sonnet. **Size**: M. **After**: VT4.4.

##### VT4.12 — the three `spectcl_check` findings

- **Files**: `rust/tcl-mcp/src/spectcl.rs`.
- **Items**: an evaluator reads a target it did not declare; an
  evaluator is silent on a declared target; a `write` names a non-target —
  each a report over `CtxScan` and the declarations.
- **Tests**: one per finding in `tcl-mcp`'s `spectcl` tests.
- **Gates**: G7, G8, G9 (`tcl-mcp`).
- **Model**: sonnet. **Size**: S. **After**: VT4.5; the
  consumer-contracts lane's CC3.4 edit of the same file (B-CC4).

##### VT4.13 — the executable example and the completion test

- **Files**: `rust/tcl-compiler/tests/fixtures/value_transfers/tenant.tclspec`
  (new), `rust/tcl-compiler/tests/value_transfer_witnesses.rs`,
  `rust/tcl-cli/tests/value_transfers_cli.rs`.
- **Items**: the page's `tenant::label` (one `arity 1`, a `semantics`
  block, `evaluate -implementation tenant.label.v1 -host bounded_tcl`
  with its `inputs`, `depends`, `budget` and `body {name} { fold
  [string cat "tenant:" $name] }`, and a `facts` block the four surfaces
  carry and no solver reads yet); the same declaration under a second
  name (`tenant::tag`) and as a subcommand form (`tenant label NAME`,
  operands shifted by one).
- **Tests**: `the_completion_test_needs_no_consumer_edit` — all three
  spellings fold `acme` to `tenant:acme` through analysis, I230 in `tcl
  diag`, O100 in `tcl opt`, the renderer and the studio round trip, with
  no file outside the fixture edited; `shipped_builtins_stay_on_the_direct_route`
  (`value_transfers.rs`: the pinned route set of the shipped packs is
  unchanged).
- **Gates**: G3 (none new), G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: VT4.6 to VT4.11.

##### VT4.14 — the EDA collection commands

- **Files**: `specs/*.tclspec` declaring `append_to_collection`,
  `remove_from_collection` and `foreach_in_collection`,
  `rust/xtask/src/value_transfers.rs` (three `KNOWN_GAPS` rows go).
- **Items**: `append_to_collection` and `remove_from_collection` as
  `semantics { stores -targets {0} -outcome may_write }` with `evaluate
  none` (a vendor collection handle is opaque);
  `foreach_in_collection` as an `iterate` plan over a vendor collection
  (the Vendor iteration row).
- **Gates**: G1, G2, G9 (`tcl-spectcl`'s `spec_corpus`).
- **Model**: sonnet. **Size**: S. **After**: VT4.4.

##### VT4.15 — docs and the landing

- **Files**: `docs/design/registry/spec-packs.md`,
  `docs/design/spec-dsl-examples/README.md`,
  `docs/design/contracts/command-spec-studio.md`,
  `docs/design/compiler/command-registry.md`,
  `docs/design/compiler/value-evaluation.md` (§ *`Engine::set_release`*
  names the profile-name argument; § *Per-evaluation state* names
  `confine_stores`), `docs/design/compiler/value-transfers.md`,
  `pass-fact-ownership-matrix.md`, a KCS howto
  `docs/kcs/spectcl/kcs-howto-declare-an-evaluator-for-a-pack-command.md`
  (new) and its index, the lane doc.
- **Gates**: G4, G5, G6, and the slice's green.
- **Model**: sonnet. **Size**: M. **After**: VT4.13.

#### Checkpoints and landing

Realised checkpoints, corrected from the plan's anticipated four (each
item's own row in the opus/sonnet/VT4.13 records above has the full
mapping):

| Checkpoint | Holds | Green means |
|---|---|---|
| `7fb8efd6` (`wip(value-transfers): slice 4 — the engine pins and confines`, landed) | VT4.1, VT4.2 | the containment and release tests; every existing host test |
| `cfa8285b` (`wip(value-transfers): slice 4 — the declaration and the loader`, landed) | VT4.3, VT4.4, VT4.5, VT4.6 (D92 rides it) | the loader and table tests; the driver tests |
| `22ea7aca` (`wip(value-transfers): slice 4 — the route, the key and the overlay`, landed) | VT4.7, VT4.8, VT4.9 | the cache, overlay and budget tests |
| `8b552ff2` (`wip(value-transfers): slice 4 — a confined engine reads no host environment`, landed) | D101, found beyond the plan | `a_confined_engine_reads_no_host_environment` |
| `51d1f3f7`, `02b3e7c8`, `200f6209`, `04fda11d` (the sonnet items, each its own commit, landed) | VT4.10, VT4.11, VT4.12, VT4.14 | the table, round-trip, findings, and pinned-route-set tests; G2 |
| `52d3d5b4`, `a3891aa7`, `ece65295`, `c945a41e` (VT4.13 and its two follow-ups, landed) | D102, VT4.13, Q12 (D104), Q13 | the completion test on every surface; the evaluator-epoch and re-decline witnesses |
| `wip(value-transfers): slice 4 — docs` (landed) | VT4.15 | G4, G5, G6, and the slice's green |
| `wip(value-transfers): slice 4 — a private SpecTcl command` (the landing) | — | every exit test; G1 to G9 |

```text
wip(value-transfers): slice 4 — a private SpecTcl command

A pack command reaches the analyser through the same interface as a
shipped one. The loader reads `semantics`, `evaluate` and `facts` at
command, subcommand and refine scope, with the explicit abstention; a
declared implementation names its identity, host, inputs, dependencies,
budget and completion; its body answers with `fold`, `write` and
`preserve`, and silence declines the whole answer. The host pins each
engine to one release, refuses every store outside the running
activation, and reads no host environment, so a body with a global
counter or a read of `$::env(...)` declines identically on every call.
The driver runs the declared route under the eligibility rule, the hook
cache compares exact inputs, incoming targets and dependencies on a hit
and is keyed by the evaluator generation, the workspace overlay reaches
every per-procedure query, and the request and iteration budgets bound a
function's evaluations; a process-wide evaluator epoch, a salsa input the
server bumps on a plan publish or a quarantine, re-keys every memoised
lattice so a healthy worker never keeps serving what a since-recovered or
since-quarantined one last computed. `-native` resolves for every family
by `SCOPE::FIELD`; the renderer, the studio, export and the reference
round-trip the three statements, with a subcommand's body surviving a
form edit. The three `spectcl_check` findings report an evaluator that
reads an undeclared target, one silent on a declared target, and a write
outside the declared targets, before a user reaches them. The bundled SDC
pack's three collection commands carry the vendor-iteration and may-write
shapes as their own worked example. A rename and a subcommand form of the
example need no consumer edit, and shipped builtins stay on the direct
route.

Behaviour changes: a workspace pack's declared evaluator folds on the
memoised path; a pack edit, a plan publish, or a quarantine invalidates
the file's lattices; a store or a host-environment read outside the
activation raises inside a hook body; a short `-native` id is a load
notice; a function whose evaluations exhaust the request declines every
re-evaluated statement of that run, the ones earlier sweeps folded
included, not only those after the point of exhaustion.
```

No issue is pinned or closed by this slice's own witnesses: the
findings table (§ *Witnesses* › *The findings table*) assigns none of
#2050–#2144 to slice 4, and #2140 and #2139, named in this slice's
§ *Upstream starting point* only, are upstream context this slice's code
sits on, never a defect its own witnesses pin.

#### Review checklist

- R1: no consumer names `tenant::label` or any pack command; the example
  lives in a fixture; `CLEAN_FILES` gains `value_transfer/declared.rs`
  implicitly (a new file is clean).
- R2 to R5; new files: `declared.rs`, the fixture (no AGPL header: a
  fixture).
- R6: every existing host, loader and studio test is byte-identical
  except the two table tests' new rows.
- Suites: `cargo test -p tcl-engine-api -p tcl-engine-tclvm -p tcl-vm -p
  tcl-spec-hooks -p tcl-spectcl -p tcl-spec-studio -p tcl-registry -p
  tcl-compiler -p tcl-lsp-db -p tcl-mcp -p tcl-cli`.
- R7: a body's error, silence or budget overrun is a decline, never a
  value; an absent host is `Transient`, never cached; a cache hit proves
  equality of every input; a body never reads state a previous
  evaluation wrote.

#### Behavioural deltas

| Delta | Mandate |
|---|---|
| a workspace pack's declared evaluator folds on the memoised path | exit; "overlay invalidation" |
| a pack edit invalidates every lattice of the file | the evaluation page's invalidation table |
| a store outside the activation raises inside a hook body | "per-evaluation state isolation in the host"; the evaluation page's witness |
| a short `-native` id is a load notice | the evaluation page's `SCOPE::FIELD` rule |
| a function whose evaluations exhaust the request declines the rest | the three nested budgets (D3) |

#### Record (2026-09-23): the opus items of slice 4

One implementer runs the opus items in plan order — VT4.1 to VT4.9 — and
reports before VT4.13, which follows the sonnet items VT4.10 to VT4.12 and
VT4.14. The decisions are D72 onward in § *Decisions taken*; this table
gains a row per item as it lands.

A container restart stopped the first implementer with VT4.1 and VT4.2
written and nothing committed. The second recovered that worktree whole —
every hunk checked against the plan, nothing backed out — and finished it
before the first checkpoint: the VM's two error globals and the
rebootstrap under confinement (D78), five more escapes in the engine test
(`regsub` and four `dict` updates, each probed for the name it would have
written), the `shape_only` note that `dialect` is now in the cache key, and
the four tests the first pass had not reached, which pinned `f5-irules` as
the release-less witness: `evaluate_def_incr_reads_the_base_under_the_targets_release`
and `evaluate_def_assign_value_folds_string_range_under_the_release`
(`sccp.rs`), `simulated_incr_reads_the_counter_under_the_release`
(`static_loops.rs`) and `sccp_text_prints_the_route_of_each_cell_update`
(`tcl-explorer`). Each now takes 8.4's answer under `f5-irules` — tclsh
8.4 prints 9 for `set x 010; incr x`, `ijkl` for `string range
abcdefghijkl 010 end`, 20 for the leading-zero loop, and -8 past the wide
boundary, which no model computes — and pins the decline under a profile
declaring no release (`tcl`, or `tk` in the Explorer's text view).

| Item | Commit | What landed | Its tests |
|---|---|---|---|
| VT4.1 | `wip(value-transfers): slice 4 — the engine pins and confines` | `Engine::set_release` (default `Unsupported("pinning a release")`); `TclVmEngine::set_release` through the registry's dialect ingress (D76) with the thread's numeral grammar claimed per operation (`GrammarGuard`, D75); the host's per-(pack, profile) pinned engines for `HookProgram::release_pinned` programs, keyed into the hook cache by the call's profile (D74); ruling 8 in `TargetSemantics::of` (D72), and a release-less profile's math functions floored at 8.4's (D73); the expression evidence reads the same release | `set_release_pins_the_numeral_grammar` (`tcl-engine-tclvm`); `a_release_pinned_hook_runs_under_the_calls_release` (`families_e2e`); `a_declared_base_release_is_the_release`, `numerals_read_under_the_named_grammar_or_the_unanimous_one` and the character-model rows (`const_ops`); the iRules and `tcl` columns of the registry, compiler, parity and CLI witnesses; the four tests the recovery restated (above) |
| VT4.2 | the same checkpoint | `Engine::confine_stores` (default `Unsupported("confining stores to the activation")`); `Vm::set_stores_confined` and the check at the VM's two store entries (D77); a confined VM publishes no `::errorInfo` / `::errorCode`, and `set_host`'s rebootstrap runs unconfined (D78); `TclVmEngine::confine_stores`; the host confines every engine after `restrict_commands` and fails the pack's sandbox on an engine that cannot | `confine_stores_refuses_every_store_outside_the_activation` (`tcl-engine-tclvm`: nineteen escapes, each probed for the name it would have written, and a caught error publishing neither global); `a_body_with_a_global_counter_answers_identically_on_every_call`, `a_local_accumulator_is_unaffected` (`containment_e2e`) |
| VT4.3 | `wip(value-transfers): slice 4 — the declaration and the loader` | `EvaluatorCapability` at the plan's shape — `ImplementationIdentity`, `HostKind`, `DeclaredInput`, `Exactness`, `ContextDependency`, `ImplementationBudget`, `CompletionSupport` — with the closed word list beside each type, and `OptionEvaluation` (D79); the route label and the inventory spell the implementation by its id | `the_capability_is_part_of_the_route_identity` (`value_transfers.rs`) |
| VT4.4 | the same checkpoint | `loader/semantics.rs`: `semantics`, `evaluate` and `facts` at command, subcommand and `refine` scope, innermost winning and sealed per row (D84), as one `DeclaredSemantics` per declaring scope (D80) whose `arg N` counts from the form's first argument (D81); the option flags (D82, D90); vocabulary 2.2 (D83, D93); `-native` and `-direct` ids installing nothing yet (D87); a form's implementation dropped with a notice (D85) | `semantics_statements_load_at_every_scope`, `a_short_native_id_is_a_load_notice`, and `a_stores_row_an_iterate_block_and_the_option_flags_load`, `what_cannot_be_used_is_reported_and_dropped` (`loader/semantics.rs`) |
| VT4.5 | the same checkpoint | `HookFamily::Evaluate` in `HOOK_FAMILIES`, its programs release-pinned and its body bound into a slot by the host plan (`DeclaredSemantics::bound`, D86); `Emission::Write` and `Emission::Preserve`, the `write` and `preserve` verbs validating against `HookCall::targets`, and `answer_of`'s three rules; `HookAnswer::Evaluation` | `a_silent_target_declines_the_whole_answer`, `a_write_to_a_non_target_raises` (`families_e2e`) |
| VT4.6 | the same checkpoint (D92) | `DeclaredSemantics::evaluate` through `pack_hooks::slot_available` and `dispatch` (D88, D89); `PackHookHost::is_available`; `HookCall::budget` and the host's per-call narrowing (D91); the driver's implementation arms in `run_script` and `call_def`, counted by `enter_implementation`, and the expression route asking the option declines first (D90) | `a_declared_implementation_folds_through_the_driver` (compiler witnesses: `tenant::label acme` folds to `tenant:acme` and counts two implementation entries; `$x` declines `not-exact`; with the host cleared, `transient`); `a_declared_budget_narrows_the_host_for_its_call_only` (`containment_e2e`) |
| VT4.7 | `wip(value-transfers): slice 4 — the route, the key and the overlay` | `EvaluatorGeneration`, shared by the workers serving one published plan and fresh for a quarantine (D94), set at `install_host` / `install_plan_host`, `clear_host` and `note_quarantine`, read by `evaluator_generation` into `AnalysisContextKey::evaluator_revision`; the hook cache keeping each content-keyed answer's call content and comparing it on a hit, with `HookCall::depends` (D95) | `a_changed_incoming_target_misses_the_cache` (`value_transfer/declared.rs`), `a_hash_collision_is_not_a_hit`, `host_install_and_quarantine_bump_the_generation` (`pack_hooks.rs`) |
| VT4.8 | the same checkpoint | `CommandRegistry::generation` and `overlay_generation` (D96), which `AnalysisContextKey::for_module` reads from the unit's registry; `compilation_unit`, `proc_taint_solve` and every per-procedure query resolving against the workspace's pack overlay, `ProcBodyKey` included (D97); `document_compilation_unit_for` for the semantic-token queries | `a_workspace_pack_evaluator_reaches_i230_on_the_memoised_path` (two I230s, top level and procedure, and none once the pack's `evaluate` row goes), `a_pack_edit_invalidates_the_lattice` (`value_transfer_parity.rs`); `memory_growth` and `interned_gc` green |
| VT4.9 | the same checkpoint | `Budget::request`, `Budget::iteration`, `Budget::evaluation_within` and the propagating `charge_work` / `charge_result` (D99); the driver's request per run and iteration per sweep; a declared implementation's commands charged one-to-one | `an_exhausted_request_declines_the_rest` (`value_transfer.rs`: twelve folds under the default request; under a 500-unit request the declines are the request's and each publishes `Overdefined`, D100) |

Deltas observed beyond the plan's list, each with its oracle:

- **iRules, iApps, tmsh, `expect` and the EDA shells evaluate under their
  declared base** (ruling 8): `set z 010; incr z` is 9 under `f5-irules`
  (tclsh 8.4 prints 9), where it declined; `puts [list # a]` is `# a`
  there (tclsh 8.4), where it declined; `format` answers 8.4's column. The
  F5 dialects' character model stays unanimous (D72).
- **A release-less profile folds only 8.4's math functions** (D73): `expr
  {min(1,2)}` under the version-less `tcl` profile folded to 1; tclsh 8.4
  raises `unknown math function "min"`.
- **The CLI's release-less witness is `tk`**: `tcl` is not a value
  `--dialect` accepts, so `explore_sccp_prints_the_route_of_each_statement`
  pins the decline under `tk`.
- **The hook cache is keyed by the call's profile** for every family, so
  two profiles of one release no longer share an entry (D74).

Green at the first checkpoint (VT4.1, VT4.2):

- tests: `cargo test -p tcl-engine-api -p tcl-engine-tclvm -p
  tcl-spec-hooks -p tcl-spectcl` 350 passed, 0 failed, 1 ignored; `cargo
  test -p tcl-vm --no-fail-fast` 1473 passed under `LANG=C.UTF-8` — without
  a UTF-8 locale `encoding_command` and `ensemble_subcommand_words_resolve_like_tclsh`
  fail as they do on `HEAD`, since they read the system encoding; `cargo
  test -p tcl-registry -p tcl-compiler -p tcl-lsp-db --no-fail-fast`
  11025 passed, 11 ignored, and the three restated compiler tests then
  passed; `tcl-explorer` 102 passed; `tcl-cli`'s `value_transfers_cli` 5
  passed and `samples_optimiser_profiles_are_regenerated` passed (no
  sample moved);
- pedantic clippy (`--no-deps --all-targets -D warnings`) on every touched
  crate, `tcl-cli` included, with no `#[allow]` added; `cargo fmt --all
  --check` clean;
- `cargo xtask value-transfers --check` OK and unchanged (17 clean, 13
  waived, 98 pinned across 39 files, 6607 rows); `pack-goldens` 0 of 24
  rewritten; the shard verifier OK; `kcs-index-links`; `owner-resolution`
  (44 rows); `cargo check --workspace --all-targets` clean.

Deltas at the second checkpoint (VT4.3 to VT4.6), beyond the plan's list:

- **VT4.6 is in it and VT4.10 is not** (D92): VT4.10 is a sonnet item, so
  until it lands a `-native` or `-direct` id installs nothing (D87).
- **`catalogue.rs` is untouched**: the closed word lists live beside their
  types (D79), where VT4.10's table tests read them; the studio's pickers
  for them arrive with VT4.11's route picker.
- **The host narrows per call** (`tcl-spec-hooks/src/host.rs`, beyond
  VT4.6's file list): the plan's "the host's budget narrowed by
  `ImplementationBudget`" is the host's to do (D91), and
  `HookHost::is_available` answers the registry's availability question.
- **The test pins of the newest vocabulary moved to 2.2** (D93).
- **`semantics_statements_load_at_every_scope` split in two** for clippy's
  function-length lint; the stores, iterate and option-flag half is
  `a_stores_row_an_iterate_block_and_the_option_flags_load`.

Green at the second checkpoint (VT4.3 to VT4.6):

- tests: `cargo test -p tcl-registry -p tcl-spec-hooks -p tcl-spectcl -p
  tcl-spec-studio --no-fail-fast` 1815 passed, 0 failed, 1 ignored; `cargo
  test -p tcl-compiler --no-fail-fast` 9716 passed, 6 ignored; `tcl-cli`'s
  `value_transfers_cli` 5 and `spec_verbs` 18 passed, and
  `samples_optimiser_profiles_are_regenerated` passed (no sample moved);
  `tcl-mcp`'s `spectcl` tests 24 passed;
- pedantic clippy (`--no-deps --all-targets -D warnings`) on
  `tcl-registry`, `tcl-spec-hooks`, `tcl-spectcl`, `tcl-compiler`,
  `tcl-spec-studio` and `xtask`, with no `#[allow]` added; `cargo fmt
  --check` clean on those crates and on the one `tcl-mcp` line;
- `cargo xtask value-transfers` rewrote nothing and `--check` is OK (17
  clean, 13 waived, 98 pinned across 39 files, 6607 rows);
  `pack-goldens` 0 of 24 rewritten; the shard script and the manifest
  proof (325 targets, 5 partitions) OK; `cargo check --workspace
  --all-targets` clean.

Deltas at the third checkpoint (VT4.7 to VT4.9), beyond the plan's list:

- **The generation is shared by plan** (D94): the plan's "bumped at
  `install_host`, `clear_host` and quarantine", with the value a plan's
  rather than a counter's, so the server's workers share a lattice; a
  quarantine still gives its worker a generation of its own.
- **The hook cache compares content, `HookCall::depends` included**
  (D95), and the profile stands in for the `TargetDigest`.
- **The overlay reaches every per-procedure query and `ProcBodyKey`**
  (D97), beyond the plan's `compilation_unit` and `function_lattice`, and
  a registry now carries its own generation (D96); `tcl-lsp-db` gains
  `tcl-spectcl` as a dev-dependency for its witnesses, and the semantic
  tokens read the overlaid unit.
- **Salsa does not see the evaluator generation** (D98, Q12), and **an
  exhausted request re-declines what it paid for** (D100, Q13).
- **Disk**: the shared target directory fell to 1.4 GB free during the
  suites; the test executables this lane had already run went (231 files,
  7 GB), and the run went on at 8.1 GB.

Green at the third checkpoint (VT4.7 to VT4.9):

- tests: `cargo test -p tcl-registry -p tcl-spec-hooks -p tcl-spectcl -p
  tcl-compiler --no-fail-fast` 11253 passed, 0 failed, 7 ignored; `cargo
  test -p tcl-lsp-db --no-fail-fast` 125 passed, 5 ignored, and the two
  memoised-against-direct corpus sweeps run with `--ignored`
  (`compiler_check_memo_matches_uncached_over_corpus`,
  `file_analysis_incremental_matches_full_over_corpus`) passed;
  `tcl-spec-studio` and `tcl-explorer` 384 passed; `tcl-cli`'s
  `value_transfers_cli` 5 and `spec_verbs` 18 passed, and
  `samples_optimiser_profiles_are_regenerated` passed (no sample moved);
- pedantic clippy (`--no-deps --all-targets -D warnings`) on
  `tcl-registry`, `tcl-spec-hooks`, `tcl-spectcl`, `tcl-compiler` and
  `tcl-lsp-db`, with no `#[allow]` added; `cargo fmt --check` clean on
  them;
- `cargo xtask value-transfers` rewrote nothing and `--check` is OK (17
  clean, 13 waived, 98 pinned across 39 files, 6607 rows);
  `pack-goldens` 0 of 24 rewritten; no new test binary; `cargo check
  --workspace --all-targets` clean.

After the third checkpoint, `wip(value-transfers): slice 4 — a confined
engine reads no host environment` closes the route contract's
host-environment clause (D101):
`a_confined_engine_reads_no_host_environment` (`tcl-engine-tclvm`);
`cargo test -p tcl-vm -p tcl-engine-api -p tcl-spec-hooks -p tcl-spectcl
--no-fail-fast` under `LANG=C.UTF-8` 1817 passed, 1 ignored, and
`tcl-engine-tclvm` 15 passed; pedantic clippy and `cargo fmt --check`
clean on the three crates it touches.

The state the sonnet items start from — the tree of this record's last
commit:

- **VT4.10.** The id rule is `loader/semantics.rs`'s `native_id`: a short
  id is a notice naming `SCOPE::FIELD`, and a full id is a notice that
  nothing this build ships holds it; either way the statement installs
  nothing (D87). The tables replace that second notice with a lookup. The
  closed word lists the table tests read sit beside their types (D79):
  `NativeEvalId::ALL`, `LanguageProfileId::ALL`, `HostKind::ALL`,
  `Exactness::ALL`, `ContextDependency::WORDS`, `OutcomeKind::ALL`,
  `DeclaredEffect::ALL`, `OptionEvaluation::REASONS`. `HOOK_FAMILIES`
  holds twelve families: the eleven VT4.10 counts, and
  `HookFamily::Evaluate` last (index 11), whose native table is the
  `evaluate` one.
- **VT4.11.** A scope's `semantics` and `evaluate` statements load into
  one `DeclaredSemantics` on the scope's `semantics` field (D80); `facts`
  is checked and not stored, so the four surfaces carry it from the
  source rows; the option flags are `DeclaredSemantics::option_declines`
  as `(option, DeclineReason)` (D90), `OptionEvaluation::decline` turning
  a flag into its decline. `tcl-spec-studio/src/store.rs`'s `family_key`
  has the `Evaluate` arm (`"evaluate"`); `render_spectcl.rs`'s `GAPS` row
  `semantics` is still there, and `DSL_VERSION` already tracks 2.2.
- **VT4.12.** `tcl-mcp/src/spectcl.rs`'s `family_key` has the
  `Evaluate` arm (D86). At run time the host already raises on a `write`
  to a non-target and declines a body silent on a declared target
  (`answer_of`); the findings are the static half.
- **VT4.14.** The `iterate` block (`binder`, `iterable`, `body`,
  `yield`, `cardinality`, `completion`, `zero_iterations` rows), `stores
  -targets {…} -outcome may_write` and `evaluate none` load today; they
  are 2.2 words, so a pack below `speclib … 2.2` draws the newer-word
  notice for each (`log.since`).
- **VT4.15.** The pages still describe the page's mechanisms where the
  build differs: § *`Engine::set_release`* (the profile-name argument,
  D76), § *Per-evaluation state* (`confine_stores`, D77 and D78), the
  budgets (D99, D100) and the memo (D94 to D98); vocabulary 2.2 is new
  to `spec-dsl-examples/README.md`, and an option input's list shape (D89)
  and the declined-implementation reasons (D88) are the KCS howto's to
  state. `value-evaluation.md` § *Target semantics* already states ruling
  8 as built.
- **VT4.13** (this lane's, after the sonnet items): the driver, the host
  and the memoised path all run a declared implementation today — the
  compiler witness through a stand-in host, and `value_transfer_parity.rs`
  through the real one from a loaded pack.

#### Record (2026-09-23): the sonnet items of slice 4

Runs after the opus items above; each item its own commit, in plan order
(VT4.10, VT4.11, VT4.12, VT4.14), reporting before VT4.13.

| Item | Commit | What landed | Its tests |
|---|---|---|---|
| VT4.10 | `wip(value-transfers): slice 4 — -native ID for every family` | `tcl_registry::pack_hooks` gained fourteen `SCOPE::FIELD`-keyed native tables — one per pre-existing `HookFamily` variant (the eleven) plus `SEMANTICS_NATIVE`, `EVALUATE_NATIVE`, `FACTS_NATIVE` — real for `CONST_FOLD_NATIVE` (20 rows) and `CONST_FOLD_VERSIONED_NATIVE` (2 rows), the shipped folders' worked example, empty for the other twelve (nothing else ships a named native implementation yet); the shipped fold functions the tables reference (`string_.rs`, `format_.rs`, `regsub_.rs`, `scan_.rs`) exposed `pub(crate)` via `commands/tcl/mod.rs` re-exports; `loader/semantics.rs`'s `native_id` made generic over a table, so a full id the table holds now installs its value and one it does not keeps the existing "names nothing this build ships" notice; `evaluate -direct ID` split out from `-native` into its own resolution against `NativeEvalId::ALL` (`enum_by_name`, Rust-spelled, since it predates `-native`), leaving `-expression` untouched (already `LanguageProfileId::ALL`-matched); `tcl_spectcl::catalogue` gained the fourteen native-id pickers (id-spelled) and eight `value_transfer` vocabulary pickers (`NativeEvalId` Rust-spelled; `LanguageProfileId`, `HostKind`, `Exactness`, `ContextDependency::WORDS`, `OutcomeKind`, `DeclaredEffect`, `OptionEvaluation::REASONS` all DSL/`as_str()`-spelled) | `native_hook_tables_cover_their_catalogues` (+14 rows), `value_tables_cover_their_catalogues` (its original 9 rows, unchanged) and the new `value_transfer_tables_cover_their_catalogues` (+8 rows, split out for clippy's function-length lint), `catalogue_keys_are_unique` (+22 catalogues); `a_short_native_id_is_a_load_notice` adapted (`evaluate -direct go::evaluate` → `evaluate -native go::evaluate` at the same subcommand scope, plus a new `evaluate -direct nonexistent` line at command scope, since `-direct` no longer shares `-native`'s notice text — R6's witness is this item's own mandate to split them); `a_native_id_a_table_holds_installs_its_value` (new: `native_id` against a synthetic non-empty table, proving the "found" branch, since every real table but const-fold is empty) |
| VT4.11 | `wip(value-transfers): slice 4 — the four surfaces` | `tcl-spec-studio/src/draft.rs`'s `semantics_value` replaces `lost.expr("semantics", …)` at command and subcommand scope: a `Declared` plan whose `as_declared()` gives a `DeclaredSemantics` renders in full (`effects`, `result`, `stores`, `iterate`, and the `None`/`Direct`/`Expression` evaluation kinds — plain data, one level deeper than `object_class`'s own precedent) unless its evaluation is `Implementation` (the body lives only in the loader's pack-hook table, never on `CommandSpec`) or it carries an option-level `-evaluate` decline (not yet carried back onto its option row) — both stay `Value::Null` and unrecoverable, exactly like a shipped, compiled-in specialisation (`as_declared() == None`) already was; `render_spectcl.rs`'s new `semantics_block` renders the recovered plan as `semantics { … }` / `evaluate …` (wired into `command_body` and `subcommand_block` beside `const_fold`/`const_fold_versioned`) or, unrecovered, the existing `-native SCOPE::FIELD` placeholder `native_hook` already gives every other opaque hook field; the `GAPS` row `semantics` is gone. `schema.rs` gained two `NestedFieldSchema` rows under `semantics` — `route` (the picker) and `body` (the box) — `relations.rs`'s "Effects and purity" cluster gained `semantics`, `route` and `body` alongside `const_fold`; `help.rs` and `examples/fields_behaviour.rs` gained matching entries, and `docs/references/command-spec/fields.md` is regenerated (`UPDATE_REFERENCE=1`). `store.rs`'s `carry_forward` now reaches one scope down: a new `find_subcommand` and a `reclaim` helper factored out of the old command-level-only loop run inside each `subcommand NAME { … }` matched by name, so a hook body hanging off a subcommand is spliced back in — closing the module doc's own documented "top level only" limitation — while a renamed/removed subcommand, or a sub-subcommand's own hook, still has nowhere to carry a body into and is still reported through `Write::dropped`. `tcl-spectcl/src/export.rs` needed no change at all: its registration record is a verbatim, property-agnostic replay of every statement the loader read, so `semantics`/`evaluate`/`facts` — including a declared implementation's body — already round-trip through it exactly as `const_fold`'s body does, proven by a new dedicated test rather than by any new code | `a_declared_semantics_plan_survives_the_round_trip` (new, `spectcl_roundtrip.rs`: a full structure-plus-`-expression` plan renders, reloads, and diffs byte-for-byte, no `GAPS` tolerance needed); `a_declared_implementations_body_stays_unrecoverable` (new, `draft.rs`: both the body and option-decline cases stay `Value::Null` and marked); `a_subcommands_hook_body_survives_a_form_edit` (`store.rs`, replacing `a_loss_carry_forward_cannot_reach_is_reported`, which pinned the bug this item fixes — R6's witness is this item's own mandate); `the_value_transfer_statements_round_trip_through_gate_a` (new, `tcl-spectcl/tests/export.rs`); `fields_doc_matches_the_schema` (`reference_doc.rs`, regenerated); `every_group_and_field_has_a_valid_example` and `every_field_is_clustered_or_declared_standalone` (existing gates, now covering `route` and `body` too); every existing `tcl-spec-studio` and `tcl-spectcl` suite green, `a_form_edit_on_a_real_pack_splices_and_preserves_its_neighbours` adapted (below) |
| VT4.12 | `wip(value-transfers): slice 4 — the three spectcl_check findings` | `tcl-mcp/src/spectcl.rs` gained `evaluate_findings`, surfaced per-hook (`hook_json`'s new `"evaluate_findings"` array) and counted (`spectcl_check`'s new `"summary"."evaluate_findings"`), plus a new `declared_semantics_for` that resolves a hook's owner (`HookOwner::Command`/`Subcommand`) back to `CommandSpec.semantics`/`SubCommand.semantics` and downcasts through `SemanticsDeclaration::Declared(..).as_declared()` — `None` for `Inherited`/`Declined`/a shipped compiled-in specialisation/an option owner (no `-evaluate` decline surface yet), same as VT4.11's draft-side check. The three findings run only for `HookFamily::Evaluate` hooks with a `HookSource::Body`, against `declared.structure.stores.targets`: **(1) an evaluator reads a target it did not declare** — `DeclaredEvaluation::Implementation(..).capability.inputs`'s `IncomingTarget{index}` entries checked against `stores.targets` directly (a declaration-vs-declaration fact, not a body scan — see deviation below); **(2) silent on a declared target** and **(3) a write names a non-target** — both read from a new `EmissionScan`/`evaluate_emissions`, a textual, pessimistic scan (unattributed occurrences never resolved either way) over the body's `fold`/`write`/`preserve` verb calls, reusing the pre-existing `is_name_byte` word-boundary helper | `an_evaluator_reading_an_undeclared_target_is_flagged`, `an_evaluator_silent_on_a_declared_target_is_flagged`, `a_write_naming_a_non_target_is_flagged` (one per finding, each asserting the single expected message and `summary.evaluate_findings == 1`), `a_consistent_declared_implementation_has_no_evaluate_findings` (new: a `tenant::label`-style evaluator with no `semantics` block and no `target … incoming` input at all (both loops vacuously empty — the ordinary case), a `probe::split3` with three correctly-paired `stores -targets` rows and a body that `write`s or `preserve`s each in turn, and an unrelated `const_fold` hook, all read `evaluate_findings == []`, `summary.evaluate_findings == 0`) |
| VT4.14 | `wip(value-transfers): slice 4 — the EDA collection commands` | `specs/sdc_base.tclspec`'s header moves `speclib sdc_base 1.1` → `2.2` (the three statements below are 2.2 words; a lower header would load them but draw a `log.since` notice per site, per this slice's own "state the sonnet items start from" note) with no other statement in the file affected — the version gate is purely forward, never reinterpreting an already-accepted spelling. `append_to_collection` and `remove_from_collection` (each already `arg 0 -role VarWrite`, nothing else) each gain `semantics { stores -targets {0} -outcome may_write } evaluate none`: the collection variable is the one target, `may_write` because `-unique`/an absent element can make the call a no-op, and `evaluate none` because a vendor collection handle is opaque — nothing computable to fold. `foreach_in_collection` (`arity 3`, `arg 0 -role VarWrite`, `arg 2 -role Body`, the pre-existing `analyser_hook -native Foreach` left untouched) gains `semantics { iterate { … } } evaluate none`, its `binder`/`iterable`/`body` rows naming operands 0/1/2 and its `-kind`/`-grammar`/`yield`/`cardinality`/`completion`/`zero_iterations` rows copied field-for-field from the Vendor iteration row's own worked example (`loader/semantics.rs`'s `collection::each` fixture: `binder -arg 0 -grammar vendor.single_variable`, `iterable -arg 1 -kind vendor.collection`, `body -arg 2 -scope enclosing`, `yield -semantic vendor.object_handle`, `cardinality -from vendor.collection_summary`, `completion -contract vendor.collection_loop_completion`, `zero_iterations -bindings preserve`) — the plan names this exact row, and the fixture is its only worked instance anywhere in the tree. `rust/xtask/src/value_transfers.rs` loses the three commands' `KNOWN_GAPS` rows and the now-empty "Slice 4" comment above them: each command now has declared semantics, so `has_semantics` is true and none is `unclassified_write` any longer. The three commands now report a *different*, un-tracked gap instead — `docs/generated/value-transfers.md` regenerates their rows from `none` / `—` / "writes a variable, no semantics" (Classified: the old slice-4 text) to `declared (command) · NAME` / `none (declared)` / "descriptor without a route" (Classified: `—`) — `"descriptor without a route"` is `writes && has_semantics && !enabled`, a state `classification_problems` never gates on, since a pack that explicitly declines a route (`evaluate none`) is finished, not gapped. `cargo xtask pack-goldens` regenerates `rust/tcl-spectcl/tests/golden/sdc_base.snap` to match: `dsl` `1.1` → `2.2`, the three commands' `spec` hashes change, every later command's `line` shifts by the inserted line count, and — confirming nothing else moved — every one of `sdc_base`'s 86 commands keeps its `hooks`/`grammar` hashes byte-identical | `every_shipped_tclspec_loads_installs_and_analyses_against_corpus` (G9, `spec_corpus.rs`: zero unexpected notices — `spec_corpus_baseline.txt`'s own header claims all of `specs/`'s commands load notice-free, and the `2.2` bump keeps that true rather than drawing three `log.since` sites); `cargo xtask value-transfers --check` (G1) and `cargo xtask pack-goldens` (G2) themselves, run to regenerate then re-verified stable; `route_stamps_match_the_pinned_set` (`rust/tcl-registry/tests/value_transfers.rs`, adapted — below) |

Deviation from the plan's text, adapting to the tree: the plan's Items line
groups "the eleven" `HookFamily` variants under one `NativeEvalTables`
umbrella without saying whether their own `-native` dispatch (the separate,
older `hook_source`-in-`apply_command_stmt` path the ten non-const-fold
families and even `const_fold` itself use, distinct from
`loader/semantics.rs`'s `native_id`) is rewired to consult the new tables at
load time. It is not, in this item: three existing tests reach that path
with ids no real native table would resolve on purpose —
`native_resolver_capabilities_survive_the_round_trip`
(`tcl-spec-studio/tests/spectcl_roundtrip.rs`, asserting `notices.is_empty()`
over the studio's rendered `arg_role_resolver -native <id>` placeholder for
the shipped `binary scan` resolver), `a_native_hook_is_named_after_the_field_it_fills`
(`render_spectcl.rs`, the renderer's own synthesised `-native probe::FIELD`
placeholders) and `docs/design/spec-dsl-examples/string.tclspec`'s
`const_fold_versioned -native string::is` (short — the correct id is
`string::is::const_fold_versioned` — read by four more tests across
`tcl-spectcl` and `tcl-spec-studio`) — so wiring dispatch for the ten empty
families would do nothing today, and wiring it for `const_fold` would need
`apply_command_stmt`'s command- and subcommand-scope match arms reworked
(a second, larger change spanning the studio's own renderer expectations,
which is VT4.11's file list, not this item's) and `string.tclspec` fixed to
the correct spelling in the same change. `loader/semantics.rs`'s own
"state the sonnet items start from" note names only its `native_id` as what
this item replaces with a lookup, which is the scope taken: the tables exist,
fully populated where the plan gives an unambiguous worked example, and are
load-bearing for `semantics` / `evaluate` (`-native`, `-direct`) / `facts`;
the pre-existing eleven families' own `-native` statements still install
nothing today, exactly as before this item, and gain no new notice — this is
unchanged, not regressed, behaviour, pinned by the suites below. A later item
may rewire `apply_command_stmt` onto these tables; nothing here blocks it.

Green: `cargo test -p tcl-registry -p tcl-spectcl --no-fail-fast` 1493
passed, 0 failed, 1 ignored (the pre-existing fuzz-shaped
`every_prefix_of_a_valid_pack_loads`); pedantic clippy
(`--no-deps --all-targets -D warnings`) clean on both crates — one
`clippy::type_complexity` fixed by naming `TaintSinkGateFn` rather than
writing `fn(&[&str]) -> bool` inline, no `#[allow]` added; `cargo fmt -p
tcl-registry -p tcl-spectcl` applied (line-wrap only); `cargo xtask
value-transfers --check` OK and unchanged (17 clean, 13 waived, 98 pinned
across 39 files, 6607 rows); `cargo xtask pack-goldens` 0 of 24 rewritten;
no new integration-test binary, so no shard-script row; `cargo check
--workspace --all-targets` clean.

Deviation from the plan's text for VT4.11, adapting to the tree, found by
`pack_store.rs`'s `a_form_edit_on_a_real_pack_splices_and_preserves_its_neighbours`
(the eleven ported examples' own carry-forward gate): reaching subcommand
level makes carry-forward reclaim `docs/design/spec-dsl-examples/oo-class.tclspec`'s
per-subcommand `world_effects class-factory-effects` / `state_transitions { …
resolver -native … }` (both `DraftOpaque`, so a fresh render says nothing
about them) verbatim from the author's own bytes — and that reclaimed text
names a pack-level `descriptor world_effects class-factory-effects` block
declared outside the command, which `PackStore::accepts`'s isolated
single-block verification cannot see. `accepts` correctly refuses that
splice and `set_command` falls back to the re-render floor for that one
edit — exactly the "a pack-level construct the splice disturbed" case the
module's own docs already name as the floor's reason to exist. Nothing is
lost either way: the re-render floor still reports the same fields through
`Write::dropped` a splice would have. The test's "every example takes the
splice path" assertion is adapted to "every example but `oo-class.tclspec`",
with the reason recorded beside it; no other example's carry-forward
changed shape.

Green: `cargo test -p tcl-spec-studio -p tcl-spectcl --no-fail-fast` 584
passed, 0 failed, 1 ignored (the pre-existing fuzz-shaped
`every_prefix_of_a_valid_pack_loads`); pedantic clippy
(`--no-deps --all-targets -D warnings`) clean on both crates — one
`clippy::match_same_arms` and one `clippy::items_after_statements` fixed
by consolidation, no `#[allow]` added; `cargo fmt -p tcl-spec-studio -p
tcl-spectcl` applied (line-wrap only); `UPDATE_REFERENCE=1 cargo test -p
tcl-spec-studio --test reference_doc` regenerated `fields.md` (14 lines),
then `fields_doc_matches_the_schema` green unmodified; `cargo xtask
value-transfers --check` OK and unchanged (17 clean, 13 waived, 98 pinned
across 39 files, 6607 rows); `cargo xtask pack-goldens` 0 of 24 rewritten;
no new integration-test binary, so no shard-script row; `cargo check
--workspace --all-targets` clean.

Deviation from the plan's text for VT4.12, adapting to the tree — a design
interpretation, not a guess, recorded per the task's own instruction: the
plan's Items line says all three findings are "each a report over `CtxScan`
and the declarations." `CtxScan` turns out to be exactly the wrong tool for
this family, not a stand-in this item merely renames. `CtxScan::of` returns
its empty `Default` for anything that is not `HookSource::Body`, and for a
body it scans literal `$ctx` occurrences (`ctx_reads`); but
`rust/tcl-spec-hooks/src/host.rs`'s calling convention for `Evaluate` (the
arm building an `-implementation` body's proc call, around lines 551–556)
passes only the declared inputs as positional proc arguments — no `$ctx`
dict is ever bound, confirmed against `rust/tcl-registry/src/value_transfer/declared.rs`'s
`evaluate_implementation`, which builds the body's `words` strictly from
`capability.inputs`. `CtxScan::of` on an evaluate body therefore always finds
zero `$ctx` occurrences and would silently no-op two of the three findings;
the third (target reads) cannot be a body scan at all under this convention,
since a body cannot read what it was never given. Three consequences:

- A new `EmissionScan`/`evaluate_emissions` carries `ctx_reads`'s own shape
  (textual, and pessimistic in the same direction: an unattributed
  occurrence never resolves either way) onto the verbs this family's engine
  actually runs a body against — `fold` / `write` / `preserve` — rather than
  `$ctx` reads, which do not occur for it.
- **Finding 1** ("an evaluator reads a target it did not declare") is read
  as a pure declaration-vs-declaration fact: an `IncomingTarget{index}`
  entry in `capability.inputs` whose `index` is absent from
  `structure.stores.targets`. Nothing else a body could be said to "read"
  exists under this calling convention, and this reading has no
  false-positive risk on a well-formed pack — the two lists either agree or
  one names an index the other omits. Two other readings were considered
  and ruled out first: a literal `$ctx` scan (impossible — confirmed above)
  and folding it into finding 3's body scan (would make it indistinguishable
  from "a write names a non-target", losing the distinction the plan's own
  three-finding shape asks for — an unused `IncomingTarget` that the body
  never writes anywhere is still worth flagging on its own, since it is
  usually a stale `-inputs` line rather than a stray write).
- **Findings 2 and 3** read the body through `EmissionScan` as the plan
  intends: 2 is "declares a target `stores` names but the body never
  `write`s or `preserve`s it" (rule 3 of the calling convention: silence
  declines the whole answer, not just that target); 3 is "a `write` or
  `preserve` names an index outside `stores.targets`" (raises at query
  time — a decline the pack can avoid by fixing it at load time instead).
  Both stay silent when a write/preserve target is unattributable (a
  computed index), since the scan can only ever be pessimistic toward the
  finding it can prove, exactly as `declaration_conflict`'s own `broader`/
  `unattributed` split already treats `CtxScan`.

Every check here is a report, never an enforcement — the driver already
raises on a write to a non-target and declines silence on a declared target
at query time (the three body-verb rules); `evaluate_findings` is what an
author sees before a user does, the same relationship `declaration_conflict`
already has to the cache's own runtime behaviour.

Green: `cargo test -p tcl-mcp --no-fail-fast` 108 passed, 0 failed, 0
ignored (104 before this item, +4 new); pedantic clippy
(`--no-deps --all-targets -D warnings`) clean, no `#[allow]` added; `cargo
fmt -p tcl-mcp` applied (line-wrap only — `git status --short rust/tcl-mcp`
names only `spectcl.rs`); `cargo xtask value-transfers --check` OK and
unchanged (17 clean, 13 waived, 98 pinned across 39 files, 6607 rows);
`cargo xtask pack-goldens` 0 of 24 rewritten; no new integration-test
binary, so no shard-script row; `cargo check --workspace --all-targets`
clean.

Deviation from the plan's text for VT4.14, adapting to the tree — a
mechanical follow-on, not a design judgment call: the plan's Files line
names only `specs/*.tclspec` and `value_transfers.rs`'s `KNOWN_GAPS`, but
`rust/tcl-registry/tests/value_transfers.rs`'s
`route_stamps_match_the_pinned_set` — its own doc comment: "a route cannot
appear, vanish, or move without this list changing beside it" — is an exact
pinned `BTreeSet` over every loadable dialect and shipped pack, keyed by
`(name, route, owner)` with no dialect column, so `sdc_base`'s six EDA
mounts collapse to one row per command regardless. Declaring `evaluate
none` makes a route exist for the first time on these three commands
(`none:declared`, where none stood for *no row at all* before), which the
pinned test correctly reported as three `extra` entries the moment the pack
change landed — caught by running `-p tcl-registry`'s suite, part of this
item's own "run the crate's tests" step rather than a distinct gate. Three
rows added in the file's existing alphabetical order
(`append_to_collection` after `append`, `foreach_in_collection` after
`foreach`, `remove_from_collection` after `lmap`), nothing else in the
pinned set touched.

Green: `cargo test -p tcl-registry --no-fail-fast` 1194 passed, 0 failed,
0 ignored (21 binaries); `cargo test -p tcl-spectcl --no-fail-fast` 300
passed, 0 failed, 1 ignored (the pre-existing fuzz-shaped
`every_prefix_of_a_valid_pack_loads`, 20 binaries) — both include
`every_shipped_tclspec_loads_installs_and_analyses_against_corpus` (G9)
and `route_stamps_match_the_pinned_set`; `cargo test -p xtask
value_transfers` 12 passed, 0 failed; pedantic clippy
(`--no-deps --all-targets -D warnings`) clean on `xtask` and
`tcl-registry`, nothing to fix, no `#[allow]` added; `cargo fmt -p xtask -p
tcl-registry` applied, no changes (already clean); `cargo xtask
value-transfers` (regenerate, since this item deliberately moves three
rows) then `--check`: OK, drift limited to exactly the three expected rows
in `docs/generated/value-transfers.md`, stable on the next run (17 clean,
13 waived, 98 pinned across 39 files, 6607 rows — the row count is
unchanged, since no command, subcommand or form was added or removed);
`cargo xtask pack-goldens` 1 of 24 rewritten first run
(`sdc_base.snap`, inspected line by line — only the three commands' `spec`
hashes and every later command's `line` number changed; `hooks` and
`grammar` hashes identical throughout), 0 of 24 on the next; `cargo check
--workspace --all-targets` clean.

#### Record (2026-09-23): VT4.13 and the two follow-ups

The opus implementer again, after the sonnet items: VT4.13 as its own
checkpoint, then the two follow-ups the coordinator named (Q12, Q13).

| Item | Commit | What landed | Its tests |
|---|---|---|---|
| — | `wip(value-transfers): slice 4 — a declared implementation reads its inputs first` | D102: `DeclaredSemantics::evaluate` reads the places and the inputs before the release, the admission and the host, found by the completion test under the lenient `tcl` profile | `an_unknown_input_declines_before_the_release_is_asked` (`value_transfer/declared.rs`) |
| VT4.13 | `wip(value-transfers): slice 4 — the executable example and the completion test` | `rust/tcl-compiler/tests/fixtures/value_transfers/tenant.tclspec`: the page's `tenant::label`, the same declaration as `tenant::tag` and as `tenant label NAME`; `tcl-spectcl` a dev-dependency of `tcl-compiler` (the cycle `tcl-registry` already has, D103); no file under any `src/` | `the_completion_test_needs_no_consumer_edit` (compiler witnesses: all three spellings fold `acme` to `tenant:acme` on the implementation route under 8.6, 9.0 and 9.1, decline `unsupported` under 8.4, 8.5, `f5-irules` and `tcl`, decline `not-exact` on an unknown argument, and fold nothing without the pack; the optimiser forwards the constant (O100); `tclsh` runs the body's `string cat` exactly where the analysis folds, and with the vendor runtime prepended the original and the optimised program print the same under 8.4 to 9.1); `the_completion_test_reaches_every_surface` (`value_transfers_cli.rs`: `tcl explore`, I230 in `tcl diag`, O100 in `tcl opt` from a scratch workspace whose discovered pack is the fixture; `tcl spec export`'s pack and a studio form edit of each declaration each fold again from their own workspace; the renderer's placeholder for the body it cannot draw; a workspace without the pack folds nothing); `shipped_builtins_stay_on_the_direct_route` (`value_transfers.rs`) |
| Q12 | `wip(value-transfers): slice 4 — the evaluator epoch reaches salsa` | D104: `pack_hooks::evaluator_epoch`, which a plan publish (`tcl_spectcl::hooks::publish`) and a quarantine (`note_quarantine`, on any thread) move; the singleton salsa input `tcl_lsp_db::EvaluatorEpoch`, created at 0 by every `TclDatabase` constructor, read by `build_unit_with_keys` and carried by `ValueTransferContext`; the server's `sync_evaluator_epoch` (compare-then-set) in `reload_spec_packs` when the set changed and at the start of the post-publish `refresh_cross_file_evidence` | `an_evaluator_epoch_re_keys_the_memoised_lattices` (`value_transfer_parity.rs`: after a quarantine the memo still folds; once the epoch is taken the lattice is recomputed and declines `transient`); `host_install_and_quarantine_bump_the_generation` (`pack_hooks.rs`: a quarantine moves the epoch); `the_pass_after_a_quarantine_takes_the_evaluator_epoch` (the server's own tests: the refresh after a pass takes a quarantine's epoch and reschedules nothing; fails without the sync); `a_pack_reload_after_an_edit_yields_the_new_answer_on_the_memoised_path` (`e2e/spec_packs.rs`: a condition folded through the pack, re-analysed after an edit from the memo, turns from always true to always false when the pack's body changes on disk) |
| Q13 | `wip(value-transfers): slice 4 — Q13 accepted as built` | The lane doc alone: Q13 accepted as sound, its precision cost stated (a function whose evaluations spend the request keeps none of its route folds); no code change | `an_exhausted_request_declines_the_rest` (`value_transfer.rs`, D100), unchanged |

Deltas beyond the plan's list:

- **The completion test is two tests** (D103): the compiler cannot reach
  the CLI binary, the renderer or the studio, so the compiler witness
  covers the analysis, the optimiser and the `tclsh` oracle, and the CLI
  witness the surfaces; each names the other.
- **The fixture folds from 8.6** (D103): the page's body uses `string
  cat`, which tclsh 8.4 and 8.5 reject, and the evaluator declines there
  rather than answer for a release the body cannot run in — the witness
  pins that against `tclsh` itself.
- **D102 came out of it**: the one change outside the fixture and the
  tests, committed on its own before the checkpoint so the checkpoint's
  diff is the fixture, the tests and the dev-dependency alone.
- **`route_stamps_match_the_pinned_set` shares its collection** with the
  new witness (`route_stamps`, `pinned_route_stamps`); its pinned set is
  unchanged.

Green at the VT4.13 checkpoint: `cargo test -p tcl-compiler` 9718 passed,
6 ignored, run in six batches whose executables were deleted as each
finished (the new dev-dependency rebuilds all 66 test binaries); `cargo
test -p tcl-registry` 1196 passed; `tcl-cli`'s `value_transfers_cli` 6
passed; `spectcl_roundtrip.rs`, `reference_doc.rs` and the three catalogue
table tests green; pedantic clippy on `tcl-registry`, `tcl-compiler` and
`tcl-cli`, no `#[allow]`; `cargo fmt --all --check` clean; `cargo xtask
value-transfers --check` unchanged (the fixture is not a shipped pack: 17
clean, 13 waived, 98 pinned across 39 files, 6607 rows); `pack-goldens` 0
of 24; no new test binary (manifest proof 325 targets); `cargo check
--workspace --all-targets` clean. The exit evidence's `tcl explore` line
over `set r [tenant::label acme]`, run in a scratch workspace whose
discovered pack is the fixture, reads `r#1 = const('tenant:acme')`, `route
tenant::label: implementation tenant.label.v1` and `· answer: evaluated`.

Deltas at Q12, beyond the coordinator's list:

- **The reload half was already keyed** (D104): a pack edit moves the
  content key, and the key alone re-keys every lattice (D97), so
  `a_pack_reload_after_an_edit_yields_the_new_answer_on_the_memoised_path`
  passes with the server's sync removed (checked); it pins the reload
  path end to end. What only the epoch covers is a quarantine, and the
  window in which a reload's plan reaches the workers before its key
  reaches the database; the tests that fail without it are the parity
  witness and the server's own `the_pass_after_a_quarantine_takes_the_evaluator_epoch`.
- **The server takes a quarantine after the pass**, at the start of the
  post-publish refresh, and reschedules nothing for it (D104).
- **The interned-GC control raises the epoch too**
  (`tests/interned_gc.rs`): a `LOW` epoch read by the deep tier stamps
  every per-body key `LOW`, so the control session that raises its
  inputs to `HIGH` stopped leaking and
  `raising_input_durability_disables_the_collector` failed. The epoch
  stays at `LOW` in production, as rule 1 of the crate docs' "The
  interned garbage collector is load-bearing" now lists it, and
  `no_input_durability_is_raised` still finds no call site.
- **Outside the lane's own files**: `rust/tcl-lsp-server/src/lib.rs`
  (the coordinator freed it once the diagnostic-policy lane finished) and
  its `tests/e2e/spec_packs.rs`, and the one publish line in
  `rust/tcl-spectcl/src/hooks.rs`.

Green at Q12: `cargo test -p tcl-lsp-db --no-fail-fast` 126 passed, 5
ignored (`interned_gc` 3 and `memory_growth` 1 among them); of the two
ignored corpus sweeps, `file_analysis_incremental_matches_full_over_corpus`
passes (895 files, no mismatch) and
`compiler_check_memo_matches_uncached_over_corpus` finds one mismatch in
893 files, which is not this change's (below); `cargo test -p
tcl-registry --lib` 905 passed; `cargo test -p tcl-spectcl` 300 passed,
1 ignored; `cargo test -p tcl-lsp-server --lib` 592 passed; its `e2e`
binary 1599 passed, 5 ignored (`spec_packs` 30 of them), and `smoke` 14,
`preview_tickets_e2e` 22 and `stdio_deadlock` 6 passed; pedantic clippy
on `tcl-registry`, `tcl-spectcl`, `tcl-lsp-db` and `tcl-lsp-server`, no
`#[allow]`; `cargo fmt --all --check` clean; `cargo xtask
value-transfers --check` unchanged (17 clean, 13 waived, 98 pinned
across 39 files, 6607 rows); no new test binary (manifest proof 325
targets); `cargo check --workspace --all-targets` clean.

Found at Q12, not fixed: `compiler_check_memo_matches_uncached_over_corpus`
fails on `tmp/tcllib-2.0/modules/imap4/imap4.tcl` in some runs and not
others. The memoised `compiler_check_diagnostics` then adds two T100s
("Tainted variable $argc flows into expr operand") at the script's
`$argc > 3` and `$i<$argc`, which the uncached path never reports; the
file alone mismatches in 14 of 24 runs with the `tcl-lsp-db` from before
this change and in 7 of 19 with it, so the run-to-run difference is older
than the epoch. T100 skips a seeded global at SSA version 0
(`is_seeded_global_v0`), so the memoised build's top level likely sees a
write to `argc` before those reads in the runs that differ, which puts
the cause in what that build records as writing globals rather than in
the taint lattice. For the owner of the memoised checks path.

At Q13: the lane doc alone; `cargo xtask value-transfers --check`
unchanged.

#### Record (2026-09-23): VT4.15 — docs and the landing

VT4.15's own state to start from was this record's own note (above): the
pages "still describe the original mechanisms where the build differs
(D76–D78, D88–D101); vocabulary 2.2, the option-input list shape and the
decline reasons still need documenting." Every page VT4.15 names was
read against the tree and D72–D104 rather than against the plan's own
prose, and edited only where a sentence described a mechanism the build
does not have or omitted one it does:

| Page | What changed |
|---|---|
| `docs/design/compiler/value-evaluation.md` | the status banner (built vocabulary vs. still-proposed slice 5–7 vocabulary vs. names the tree never adopted); § *Per-evaluation state* rewritten to `Engine::confine_stores` (D10, D77, D78, D101), replacing `ActivationStore` throughout, including the mermaid diagram and a test-anchor line; § *Two policies* corrected; § *`Engine::set_release`* rewritten to the `&str` signature and the dialect-ingress resolution (D76), with the per-program opt-in and no-load-time-notice corrections (D74) and `GrammarGuard` (D75); § *The rest of the route contract*'s host-environment bullet (D101); § *The memo key* rewritten to `ShapeKey` / `CallContent` (D95); the invalidation table gained the `EvaluatorEpoch` row (D104); § *The evaluator generation*'s salsa paragraph rewritten to the built overlay-reaching-every-query state (D96, D97) with D98 and D104 reconciled; § *The three nested budgets* rewritten to one `Budget` type at three call sites (D99), with the exhausted-request re-decline nuance (D100, Q13); § *Authoring on the routes*' intro (the vocabulary is built; no shipped builtin is declared with it yet) and the `incr` / `expr` / `regexp` / `tenant::label` worked examples' framing comments (the last now cites the real fixture, byte-identical); § *`-native ID`* rewritten to the fourteen separate tables and the `-direct` / `-native` / `-expression` three-catalogue split (a plan deviation this record's sonnet-items table already named); § *The four surfaces*' cluster name fixed ("Effects and purity") and the unbuilt "try it" box flagged as not delivered; § *Where each part lands*'s `ActivationStore` reference fixed |
| `docs/design/compiler/value-transfers.md` | § *One invocation, one context*'s closing paragraph rewritten from the pre-slice-4 gap description to the built overlay/epoch state (D96, D97, D104) |
| `docs/design/compiler/value-transfers-migration.md` | slice 4's row in § *The slices* marked landed, with `Engine::confine_stores` named against the interface page's `ActivationStore` and a pointer to the lane record; the ledger and the ratchet table needed no edit — confirmed unchanged through every slice-4 checkpoint (17 clean, 13 waived, 98 pinned across 39 files, 6607 rows) |
| `docs/design/compiler/value-transfers-examples.md` | § *A vendor loop and a private command in a workspace pack* rewritten from "Proposed" to the built declarations, verbatim from `specs/sdc_base.tclspec` and the `tenant.tclspec` fixture, `append_to_collection`'s shape added, and the release-decline / unknown-argument paragraph corrected (D88, D103) |
| `docs/design/compiler/registry-consumer-contracts.md` | § *Dialects and packages*' "release for versioned evaluation" row annotated: `TclVersion::from_profile` is still CC9.2's open gap, but the value-transfer route's own base-release rule no longer waits on it (D72) |
| `docs/design/registry/spec-packs.md` | a new paragraph after "Crash containment is a load-bearing guarantee" stating the store- and host-environment-confinement rule (D77, D78, D101) |
| `docs/design/contracts/command-spec-studio.md` | § *Parity with native specs* gained a paragraph citing the value-transfer statements as the newest concrete instance of the four-surface rule |
| `docs/design/compiler/pass-fact-ownership-matrix.md` | the `value_transfer.rs` producer row extended to name the declared-implementation route and the evaluator-epoch re-keying (D94, D104) |
| `docs/design/spec-dsl-examples/README.md` | a 2.2 row added to § *Vocabulary changelog*; § *Purity and the sandbox* gained the same confinement paragraph as spec-packs.md, scoped to the hook-body sandbox |
| `docs/kcs/spectcl/kcs-howto-declare-an-evaluator-for-a-pack-command.md` (new) | states the option-input list shape (D89) and the declined-implementation reasons (D88) in plain terms, as the note this slice's own record asked for; indexed in `docs/kcs/README.md` and cross-linked from `kcs-howto-write-a-tclspec-pack.md` and `compiler/kcs-qa-what-does-a-value-transfer-declaration-say.md` |
| this lane doc | § *Checkpoints and landing* rewritten to the realised commit list; the landing commit's template corrected ("target axes" removed — D79 gives the DSL's implementation block no target row — and the evaluator epoch, the host-environment read, and the `spectcl_check` / EDA-command deliverables added, none of which the plan's original template named); a `Status: slice 4 landed` section added; this record |

Deviations from the plan's own wording, each a correction rather than a
new decision: the landing template's "target axes" phrase (D79 already
says the implementation block carries none); the salsa paragraph's old
"insufficient if `FnLatticeKey` … still drops it" framing, which the
built overlay plumbing (D96, D97) makes moot for `compilation_unit` and
`function_lattice` specifically, while leaving the separate
`ModuleCommandMutations` / rename-invalidation gap exactly as it was (not
this lane's to close). No design decision was needed: every correction
traces to a D72–D104 entry already taken, or to source read directly
(`rust/tcl-engine-api/src/lib.rs`, `rust/tcl-registry/src/pack_hooks.rs`,
`rust/tcl-registry/src/value_transfer/context.rs`, `route.rs`,
`rust/tcl-lsp-db/src/lib.rs`, `specs/sdc_base.tclspec`, the `tenant.tclspec`
fixture).

Green: `cargo xtask kcs-index-links` ("KCS docs checks passed"); `cargo
xtask owner-resolution` (OK, 44 rows, unchanged); `cargo test -p
tcl-spec-studio --test reference_doc` passes with no `UPDATE_REFERENCE`
run (the studio schema is unchanged since VT4.11, so
`docs/references/command-spec/fields.md` was not regenerated); `cargo
check --workspace --all-targets` clean; `cargo test -p tcl-registry --lib`
905 passed, 0 failed (the smoke run; unchanged from Q12's checkpoint).

#### Record (2026-09-23): the review of slice 4

The review of the landing (`1fa954b1`) returned "land after fixes" with
nine findings. They land as one commit, `wip(value-transfers): review
fixes for slice 4`, after VT5.1 (`316ee045`) and the consumer-contracts
lane's CC2.6 (`e67ae23e`); the decisions they needed are D110 to D114.
The review verified the rest as correct and asked for no action: store
and host-environment confinement, the D88 reasons, the D89 option shapes,
D102's order, ruling 8 per axis, the hook cache's full content
comparison, the epoch mechanics and both epoch tests, the overlay's
reach, the budgets' nesting, VT4.11's round trip, VT4.12's pessimism,
VT4.14's outcomes and goldens, `shipped_builtins_stay_on_the_direct_route`,
headers and spelling.

| Finding | What changed | Its tests |
|---|---|---|
| 1. `rand()` and `srand()` reach interpreter state under every 8.4-based release | `Vm::confine_generator` (`rust/tcl-vm/src/interp.rs`, beside `confine_store`); `m_rand` and `m_srand` (`cmd_math.rs`) refuse while `confined_stores` is set (D110); the evaluation page's § *Per-evaluation state* says so | `confine_stores_refuses_every_store_outside_the_activation` gains the generator rows (the engine's default release, `tcl8.4`, `f5-irules`, each × `expr {srand(7)}` and `expr {rand()}`); `the_generator_is_refused_under_every_pinned_release` (`containment_e2e`: release-pinned bodies under `tcl8.4`, `f5-irules`, `tcl8.6`, `tcl9.0` — the seeding and drawing bodies abstain, `abs(-1)` folds `1`); both fail without the fix |
| 2. A pool thread keeps a host from a superseded plan | `pack_hooks::ensure_host` runs the registered installer before it reads `HOST` whenever the thread's host came from a plan (`FROM_PLAN`, set by `install_plan_host`, cleared by `install_host` and `clear_host`); a host installed directly is returned as it is; its comment says what it guards (D111) | `a_pool_thread_with_a_stale_host_answers_with_the_new_plan` (`tcl-lsp-db`'s parity module: a thread whose host is the first plan's, after the edited pack is published and its key set, folds `[tenant::label acme]` to the new body's `t:acme`, not the old `tenant:acme`); fails without the fix. It lives beside the other publishing witnesses rather than in the server's tests (D111) |
| 3. A write, a preserve and `target N incoming` work only in statement position, and only with the role and the trait | one real-driver witness; the KCS howto and the page's § *A private command* state the `arg N -role VarWrite` and `traits {READS_BEFORE_WRITE}` requirements and the value-position limit | `a_pack_write_through_an_incoming_target_reaches_the_driver` (compiler witnesses, `acc` pack: `acc::add s cd` in statement position writes `s` to `abcd`; `[acc::add s ef]` in value position declines `not-exact` and `[acc::store u xy]` is `not substituted: the outcome writes storage`, both results `Overdefined`; `acc::put`, declared without the trait, declines `not-exact`) — `abcd` checked against tclsh 8.4 to 9.1 with the body as an `upvar` procedure |
| 4. `-native ID` resolves nothing | the loader's `native_fold` resolves `const_fold -native ID` and `const_fold_versioned -native ID` at command and subcommand scope (D112); `CONST_FOLD_NATIVE` (47 rows) and `CONST_FOLD_VERSIONED_NATIVE` (3) hold every folder the shipped registry installs, the catalogue's pickers match them, and the folders they name became `pub(crate)` (`string_.rs`, `namespace_.rs`, and one visibility hunk in `subst_.rs`); `string.tclspec` spells `string::is::const_fold_versioned` and its golden is regenerated; the page's § *`-native ID`* names the twelve families whose tables are still empty | `a_native_fold_id_installs_the_shipped_folder` (loader: the named cores fold `string range abcdef 1 3` to `bcd` and `string is integer 42` to `1`, both checked against tclsh 8.4 to 9.1; a short id and an unknown id are notices and install nothing); the studio's `every_command_in_every_dialect_round_trips_through_spectcl` and `the_eleven_port_fixtures_render_and_reload_as_themselves` now reload every rendered shipped folder by name |
| 5. The KCS howto's words | the outcome words are `write`, `write_or_preserve`, `may_write` and `unbind`; only `body` is required; the role and trait bullets (finding 3) | `cargo xtask kcs-index-links` |
| 6. Three sentences | the page's § *Per-evaluation state* (with finding 1); D104's window corrected in place ("whichever computes first is the memo every worker reads until the next edit or epoch"); `rust/tcl-engine-tclvm/src/lib.rs`'s unit-name comment now says what the guarantee is — the name is an ordinary one a body can spell (`::spectcl::unit::N`), so it is not private to the host; what makes that safe is that an engine serves one pack, so a body reaches only its own pack's units, a recursive call raises through the command budget or the nesting limit, and nothing in the sandbox defines a command, so no body can shadow a unit | — |
| 7. The viewport reads the unit without the pack overlay | the server's `db_compilation_unit_handle` reads `document_compilation_unit_for(&*snapshot, file, config)` with `config` from `resolved_db_config(uri)` (D113) | `range_tokens_read_the_unit_under_the_workspace_pack_overlay` (server: a `mylib::put` pack command with `arg 0 -role VarWrite`; the enriched viewport over the whole document equals the full-document query's tokens, and the same tier over the unit built without the overlay differs); fails without the fix |
| 8. Math functions are gone under a pinned engine at 8.5 or later | `restrict_commands` keeps every `tcl::mathfunc::*` command except `rand` and `srand` when `expr` is allowed (D110) | `a_restricted_engine_keeps_the_math_functions_but_the_generator` (`tcl8.4` to `tcl9.1` and `f5-irules`: `expr {abs(-1) + int(2.5) + double(1)}` is `4.0`, checked against tclsh 8.4 to 9.1; `rand()` and `srand(7)` raise) |
| 9. D99's "200 ms", and two budget rules | D99, Q13, the page's § *The three nested budgets* and `Budget::REQUEST_WORK`'s doc state the real bound; the loader's `budget_row` records the row as written and the host is the only place a declared budget is capped (D114, amending D91) | `what_cannot_be_used_is_reported_and_dropped` (loader: `budget {-commands 900000}` loads as written, with no notice); `a_declared_budget_above_the_hosts_runs_under_the_hosts` (`containment_e2e`: a host configured at 200 commands runs a call that declares 900,000 under its own 200 — a command-budget crash and a quarantine — and a 50-command body answers `50`) |

Deviations from the findings' own wording, each for the reason given:
finding 2's witness is not a server test (D111 says why); finding 4
filled both tables with every shipped folder, not only the ids a
test names, because once an id resolves an unresolved one is a notice,
and the renderer writes `-native SCOPE::FIELD` for every shipped folder —
until the tables held all fifty, the studio's round trips failed with 28
"names nothing this build ships" notices.

Green at the review fixes:

- tests, each crate's full suite unless named: `tcl-vm` (under
  `LANG=C.UTF-8`) 1473 passed across 50 binaries; `tcl-engine-tclvm` 16;
  `tcl-spec-hooks` 47; `tcl-registry` 1209; `tcl-spectcl` 302, 1 ignored;
  `tcl-spec-studio` 284; `tcl-compiler` 9721, 6 ignored, across 68
  binaries; `tcl-lsp-db` 127, 5 ignored; `tcl-lsp-server --lib` 593 and
  its `e2e` binary's `semantic_tokens` and `spec_packs` modules 122, 1
  ignored; `tcl-mcp`'s `spectcl` and `spec_import` tests 34; `tcl-cli`'s
  `samples_optimiser_profiles_are_regenerated` passes (no sample moved);
  no failure anywhere;
- pedantic clippy (`--no-deps --all-targets -D warnings`) on the eight
  touched crates, no `#[allow]` added; `rustfmt` on the touched files;
- `cargo xtask value-transfers --check`: OK, unchanged (17 files clean, 13
  sites waived, 98 pinned across 39 files, 6607 inventory rows);
- `cargo xtask registry-axes --check`: OK (7831 vocabulary words, 1089
  sites pinned across 163 files);
- `cargo xtask pack-goldens`: `string.snap` rewritten, 24 packs scanned;
  it carries CC2.6's `spec` hash and this commit's `hooks` hash;
- the nextest shard verifier: OK (no new test binary);
- `cargo xtask kcs-index-links`: "KCS docs checks passed";
- `cargo check --workspace --all-targets`: clean.

### Slice 5 — destructuring and structured bodies

#### Goal and exit

In the plan's words: "Write, preserve, unbind, and may-write outcomes with
heterogeneous per-target types and duplicate targets resolved to places;
the regexp owner's typed precision result; `regexp`, `scan`, `lassign`,
`binary scan`; `dict with` and `dict update` as structural plans with a
key-binding projection; the template-word plan, with `subst`'s option rows
declaring the kinds; W210 consuming preserve outcomes; the existence branch
fact stored once with its kind; O111 consuming the same fact as W100 or an
explicit rule-group policy." The migration plan's inventory adds the
editor consumers and "the analyser's literal-only diagnostics"; the ledger
adds the loop header's iteration plan over the source layout; the
analysis table adds `folded_types`; `KNOWN_GAPS` adds the other slice-5
writers; CC2.13 puts the `DictWith` and `RegexPatternCapture` analyser
hooks on this slice.

*Exit*: "program (2) folds and is typed as a byte array; the private
regexp / scan prover in `dataflow.rs` is retired; the no-match preserve,
partial `scan`, and `lassign … a a` witnesses pass; the four consumers read
`TemplateWordPlan` and none walks a template word."

Exit evidence: `program_two_folds_and_is_a_byte_array`,
`the_no_match_preserve_witness`, `the_partial_scan_witness`,
`the_repeated_target_witness`, `the_template_plan_answers_the_fourteen_witnesses`,
`a_materialised_child_carries_its_factory_call_span` (compiler witnesses
and `value_transfers.rs`); G1 with `analyser/diagnostics/dataflow.rs` at
pin 2, `helpers.rs` at 5, `var_command.rs`, `specialise_factories.rs` and
`rust/tcl-lsp-core/src/document_links.rs` in `CLEAN_FILES`, and
`KNOWN_GAPS` without a slice-5 row; `tcl explore --source 'set h [binary
format H* 414243444546]' --show sccp --text --no-colour` prints
`h#1 = const('ABCDEF')` with the folded type `bytearray (constructed)`.

#### Upstream starting point

From `git diff 3b5eba8a origin/rust -- rust/tcl-cmd-core/src/regex.rs
rust/tcl-regex/src rust/tcl-vm/src/cmd_regexp.rs
rust/tcl-registry/src/commands/tcl/regexp_.rs
rust/tcl-registry/src/commands/tcl/scan_.rs
rust/tcl-registry/src/commands/tcl/binary_.rs
rust/tcl-registry/src/commands/tcl/subst_.rs rust/tcl-registry/src/traits.rs
rust/tcl-compiler/src/ssa.rs`:

- `tcl-cmd-core/src/regex.rs` (+741) and `tcl-regex/src/cmd_core.rs`:
  `regexp -about` answers from the engine's `re_info` flags
  (`regexp -about {(?:a)}` is `0 REG_UNONPOSIX` on 8.4 to 9.1), `regsub
  -command` runs through a callback adapter (`RegsubError`), and
  `-nocase` folds the full Unicode range with a bounded repeat (#2124,
  #2125, #2127). The evaluation page's "the core refuses both" is out of
  date: VT5.4 evaluates `-about` through the core and declares `regsub
  -command` `NoRoute(Callback)` (a callback form needs a declared route on
  the callback), and VT5.3 threads `RegexpPrecision` through the merged
  functions.
- `regexp_.rs` (+109, #2222): `-about` and `-inline` name no match
  variables; VT5.4's targets come from those roles.
- `traits.rs`, `scan_.rs`, `binary_.rs`, `ssa.rs` (#2225):
  `Traits::CONDITIONAL_VARIABLE_WRITE` and the SSA rule that such a call
  reads its targets' prior versions. VT5.11 keeps the rule as the SSA's
  reading of a declared `Preserve`, and
  `conditional_writers_declare_preserve_outcomes` (`value_transfers.rs`)
  holds the trait and the outcomes equal, so one fact has one source.
- `subst_.rs` (+8, #2136): `reserved_trailing_words: 1` names the
  template operand; VT5.8 reads it.
- The compiler consumers this slice rewrites (`security.rs`,
  `var_command.rs`, `helpers.rs`, `dynamic_names.rs`,
  `specialise_factories.rs`, `subst_nocommands.rs`, `type_infer.rs`,
  `shimmer/` but `graph.rs`) and the `tcl-lsp-core` providers are
  unchanged upstream.

#### Work items

##### VT5.1 — every outcome kind, applied per place

- **Files**: `rust/tcl-registry/src/value_transfer/answers.rs`,
  `rust/tcl-compiler/src/value_transfer.rs`, `rust/tcl-compiler/src/sccp.rs`.
- **Items**:
  ```rust
  // answers.rs
  /// The driver's checks before anything publishes: one outcome per
  /// declared target at most, indices in range, every store naming a
  /// validated place-bearing target, overlap resolved to places.
  pub fn validate_outcome(
      plan: &PlanAnswer,
      targets: &[TargetId],
      outcome: &InvocationOutcome,
  ) -> Result<(), DeclineReason>;
  // value_transfer.rs — replaces VT2.4's `store_def`
  fn apply_outcome(
      &self,
      outcome: &InvocationOutcome,
      defs: &[(String, ValueKey)],
      input: &dyn AnalysisInputs,
  ) -> Vec<(ValueKey, LatticeValue)>;
  ```
  Each written place's definition takes, in execution order: a `Write`'s
  value (a repeated target composes in order, so the last write wins); a
  `Preserve`'s prior version's value and representation; `Overdefined`
  for a `MayWrite` (its `FactBounds` go to `folded_types`) and for an
  `Unbind` (the existence fact is slice 8's). An element write and a base
  write of one array, or a traced or escaping target, is
  `OverlappingTargets` / `TracedPlace` / `EscapingPlace` and widens. A
  malformed answer is `MalformedAnswer`, a notice, and `Overdefined`.
- **Preserves**: every one-write answer of slice 2.
- **Changes**: through VT5.4 and VT5.5 (their deltas).
- **Tests**: `validate_outcome_rejects_a_store_to_a_non_target`
  (`value_transfers.rs`).
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: L. **After**: slice 4.

##### VT5.2 — `folded_types` and representation evidence

- **Files**: `rust/tcl-compiler/src/sccp.rs` (`SccpResult`),
  `rust/tcl-compiler/src/value_transfer.rs`, `rust/tcl-compiler/src/type_infer.rs`,
  `rust/tcl-compiler/src/shimmer/byte_array.rs`,
  `rust/tcl-compiler/src/shimmer/use_site.rs`,
  `rust/tcl-compiler/src/representation_plan.rs`,
  `rust/tcl-explorer/src/serialise.rs`, `view_tree.rs`.
- **Items**:
  ```rust
  /// The semantic type, shape and representation evidence of one value,
  /// from the evaluation that produced it.
  pub struct FoldedType {
      pub intrep: Option<TclType>,
      pub shape: Option<ValueShape>,
      pub representation: RepresentationEvidence,
  }
  pub struct SccpResult { /* … */ pub folded_types: HashMap<ValueKey, FoldedType> }
  ```
  `type_infer` joins it; S110 and the S100 use-site check read
  `representation`, never `TclType` alone. Keyed by `ValueKey`, so
  `lattice_rebase.rs` is untouched.
- **Changes**: a computed `binary format` is not reported as a
  conversion. Mandate: the S100/S101 and S102/S103/S110 rows ("S110 reads
  representation evidence for a byte-array value").
- **Tests**: `program_two_folds_and_is_a_byte_array` (with VT5.6);
  `serialise::tests::sccp_reports_folded_types`.
- **Gates**: G7, G8, G9 (`tcl-compiler`, `tcl-explorer`).
- **Model**: opus. **Size**: M. **After**: VT5.1.

##### VT5.3 — the regexp owner's typed precision result

- **Files**: `rust/tcl-regex/src/lib.rs` (`Regex::exec`),
  `rust/tcl-regex/src/exec.rs` (`Matcher::spend_fuel`, `spend_fuel_n`,
  `Bt::m`, `dissect_repeat`), `rust/tcl-regex/src/cmd_core.rs`
  (`AreEngine`), `rust/tcl-cmd-core/src/regex.rs` (`RegexEngine::exec`,
  `regexp`, `regsub`, `RegsubResult`), `rust/tcl-regex/tests/precision_oracle.rs`
  (new), `scripts/dev/rust-test-binary-shards.tsv`.
- **Items**: the evaluation page's `RegexpPrecision<V>` (`Exact { whole,
  groups, captures_exact }`, `NoMatch`, `Declined(PrecisionDecline)`) and
  `PrecisionDecline` (`FuelExhausted { spent }`, `DepthExhausted { limit }`,
  `ApproximateCapture { group }`, `PatternError(RegexError)`,
  `FormUnsupported { option }`, `Cancelled`), verbatim, through the five
  layers the page lists; the page's `PatternCacheKey` cache, bounded at
  4 MiB of retained bytes, compile charged as the pattern's length
  squared; cancellation read at `spend_fuel` / `spend_fuel_n`.
- **Preserves**: every completed match and no-match on every existing
  `tcl-regex`, `tcl-cmd-core` and runtime test.
- **Changes**: on the runtime path a `PrecisionDecline` is a `RegexError`,
  so a search that exhausts its fuel raises instead of answering 0; on the
  analysis path it is a typed decline. Mandate: § *The typed precision
  result*, step 3.
- **Tests**: `the_three_precision_witnesses` (`precision_oracle.rs`, the
  page's table under 8.4 to 9.1: `regexp -indices {^a*(b)\1$}` over 300
  `a` and `bb`; `regexp -indices {(x)*}` over 300 `x`; `regexp
  {^(a+)+\1$}` and `regexp {^(a+)+b$}` over 300 and 301 `a`, which answer
  1 and 0); `an_exhausted_search_is_never_a_no_match` (a fuel limit small
  enough to exhaust yields `FuelExhausted`, never `NoMatch`). Shard row:
  `2\ttcl-regex::precision_oracle\ttest\ttcl-regex\tprecision_oracle`.
- **Gates**: G3, G7, G8, G9 (`tcl-regex`, `tcl-cmd-core`, `tcl-vm`,
  `tcl-registry`).
- **Model**: opus. **Size**: L. **After**: —.

##### VT5.4 — `regexp` and `regsub`

- **Files**: `rust/tcl-registry/src/value_transfer/regex.rs` (new),
  `rust/tcl-registry/src/commands/tcl/regexp_.rs`,
  `rust/tcl-registry/src/commands/tcl/regsub_.rs`,
  `rust/tcl-registry/tests/value_transfers.rs`,
  `rust/tcl-registry/tests/differential_fold.rs`,
  `rust/tcl-compiler/src/analyser/handlers.rs`
  (`handle_regex_pattern_capture`), `rust/tcl-registry/src/hooks.rs`
  (`AnalyserHookId::RegexPatternCapture` goes),
  `rust/tcl-registry/tests/analyser_hooks.rs`, `rust/xtask/src/value_transfers.rs`.
- **Items**: `RegexpSemantics` and `RegsubSemantics` over
  `tcl_cmd_core::regex::{regexp, regsub}` and `AreEngine`
  (`NEEDS = REGEXP_FEATURES | CHAR_INDEXING | SOURCE_ENCODING`): on a
  match, one `Write` per match variable (an unmatched subgroup writes the
  empty string, or `-1 -1` with `-indices`); on `NoMatch`, one `Preserve`
  per match variable; `-inline` writes nothing and returns the list;
  `-all` counts; `-about` and `regsub -command` are
  `NoRoute(FormUnsupported)`; any `PrecisionDecline` declines the whole
  answer. `regsub`'s shipped folder becomes this route through
  `evaluate_literal`, as `string range`'s did. The analyser's
  `handle_regex_pattern_capture` reads the declared match targets, and
  the hook variant retires (CC2.13's ledger row).
- **Preserves**: `regsub`'s folded answers; every `regexp` test.
- **Changes**: a no-match `regexp` keeps its match variables' values in
  the lattice; a match writes them. Mandate: the Storage row ("`regexp`
  no-match"); the Regexp row.
- **Tests**: `regexp_writes_or_preserves_its_match_variables`
  (`value_transfers.rs`) and `regexp_witnesses_match_every_release_on_path`
  (`differential_fold.rs`, 8.4 to 9.1): `set a before; set b before;
  regexp {(x)(y)} zz a b` is 0 with both `before`; `regexp -inline
  -indices {(a)(b)?} ac` is `{0 0} {0 0} {-1 -1}`; `regexp -all {a*}
  xaax` is 3; `regsub -all {} abc -` is `-a-b-c`; `regexp -about {a}`
  declines.
- **Gates**: G1 (`regexp`, `regsub` rows go), G2, G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: VT5.1, VT5.3.

##### VT5.5 — `scan`, `binary scan`, `lassign`, `array set`

- **Files**: `rust/tcl-registry/src/value_transfer/destructure.rs` (new),
  the four commands' spec modules, `value_transfers.rs`,
  `differential_fold.rs`, `rust/xtask/src/value_transfers.rs`.
- **Items**: `ScanSemantics` over `tcl_cmd_core::scan::{validate_format,
  scan_match}` (`ScanOutcome`, `Scanned`): the count as the result, a
  `Write` per converted target with its type, a `Preserve` for the rest;
  `BinaryScanSemantics` over `tcl_cmd_core::binary`
  (`NEEDS = BINARY_FIELDS | BYTE_STRINGS`); `LassignSemantics` over
  `ConstOps::list_elements` (from 8.5: absent in the profile is no
  route): a `Write` per target in order, the empty string past the end,
  the remaining elements as the result; `ArraySetSemantics`: one element
  `Write` per pair of a constant list (`PlaceKind::Element`).
- **Changes**: partial `scan`, `binary scan`, `lassign` and `array set`
  reach the lattice and `folded_types` per target. Mandate: the Storage
  row ("partial `scan`; … repeated targets; array and base overlap").
- **Tests**: `destructuring_writers_run_the_shared_cores`
  (`value_transfers.rs`) and `destructuring_witnesses_match_every_release_on_path`
  (8.5 to 9.1 for `lassign`, 8.4 to 9.1 for the rest): `scan {12 nope}
  {%d %d} a b` is 1 with `a` 12 and `b` preserved; `lassign {first second
  extra} a a` is `extra` with `a` `second`; `binary scan \x01\x02 cc a b`
  is 2 with `a` 1 and `b` 2; `array set arr {k1 v1 k2 v2}` writes two
  elements.
- **Gates**: G1 (four rows go), G2, G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: VT5.1.

##### VT5.6 — `binary format`, program (2)

- **Files**: `rust/tcl-registry/src/value_transfer/builtins.rs`,
  `rust/tcl-registry/src/commands/tcl/binary_.rs`, `value_transfers.rs`,
  `differential_fold.rs`.
- **Items**: `BinaryFormatSemantics` over `tcl_cmd_core::binary::format`
  (`NEEDS = BINARY_FIELDS | BYTE_STRINGS`), answering
  `RepresentationEvidence::Constructed(TclType::ByteArray)`; declared on
  the `binary format` subcommand, which declares `pure: true` and no
  evaluator today.
- **Changes**: `set h [binary format H* 414243444546]` is `ABCDEF`, typed
  a byte array. Mandate: the exit; program (2) of the interface page.
- **Tests**: `program_two_folds_and_is_a_byte_array` (compiler witnesses);
  `binary_format_witnesses_match_every_release_on_path` (8.4 to 9.1).
- **Gates**: G1, G2, G7, G8, G9.
- **Model**: opus. **Size**: S. **After**: VT5.2.

##### VT5.7 — structural plans: `dict with`, `dict update`, the loop's source layout

- **Files**: `rust/tcl-registry/src/value_transfer/body.rs` (new),
  `rust/tcl-registry/src/value_transfer/iteration.rs`,
  `rust/tcl-registry/src/commands/tcl/dict.rs` (and the `::tcl::dict::`
  copies through `qualified_specs()`),
  `rust/tcl-compiler/src/analyser/handlers.rs` (`handle_dict_with_command`),
  `rust/tcl-registry/src/hooks.rs` (`AnalyserHookId::DictWith` goes),
  `rust/tcl-registry/tests/analyser_hooks.rs`,
  `rust/tcl-compiler/src/value_transfer.rs` (`evaluate_call`'s loop-header
  transfer reads the plan's binders).
- **Items**: `DictWithSemantics` and `DictUpdateSemantics` answering
  `PlanAnswer::Body { binders, body, reconcile:
  Reconcile::WriteBackKeys(dict), completion: CompletionProtocol::TclBody }`,
  the binders projected from the dictionary's proven keys (`dict with`)
  or the declared key and variable pairs (`dict update`) — a projection
  on body entry, never the command's value; `IterationSemantics::structure`
  answers `InvocationLayout::Source` for `foreach` and `lmap` (binders from
  each var-list word, one `IterableKind::List` per list operand, the body,
  `CompletionProtocol::Absorb(&[Break, Continue])`,
  `zero_iterations_bind: false`). `handle_dict_with_command` binds the
  plan's binders instead of reading `lookup_const_string`, and the hook
  variant retires (CC2.13).
- **Preserves**: `evaluate_def_*` answers over loop headers; the
  analyser's `dict with` bindings over a literal dictionary.
- **Changes**: a `dict with` over a lattice-constant dictionary binds its
  keys in the body. Mandate: "`dict with` and `dict update` as structural
  plans with a key-binding projection"; the ledger row for the loop header
  ("slice 5 — the iteration plan's binders over the source layout").
- **Tests**: `dict_with_binds_the_proven_keys` (`set d {a 1}; dict with d
  {incr a; set result done}` gives `result` `done`; the oracle: `d` is
  `a 2` after, in 8.5 to 9.1); `the_source_layout_answers_an_iteration_plan`
  (`value_transfers.rs`); the mirror-pairs witness gains its
  `CorrelatedSets` reason once the two-binder `foreach` source is lowered
  (`the_mirror_pairs_decline_as_correlated` asserts the reason as well as
  the outcome, D64).
- **Gates**: G1 (`dict with`, `dict update` and their qualified rows go),
  G2, G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: VT5.1.

##### VT5.8 — the template-word plan

- **Files**: `rust/tcl-registry/src/value_transfer/template.rs` (new),
  `rust/tcl-registry/src/commands/tcl/subst_.rs` (the `semantics` field
  only; its option rows are the consumer-contracts lane's),
  `rust/tcl-compiler/src/value_transfer.rs`, `rust/tcl-compiler/src/sccp.rs`,
  `rust/tcl-compiler/src/lattice_rebase.rs`.
- **Items**: `SubstSemantics` whose `structure` answers
  `PlanAnswer::TemplateWord(TemplateWordPlan)` (the tree's shape, which is
  the page's): `kinds` from `option_effects` (CC2.6) over the switch
  operands — a `Const` switch as its literal spelling, a `ConstSet` joined
  per member, an unproven one `SubstitutionKinds::ALL`; the 9.1 positive
  family; a profile spanning 9.1 declines `ReleaseAmbiguous`; `braced`,
  `dynamic`, `script_regions`, `reads` and `escapes` from `word_structure`.
  The driver records one `TemplatePlanRecord { span: Span, plan:
  TemplateWordPlan }` per `subst` statement in `SccpResult::template_plans`,
  which `rebase_function_unit` shifts in the same change. (VT5.20: VT5.10
  (D154) grows this to `TemplatePlanRecord { span, command: String,
  switches: Option<Vec<String>>, plan }` — the command as spelled and each
  switch's proven spelling, `None` when one is unproven, which the W102
  narrowing's advice reads; the tree's shape is current as of slice 5's
  landing.)
- **Tests**: `the_template_plan_answers_the_fourteen_witnesses`
  (`value_transfers.rs`, the page's fourteen programs as plan fixtures);
  `template_witnesses_match_every_release_on_path` (`differential_fold.rs`,
  8.4 to 9.1, with the three 9.1-only rows: `a5[set b]`, `a$b[set b]A`,
  and the mixed-family error); the `tp_*` and `fp_*` tests in
  `rust/tcl-registry/src/substitution.rs` unchanged;
  `rebase_shifted_unit_spans_match_fresh` covers the new spans.
- **Gates**: G1, G2, G7, G8, G9.
- **Model**: opus. **Size**: L. **After**: VT3.1; CC2.6 (B-CC2).

##### VT5.9 — the template folders read the plan; the child keeps its span (#2143)

- **Files**: `rust/tcl-compiler/src/lowering/mod.rs`
  (`eval_subst_nocommands_body`), `rust/tcl-compiler/src/subst_nocommands.rs`,
  `rust/tcl-compiler/src/specialise_factories.rs`
  (`extract_subst_nocommands_template`, `register_synthesised`),
  `rust/xtask/src/value_transfers.rs`.
- **Items**: both folders ask the registry for the plan over the call's
  literal words (`LiteralInputs`; lowering runs before SSA) and act only
  on `kinds == SUBST_NOCOMMANDS_KINDS`, `braced`, every `reads` name in
  the const map, and `escapes`; `subst_nocommands` consumes `reads`
  instead of scanning the template; `cmd.texts[0] != "subst"` goes.
  `register_synthesised(module, name, params, body, span: Span)` records
  the factory call's statement span on the child `Procedure`.
- **Preserves**: `proc_subst_nocommands_body_materialised`,
  `proc_subst_nocommands_missing_var_skips_materialisation`,
  `proc_subst_nocommands_nobackslashes_refused`, `detects_*`,
  `rejects_factory_with_computed_subst_switch`.
- **Changes**: the materialised child's findings anchor at the factory
  call, not at 1:1. Mandate: #2143 ("a materialised child carries the
  span of the factory call that produced it").
- **Tests**: `a_materialised_child_carries_its_factory_call_span`
  (compiler witnesses: the #2143 program's W214 for `::port` is on line
  4, the `Configure port 8080 {the port}` call).
- **Gates**: G1 (`specialise_factories.rs` 1 → 0: the remaining `proc`
  definer site is waived `definition_body`; the file joins
  `CLEAN_FILES`), G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: VT5.8.

##### VT5.10 — W102, extract-proc and the dynamic-name barrier read the plan

- **Files**: `rust/tcl-compiler/src/analyser/diagnostics/security.rs`
  (`emit_w102_subst_injection`, `substitution_narrowing_switches`,
  `inner_head_performs_substitution`),
  `rust/tcl-compiler/src/analyser/diagnostics.rs` (the per-function pass
  emits W102 from `SccpResult::template_plans`),
  `rust/tcl-compiler/src/dynamic_names.rs` (`DynamicNameBarrier`,
  `template_word_is_substituted`),
  `rust/tcl-lsp-core/src/refactor/extract_proc.rs` (`literal_word_holes`),
  `rust/tcl-lsp-core/src/refactor/mod.rs` (`push_substituted_commands`,
  `same_frame_regions`).
- **Items**: W102 moves from the walk to the per-function pass and reads
  `kinds`, `dynamic` and the option rows' narrowing advice from the
  statement's plan; the barrier sets `reads` only when `dynamic &&
  kinds.variables` and scans `script_regions`; extract-proc keeps or cuts
  a braced word by `kinds.variables` and takes `script_regions` as the
  same-frame regions, over the plan of the call's literal words.
- **Preserves**: every `w102_*` test;
  `tp_a_substituting_call_can_switch_its_variable_reads_off`,
  `tp_a_substituted_bracket_reads_the_caller_even_with_variables_off`,
  `tp_a_substituted_bracket_writes_through_to_the_caller`.
- **Changes**: `set opt -novariables; subst $opt {hello $name}` narrows
  W102's advice to `$var` as the literal spelling does; `subst
  -novariables $t` no longer blinds every read of the function. Mandate:
  the W102 row; the dynamic-name row ("so `subst -novariables $t` stops
  blinding every read").
- **Tests**: `w102_narrows_a_proven_switch_word` (negative: `opt` a
  parameter keeps every kind); `a_novariables_subst_of_a_dynamic_template_does_not_blind_reads`
  (`dynamic_names.rs`).
- **Gates**: G1, G7, G8, G9 (`tcl-compiler`, `tcl-lsp-core`).
- **Model**: opus. **Size**: M. **After**: VT5.8; the diagnostic-policy
  lane's `tcl-lsp-core` commits (B-DP1).

##### VT5.11 — W210 reads preserve outcomes; the private prover retires

- **Files**: `rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs`
  (`emit_provably_unset_w210`), `rust/xtask/src/value_transfers.rs`,
  `docs/design/compiler/value-transfers-migration.md`.
- **Items**: `emit_provably_unset_w210` asks the unit for the call's
  evaluated outcome: a target the outcome `Preserve`s keeps the prior
  definition as the read's reaching definition, so a read after a
  no-match `regexp` or a partial `scan` of a variable with no prior
  definition is W210. The `regexp` / `scan` recognisers and the form
  parsing at the prover go, with the second traversal for embedded
  conditions.
- **Preserves**: every `emit_cfg_ssa_diagnostics_w210_*` test and every
  answer the prover gave on the forms it proved.
- **Changes**: W210 follows any registry-declared preserve outcome, a
  pack command's included. Mandate: "W210 consuming preserve outcomes";
  § *Diagnostics consume facts*, first producer; the Diagnostic separation
  row ("no private regexp or scan evaluator in W210").
- **Tests**: `w210_reads_a_no_match_preserve_outcome`
  (`analyser/diagnostics/tests.rs`: `regexp {(x)(y)} zz a b; puts $a` is
  W210; negative: `regexp {(x)(y)} xy a b; puts $a` is not);
  `a_no_match_keeps_the_store_it_preserves` (compiler witnesses: `set a
  before; regexp {(x)(y)} zz a b; puts $a` keeps `set a before` under O109
  and W220 and prints `before` under 8.4 to 9.1 — #2051's program).
- **Gates**: G1 (`dataflow.rs` 4 → 2; its ledger row names only the
  `unset` scans and slice 8), G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: VT5.4, VT5.5.

##### VT5.12 — the branch fact records its kind

- **Files**: `rust/tcl-compiler/src/sccp.rs` (`ConstantBranch`),
  `rust/tcl-compiler/src/compilation_unit.rs` (the existence post-pass),
  `rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs`
  (`emit_existence_constant_branch_diagnostics`,
  `emit_constant_branch_diagnostics`), `rust/tcl-compiler/src/compiler_checks.rs`.
- **Items**:
  ```rust
  /// Which of the three branch facts this is: a proven condition, a
  /// selected arm with no CFG edge of its own, or applied reachability.
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
  pub enum BranchFactKind { Proven, Selected, Applied }
  pub struct ConstantBranch { /* … */ pub kind: BranchFactKind }
  ```
  The solver's decided branches are `Applied`; the existence post-pass's
  are `Proven`; `emit_existence_constant_branch_diagnostics` reads the
  stored kind and stops running `existence_constant_branches` a second
  time.
- **Preserves**: every I230 and I231, and the `info_exists_*` tests.
- **Changes**: none observable. Mandate: "the existence branch fact
  stored once with its kind"; § *Branch facts*.
- **Tests**: `the_existence_branch_fact_is_stored_once`
  (`analyser/diagnostics/tests.rs`).
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: S. **After**: —.

##### VT5.13 — W100's produced set is the unbraced-expression fact

The diagnostic-policy lane's plan makes the analyser's produced W100
findings the fact both rules read: DP8.1 puts W100 in `FACT_CODES`, so
the production skip never drops it, and DP8.2's `brace_expr_hints` emits
O111 at the span of every produced W100 before policy runs. That is "the
same fact" of the migration plan's slice 5, so this lane adds no second
representation (D23); its part is to prove the produced set covers every
unbraced expression.

- **Files**: `rust/tcl-compiler/src/analyser/diagnostics/tests.rs`.
- **Items**: none in production code; `emit_w100_unbraced_expr` and
  `push_w100` are unchanged.
- **Tests**: `w100_marks_every_unbraced_expression` — `expr $a+1`,
  `expr "$a + 1"`, `if "$x" {…}`, `while $c {…}`, `for {} $c {} {…}` and
  an unbraced `expr` inside a `[…]` each produce one W100 at the
  expression word's span; the braced forms produce none.
- **Gates**: G9.
- **Model**: sonnet. **Size**: S. **After**: —; DP8.1 and DP8.2 consume
  it (B-DP3).

##### VT5.14 — the no-route writers

- **Files**: the spec modules of `file stat`, `file lstat`,
  `foreachLine`, `gets`, `chan gets`, `file tempfile`, `vwait`,
  `tk_optionMenu`, `trace add`, `trace remove`, `trace variable`, `trace
  vdelete`; `rust/tcl-registry/src/value_transfer/builtins.rs`;
  `rust/xtask/src/value_transfers.rs`.
- **Items**: one `MayWriteSemantics { targets: &'static [ArgRole], reason:
  NoRouteReason }` whose transfer is `MayWrite` on each declared target
  and whose route is `EvalRoute::None { reason }` (`PLATFORM` for the
  `file` forms, `Declared` for the channel, event-loop and widget forms);
  `foreachLine` declares an `Iterate` plan over a file source with no
  route; the `trace` subcommands declare `TransferAnswer::Generic` with no
  route.
- **Changes**: the inventory classifies the rows; no lattice value
  changes (each was `Overdefined`).
- **Gates**: G1 (the rows go), G2, G9.
- **Model**: sonnet. **Size**: M. **After**: VT5.1.

##### VT5.15 — one query for a proven word

- **Files**: `rust/tcl-compiler/src/value_transfer.rs`,
  `rust/tcl-compiler/src/compilation_unit.rs`.
- **Items**:
  ```rust
  /// The exact value a word of a statement has at that statement, with
  /// its folded type — the lattice's answer, never a token relabelled as
  /// a literal. `None` when it is not exact.
  pub fn proven_word_value(
      fu: &FunctionUnit,
      statement: StatementId,
      word: usize,
  ) -> Option<(ExactValue, Option<FoldedType>)>;
  ```
- **Tests**: `proven_word_value_reads_the_lattice_at_the_statement`
  (positive through a `set`; negative after a redefinition in a branch).
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: S. **After**: VT5.2.

##### VT5.16 — the literal-only diagnostics read proven values

- **Files**: `rust/tcl-compiler/src/analyser/diagnostics/usage.rs` (W121,
  W200, W202), `security.rs` (W127, W137, W141, W303), `validity.rs`
  (W145, W146, W147, W152), `version_gate.rs` (W138),
  `rust/tcl-compiler/src/analyser/bounds_checks.rs` (the syntactic half of
  W230, W232), `rust/tcl-compiler/src/irules_checks.rs` (IRULE4004),
  `rust/tcl-compiler/src/taint.rs` (`find_setter_constraint_warnings`,
  IRULE3101), `rust/tcl-compiler/src/uri_split.rs` (IRULE3103, the
  `Const(String)`-only read), `rust/tcl-compiler/src/analyser/diagnostics.rs`.
- **Items**: each check keeps its literal path in the walk and records
  the `(span, check)` pairs it abstains on for a non-literal word; the
  per-function pass re-runs exactly those checks over
  `proven_word_value`, so no finding is emitted twice and none is emitted
  at a span the user did not write. W146 feeds the exact value through
  `LiteralArgumentValidator` with its computed provenance.
- **Changes**, each mandated by its row of "Codes that are literal-only
  today and would gain" or "Codes that read the lattice today": W121 on
  `set m 255.0; append m .255.0; IP::addr $ip mask $m`; W127, W137, W141
  on a propagated or `[string tolower CONST]` option value; W145, W147,
  W152 on a computed option name; W146 on a proven value; W303 on `set re
  {(a+)+$}; regexp $re $s`; W230 and W232 on `set l {a b c}; lindex $l
  9`; W138, W200, W202 on a computed template; IRULE4004 on `set x
  [string range CONST 0 3]`; IRULE3101 silent on `set p /a; HTTP::path
  $p` (#2055); IRULE3103 on computed operands.
- **Tests**: one positive (the row's program) and one negative (the same
  shape over an unknown value) per code in `analyser/diagnostics/tests.rs`
  and, for IRULE3101 and IRULE3103, in `rust/tcl-compiler/tests/`'s
  iRules suites; `irule3101_reads_the_proven_path` names #2055's program.
- **Gates**: G4 (a message that names a computed value), G7, G8, G9.
- **Model**: opus. **Size**: L. **After**: VT5.15.

##### VT5.17 — the editor consumers

- **Files**: `rust/tcl-lsp-core/src/hover.rs` (`literal_at_token`),
  `rust/tcl-lsp-core/src/inlay_hints.rs` (`collect_format_string_hints`),
  the semantic-token families for regexp, format, clock and binary
  patterns, `rust/tcl-lsp-core/src/document_links.rs`,
  `rust/xtask/src/value_transfers.rs`.
- **Items**: each reads `proven_word_value` through the memoised unit; a
  computed pattern is explained at its use and never painted at a token
  range it does not have; `document_links.rs`'s `speclib` and `include`
  sites carry `// value-transfer-ok: irreducible — the pack grammar's own
  statements`.
- **Changes**: `set fmt "%-20s %d"; format $fmt a 1` gets hover on
  `$fmt` and the `int:` inlay label. Mandate: the inventory's item 4 and
  the "Literal-only editor features" paragraph.
- **Tests**: `hover_explains_a_computed_format_string`,
  `inlay_hints_label_a_computed_format_string` (`tcl-lsp-core`; negative:
  an unknown `$fmt` gets neither).
- **Gates**: G1 (`document_links.rs` 2 → 0, clean), G7, G8, G9.
- **Model**: sonnet. **Size**: M. **After**: VT5.15; B-DP1.

##### VT5.18 — the container harvesters read structured writes

- **Files**: `rust/tcl-compiler/src/analyser/diagnostics/var_command.rs`,
  `rust/tcl-compiler/src/analyser/diagnostics/helpers.rs`,
  `rust/xtask/src/value_transfers.rs`.
- **Items**: W307 and W308 read the element writes of `set arr(k)`,
  `array set`, `dict set` and `dict with` from their outcomes and plans,
  and `helpers.rs`'s `dict with` / `dict update` key harvest reads the
  plan's binders.
- **Preserves**: every W307 and W308 test.
- **Changes**: a lattice-constant `array set` or `dict set` operand
  harvests its keys. Mandate: the W123, W307, W308 row.
- **Gates**: G1 (`var_command.rs` 3 → 0, clean; `helpers.rs` 6 → 5),
  G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: VT5.5, VT5.7.

##### VT5.19 — the slice's witnesses

- **Files**: `rust/tcl-compiler/tests/value_transfer_witnesses.rs`,
  `rust/tcl-cli/tests/value_transfers_cli.rs`, `rust/tcl-lsp-db/src/value_transfer_parity.rs`.
- **Tests**: `the_no_match_preserve_witness`, `the_partial_scan_witness`,
  `the_repeated_target_witness` (each the interface page's program through
  analysis, the memoised unit and `tcl opt`, whose output prints what
  `tclsh` prints under every release); `explore_sccp_prints_folded_types`
  (CLI); `destructuring_agrees_on_both_paths` (parity).
- **Gates**: G7, G8, G9.
- **Model**: sonnet. **Size**: M. **After**: VT5.1 to VT5.18.

##### VT5.20 — docs and the landing

- **Files**: `pass-fact-ownership-matrix.md` (`folded_types`, the
  outcomes, the template plan, the unbraced-expression fact),
  `downstream-pass-contracts.md`, `sccp-core-analyses.md`,
  `constant-folding-type-inference.md`, `optimisation-passes.md`,
  `precision-limitations.md` (the regexp precision declines),
  `value-transfers-migration.md` (ratchet rows, the dataflow sites'
  status), `value-transfers.md`, `docs/kcs/codes/` notes for W210 and
  W102 (the proven-switch narrowing), a KCS note
  `docs/kcs/compiler/kcs-qa-why-does-a-regexp-sometimes-not-fold.md`
  (new), the lane doc. The rows for `diagnostics-calculation.md` and
  `diagnostics-integration.md` are drafted in the lane doc (B-DP4).
- **Gates**: G4, G5, G6, and the slice's green.
- **Model**: sonnet. **Size**: M. **After**: VT5.19.

#### Checkpoints and landing

| Checkpoint | Holds | Green means |
|---|---|---|
| `wip(value-transfers): slice 5 — outcomes and folded types` | VT5.1, VT5.2, VT5.12, VT5.13, VT5.15 | every existing test byte-identical; the W100 coverage test |
| `wip(value-transfers): slice 5 — the regexp owner` | VT5.3 | the precision witnesses; every runtime regexp test |
| `wip(value-transfers): slice 5 — the destructuring writers` | VT5.4 to VT5.7, VT5.14 | the storage witnesses under every release; G1 without the slice-5 gap rows; G2 |
| `wip(value-transfers): slice 5 — the template-word plan and its consumers` | VT5.8 to VT5.10 | the fourteen template witnesses; every `w102_*`, `tp_*`, `proc_subst_nocommands_*` test |
| `wip(value-transfers): slice 5 — the consumers read the facts` | VT5.11, VT5.16 to VT5.18 | the W210 and literal-only witnesses; G1 pins |
| `wip(value-transfers): slice 5 — destructuring and structured bodies` (landing) | VT5.19, VT5.20 | every exit test; G1 to G9 |

```text
wip(value-transfers): slice 5 — destructuring and structured bodies

A transfer is a result and ordered storage outcomes: write, preserve,
unbind and may-write, validated against the declared targets and applied
per place in execution order, so a repeated target composes and a
no-match keeps its prior value. The regexp owner answers exactly,
no-match, or a typed decline, from the engine up; an exhausted search is
never a no-match. `regexp`, `regsub`, `scan`, `binary scan`, `lassign`
and `array set` are destructuring writers over the shared cores, `binary
format` is a byte array by construction, and `folded_types` carries each
value's type and representation evidence. `dict with`, `dict update` and
the loops' source layout are structural plans. `subst` answers a
template-word plan from its option rows and proven switches, and the
lowering fold, the factory extraction, W102, extract-proc and the
dynamic-name barrier read it instead of walking the template; a
materialised child keeps its factory call's span. W210 reads preserve
outcomes, so the private no-match prover is gone; the existence branch
fact is stored once with its kind; W100's produced set is proven to mark
every unbraced expression, the fact O111 reads; the literal-only
diagnostics and the editor's
hover, inlay hints, semantic tokens and links read proven values.

Behaviour changes: program (2) folds; destructuring writers reach the
lattice; a regexp search that exhausts its fuel raises at run time
instead of answering 0; W102 narrows a proven switch word; `subst
-novariables $t` no longer blinds the function's reads; the literal-only
checks fire on proven values; IRULE3101 reads the proven path; a
materialised proc's findings anchor at its factory call.

Closes #2055. Pins #2051 (closed on rust by #2225). Closes #2143, or
"Pins the span half of #2143" (Q3).
```

#### Review checklist

- R1: `CLEAN_FILES` gains `var_command.rs`, `specialise_factories.rs`,
  `document_links.rs`; `dataflow.rs` at 2 and `helpers.rs` at 5; no file
  outside `value_transfer/template.rs` and the driver's `word_structure`
  segments a template word; no consumer matches `"regexp"`, `"scan"`,
  `"subst"`, `"lassign"` or `"dict"`.
- R2 to R5; new files: `regex.rs`, `destructure.rs`, `body.rs`,
  `template.rs` (registry), `precision_oracle.rs`.
- R6: expectations move only for the witnesses named in each item.
- Suites: `cargo test -p tcl-regex -p tcl-cmd-core -p tcl-vm -p
  tcl-registry -p tcl-compiler -p tcl-explorer -p tcl-lsp-core -p
  tcl-lsp-db -p tcl-cli -p xtask`.
- R7: a preserve outcome never invents a prior value (a pending prior
  stays pending); an approximate capture never becomes a value; a
  repeated target composes in order; an error completion is still a
  decline in this slice (slice 10 owns the prefix rule); a switch word
  that is not proven answers every kind; the mirror-pairs witness gains
  its `CorrelatedSets` reason once the two-binder `foreach` source is
  lowered (D64).

#### Behavioural deltas

| Delta | Mandate |
|---|---|
| program (2) folds and is a byte array; S100 / S110 report no conversion for it | exit; the S-rows |
| `regexp`, `regsub`, `scan`, `binary scan`, `lassign`, `array set` write or preserve their targets in the lattice | the Storage row; § *Storage-writing commands* |
| an exhausted regexp search raises at run time instead of answering 0 | § *The typed precision result*, step 3 |
| W210 follows declared preserve outcomes | "W210 consuming preserve outcomes" |
| W102 narrows a proven switch word; the barrier stops blinding reads for `subst -novariables $t` | the W102 and dynamic-name rows |
| the literal-only codes fire on proven values; IRULE3101 is silent on a proven `/` path | the literal-only table; #2055 |
| hover and inlay hints explain a computed format string | the inventory's item 4 |
| a materialised proc's findings anchor at its factory call | #2143 |

#### Record (2026-09-23): the opus items of slice 5

One implementer runs the opus items in the order the coordinator gave —
VT5.1, VT5.2, VT5.3, VT5.4, VT5.5, VT5.6, VT5.7, VT5.12, VT5.11, VT5.15,
VT5.16, VT5.18 — and reports at the VT5.8 gate, which waits for the
consumer-contracts lane's CC2.6 (`option_effects` and the
`substitutions_performed` projection); VT5.8 to VT5.10 follow it. The
sonnet items (VT5.13, VT5.14, VT5.17, VT5.19, VT5.20) are not this
implementer's. The decisions are D105 to D109 and D115 onward in §
*Decisions taken* (D110 to D114 are the review of slice 4's); this table
gains a row per item as it lands. Each item is its own checkpoint commit
rather than the plan's grouped checkpoints, so a restart loses at most one
item; the grouping stays the plan's account of what lands together.

| Item | Commit | What landed | Its tests |
|---|---|---|---|
| VT5.1 | `wip(value-transfers): slice 5 — every outcome kind, applied per place` | `validate_outcome` (`answers.rs`): every store names a declared target — the driver's `CommandSemantics::store_targets` (the `VarWrite` operands and a cell-update plan's target by default; a pack's `stores -targets` for `DeclaredSemantics`, D105) — each target has one outcome at most, the type facts name only targets, and an error completion lists no more stores than ran; the driver's `apply_outcome` replaces `store_def` (D26's shape): each store resolves to its place, a repeated place composes in execution order (the last write wins, a `Preserve` keeps what the place holds at that point, a `MayWrite` or `Unbind` widens), an element beside its array's base declines `OverlappingTargets`, a traced place declines `TracedPlace`, an escaping one widens its own definition (D106), a non-normal completion declines (D107); the lifted members join per definition with a pending member keeping it pending (D108); SCCP evaluates a statement once per sweep for every definition (`DefValues`), so a call that defines several variables takes each one's own value (D108); `PlaceRef::{is_element, base, shares_storage_with, overlaps_as_element_and_base}`, which `declared.rs` now reads in place of its private helper (D109) | `validate_outcome_rejects_a_store_to_a_non_target` (`value_transfers.rs`); every `evaluate_def_*`, `sccp_*` and witness test byte-identical |
| VT5.2 | `wip(value-transfers): slice 5 — folded types and representation evidence` | `FoldedType { intrep, shape, representation }` (`value_transfer.rs`) and `SccpResult::folded_types`: the driver's per-definition answers carry the folded type their evaluation states (`DefAnswer`; a result's from its type facts and value, a place's from the stores to it in order, a copy's from its source), SCCP records the settled sweep's, joins a φ's by agreement and forgets them at a barrier (D115); the Explorer's `sccp` view shows each (`h#1 = const(…)` · `type: bytearray (constructed)`); the shimmer purity read (`is_pure_value`, `is_free_first_conversion`, the commit facts' initial state) reads representation before the literal rule, so a computed constant no longer hides a conversion (D116); `type_infer` takes a folded type where its static typing knows nothing (D117); S110 takes a constructed byte array as a byte source (D118); `find_shimmer_warnings` and `find_byte_array_warnings` take the function's `SccpResult`, and the use-site and expression passes its `CommitCtx` | `folded_types_state_what_each_route_constructed` (witnesses); `serialise::tests::sccp_reports_folded_types`; `a_computed_constant_never_hides_a_conversion` (use-site S100, tclsh 8.6 to 9.1 checked); `a_folded_type_refines_only_what_the_static_typing_leaves_unknown`; `constructed_byte_array_evidence_is_a_byte_source` |
| VT5.3 | `wip(value-transfers): slice 5 — the regexp owner` | the engine answers three ways (`tcl_regex::ExecOutcome`: `Matched`, `NoMatch`, `Stopped(ExecStop::{Fuel, Depth, Cancelled})`), `Regex::exec` and `Regex::exec_with(…, &ExecLimits { fuel, cancel })`, the token read where the fuel is charged (D119); dissection walks a repeat's iterations and a concatenation's items in loops with a backward finish table, skips a subtree without a capture, and stops rather than approximates past its depth cap, the unbounded repeat's reach is a worklist closure, and the backtracker matches a single character's repeat and a literal run in loops (D120, D121); the plumbing's `RegexpPrecision<RegMatch>` and `PrecisionDecline`, verbatim, `RegexEngine::exec` returning it with `exec_within`, `IDENTITY` and `retained_bytes`; `regexp`, `regsub`, `switch -regexp`, `lsearch -regexp` and the VM's match helper raise a decline (`error while matching regular expression: …`), the C API returns `REG_ESPACE`, and `regexp_analysis` / `regsub_analysis` keep it typed (D122); the thread's `PatternCacheKey`-keyed cache, 4 MiB of retained bytes, coldest first, a compile charged its length squared through `AnalysisMatch::charge` (D123) | `the_three_precision_witnesses` and `an_exhausted_search_is_never_a_no_match` (`precision_oracle.rs`, the page's table under tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1b0, all five agreeing); `a_stopped_search_is_raised_at_run_time_and_typed_in_analysis`, `the_pattern_cache_charges_a_compile_once_per_key`, `the_pattern_cache_stays_within_its_byte_bound` (`tcl-cmd-core`); `capture_past_the_old_dissect_cap_is_exact` replaces the approximate-span test; every other `tcl-regex` (the `reg.test` corpus included), `tcl-cmd-core`, `tcl-vm` and `runtime/rust` regexp test unchanged |
| VT5.4 | `wip(value-transfers): slice 5 — regexp and regsub` | `RegexpSemantics` and `RegsubSemantics` (`value_transfer/regex.rs`, `NativeEvalId::{RegexpMatch, RegsubSubstitute}`, registry-owned) over `regexp_analysis` / `regsub_analysis` and `AreEngine`, declared on both specs: a match writes one value per match variable (an unmatched subgroup the empty string, `-1 -1` with `-indices`), a completed no-match, `-inline`, `-about` and a variable-less `regsub` preserve every declared target, `-all` counts, `regsub` with a variable writes it whether or not anything matched, and every `PrecisionDecline` declines the whole answer (`Approximate`, `Unsupported`, the cancelled budget, the command's error) (D126); `-about` is evaluated and the `-command` form is `NoRoute(Callback)` (D124); the axes gain `LIST_RENDERING` and a `-start` index evaluates only as a plain decimal integer (D125); the engine's work is metered (`Regex::exec_metered`, `MatchLimits::spent`) and charged to the evaluation's budget with the published bytes (`ConstOps::remaining_work`, `ConstOps::take_all`) (D127); `regsub`'s folders are the route through `evaluate_literal`, `const_fold_versioned` new, `CONST_FOLD_VERSIONED_NATIVE` at 4 rows (D128); G1's `regexp` and `regsub` gap rows gone, the inventory regenerated; the page's two core-table rows and its table count updated. Not done: the `RegexPatternCapture` hook's retirement and `handle_regex_pattern_capture` wait for CC2.13 (D129) | `regexp_writes_or_preserves_its_match_variables`, `a_regexp_that_established_nothing_declines`, `regsub_writes_its_variable_and_declines_its_callback` (`value_transfers.rs`); `regexp_witnesses_match_every_release_on_path` (`differential_fold.rs`: 22 witnesses, 18 answered and agreeing on each of tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1b0, the plan's five answered on every one); `a_search_reports_the_work_it_spent` and `searches_charged_to_one_counter_share_one_budget` (`precision_oracle.rs`); `route_stamps_match_the_pinned_set` gains the two stamps; changed by the mandate ("a no-match `regexp` keeps its match variables' values in the lattice; a match writes them"): `a_conditional_writer_does_not_kill_the_store_it_may_preserve` asserts each preserved store survives by line and that `puts "$a $b"` now reads `before before`, and `var_write_typing_shapes_destructure_target_types` reads the registry's typing over an unknown subject and the written `String` over a literal one |
| VT5.5 | `wip(value-transfers): slice 5 — scan, binary scan, lassign, array set` | `ScanSemantics`, `BinaryScanSemantics`, `LassignSemantics` and `ArraySetSemantics` (`value_transfer/destructure.rs`, `NativeEvalId::{ScanFormat, BinaryScan, ListAssign, ArraySet}`, registry-owned) over `scan::validate_format` / `scan_match`, `binary::scan` and `ConstOps::list_elements`, declared on the four specs: a converted field writes its variable typed as it was built and a field the input did not reach preserves it (`-1` and every variable preserved when the input ended first), `lassign` writes in order and returns the rest, `array set` writes one element per key; each answers only where every release the target names reads the words alike (D132); the writing routes share `value_transfer/publication.rs` (`open_words`, `targets_are`, `PendingStore`, `Publication`, moved out of `regex.rs`, D134); `StoreOutcome::WriteElement` names an element of an array target by key (D130), and SCCP takes a stated element write for a fanned may-write rather than joining it with the prior (`DefAnswer::stated`, D131); the scan and unpack work is charged per byte, the page's two core rows updated; G1's four gap rows gone and the inventory regenerated; the oracle harness spells non-ASCII as `\uXXXX` (D133) | `destructuring_writers_run_the_shared_cores` (`value_transfers.rs`, with the `%u`, infinity and negative-zero declines); `destructuring_witnesses_match_every_release_on_path` (`differential_fold.rs`: 46 witnesses, the plan's four answered and agreeing on every release that has the command; answered and agreeing 21 on tclsh 8.4.20, 30 on 8.5.19, 32 on 8.6.18, 35 on 9.0.4 and 35 on 9.1b0, every other one declined by the route or raised by `tclsh`); `an_array_set_writes_the_elements_it_names` (`value_transfer.rs`); `route_stamps_match_the_pinned_set` gains the four stamps; `var_write_typing_shapes_destructure_target_types` (`type_infer.rs`) reads a written `scan` target as the `String` its conversion built over literal operands and keeps the unknown subject's case |
| VT5.6 | `wip(value-transfers): slice 5 — binary format, program (2)` | `BinaryFormatSemantics` (`value_transfer/builtins.rs`, `NativeEvalId::BinaryFormat`, registry-owned) over `binary::format`, declared on the `binary format` subcommand: the packed bytes as the characters `U+0000` to `U+00FF`, `RepresentationEvidence::Constructed(ByteArray)`, the result type `ByteArray`; `binary::format_size_bound` (the page's proposed core, built beside the packer) charged as allocation before the run and the output per byte after it; only what every release packs alike answers (D136); a rewrite never writes a computed byte array into the source — `SccpResult::materialises`, read by O100's two projections, O103's return read, O127's skip and the chain folds, so the optimised program keeps the command that builds it (D135); the page's two rows updated; the inventory regenerated | `program_two_folds_and_is_a_byte_array` (compiler witnesses: `h#1` is `ABCDEF` with the folded type `bytearray (constructed)` under `tcl8.4`, `tcl8.6`, `tcl9.0`, `f5-irules` and `tcl`, no S100, S101 or S110, no rewrite spells `ABCDEF` or a returned `GH`, and the original and optimised programs print the same under tclsh 8.4 to 9.1); `binary_format_witnesses_match_every_release_on_path` (`differential_fold.rs`: 49 witnesses; the 24 every release packs alike answered and agreeing on each of tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1b0, the non-ASCII argument from 9.0 and the 8.5 fields from 8.5, every spelling the releases part on declined); `format_size_bound_covers_the_packed_output` (`tcl-cmd-core`); `route_stamps_match_the_pinned_set` gains the stamp; `tcl explore --source 'set h [binary format H* 414243444546]' --show sccp --text --no-colour` prints `h#1 = const('ABCDEF')` · `type: bytearray (constructed)` |
| VT5.7 | `wip(value-transfers): slice 5 — structural plans` | `DictWithSemantics` and `DictUpdateSemantics` (`value_transfer/body.rs`, new) answering `PlanAnswer::Body` — the binders a projection on body entry (the proven keys of the dictionary, or of the nested one a key path names, for `dict with`; the declared variables for `dict update`), the body in the caller's frame, `Reconcile::WriteBackKeys` of the dictionary operand, `CompletionProtocol::TclBody`, no route — declared on both subcommands and so on their `::tcl::dict::` spellings (D137); `IterationSemantics` answers the source layout (one binder per var-list name, one list, the body, `break` and `continue` absorbed, nothing bound on zero iterations) (D138); the driver's loop header binds each binder the elements it takes, so a two-binder `foreach` is two finite inputs (D139); G1's four `dict` body rows gone and the inventory regenerated. Not done: the `DictWith` hook's retirement and `handle_dict_with_command` reading the binders wait for CC2.13 (D140) | `dict_with_binds_the_proven_keys`, `the_source_layout_answers_an_iteration_plan` (`value_transfers.rs`); `dict_with_binds_the_keys_tclsh_binds` (`differential_fold.rs`: the variables five dictionaries and key paths bind on body entry are the plan's binders, and the page's program answers `done` and leaves `d` as `a 2`, on tclsh 8.5.19, 8.6.18, 9.0.4 and 9.1b0); `a_loop_header_binds_each_binder_its_elements` (`value_transfer.rs`); `the_mirror_pairs_decline_as_correlated` asserts both quotients decline `correlated-sets` (D64's deferral closed); `route_stamps_match_the_pinned_set` gains the four body stamps; `the_loop_header_projects_to_the_declared_iteration_plan`'s source-layout row reads the new answer (a call without its body is the command's error, where it read "not yet described") |
| VT5.12 | `wip(value-transfers): slice 5 — the branch fact records its kind` | `BranchFactKind { Proven, Selected, Applied }` (`sccp.rs`) on `ConstantBranch`: the solver's decided branches are `Applied`, the existence post-pass's folds `Proven`, and `Selected` waits for slice 6's selection record; `emit_constant_branch_diagnostics` reads the `Applied` facts and `emit_existence_constant_branch_diagnostics` the `Proven` ones, and no longer reruns `existence_constant_branches` (the analyser's `BodyFrame::existence_frame` went with its one caller); `compiler_checks.rs` already reported every stored fact and needed nothing (D141) | `the_existence_branch_fact_is_stored_once` (`analyser/diagnostics/tests.rs`: the kinds as stored, one I230 per stored fact, a unit whose proven fact is removed reports nothing for it, and under iRules no I230 for a variable another event sets); every other I230, I231 and `info_exists_*` test unchanged |
| VT5.11 | `wip(value-transfers): slice 5 — W210 reads preserve outcomes` | `SccpResult::preserved` (`sccp.rs`): each definition its statement left untouched — every store the evaluated outcome makes to its place a `Preserve` (`DefAnswer::preserved`), a pack command's declared preserve included — with the version its place held before the statement, and each definition a condition's `<cond>` statement reads first when the shared engine decided that condition, every nested command it ran having answered without a store (D142); the undef trace reads through a preserved definition to that version (`PhiUndefCtx::preserved`), so W210 on a read, a `return` or a condition's no-match arm, and W213 on an `unset`, come from the general read-before-set pass, counted only in executable blocks and past the name-level condition-write suppression the fact answers (`UndefSuppression::preserved_undef`, `suppresses_read`) (D143); the private prover is gone — `emit_provably_unset_w210`, its embedded-condition walk, `regexp_scan_no_match`, `skip_options` and the literal-substring matcher — G1's `dataflow.rs` 4 → 2 with its ledger row, `registry-axes`' 17 → 9; a conditional writer nested in a word or a condition reads its targets as the statement form does (`ir_helpers::variable_write_effects_from_commands`), so the optimiser keeps the store a no-match preserves there too (D144) | `w210_reads_a_no_match_preserve_outcome` (`analyser/diagnostics/tests.rs`: W210 at `$a` after `regexp {(x)(y)} zz a b`, on `return $b`, and in `if {![regexp {x} y -> v]}`'s arm; none after a matching subject or a `set` before the call, or in the arm when `v` was set first — each checked on tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1b0); `a_no_match_keeps_the_store_it_preserves` (compiler witnesses: #2051's program and the condition and word forms keep `set … before` under every dialect, no O109, no W210 or W220 naming it, and the original and optimised programs print the same under tclsh 8.4 to 9.1); `a_pack_declared_preserve_holds_the_prior_version` (a `write_or_preserve` pack command's preserved definition names the root in one procedure and the `set` in the other); changed by the mandate ("W210 follows any registry-declared preserve outcome"): `fp_sty_11_binary_scan_many_vars_no_false_w210` reads an input that fills its twenty fields, and its empty input is now the true positive tclsh reports (`can't read "t"` on every release); every other `w210_*`, `emit_cfg_ssa_diagnostics_w210_*`, `fp_rbs_02_*`, `fp_sty_10_*` and `scan_predicate_w210_*` test unchanged |
| VT5.15 | `wip(value-transfers): slice 5 — one query for a proven word` | `value_transfer::proven_word_value(fu, statement, word, config)`: the exact value a call's word has at that statement from the lattice alone — a literal word's text; a substituted word's decoded runs and variable reads at the statement's use versions, concatenated; the folded type its definition states for a whole-word read — and `None` for a command substitution, a finite or unknown read, an expansion, a respelled word or an unreached block; `StatementId { block, index }` and `FunctionUnit::word_at(span)`, the address a consumer holding a word's source range reads it at (D145) | `proven_word_value_reads_the_lattice_at_the_statement` (`value_transfer.rs`: `$f` after `set f %d` is `%d`, never `$f`; `$n` after `set n [string length abc]` is `3` typed int; `"x$f"` is `x%d`; `7` is itself; a `[…]` word and the command word are `None`; `$f` after an `if` redefines it is `None`) |
| VT5.16 | `wip(value-transfers): slice 5 — the literal-only diagnostics read proven values` | the walk keeps each call with a word a literal-only check could not read — a variable read, or a substituted word with no command substitution (`ProvenSite`, `analyser/diagnostics/proven.rs`, recorded by one hunk in `commands.rs`); once the unit exists the pass substitutes each word the lattice proves at its statement (`proven_word_value`, through an index of the unit's call words by span) and runs the checks again: W121, W127, W137, W138, W145 (subcommand and option words), W146, W200, W202 and W303 keep only what they report at a proven word, with no fix; W147 and W152 evaluate the declared relations over the proven option words and keep what the written words did not draw; W230 and W232 keep what the index checks draw over a proven list or string and did not draw over the written one; a verdict the arity flush settles (W146, W147, W152) settles through the same user-resolution rule (`settle_builtin_verdicts`, factored out of `flush_arity_diagnostics`) (D146); IRULE4004 hoists a value that reads no variable and that the lattice proves; IRULE3101 checks a proven setter value as a literal — `find_setter_constraint_warnings` takes the unit's values, and `tcl-lsp-core`'s `graphs.rs` passes them; IRULE3103 reads any proven constant, a condition's variable operand included (D147); W141 and a computed subcommand word stay out of reach (D148) | `literal_only_checks_read_proven_words` (`analyser/diagnostics/tests.rs`: for each of W121, W127, W137, W138, W145, W146, W147, W152, W200, W202, W230, W232 and W303, the row's program reports at the proven word, or over the relation's options, or at the literal index, and the same call over a parameter draws nothing); `a_proven_word_is_reported_once_beside_a_written_one`; `irule4004_proven_command_value_is_hoistable` (`irules_checks.rs`); `irule3101_reads_the_proven_path` (#2055's program is clean, a proven `a` still warns) and `irule3103_reads_a_proven_operand` (`tests/taint.rs`); changed by the mandate (#2055): `irule3101_pure_var_ref_always_warns_without_safe_colour` warns over a value two arms set, and its proven `/safe` is clean; every other test of these codes unchanged |
| VT5.18 | `wip(value-transfers): slice 5 — the container harvesters read structured writes` | W307's constant sets read the writes each statement states and the plans rather than the spellings: `var_command.rs`'s three harvesters go — a literal `set arr(k) v` is the lowering's own element assignment, `array set` (and any route's element write) is the `WriteElement` outcome its registry route states over the call's literal words (`value_transfer::literal_element_writes`), and a `dict with` binds the keys `DictWithSemantics` declares over the dictionary the lattice holds at the version the statement reads, with each key's value at the plan's key path (D150); `helpers.rs`'s W210 key harvest finds a dictionary body by its plan whatever the dictionary holds (`value_transfer::dict_body_operand`, over a probe dictionary that holds the key path the plan reads) and reads the plan's binders over a known one (`value_transfer::dict_body`): each key `dict with` binds at its key path, and each `dict update` variable whose literal key the dictionary holds (D149); G1's `var_command.rs` 3 → 0 (clean) and `helpers.rs` 6 → 5 with the ledger's rows, `registry-axes`' 12 → 7 and 10 → 6 | `a_dictionary_body_is_found_by_its_plan` (`value_transfer.rs`: both spellings, a key path over an unknown dictionary, a repeated key's last value, a path the dictionary lacks, and `dict update`'s literal and dynamic keys); `w210_dict_body_keys_come_from_the_plan` (`analyser/diagnostics/tests.rs`: a key path binds its nested keys, an unknown dictionary's key path stays unknown shape, `::tcl::dict::with` is the same plan, and `dict update` binds only a present key — each program's answer checked on tclsh 8.5.19, 8.6.18, 9.0.4 and 9.1b0); `w307_reads_element_and_body_bindings` (a key path's nested key, a lattice-constant `array set`, and a literal `array set` or `set arr(k)` in a function with a barrier, each silent over `puts` and firing over a non-command); changed: `dict with d a {…}` over `{a {x 1}}` binds `x`, where the spelling harvest bound `a`, so the false W210 on `$x` after it and the false W307 on a `$cmd` dispatch inside it are gone; every other W210, W307 and W308 test unchanged |
| VT5.8 | `wip(value-transfers): slice 5 — the template-word plan` | `TemplateSemantics` (`value_transfer/template.rs`, new; `template:subst`, route none) declared on `subst` over its own switch table: `structure` answers `PlanAnswer::TemplateWord` — the kinds from `option_effects` over each switch's proven value (an exact value its spelling, a finite set joined per member, an unproven switch every kind), read at every release the profile names, a spelling that raises contributing nothing, the releases reading one differently `ReleaseAmbiguous(Availability(9.1))`, every spelling raising `WrongRepresentation` (D151); `braced`, `dynamic`, the script regions (caller's frame), the reads outside them and the escapes from the template's word structure decomposed under the kinds, an array index substituting every kind whatever the switches say, every span an offset into the word, and a template `subst` rejects the command's error (D152); the driver records one `TemplatePlanRecord { span, plan }` per executable trusted `subst` call over the settled lattice (`SccpResult::template_plans`, the template word's token span), which `rebase_function_unit` shifts; the inventory's `subst` row declared | `the_template_plan_answers_the_fourteen_witnesses`, `the_positive_switches_are_9_1s`, `a_template_plan_joins_proven_switches_and_reads_indexes` (`value_transfers.rs`: the page's programs as plan fixtures under `tcl8.4` to `tcl9.1` and the spanning `tcl`); `template_witnesses_match_every_release_on_path` (`differential_fold.rs`: the fourteen programs and four more — an array index, `-nobackslashes` over `\$`, and an unclosed bracket with and without `-nocommands` — under tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1b0 — where tclsh raises the plan is the command's error, where it answers the kinds are the ones tclsh runs, probed one at a time, and the output rebuilt from the plan's escapes, reads and regions is the output tclsh prints; the three 9.1-only rows answer on 9.1b0 alone); `a_subst_call_records_its_template_plan` (`value_transfer.rs`: `set opt -novariables; subst $opt {hello $name}` reads no `name`); `route_stamps_match_the_pinned_set` gains `subst`; `rebase_shifted_unit_spans_match_fresh` carries a `subst` and fails without the shift; the `tp_*` and `fp_*` tests in `substitution.rs` unchanged |
| VT5.9 | `wip(value-transfers): slice 5 — the template folders read the plan` | both `subst -nocommands` folders — `eval_subst_nocommands_body` in lowering and `extract_subst_nocommands_template` in `specialise_factories.rs` — ask the registry for the template-word plan over the call's literal words (`value_transfer::literal_template_plan`; `LiteralInputs::with_braced` answers a brace-quoted word's structure, since lowering runs before SSA) and act only on `kinds == SUBST_NOCOMMANDS_KINDS` over a braced template, so neither matches the head `subst` by spelling any more; `subst_nocommands` renders the plan's `reads` and `escapes` over the const-map instead of scanning the template, refusing an element read, a qualified name or any script region; a profile-less question reads every switch, as the registry's own surface-blind query does (D153); `register_synthesised` records the factory call's statement span on the materialised child, so its findings anchor at the call — the span half of #2143; G1's `specialise_factories.rs` 1 → 0, its `proc` definer site waived `definition_body`, the file clean; `registry-axes`' `lowering/mod.rs` 26 → 25 and `specialise_factories.rs` 2 → 1 | `a_materialised_child_carries_its_factory_call_span` (compiler witnesses: the #2143 program's W214 for `::port` on line 4 under `tcl8.4`, `tcl8.6`, `tcl9.0` and `tcl`, where it was at 1:1; `port ignored` prints `8080` under tclsh 8.4 to 9.1 before and after `tcl opt`); `the_positive_switches_are_9_1s` gains the profile-less question; the `subst_nocommands` unit tests and `var_escape_typeinfer.rs`'s read through the plan with their answers unchanged; `proc_subst_nocommands_body_materialised`, `proc_subst_nocommands_missing_var_skips_materialisation`, `proc_subst_nocommands_nobackslashes_refused`, `detects_*` and `rejects_factory_with_computed_subst_switch` unchanged |
| VT5.10 | `wip(value-transfers): slice 5 — W102, extract-proc and the barrier read the plan` | W102 reads the call's template-word plan — its kinds, `dynamic` and template word — over the source words in the walk (`value_transfer::literal_template_plan` with `SourceWord`: a substituted word unproven, so a computed switch runs every kind), and the per-function pass re-reads each call the unit has a record for over the lattice and replaces the walk's finding at the template word, so a proven switch narrows the warning and its advice as the literal spelling does (`emit_w102_template_plans`; `TemplatePlanRecord` gains the command's spelling and the proven switch spellings the advice reads) (D154); the dynamic-name barrier sets `reads` only when the plan's template is `dynamic` and variable or command substitution runs over it — `subst -nocommands -novariables $t` stops blinding, `subst -novariables $t` keeps it, since its `[set x]` reads — and scans each script region as script in the frame (`scan_template`; `template_word_is_substituted` gone) (D155); extract-proc keeps or cuts a braced word by the plan's `kinds.variables` and takes its `script_regions` as the same-frame regions (`refactor::source_word`), a substituting command with no plan keeping the registry's answer | `w102_narrows_a_proven_switch_word` (`analyser/diagnostics/tests.rs`: `set opt -novariables; subst $opt $x` reports what `subst -novariables $x` reports, a parameter switch keeps every kind and no advice, proven switches turning both kinds off report nothing); `a_computed_template_blinds_reads_while_it_substitutes` (`dynamic_names.rs`: `-nocommands -novariables` clear, `-novariables`, `-nocommands` and no switch blinding, a braced template's region scanned); `a_computed_template_that_runs_commands_keeps_the_stores_it_reads` (compiler witnesses: the D155 program keeps `set x 1` under `tcl8.4`, `tcl8.6`, `tcl9.0` and `tcl`, and prints `1` under tclsh 8.4 to 9.1 before and after `tcl opt`); every `w102_*`, `subst_injection` and `tp_*` extract-proc test unchanged |

A container restart ended the first implementer at VT5.5, uncommitted;
a second implementer took the opus items over from VT5.5 on (VT5.5, VT5.6,
VT5.7, VT5.12, VT5.11, VT5.15, VT5.16, VT5.18, then VT5.8 to VT5.10, CC2.6
having landed at `e67ae23e`). What it recovered from the worktree was
VT5.5 nearly whole: the four routes, `publication.rs`, `WriteElement`, the
driver's element place, both registry tests and the gate rows. Kept: all of
it. Changed before the checkpoint: the element writes did not reach the
lattice, since SCCP joined each fanned may-write with its prior (D131);
three `scan` answers were wrong against every oracle from 8.5 (`scan -1
%u`, `scan -inf %f`, `scan -0 %f`) and now decline (D132); the harness
compared non-ASCII witnesses against a misread script (D133); the two doc
comments the oracle contradicted were corrected, and the per-byte charges
the page states were added. Nothing was backed out.

Green at VT5.10:

- tests: `tcl-registry`, `tcl-compiler`, `tcl-lsp-core` and `tcl-cli`
  together 14664 passed, 6 ignored, no failure — no existing expectation
  moved; `samples_optimiser_profiles_are_regenerated` among them, no
  sample moved; after the barrier test's rename (D155), `dynamic_names`'
  45 unit tests again;
- pedantic clippy (`--no-deps --all-targets -D warnings`) on
  `tcl-registry`, `tcl-compiler` and `tcl-lsp-core`, no `#[allow]` added;
  `rustfmt` on the touched files;
- `cargo xtask value-transfers` (no pin moved, no generated page changed)
  and `--check` (19 clean, 14 waived, 91 pinned across 37 files, 6607
  rows), `registry-axes --check` (1070 pinned across 163 files),
  `pack-goldens --check` (24 packs, no snapshot moved), the test-binary
  shard verifier's self-test, `cargo check --workspace` clean.

Left, inside this item's file list: W309's gate,
`inner_head_performs_substitution`, still asks only whether the nested
command performs substitution at all. A `subst` that runs backslashes
alone still decodes `\x5b` into a `[` the outer command runs (`set x
{\x5bputs hi\x5d}; eval [subst -nocommands -novariables $x]` prints `hi`
on tclsh 8.4, 8.6 and 9.0), so reading the plan would clear W309 only
for `subst -nobackslashes -nocommands -novariables`, which substitutes
nothing; the item's Items and Changes ask nothing of W309.

Green at VT5.9:

- tests: `tcl-registry`, `tcl-compiler`, `tcl-lsp-core` and `tcl-cli`
  together 14661 passed, 6 ignored, no failure — no existing expectation
  moved; `samples_optimiser_profiles_are_regenerated` among them, no
  sample moved;
- pedantic clippy (`--no-deps --all-targets -D warnings`) on
  `tcl-registry`, `tcl-compiler` and `xtask`, no `#[allow]` added;
  `rustfmt` on the touched files;
- `cargo xtask value-transfers` (`specialise_factories.rs` 1 → 0, clean)
  and `--check` (19 clean, 14 waived, 91 pinned across 37 files, 6607
  rows), `registry-axes` (`lowering/mod.rs` 26 → 25,
  `specialise_factories.rs` 2 → 1) and `--check` (1070 pinned across 163
  files), `cargo check --workspace` clean.

Green at VT5.8:

- tests: `tcl-registry`, `tcl-compiler`, `tcl-lsp-core` and `tcl-cli`
  together 14660 passed, 6 ignored, no failure — no existing expectation
  moved; `samples_optimiser_profiles_are_regenerated` among them, no
  sample moved; after the rejected-template decline, the registry's
  template tests and the differential (now eighteen programs) again;
- pedantic clippy (`--no-deps --all-targets -D warnings`) on
  `tcl-registry` and `tcl-compiler`, no `#[allow]` added; `rustfmt` on
  the touched files;
- `cargo xtask value-transfers` (the inventory's `subst` row declared,
  `template:subst`, route none) and `--check` (18 clean, 13 waived, 92
  pinned across 38 files, 6607 rows), `registry-axes --check` (1072
  pinned), `pack-goldens` (G2 for the new declaration: 24 packs, no
  snapshot rewritten), `cargo check --workspace` clean.

Found and left, outside this item: `tcl_lexer::word_parts::decompose`
scans an array index under the caller's flags, where `Tcl_ParseVarName`
scans it with every kind, so under `-nocommands` the `[b]` in `$a([b])`
comes back as text; the plan rescans each index itself.

Green at VT5.18:

- tests: `tcl-compiler`, `tcl-lsp-core` and `tcl-cli` together 13434
  passed, 6 ignored, no failure — no existing expectation moved;
  `samples_optimiser_profiles_are_regenerated` among them, no sample
  moved;
- pedantic clippy (`--no-deps --all-targets -D warnings`) on
  `tcl-compiler` and `xtask`, no `#[allow]` added; `rustfmt` on the
  touched files;
- `cargo xtask value-transfers` (`var_command.rs` 3 → 0, clean;
  `helpers.rs` 6 → 5) and `--check` (18 clean, 13 waived, 92 pinned across
  38 files, 6607 rows), `registry-axes --check` (`var_command.rs` 12 → 7,
  `helpers.rs` 10 → 6; 1072 pinned across 163 files), `pack-goldens
  --check` (24 packs, no snapshot moved), `cargo check --workspace` clean.

Found and left, outside this item: `harvest_table_command_value_spans`
(the dispatch-table references rename follows) still spells `set arr(k)`,
`array set` and `dict set` — in a tuple match the G1 scan does not
recognise, so the file reads clean with it — because it needs each
value's token span, which no outcome carries; the W210 harvest's
same-block literal fallback reads through an intervening write of the
dictionary (`set d {a 1}; dict unset d a; dict with d {puts $a}` binds
`a`, where tclsh raises `can't read "a"`), since the lattice that knows
the version the statement reads is widened by the `dict with` barrier;
and `::tcl::dict::with` draws W002 ("disabled in the active dialect
profile") and a W211 on its dictionary under `tcl8.6`, where `dict with`
draws neither — the analyser's own reading of the qualified spelling.

Green at VT5.16:

- tests: `tcl-compiler`, `tcl-lsp-core` and `tcl-cli` together 13431
  passed, 6 ignored, no failure — the one expectation the mandate moves,
  `irule3101_pure_var_ref_always_warns_without_safe_colour`, pinned
  #2055's false positive (`set p /safe; HTTP::uri $p`) and is restated
  over a value two arms set, with the proven `/safe` clean;
  `samples_optimiser_profiles_are_regenerated` among them, no sample
  moved;
- pedantic clippy (`--no-deps --all-targets -D warnings`) on
  `tcl-compiler` and `tcl-lsp-core`, no `#[allow]` added; `rustfmt` on
  the touched files;
- `cargo xtask value-transfers --check` and `registry-axes --check`
  unchanged (the new `proven.rs` has no site on either); `cargo check
  --workspace` clean.

Green at VT5.15:

- tests: `tcl-compiler`'s unit tests, the witness binary and the two
  architecture binaries together 6536 passed, 2 ignored, no failure (the
  item adds an entry point nothing else calls yet);
- pedantic clippy (`--no-deps --all-targets -D warnings`) on
  `tcl-compiler`, no `#[allow]` added; `rustfmt` on the touched files;
- `cargo xtask value-transfers --check` and `registry-axes --check`
  unchanged; `cargo check --workspace` clean.

Green at VT5.11:

- tests: `tcl-compiler`, `tcl-lsp-core` and `tcl-cli` together 13425
  passed, 6 ignored, no failure (`samples_optimiser_profiles_are_regenerated`
  among them, no sample moved), and `xtask`'s own;
- pedantic clippy (`--no-deps --all-targets -D warnings`) on
  `tcl-compiler` and `xtask`, no `#[allow]` added; `rustfmt` on the
  touched files;
- `cargo xtask value-transfers` (`dataflow.rs` 4 → 2) and `--check` (17
  clean, 13 waived, 96 pinned across 39 files, 6607 rows),
  `registry-axes` (`dataflow.rs` 17 → 9) and `--check` (1081 pinned
  across 163 files), `cargo check --workspace` clean.

Found and left, outside this item: W210 reports a `return` read of a
version that can be undefined twice, once from the return pass and once
from the def-use pass (`proc f {} {set v 1; unset v; return $v}` already
drew both); and `collect_expr_cmd_sub_writes` suppresses W210 by name for
the whole function wherever a condition's substitution writes the name —
under a hard-coded `tcl8.6` registry, and for a read before the condition
too. Both are the existence readings slice 8 folds into one query.

Green at VT5.12:

- tests: `tcl-compiler`, `tcl-lsp-core` and `tcl-cli` together 13422
  passed, 6 ignored, no failure (`samples_optimiser_profiles_are_regenerated`
  among them, no sample moved);
- pedantic clippy (`--no-deps --all-targets -D warnings`) on
  `tcl-compiler`, no `#[allow]` added; `rustfmt` on the touched files;
- `cargo xtask value-transfers --check` and `registry-axes --check`
  unchanged; `cargo check --workspace` clean.

Green at VT5.7:

- tests: `tcl-registry`, `tcl-compiler` and `xtask` together 11184
  passed, 6 ignored, no failure (the one expectation the mandate moves,
  `evaluate_def_foreach_multi_var_widens`, restated as
  `evaluate_def_foreach_multi_var_binds_each_its_elements`: a two-binder
  header over `a b` gives its first binder `a` where it had widened);
  `tcl-cli`'s `samples_optimiser_profiles_are_regenerated` passes over this
  checkpoint and VT5.6's, no sample moved;
- pedantic clippy (`--no-deps --all-targets -D warnings`) on
  `tcl-registry`, `tcl-compiler` and `xtask`, no `#[allow]` added;
  `rustfmt` on the touched files;
- `cargo xtask value-transfers` (the four `dict` body rows declared) and
  `--check` (17 clean, 13 waived, 98 pinned across 39 files, 6607 rows),
  `registry-axes --check` (1089 pinned across 163 files), `pack-goldens
  --check` (24 packs, no snapshot moved), `cargo check --workspace` clean.

Green at VT5.6:

- tests: `tcl-cmd-core`, `tcl-registry`, `tcl-spectcl` and `tcl-compiler`
  together 11380 passed, 7 ignored, no failure (the samples test runs at
  the next checkpoint, over this one's rewrites too);
- pedantic clippy (`--no-deps --all-targets -D warnings`) on
  `tcl-cmd-core`, `tcl-registry`, `tcl-compiler` and `tcl-spectcl`, no
  `#[allow]` added; `rustfmt` on the touched files;
- `cargo xtask value-transfers` (the `binary format` row declared) and
  `--check` (17 clean, 13 waived, 98 pinned across 39 files, 6607 rows),
  `registry-axes --check` (1089 pinned across 163 files), `owner-resolution`
  (45 rows), `pack-goldens --check` (24 packs, no snapshot moved), `cargo
  check --workspace` clean.

Found and left, outside this item: O100 already writes a computed
non-ASCII character into the source under 8.x — `set x [format %c 255];
puts $x` under `tcl8.6` optimises to `puts ÿ` — which a release reading
source in a system encoding other than UTF-8 reads as other characters.
The materialisation gate above covers byte arrays only; a computed
string's source spelling under a non-UTF-8 target is the rewrites' to
settle (a `\uXXXX` spelling, or no rewrite), and nothing in this slice
owns it.

Green at VT5.5:

- tests: `tcl-registry`, `tcl-spectcl`, `xtask` and `tcl-compiler`
  together 11482 passed, 7 ignored, no failure; after the last `scan`
  declines, `tcl-registry` again in full (1217) and `tcl-compiler`'s
  `value_transfer`, `type_infer` and `sccp` unit tests (147) and witness
  binary (39);
- pedantic clippy (`--no-deps --all-targets -D warnings`) on
  `tcl-registry`, `tcl-compiler`, `tcl-spectcl` and `xtask`, no `#[allow]`
  added; `rustfmt` on the touched files;
- `cargo xtask value-transfers` (the inventory regenerated: `scan`,
  `binary scan`, `lassign` and `array set` declared with direct routes)
  and `--check` (17 clean, 13 waived, 98 pinned across 39 files, 6607
  rows), `registry-axes --check` (1089 pinned across 163 files, unchanged),
  `pack-goldens --check` (24 packs, no snapshot moved), `cargo check
  --workspace` clean.

Green at VT5.4:

- tests: `tcl-regex` (all features), `tcl-cmd-core`, `tcl-registry` and
  `tcl-spectcl` 1676 together; `tcl-compiler` and `tcl-explorer` 9828;
  `tcl-lsp-core` 3566; `tcl-lsp-db` 127; `xtask` 235; `tcl-vm` under
  `LANG=C.UTF-8` (five of its tests, none of them regexp's, read the
  locale and fail when none is set); the samples test; no failure;
- pedantic clippy (`--no-deps --all-targets -D warnings`) on `tcl-regex`
  (all features), `tcl-cmd-core`, `tcl-registry`, `tcl-compiler` and
  `xtask`, no `#[allow]` added; `tcl-spectcl`'s run stops in the
  consumer-contracts lane's `loader.rs` (`member_row`, 103 lines, from
  `d2f1ece4`), theirs, and this item's hunk there is two catalogue rows;
  `rustfmt` on the touched files;
- `cargo xtask value-transfers` (the inventory regenerated) and `--check`,
  `registry-axes --check`, `pack-goldens --check` (24 packs, no snapshot
  moved), the shard verifier's self-test, and `cargo check --workspace
  --all-targets`, all clean.

Beyond the plan's file list: `tcl-regex`'s `lib.rs`, `exec.rs`,
`cmd_core.rs` and `precision_oracle.rs` and `tcl-cmd-core`'s `regex.rs`
(the metering, D127), `value_transfer/{const_ops,route,builtins,mod}.rs`,
`commands/tcl/mod.rs`, `pack_hooks.rs` and `tcl-spectcl`'s `catalogue.rs`
(the ids, the versioned folder's row), the page's two rows, and the two
compiler tests the mandate changes.

Green at VT5.3:

- tests: `tcl-regex --features cmd-core` 25 across 9 binaries;
  `tcl-cmd-core` 130; `tcl-dialect` 166; `tcl-vm` (under `LANG=C.UTF-8`)
  1473 across 50 binaries; `tcl-registry` 1198 (the consumer-contracts
  lane's `c6a29f41` moved its own counts); `tcl-lsp-core`'s
  `are_tokenizer_consistency`; `runtime/rust`'s regex lib tests (12) and
  `parser_gaps` (34), that crate checked with `--all-targets`; no failure;
- pedantic clippy (`--no-deps --all-targets -D warnings`) on `tcl-regex`
  (with `cmd-core`), `tcl-cmd-core`, `tcl-vm` and `tcl-dialect`, no
  `#[allow]` added (the one on `regexp_run` is the one `regexp` carried);
  `rustfmt` on the touched files;
- the shard row `2 tcl-regex::precision_oracle` and the shard verifier;
  `cargo xtask value-transfers --check` and `registry-axes --check` OK,
  unchanged; `cargo check --workspace --all-targets` clean.

Beyond the plan's file list, each forced by the trait change or the key:
`tcl-cmd-core`'s `switch.rs` and `lsearch.rs` (callers of
`RegexEngine::exec`), `tcl-vm`'s `cmd_regexp.rs` (its match helper),
`runtime/rust/src/regex_capi.rs` (the C API's `TclReExec`, outside the
workspace), and `tcl-dialect`'s `version.rs` (`Hash` on the two
target-semantics enums the cache key holds).

Green at VT5.2:

- tests: `tcl-compiler` 9725 passed, 6 ignored, across 68 binaries (every
  existing test unchanged; the four new ones pass); `tcl-explorer` 103;
  `tcl-lsp-core` 3566; `tcl-lsp-db` 127, 5 ignored; `tcl-lsp-server`'s
  lib and `e2e` binaries 2192, 5 ignored; `tcl-cli` 127, the samples test
  among them (no sample moved); no failure anywhere;
- pedantic clippy (`--no-deps --all-targets -D warnings`) on
  `tcl-compiler` and `tcl-explorer`, no `#[allow]` added; `rustfmt` on the
  touched files;
- `cargo xtask value-transfers --check`: OK, unchanged (17 clean, 13
  waived, 98 pinned across 39 files, 6607 rows); `cargo xtask
  registry-axes --check`: OK (1089 sites pinned across 163 files);
- `cargo check --workspace --all-targets`: clean.

The files the plan named that needed no edit: `representation_plan.rs`
holds no `TclType` judgement at all — its plans are target-independent
and read no lattice — so there was nothing in it to read representation
evidence instead of a type (D118).

Green at VT5.1:

- tests: `cargo test -p tcl-registry -p tcl-compiler --no-fail-fast`, no
  failure (`tcl-compiler`'s lib 6475 passed, 2 ignored; `tcl-registry`'s
  lib 905; `value_transfers` 27);
- pedantic clippy (`--no-deps --all-targets -D warnings`) on
  `tcl-registry` and `tcl-compiler`, no `#[allow]` added; `rustfmt` on the
  touched files;
- `cargo check --workspace --all-targets --exclude xtask` clean. `xtask`
  does not compile at this commit's worktree for a reason outside it: the
  consumer-contracts lane's uncommitted `rust/xtask/src/registry_axes.rs`
  (its step-2 lint) is mid-edit, so G1 was not run here; this commit adds
  no recogniser site to a scanned file and no declaration, so the gate's
  verdict and the inventory are unchanged, and G1 runs at the next
  checkpoint.

#### Record (2026-09-24): the sonnet items of slice 5

A second implementer runs the sonnet items the coordinator held back from
the opus one — VT5.13, VT5.14, VT5.17, VT5.19, VT5.20, in that order —
starting from `3c6714b4` (VT5.10 landed). Each item is its own checkpoint
commit; this table gains a row as each lands, and any deviation from the
plan's literal wording is called out beside the row it belongs to.

| Item | Commit | What landed | Its tests |
|---|---|---|---|
| VT5.13 | `wip(value-transfers): slice 5 — W100's produced set is the unbraced-expression fact` | No production change: `emit_w100_unbraced_expr` and `push_w100` (`analyser/diagnostics/usage.rs`) are untouched, as the item specifies. The test proves the produced W100 set already covers every EXPR-role form — `expr $a+1`, `expr "$a + 1"`, `if "$x" {…}`, `while $c {…}`, `for {} $c {} {…}` and an unbraced `expr` nested in `[…]` — one finding at the expression word's own span (for the quoted-argument case, the span the walk already draws stops one short of the closing quote, the same inner-end convention `AGENTS.md` § *Word-token closing delimiters* documents for a bracketed token; unchanged by this item and pinned as found), and the braced forms of all six draw nothing — the fact DP8.1 keeps in `FACT_CODES` and DP8.2's `brace_expr_hints` reads for O111 (B-DP3, D23) | `w100_marks_every_unbraced_expression` (`analyser/diagnostics/tests.rs`, appended) |

Green at VT5.13: `cargo test -p tcl-compiler --lib` 6495 passed, 2 ignored,
0 failed (byte-identical; one test added); `cargo clippy -p tcl-compiler
--all-targets --no-deps -- -D warnings` clean, no `#[allow]` added;
`cargo fmt -p tcl-compiler` no further change. No production file touched,
so `cargo xtask value-transfers --check` and `registry-axes --check` stay
at the baseline (19 clean, 14 waived, 91 pinned across 37 files, 6607
rows; 1070 pinned across 163 files); `cargo check --workspace` clean.

| VT5.14 | `wip(value-transfers): slice 5 — the no-route writers` | `MayWriteSemantics { targets: &'static [ArgRole], reason: NoRouteReason }` (`value_transfer/builtins.rs`, new): `route` is `EvalRoute::None { reason }`, `identity` the shared spelling `"may_write"` (parallels `UnbindSemantics`'s one shared identity), and `store_targets` scans the declared roles — inert for these commands today (`call_defs` widens before it reads `store_targets` for a `None` route) but not decorative, since a future direct route reuses it unchanged. Declared: `FILE_STAT`/`FILE_LSTAT` (`file stat`, `file lstat`, `NoRouteReason::Platform`), `FILE_TEMPFILE` (`file tempfile`, `Platform`), `GETS` (`gets`, `chan gets`, `Declared`), `VWAIT` (`vwait`, `Declared`), `TK_OPTION_MENU` (`tk_optionMenu`, `Declared`), `TRACE` (`trace add`/`remove`/`variable`/`vdelete`, `Callback` — D156). `ForeachLineSemantics` (new, unit struct): `structure` answers `PlanAnswer::Iterate` over one `BinderName::Operand` binder and `IterableKind::Vendor { collection, cardinality: None }` for the filename operand, under `InvocationLayout::Source` only — `tcl-compiler`'s structured lowering (`lower_foreach_line`) turns every statically-bodied call into a plain `Statement::Foreach` before this declaration is ever consulted, and the CFG's own synthetic loop header always names its command `foreach`/`lmap`/`dict for`/`dict map` (`cfg_lower.rs`'s `fe_cmd`), never `foreachLine`, so the `Iterate` plan is read only for the dynamic-body fallback call; `IterableKind::List` is not used for it — the operand is a filename, and reading it as a list would misreport the name as the file's contents. Twelve `KNOWN_GAPS` rows removed (`xtask/src/value_transfers.rs`); `NoRouteReason::Platform` added (D156) | `route_stamps_match_the_pinned_set` (`tcl-registry/tests/value_transfers.rs`) gains the twelve stamps (`route_label`'s `NoRouteReason::Platform` arm, `"none:platform"`); every existing `value_transfers.rs` test unchanged |
| VT5.17 | `wip(value-transfers): slice 5 — the editor consumers` | Deviation, recorded here since the item's own wording ("gets hover on `$fmt`") does not match the tree: `hover_impl`'s `variable_position_hover` tier is *definitive* for a `$`-led read (its own doc comment: answering the wrong kind of card is "worse than none"), so it always wins over `registry_pattern_format_hover` for a cursor on `$fmt` itself — there is no tier reordering that keeps that invariant and also lets the format tier answer first. Instead `variable_hover`'s read branch now also asks `registry_pattern_format_hover` for this same position and appends its markdown to `var_hover_text` as a fourth, optional section (`format_info`), the same additive shape the card's existing intrep/taint sections already use — never replacing the "Variable" card, only extending it. `pattern_format_hover_for_command` itself gains `proven_text_at_token` (a `CompilationUnit` built at most once per call, lazily, only once a literal read fails — the same construction `infer_var_type_and_taint` already uses for a `$var` hover) tried through `function_units_in_order`'s existing top-level-then-procs order; extracted `format_hover_for_command` to stay under clippy's line budget. `inlay_hints.rs`: `collect_type_hints` and `collect_format_string_hints` now share one `CompilationUnit`, built once in `inlay_hints_in_program` (previously two, one per family); `collect_format_string_hints`'s literal branch is now gated on `tok.kind` being `Str`/`Esc` — its own `format_content_range` merely strips a delimiter *if present* and does not itself refuse a `$`/`[`-led token, so before this fix a computed word's raw source text (`"$fmt"`) was scanned for `%`-specifiers and silently found none, never reaching any fallback; the proven fallback (`cu.functions().find_map(word_at → proven_word_value)`) anchors every specifier at the token's own end, since a computed word carries no in-source span of the *value's* text to place them at; folded `collect_binary_hints` and the sprintf/clock/regsub match arms into one `emit_format_specifier_hints`, parameterised by a `position_at` closure (`cstart + offset` for a literal, a fixed anchor for a proven one) — the refactor `emit_format_specifier_hints`'s own argument count forced (`FormatHintCtx`, bundling `range`/`source`/`line_index`/`profile`). `semantic_tokens.rs`: verified, production-unchanged — `insert_format_overrides` marks a format-role argument by its registry *position* regardless of literalness, and each family's own sub-tokeniser already "falls back to the default classification" when it finds no specifier in the token's own bytes, so a computed word (whose own text is just `"$fmt"`) already renders as a plain `variable` token today; "a computed pattern is explained at its use" is `hover.rs`'s job, and "never painted at a token range it does not have" was already true here — a permanent regression test pins it rather than changing anything. `regexp`'s own family needed nothing further: `tcl_compiler::regex_source` already traces a pattern to its originating literal for semantic tokens, a pre-existing, separate mechanism this item does not touch. `document_links.rs`'s `speclib` and `include` sites (`document_link_root_pack`) each gain `// value-transfer-ok: irreducible — the pack grammar's own statements`, dropping its pin 2 → 0; moved from `RATCHET` to `CLEAN_FILES` (`xtask/src/value_transfers.rs`) and its ledger row removed (`value-transfers-migration.md`) | `hover_explains_a_computed_format_string`, `var_hover_text_appends_proven_format_info` (`hover.rs`); `inlay_hints_label_a_computed_format_string` (`inlay_hints.rs`); `a_computed_format_word_falls_back_to_its_plain_classification` (`semantic_tokens.rs`); each with its negative (an unproven `$fmt` draws nothing extra); every existing hover/inlay-hints/semantic-tokens/document-links test unchanged |
| VT5.19 | `wip(value-transfers): slice 5 — the slice's witnesses` | Three compiler witnesses (`value_transfer_witnesses.rs`, appended): `the_no_match_preserve_witness` (#2051's plain program — VT5.11 already pins the condition/word forms in a diagnostics-focused test; this one reads the lattice directly via `last_value` and prints under every release); `the_partial_scan_witness` (`scan "12" "%d %d" a b` as its own statement — nested in `set n [scan …]`, both `a` and `b` were `Overdefined`, since a value-position nested invocation's stores answer under the effect-free policy `fold_cmd_subst_routes` documents, which has no definition for them to land on, an outcome unrelated to what a partial scan proves; `a` folds typed `Int`, as `%d` built it, not `text("12")`); `the_repeated_target_witness` (deviation: the interface page's own "`lassign … a a`" is 8.5+ and raises under a tclsh8.4 on `PATH`, so this reads the same "last position wins" fact through `regexp`'s match-variable list — `(a)(b)` against `ab` into `x x` — which every release shares). One CLI witness (`value_transfers_cli.rs`): `explore_sccp_prints_folded_types`, the exit line's own `binary format` program beside `format`'s and `list`'s constructed types (VT5.2) and a plain literal's absent type line, so "types" is not one route's alone. One parity witness (`value_transfer_parity.rs`): `destructuring_agrees_on_both_paths`, the same three commands plus `array set` (whose element write is part of the parity though `arr` itself carries no scalar value), value for value, on `tcl8.6` and `tcl9.0`. Every expected value hand-verified against `/root/.local/bin/tclsh8.4` to `9.1` before being written into a test (D-none: no new decision, an oracle check) | `the_no_match_preserve_witness`, `the_partial_scan_witness`, `the_repeated_target_witness` (`value_transfer_witnesses.rs`); `explore_sccp_prints_folded_types` (`value_transfers_cli.rs`); `destructuring_agrees_on_both_paths` (`value_transfer_parity.rs`); every existing witness, CLI and parity test unchanged |
| VT5.20 | `wip(value-transfers): slice 5 — destructuring and structured bodies` (landing) | Docs only, no Rust file touched. The nine design pages the item names: `pass-fact-ownership-matrix.md` and `downstream-pass-contracts.md` gain the `proven.rs` per-function pass and its consumers (already done at VT5.17's checkpoint, verified here); `sccp-core-analyses.md` gains the `BranchFactKind` explanation and a `SccpResult::preserved` section; `constant-folding-type-inference.md` gains the destructuring-writers section; `optimisation-passes.md` documents `SccpResult::materialises` gating O100/O103/O127; `precision-limitations.md` gains the regexp-cap "Accepted" entry; `value-transfers-migration.md`'s ratchet table is verified byte-equal to `RATCHET` (the cc-step2 merge already kept the two in sync) and its "dataflow sites this design owns" § *Literal-only editor features* bullet is corrected — stale since VT5.17 gave hover and inlay hints a proven-value fallback for the format family; `value-transfers.md` checked against D141/D142/D145/D151–D155 and found already accurate, no edit needed. Two KCS diagnostic notes gain sections (W102's proven-switch narrowing, W210's preserve-outcome section); a new KCS note, `kcs-qa-why-does-a-regexp-sometimes-not-fold.md`, is indexed in both `docs/kcs/README.md` and `docs/kcs/compiler/README.md`. The lane doc's own VT5.8 item text is corrected: the page's `TemplatePlanRecord { span, plan }` grows to the tree's `{ span, command, switches, plan }` at VT5.10 (D154) — this row, D156, and this status section. `docs/design/lanes/README.md`'s in-flight line marks slice 5 landed, slice 8 next. `diagnostics-calculation.md` / `diagnostics-integration.md` stay drafted in the lane doc for the diagnostic-policy lane (B-DP4), unchanged — not this lane's files to edit | none (docs); G4 not triggered (no generated-catalogue text changed) |

Green at VT5.14: `cargo test -p tcl-registry` 36 (`value_transfers.rs`) +
the crate's other suites, all passed, 0 failed (`route_stamps_match_the_pinned_set`
and `shipped_builtins_stay_on_the_direct_route` among them); `cargo
clippy -p tcl-registry -p xtask --all-targets --no-deps -- -D warnings`
clean, no `#[allow]` added; `cargo fmt -p tcl-registry -p xtask` applied
(import ordering only). `cargo xtask value-transfers` (twelve rows'
`has_semantics` flips true, their gap note changes from "writes a
variable, no semantics" to "descriptor without a route" — the same
classification `foreach_in_collection`/`append_to_collection` already
carry, so no new `KNOWN_GAPS` row replaces the twelve removed) and
`--check` (19 clean, 14 waived, 91 pinned across 37 files, 6607 rows,
unchanged — no scanned consumer file touched); `registry-axes --check`
(1070 pinned across 163 files, unchanged); `pack-goldens` (24 packs, 0
rewritten); `cargo check --workspace` clean.

Green at VT5.17: `cargo test -p tcl-lsp-core --lib` 2349 passed, 0 failed
(no existing expectation moved; four tests added); `cargo clippy -p
tcl-lsp-core --all-targets --no-deps -- -D warnings` clean, no `#[allow]`
added (`too_many_lines` on `pattern_format_hover_for_command` and
`too_many_arguments` on `emit_format_specifier_hints` fixed by extraction
and a bundled context struct, not waived); `cargo fmt -p tcl-lsp-core -p
xtask` applied. `cargo xtask value-transfers` (`document_links.rs` 2 → 0,
clean; two waiver sites added) and `--check` (20 clean, 16 waived, 89
pinned across 36 files, 6607 rows); `registry-axes --check` (1070 pinned
across 163 files, unchanged); `pack-goldens` (24 packs, 0 rewritten);
`cargo check --workspace` clean.

Between VT5.17 and VT5.19 the coordinator fast-forwarded this branch over
the consumer-contracts lane's `cc-step2` merge (`50ce88a6` → `d9d9868b`,
96 files, green under `make rust-check`, none of this item's three files
touched): `AnalysisContext::surface_query` landed in
`value_transfer/context.rs`, and that lane's own B-CC10 edit to
`rust/xtask/src/value_transfers.rs` lowered `analyser/commands.rs` and
`analyser/oo.rs` off the ratchet and `lowering/mod.rs` to 1 (their sites
went with that lane's work). The baseline VT5.19's gates run against is
therefore the merge's, not VT5.17's: `value-transfers --check` 20 clean,
16 waived, 83 pinned across 34 files, 6607 rows; `registry-axes --check`
6 clean, 5 waived, 956 pinned across 157 files. Neither gate's counts
move again under VT5.19 — it declares no semantics and touches no scanned
file.

Green at VT5.19: `cargo test -p tcl-compiler --test value_transfer_witnesses`
47 passed, 0 failed (three added, every existing witness unchanged);
`cargo test -p tcl-cli --test value_transfers_cli` 7 passed, 0 failed
(one added); `cargo build -p tcl-cli` then `cargo test -p tcl-cli --test
cli samples_optimiser_profiles_are_regenerated` passed, no sample moved;
`cargo test -p tcl-lsp-db --lib` 102 passed, 0 failed (one added, every
existing parity test unchanged); `cargo clippy -p tcl-compiler -p tcl-cli
-p tcl-lsp-db --all-targets --no-deps -- -D warnings` clean, no `#[allow]`
added; `cargo fmt -p tcl-compiler -p tcl-cli -p tcl-lsp-db` applied;
`bash scripts/dev/test-nextest-binary-shards.sh` ok (no new test binary,
no shard row needed); `cargo xtask value-transfers --check` and
`registry-axes --check` unchanged from the post-merge baseline above;
`pack-goldens` (24 packs, 0 rewritten); `cargo check --workspace` clean.

Green at VT5.20 (docs only): `cargo xtask kcs-index-links` — "KCS docs
checks passed" (the new note indexed in both `docs/kcs/README.md` and
`docs/kcs/compiler/README.md`, every link resolves); `cargo xtask
owner-resolution` — OK, 45 owner rows (grown from 44 by the cc-step2
merge, not this item); `cargo xtask value-transfers --check` and
`registry-axes --check` unchanged from the post-merge baseline (20
clean, 16 waived, 83 pinned across 34 files, 6607 rows; 6 clean, 5
waived, 956 pinned across 157 files) — the ratchet table in
`value-transfers-migration.md` was already equal to `RATCHET` before this
item touched the file, confirming the cc-step2 merge kept G1's ledger
half green; `pack-goldens` (24 packs, 0 rewritten); `cargo check
--workspace --all-targets` clean. The landing's own full battery, run
after VT5.20's docs: `cargo test -p tcl-regex -p tcl-cmd-core -p tcl-vm
-p tcl-registry -p tcl-compiler -p tcl-explorer -p tcl-lsp-core -p
tcl-lsp-db -p tcl-cli -p xtask` (R6's suite) — every crate green except
one pre-existing, unrelated failure: `tcl-vm`'s `builtins_e2e::
ensemble_subcommand_words_resolve_like_tclsh` asserts `encoding system`
answers `utf-8`, and this container's locale is `POSIX`/`C`
(`LC_CTYPE=POSIX`), so it answers `iso8859-1` instead; `tcl-vm` is not a
file any VT5.x item touches or any item's own suite names, and the
assertion is about the host's default encoding, not a value-transfer
fact. tcl-compiler: `--lib` 6499 passed, 2 ignored, 0 failed, plus every
one of its 66 integration-test files and its doctests ok, 0 failed (68
binaries, 9757 tests total, including every exit-evidence witness named
above); tcl-registry: `--lib`
924 passed, every other binary ok (`value_transfers.rs`'s 36 among them);
tcl-explorer, tcl-lsp-db (`--lib` 102), tcl-lsp-core (`--lib` 2349), and
tcl-cli (128 across its binaries, including the CLI's built `tcl` binary
run by `explore_sccp_prints_folded_types`) all ok, 0 failed; tcl-regex,
tcl-cmd-core and xtask all ok, 0 failed. `cargo clippy -p tcl-registry -p
tcl-lsp-core -p tcl-compiler -p tcl-cli -p tcl-lsp-db -p xtask
--all-targets --no-deps -- -D warnings` clean, no `#[allow]` added; `cargo
fmt` (the same six crates) `-- --check` clean, no diff.

#### Record (2026-09-24): review fixes for slice 5

The review of the slice 5 landing (`e4404113` to `9906d6af`) returned
"land with fixes". The commit `wip(value-transfers): slice 5 — review
fixes` holds every fix, applied after VT8.1 (`f73ffe63`) and before any
slice 8 item reads a preserve outcome; each is pinned by a test whose
expectation was run under tclsh 8.4 to 9.1. The decisions are D162–D164
and D156's amendment.

| Finding | What changed | Its tests |
|---|---|---|
| B1 (blocking) — `%n` before an exhausted conversion | `tcl_cmd_core::scan::scan_match` counts `nconv` as C's `nconversions` (D163): `%n`, suppressed or not, and every successful conversion, suppressed or not, so the underflow fires only when the input ran out before any of them. `scan {} %n%d n a` is 1 with `n` written 0 and `a` preserved, `scan abc %s%n a n` 2, `scan {} %*n%d a` and `scan 5 %*d%d a` 0, and only `scan {} %*d%d a` -1, on every release; the route had answered -1 and preserved `n`, so `tcl opt` rewrote `puts $n` to `puts 5` and W210 reported the read. The route's `ended_first` and the VM's variable-mode `-1` follow from the core with no edit of their own, so the VM's `scan` result for the same inputs is corrected by the same change. Deviation: the review named `%n` and allowed only `percent_n_reports_consumed` to move; C counts a suppressed success with the same increment, and `scan 5 %*d%d a` answered -1 where every release answers 0, so it is fixed in the same place and `float_suppress_and_width` moves too (`%f %*d` over `3.5 99`: `nconv` 1 → 2, its values unchanged). Deviation: the fixed route proves `n` is 0, so the witness asserts the optimised program is not `puts 5` and prints 0 rather than that `puts $n` stays — O100's `puts 0` prints what the original prints | `percent_n_reports_consumed` (`nconv` 1 → 2) and `float_suppress_and_width` (1 → 2) move; `percent_n_and_a_suppressed_success_prevent_the_underflow` (`scan.rs`, new); `destructuring_witnesses_match_every_release_on_path` (`tcl-registry/tests/differential_fold.rs`) reads `SCAN_COUNT_WITNESSES` after its table, nine rows (the review's two, the suppressed forms, their inline forms) that must answer on every release; `the_percent_n_count_witness` (`value_transfer_witnesses.rs`, new): `n` is 0 in five dialects, no `puts 5`, no W210 on `n` with or without the prior `set`, and both programs print 0 before and after `tcl opt` under 8.4 to 9.1. `cargo test -p tcl-cmd-core -p tcl-vm` moved no other expectation |
| S1 — the no-route writers' existence transfer | `MayWriteSemantics` gains `kind: Option<BindingKind>` and answers `TransferAnswer::Existence` with one `MayBind(kind)` per target on the normal path; the driver's `transferred` already reads a declaration's existence transfer for a route-less call, so each target steps `Join(Bound(kind))` in place of D159's widening. `file tempfile`'s variable is a scalar and `vwait` stays generic (D156, amended) | `each_may_write_declaration_answers_a_may_bind_of_its_target` (`tcl-registry/tests/value_transfers.rs`, new): the eleven declarations from the full registry, each `VarWrite` operand from `arg_indices_for_role`; ten answer one `MayBind` of it, `vwait` `Generic`, and every type transfer is `Generic` |
| S2 — a substituted subject hid the targets | `LatticeInputs::resolve_roles_over_values` (`value_transfer.rs`), called after `view_of` in `evaluate_source_call`, re-runs the command's `arg_role_resolver` over the words' exact lattice texts (D164) | `a_proven_subject_resolves_the_targets_roles` (`value_transfer.rs`, new): `regexp {(a+)b} $s -> g` gives `g` `aa`, `lassign $l a b` binds 1 and 2, and a no-match `regexp {(x)} $t -> m` preserves `m` at `before` (tclsh 8.5 to 9.1: `aa 1 2 before`) |
| S3 — unwaived tuple arms in a clean file | `harvest_table_command_value_spans`'s `set`, `array set` and `dict set` arms (`var_command.rs`) each carry `// value-transfer-ok: dataflow — needs each value's token span, which no outcome carries`, the VT5.18 record's own reason; G1's `scan()` recognises a tuple-pattern arm naming a literal under a `match (…)` whose tuple binds a head (`is_tuple_literal_arm`), so the waivers are what keep the file clean | `flags_a_tuple_match_arm_naming_a_literal` (xtask, new): the two arms under `match (command.as_str(), canonical)` are sites, the arm under `match (kind, other)` is not |
| S4 — two stale doc lines | `pass-fact-ownership-matrix.md`'s `value_transfer.rs` row: hover and inlay hints read `proven_word_value` for a computed pattern or format argument, and the semantic-token families stay literal-only; `value-transfers-migration.md`'s item 5 carries the "(landed)" marker and the closing line item 4 carries | none (docs) |
| 6 — G1 stopped at a test-only item | `scan()` (`xtask/src/value_transfers.rs`) ends only at an inline test module; `test_item_end` skips any other annotated item alone, counting braces outside string and character literals (`code_braces`). The wider scan's per-file deltas: `analyser/commands.rs` 0 → 4 and `taint.rs` 0 → 3, pinned (D162); every other file unchanged. The four in `commands.rs` are VT8.9's; the three in `taint.rs` are listed in D162 | `a_test_only_item_is_skipped_alone` (xtask, new): a test-only function, `use` and `mod name;` are skipped, the function after them is a site, and the inline tests module ends the scan |
| Dialect drift (#2253) | `dict_value_at` (`value_transfer.rs`), the site the coordinator named at `value_transfer.rs:3294`, splits the dictionary with the dialect's `WordValueRules` (`of_profile(registry.profile())`) instead of `tcl_syntax::list::split_list`; `cargo xtask dialect-drift` is back to its eight pre-existing sites (the gate is disabled pending #2253) | the dictionary-body tests (`dict with` / `dict update`) unchanged |
| The slice 8 heading | The slice 5 landing's docs dropped this document's `### Slice 8 — the existence rung` heading, leaving slice 8's goal and items under slice 5; it is restored | none (docs) |

Green at the review fixes:

- tests: `tcl-compiler` 9753 passed, 6 ignored across its 67 binaries,
  and 7 doctests; `tcl-registry` 1242; `tcl-lsp-db` 128, 5 ignored;
  `tcl-cli` 128; `tcl-lsp-core` 3567; `tcl-explorer` 103; `tcl-cmd-core`
  131 and a doctest; `xtask` 237; `tcl-vm` 912 under `LC_ALL=C.UTF-8` —
  under this container's `POSIX` locale
  `ensemble_subcommand_words_resolve_like_tclsh` (`builtins_e2e`) fails
  as VT5.20's battery recorded, `encoding system` answering `iso8859-1`,
  which nothing here touches — no other failure;
- pedantic clippy (`--all-targets --no-deps -D warnings`) on
  `tcl-cmd-core`, `tcl-registry`, `tcl-compiler` and `xtask`, no
  `#[allow]` added; `cargo fmt` on the four;
- `cargo xtask value-transfers` (the inventory gains the three waived
  `var_command.rs` sites and the two pinned files) and `--check` (20
  clean, 19 waived, 90 pinned across 36 files, 6607 rows);
  `registry-axes --check` (956 pinned across 157 files, 5 waived, 6
  clean); `pack-goldens` (24 packs, 0 rewritten); `dialect-drift` 8
  sites, the eight upstream ones (#2253); `cargo check --workspace`
  clean.

### Slice 8 — the existence rung

#### Goal and exit

In the plan's words: "A flow-sensitive bound/unbound fact per place and
per SSA version of the binding, owned by the solver and fed by storage
outcomes: the entry states, the join, the absent-cell release rule
(`safe_on_uninit`, the plan's `creates_absent`), `[info exists]` and
`[array exists]` through the expression route's `nested` service, and the
guard narrowing as an edge refinement in the existence domain. W210,
W211, W213, W214, O108, O109, I230, O101, and S100 consume the one fact; a
fast-tier request and a function over the complexity ceiling read
`Unavailable`, which is neither bound nor unbound." `KNOWN_GAPS` adds
`array unset`, `array default` and (VT2.8) `const`; CC2.13 puts the `Set`
analyser hook here.

*Exit*: "`sccp.rs` recognises no command by spelling;
`existence_constant_branches` and `scan_defined_and_unset` are deleted;
`emit_provably_unset_w210` reads the fact; the release table for an
absent cell, `set x 1; unset x; info exists x` deciding `0`, the definite
W213 after a killed version, the two O109 refusals, and the S100 silence
pass."

Exit evidence: `the_absent_cell_release_table`,
`set_unset_info_exists_decides_zero`, `a_second_unset_is_a_definite_w213`,
`o109_keeps_a_store_an_existence_read_observes`, `s100_ignores_an_unset_arm`
(compiler witnesses); G1 with `dataflow.rs` in `CLEAN_FILES` and
`helpers.rs` at 4, and `sccp.rs` holding no `existence_constant_branches`
or `scan_defined_and_unbound`; `tcl explore --source 'proc p {} {set x 1;
unset x; if {[info exists x]} {puts yes}}' --show sccp --text --no-colour`
prints the branch as decided and the `if`'s true block outside
`executable blocks`.

#### Upstream starting point

From `git diff 3b5eba8a origin/rust -- rust/tcl-compiler/src/ssa.rs
rust/tcl-registry/src/registry.rs rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs
rust/tcl-compiler/src/optimiser/code_sinking.rs rust/tcl-compiler/src/script_binds.rs
rust/tcl-compiler/src/existence_query.rs`:

- #2220: `CommandRegistry::variable_read_projection` and the CFG's
  `ConditionEffects` make a condition's `[info exists x]` a read of `x`,
  so W211, O126 and O109 keep the store behind an existence test in a
  condition, and `code_sinking.rs` refuses to sink past one. VT8.4 and
  VT8.5 generalise the existence read to every position (a statement's
  `unset`, a `DESTROYS_VARIABLE` command, a nested `[info exists]`) and
  re-express #2220's condition case as the same existence read — one
  implementation.
- `dataflow.rs`'s `statement_is_synthetic_effect`: W210 skips the reads a
  synthetic `<cond>` or `<upvar-invalidate>` statement carries because
  they are existence-tolerant; VT8.4 replaces the exemption with the
  existence fact those reads consult.
- `script_binds.rs` (new, 287): `script_binds_name`, the registry-driven
  answer to "does this script bind the name" that W210 and the SSA's use
  rule both ask; VT8.4 keeps it.
- `existence_query.rs`: unchanged upstream.

#### Work items

##### VT8.1 — the existence domain in the solver

- **Files**: `rust/tcl-compiler/src/sccp.rs`, `rust/tcl-compiler/src/value_transfer.rs`,
  `rust/tcl-compiler/src/dynamic_names.rs`, `rust/tcl-compiler/src/compilation_unit.rs`.
- **Items**: `SccpResult::existence: HashMap<ValueKey, Existence>` (the
  registry's `Existence`, `BindingKind`), computed in the same fixed point
  as the values. Entry: parameters `Bound(Scalar)`; other locals
  `Unbound`; a scope-alias local (`Traits::CREATES_SCOPE_ALIAS`) and a
  TclOO instance variable `MayBound` at declaration; in the global frame
  a special variable per `startup_read_facts`, a name another procedure
  may write (`scan_module_global_names`) `MayBound`, the rest `Unbound`;
  a `::when::*` handler's cross-event variables (`ConnectionScope::cross_event_defs`)
  `MayBound`; after a `Barrier` or `UpFrame` every place `MayBound`. Join:
  `Bound(Scalar) ⊔ Bound(Array)` is `Bound(Either)`, `Unbound ⊔ Bound(_)`
  and anything with `MayBound` are `MayBound`. Transfer per outcome, as the
  page's § *Existence* states; an unbind of an `Unbound` place completes
  with an error and writes nothing. The release rule: a cell update on an
  `Unbound` place binds only under a profile whose every release creates
  (`creates_absent`); otherwise the value declines `UnboundPlace`.
  `AnalysisInputs::prior_store(place, FactDomain::Existence)` answers from
  the map; the dynamic-name barrier becomes flow-sensitive (a dynamic
  write turns every `Unbound` place `MayBound` from that statement on, a
  dynamic destroy every `Bound` place).
- **Preserves**: every value in `SccpResult::values`.
- **Changes**: `incr fresh` binds under `tcl8.5` onwards and declines
  under `tcl8.4` and a profile spanning both. Mandate: the release rule;
  the Existence row.
- **Tests**: `the_absent_cell_release_table` (compiler witnesses, every
  line of the page's table except the `unset p nosuch q` prefix line,
  which is slice 10's; oracle under 8.4 to 9.1).
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: L. **After**: slice 5.

##### VT8.2 — `info exists` decides inside the fixed point

- **Files**: `rust/tcl-compiler/src/existence_query.rs`,
  `rust/tcl-compiler/src/value_transfer.rs` (`LatticeInputs::nested`),
  `rust/tcl-compiler/src/sccp.rs` (`existence_constant_branches`,
  `scan_defined_and_unbound`, `ExistenceFrame` go),
  `rust/tcl-compiler/src/compilation_unit.rs` (the post-pass and
  `drop_cross_event_existence_folds` go),
  `rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs`
  (`emit_existence_constant_branch_diagnostics`, the reachability gate in
  `emit_constant_branch_diagnostics`), `docs/design/compiler/value-transfers-migration.md`
  (the ledger's `unset` row goes).
- **Items**: the intrinsics `IntrinsicId::InfoExists` and `ArrayExists`
  answer through `nested` as a read of `FactDomain::Existence`:
  `Bound(_)` is 1 to `info exists`, `Bound(Array)` is 1 to `array exists`,
  `Unbound` 0 to both, `Bound(Scalar)` 0 to `array exists`,
  `Bound(Either)` and `MayBound` undecided. The existence branch is an
  ordinary `Applied` branch.
- **Preserves**: every `existence_fold_abstains_*` and
  `upframe_body_models_*` test and every `info_exists_*` test.
- **Changes**: the existence branch updates `executable_blocks`, so O107,
  taint, shimmer and the reachability-gated checks see the dead arm.
  Mandate: § *Existence* ("the post-pass … becomes one path").
- **Tests**: `set_unset_info_exists_decides_zero`.
- **Gates**: G1 (`sccp.rs` stays clean), G7, G8, G9.
- **Model**: opus. **Size**: L. **After**: VT8.1.

##### VT8.3 — the guard as an edge refinement in the existence domain

- **Files**: `rust/tcl-compiler/src/sccp.rs`.
- **Items**: `EdgeRefinement` (the page's shape, `domain` restricted to
  `FactDomain::Existence` in this slice) and the existence map's
  block-qualified lookup `(BlockId, ValueKey)`: on a `MayBound` place the
  true edge of `[info exists x]` carries `Bound(Either)` and the false
  edge `Unbound`; `!` swaps them. `collect_existence_guards` keeps its
  three callers until slice 11 (D8).
- **Preserves**: `info_exists_guard_narrows_read_in_then_arm`,
  `info_exists_negated_guard_narrows_false_arm`,
  `info_exists_read_outside_guard_still_flags_w210`.
- **Tests**: `the_existence_guard_refines_its_edges`.
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: VT8.2.

##### VT8.4 — the lifecycle diagnostics read the fact

- **Files**: `rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs`
  (`emit_provably_unset_w210`, `emit_read_before_set_diagnostics`,
  `record_chain_w210_uses`, `emit_return_phi_undef_w210`,
  `emit_unused_variable_diagnostics`),
  `rust/tcl-compiler/src/analyser/diagnostics/helpers.rs`
  (`whole_unset_names`), `rust/xtask/src/value_transfers.rs`.
- **Items**: W210 reports a read at an `Unbound` or `MayBound` place and
  never at `Bound(_)` or `Unavailable`; W213 reads the fact at the
  `unset` (definite on `Unbound`, "may not exist" on `MayBound`, nothing
  on `Bound`, never for `-nocomplain`), recognised through
  `Traits::DESTROYS_VARIABLE` and the unbind outcome, not the spelling;
  W211 counts an existence read (`info exists`, `array exists`, an
  unbind) as a use; W214 reads the parameters' entry state.
- **Preserves**: every `emit_cfg_ssa_diagnostics_w210_*`, `w213_*` and
  `info_exists_*` test.
- **Changes**: a second `unset` after a killed version is a definite
  W213. Mandate: the W210 row; the exit.
- **Tests**: `a_second_unset_is_a_definite_w213` (`set x 1; unset x;
  unset x`); `a_conditional_unset_gives_w210` (`set x 1; if {$c} {unset
  x}; puts $x`); `nocomplain_never_reports_w213`.
- **Gates**: G1 (`dataflow.rs` 2 → 0, clean; `helpers.rs` 5 → 4), G7,
  G8, G9.
- **Model**: opus. **Size**: M. **After**: VT8.2.

##### VT8.5 — O108 and O109 keep what an existence read observes

- **Files**: `rust/tcl-compiler/src/optimiser/elimination.rs`
  (`assignment_safe_to_delete_with_effect`, the dead-store guards,
  `collect_rmw_hidden_reads`).
- **Items**: a store is removable only when no value read and no
  existence read of its version remains; an unbind statement is never
  removed; `collect_rmw_hidden_reads` shrinks to what the SSA does not
  already record.
- **Changes**: the two refusals — `proc p {} {set x 1; if {[info exists
  x]} {puts yes}}` keeps `set x 1`, and `proc p {} {incr n; if {[info
  exists n]} {puts yes}}` keeps `incr n`. Mandate: the O108 and O109
  rows; the findings table (#2132).
- **Tests**: `o109_keeps_a_store_an_existence_read_observes` (the
  optimised programs print `yes`; the second under 8.5 to 9.1, where the
  original does).
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: S. **After**: VT8.2.

##### VT8.6 — an unbound arm carries no representation (#2133)

- **Files**: `rust/tcl-compiler/src/type_infer.rs` (`evaluate_type_def`),
  `rust/tcl-compiler/src/shimmer/phi.rs`.
- **Items**: the whole-variable kill definition of a
  `DESTROYS_VARIABLE` command is typed `TypeLattice::unknown()`, not the
  command's return type; the phi merge classification skips an arm whose
  existence is `Unbound`.
- **Preserves**: `phi_shimmer_emitted_for_int_string_merge`.
- **Changes**: `set x 1; if {$c} {unset x}; puts $x` loses its S100 and
  keeps its W210. Mandate: #2133; the S100 row.
- **Tests**: `s100_ignores_an_unset_arm`;
  `s100_still_fires_between_the_bound_arms_of_a_three_way_switch` (`set
  x 1`, `set x "hi"` and `default {unset x}` arms: S100 for the two bound
  arms).
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: S. **After**: VT8.1.

##### VT8.7 — `Unavailable` at the fast tier and past the ceiling

- **Files**: `rust/tcl-compiler/src/compilation_unit.rs`,
  `rust/tcl-compiler/src/value_transfer.rs`.
- **Items**: a fast-tier request, a function over the complexity
  ceiling, and a consumer with no SSA read `FactView::Top(DeclineReason::Unavailable(tier))`;
  every consumer of VT8.4 to VT8.6 is silent on it.
- **Tests**: `existence_is_unavailable_at_the_fast_tier`.
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: S. **After**: VT8.4.

##### VT8.8 — `const`, `array unset`, `array default`

- **Files**: `rust/tcl-registry/src/value_transfer/cell_write.rs`, the
  three spec modules, `value_transfers.rs`, `differential_fold.rs`,
  `rust/xtask/src/value_transfers.rs`.
- **Items**: `ConstWriteSemantics` (from 9.0): on an `Unbound` place a
  `Write` and `Bound(Scalar)`; on any other place it declines (`const`
  over an existing ordinary variable errors, and over an existing
  constant keeps the old value — the oracle — so only the absent case is
  a value). `array unset arr` is an `Unbind` of the array; with a
  pattern, `MayWrite` of its elements; `array default` (from 9.0) is a
  `MayWrite` of the base.
- **Tests**: `const_binds_only_an_absent_place` (oracle, 9.0 and 9.1:
  `const c 5; const c 7; set c` is 5; `set x 1; const x 2` errors
  `can't make constant "x": variable already exists`).
- **Gates**: G1 (three rows go), G2, G7, G8, G9.
- **Model**: opus. **Size**: S. **After**: VT8.1.

##### VT8.9 — the `Set` analyser hook retires

- **Files**: `rust/tcl-compiler/src/analyser/handlers.rs`
  (`handle_set_command`), `rust/tcl-registry/src/hooks.rs`,
  `rust/tcl-registry/src/commands/tcl/set_.rs`,
  `rust/tcl-registry/tests/analyser_hooks.rs`.
- **Items**: `define_var` for the two-word form comes from the generic
  role binding (CC2.12's `handle_var_binding_command` over `VarWrite`);
  the constant-string environment (`set_const_string`) reads the value
  word's `CellWrite` evaluation; the `interp create` value binding keeps
  its interpreter-domain key, reached from the generic binding (Q5).
  `AnalyserHookId::Set` leaves the pinned set.
- **Preserves**: every `handlers.rs` test of `set`, the regex-source
  highlighting of a `set`-held pattern, and the `interp create` bindings.
- **Gates**: G2, G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: VT8.4; CC2.12 (B-CC5).

##### VT8.10 — the slice's witnesses

- **Files**: `rust/tcl-compiler/tests/value_transfer_witnesses.rs`,
  `rust/tcl-cli/tests/value_transfers_cli.rs`,
  `rust/tcl-lsp-db/src/value_transfer_parity.rs`.
- **Tests**: every Existence-row witness not named above: a parameter
  and a never-assigned local; `unset -nocomplain`; a scope alias and a
  cross-event variable entering `MayBound`; an existence read keeping its
  store; `o130_folds_a_chain_from_an_absent_cell` (`proc p {} {lappend l
  a; lappend l b; return $l}` folds to `set l {a b}`, the O130 row's
  absent-start chain); `a_failing_dead_lappend_is_retained` (`set l "a
  {b"; lappend l c` with `l` never read keeps the `lappend`, which raises
  `unmatched open brace in list` in every release — the Rewrites row's
  "failing dead write retained", under O108's totality proof); the CLI
  and memoised twins of the exit tests.
- **Gates**: G7, G8, G9.
- **Model**: sonnet. **Size**: M. **After**: VT8.1 to VT8.9.

##### VT8.11 — docs and the landing

- **Files**: `pass-fact-ownership-matrix.md` (the existence fact:
  producer the solver, consumers W210, W211, W213, W214, O108, O109,
  I230, O101, S100, unavailable at the fast tier),
  `downstream-pass-contracts.md`, `sccp-core-analyses.md`,
  `optimisation-passes.md`, `precision-limitations.md`,
  `value-transfers-migration.md`, `docs/kcs/codes/` notes for W210, W213,
  I230 (existence), the lane doc; the diagnostics rows drafted (B-DP4).
- **Gates**: G5, G6, and the slice's green.
- **Model**: sonnet. **Size**: S. **After**: VT8.10.

#### Checkpoints and landing

| Checkpoint | Holds | Green means |
|---|---|---|
| `wip(value-transfers): slice 8 — the existence domain` | VT8.1, VT8.6, VT8.8 | the release table; every value byte-identical |
| `wip(value-transfers): slice 8 — existence decides inside the fixed point` | VT8.2, VT8.3 | the post-pass deleted; every `info_exists_*` and `existence_fold_abstains_*` test |
| `wip(value-transfers): slice 8 — the consumers read the fact` | VT8.4, VT8.5, VT8.7, VT8.9 | W210, W213, O109 witnesses; G1 pins |
| `wip(value-transfers): slice 8 — the existence rung` (landing) | VT8.10, VT8.11 | every exit test; G1 to G9 |

```text
wip(value-transfers): slice 8 — the existence rung

Existence is a fact the solver owns: bound, unbound or may-bound per
place and per SSA version, fed by storage outcomes, entered from the
frame's rules, joined at merges, and released per profile for a cell
update on an absent place. `info exists` and `array exists` read it
through the expression route, so the existence branch decides inside the
fixed point with applied reachability, and the post-pass, its second run
and the cross-event filter are gone with `existence_constant_branches`
and `scan_defined_and_unbound`. The guard narrows as an edge refinement
in the existence domain. W210, W211, W213 and W214 read the fact; O108
and O109 keep a store an existence read observes and never remove an
unbind; S100 skips an unbound arm; a fast-tier request reads
`Unavailable`, which is neither bound nor unbound. `const`, `array unset`
and `array default` have semantics, and the analyser's `set` hook is
retired.

Behaviour changes: the existence branch prunes its dead arm for every
reachability consumer; a second `unset` after a killed version is a
definite W213; O109 keeps the stores behind `[info exists …]`; S100 no
longer reports an unset arm as a string; an `incr` of an absent place
binds under 8.5 onwards and declines under 8.4.

Closes #2133. Pins #2132 (closed on rust by #2220).
```

#### Review checklist

- R1: `CLEAN_FILES` gains `analyser/diagnostics/dataflow.rs`; `helpers.rs`
  at 4; `sccp.rs` recognises no command by spelling (the intrinsics are
  typed); no consumer matches `"unset"`.
- R2 to R5; no new source file.
- R6: every existing existence test byte-identical; the O109 and S100
  expectations move only for the witnesses named.
- Suites: `cargo test -p tcl-registry -p tcl-compiler -p tcl-explorer -p
  tcl-lsp-db -p tcl-cli -p xtask`.
- R7: `Unavailable` is never read as `Unbound`; a cross-event or aliased
  place is never `Unbound` at entry; a refinement never survives a
  barrier or an up-frame; an absent cell's value is never manufactured.

#### Behavioural deltas

| Delta | Mandate |
|---|---|
| the existence branch prunes its dead arm for O107, taint, shimmer and the gated checks | § *Existence*, "becomes one path" |
| a second `unset` after a killed version is a definite W213 | the exit; the W210 row |
| O109 and O108 keep stores behind an existence read | the O108 / O109 rows; #2132 |
| S100 skips an unset arm | #2133; the S100 row |
| `incr` of an absent place binds under 8.5 onwards, declines under 8.4 | the absent-cell release rule |
| an absent-start `lappend` chain folds (O130) | the O104 / O130 row |

#### Record (2026-09-24): the opus items of slice 8

One implementer runs the opus items in the order the coordinator gave —
VT8.1, VT8.6, VT8.8 (the plan's first checkpoint), VT8.2, VT8.3 (the
second), VT8.4, VT8.5, VT8.7, VT8.9 (the third) — each its own checkpoint
commit, as slices 4 and 5 did; VT8.10 and VT8.11 are the sonnet
implementer's. The decisions are D157 onward in § *Decisions taken*. The
slice 5 review's fixes landed as their own commit after VT8.1, before any
item reads a preserve outcome (§ *Slice 5* › *Record (2026-09-24): review
fixes for slice 5*, D162–D164).

| Item | Commit | What landed | Its tests |
|---|---|---|---|
| VT8.1 | `wip(value-transfers): slice 8 — the existence domain in the solver` | The existence rung in the solver (`sccp.rs`): a forward fact per place — the registry's `Existence`, with `Existence::join` and `after` and `BindingKind::join` added in `answers.rs` — run beside the values over the same executable blocks and edges when the caller passes `TraceInputs::existence` (`ExistenceEntry`: the parameters, the object state, the initial global frame, the iRules connection-scoped names, the module's own computed-trace fact, the document's lexer config); `SccpResult::existence` per SSA version (version 0 its entry fact, a φ the join of its executable edges), `existence_reads` per statement read and `existence_exits` per block exit (D158). Entry: parameters `Bound(Scalar)`; qualified, escaping (scope aliases and traced names included) and computed-trace places `MayBound` wherever read; `TclOO` instance variables, a `when` handler's connection-scoped names (`AnalysisContextKey::connection_scoped`, every name a handler of the module binds, so the memoised lattice re-keys) and an element of a held array `MayBound`; in the initial global frame a special variable bound as its registry kind when startup binds it (`CommandRegistry::is_initially_bound`, D157), `MayBound` otherwise; the caller-frame barrier from the entry (D160). Transfer: a typed assignment binds its place (an element write the element as a scalar and its array as an array, a fanned element may-bind); a call takes its evaluated outcome's per-store steps (`ExistenceStep`, `value_transfer.rs`), else its declaration's own `transfer(Existence)` on the normal completion, else widens to `MayBound` (D159); the loop header binds on a proven non-empty list and preserves on a proven-empty one; a `Barrier` / `UpFrame` makes every place `MayBound`; a computed name applies from its own statement on (`dynamic_names::statement_barrier`, `terminator_barrier`); an inline nested body makes what it defines `MayBound`; an exception edge carries every point of its handler's region (D161). The release rule (`cell_update.rs`): an `Unbound` place is an absent cell, run over no prior value only where every release the target names creates it (`creates_absent`), else `UnboundPlace`; `LatticeInputs::prior_store` / `variable` answer `FactDomain::Existence` from the driver's cursor. The hand-over: the taint seed and the existence post-pass read the registry's special variables, a pack-declared one included (D157). Not done here: `info exists` still decides only in the post-pass (VT8.2) | `the_absent_cell_release_table` (compiler witnesses: every line of the page's table but the `unset p nosuch q` prefix line under `tcl8.4` to `tcl9.1` and the spanning `tcl`, each fact checked against tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1b0 wherever the line completes; `incr fresh`, `incr fresh 2` and `incr arr(k)` bind from 8.5 and decline `unbound-place` under 8.4 and `tcl`); `an_absent_cell_is_created_where_every_release_creates_it` (`tcl-registry` `value_transfers.rs`); changed by the mandate (the release rule): `sccp_records_a_route_explanation_per_statement` reads `append s x` over an unbound `s` as evaluated (the table's `append s foo` is `foo` on every release), and `keyed_updates_agree_on_both_paths` (`tcl-lsp-db`) reads `dict set d a 1` over an unbound `d` as `a 1` on both paths — tclsh 8.5 to 9.1 print `a 1` — where D36 pinned the decline below the rung; every other test unchanged |
| VT8.6 | `wip(value-transfers): slice 8 — an unbound arm carries no representation` | `evaluate_type_def` (`type_infer.rs`) types a destroying command's whole-variable kill `TypeLattice::unknown()` — `kills_whole_variables`: the call's spec carries `Traits::DESTROYS_VARIABLE` and a word spells each definition whole, so `unset x` qualifies and `unset a(k)` keeps the typing it had — where it took `unset`'s empty-string return type and a φ joined that string with the bound arm's int. `find_phi_shimmers` (`shimmer/phi.rs`) takes the `SccpResult` and `classify_incoming_types` skips an incoming version whose existence (`SccpResult::existence`) is `Unbound`, so an arm on which the variable does not exist brings no intrep to the merge whatever its definition is typed. The S100 KCS note gains "A branch that unsets the variable"; the design page's S100 bullet states the built behaviour. Mandate: #2133, the S100 row | `s100_ignores_an_unset_arm` (`set x 1; if {$c} {unset x}; puts $x`: no S100 or S101 on `x`, W210 kept); `s100_still_fires_between_the_bound_arms_of_a_three_way_switch` (arms `set x 1`, `set x "hi"`, `default {unset -nocomplain x}`: one S100 merging int and string); `phi_shimmer_emitted_for_int_string_merge` and the other phi tests unchanged but for the new argument |
| VT8.8 | `wip(value-transfers): slice 8 — const, array unset, array default` | `ConstWriteSemantics` (`value_transfer/cell_write.rs`, `NativeEvalId::ConstWrite`, `"const-write"`): over an `Unbound` place a `Write` of the value and the empty string as the result; `Pending` is pending; any other fact declines `Unsupported`, since an existing variable raises and an existing constant keeps its value; the existence transfer is `Bind(Scalar)` on the normal path. `ArrayUnsetSemantics` (`value_transfer/unbind.rs`, `EvalRoute::None { Unauthored }`): without a pattern it reads the prior fact — `Bound(Array)` unbinds, `Bound(Scalar)`, `Unbound`, `MayBound` and `Pending` preserve, `Bound(Either)` keeps the generic widening; with a pattern the array's place is preserved. `ARRAY_DEFAULT` (`builtins.rs`): a `MayWriteSemantics` of the name as an array, `NoRouteReason::Declared`. `const_.rs` and `array_.rs` declare them; the three `KNOWN_GAPS` rows go; `tcl-spectcl`'s `NATIVE_EVAL_IDS` catalogue gains `ConstWrite`, so a pack may name `evaluate -direct ConstWrite`. Deviation: the item says `array unset arr` is an `Unbind` of the array; every release leaves a scalar in place (`set s 1; array unset s` keeps `s`, never raising), so the pattern-less form unbinds only a place proven an array. The oracle test is a compiler witness, not a `differential_fold.rs` row: `LiteralInputs` answers no existence fact, so the route declines every literal-input call, as R7 requires. `sccp-core-analyses.md` states the three | `const_binds_only_an_absent_place` and `array_unset_unbinds_only_an_array` (`tcl-registry/tests/value_transfers.rs`, new); `const_writes_only_an_absent_place` (`value_transfer_witnesses.rs`, new: `const c 5; puts $c; const c 7; puts $c` prints `5` twice before and after `tcl opt` under tclsh 9.0 and 9.1, the lattice holding 5 after the first and no value after the second; `set x 1; const x 2` raises there and the route declines); `route_stamps_match_the_pinned_set` gains `array default`, `array unset` and `const` |
| VT8.2 | `wip(value-transfers): slice 8 — info exists decides inside the fixed point` | The driver's `run_script` (`value_transfer.rs`) answers an invocation whose resolved operation is `IntrinsicId::InfoExists` or `ArrayExists` (`existence_query::kind_of`, shared with `in_text`) from the rung at the current point, through `LatticeDriver::existence_answer`, before any route: `Bound(_)` is 1 to `info exists`, `Bound(Array)` 1 and `Bound(Scalar)` 0 to `array exists`, `Unbound` 0 to both; an element query reads its array and decides 0 only on an unbound one, so a computed key on a bareword array (`Params($k)`, `existence_query::computed_element_base`, the old `array_element_base` test) reads its array too; `Pending` is pending; `MayBound`, `Bound(Either)` for `array exists`, `Unavailable`, any other computed name and a registry special variable in the initial global frame decide nothing (D165). The nested service and a value-position `[info exists x]` reach it alike, so `evaluate_branch` decides the condition inside the fixed point and the branch is an ordinary `Applied` fact that updates `executable_blocks`. A name the function only asks about takes a rung slot past the SSA's symbols (`query_only_places`, handed to the driver as `existence_places`), so its entry rule and every clobber reach it. Gone: `existence_constant_branches`, `scan_defined_and_unbound` and `ExistenceFrame` (`sccp.rs`), the post-pass in `FunctionUnit::build` and `drop_cross_event_existence_folds` (`compilation_unit.rs`), `emit_existence_constant_branch_diagnostics` (`dataflow.rs`) and its call, and `value_transfer::unbound_names` with the ledger's `unset` row. Deviation: the reachability gate in `emit_constant_branch_diagnostics` stays — it now serves the existence branch too, which is the "one path" the item asks for, and removing it would let I230 say an arm reached by another edge is unreachable. `BranchFactKind::Proven` stays in the enum with no producer. `sccp-core-analyses.md` describes the in-fixpoint decision | `set_unset_info_exists_decides_zero` (`sccp.rs`, new: the branch is `Applied`, value false, its true arm unreachable); `a_computed_key_leaves_a_bareword_array_fixed` (`existence_query.rs`, new); `the_existence_branch_fact_is_stored_once` moves (mandate: the item's "ordinary `Applied` branch") — the `[info exists b]` fact is `Applied`, its dead arm unreachable, and the emitter it pinned is gone; `info_exists_does_not_fold_unset_parameter` moves to `info_exists_folds_an_unset_parameter_false` (mandate: the rung reads the `unset`, so the guard folds always false — tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1b0 print 0 for `proc f {a} { unset a; info exists a }; f 1` — where the old test pinned the post-pass's missing "always true" by the absence of any I230); three `core_analyses.rs` tests that pinned the post-pass's flow-insensitive non-folds move to the rung's verdicts (mandate: the query reads the place's fact where it runs; tclsh 8.4.20 to 9.1b0 agree on each) — `lazy_init_reuse_branch_is_dead_for_local_diverges` becomes `lazy_init_reuse_branch_is_dead_for_local` (`H` is unbound at the check, so "always false"), `set_before_check_diverges_no_fold` becomes `set_before_check_folds_true`, and `tn_unset_parameter_abstains` becomes `unset_parameter_follows_the_rung` (`unset a; array set a {x 1}` then `array exists a` decides true, `unset a` alone false); `a_condition_substitution_reads_the_frames_variables` (`optimiser.rs`) moves its `info exists` program out of the no-rewrite loop (mandate: the decided query folds) — the condition folds to `1` (O101) and the store it read stays, which still prints `yes`; every other `existence_fold_abstains_*`, `upframe_body_models_*` and `info_exists_*` test, and `i230_existence_fold_abstains_on_interpreter_globals_at_top_level`, unchanged |
| VT8.3 | `wip(value-transfers): slice 8 — the guard as an edge refinement` | `EdgeRefinement` (`sccp.rs`, the page's shape: `edge`, `key`, `domain` — `FactDomain::Existence` in this slice — `fact`, `evidence`), built per run by `edge_refinements` from each branch whose condition states an existence fact (`condition_facts`, the page's table): `info exists` binds its true edge `Bound(Either)` and unbinds its false edge, `array exists` binds its true edge `Bound(Array)`, `!` swaps, `&&` states both true-edge answers and `\|\|` both false-edge answers; a literal element's guard binds the element as a scalar and its array as an array on the true edge and unbinds the element on the false edge, a computed key only the bareword array on the true edge (`query_facts`). `ExistenceRun::block_entry` narrows each refined place as the edge arrives (`arriving`, `narrowed`: a `MayBound` place to the fact, a `Bound(Either)` one to a kind; a contradicted fact, on an edge D165 left undecided, keeps the place). Every place the rung carries is refined, with no exclusion list (D166): a definition of an externally mutable place still steps to `MayBound`, and a barrier or an up-frame clobbers every place, so the refinement ends there — VT8.1's "`MayBound` wherever read" for such a place now reads "`MayBound` at entry, after every definition and after every barrier or up-frame". `SccpResult` gains `refinements` and the block-qualified `existence_entries` — the fact the version live at a block's entry holds there, where a refinement or a clobber made it differ from the version's own (`ExistenceRun::block_qualified`, over each block's recorded entry state after the run) — read through `SccpResult::existence_at(block, key)`; every literal `SccpResult` in the tests gains the two fields. The slice's R7 line takes D166's form. `collect_existence_guards` keeps its three callers (D8). `sccp-core-analyses.md` states the refinement | `the_existence_guard_refines_its_edges` (`sccp.rs`, new: the guarded read of a conditionally set `x`, and of a `global x`, is `Bound(Either)`, the other arm holds it `Unbound` through `existence_at`, `!` swaps the arms, and past the merge it is `MayBound`); the coordinator's witnesses, all `sccp.rs` and new — `the_refinement_alone_reads_the_idioms_bound` (`puts $::errorInfo` under its guard at the top level and in a procedure, and a `TclOO` instance variable's `return $x`, read `Bound(Either)` through the refinement alone), `a_barrier_ends_the_refinement` (the instance variable reads bound before `eval $script`, which lowers to a barrier, and `MayBound` after it) and `a_negated_guard_refines_a_special_variable_on_one_edge` (`![info exists errorCode]` at the top level: one edge `Unbound`, the other arm bound, `MayBound` past the merge); `info_exists_guard_narrows_read_in_then_arm`, `info_exists_negated_guard_narrows_false_arm` and `info_exists_read_outside_guard_still_flags_w210` unchanged |
| VT8.4 | `wip(value-transfers): slice 8 — the lifecycle diagnostics read the fact` | W210 reads the existence rung where the read runs (`place_fact`: the statement's own read, or the block's exit for a terminator) and reports only at an `Unbound` or `MayBound` place, never at `Bound(_)` or where the run computed no fact; the `return` pass reads the exit likewise. W213 is recognised through `Traits::DESTROYS_VARIABLE` under the source spelling (`destroys_variable`, `get_exact`, so a rooted `::unset` stays silent as `w213_qualified_and_aliased_do_not_fire_rust_behaviour` pins) and reads the fact at the `unset`: definite ("does not exist here") on `Unbound`, "may not exist" on `MayBound`, nothing on a bound place; `unset -nocomplain` reports neither W213 nor W210 (the W210 on `unset -nocomplain x` of a never-set `x` goes). IRULE4005's unbind skip and `build_phi_undef_index`'s killed set read the same trait. The coordinator's additions: W210 is one per variable across both passes (`emit_read_before_set_diagnostics` returns what it reported; `ReturnUndefCtx::already_reported`), so `set v 1; unset v; return $v` draws one; `collect_expr_cmd_sub_writes` records where each `expr` substitution writes (`(block, statement)`) and a read is suppressed only at or after that write in a dominated block (`UndefSuppression::written_by_substitution_before`), the branch-condition half dropped because the `<cond>` statement already defines those names in SSA; it and `collect_script_concat_writes` take the analyser's registry in place of a hard-coded `tcl8.6` one, and the shared harvest (`ir_helpers::cmd_substitution_out_vars`) asks whether the release has the command, so a command the release lacks writes nothing — as committed through `has_command_in_this_dialect`, which `cargo xtask retired-api-gate`'s one-oracle rule (R10) keeps to the registry and codegen, so VT8.5's commit asks the availability owner instead: the invocation resolved under the registry's own surface (`resolve_invocation(…, own_surface_query())`), `None` for a head the profile lacks. The hand-over (D157): `startup_read_facts`, both `is_externally_read` sites (now `Analyser::externally_read`) and `StartupFacts::for_name` (through `PhiUndefCtx::registry`, also on `ReturnUndefCtx`) read the registry's faces, pack rows included. W211 and W214 already count an existence read and an unbind as a use through the `<cond>` statement's reads and the unset's use; `an_existence_read_or_an_unbind_is_a_use` pins it. A read in an arm the run proved dead carries no fact, so W210 no longer reports it there (the migration page's "fewer false positives in dead arms"). VT8.3's `&&` / `\|\|` composition now keeps the left operand's facts only when the right operand, which runs after it, changes no place (`existence_pure`, `sccp.rs`: every command it substitutes is an existence query over a name that runs none; a math function, which may be a procedure, is not) — `and_impure_right_drops_left_fact` caught the unsound case once W210 read the fact. G1: `dataflow.rs` 2 → 0, into `CLEAN_FILES`; `helpers.rs` 5 → 4 (the ledger names slice 13's binder checks). G2 (`registry-axes`): the registry faces replace free-function reads, so `dataflow.rs` 9 → 7 and `helpers.rs` 6 → 5. The W210 and W213 KCS notes describe the fact | `a_second_unset_is_a_definite_w213`, `a_conditional_unset_gives_w210`, `nocomplain_never_reports_w213`, `a_killed_return_read_reports_once`, `a_read_before_the_conditions_write_still_reports` (a `catch` in a condition and in an `expr` word), `a_concatenated_script_writes_through_the_documents_registry` (`eval lassign {1 2} a b; puts $a`: silent under `tcl8.6`, one W210 under `tcl8.4`, where tclsh 8.4.20 raises `invalid command name "lassign"`), `an_existence_read_or_an_unbind_is_a_use` and `a_guard_under_and_narrows_the_read` (analyser `tests.rs`, all new; every program checked against tclsh 8.4.20 to 9.1b0); `an_impure_operand_ends_the_facts_before_it` (`sccp.rs`, new); two `core_analyses.rs` tests move — `and_pure_right_keeps_both_facts_diverges` becomes `and_pure_right_keeps_both_facts` (mandate: the design table's `C1 && C2` row; tclsh reads both names only when both exist, which the old name marked a divergence), and `and_impure_right_drops_left_fact` keeps its assertion over an `X` set on one path (mandate: with `X` never set the guard decides false and the arm is dead, where W210 reads no fact — tclsh never runs it); both also pin that the never-set form reports nothing; every `emit_cfg_ssa_diagnostics_w210_*`, `w213_*` and `info_exists_*` test unchanged |
| VT8.5 | `wip(value-transfers): slice 8 — O108 and O109 keep what an existence read observes` | A store is removable only when no value read and no existence read of its version remains, and an existence read is an SSA use in every position the lowering materialises (D167). The registry's read projection (`CommandRegistry::variable_read_projection`) names a `DESTROYS_VARIABLE` command's targets beside its `VarRead` words, so the one consumer of it (`ir_helpers::variable_read_effects_from_commands`) records a nested `[unset x]` — in an argument, a value word, a condition or a `return` word — as a read of the version it observes, and code sinking refuses to move a store past one: #2220's condition case and the statement rule are one implementation. A nested unbind gets no kill definition (D167). `array unset` carries `Traits::CONDITIONAL_VARIABLE_WRITE` (D168: every release leaves a scalar, an absent variable and every element the pattern misses in place), so the SSA reads the prior version at a statement's and a nested `array unset` alike. W210 treats a nested destroyer's target as an existence word (`existence_query_vars`, `ir_helpers::destroyed_variables`), so `puts [unset -nocomplain x]` of a never-set `x` draws nothing, as before; W211 no longer reports the store a nested `[unset x]` needs. `collect_rmw_hidden_reads` drops every name the SSA records where the word runs — a statement's own uses, and those of the synthetic statements the lowering pushes ahead of a host or a `return` terminator under its span (D169) — so what is left is what the SSA cannot see. An unbind statement is a `Call`, which `assignment_safe_to_delete_with_effect` never deems deletable, so "an unbind statement is never removed" held already; its doc and the module doc say so, and the witness pins it. The coordinator's gate fix rides here: `cmd_substitution_out_vars` asks the availability owner (the invocation resolved under the registry's own surface) instead of `has_command_in_this_dialect`, which `cargo xtask retired-api-gate` (R10) keeps to the registry and codegen (the VT8.4 row). The O108, O109, O126 and W211 KCS notes and the design page's W211 and O108 / O109 bullets state the built rule. Deviations: the item names `elimination.rs` alone; the existence read it asks for is recorded where the SSA's uses are made, so `ir_helpers.rs`, `registry.rs`, `array_.rs`, `traits.rs` and `dataflow.rs` (the W210 existence word) are edited too, as the slice's § *Upstream starting point* assigns VT8.5 "the existence read to every position". Found and left, value and existence reads alike (tclsh 8.6.18 prints what the original does, the rewrite does not): a script body nested in a substitution records no read or write (#2231, VT10.3: `set x 1; puts [catch {unset x}]` loses `set x 1`, and `[eval {info exists x}]`, `[lmap v {1} {unset x}]` the same); an `uplevel 0 {…}` body records none (`UpFrame`: `set x 1; uplevel 0 {info exists x}` loses its store to O126, `set x 1; uplevel 0 {puts $x}; set x 2` to O109); a `foreach` list word's substitution effects are not materialised (`set n 1; foreach v [incr n] {}; puts $n` rewrites to `puts 1`; `foreach v [unset x] {}` loses its store); a nested unbind's kill is no definition (`set x 1; puts [unset x]; puts $x` rewrites to `puts 1` by O102, where tclsh raises `can't read "x"`); a procedure's last command's value is no use (`proc p {} {set y 5; set y}` loses `set y 5` to O126); and a dead `incr` under 8.4, which raises on an absent place, is still removed (permission 3's totality proof) | `o109_keeps_a_store_an_existence_read_observes` (compiler witnesses, the draft completed: the item's two refusals — `set x 1` and `incr n` behind `[info exists …]` survive one pass under `tcl8.4` to `tcl9.0` and `tcl` / `tcl8.6` and `tcl9.0`, and both print `yes` before and after the multipass optimiser, the `incr` program from 8.5 — and fourteen positions — a bare statement, a `catch` body, a value word, a `return`, `array exists`, an `expr` word, a statement and a conditional `unset`, a nested `[unset x]` in an argument, a value word, a condition and a `return`, `array unset` of a scalar as a statement and nested — each keeping its store under `tcl8.4`, `tcl8.6` and `tcl9.0`, every unbind surviving the multipass optimiser, and the program printing `1 1 1 1 1 yes 0 1 0 0 0 0 {} 1 1` before and after it under tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1b0); `hidden_reads_are_what_the_ssa_does_not_record` (`elimination.rs`, new); `a_nested_unbind_is_an_existence_read` (analyser `tests.rs`, new: no W211 on the store a nested `[unset x]` needs, no W210 on a nested `[unset x]` or `[unset -nocomplain x]` of a never-set `x`); `variable_read_projection_names_a_destroyers_targets` (`registry.rs`, new); one test moves — `issue_1078_braced_double_store_matches_the_plain_control` (`fp/rbs.rs`; mandate: the item's "`collect_rmw_hidden_reads` shrinks to what the SSA does not already record", D169): the `return` word's `[set n]` was a name-level hidden read that silenced W220 on both stores, and the SSA records it as a use of the second store's version alone, so both spellings now draw the same one W220, on the first store, which tclsh 8.4.20 to 9.1b0 return 2 without and O109 already deleted; every other test unchanged |
| VT8.7 | `wip(value-transfers): slice 8 — Unavailable at the fast tier and past the ceiling` | The existence rung is a deep-tier fact and its every read is typed (D170). `FunctionUnit::tier` records the tier the unit's lattices ran at — the context key's (`AnalysisContextKey::at_tier` makes a request below the deep tier), `ComplexityGuarded` for a trivial guarded unit — and `FunctionUnit::existence(symbol, ExistencePoint)` answers the registry's `FactView`: `Domain(Existence(_))` where the run computed a fact, `Pending` where it never reached the point, `Top(Unavailable(tier))` below the deep tier or past the ceiling. A build under a key below the deep tier runs no rung (`build_full`), so the lattice driver answers an existence query inside it `Unavailable(tier)` too and `info exists` decides nothing. W210, W213 and the `return` pass read through the view (`place_fact`, `reportable`: only `Unbound` and `MayBound` report), so each stays silent on `Unavailable`; S100 reads the per-version map its producer hands it, where an absent entry is never `Unbound`; O108 and O109 read SSA uses, not the fact, and skip a guarded unit whole. A consumer with no SSA: the detached inputs already answered `Unavailable` for an existence read, and the literal-word inputs (`LiteralInputs::variable`) now do too, at the structure tier, where they answered `NotExact`. The Explorer's `semantic` view shows the tier beside `complexityGuarded`, since its durable-field inventory asks every field for a row. The design page's availability paragraph states the built read. Deviations: `dataflow.rs` (the consumers), `literal.rs` (the no-SSA consumer) and the Explorer's `coverage.rs` and `serialise.rs` (its durable-field witness does not compile past a new field) are edited beside the item's two files | `existence_is_unavailable_at_the_fast_tier` (`compilation_unit.rs`, new: `proc p {} {set x 1; unset x; unset x; puts $x}` — the deep unit computes the fact and the analyser reports W213 and W210; the same procedure rebuilt under a fast-tier key has no existence map, answers `Unavailable(Fast)` at every point, and the analyser, handed that unit, reports neither; a trivial guarded unit answers `Unavailable(ComplexityGuarded)`); `literal_inputs_answer_existence_unavailable` (`tcl-registry` `value_transfers.rs`, new); every existing test unchanged |
| VT8.9 | `wip(value-transfers): slice 8 — the Set analyser hook retires` | `AnalyserHookId::Set` is gone: from `hooks.rs`, from `set_.rs`'s stamp, from the pinned set in `analyser_hooks.rs` (31 variants, 42 stamp rows) and from `tcl-spectcl`'s `ANALYSER_HOOKS` tables, so a pack may no longer name `analyser_hook -native Set`; `fields.md` is regenerated. `set`'s two-word form is defined by the generic role binding (`handle_var_binding_command`, over `VarWrite`), then by `Analyser::bind_value_word_assignment`, which runs in the dispatch tail for every invocation whose resolved semantics is the direct one-target write of a value word (`ResolvedSemantics::writes_value_word`, new in `declaration.rs`: the route `Direct { CellWrite }`) and escalates the definition's `warn_if_unused` to the assignment's `true` (D171). The constant-string environment reads the value word's `CellWrite` evaluation over the call's literal words with the resolver's roles (`value_word_write`, `LiteralInputs`) when the word is one literal token; otherwise a value word creating an interpreter binds the name to its interpreter-domain key (Q5, reached from the generic binding), a folded `[cmd]` binds its constant, and anything else clears both. `set auto_path …`'s record reads the same predicate, in the analyser and in the signature scan (`walker.rs`). The one-word read form was already the walk's `VarRead`-role reference pass's. The four `set VAR [CLASS new]` instance-tracking sites in `commands.rs` read the registry's handle-binding layout (`CommandRegistry::handle_binding`, `HandleClassSource::ConstructionValue`) through `construction_value_binding` (D172), so the file joins `CLEAN_FILES` (G1: `commands.rs` 4 → 0) and its four `until slice 8` registry-axes waivers go with the sites (40 → 36 waived). Behaviour, each change checked against tclsh 8.4.20 to 9.1b0 where Tcl can show it: a literal value word is recorded cooked, as Tcl reads it — `set p "a\\d"` holds `a\d` in every release, where the hook kept the token's raw text; a computed target (`set $n 1`, which writes the variable `n` names) defines no variable, where the hook defined `n`; and a rooted `::set d [Dog new]` is tracked as an instance of `::Dog`, as `set`'s is, where the tracking compared the spelling (8.6.18 to 9.1b0). A rooted `::set`'s constant was bound through the hook's resolved dispatch and still is. A pack command declaring `evaluate -direct CellWrite` takes the same binding, its target an assignment; its constant is what the declaration's own evaluation writes, and a pack's named route evaluates nothing yet (the lattice driver declines it `Unsupported` too), so it records none. The design page's analyser-hook answer and the migration page's object-binding survey line state the retirement. The consumer-contracts lane doc's CC2.13 record still says "`Set` keeps `handle_set_command`"; it is that step's history and is not edited. Of that lane's code only what the retirement needs is touched: the binder's doc paragraph on the double binding and `record_search_path_write`'s doc (`handlers.rs`), the signature scan's new arm beside CC2.13's `lappend` one, the two `ANALYSER_HOOKS` tables and the pinned set in `analyser_hooks.rs`. Deviations: beyond the item's four files, `commands.rs` (the dispatch tail, the instance tracking), `scope.rs`, `walker.rs` (the signature scan's `set auto_path`), `declaration.rs` (the predicate), `tcl-spectcl`'s `loader.rs` and `catalogue.rs` (the hook tables), two comments in `tcl-lsp-core` and the G1 files are edited, since each named the hook or `set`'s spelling, and `state.rs` and the compiler's `tests/analyser.rs` hold new witnesses | `a_literal_value_word_is_recorded_as_tcl_reads_it` and `a_computed_set_target_defines_no_variable` (`handlers.rs`, new: `set p "a\\d"` holds `a\d`, and `set $n 1` defines no `n`, as tclsh 8.4.20 to 9.1b0 read them); `analyse_records_instance_class_rooted_set_new` (`state.rs`, new: `::set d [Dog new]` binds `d` to `::Dog`, as `$d bark` dispatches under tclsh 8.6.18 to 9.1b0); `a_rooted_set_propagates_its_constant_pattern` (compiler `tests/analyser.rs`, new: the retirement keeps `::set pat {^\d+$}`'s pattern for the later `regexp`, which is 1 under every release); the six `handle_set_*` tests in `handlers.rs` keep their names and assertions and move off the deleted `handle_set_command` onto `dispatch_tokens`, which dispatches the whole command through `process_command` (mandate: the item's "every `handlers.rs` test of `set`" preserved) — `handle_set_defines_variable` also asserts the definition is an assignment (`warn_if_unused`), and `handle_set_no_value_records_read_not_definition` reads the one-word form's reference from a whole-file analysis, since the walk's `VarRead`-role pass, which runs after `process_command`, records it; the pinned set in `analyser_hooks.rs` re-baselined without `("set", "", Set)`, and `tcl-spectcl`'s catalogue test asserts `covered_analyser(Proc)` where it named `Set` (mandate: the item's "`AnalyserHookId::Set` leaves the pinned set"); the regex-source tests of a `set`-held pattern (`set_then_regexp_and_regsub_propagate_constant_pattern`, `set_then_switch_regexp_propagates`, `variable_pattern_in_proc_scope`, `literal_and_variable_patterns_mixed`), the `interp create` bindings (`interp_value_flow`, all thirteen) and every `analyse_records_instance_class_*` test unchanged |

Green at VT8.1:

- tests: `tcl-compiler` 9751 passed, 6 ignored across its 67 binaries, and
  7 doctests; `tcl-registry` 1241; `tcl-lsp-db` 128, 5 ignored;
  `tcl-explorer` 103; `tcl-cli` 128; `tcl-lsp-core` 3567; `xtask` 235 —
  no failure;
- pedantic clippy (`--all-targets --no-deps -D warnings`) on
  `tcl-registry`, `tcl-compiler` and `tcl-lsp-db`, no `#[allow]` added;
  `cargo fmt` on the three;
- `cargo xtask value-transfers` (no pin moved, no generated page changed)
  and `--check` (20 clean, 16 waived, 83 pinned across 34 files, 6607
  rows); `registry-axes --check` (956 pinned across 157 files, 5 waived,
  6 clean); `dialect-drift` 9 sites, the eight upstream ones and the
  slice 5 review's `value_transfer.rs` split, none new; `cargo check
  --workspace` clean.

Green at VT8.6: `tcl-compiler` 9755 passed, 6 ignored across its 67
binaries, and 7 doctests; `tcl-lsp-core` 3567 — no existing test
moved; pedantic clippy on `tcl-compiler`, no `#[allow]` added, and `cargo
fmt`; `value-transfers --check` (20 clean, 19 waived, 90 pinned across 36
files, 6607 rows) and `registry-axes --check` (956 pinned across 157
files) unchanged; `dialect-drift` 8 sites, none new; `kcs-index-links`
passes; `cargo check --workspace` clean.

Green at VT8.8: `tcl-registry` 1244 passed; `tcl-spectcl` 308, 1 ignored;
`tcl-compiler` 9756 passed, 6 ignored across its 67 binaries;
`tcl-lsp-core --lib` 2349; `xtask` 237 — no existing test moved; pedantic
clippy on `tcl-registry`, `tcl-spectcl`, `tcl-compiler` and `xtask`, no
`#[allow]` added, and `cargo fmt`; `value-transfers` (the inventory's
three rows gain their declarations; 20 clean, 19 waived, 90 pinned across
36 files, 6607 rows); `registry-axes --check` (956 pinned across 157
files, unchanged); `pack-goldens` (24 packs, 0 rewritten); `dialect-drift`
8 sites, none new; `cargo check --workspace` clean.

Green at VT8.2: `tcl-compiler` 9758 passed, 6 ignored across its 67
binaries, and 7 doctests; `tcl-lsp-core --lib` 2349; `tcl-lsp-db --lib`
102; the `tcl-lsp-server` e2e diagnostics, iRules, precision-review,
code-action, diagnostic-matrix and spec-pack modules 580; `tcl-cli`
`cli` and `value_transfers_cli` 57 — the moved tests are the row's;
pedantic clippy on `tcl-compiler`, no `#[allow]` added, and `cargo fmt`;
`value-transfers` (no pin moved, no generated page changed) and
`--check` (20 clean, 19 waived, 90 pinned across 36 files, 6607 rows);
`registry-axes --check` (956 pinned across 157 files, unchanged);
`owner-resolution` passes; `dialect-drift` 8 sites, none new; `cargo
check --workspace` clean.

After the consumer-contracts step 2 merge (`b39e012b`, over VT8.2):
`cargo check --workspace` clean; `tcl-compiler` 9756 passed, 6 ignored
across its 67 binaries (the merge retired three `try` hook tests and
added one); `tcl-registry` 1244; `value-transfers --check` unchanged;
`registry-axes --check` at the merge's own baseline (896 pinned across
147 files, 40 waived, 16 clean); `dialect-drift` 8 sites — nothing to fix.

Green at VT8.3: `tcl-compiler` 9760 passed, 6 ignored across its 67
binaries, and 7 doctests; `tcl-lsp-core --lib` 2350; `tcl-lsp-db --lib`
102; the `tcl-lsp-server` e2e diagnostic modules 580; `tcl-cli` `cli`
and `value_transfers_cli` 57 — no existing test moved; pedantic clippy
on `tcl-compiler`, no `#[allow]` added, and `cargo fmt`;
`value-transfers` (no pin moved, no generated page changed) and
`--check` (20 clean, 19 waived, 90 pinned across 36 files, 6607 rows);
`registry-axes --check` (896 pinned across 147 files, unchanged);
`owner-resolution` passes; `dialect-drift` 8 sites, none new; `cargo
check --workspace` clean.

Green at VT8.4: `tcl-compiler` 9769 passed, 6 ignored across its 67
binaries, and 7 doctests; `tcl-lsp-core --lib` 2350; `tcl-lsp-db --lib`
102; the `tcl-lsp-server` e2e diagnostic modules 580; `tcl-cli` `cli`
and `value_transfers_cli` 57; `xtask` 237 — the moved tests are the
row's; pedantic clippy on `tcl-compiler` and `xtask`, no `#[allow]`
added, and `cargo fmt`; `value-transfers` (the generated page's ratchet
section moves `dataflow.rs` into the clean list) and `--check` (21
clean, 19 waived, 87 pinned across 35 files, 6607 rows);
`registry-axes` regenerated and `--check` (893 pinned across 147
files); `kcs-index-links` and `owner-resolution` pass; `dialect-drift` 8
sites, none new; `cargo check --workspace` clean.

Green at VT8.5: `tcl-registry` 1246 passed across its binaries;
`tcl-compiler` 9772 passed, 6 ignored across its 67 binaries, and 7
doctests; `tcl-lsp-core --lib` 2350; `tcl-lsp-db --lib` 102; the
`tcl-lsp-server` e2e diagnostic modules 580; `tcl-cli` `cli` 50 and
`value_transfers_cli` 7; `tcl-explorer` 103; `xtask` 237 — the moved
test is the row's; pedantic clippy on `tcl-registry` and `tcl-compiler`,
no `#[allow]` added, and `cargo fmt`; `value-transfers` (no pin moved, no
generated page changed) and `--check` (21 clean, 19 waived, 87 pinned
across 35 files, 6607 rows); `registry-axes --check` (893 pinned across
147 files, unchanged); `pack-goldens` (25 packs, 0 rewritten);
`retired-api-gate` passes; `kcs-index-links` passes; `dialect-drift` 8
sites, none new; `cargo check --workspace` clean.

Green at VT8.7: `tcl-registry` 1247 passed across its binaries;
`tcl-compiler` 9773 passed, 6 ignored across its 67 binaries, and 7
doctests; `tcl-lsp-core --lib` 2350; `tcl-lsp-db --lib` 102; the
`tcl-lsp-server` e2e diagnostic modules 580; `tcl-cli` `cli` 50 and
`value_transfers_cli` 7; `tcl-explorer` 103; `xtask` 237 — no existing
test moved; pedantic clippy on `tcl-registry`, `tcl-compiler` and
`tcl-explorer`, no `#[allow]` added, and `cargo fmt`;
`value-transfers --check` (21 clean, 19 waived, 87 pinned across 35
files, 6607 rows, unchanged); `registry-axes --check` (893 pinned
across 147 files, unchanged); `pack-goldens` (25 packs, 0 rewritten);
`retired-api-gate`, `owner-resolution` and `kcs-index-links` pass;
`dialect-drift` 8 sites, none new; `cargo check --workspace` clean.

Green at VT8.9: `tcl-registry` 1247 passed across its binaries;
`tcl-compiler` 9777 passed, 6 ignored across its 67 binaries, and 7
doctests; `tcl-spectcl` 309 passed, 1 ignored; `tcl-spec-studio` 292;
`tcl-lsp-core` 3568 passed across its binaries (the library's 2350
among them) and 3 doctests; `tcl-lsp-db --lib` 102; `tcl-cli` `cli` 50
and `value_transfers_cli` 7; `tcl-explorer` 103; `xtask` 237; the whole
`tcl-lsp-server` e2e suite, 1598 passed and 5 ignored, with one failure
that is not this item's:
`rename_safety::fp_namespace_variable_rename_refuses_beside_a_computed_alias_cell`
fails the same way at VT8.7's commit, and the consumer-contracts lane's
CC2.13 row records it as pre-existing there. Its cause: `alias_cell`
(that lane's step 2, `dbb886ea`) names no cell for `namespace upvar $ns
version local`, where the retired handler linked the local to the
computed text, so the workspace index sees no computed alias to refuse
the rename over — left to that lane. The moved tests are the row's;
pedantic clippy on `tcl-registry`, `tcl-compiler`, `tcl-spectcl`,
`tcl-lsp-core` and `xtask`, no `#[allow]` added, and `cargo fmt`;
`value-transfers` (`commands.rs` 4 → 0, into `CLEAN_FILES`, its ledger
row gone; the generated page rewritten) and `--check` (22 clean, 19
waived, 83 pinned across 34 files, 6607 rows);
`registry-axes` (the four `until slice 8` waivers gone; the generated
page rewritten) and `--check` (893 pinned across 147 files, 36 waived);
`fields.md` regenerated (`UPDATE_REFERENCE=1 cargo test -p
tcl-spec-studio --test reference_doc`); `pack-goldens` (25 packs, 0
rewritten); `retired-api-gate`, `callback-inventory --check`,
`owner-resolution`, `kcs-index-links`, `gen-editor-catalogs --check` and
`gen-zed-queries --check` pass; `dialect-drift` 8 sites, none new;
`cargo check --workspace` clean.

### Slice 6 — branch integration and optional rewrites

#### Goal and exit

In the plan's words: "The exact whole-variable `Raw` resolution;
selection facts for opaque forms through `tcl_cmd_core::switch`; O112, the
analyser's `switch_body_is_selected`, and `static_loops::exec_switch`
consuming them; applied reachability only with real lowering or explicit
arm blocks; arm-deletion edits only once their proof and source-edit
contracts exist, with no code reserved until then." The two applied-
reachability consumers the findings table names (#2056, #2057) land here.

*Exit*: "program (4) yields O112 and I231 on the dead arm for every form,
and O107 on its body for the flattened form."

Exit evidence: `program_four_yields_o112_and_i231_for_every_form`
(compiler witnesses: the exact, `-glob`, `-regexp`, `-nocase` and
fall-through forms of program (4)); `the_flattened_form_yields_o107`;
`switch_dispatch_branches_are_skipped` unchanged; `tcl diag` on program
(4) prints I231 on `baz`'s arm and `tcl opt --profile full` removes it,
with the optimised program printing `always` under 8.4 to 9.1 (`-nocase`
from 8.5); `tcl explore --show sccp --text --no-colour` over the `-glob`
form prints a `selection` line whose selected arm is `default`.

#### Upstream starting point

`git diff 3b5eba8a origin/rust -- rust/tcl-cmd-core/src/switch.rs
rust/tcl-compiler/src/optimiser/structure_elimination.rs
rust/tcl-compiler/src/analyser/handlers.rs rust/tcl-compiler/src/irules_checks.rs
rust/tcl-compiler/src/analyser/bounds_checks.rs`: `switch -nocase -exact`
folds the full Unicode range in the core (+44), which VT6.2 inherits;
`bounds_checks.rs` gained `writes_the_name` (#2054), beside VT6.7's
change to the loop-termination half of the file; the other three are
unchanged.

#### Work items

##### VT6.1 — the whole-variable `Raw` subject

- **Files**: `rust/tcl-compiler/src/sccp.rs` (`evaluate_branch`,
  `env_from_uses`), `rust/tcl-compiler/src/cfg_builder/cfg_lower.rs`
  (unchanged `switch_subject_operand`; its doc names the resolution).
- **Items**: an `ExprNode::Raw` operand resolves from the lattice only
  when the variable-name owner proves its text is exactly one variable
  reference (`simple_var_ref_name` over the profile's close rules);
  arbitrary `Raw` text stays unevaluable.
- **Preserves**: `switch_dispatch_branches_are_skipped` (O101 stays
  suppressed on the synthetic chain, `is_switch_dispatch` unchanged).
- **Changes**: the flattened `switch -- $acc` over a constant decides
  per arm: O107 and I231 fire. Mandate: § *`switch`*, step 1.
- **Tests**: `the_flattened_form_yields_o107` (negative: a `${…}` subject
  with a backslash stays `Raw` and undecided).
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: S. **After**: slice 8.

##### VT6.2 — `switch` declares its selection contract

- **Files**: `rust/tcl-registry/src/value_transfer/selection.rs` (new),
  the `switch` spec module, `value_transfers.rs`, `differential_fold.rs`.
- **Items**: `SwitchSemantics`: `structure` answers
  `PlanAnswer::CaseList { subject, arms, fallthrough, selection:
  SelectionContract { mode, nocase, final_default } }` from `CaseListSpec`;
  `transfer(FactDomain::Selection, …)` answers
  `TransferAnswer::Selection(SelectionFact { selected })` through
  `tcl_cmd_core::switch::{parse_options, select}`, one entry per subject
  member (a `ConstSet` subject joins), with ordered first match, the final
  default, fall-through to the next body, regexp mode through `AreEngine`
  and `RegexpPrecision` (only `Exact` and `NoMatch` select), and
  `-matchvar` / `-indexvar` as `Write` outcomes; a pattern that cannot be
  evaluated declines the whole fact.
- **Tests**: `switch_selection_runs_the_shared_core` (`value_transfers.rs`)
  and `switch_witnesses_match_every_release_on_path` (8.4 to 9.1;
  `-nocase` from 8.5): ordered patterns, the final default, a `-` arm
  whose pattern never matches supplying the next body, regexp captures, a
  malformed regexp declining.
- **Gates**: G2, G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: slice 5 (`RegexpPrecision`).

##### VT6.3 — the selection record

- **Files**: `rust/tcl-compiler/src/sccp.rs`, `rust/tcl-compiler/src/value_transfer.rs`,
  `rust/tcl-compiler/src/lattice_rebase.rs`, `rust/tcl-explorer/src/serialise.rs`,
  `view_tree.rs`.
- **Items**:
  ```rust
  /// The selected arm of one opaque `switch`, per subject member, against
  /// the arms' pattern spans.
  pub struct SelectionRecord { pub span: Span, pub arm_pattern_spans: Vec<Span>, pub fact: SelectionFact }
  pub struct SccpResult { /* … */ pub selections: Vec<SelectionRecord> }
  ```
  The driver records one per `Statement::Switch` whose subject is `Const`
  or `ConstSet`; `rebase_function_unit` shifts `span` and every
  `arm_pattern_spans` entry in the same change; the Explorer renders it.
- **Tests**: `rebase_shifted_unit_spans_match_fresh` covers it;
  `serialise::tests::sccp_reports_the_selection`.
- **Gates**: G7, G8, G9 (`tcl-compiler`, `tcl-explorer`).
- **Model**: opus. **Size**: M. **After**: VT6.2.

##### VT6.4 — O112, `switch_body_is_selected` and `exec_switch` read the owner

- **Files**: `rust/tcl-compiler/src/optimiser/structure_elimination.rs`
  (`resolve_subject`, `pattern_matches` go), `rust/tcl-compiler/src/analyser/handlers.rs`
  (`switch_body_is_selected`), `rust/tcl-compiler/src/static_loops.rs`
  (`exec_switch`).
- **Items**: O112 reads the selection record, and its
  first-unfoldable-clause and `catch`-descent limits go;
  `switch_body_is_selected` asks `SwitchSemantics` over a `LiteralInputs`
  of its static subject and patterns, regexp mode included, and its own
  clause matching goes; `exec_switch` asks the same owner over the
  simulator's environment, honouring the mode it ignores today.
- **Preserves**: every O112 test; `summarise_resolves_switch_dispatch_in_body`
  and `summarise_bails_on_unresolvable_switch_subject`.
- **Changes**: O112 fires for `-glob`, `-regexp`, `-nocase` and
  fall-through forms over a constant subject; the simulator stops treating
  a `-glob` switch as exact. Mandate: step 2 of § *`switch`*; the O112 row.
- **Tests**: `program_four_yields_o112_and_i231_for_every_form` (with
  VT6.5).
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: VT6.3.

##### VT6.5 — I231 on an opaque form

- **Files**: `rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs`
  (`emit_constant_branch_diagnostics`), `rust/tcl-compiler/src/compiler_checks.rs`.
- **Items**: an opaque form's unselected arms report I231 from the
  `Selected` branch fact (VT5.12's kind), never as applied reachability:
  no block is dropped and O107 does not fire for an opaque form.
- **Tests**: in `program_four_yields_o112_and_i231_for_every_form`.
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: S. **After**: VT6.3.

##### VT6.6 — the iRules flow checks read applied reachability (#2056)

- **Files**: `rust/tcl-compiler/src/irules_checks.rs`
  (`find_http_flow_warnings`, and each IRULE1005–1008, 1202, 4002, 4004,
  5002, 5004 walk that does not consult `executable_blocks`).
- **Items**: a respond or redirect in a block that is not executable does
  not commit the response; every walk skips non-executable blocks as
  lines 356 and 738 already do.
- **Changes**: `when HTTP_REQUEST {if {0} {HTTP::respond 200};
  HTTP::header insert X-Custom val}` and the `set flag 0` variant give no
  IRULE1201. Mandate: #2056; the IRULE row ("every check consumes applied
  reachability").
- **Tests**: `irule1201_ignores_a_respond_in_a_dead_arm` (positive and the
  live-arm negative).
- **Gates**: G7, G8, G9.
- **Model**: sonnet. **Size**: S. **After**: —.

##### VT6.7 — W240 and W241 from the loop header's branch fact (#2057)

- **Files**: `rust/tcl-compiler/src/analyser/bounds_checks.rs`
  (`loop_termination_diagnostics`), `rust/tcl-compiler/src/analyser/commands.rs`
  (its caller), `rust/tcl-compiler/src/analyser/diagnostics.rs`.
- **Items**:
  ```rust
  /// A loop whose termination the walk examined, resolved against the
  /// unit's branch fact at its condition span in the per-function pass.
  pub(crate) struct LoopTerminationCandidate {
      pub condition_span: Span,
      pub loop_span: Span,
      pub lexical: LexicalVerdict, // Dead, Infinite, Unprovable, Silent
  }
  ```
  The walk records candidates; the per-function pass resolves each: a
  header branch decided false at entry is W240, decided true with no
  executable exit is W241, and either suppresses W242; an undecided one
  keeps its lexical verdict.
- **Changes**: `set n 0; while {$n} {puts "never runs"}` gives W240 and
  no W242; `set go 1; while {$go} {puts x}` gives W241 and no W242.
  Mandate: #2057 ("W240–W242 consume the branch fact instead of the
  condition's text"); the W240–W242 row.
- **Tests**: `w240_and_w241_read_the_branch_fact` (positive both; negative:
  `while {$n}` with `n` a parameter keeps W242).
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: VT6.1.

##### VT6.8 — the slice's witnesses

- **Files**: `rust/tcl-compiler/tests/value_transfer_witnesses.rs`,
  `rust/tcl-cli/tests/value_transfers_cli.rs`.
- **Tests**: program (4) in every form through `tcl diag` and `tcl opt`,
  the optimised output against `tclsh`; `explore_sccp_prints_the_selection`.
- **Gates**: G7, G8, G9.
- **Model**: sonnet. **Size**: S. **After**: VT6.1 to VT6.7.

##### VT6.9 — docs and the landing

- **Files**: `optimisation-passes.md` (O107, O112), `sccp-core-analyses.md`,
  `pass-fact-ownership-matrix.md` (the selection record),
  `value-transfers-migration.md`, `docs/kcs/codes/` notes for I231 and
  W240–W242, the lane doc; the diagnostics rows drafted (B-DP4).
- **Gates**: G5, G6, and the slice's green.
- **Model**: sonnet. **Size**: S. **After**: VT6.8.

#### Checkpoints and landing

| Checkpoint | Holds | Green means |
|---|---|---|
| `wip(value-transfers): slice 6 — the subject and the selection owner` | VT6.1, VT6.2, VT6.3 | the flattened O107 witness; the selection tests under every release |
| `wip(value-transfers): slice 6 — the consumers read one selection` | VT6.4, VT6.5 | program (4) in every form |
| `wip(value-transfers): slice 6 — branch integration and optional rewrites` (landing) | VT6.6 to VT6.9 | every exit test; G1 to G9 |

```text
wip(value-transfers): slice 6 — branch integration and optional rewrites

A whole-variable `switch` subject resolves from the lattice when the
name owner proves it is one variable reference, so the flattened form
decides per arm with applied reachability. Opaque forms get one
selection fact from the shared `switch` core — ordered first match,
final default, fall-through, regexp captures through the precise regexp
owner — recorded against the arms' pattern spans; O112, the analyser's
selected-body test and the loop simulator read it instead of three
private matchers, and I231 reports the unselected arms without
pretending blocks exist. The iRules flow checks read applied
reachability, and W240 and W241 read the loop header's branch fact. No
code is reserved for arm deletion.

Behaviour changes: program (4) gives O112 and I231 in every form and O107
in the flattened one; IRULE1201 ignores a respond in a dead arm; a
constant `while` condition gives W240 or W241 instead of W242.

Closes #2056. Closes #2057.
```

#### Review checklist

- R1: no consumer matches `"switch"` or a mode spelling; the selection
  goes through `tcl_cmd_core::switch`; no new file is ratcheted.
- R2 to R5; new file: `selection.rs`.
- R6: `switch_dispatch_branches_are_skipped` and every O112 test
  byte-identical.
- Suites: `cargo test -p tcl-registry -p tcl-compiler -p tcl-explorer -p
  tcl-cli`.
- R7: a selection is never applied reachability without a real edge; an
  approximate regexp match never selects; a `ConstSet` subject keeps
  every member's arm and its error possibilities.

#### Behavioural deltas

| Delta | Mandate |
|---|---|
| program (4) gives O112 and I231 in every form, O107 in the flattened one | the exit |
| the loop simulator honours `switch`'s mode | step 2 of § *`switch`* |
| IRULE1201 ignores a respond in a dead arm | #2056 |
| a constant `while` condition gives W240 or W241 instead of W242 | #2057 |

### Slice 9 — nested writes in expressions

#### Goal and exit

In the plan's words: "The ordered evaluation state at `LocalWrites`: a
nested invocation whose ordered stores name only places the state can own
is applied to the state in order, so the next `variable` read sees it,
with its evidence merged and an error completion ending the evaluation
with the writes so far. A nested outcome naming a place outside the
admitted set is the `StatefulNested` decline."

*Exit*: "the seven `expr` witnesses — `expr {$x + [incr x] + $x}`, `expr
{0 && [incr x]}`, `expr {$x + [set x 10] + $x}`, the two `[incr x]`
operands, the ternary, the quoted word, and the error path — through `tcl
opt` and the memoised path, and `command_substitution_is_none` in
`tcl_expr_eval.rs` flips."

Exit evidence: `the_seven_ordered_state_witnesses` (compiler witnesses,
CLI, and `value_transfer_parity.rs`), each optimised program printing
what `tclsh` prints under 8.4 to 9.1; the restated
`command_substitution_evaluates_through_the_nested_service` (Q2); `tcl
explore --source 'proc p {} {set x 1; set r [expr {$x + [incr x] + $x}];
return $x}' --show sccp --text --no-colour` prints `r#1 = const(5)` and
`x#2 = const(2)`.

#### Upstream starting point

`git diff 3b5eba8a origin/rust -- rust/tcl-compiler/src/ir_helpers.rs
rust/tcl-compiler/src/cfg_builder/mod.rs rust/tcl-compiler/src/cfg_builder/global_write_info.rs`:
#2215's in-frame expression descent (`in_frame_expression_commands`)
already records the reads and writes of a braced `expr`'s nested
commands, with a recovered head resolved through the module's bindings
(`interp alias {} e {} expr` is an expression word); and
`global_write_info.rs` counts a write from an in-frame expression word
(`set y [expr {[incr ::hits]}]`). VT9.3 verifies the descent instead of
building it; VT9.2 supplies the values the descent's definitions lacked.

#### Work items

##### VT9.1 — `LocalWrites`

- **Files**: `rust/tcl-compiler/src/value_transfer.rs`
  (`LatticeInputs::nested`, `LatticeInputs::variable`),
  `rust/tcl-compiler/src/tcl_expr_eval.rs` (`ExprServices`).
- **Items**: under `NestedPolicy::LocalWrites`, `nested` admits an outcome
  whose every ordered store names a place the state can own — local, not
  escaping, not traced, not dynamic — and whose completion is exact, and
  applies its stores to `EvaluationState::writes` in order, merging its
  evidence; `variable` consults `writes` before the program point's
  facts; any other outcome is `StatefulNested`. An error completion ends
  the evaluation with that completion and the writes so far.
- **Preserves**: every answer under `EffectFreeOnly` (branch conditions
  keep it).
- **Tests**: `local_writes_apply_in_order` (`value_transfers.rs`-style
  driver test: `$x + [incr x] + $x` reads 1, then 2).
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: slices 3 and 5.

##### VT9.2 — a word's ordered stores become the statement's definitions

- **Files**: `rust/tcl-compiler/src/value_transfer.rs`,
  `rust/tcl-compiler/src/sccp.rs`, `rust/tcl-compiler/src/cfg_builder/mod.rs`
  (the synthetic embedded-substitution call carries its host statement's
  index).
- **Items**: the host statement's words evaluate once under one
  `EvaluationState` starting from the synthetic call's incoming versions
  (its named reads); the synthetic call's definitions take the state's
  final write per place, and the host's definition takes the result. A
  quoted word substitutes under the same state before `expr` parses it.
- **Changes**: `set r [expr {$x + [incr x] + $x}]` gives `r` 5 and `x` 2;
  the forwarding O102 and O100 do from those definitions is the nested
  store's. Mandate: the exit; the Expressions row ("the ordered evaluation
  state's nested writes").
- **Tests**: in `the_seven_ordered_state_witnesses`.
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: L. **After**: VT9.1.

##### VT9.3 — a read inside a braced `expr` is a use

- **Files**: `rust/tcl-compiler/src/ssa.rs`, `rust/tcl-compiler/src/ir_helpers.rs`
  (as merged from `rust`'s #2215, `in_frame_expression_commands`).
- **Items**: the read of `n` in `set r [expr {$n + [incr n]}]` is an SSA
  use of the version it reads, so O109, O126 and W211 keep `set n 1`.
  VT2.M brings `rust`'s descent; this item verifies it on the page's `p`
  and `q` and fills any gap the ordered state exposes.
- **Changes**: none beyond #2215's. Mandate: the O109 row ("slice 9
  makes a read inside a braced `expr` a use of the version it reads").
- **Tests**: `a_braced_expr_read_keeps_its_store` (`p` prints 3, `q`
  prints 5, optimised, under 8.5 to 9.1; `q` under 8.4 prints 5 where the
  original does).
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: S. **After**: VT2.M.

##### VT9.4 — `command_substitution_is_none` flips

- **Files**: `rust/tcl-compiler/src/tcl_expr_eval.rs` (tests).
- **Items**: the test becomes
  `command_substitution_evaluates_through_the_nested_service`: with the
  nested service at `LocalWrites` and `x` 1, `[incr x] + 1` is 3 and `x`
  becomes 2; its negative keeps the old program, `[clock seconds] + 1`,
  which reads the wall clock and is `None` under every policy (Q2).
- **Model**: sonnet. **Size**: S. **After**: VT9.1.

##### VT9.5 — the seven witnesses

- **Files**: `rust/tcl-compiler/tests/value_transfer_witnesses.rs`,
  `rust/tcl-cli/tests/value_transfers_cli.rs`,
  `rust/tcl-lsp-db/src/value_transfer_parity.rs`,
  `rust/tcl-registry/tests/differential_fold.rs`.
- **Tests**: `the_seven_ordered_state_witnesses` — the interface page's
  seven programs (5 and 2; 0 and 1; 21 and 10; 5 and 3; 2 and 2; 3 and 2;
  and `catch {expr {[incr x] + [error mid]}}`, which is 1 with `x` 2),
  each through the direct unit, the memoised unit and `tcl opt`, whose
  output prints what `tclsh` prints under 8.4 to 9.1. On the error path
  this slice proves only that nothing forwards the stale `x` (the default
  build's `catch` body is opaque); slice 10 makes `x` exact on that path.
  `a_nested_write_outside_the_state_declines` (`expr {$x + [incr ::g]}`
  is `StatefulNested`). #2141's program pins through the CLI.
- **Gates**: G7, G8, G9.
- **Model**: sonnet. **Size**: M. **After**: VT9.1 to VT9.4.

##### VT9.6 — docs and the landing

- **Files**: `value-transfers.md` (§ `expr`'s state paragraph in the
  present tense), `optimisation-passes.md` (O100, O102, O109),
  `pass-fact-ownership-matrix.md`, `value-transfers-migration.md`, the
  lane doc.
- **Gates**: G6, and the slice's green.
- **Model**: sonnet. **Size**: S. **After**: VT9.5.

#### Checkpoints and landing

| Checkpoint | Holds | Green means |
|---|---|---|
| `wip(value-transfers): slice 9 — the ordered state admits local writes` | VT9.1, VT9.3, VT9.4 | the driver tests; every `EffectFreeOnly` answer byte-identical |
| `wip(value-transfers): slice 9 — nested writes in expressions` (landing) | VT9.2, VT9.5, VT9.6 | the seven witnesses on three paths; G1 to G9 |

```text
wip(value-transfers): slice 9 — nested writes in expressions

An expression's nested invocations run in the engine's left-to-right
order under one evaluation state: a nested write to a place the state
can own is applied in order, so the next read sees it, and its evidence
joins the answer; a write the state cannot own declines as stateful. A
statement's substitutions evaluate once, from the versions its synthetic
embedded call reads, and the call's definitions take the final writes,
so `$x + [incr x] + $x` is 5 and leaves `x` at 2 for every consumer. A
read inside a braced `expr` is a use of the version it reads.

Behaviour changes: nested `incr` and `set` inside `expr` fold with their
writes; O100 and O102 forward the nested store's value; an error inside
an expression ends the evaluation with the writes so far.

Pins #2141 (closed on rust by #2215).
```

#### Review checklist

- R1: no consumer matches `"incr"` or `"set"` inside an expression; the
  admission is by place ownership.
- R2 to R5.
- R6: `command_substitution_is_none` is the one restated test (Q2).
- Suites: `cargo test -p tcl-compiler -p tcl-lsp-db -p tcl-cli -p
  tcl-registry`.
- R7: a nested write to a traced, escaping, dynamic or qualified place
  declines; `0 && [incr x]` never applies the write; an error completion
  publishes only the writes so far.

#### Behavioural deltas

| Delta | Mandate |
|---|---|
| nested `incr` / `set` inside `expr` fold, with their writes | the exit; the Expressions row |
| O100 / O102 forward the nested store's value past the expression | #2141's contract point |
| `command_substitution_is_none` becomes `command_substitution_evaluates_through_the_nested_service` | the exit ("flips"); Q2 |

### Slice 10 — completion paths

#### Goal and exit

In the plan's words: "Storage outcomes indexed by completion path: the
prefix rule (`Error { written, … }`), the completion protocols of `catch`
and `try`, the `Absorb` rule for loop bodies, the options dictionary's
exact and `Unavailable` keys, and the per-path publication the solver
already needs for the existence rung." `KNOWN_GAPS` adds `catch`.

*Exit*: "the prefix rule holds in the default and the faithful-exceptions
build; the nine witnesses and the `catch` code table pass; O109 refuses the
store ahead of `catch {lassign {new second} a b} msg`."

Exit evidence: `the_prefix_rule_holds_in_both_builds` (the nine programs,
each under the default and the faithful-exceptions builds),
`the_catch_code_table`, `o109_refuses_the_store_ahead_of_a_partial_lassign`
(compiler witnesses); `try_finally_runs_on_every_path` (#2142);
`try_finally_creates_finally_block` and `try_with_handler` unchanged;
`tcl opt --profile full` on #2142's global witness prints `1` under 8.6
to 9.1; `tcl explore --source 'proc p {} {set c [catch {error boom} m];
return $c}' --show sccp --text --no-colour` prints `c#1 = const(1)`,
`m#1 = const('boom')` and a `route catch:` line answering `evaluated`.

#### Upstream starting point

`git diff 3b5eba8a origin/rust -- rust/tcl-compiler/src/cfg_builder/cfg_lower.rs
rust/tcl-compiler/src/cfg_builder/mod.rs rust/tcl-compiler/src/ir.rs
rust/tcl-compiler/src/codegen/emitter/try_blocks.rs rust/tcl-vm/src/cmd_try.rs`:
#2207 flattens a straight-line `catch` body inside a procedure into real
blocks (`lower_catch`, body to end with exception edges from the body's
throw points, `result_var` and `options_var` defined on both paths), the
analogue of `lower_try`; `Module::top_level_kind` (`TopLevelKind`) says
whether `::top` is a procedure body. So the default build is no longer
"one opaque call" for every `catch`: the opaque form remains at a
script's top level and for bodies that are not straight-line. VT10.2 and
VT10.5 give `lower_catch`'s blocks the same plan, completion protocol and
prefix rule as the faithful build, and the nine witnesses run in all
three shapes (opaque, flattened `catch`, faithful build).

#### Work items

##### VT10.1 — an error is a completion, not a decline

- **Files**: `rust/tcl-registry/src/value_transfer/const_ops.rs`, every
  registry-owned route, `rust/tcl-compiler/src/value_transfer.rs`.
- **Items**: a core's `CmdError` after `k` stores is
  `CompletionOutcome::Error { written: k, message, error_code }`; `message`
  and `error_code` are `Exact` only when the route proves them under every
  release of the profile (the `scan` / `regexp` array-target messages
  differ between 8.5 and 8.6), else `Unavailable`; a `ConstOps` fault
  (inadmissible axis, budget) stays a decline.
- **Preserves**: every normal-completion answer.
- **Changes**: statements whose route proves an error gain an error
  completion instead of `Overdefined`; the value on the normal path is
  unchanged (there is none). Mandate: "the prefix rule (`Error {
  written, … }`)"; the slice-2 decision that ends here.
- **Tests**: `a_route_error_is_a_completion` (`value_transfers.rs`:
  `lassign {new second} a b` with `b` an array is `Error { written: 1 }`).
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: slice 9.

##### VT10.2 — per-path publication

- **Files**: `rust/tcl-compiler/src/sccp.rs`, `rust/tcl-compiler/src/value_transfer.rs`,
  `rust/tcl-compiler/src/cfg_builder/mod.rs` (`emit_opaque_catch`,
  `lower_try_dispatch`), `rust/tcl-compiler/src/cfg_builder/cfg_lower.rs`
  (`push_try_handler_exception_edges`, `lower_catch`),
  `rust/tcl-explorer/src/serialise.rs`, `view_tree.rs` (the per-path
  view: each statement's stores per completion path).
- **Items**: the solver publishes a statement's values and existence per
  completion path: the normal edge carries every ordered store; an
  exception edge (the faithful build) carries the prefix `written`; the
  `Existence` transfer lists the paths (the page's rule for a failing
  step whose kind is not proven `Scalar`). A `catch` flattened by
  `lower_catch` publishes per path like the faithful build; an opaque
  `catch` body (`emit_opaque_catch`) stays one call whose defs are
  `MayWrite` and whose result variable is a `Write` of an unavailable
  value.
- **Tests**: `the_prefix_rule_holds_in_both_builds` (the opaque and the
  flattened `catch` of the default build, and the faithful build).
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: L. **After**: VT10.1.

##### VT10.3 — `catch`

- **Files**: `rust/tcl-registry/src/value_transfer/completion.rs` (new),
  the `catch` spec module, `rust/xtask/src/value_transfers.rs`.
- **Items**: `CatchSemantics`: `PlanAnswer::Body { completion:
  CompletionProtocol::CatchAll { result_var, options_var }, … }`; a closed
  body evaluates concretely: the code is the result, the message or
  result goes to `result_var`, the options dictionary (from 8.5) to
  `options_var` with `-code` and `-level` exact, `-errorcode` exact when
  proven (`NONE` for a plain `error`), and `-errorinfo`, `-errorline`,
  `-errorstack` `Unavailable` with type `dict`; `::errorCode` and
  `::errorInfo` are effects.
- **Tests**: `the_catch_code_table` (8.4 to 9.1; options from 8.5):
  `catch {return 5}` is 2 with result 5; `catch {break}` 3; `catch
  {continue}` 4; `catch {error boom} m` 1 with `m` `boom`; `catch {set v
  1} r` 0 with `r` 1; `return -code 5 custom` in a procedure 5; `catch
  {expr {1/0}}` 1 with `divide by zero`; `catch {incr absent}` 1 under 8.4
  with `absent` unbound, 0 from 8.5 with `absent` 1.
- **Also**: #2231 (D44): the body's reads reach the host — the statement
  form records them beside its definitions, and the nested `[catch {…}]`
  is descended — so the issue's three programs (§ *Slice 2* › *Record*,
  found and left) keep their stores and branches; they join VT10.8's
  witnesses, and the landing says "Closes #2231".
- **Gates**: G1 (`catch` row goes), G2, G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: VT10.2.

##### VT10.4 — `try` and `finally` on every path (#2142)

- **Files**: `rust/tcl-compiler/src/cfg_builder/cfg_lower.rs`
  (`lower_try`, `push_try_handler_exception_edges`),
  `rust/tcl-registry/src/value_transfer/completion.rs`, the `try` spec
  module.
- **Items**: every unhandled throw source of a `try` body reaches its
  `finally` block, or `try_end` when there is none — with no handler, and
  where a handler need not catch what was thrown; `TrySemantics` answers
  `CompletionProtocol::Handlers { handlers, finally }`, each
  `HandlerPlan::matches` from the clause-grammar descriptor's
  `HandlerMatch` (CC2.2).
- **Changes**: `proc p {} {set f 0; try {error boom} finally {set f 1};
  return $f}` gives neither W220 nor W210; `set g 0; proc p {} {global g;
  try {error boom} finally {set g 1}}; catch {p}; puts $g` keeps its
  `finally` body under O107. Mandate: #2142; the Completion paths row
  ("`try` handlers and `finally` on every path").
- **Tests**: `try_finally_runs_on_every_path` (both programs; the second
  prints `1` optimised under 8.6 to 9.1); the issue's two that must keep
  firing — `proc q {c} {try {if {$c} {error boom}} on error {} {set g
  1}; return $g}` keeps W210, and `try {error boom} on error {} {set f
  2} finally {set f 1}` keeps its W220.
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: VT10.2; CC2.2 (B-CC6).

##### VT10.5 — the prefix rule in the faithful-exceptions build

- **Files**: `rust/tcl-compiler/src/cfg_builder/mod.rs`
  (`with_faithful_exceptions`), `rust/tcl-compiler/src/cfg_builder/cfg_lower.rs`.
- **Items**: a handler's entry state is the join of its throw sources'
  prefix states; the default build (`emit_opaque_catch`,
  `lower_try_dispatch`) and the faithful build share one plan and one
  protocol.
- **Tests**: the faithful half of `the_prefix_rule_holds_in_both_builds`.
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: VT10.2.

##### VT10.6 — `Absorb` for loop bodies

- **Files**: `rust/tcl-registry/src/value_transfer/iteration.rs`,
  `rust/tcl-compiler/src/value_transfer.rs`.
- **Items**: a loop body's `break` and `continue` are absorbed
  (`CompletionProtocol::Absorb(&[Break, Continue])`); every other code
  passes to the loop.
- **Tests**: `a_loop_absorbs_break_and_continue`.
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: S. **After**: VT10.2.

##### VT10.7 — O109 and the prefix rule

- **Files**: `rust/tcl-compiler/src/optimiser/elimination.rs`.
- **Items**: a store ahead of a partial write is dead only when the
  prefix rule proves every path overwrites it.
- **Changes**: O109 keeps `set a old` ahead of `catch {lassign {new
  second} a b} msg` when `b` may be an array. Mandate: the exit.
- **Tests**: `o109_refuses_the_store_ahead_of_a_partial_lassign`.
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: S. **After**: VT10.2.

##### VT10.8 — the slice's witnesses

- **Files**: `rust/tcl-compiler/tests/value_transfer_witnesses.rs`,
  `rust/tcl-cli/tests/value_transfers_cli.rs`,
  `rust/tcl-registry/tests/differential_fold.rs`.
- **Tests**: the nine programs of § *`catch`, `try`, and completion* under
  both builds and every release (`lassign` from 8.5, `try` from 8.6);
  the `unset p nosuch q` line of the absent-cell table (p unbound, q 2);
  slice 9's error-path witness with `x` exact on the error path.
- **Gates**: G7, G8, G9.
- **Model**: sonnet. **Size**: M. **After**: VT10.1 to VT10.7.

##### VT10.9 — docs and the landing

- **Files**: `value-transfers.md`, `sccp-core-analyses.md`,
  `downstream-pass-contracts.md`, `optimisation-passes.md`,
  `precision-limitations.md`, `pass-fact-ownership-matrix.md`,
  `value-transfers-migration.md`, the lane doc; the diagnostics rows
  drafted (B-DP4).
- **Gates**: G6, and the slice's green.
- **Model**: sonnet. **Size**: S. **After**: VT10.8.

#### Checkpoints and landing

| Checkpoint | Holds | Green means |
|---|---|---|
| `wip(value-transfers): slice 10 — errors are completions` | VT10.1, VT10.2 | every normal-path answer byte-identical; the prefix witnesses in the default build |
| `wip(value-transfers): slice 10 — catch, try and the faithful build` | VT10.3 to VT10.6 | the code table; #2142's programs; the faithful build's prefix witnesses |
| `wip(value-transfers): slice 10 — completion paths` (landing) | VT10.7 to VT10.9 | every exit test; G1 to G9 |

```text
wip(value-transfers): slice 10 — completion paths

Every invocation completes one way, and its storage outcomes are indexed
by the path: an error after k stores publishes those k on the error edge
and nothing else, in the default and the faithful-exceptions build alike.
`catch` evaluates a closed body to its code, message and options
dictionary — exact code and level, exact error code when proven,
unavailable error info — and keeps the default build's opaque body
otherwise; `try` runs its handlers by the clause grammar's match, and
every unhandled throw reaches `finally`. Loop bodies absorb `break` and
`continue`. O109 removes a store ahead of a partial write only when the
prefix rule proves it dead.

Behaviour changes: a proven error is a completion fact; `catch` and
`try` bodies decide their results where closed; a `finally` body is live
on every path; O109 keeps the store ahead of a partial `lassign`.

Closes #2142.
```

#### Review checklist

- R1: no consumer matches `"catch"`, `"try"` or a handler keyword; the
  handlers come from the clause-grammar descriptor.
- R2 to R5; new file: `completion.rs`.
- R6: `try_finally_creates_finally_block` and `try_with_handler`
  byte-identical.
- Suites: `cargo test -p tcl-registry -p tcl-compiler -p tcl-cli -p
  tcl-lsp-db`.
- R7: the prefix rule on every error edge; an `Any` completion domain
  makes every target `MayWrite` on the error edge; a message is exact
  only when every release of the profile gives it.

#### Behavioural deltas

| Delta | Mandate |
|---|---|
| a proven error is a completion, published per path | "the prefix rule"; the Completion paths row |
| `catch` decides its code and variables for a closed body | the `catch` code table |
| a `finally` body is live on every path | #2142 |
| O109 keeps the store ahead of a partial `lassign` | the exit |

### Slice 11 — predicate refinement

#### Goal and exit

In the plan's words: "`EdgeRefinement` as a fact on one CFG edge for one
SSA version, with the block-qualified lookup `(BlockId, ValueKey)`
consulted first by `env_from_uses`, `evaluate_branch`, and
`evaluate_def_with_folds`, and by every other domain for the domains the
refinement names; the per-shape table, including the numeric `==` rows
that refine `Range` and `Type` and never `ExactValue`; no refinement of an
externally mutable place."

*Exit*: "the nested `if {$x eq "a"}` / `if {$x eq "b"}` program decides
through `tcl diag` and `tcl opt`, `collect_existence_guards` is deleted,
and the twelve witnesses, the merge that drops a refinement, and the
traced variable that is never refined pass."

Exit evidence: `the_nested_equality_decides`, `the_twelve_refinement_witnesses`,
`a_merge_drops_the_refinement`, `a_traced_variable_is_never_refined`
(compiler witnesses; the twelve under 8.4 to 9.1, with `in` and `-nocase`
from 8.5); `tcl diag` on `proc p {x} {if {$x eq "a"} {if {$x eq "b"}
{puts never}}}` prints I230 on the inner condition and `tcl opt --profile
full` removes the inner `if`; `collect_existence_guards` and
`block_dominated_by` absent from `analyser/diagnostics/helpers.rs`; `tcl
explore --show sccp --text` over the nested program prints a `refinement
x = 'a'` line on the outer true edge and the inner `if`'s true block
outside `executable blocks`.

#### Upstream starting point

`git diff 3b5eba8a origin/rust -- rust/tcl-compiler/src/sccp.rs
rust/tcl-compiler/src/analyser/diagnostics/helpers.rs
rust/tcl-compiler/src/analyser/diagnostics.rs`: only the trust stances in
`sccp.rs` (VT2.M); `helpers.rs` and the per-function pass are unchanged.

#### Work items

##### VT11.1 — `EdgeRefinement` in every domain

- **Files**: `rust/tcl-compiler/src/sccp.rs`, `rust/tcl-compiler/src/value_transfer.rs`,
  `rust/tcl-explorer/src/serialise.rs`, `view_tree.rs` (a `refinement`
  line per edge fact).
- **Items**: VT8.3's `EdgeRefinement` without its domain restriction:
  ```rust
  /// A fact that holds on one CFG edge and in every block that edge's
  /// target dominates, for one SSA version.
  pub struct EdgeRefinement {
      pub edge: (BlockId, BlockId),
      pub key: ValueKey,
      pub domain: FactDomain,
      pub fact: DomainFact,
      pub evidence: DependencyEvidence,
  }
  pub struct SccpResult { /* … */ pub refinements: Vec<EdgeRefinement> }
  ```
  A second lookup keyed `(BlockId, ValueKey)`, consulted first by
  `env_from_uses`, `evaluate_branch` and `evaluate_def_with_folds` through
  the block they run in, and by each domain's lookup for its own domain; a
  refinement never creates an SSA version and is not consulted at a merge
  its edge's target does not dominate; an externally mutable place
  (`is_externally_mutable`) is never refined. The record carries no span.
- **Preserves**: every existing branch decision and value.
- **Tests**: `a_merge_drops_the_refinement` (two arms refining `x` to `a`
  and `b` meet as the version's own value);
  `a_traced_variable_is_never_refined`.
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: L. **After**: slices 6 and 8.

##### VT11.2 — the per-shape table

- **Files**: `rust/tcl-compiler/src/tcl_expr_eval.rs` (the expression
  route's `Selection` transfer over the condition tree),
  `rust/tcl-registry/src/value_transfer/selection.rs` (the case-list arms).
- **Items**: the page's table, row for row: `eq` refines the true edge's
  `ExactValue`; `ne` the false edge's; `==` with a literal non-numeric
  under every release refines `ExactValue`, with a numeric literal `Range`
  (the point) and `Type` and never `ExactValue`; `!=` as `==` on its false
  edge; `in` / `ni` (from 8.5) the finite set; `[string is CLASS ?-strict?
  $x]` the `Type`; `[info exists x]` / `[array exists x]` the
  `Existence`; `!`, `&&`, `||` compose; a `switch -exact` arm refines
  `ExactValue` at its entry (a body reached through `-` arms gets the
  patterns' finite set), `-nocase` (from 8.5) a case-insensitive `Type`,
  `-glob` the literal prefix as `Segments`; `-regexp`, `$x` alone and
  `$x eq $y` refine nothing.
- **Tests**: `the_twelve_refinement_witnesses`.
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: VT11.1.

##### VT11.3 — the consumers, and the guard helpers go

- **Files**: `rust/tcl-compiler/src/analyser/diagnostics/helpers.rs`
  (`collect_existence_guards`, `block_dominated_by`),
  `rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs`,
  `rust/tcl-compiler/src/analyser/diagnostics.rs`, `rust/tcl-compiler/src/type_infer.rs`,
  `rust/tcl-compiler/src/intervals.rs`.
- **Items**: the value lattice, `type_infer` (`string is`), `intervals`
  (the numeric point) and the existence rung read the refined facts; the
  three callers of `collect_existence_guards` (`phi_can_undef`'s guard in
  `helpers.rs`, `emit_read_before_set_diagnostics`,
  `emit_return_phi_undef_w210`) read the block-qualified existence fact,
  and both helpers are deleted.
- **Preserves**: `info_exists_guard_narrows_read_in_then_arm`,
  `info_exists_negated_guard_narrows_false_arm`,
  `info_exists_read_outside_guard_still_flags_w210`, byte-identical.
- **Changes**: the nested `if` decides (I230, O101, O107, O112); `string
  is integer $x` types `x` in its arm. Mandate: the exit; the Branches
  row ("predicate refinement per shape").
- **Tests**: `the_nested_equality_decides`.
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: VT11.2.

##### VT11.4 — the slice's witnesses

- **Files**: `rust/tcl-compiler/tests/value_transfer_witnesses.rs`,
  `rust/tcl-cli/tests/value_transfers_cli.rs`,
  `rust/tcl-registry/tests/differential_fold.rs`.
- **Tests**: the page's twelve programs (`1.0 == 1` keeps `string
  length` 3; `" 1" == 1` keeps 2; `01 == 1` keeps `01`; `1.0 eq "1"` is
  no; `a eq "a"` is `a`; `string is integer " 12 "` is 1; `{}` is 1 and 0
  with `-strict`; `0x10` is 1; `yes` stays `yes`; `08 == 8` is no to 8.6
  and yes from 9.0; `switch -nocase` keeps `A` from 8.5; `$x in {a b c}`
  keeps `b` from 8.5), each under every release; the nested-`if` I230 and
  O-codes through the CLI.
- **Gates**: G7, G8, G9.
- **Model**: sonnet. **Size**: M. **After**: VT11.1 to VT11.3.

##### VT11.5 — docs and the landing

- **Files**: `sccp-core-analyses.md`, `pass-fact-ownership-matrix.md`
  (refinements), `optimisation-passes.md`, `value-transfers-migration.md`,
  `docs/kcs/codes/` note for I230, the lane doc; diagnostics rows drafted
  (B-DP4).
- **Gates**: G5, G6, and the slice's green.
- **Model**: sonnet. **Size**: S. **After**: VT11.4.

#### Checkpoints and landing

| Checkpoint | Holds | Green means |
|---|---|---|
| `wip(value-transfers): slice 11 — edge refinements` | VT11.1, VT11.2 | the merge and trace tests; every decision byte-identical |
| `wip(value-transfers): slice 11 — predicate refinement` (landing) | VT11.3 to VT11.5 | every exit test; G1 to G9 |

```text
wip(value-transfers): slice 11 — predicate refinement

A branch condition's `Selection` transfer refines its edges: a fact on
one CFG edge for one SSA version, read first through a block-qualified
lookup by the lattice, the evaluator and every domain it names, never
at a merge its edge does not dominate, and never for an externally
mutable place. Equality refines the value, numeric equality only the
range and type, `in` a finite set, `string is` a type, `info exists`
existence, and the `switch` arms their patterns. The nested equality
program decides for the diagnostics and the optimiser alike, and the
existence-guard helpers are gone.

Behaviour changes: nested conditions over a refined variable decide
(I230, O101, O107, O112); `string is` types its arm; the numeric `==`
never rewrites the string.
```

#### Review checklist

- R1: no consumer matches an operator or a `switch` mode spelling
  outside the expression route's condition tree.
- R2 to R5.
- R6: the three `info_exists_*` precedent tests byte-identical.
- Suites: `cargo test -p tcl-registry -p tcl-compiler -p tcl-cli`.
- R7: a numeric `==` never refines `ExactValue`; a refinement is never
  read past a non-dominated merge; an externally mutable or traced place
  is never refined; `08 == 8` refines only the range, so its
  release-dependence never reaches the string.

#### Behavioural deltas

| Delta | Mandate |
|---|---|
| a nested condition over a refined variable decides | the exit |
| `string is CLASS $x` types `x` in its arm | the per-shape table |
| the existence guard is the refinement lookup (byte-identical effect) | the exit |

### Slice 12 — bounded-loop enumeration

#### Goal and exit

In the plan's words: "`LoopEnumeration`: ordered execution of an
iteration plan over exact state at a loop's pre-header, with the
iteration cap as a `Budget` decline, exact values and existence instead
of `StaticValue`, and the exit state published on the exit edge only.
`exec_statement` applies each statement's registry-owned `evaluate` and
`exec_switch` consumes the `Selection` fact."

*Exit*: "`static_loops.rs` performs no arithmetic of its own — its `Incr`
arm, `parse_literal_value`, and `resolve_switch_subject` are gone — the
eleven witnesses fold under every release found on `PATH`, and
`bounds_checks.rs` seeds W240–W242 from the plan's bound and step rather
than from `set v INT` text."

Exit evidence: `the_eleven_loop_witnesses` (compiler and CLI, 8.4 to
9.1); the `summarise_*` tests and
`sccp_folds_post_loop_branch_via_static_summary` unchanged;
`the_correlated_pairs_decide_by_enumeration` (the mirror programs now give
20 and 25); `w240_seeds_from_the_iteration_plan` (`set i $start` no
longer disables the check); G1 with `static_loops.rs` clean and holding
no `parse_literal_value`, `resolve_switch_subject` or `Incr` arm, and
`irules_event_checks.rs` at 6; the ledger's `static_loops` row gone;
`tcl explore --source 'for {set i 0} {$i < 5} {incr i} {}; if {$i == 5}
{puts five}' --show sccp --text --no-colour` prints an `enumerated loop`
line (5 iterations, exhaustion) with `i` `const(5)` on the exit edge.

#### Upstream starting point

`git diff 3b5eba8a origin/rust -- rust/tcl-compiler/src/static_loops.rs
rust/tcl-compiler/src/intervals.rs rust/tcl-compiler/src/analyser/bounds_checks.rs
rust/tcl-compiler/src/analyser/irules_event_checks.rs`: `static_loops.rs`
drops one unused import; `bounds_checks.rs` gained W231's
`writes_the_name` abstention (#2054), which VT12.5 keeps beside the
loop-bound seeding; `intervals.rs` and `irules_event_checks.rs` are
unchanged.

#### Work items

##### VT12.1 — `LoopEnumeration`

- **Files**: `rust/tcl-compiler/src/sccp.rs`, `rust/tcl-compiler/src/static_loops.rs`,
  `rust/tcl-registry/src/value_transfer/decline.rs`,
  `rust/tcl-explorer/src/serialise.rs`, `view_tree.rs` (an `enumerated
  loop` line with its iterations, exit rule and exit state).
- **Items**: the page's `LoopEnumeration { plan, entry, iterations, exit,
  state, evidence }`, run at a loop's pre-header when the plan and the
  state admit it; `BudgetLimit::Iterations` (new) is the cap
  (`DEFAULT_MAX_STATIC_LOOP_ITERS`), and a capped loop publishes nothing;
  the exit state is published on the exit edge only, while the header
  phis widen as today; `break`, `continue` and an error path follow the
  plan's completion protocol (slice 10); a zero-iteration `foreach` leaves
  its binders `Unbound`; a nested loop is one statement of the outer body
  under the same budget.
- **Tests**: `the_iteration_cap_publishes_nothing`.
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: L. **After**: slices 6 and 10.

##### VT12.2 — the counted and conditional iteration plans

- **Files**: `rust/tcl-registry/src/value_transfer/iteration.rs`, the
  `for` and `while` spec modules, `value_transfers.rs`.
- **Items**: `FOR` and `WHILE` (`IterationSemantics`) answering
  `IterableKind::Counted { init, condition, next }` and
  `IterableKind::Condition`, with `CompletionProtocol::Absorb(&[Break,
  Continue])`; `foreach` with several var-lists and lists pads with the
  empty string.
- **Tests**: `the_loop_plans_name_their_bound_and_step`.
- **Gates**: G2, G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: VT12.1.

##### VT12.3 — the simulator runs the registry's routes

- **Files**: `rust/tcl-compiler/src/static_loops.rs`,
  `rust/tcl-compiler/src/value_transfer.rs`,
  `docs/design/compiler/value-transfers-migration.md` (the ledger row).
- **Items**: `exec_statement` applies each statement's resolved
  `evaluate` through the driver's lifted evaluation with the
  enumeration's state as its inputs; `exec_switch` reads the selection
  owner (VT6.2) over the state; `StaticValue` and `StaticEnv` give way to
  `ExactValue` and `Existence`; `parse_literal_value`,
  `resolve_switch_subject` and the `Incr` arm are deleted. A condition's
  math function resolves through the `math_function` service, so a
  module-rebound one declines (the slice 3 review fixes' found-and-left
  program: with `proc ::tcl::mathfunc::abs {x} {return 99}`, `for {set i
  0} {$i < abs(-3)} {incr i} {}; if {$i == 3} {puts three} else {puts
  other}` prints `other` from 8.5, where `tcl opt` rewrites the condition
  to `1` and drops the `else` arm).
- **Tests**: `a_loop_condition_reads_the_math_binding` (that program,
  optimised, prints what `tclsh` prints under every release).
- **Preserves**: every `summarise_*` answer; `summarise_respects_iteration_cap`
  now as the `Iterations` decline.
- **Gates**: G1, G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: VT12.1.

##### VT12.4 — the consumers of the exit state

- **Files**: `rust/tcl-compiler/src/sccp.rs` (`loop_summary_decision`),
  `rust/tcl-compiler/src/intervals.rs`, `rust/tcl-compiler/src/optimiser/propagation.rs`
  (`evaluate_proc_with_constants`).
- **Items**: `loop_summary_decision` reads the enumeration's exit state;
  `intervals` bypasses the header widening (`MAX_ITERS`) for an
  enumerated loop; the argument-sensitive O103 re-run enumerates the
  callee's loops under the call's seeds.
- **Changes**: the post-loop branches of the eleven witnesses decide
  through I230 as well as O101 (today only `tcl opt` decides two of
  them). Mandate: the exit; the Analysis row ("a bounded loop's exit
  state against its in-loop widening").
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: VT12.3.

##### VT12.5 — W240–W242 and the iRules loop bound read the plan

- **Files**: `rust/tcl-compiler/src/analyser/bounds_checks.rs`,
  `rust/tcl-compiler/src/analyser/irules_event_checks.rs`,
  `rust/xtask/src/value_transfers.rs`.
- **Items**: the W240–W242 seeding reads the iteration plan's bound and
  step (the `set v INT` / `incr v ?INT?` text scan goes); the IRULE5003
  loop-bound reader's `body_decrements` substring scan reads the plan.
- **Changes**: `set i $start; while {$i < 10} {incr i}` is checked where
  today the check is disabled. Mandate: the exit; the W240–W242 row.
- **Tests**: `w240_seeds_from_the_iteration_plan`.
- **Gates**: G1 (`irules_event_checks.rs` 7 → 6), G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: VT12.2.

##### VT12.6 — the slice's witnesses

- **Files**: `rust/tcl-compiler/tests/value_transfer_witnesses.rs`,
  `rust/tcl-cli/tests/value_transfers_cli.rs`.
- **Tests**: `the_eleven_loop_witnesses` (the page's list — `i` 5; `t`
  6; `i` 3 after `break`; `t` 3 with `continue`; `i` 3 for `$i < 2.5`;
  `i` 4 for the empty-init loop; `i` 4 and `n` 2; `i` 2 on the error
  path; `x` unbound after `foreach x {} {}`; `a` 3 and `b` empty; `n` 1
  and `x` 2), each through the lattice, I230 and `tcl opt`, whose output
  prints what `tclsh` prints; `the_correlated_pairs_decide_by_enumeration`.
- **Gates**: G7, G8, G9.
- **Model**: sonnet. **Size**: M. **After**: VT12.1 to VT12.5.

##### VT12.7 — docs and the landing

- **Files**: `sccp-core-analyses.md`, `constant-folding-type-inference.md`,
  `optimisation-passes.md`, `pass-fact-ownership-matrix.md`,
  `value-transfers-migration.md`, `docs/kcs/codes/` notes for W240–W242,
  the lane doc; diagnostics rows drafted (B-DP4).
- **Gates**: G5, G6, and the slice's green.
- **Model**: sonnet. **Size**: S. **After**: VT12.6.

#### Checkpoints and landing

| Checkpoint | Holds | Green means |
|---|---|---|
| `wip(value-transfers): slice 12 — the enumeration and the loop plans` | VT12.1, VT12.2 | the cap test; every existing loop answer |
| `wip(value-transfers): slice 12 — the simulator is the enumeration` | VT12.3, VT12.4 | the `summarise_*` tests; the static-summary branch test |
| `wip(value-transfers): slice 12 — bounded-loop enumeration` (landing) | VT12.5 to VT12.7 | every exit test; G1 to G9 |

```text
wip(value-transfers): slice 12 — bounded-loop enumeration

A loop whose iteration plan and pre-header state admit it is enumerated
in order over exact values and existence: each statement runs its
registry-owned evaluation, `switch` reads the shared selection, `break`,
`continue` and errors follow the completion protocol, and the exit state
is published on the exit edge only, with the header still widening
inside the loop. The iteration cap is a budget decline that publishes
nothing. The loop simulator has no arithmetic, ingress or subject
resolution of its own; the intervals bypass their widening for an
enumerated loop; W240–W242 and the iRules loop bound read the plan's
bound and step.

Behaviour changes: the eleven loop programs decide their post-loop
branches for the diagnostics as for the optimiser; the correlated
`foreach` pairs decide by enumeration; a loop counter seeded from a
variable is checked by W240–W242.
```

#### Review checklist

- R1: `static_loops.rs` holds no arithmetic and no command spelling;
  `irules_event_checks.rs` at 6.
- R2 to R5.
- R6: every `summarise_*` test byte-identical.
- Suites: `cargo test -p tcl-registry -p tcl-compiler -p tcl-cli`.
- R7: a capped enumeration publishes nothing; the exit state never
  narrows an in-loop version; an error path publishes its prefix only.

#### Behavioural deltas

| Delta | Mandate |
|---|---|
| post-loop branches decide for I230 as for O101 | the exit; the Analysis row |
| the correlated `foreach` pairs decide (20, 25) | the correlated finite-set limit's rung (enumeration supplies them) |
| W240–W242 seed from the plan | the exit |

### Slice 7a — seedless return summaries

#### Goal and exit

The part of slice 7 that slice 13 consumes, in the plan's words:
"`summarise_returns` consults a seedless lattice so the
argument-independent O103 sees computed returns." Its exit is that
sentence's witness: the argument-independent O103 folds a procedure whose
return is a computed constant.

Exit evidence: `o103_summary_path_folds_a_computed_return` (compiler
witnesses: `proc p {} {set x [string range foobar 0 2]; return $x}; puts
[p]` gives `puts foo` on the summary path, and `tcl opt --profile full`
prints `puts foo`);
`o103_folds_implicit_return_proc_cmd_subst` and
`o103_folds_arg_sensitive_passthrough_cmd_subst` unchanged.

#### Upstream starting point

`git diff 3b5eba8a origin/rust -- rust/tcl-compiler/src/interprocedural.rs
rust/tcl-compiler/src/optimiser/propagation.rs`: `interprocedural.rs` is
unchanged; `propagation.rs` gates the top-level propagation on the
caller's registry (`scan_module_global_names(module, registry)`), which
VT7a.1's staged run uses as is.

#### Work items

##### VT7a.1 — `summarise_returns` over the seedless lattice

- **Files**: `rust/tcl-compiler/src/interprocedural.rs`
  (`summarise_returns`, `classify_return`, `ReturnKind`),
  `rust/tcl-compiler/src/optimiser/propagation.rs`.
- **Items**: `summarise_returns` runs each callee's lattice with its
  parameters `Overdefined` (no seeds) in a staged fixed point over the
  call graph, bottom-up, a cycle bounded by
  `MAX_INTERPROCEDURAL_WALK_DEPTH` and answering computed; `classify_return`
  reads the lattice value at the return through the exact ingress (no
  `trim`); `enum ReturnKind` becomes `pub(crate)` and `Literal` carries
  the `ExactValue`.
- **Preserves**: every `ProcSummary` answer on a literal or passthrough
  return.
- **Changes**: the summary path folds a computed constant return.
  Mandate: slice 7's sentence.
- **Tests**: `o103_summary_path_folds_a_computed_return` (negative: a
  return that reads a parameter stays `UsesParam`).
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: slice 3.

##### VT7a.2 — witnesses, docs and the landing

- **Files**: `rust/tcl-compiler/tests/value_transfer_witnesses.rs`,
  `interprocedural-analysis.md`, `interprocedural-call-site-seeding.md`,
  `optimisation-passes.md` (O103), the lane doc.
- **Gates**: G6, and the slice's green.
- **Model**: sonnet. **Size**: S. **After**: VT7a.1.

#### Checkpoints and landing

One commit, `wip(value-transfers): slice 7a — seedless return summaries`:

```text
wip(value-transfers): slice 7a — seedless return summaries

`summarise_returns` reads each callee's lattice with no call-site seeds,
in a staged fixed point over the call graph, so a return that is
constant for every caller is a constant return and the
argument-independent O103 folds it; a return that depends on a seed
stays the argument-sensitive path's. `classify_return` reads the exact
value instead of a trimmed text.

Behaviour changes: O103's summary path folds a procedure whose return is
a computed constant.
```

#### Review checklist

R1 (no command spelling in `interprocedural.rs`'s new code), R2 to R5,
R6 (both O103 anchor tests byte-identical); suites `cargo test -p
tcl-compiler`; R7: a seed-dependent value never enters the summary.

#### Behavioural deltas

| Delta | Mandate |
|---|---|
| O103's summary path folds a computed constant return | slice 7's `summarise_returns` sentence |

### Slice 13 — proc-level transfer summaries

#### Goal and exit

In the plan's words: "`TransferSummary` beside `ProcSummary`: parameter
roles with a `Name` parameter's frame level and ordered outcomes, the
global places a callee may write, the seedless `ReturnKind`, the
completion domain, and the effect footprint, composed bottom-up with a
cycle resolving to `MayBind`, `Any`, and a computed result. The caller's
driver applies them at the call site, and the argument-sensitive re-run
seeds `Name` parameters from the caller's places." `KNOWN_GAPS` adds
`global`, `variable`, `my variable`, `sharedvar`, `info default`.

*Exit*: "`bump n` decides through both O103 paths, `param_traits.rs`
matches no command by name, the synthetic `<upvar-invalidate>` def is
gone, and the seven witnesses pass."

Exit evidence: `bump_decides_through_both_o103_paths`,
`the_seven_summary_witnesses` (compiler and CLI, 8.4 to 9.1); G1 with
`param_traits.rs`, `interprocedural.rs`, `analyser/diagnostics/helpers.rs`
and `var_escape/slot_resolution.rs` in `CLEAN_FILES` and no slice-13 row
in `KNOWN_GAPS`; `<upvar-invalidate>` absent from `cfg_builder/mod.rs`
and `SyntheticMarker`; `tcl explore --source 'proc bump {name} {upvar 1
$name v; incr v}; proc p {} {set n 1; bump n; return $n}' --show sccp
--text --no-colour` prints `n` after the call as `const(2)` in `::p`.

#### Upstream starting point

`git diff 3b5eba8a origin/rust -- rust/tcl-compiler/src/unit_scope.rs
rust/tcl-compiler/src/var_observability.rs rust/tcl-compiler/src/realm.rs
rust/tcl-compiler/src/cfg_builder rust/tcl-compiler/src/ir_helpers.rs`:

- `unit_scope.rs` (#2134): a command substitution nested in a word is a
  call site, so `[bump m]` counts as a caller and a single visible `bump
  n` no longer licenses the single-call-site rewrite. VT13.2 keeps that
  call-site evidence as the closed-call-set proof #2134's contract point
  asks for before any callee body is rewritten.
- `var_observability.rs` (+252), `realm.rs` (+358): registry-declared
  alias transitions and namespace-local shadows; the summaries'
  `globals` and binding evidence read them.
- `cfg_builder/mod.rs` and `ir_helpers.rs`: the synthetic
  `<upvar-invalidate>` call now carries `SyntheticMarker::UpvarInvalidate`
  and both embedded-effect halves — a proc's caller-frame defs and a
  nested cell update's read and write (#2050, #2141). Removing it (VT13.2)
  moves both halves onto the host statement.
- `global_write_info.rs`: writes from an in-frame expression word count;
  #2214's qualified-name half is VT2.9's.

#### Work items

##### VT13.1 — `TransferSummary`

- **Files**: `rust/tcl-compiler/src/interprocedural.rs`.
- **Items**, the page's shapes in the compiler:
  ```rust
  pub(crate) struct TransferSummary {
      pub params: Vec<ParamRole>,
      pub globals: Vec<(PlaceRef, ExistenceOutcome)>,
      pub result: ReturnKind,
      pub completion: CompletionCodeDomain,
      pub effects: EffectFootprint,
      pub evidence: DependencyEvidence,
  }
  pub(crate) enum ParamRole {
      Value,
      Name { level: FrameLevel, outcomes: Vec<ExistenceOutcome> },
      Unused,
  }
  ```
  computed from the seedless lattice (VT7a.1) beside `ProcSummary`,
  composed bottom-up (`twice`'s `Name` outcome is two `bump` updates), a
  cycle that does not converge within `MAX_INTERPROCEDURAL_WALK_DEPTH`
  answering `MayBind` for every `Name` place, `Any` and a computed
  result; an unknown callee, `has_unknown_calls`, or a suspect binding is
  a barrier.
- **Tests**: `summaries_compose_through_two_callees`,
  `a_cycle_that_does_not_converge_is_may_bind`.
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: L. **After**: slices 7a and 8.

##### VT13.2 — the caller applies the summary; the synthetic call goes

- **Files**: `rust/tcl-compiler/src/value_transfer.rs`,
  `rust/tcl-compiler/src/cfg_builder/mod.rs` (`upvar_invalidated`,
  `apply_upvar_invalidation`), `rust/tcl-compiler/src/ir.rs`
  (`SyntheticMarker`), `rust/tcl-compiler/src/ssa.rs`,
  `rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs`
  (`statement_is_synthetic_effect`).
- **Items**: a host statement carries its embedded effects
  (`EmbeddedSubstExtras { defs, read_before_write, opaque_global }`) as a
  field SSA reads like a call's defs and reads, in place of a prepended
  `<upvar-invalidate>` call; the driver resolves each `Name` argument to
  a place in the frame `level` selects, applies the outcomes in order (a
  cell update's value from the argument-sensitive re-run when O103 runs,
  a `MayWrite` with the summary's bounds otherwise) and the `globals`, and
  applies a nested cell update's writes through the ordered state (VT9.2).
  `SyntheticMarker::UpvarInvalidate` is deleted.
- **Preserves**: every #2050, #2141 and #2132 witness (the reads and
  writes the synthetic call carried are the host's).
- **Changes**: the caller sees a callee's `Name` and global effects as
  values and existence: `bump n; if {$n == 2}` decides. Mandate: the exit;
  the Summaries row.
- **Tests**: `bump_decides_through_both_o103_paths`.
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: L. **After**: VT13.1.

##### VT13.3 — the argument-sensitive re-run seeds `Name` parameters

- **Files**: `rust/tcl-compiler/src/optimiser/propagation.rs`
  (`evaluate_proc_with_constants`, `seed_params_from_args`).
- **Items**: `bump n` re-runs `bump` with `v` seeded from the caller's
  `n`; the re-run's outcome for `v` is the value VT13.2 applies.
- **Tests**: in `bump_decides_through_both_o103_paths`; the recursive
  `rec 4` folds to 10.
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: VT13.2.

##### VT13.4 — the name-matching consumers read the summary

- **Files**: `rust/tcl-compiler/src/analyser/param_traits.rs`,
  `rust/tcl-compiler/src/interprocedural.rs`,
  `rust/tcl-compiler/src/analyser/diagnostics/helpers.rs`,
  `rust/tcl-compiler/src/var_escape/slot_resolution.rs`,
  `rust/xtask/src/value_transfers.rs`.
- **Items**: `param_traits.rs`'s two-command copy tracker reads the
  `Name` outcomes; `interprocedural.rs` reads `upvar`'s level as a
  `FrameLevel` through `ParamRole::Name { level }` and the `global` /
  `variable` names through `Traits::CREATES_SCOPE_ALIAS` and
  `FrameArgLayout`; `helpers.rs`'s binder checks read the same;
  `slot_resolution.rs`'s `info level` / `info frame` and `trace`
  subcommand arms read the frame-effect and trace facts.
- **Gates**: G1 (`param_traits.rs` 1 → 0, `interprocedural.rs` 2 → 0,
  `helpers.rs` 4 → 0, `slot_resolution.rs` 6 → 0; all four clean), G7,
  G8, G9.
- **Model**: opus. **Size**: M. **After**: VT13.2.

##### VT13.5 — the binder commands' declarations

- **Files**: the spec modules of `global`, `variable`, `my variable`
  (TclOO), `sharedvar`, `info default`; `rust/xtask/src/value_transfers.rs`.
- **Items**: `global` and `variable` declare a scope-alias plan
  (existence `MayBound` at declaration, no value); `my variable` and
  `sharedvar` the same over their frames; `info default` a `Write` of
  the default value when the procedure and parameter are proven, else a
  `MayWrite`.
- **Gates**: G1 (five rows go), G2, G9.
- **Model**: sonnet. **Size**: S. **After**: VT13.1.

##### VT13.6 — invalidation

- **Files**: `rust/tcl-compiler/src/value_transfer.rs`
  (`AnalysisContextKey`), `rust/tcl-lsp-db/src/lib.rs`.
- **Items**: the summary revision rides the context's `seeds_revision`,
  so a `rename` or redefinition anywhere in the module invalidates every
  summary, and the memoised path keys on it.
- **Tests**: `a_rename_invalidates_every_summary` (`value_transfer_parity.rs`).
- **Gates**: G7, G8, G9.
- **Model**: opus. **Size**: S. **After**: VT13.1.

##### VT13.7 — the slice's witnesses

- **Files**: `rust/tcl-compiler/tests/value_transfer_witnesses.rs`,
  `rust/tcl-cli/tests/value_transfers_cli.rs`.
- **Tests**: `the_seven_summary_witnesses` — `bump n; bump n 2` is 4;
  `reset m; info exists m` is 0; `twice t` is 3; `g; set ::counter` is 5;
  `rec 4` is 10; `ctr; ctr; set ::hits` is 2 from 8.5 (8.4 errors); `bump
  absent` errors under 8.4 and is 1 from 8.5 — each under every release;
  `two_callers_share_one_summary` (`bump n; bump other` prints `2 11`);
  #2134's program keeps `upvar 1 $name v` in `bump`.
- **Gates**: G7, G8, G9.
- **Model**: sonnet. **Size**: M. **After**: VT13.1 to VT13.6.

##### VT13.8 — docs and the landing

- **Files**: `interprocedural-analysis.md`,
  `interprocedural-call-site-seeding.md`, `pass-fact-ownership-matrix.md`,
  `downstream-pass-contracts.md`, `optimisation-passes.md` (O100, O103),
  `value-transfers-migration.md`, the lane doc; diagnostics rows drafted
  (B-DP4).
- **Gates**: G6, and the slice's green.
- **Model**: sonnet. **Size**: S. **After**: VT13.7.

#### Checkpoints and landing

| Checkpoint | Holds | Green means |
|---|---|---|
| `wip(value-transfers): slice 13 — transfer summaries` | VT13.1, VT13.5, VT13.6 | the composition and cycle tests |
| `wip(value-transfers): slice 13 — the caller applies them` | VT13.2, VT13.3 | `bump` on both O103 paths; every #2050 / #2141 / #2132 witness |
| `wip(value-transfers): slice 13 — proc-level transfer summaries` (landing) | VT13.4, VT13.7, VT13.8 | every exit test; G1 to G9 |

```text
wip(value-transfers): slice 13 — proc-level transfer summaries

Every procedure has a transfer summary beside its `ProcSummary`, from its
seedless lattice: which parameters name a caller place and at what frame
level, the ordered outcomes the body applies to them, the global places
it may write, its return shape, its completion domain and its effects,
composed bottom-up, with a cycle answering may-bind. The caller's driver
applies the summary at the call — its `Name` places, its globals — and
the argument-sensitive re-run seeds a `Name` parameter from the caller's
place, so `bump n` decides on both O103 paths without the callee's body
being specialised to one call. The synthetic invalidation call is gone:
a statement carries its embedded effects. The parameter-trait tracker,
the interprocedural name lists, the binder checks and the frame and trace
arms read the summaries and the frame facts.

Behaviour changes: a callee's writes through `upvar` and to globals are
values and existence in the caller; `[rec 4]` folds; the binder commands
have semantics.

Pins #2134 (closed on rust by #2211); pins #2214's global write (closed
by slice 2).
```

#### Review checklist

- R1: `CLEAN_FILES` gains the four files; no consumer matches `"upvar"`,
  `"global"`, `"variable"`, `"info"` or `"trace"`.
- R2 to R5.
- R6: both O103 anchor tests byte-identical; every #2050 / #2141 witness
  byte-identical.
- Suites: `cargo test -p tcl-registry -p tcl-compiler -p tcl-lsp-db -p
  tcl-cli`.
- R7: a summary never applies a seed-dependent value without the re-run;
  an unknown or suspect callee is a barrier; a non-converging cycle is
  `MayBind`; a callee body is rewritten only under the proven closed call
  set.

#### Behavioural deltas

| Delta | Mandate |
|---|---|
| a callee's `upvar` and global effects reach the caller | the exit; the Summaries row |
| `[rec 4]` folds on the argument-sensitive path | the seven witnesses |
| `global`, `variable`, `my variable`, `sharedvar`, `info default` have semantics | `KNOWN_GAPS` |

### Slice 7 — broader execution and runtime consumers

#### Goal and exit

In the plan's words: "The declared executable catalogue grows with
independent oracle evidence — the iRules pure functions as shared cores
registered into the simulator, the tcllib candidates as their specs move
to SpecTcl; `summarise_returns` consults a seedless lattice so the
argument-independent O103 sees computed returns [landed as 7a]; then
package backing, intrinsic guards, the engine's WASM sibling, and
extensions in their own changes under registry-consumer-contracts.md."
The evaluation page adds "the remaining core-table rows as their
catalogue entries appear, and the `PLATFORM` / `WALL_CLOCK` exclusions as
a recorded reason"; `KNOWN_GAPS` adds `lset`, `ledit`, `lpop` (VT2.8), the
tcllib procedures, `tcl_findLibrary` and `tcltest::normalizePath`; the
ratchet ledger names `auto_path_eval.rs`, `uri_split.rs` and
`tcl-mcp/src/irule_gen.rs`.

*Exit*: "every command that declares purity has a route or an explicit
"none" with its reason in the inventory."

Exit evidence: G1's inventory with no row whose *Gap* reads `pure, no
route`; `KNOWN_GAPS` empty of slice-7 rows; `auto_path_eval.rs` and
`uri_split.rs` in `CLEAN_FILES`, `irule_gen.rs` at 2;
`irules_pure_functions_match_the_simulator_and_the_oracle` and the
performance report (VT7.9); `tcl explore --dialect f5-irules --source
'when RULE_INIT {set h [b64encode abc]}' --show sccp --text --no-colour`
prints `h#1 = const('YWJj')` and a `route b64encode: direct` line owned
by the registry.

#### Upstream starting point

`git diff 3b5eba8a origin/rust -- rust/tcl-irule-test rust/tcl-cmd-core/src
rust/tcl-compiler/src/auto_path_eval.rs rust/tcl-compiler/src/uri_split.rs
rust/tcl-mcp/src/irule_gen.rs rust/tcl-compiler/src/codegen`:
the simulator gained an expression-operator layer (`expr_ops.tcl`, +720)
and a live-device harness (`live.rs`, +378), which VT7.1 registers the
cores beside; `tcl-cmd-core`'s `lsearch.rs`, `sort.rs`, `string.rs`
(`string repeat` bounded before allocating) and `format.rs` changed, and
their routes inherit the merged cores; `codegen/cmd_subst.rs` (+283),
`statements.rs`, `control_flow.rs` and the emitter changed around
VT7.8's folders; the other three files are unchanged.

#### Work items

##### VT7.1 — the iRules pure functions as shared cores

- **Files**: `rust/tcl-cmd-core/src/irules.rs` (new: `b64encode`,
  `b64decode` over the base64 core in `tcl_cmd_core::binary`, `crc32`,
  `md5`, `sha1`, `sha256`, `sha384`, `sha512` over one digest core,
  `htonl`, `htons`, `ntohl`, `ntohs`, `findstr`, `getfield`, `substr`,
  `domain`, `URI::basename` / `path` / `query` / `host` / `port` /
  `protocol` / `decode` / `encode` / `escape` / `compare`, `IP::addr A
  equals B`), the iRules spec modules (declared routes),
  `rust/tcl-irule-test/tcl/command_mocks.tcl`, `_mock_stubs.tcl`,
  `rust/xtask/src/gen_irule_test_data.rs`.
- **Items**: each a registry-owned direct route over the core; the
  simulator registers the same cores as host commands in place of the
  generated stubs that return the empty string.
- **Tests**: `irules_pure_functions_match_the_simulator_and_the_oracle`
  (the Tcl-expressible ones — `b64encode`'s `binary encode base64`
  equivalent, `crc32` through `zlib crc32` from 8.6 — against `tclsh`; the
  rest against published vectors).
- **Gates**: G1, G2, G4 (`gen-irule-test-data`), G7, G8, G9.
- **Model**: opus. **Size**: L. **After**: slice 13.

##### VT7.2 — `lset`, `ledit`, `lpop`

- **Files**: `rust/tcl-cmd-core/src/list.rs` (the three cores, adopted by
  both runtimes' adapters), `rust/tcl-registry/src/value_transfer/`, the
  three spec modules, `differential_fold.rs`, `rust/xtask/src/value_transfers.rs`.
- **Items**: list cell updates over the new cores (`lpop` from 9.0,
  `ledit` from 9.1): the result and one `Write`, a pop returning the
  removed element and writing the remainder.
- **Tests**: `list_cell_updates_match_every_release_on_path`.
- **Gates**: G1, G2, G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: —.

##### VT7.3 — the path routes

- **Files**: the `file` spec module (`join`, `dirname`, `tail`,
  `extension`, `rootname`, `split`, `normalize`),
  `rust/tcl-compiler/src/auto_path_eval.rs`,
  `rust/tcl-lsp-core/src/document_links.rs`,
  `rust/tcl-lsp-core/src/package_resolver.rs`.
- **Items**: platform-conditional routes (`Needs::PLATFORM` declines
  where the profile does not fix the platform); the private path folders
  read `proven_word_value`.
- **Gates**: G1 (`auto_path_eval.rs` 3 → 0), G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: —.

##### VT7.4 — `uri_split.rs` reads routes

- **Files**: `rust/tcl-compiler/src/uri_split.rs`.
- **Items**: `split`, `string first` and `string match` routes replace the
  private URI evaluator.
- **Gates**: G1 (6 → 0), G7, G8, G9.
- **Model**: opus. **Size**: M. **After**: —.

##### VT7.5 — tcllib declared implementations

- **Files**: the tcllib `.tclspec` packs as they move to SpecTcl;
  `rust/xtask/src/value_transfers.rs`.
- **Items**: `evaluate -implementation` bodies for the page's candidate
  list (`base32`, `ip::normalize` … `ip::collapse`, `uri::canonicalize`,
  `textutil::*`, `json::json2dict`, `csv::split`, `struct::list`,
  `math::statistics::*`, …), each proven against the real command by the
  pack's `tclsh` differential; `tcl_findLibrary` and
  `tcltest::normalizePath` declare `EvalRoute::None` with their reason.
- **Gates**: G1, G2, G9 (`tcl-spectcl`'s `spec_corpus`).
- **Model**: opus. **Size**: L. **After**: the tcllib specs' move to
  SpecTcl (B-CC7).

##### VT7.6 — every pure command has a route or a reason

- **Files**: the spec modules the inventory lists as `pure, no route`.
- **Items**: a route where a shared core exists, otherwise
  `EvalRoute::None { reason }` — `Declared` with the `PLATFORM` or
  `WALL_CLOCK` exclusion named, `Callback`, `FormUnsupported` or
  `Unauthored`.
- **Gates**: G1 (the exit), G2, G9.
- **Model**: sonnet. **Size**: L. **After**: VT7.1 to VT7.5.

##### VT7.7 — `irule_gen.rs` reads the resolved cell update

- **Files**: `rust/tcl-mcp/src/irule_gen.rs`.
- **Items**: the `set` and `incr` recognisers behind the CMP-sensitivity
  facts resolve the cell update; `table` and the terminal-action table
  are `side_effects` debt, waived by axis.
- **Gates**: G1 (5 → 2), G7, G8, G9 (`tcl-mcp`).
- **Model**: sonnet. **Size**: S. **After**: —; B-DP6.

##### VT7.8 — the codegen folders retire

- **Files**: `rust/tcl-compiler/src/codegen/helpers.rs` (`fold_list_cmd`,
  `try_format_fold`), `rust/tcl-compiler/src/codegen/values.rs`
  (`try_emit_constant_fold`).
- **Items**: codegen reads the registry's routes through
  `evaluate_literal`, so one implementation answers the lattice, O129 and
  codegen.
- **Preserves**: every `codegen_golden` output.
- **Gates**: G7, G8, G9 (`codegen`, `codegen_golden`, `codegen_depth`).
- **Model**: opus. **Size**: M. **After**: —.

##### VT7.9 — performance acceptance

- **Files**: `rust/tcl-compiler/benches/value_transfers.rs` (new) and its
  `Cargo.toml` entry.
- **Items**: the evaluation page's comparison — the unchanged tree, direct
  cores, expression evaluation and declared execution on one workload —
  cold host setup, warm evaluation, cache hits, changed inputs, solver
  iterations, cancellation latency, memory and incremental editor latency,
  reported with `RouteTally` (VT3.9).
- **Gates**: G7, G8.
- **Model**: sonnet. **Size**: M. **After**: VT7.6.

##### VT7.10 — docs and the lane's close

- **Files**: every design page the slices touched (status lines),
  `docs/design/lanes/README.md` (the entry goes),
  `docs/design/lanes/value-transfers.md` (removed; its content folds into
  the final commit message, as the lanes README requires).
- **Gates**: G5, G6, and the slice's green.
- **Model**: sonnet. **Size**: S. **After**: VT7.1 to VT7.9.

#### Checkpoints and landing

| Checkpoint | Holds | Green means |
|---|---|---|
| `wip(value-transfers): slice 7 — the iRules cores and the list cell updates` | VT7.1, VT7.2 | the simulator and oracle tests |
| `wip(value-transfers): slice 7 — the path and URI routes` | VT7.3, VT7.4, VT7.7, VT7.8 | G1 pins; codegen goldens |
| `wip(value-transfers): slice 7 — broader execution and runtime consumers` (landing) | VT7.5, VT7.6, VT7.9, VT7.10 | the exit; G1 to G9 |

```text
wip(value-transfers): slice 7 — broader execution and runtime consumers

The executable catalogue grows with oracle evidence: the iRules pure
functions are shared cores the simulator runs, `lset`, `ledit` and
`lpop` are list cell updates over shared cores, the `file` path
subcommands and the URI helpers read routes instead of private
evaluators, the tcllib candidates carry proven declared implementations
as their specs move to SpecTcl, and codegen folds through the same
routes. Every command that declares purity has a route or a recorded
reason for having none. The lane's tracking document is folded into this
message and removed.

Behaviour changes: iRules pure functions fold in `RULE_INIT` bodies and
over literal arguments; list cell updates reach the lattice; the path
and URI checks see computed values.
```

#### Review checklist

- R1: `CLEAN_FILES` gains `auto_path_eval.rs`, `uri_split.rs`;
  `irule_gen.rs` at 2; no consumer names an iRules function.
- R2 to R5; new files: `irules.rs`, the bench.
- R6: every simulator, codegen and `spec_corpus` test byte-identical
  except the stubs the cores replace.
- Suites: `cargo test -p tcl-cmd-core -p tcl-registry -p tcl-compiler -p
  tcl-irule-test -p tcl-lsp-core -p tcl-mcp -p tcl-spectcl`.
- R7: a platform- or clock-dependent core never answers under a profile
  that does not fix the axis; a declared implementation's answer is
  cached only on proven-equal inputs.

#### Behavioural deltas

| Delta | Mandate |
|---|---|
| iRules pure functions fold over literals | § *Third-party commands*, Tier 1 |
| `lset` / `ledit` / `lpop` write exact values | Tier 1; VT2.8's reclassification (D5) |
| path and URI checks read computed values | the ratchet ledger rows |
| every pure command has a route or a reason | the exit |

### Witnesses

#### The findings table, and what upstream already closes

Issue states as of 2026-09-22. "Upstream" is `origin/rust` at `08bceb36`,
which VT2.M merges. A defect upstream closes is pinned, never claimed: its
slice's landing message says "Pins #N (closed on rust by #PR)", and only a
defect that is open when its slice lands gets "Closes #N".

| Issue | State | Upstream | Slice | Pinned by | Landing wording |
|---|---|---|---|---|---|
| #2050 nested `incr` read | closed | fixed by #2215 (`uses_in_call`'s read-and-def rule, the same as VT2.0's) | 2 | `store_read_by_a_nested_cell_update_is_not_dead`; `set_result_incr_keeps_its_increment` | Pins #2050 (closed on rust by #2215) |
| #2051 no-match preserve | closed | fixed by #2225 (`CONDITIONAL_VARIABLE_WRITE`) | 5 | `a_no_match_keeps_the_store_it_preserves`; `w210_reads_a_no_match_preserve_outcome` | Pins #2051 (closed on rust by #2225) |
| #2052 trimmed ingress | closed | fixed by #2211 (`parse_literal_value` exact) | 2 | `var_piece_proven_by_the_lattice_folds_the_chain`; `evaluate_def_append_var_piece_reads_the_lattice_exactly`; `a_literal_keeps_its_surrounding_whitespace`; `o104_and_o130_fold_a_chain_through_a_lattice_operand` | Pins #2052 (closed on rust by #2211) |
| #2053 stray `"` | closed | fixed by `33be5cef`, already in HEAD's history | 2 | `deleting_a_quoted_store_leaves_no_quote` | Pins #2053 (closed by 33be5cef) |
| #2054 W231 length | closed | fixed by #2211 (`writes_the_name`) | 2 | `w231_length_follows_the_cell_updates`; `w231_abstains_after_a_write_it_cannot_measure` | Pins #2054 (closed on rust by #2211) |
| #2055 IRULE3101 | open | not addressed | 5 | `irule3101_reads_the_proven_path` | Closes #2055 |
| #2056 IRULE1201 in a dead arm | open | not addressed | 6 | `irule1201_ignores_a_respond_in_a_dead_arm` | Closes #2056 |
| #2057 W242 on a dead `while` | open | not addressed | 6 | `w240_and_w241_read_the_branch_fact` | Closes #2057 |
| #2118 O122 through a braced `expr` | open | PR #2226, open and not in `08bceb36` | — (not this lane's) | `o122_sees_a_self_call_inside_a_braced_expr`, added in slice 3 once #2226 is merged into the branch | never "Closes"; "Pins #2118 (closed on rust by #2226)" once it is |
| #2132 existence read is a use | closed | fixed by #2220 (conditions' command substitutions read the frame) | 8 | `o109_keeps_a_store_an_existence_read_observes` (statement-position reads beyond #2220's conditions) | Pins #2132 (closed on rust by #2220) |
| #2133 S100 on an unset arm | open | not addressed | 8 | `s100_ignores_an_unset_arm`; `s100_still_fires_between_the_bound_arms_of_a_three_way_switch` | Closes #2133 |
| #2134 `upvar` specialised to one call site | closed | fixed by #2211 (a nested `[bump m]` counts as a caller); the closed-call-set proof remains | 13 | `two_callers_share_one_summary`; `the_seven_summary_witnesses` | Pins #2134 (closed on rust by #2211) |
| #2141 O102 past a nested `[incr x]` | closed | fixed by #2215 (in-frame expression descent); the value forwarding is slice 9's | 9 | `the_seven_ordered_state_witnesses` | Pins #2141 (closed on rust by #2215) |
| #2142 `finally` on the error path | open | not addressed (#2207 flattens `catch`, not `try`) | 10 | `try_finally_runs_on_every_path` | Closes #2142 |
| #2143 materialised child's span | open | not addressed | 5 | `a_materialised_child_carries_its_factory_call_span` | Closes #2143 only with the W123 half (Q3); otherwise "Pins the span half of #2143" |
| #2144 `lassign` under 8.4 | closed | fixed by #2211 (`command_is_unavailable_here`) | 2 | `lassign_is_not_a_write_under_a_profile_without_it`; upstream's `a_write_by_a_command_the_profile_lacks_does_not_kill_the_store` | Pins #2144 (closed on rust by #2211) |

Two issues outside the table touch the lane: #2164 (a shadowed builtin
folded with builtin semantics; closed on `rust` by #2169) is pinned in
slice 2 by `the_shared_lattice_declines_a_renamed_head` and upstream's
`both_stances_decline_a_builtin_a_proc_shadows`; #2214 (a qualified
global written by a nested cell update; open, and still reproducing at
`08bceb36`) is closed by VT2.9.

#### Validation-matrix rows, by slice

| Row | Slices that satisfy it |
|---|---|
| Ownership | 2 (a qualified spelling and a subcommand share one declaration, VT2.6), 4 (the completion test's rename and subcommand form) |
| Templates | 5 |
| Summaries | 13 |
| Values | 2 (whitespace, NUL and backslash preservation; noncanonical integers; Unicode target differences; bignum promotion), 3 (bignum cancellation in `expr`), 5 (binary round trips) |
| Storage | 2 (missing, unknown and existing cells), 5 (partial `scan`, `regexp` no-match, repeated targets, array and base overlap, traced and escaping targets) |
| Existence | 8; 10 (the `unset p nosuch q` prefix) |
| Completion paths | 10 |
| Expressions | 3 (lazy branches, quoted versus braced, multiple arguments, nested commands, rebound math functions, unknown inputs, errors), 9 (the ordered state's nested writes, the error keeping the writes so far, `StatefulNested`) |
| Regexp | 5 |
| Rewrites | 2 (stateful producer retained; implicit return retained), 8 (failing dead write retained), 9 (effect ordering), every slice (traced places decline) |
| Analysis | 2 (the correlated limit in the lift), 3 (the mirror witnesses; `$a * $a`), 5 (per-target types), 12 (a bounded loop's exit state, the cap, `break`, `continue`, an error path, a zero-iteration `foreach`), 13 (summary recursion) |
| Incrementality | 2 (body edit, shifted spans, trace installation), 4 (overlay-only edit, target change, pack reload, worker migration), 13 (a rename in another proc) |
| Execution | 4 (persistent-state attempts, undeclared inputs, absent implementations, budget and cancellation, host unavailable or quarantined), 7 (expensive direct cores, VT7.9) |
| Branches | 6 (ordered patterns, final default, fall-through, regexp captures and errors, finite subject sets, opaque versus lowered), 11 (refinement per shape; the merge; the externally mutable place) |
| Diagnostic separation | 5 (one unbraced-expression fact; no private regexp or scan evaluator in W210); the rest is the diagnostic-policy lane's |
| Diagnostic integration | 2, 5, 6 (relative-span rebasing of explanations, template plans, selections), 8 (fast and deep availability); the rest is the diagnostic-policy lane's |
| Consumer parity and cost | 2 and every later slice's parity module; 2 (`file_token_facts_never_evaluates`) |
| Partial knowledge | 2 (per-version loss with reasons: the explanations), 3 (the floating-point counterexample; closed-subtree folding), 5 (shape facts in `folded_types`) |
| Full fact integration | 5 (dynamic return hooks keep their intrep; type, taint and range survive a decline) |
| Vendor iteration | 4 (VT4.14's pack loop spelling), 12 (zero and multiple iterations, completion) |
| BPF | 3 (the division difference: a BPF expression never takes the Tcl answer, VT3.12); the rest is the BPF frontend's |
| Catalogue completeness | 7 (every pure command has a route or a reason) |

### Boundaries

#### With the diagnostic-policy lane

The diagnostic-policy lane owns `rust/tcl-lsp-core/src/diagnostic_policy.rs`,
`diagnostic_report.rs`, `config_ini*`, `code_actions.rs`, the
diagnostic-code table, the catalogue generators, and the adapter edits in
`rust/tcl-lsp-server/src/lib.rs`, `rust/tcl-cli/src/commands/diag.rs`,
`policy.rs` and `rust/tcl-mcp/src/tools.rs`; it never edits
`tcl-compiler`, `tcl-registry`, `tcl-cmd-core`, `tcl-spectcl`,
`tcl-spec-hooks` or `rust/xtask/src/value_transfers*`.

- **B-DP1 — the crates it owns.** This lane edits, in `tcl-lsp-core`,
  only `hover.rs`, `inlay_hints.rs`, the semantic-token families,
  `document_links.rs`, `package_resolver.rs`, `refactor/extract_proc.rs`
  and `refactor/mod.rs` (slices 5 and 7), and in `tcl-mcp` only
  `irule_gen.rs` (VT7.7), each after the diagnostic-policy lane's commits
  to that crate have landed, rebased over them. Its CLI tests live in
  `rust/tcl-cli/tests/value_transfers_cli.rs`, never in `cli.rs`.
- **B-DP2 — `rust/tcl-lsp-db/src/lib.rs`.** The diagnostic-policy lane's
  `file_analysis` commit (DP4.0) lands first. This lane's footprint is
  `FnLatticeKey`, `ValueTransferContext`, `function_lattice`,
  `compilation_unit` (VT4.8) and the one `mod value_transfer_parity;`
  line (VT2.11); its tests live in `value_transfer_parity.rs`. Neither lane
  stages the other's hunk.
- **B-DP3 — O111 and W100.** The unbraced-expression fact is the
  analyser's produced W100 set: the diagnostic-policy lane's DP8.1 keeps
  W100 computed under every policy (`FACT_CODES`), and DP8.2's
  `brace_expr_hints` emits O111 at each produced W100's span, replacing
  `with_brace_expr_hints` and `append_brace_expr_perf_hints`. This lane
  adds no second representation (D23) and pins, in VT5.13, that the
  produced set marks every unbraced expression.
- **B-DP4 — the owner documents.** `diagnostics-calculation.md` and
  `diagnostics-integration.md` are the diagnostic-policy lane's (its slice
  10 rewrites them). This lane drafts each slice's rows in its tracking
  document — the value-transfer producer, the existence fact, the
  selection record, the refinements, the summaries — and commits them
  only after that lane's slice 10 has landed. `pass-fact-ownership-matrix.md`
  is shared row by row (the diagnostic-policy lane edits the `tcl-lsp-db`
  row); `shared-utility-contracts-rust.md`'s "constant evaluation and value
  transfers" owner row is this lane's, each lane committing after the
  other's pending hunk is committed.
- **B-DP5 — the analyser's diagnostics.** The producers this plan changes
  — W210's preserve consumer (VT5.11), the existence consumers W210, W211,
  W213, W214 (VT8.4), S100's existence reading (VT8.6), W102 (VT5.10),
  W240 and W241 (VT6.7), the literal-only checks (VT5.16) — are
  `tcl-compiler` code and this lane's. They emit `Diagnostic` exactly as
  today, apply no policy (rule 1 of § *Diagnostics consume facts*), and
  reach the diagnostic-policy lane's `Finding` and `apply` unchanged. A
  fixture of that lane's truth table or parity tests whose program a
  slice's delta touches is named in the slice's landing message. The
  three questions that lane's plan puts to the `tcl-compiler` owner (the
  file-directive fold, W305's self-filter, a single-bucket
  `line_suppressed`) are not value-transfer work and stay with the owner
  (Q11).
- **B-DP6 — `tcl-mcp`.** `tools.rs` is the diagnostic-policy lane's,
  `spectcl.rs` the consumer-contracts lane's (B-CC4), `irule_gen.rs` this
  lane's.

#### With the consumer-contracts lane

- **B-CC1 — `rust/tcl-registry/src/spec.rs` and `resolved_invocation.rs`.**
  This lane owns the `semantics` field on the three scopes and
  `InvocationSemantics::value`; the consumer-contracts lane adds
  `clause_grammar`, `option_effect_families`, `alias_of`,
  `runtime_backing`, the stamp windows and the query methods (CC2.8), and
  deletes `substitution_resolver` and the five `CaseListSpec` option
  fields. Disjoint fields and impl blocks; whichever lands second rebases.
  CC2.8's `AnalysisContext::surface_query` is a two-line addition to this
  lane's `context.rs`, accepted.
- **B-CC2 — `subst`.** CC2.6 owns `subst_.rs`'s option rows,
  `option_effects` and the `substitutions_performed` projection; VT5.8
  builds `SubstSemantics` over `option_effects` and adds only the
  `semantics` field and `value_transfer/template.rs`. CC2.6 lands first.
- **B-CC3 — slice 4 against steps 2 to 4.** The loader, `render_spectcl.rs`,
  `schema.rs`, `draft.rs`, `help.rs` and `coverage.rs`: slice 4 adds the
  `semantics`, `evaluate` and `facts` statements; step 2 adds
  `clause_grammar`, `-effect`, option `-effect` and the resolver family;
  step 4 `alias_of`. Each is its own statement; the `GAPS` table is edited
  by both (this lane removes `semantics`). Step 2 lands before slice 4
  where possible; the second rebases and re-runs `spectcl_roundtrip`.
  Step 3's `trust` on `EvalOptions` / `EvalSnapshotKey` is that lane's;
  slice 4 does not edit `loader/eval.rs` (D13). A declared implementation
  of an alias names the alias target's identity from step 4's `alias_of`.
- **B-CC4 — `spectcl_check`.** `rust/tcl-mcp/src/spectcl.rs` is CC3.4's
  first; VT4.12's three findings follow it.
- **B-CC5 — the analyser hooks.** CC2.12's generic
  `handle_var_binding_command` and CC2.13's retirement put `Set`,
  `DictWith` and `RegexPatternCapture` on the ledger "until slice 5|8":
  VT5.4 retires `RegexPatternCapture`, VT5.7 `DictWith`, VT8.9 `Set`, each
  after CC2.13's re-baseline and, for `Set`, CC2.12.
- **B-CC6 — `HandlerMatch`.** VT10.4 reads each `try` handler's match
  from CC2.2's clause-grammar descriptor.
- **B-CC7 — the tcllib specs.** VT7.5 follows the move of the tcllib
  specs to SpecTcl, whichever lane carries it.
- **B-CC8 — CC9.2.** `TclVersion::from_profile` for vendor profiles
  (`runtime_base`, `f5-irules` measured as 8.4) changes what
  `TargetSemantics::of` answers for them, so slice 2's unnamed-release
  answers under iRules become 8.4's; CC9.2 lands only with this lane's
  agreement and the `differential_fold.rs` iRules rows updated in the same
  checkpoint. *Recorded for that lane by VT4.1 (D72):* ruling 8 is built
  without `from_profile` — `TargetSemantics::of` reads
  `DialectProfile::runtime_version` — so CC9.2's change to `from_profile`
  no longer moves a value-transfer answer, and an evidence gate that
  should hold an unmeasured base back must reach `TargetSemantics::of`
  (this lane's file) instead. `differential_fold.rs` has no iRules row.
  `registry-consumer-contracts.md`'s "release for versioned evaluation" row
  still says every vendor environment evaluates under the invariant subset;
  for the value-transfer routes that is no longer so, and the row is that
  lane's to restate.
- **B-CC9 — `Engine::set_release` and the overlay.** VT4.1 lands
  `set_release` before CC8.2 uses it; VT4.8 lands the analysis half of
  `spec_pack_key` reaching `compilation_unit` before CC6.3's compile-service
  half.
- **B-CC10 — the ratchet in files that lane rewrites.** A stale pin fails
  G1, so the lane whose rewrite removes a site lowers the pin — never
  raises it — and the migration plan's ratchet row in the same commit.
  That is the one edit the consumer-contracts lane makes to
  `rust/xtask/src/value_transfers.rs` and the ratchet table, and it needs
  no request (D20). CC2.1's gate stays a sibling module; CC2.15 merges the
  lint roots after slice 2 lands.
- **B-CC11 — the option-effect descriptor for `subst` and `regexp`.** After
  VT2.M, `regexp_.rs` carries upstream's #2222 (`-about` and `-inline`
  name no match variables); CC2.6's `option_effects` rows for `regexp`
  build on the merged file, and VT5.4 reads them.

### Decisions taken

- **D1 — No squash onto the checkpoint.** VT2.0 committed on top of
  HEAD (`206f6eee`, `bdfe3a96`), as the plan requires: `3a83a9f8` already
  sat above `f3f9390f`, other lanes commit on top, shared history is not
  rewritten, and `git rebase -i` is unavailable. The lane doc's remaining
  step 5 still says "squash onto the checkpoint"; it is superseded. The
  hand-off's subject is VT2.13's landing commit, and the orchestrator
  squashes a slice's `wip` commits with its landing commit's body.
- **D2 — The hand-off's decisions.** Per-operation admission under an
  unnamed release stands (flagged, F1); `SOURCE_ENCODING` subsuming
  `CHAR_INDEXING` stands (the oracle: `string length héllo` is 6 under
  8.x and 5 under 9.x from a UTF-8 file); an error as a decline stands
  until VT10.1; the lift's one-finite-identity rule stands (it is the
  page's correlated limit); the interval model from the descriptor stands;
  `Constructed` evidence in a route's answer stands; `Pending` passing
  through a test helper stands; `string range` through `evaluate_literal`
  in both folders stands, with the unversioned `const_fold` restored
  (VT2.0, `fold_range_unanimous`); `FormatTemplate` transitional until
  slice 3 stands and is ledgered by VT2.7 and retired by VT3.8;
  committing the inventory from a failing run is superseded (VT2.0
  regenerated it from a passing run, and no later checkpoint commits one
  from a failing G1); deferring the request and iteration budgets partly
  stands (D3); the slice-1 rule that the mutation-fact-free shared
  lattice trusts every binding is withdrawn (VT2.M, #2164); "the #2050
  cause is not yet established" is resolved (VT2.0 traced it to
  `uses_of_classified`); VT2.0's two rules — a `reads` name the call also
  defines is a read of the prior value, and read-before-set does not
  claim an embedded cell update's read — stand, reconciled with
  upstream's identical rule and `statement_is_synthetic_effect` in VT2.M.
- **D3 — Budgets.** The request and the iteration levels land together in
  VT4.9: the iteration level divides the request's remaining work, so it
  has nothing to divide before the request exists; slice 12's cap is its
  own `BudgetLimit::Iterations`. This moves a row of the evaluation page's
  slice-2 landing ("the three nested budgets") to slice 4 (Q1).
- **D4 — `const` is slice 8's.** Only the absent case of `const` is a
  value (over a constant it keeps the old value, over an ordinary variable
  it errors), and existence is slice 8's fact.
- **D5 — `lset`, `ledit`, `lpop` are slice 7's.** `tcl-cmd-core` has no
  core for them; a new core is shared with the runtimes, which is slice
  7's oracle-evidenced catalogue growth.
- **D6 — `set` stays the typed `Assign` in statement position.** The
  route serves invocation-shaped uses (`[set x]`, the lexical consumers,
  slice 9's nested `[set x 10]`); the solver's native `Assign` transfer
  is unchanged.
- **D7 — Slice 7 splits** into 7a, before 13, and 7, last (§ *Order of
  delivery*).
- **D8 — `collect_existence_guards` goes in slice 11.** Slice 8 lands the
  existence-domain refinement and W210's reading of it; slice 11's exit
  names the helper's deletion, so its three callers move there.
- **D9 — The witness homes.** One compiler integration binary, one CLI
  binary (separate from `cli.rs`, which the diagnostic-policy lane edits),
  and one lane-owned test module in `tcl-lsp-db`; every later slice
  extends them.
- **D10 — Stores are confined in the engine.** `HostCommand` cannot reach
  the calling frame, so the page's host-command `ActivationStore` cannot
  keep a body's locals; `Engine::confine_stores` refuses a store outside
  the activation at the VM's name-resolving store path (Q4).
- **D11 — `Engine::set_release` takes the profile's name**, because
  `tcl-engine-api` is dependency-free by design.
- **D12 — `EvaluatorCapability` keeps `EvalRoute: Copy`** with `&'static`
  slices and a registry-side `ImplementationBudget`.
- **D13 — "Cache inputs (target values)"** is `EvalMemoKey`'s incoming
  targets in `pack_hooks.rs` (the evaluation page's § *The memo key*), not
  the loader's `EvalSnapshotKey`; the consumer-contracts lane's B1 line
  that attributes target values to `loader/eval.rs` reads as the pack-hook
  memo key.
- **D14 — A check that needs the unit's facts decides in the
  per-function pass** (`emit_cfg_ssa_diagnostics`), where the lattice
  is: W102 moves there wholly (VT5.10); W240 and W241 resolve the
  candidates the walk records (VT6.7); the literal-only checks keep their
  literal path in the walk and re-run there only for the words the walk
  abstained on (VT5.16). No finding is emitted twice.
- **D15 — `command_substitution_is_none`** stays byte-identical in slice 3
  (its `[clock seconds]` reads the wall clock under every policy) and is
  restated in slice 9 (Q2).
- **D16 — The analyser hooks retire per CC2.13's ledger**: `DictWith` and
  `RegexPatternCapture` in slice 5, `Set` in slice 8.
- **D17 — #2214 is fixed in slice 2.** It is a wrong-output rewrite in the
  nested-cell-update family slice 2's witnesses exercise, and waiting for
  slice 13's `globals` would leave it for eleven slices; slice 13's
  summary subsumes the fix.
- **D18 — The regexp owner's runtime delta is accepted.** A search that
  exhausts its fuel raises instead of answering 0 on the runtime path, as
  the evaluation page's step 3 states (Q8).
- **D19 — An `Unbind` in slice 5** publishes `Overdefined` for the value
  and waits for slice 8's existence fact.
- **D20 — A pin falls in the commit that removes its sites**, whichever
  lane makes that commit (B-CC10).
- **D21 — Upstream is absorbed, not duplicated** (VT2.M): one
  implementation per fact, re-expressed through the registry interface
  where slice 1 or 2 moved the logic; the witnesses upstream satisfies
  stay; the ratchet is re-measured. The shared lattice takes upstream's
  `FoldTrust::ObservedBindings` stance and a rewrite's re-run
  `FoldTrust::WholeModule` (F4).
- **D22 — A BPF expression declines** (VT3.12); the BPF arithmetic
  adapter is the BPF frontend's.
- **D23 — W100's produced findings are the unbraced-expression fact.**
  The diagnostic-policy lane's DP8.1 and DP8.2 already make them the one
  fact W100 and O111 read, so a compiler-side `UnbracedExpression` record
  would be a second representation of it; VT5.13 proves the produced set
  complete instead.

Taken while slice 2 was executed (its record, § *Slice 2* › *Record*, has
the witnesses):

- **D24 — One cooking rule for every literal word a route reads.**
  `value_transfer::literal_token_value` (a single-token `Esc` word through
  `backslash_subst_in`, a `Str` word through `collapse_braced_word`) serves
  the value-position folder, the call arguments (`call_arguments`, through
  the statement's token snapshot), `const_subst.rs`, the chain fold and
  W231's walk, because the raw spelling reached three readers beyond item
  8's, each a wrong rewrite or finding with an oracle witness.
- **D25 — The command trust is a field of the module facts.**
  `ModuleAnalysisFacts::command_trust`, and the interned
  `ValueTransferContext` holds the rebuilt `ModuleCommandMutations` beside
  its key (`ValueTransferContext::of`; `Hash` over the complete snapshot,
  so it agrees with `Eq`), rebuilt once per distinct context.
- **D26 — `store_def` takes no receiver** (pedantic `unused_self`), and the
  source-layout call path is `evaluate_source_call` (pedantic
  `too_many_lines`); `call_def` wraps both for the typed `incr` and every
  call.
- **D27 — #2214 is fixed at the caller too.** A `::`-qualified summary
  name also records the spelling a global-frame caller reads
  (`insert_outer_name`), and a call site records its callee's summary names
  as observed (`record_alias_observed`), because O109 read no callee reads;
  the summary records writes, not reads, so every such name counts as
  observed — conservative, and `set hits 10` ahead of a `bump` that runs
  `global hits; incr hits 5` is no longer deleted.
- **D28 — `set` finds its forms by role**, the write form one `VarWrite`
  operand with the value after it and last, the read form one `VarRead`
  operand alone; registry tests state the resolver's roles
  (`LiteralInputs::with_role`).
- **D29 — Three keyed-update answers the plan does not list**, each the
  oracle's under 8.5 to 9.1: an absent key's `dict incr d k 010` stores
  `010` as written once it proves an integer; a missing intermediate key
  of `dict unset` declines `WrongRepresentation`; an absent dictionary is
  empty only when the inputs prove the variable unbound.
- **D30 — The keyed-update differential asks the resolver**: the 8.4
  registry still answers a `dict` record, which the resolver does not
  select.
- **D31 — `llength` reads no release axis**: `ConstOps::list_len` runs the
  list parse alone, so `NEEDS = Needs::NONE` holds.
- **D32 — The pinned route set pins each route's owner.**
- **D33 — G2 has nothing to rewrite for the `tcl` commands**: they are the
  registry's Rust specs, and no shipped `.tclspec` declares `set` or
  `dict`.
- **D34 — The `lassign` witness pins the merged behaviour.** Under 8.4
  the store is not killed, so the optimiser forwards `old` into the
  `puts` and the store is then dead — the program prints `old`, as
  `tclsh8.4` does; the plan's "keeps `set a old`" predates #2211's
  forwarding. Under 8.6 O109 deletes it soundly.
- **D35 — The CLI witness binary waits for the diagnostic-policy lane.**
  The implementer's brief forbade touching `rust/tcl-cli` while that lane
  works there, so `value_transfers_cli.rs` and its shard row were not
  added. D9 stands: the next slice that may touch `rust/tcl-cli` adds the
  binary with the four tests VT2.10 names; until then their evidence is
  the hand-run record and the explorer and compiler witnesses. Resolved:
  VT3.10 is that slice (D65); the four tests are pinned, each re-checked
  against the hand-run record above before it was written.
- **D36 — Two parity tests read what the tree can observe.**
  `keyed_updates_agree_on_both_paths` seeds `d` with `set d {}`, because
  below the existence rung (slice 8) a local no store has bound is not
  proven absent; the unseeded program declines on both paths, which the
  test pins too. `file_token_facts_never_evaluates` observes the salsa
  executions: no lattice-building query runs behind the token tier, and
  no route is recorded anywhere else a test can read.
- **D37 — The examples page's restated lines are `merged:` lines.** Its
  `today:` lines are dated observations at `3b5eba8a`; the programs whose
  defects `rust` fixed, and those slice 2's routes changed, were re-run on
  this tree and marked `merged:`, with a status note, rather than
  re-running the whole corpus.

Taken while the review of slice 2 was answered (§ *Slice 2* › *Record
(2026-09-23): review fixes for slice 2* has the witnesses):

- **D38 — The typed assignment's token kind is already on its node.**
  The review asked for the value word's token kind on `AssignConst` and
  `AssignValue`. An `AssignConst` value is a braced word's content by
  construction (the lowering's other arm writes a canonical integer, which
  cooks to itself), and `AssignValue` carries `value_needs_backsubst`,
  which the lowering sets for a bare or quoted word with an escape. So the
  driver cooks them through `literal_token_value` as `Str` and `Esc`, and
  an unmarked spelling holding a backslash has no value; no IR field was
  added.
- **D39 — A typed node's head is every command lowered to it.**
  `typed_node_commands(registry, hook)` lists
  `command_names_for_semantic_operation(StructuredLowering(hook))`,
  shortest first. The typed `incr` resolves through the first and needs
  every one trusted; the typed assignment is overdefined unless every
  `Set` command is still its builtin. That is the chain fold's
  `observed_binding_is_the_builtin` question, asked once per driver; with
  no trust fact (a detached run) the lowering stands.
- **D40 — `FactView::exact` is the direct routes' one exact input.**
  `Exact` answers; `Pending` stays pending; `Finite` declines
  `CorrelatedSets`, because the lift evaluates per member before a route
  sees a set; `Domain` declines `MalformedAnswer`; `Top` carries its
  reason.
- **D41 — A braced `incr` amount is a flag beside its text**
  (`Statement::Incr::amount_braced`, as `name_braced` is for the name).
  SSA takes no use from it, inlining does not rename it, and the lattice
  and the loop simulator read it as a literal word. The codegen still
  loads it as a variable name, which is recorded as found and left.
- **D42 — List rendering reads the target in every consumer.** The rule
  is `TargetSemantics::quotes_leading_hash` (8.5 onward, `None` without a
  release), `TargetSemantics::render_list` (`None` where the releases
  disagree), and `ConstOps::new_list`, which poisons
  `ReleaseAmbiguous(ListRendering)`; `new_dict` gets the rule through
  `new_list`. O130's `render_list_word` and O103's `seed_params_from_args`
  render through `render_list` and skip the rewrite where it answers
  `None`.
- **D43 — #2232 is not closed here.** O100's word keeps the tab; the
  space is the CLI's stdout writer (`write_highlighted_output` with
  `DEFAULT_TAB_WIDTH`), which expands every tab in a program-emitting
  verb's stdout. That writer and its callers are the CLI's, which the
  diagnostic-policy lane holds. The lane pins the compiler's word, and
  the issue waits for the CLI's half.
- **D44 — #2231 waits for slice 10's `catch`.** Recording the body's
  reads needs a descent that orders a read before a write inside the
  body. It is needed for both the statement form and the nested `[catch
  …]`, which the substitution scan does not enter. That is VT10.3's body
  plan, not a contained edit.
- **D45 — A read-modify-write is declared where its route is.** The five
  `dict` keyed updates declare `READS_BEFORE_WRITE` on their subcommands,
  and the `::tcl::dict::` specs inherit it. The generic lowering and the
  variable-reference scan ask `invocation_traits` (`spec | sub`) rather
  than the head's spec. `a_route_that_reads_its_target_declares_the_read`
  holds every cell and keyed update to the trait.
- **D46 — `ValueTransferContext::of` keeps interning `mutations`** beside
  its key, as the review said. Revisiting it is a later tidy.

Taken while slice 3's opus items were executed (§ *Slice 3* › *Record
(2026-09-23): the opus items of slice 3* has the witnesses):

- **D47 — The engine adapter is crate-private.** `ExprServices` and
  `evaluate_expression` are `pub(crate)`, where the plan wrote `pub fn`,
  because the services borrow `dyn AnalysisInputs` and the const-folder's
  `FoldValue`, which stay inside the compiler. `ExprAnswer` is public.
- **D48 — The engine's value semantics are the const-folder's.**
  - `FoldOps::for_services` supplies the arithmetic, the comparisons and
    the math dispatch, so every braced answer is the old fold's.
  - The leading-zero rule is `FoldPolicy::octal`, the runtime base's: iRules
    reads `010` as 8, as it did.
  - The grammar is the policy's `NumberSyntax` when it has one, so 8.4 no
    longer reads `0b` or `0o`.
  - The integer tower and infinities follow the profile's `runtime_base`.
    From 8.5 a beyond-wide integer is its decimal spelling. Under an 8.4
    runtime it declines `WrongRepresentation`, because 8.4 wraps or raises.
    With no runtime it declines `ReleaseAmbiguous(IntTower)`. With no
    profile at all the 9.0 default stands, which keeps a detached caller's
    answers.
- **D49 — A quoted operand inside an expression is substituted.** The new
  hook `ExprOps::quoted_string` defaults to `string`. The services
  substitute the operand as a quoted word through
  `word_parts::decompose`. The const-folder, which cannot, declines one
  holding `$`, `[` or `\`. `if {"$a" eq "x"}` had been folded false, where
  tclsh 8.4 to 9.1 take the branch.
- **D50 — The services arrived with the engine.**
  - The math-function service's availability check keeps `min` from
    folding under 8.4 and `ABS` anywhere.
  - The nested service is the value position's path (`run_script`).
  - Both landed in the first checkpoint; the second holds their rebinding
    and evidence tests.
- **D51 — A math function's wrapper is a builtin to the binding scan.**
  `default_binding` counts `::tcl::mathfunc::NAME` as a builtin when the
  registry has its spec. A `proc` or `rename` of it is then a mutation the
  trust fact records, and the memo key carries it. `ModuleCommandMutations`
  had no other record of a qualified definition. Under 8.4 the service
  skips the check, since there are no wrapper commands to rebind.
- **D52 — A typed node's heads stand without a trust fact.** The fused
  assignment's `expr` and a math function are trusted as the typed
  assignment is (D39). A nested command's head needs the fact, as every
  route's does.
- **D53 — A branch condition is its own explanation.** `evaluate_condition`
  records the answer under the command `condition` at the condition's
  span. The condition rests on no `expr` binding, because `if` reads it
  itself.
- **D54 — The lattice inputs decompose a substituted word themselves.**
  - `OperandSource` says how each operand substitutes.
  - A lone variable read keeps its fact and identity, so the lift can pin
    it.
  - A finite part inside a concatenation declines `CorrelatedSets`.
  - A part that never resolves makes the word decline; otherwise a pending
    part makes it pending.
- **D55 — Regrouping needs every term to be a variable proven integer.** A
  closed left operand still folds (`fold_closed_left`), keeping the
  evaluation order. The plan's preserved `2 + 3 + $x` → `5 + $x` depends
  on it; the old output was `$x + 5`, a regrouping.
- **D56 — The format route decides per release.**
  - A named release runs once.
  - A profile naming none runs every modelled release and answers only
    when all agree, else `ReleaseAmbiguous(FormatVerbs)`; a numeral
    ambiguity inside a run keeps its own reason.
  - A release-gated conversion (`version_gated_uses`) raises below its
    release.
  - `fold_format`, the registry's const fold that codegen uses, is
    untouched.
- **D57 — The shared format core's zero precision follows the release.**
  8.4 formats through C's `printf`: `%.0d 0` is empty and `%#.0o 0` is `0`.
  8.5 to 9.1 render `0`. The core rendered empty everywhere, so the VM's
  `format %.0d 0` printed nothing where tclsh 9.0 prints 0.
  `format_witnesses_match_every_release_on_path` found it.
- **D58 — O103 renders a double as Tcl does** (`format_double`), not with
  Rust's `Display`. `proc p {} {return [expr {1.0 * 3}]}; puts [p]` had
  become `puts 3`, where tclsh prints `3.0` (`a_folded_call_keeps_a_doubles_spelling`).
- **D59 — The retired handler's code went with it:**
  - `transitional_direct` and its waiver;
  - `codegen::helpers::try_format_fold`, with its tests (its escape-grammar
    witness moved to `format_folds_through_the_shared_core`);
  - the `DirectRoute` declaration type, with its last user.
- **D60 — `simple_var_ref_name` reads one reference only.** `${a} + ${b}`
  is not the braced name `a} + ${b`. A lowered quoted `expr` word is
  spelled that way.
- **D61 — The optimiser samples move with the change that moves them.**
  This is the coordinator's gate. `set candidate [expr {$request_count + 1
  + 2}]` keeps its chain, because nothing proves `request_count` an
  integer. The standard, full and aggressive counts fall from 28, 35 and
  45 to 27, 34 and 44, and the README prose says why.

Taken while slice 3's sonnet items were executed (§ *Slice 3* › *Record
(2026-09-23): the opus items of slice 3* has the witnesses):

- **D62 — The route tally resets at the top of each SCCP sweep.** The
  fixed point re-evaluates every executable statement each sweep until
  its answers stop changing, and once more to finalise
  (`rust/tcl-compiler/src/sccp.rs`'s `loop { … }`), so a counter
  incremented at every call to `call_def` / `run_script` /
  `evaluate_assign_expr` / `evaluate_condition` triples the true count on
  a straight-line program with no loop or branch to re-converge.
  `LatticeDriver::reset_tally_for_sweep` zeroes `RouteTally` at the top of
  each sweep, so only the last (settled) sweep's entries survive to
  `SccpResult::route_tally`; the branch-fold pass that runs once after the
  fixed point (`collect_constant_branches`) adds to that settled count,
  which is why a single decided `if {1} {…}` counts as two expression
  entries (reachability, then the collected branch), not one.
- **D63 — `expression_witnesses_match_every_release_on_path` is
  `tcl-compiler`'s, not `tcl-registry`'s.** D47 made `ExprServices` and
  `evaluate_expression` `pub(crate)` to `tcl-compiler`, so `tcl-registry`
  — which carries no dev-dependency on `tcl-compiler`, and gains none here
  — cannot run an `expr` program at all. The plan's file list named
  `rust/tcl-registry/tests/differential_fold.rs`, the precedent VT3.8's
  `format_witnesses_match_every_release_on_path` set; that precedent
  holds because `format`'s whole route is registry-owned; `expr`'s is
  not. The test instead joins the other acceptance-list witnesses in
  `rust/tcl-compiler/tests/value_transfer_witnesses.rs`;
  `differential_fold.rs` is untouched.
- **D64 — The mirror pairs decline `not-exact`, not `CorrelatedSets`,
  before slice 5.** `foreach {a b} …`'s binders decline their source
  layout until slice 5 (the ledger's `foreach` / `lmap` row: "declining
  the source layout until slice 5"), so on this tree `a` and `b` are
  `Overdefined` from the header rather than the interface page's two
  distinct `Finite` identities the correlated limit would refuse with
  `CorrelatedSets` — `expr {$b / $a}` declines `not-exact` instead, an
  operand-shape decline, not the correlated one. `x` never folds to 20
  nor `y` to 25, and neither post-loop branch decides, exactly as the
  page says; slice 5 gives the decline reason its named shape, and
  `the_mirror_pairs_decline_as_correlated` pins the outcome, not the
  reason string, so it needs no change when slice 5 lands.
- **D65 — VT2.10's CLI witness binary is VT3.10's file, extended in its
  own commit.** D35 deferred `rust/tcl-cli/tests/value_transfers_cli.rs`
  to "the next slice that may touch `rust/tcl-cli`"; VT3.10 is that
  slice, creating the file for `explore_sccp_prints_the_route_tally`. The
  four tests VT2.10 named — `explore_sccp_prints_the_route_of_each_statement`,
  `opt_forwards_program_three`, `opt_keeps_the_store_behind_a_nested_increment`,
  and VT2.9's `opt_keeps_a_global_a_nested_increment_writes` — land in the
  same file afterward, in their own commit titled for slice 2, the slice
  whose item they finish; D9's "one CLI binary" stands, one file, two
  commits.

Taken while the review of slice 3 was answered (§ *Slice 3* › *Record
(2026-09-23): review fixes for slice 3* has the witnesses):

- **D66 — A namespace-local math function is a module-wide rebinding.**
  `builtin_shadowed_by_qualified_definition` maps a definition inside any
  `tcl::mathfunc` namespace (`::ns::tcl::mathfunc::abs`) to the global
  wrapper it shadows (`::tcl::mathfunc::abs`), which the trust fact then
  distrusts for the whole module, as the `::n::expr` case already is.
  Narrowing it to the defining namespace needs a resolution namespace at
  every call site, which `trusts` does not take; the whole-module stance
  is sound and costs only the fold.
- **D67 — A condition reads only a canonical digit string as a number.**
  `truth_of` has no release to ask, so a leading-zero digit string (`08`,
  `-08`, `007`, `00`) decides nothing rather than guess the grammar that
  left it text. `00` is false on every release, and the decline forgoes
  only that.
- **D68 — The rewrites consume the route; the old folder keeps only the
  gates that need no module facts.** O101, the branch folds, the
  propagation folds and O112 evaluate on
  `value_transfer::evaluate_expression_detached` and
  `decide_condition_detached`: a detached `LatticeDriver` over
  `PassContext::rewrite_folds` (the pass's registry, mutation facts and
  whole-module trust), with no solver run and a nested command declining.
  That is the lattice's evidence — the binding (namespace-local override
  included), the availability, the case and the tower — so a rewrite
  folds only what the lattice proves. Threading the gates into the old
  folder instead would duplicate the binding service, which needs module
  facts the folder never had. The folder keeps its numeric callers (code
  generation's constant operands, the static loop simulator) and gains the
  three gates that need none: case-sensitive names, availability from the
  dialect's ceiling in `eval_tcl_expr_with_policy`, and the tower.
- **D69 — The tower covers an infinity anywhere in the walk.** One test,
  `past_the_wide_tower` (a beyond-wide integer or an infinity, as an
  operand, a string operand's reading or a result), serves both
  evaluators, under the one rule `widens_past_a_wide`: from 8.5, or no
  dialect named; never an 8.4 runtime or a profile naming no release. The
  old folder holds it as a three-state `Tower` (widens, wide, breached),
  not two more flags: pedantic clippy's `struct_excessive_bools` stops a
  fourth `bool` on `FoldOps`, and the breach is a state of the tower.
- **D70 — A route entry is counted once the route is entered.**
  `call_def` counts after its route check and `evaluate_assign_expr`
  after its trust check. `run_script`'s `Implementation` arm neither
  enters nor counts, because no declared implementation runs before
  slice 4; VT4.6 counts it where one does, and `enter_implementation`
  went with its last caller.
- **D71 — One element-name assembly, one constant conversion.**
  `ExprServices::quoted_string` calls `value_transfer::variable_name`
  (now `pub(crate)`); `fold_expr_under_lattice` converts the lattice's
  constants once with `const_to_exact` and asks the route, so the second
  environment builder (`const_to_env_value`) and `sccp::tcl_value_to_const`
  went.

Taken while slice 4's opus items were executed (§ *Slice 4* › *Record
(2026-09-23): the opus items of slice 4* has the witnesses):

- **D72 — Ruling 8 reads the declared runtime, per axis.**
  `TargetSemantics::of` takes `DialectProfile::runtime_version` (the
  profile's `runtime_base`) as the release, not `TclVersion::from_profile`,
  which is CC9.2's file and stays untouched. The numeral grammar and the
  character model are the release's only where the profile's own
  declaration agrees (`grammar.numbers`, `character_model()`): a declared
  divergence — the F5 dialects' `Utf16CodeUnits`, where 8.4 counts BMP
  characters and UTF-8 bytes beyond — leaves that axis to unanimity, which
  is the ruling's "a vendor pack that diverges on an axis blocks the fold
  by declaring the axis". The expression route's evidence names the same
  release.
- **D73 — A release-less profile folds 8.4's math functions.**
  `fold_math_ceiling` floors a fold's function set at 8.4 for a profile
  naming no release, in the route's `math_function` service and in the old
  folder, since only those answer on every release. The availability
  diagnostic keeps `math_func_ceiling_for_dialect`'s unbounded ceiling, so
  it never flags a function one of the profile's releases has. Found by
  the witnesses' new `tcl` column.
- **D74 — Release pinning is per program and opt-in.**
  `HookProgram::release_pinned` (`pinned_to_release()`) runs a body on the
  pack's engine pinned to the call's profile (`HookCall::dialect`), one
  per (pack, profile, thread), each hook compiled on it at first use. A
  call naming no profile abstains rather than run at a default, and a
  profile no engine can pin is logged once. Every existing family stays on
  the unpinned engine, byte-identical; the evaluate family (VT4.6) is the
  first to set the flag. The hook cache's `ShapeKey` carries the profile,
  so a pinned answer is never served under another. The plan's "a pack
  whose capability names a release the engine cannot pin gets a load notice
  and no route" has no load-time half: a capability names no release (its
  `target` names axes), and the release is each call's profile, so the
  notice is the host's one error-log line the first time a profile cannot be
  pinned, and the answer under that profile is a decline.
- **D75 — An engine holds the thread's numeral grammar only while it
  runs.** `Vm::set_dialect_profile` installs the release's grammar for the
  whole thread, and the analysis thread that owns the engine reads
  numerals too. `GrammarGuard` claims the pinned release's grammar for
  each compile and invoke and restores the caller's on every exit,
  including building a fresh VM.
- **D76 — `set_release` resolves through the registry's ingress.** The
  plan said `DialectProfile::find`; the engine uses
  `resolve_known_environment(name).catalogue_profile()`, the one dialect
  ingress, so the lenient `tcl` sink, `tk`, `jim` and an unknown name — none
  of which names a release the VM can run — are `Unsupported`. The same pin
  twice is a no-op, and a different one after a unit was compiled is
  `Unsupported`: the VM does not switch release under compiled code.
- **D77 — Stores are confined at the VM's two store entries.** `Vm::set_var`
  and `write_array_raw_from` check `LimitSet::confined_stores` before
  writing: a name that resolves anywhere but the running procedure's own
  frame — a qualified or namespace name, level 0, a link, another level —
  raises `can't set "NAME": stores are confined to the activation`
  (`TCL WRITE VARNAME`). Every store path reaches one of the two: the
  bytecode store and increment ops, `lappend`, `foreach`, `lassign`,
  `scan`, `regexp`, `regsub`, `binary scan` and `dict`. The plan's "every
  store that resolves a name rather than a local slot" is every store in
  this VM, whose local-slot forms resolve by name as well. An engine that
  cannot confine fails the pack's sandbox, as one that cannot restrict does.
- **D78 — A confined VM publishes no error globals; its own bookkeeping
  runs unconfined.** Two VM-internal writes reach a global without passing
  a body's store entry. A caught error publishes `::errorInfo` and
  `::errorCode` (`publish_error`), so with stores confined `catch {lindex
  {} y}` still left both for the next invocation to read; confined, the VM
  now publishes neither, and the error's options still carry them.
  `catch` and `try` are off `SANDBOX_COMMANDS`, and the compiled `dict
  for`, `dict map`, `dict update` and `dict with` ranges do not publish, so
  the whitelisted host never reached the hole: the engine contract did,
  and the engine test pins it. The other direction is `set_host`'s
  rebootstrap of `::tcl_platform` and `::env`, which is the embedder's own
  bookkeeping and now lifts the confinement while it runs, so a host swapped
  in after `confine_stores` still gets its globals.
- **D79 — The capability's shape as built.** `EvaluatorCapability` is the
  plan's, its lists leaked `&'static` slices. `Exactness` has the one
  variant `Exact`: the page names the type and `arg N exact` is its only
  spelling. The loader writes `target: Needs::NONE`, because the four-row
  implementation block has no target row and the body runs pinned to the
  call's release (D74), so a profile naming no release declines before the
  host is asked (D88) rather than at admission. A `binding NAME` dependency is the name
  as written with its unrooted identity. `LanguageProfileId::ALL`,
  `HostKind::ALL`, `Exactness::ALL`, `ContextDependency::WORDS`,
  `OutcomeKind::ALL`, `DeclaredEffect::ALL` and
  `OptionEvaluation::REASONS` are the closed vocabularies VT4.10's
  `value_tables_cover_their_catalogues` rows read.
- **D80 — One declaration type per declaring scope.** The loader leaks a
  `DeclaredSemantics` (`value_transfer/declared.rs`) onto the scope's
  `semantics` field: the `semantics { … }` block's `DeclaredStructure`,
  the `evaluate` statement's `DeclaredEvaluation`, and the scope's option
  declines. The plan's `DeclaredImplementation` is its implementation
  half — the capability and the hook slot. The pack's name joins the
  implementation identity when the host plan binds the body
  (`DeclaredSemantics::bound`, reached through the new
  `CommandSemantics::as_declared`), because the evaluation loader's tables
  do not carry the pack name at command build and D13 keeps
  `loader/eval.rs` out of this slice.
- **D81 — `arg N` counts from the resolved form's first argument.**
  `ResolvedInvocationView::argument_offset` is the resolver's offset, set
  by the driver's `view_of` (1 past a subcommand word), so `tenant::label
  NAME` and `tenant label NAME` read one declaration, as an `arg` row at
  the same scope does.
- **D82 — The option flags live on the scope's declaration.**
  `-evaluate none` and `-evaluate-reason WORD` fill
  `DeclaredSemantics::option_declines`, not an `OptionSpec` field: 1,385
  `OptionSpec` literals in the command tables spell every field, so a new
  field is a sweep across them for a flag no shipped spec uses. Each entry
  is the option and the decline it records (D90). A flag on a scope that
  declares no route of its own is reported and dropped rather than read as
  `evaluate none`.
- **D83 — Vocabulary 2.2.** The three statements and the two flags are
  2.2 words (`log.since`), `KNOWN_VOCABULARY_VERSIONS` gains `2.2` and
  `NEWEST_VOCABULARY_VERSION` is `2.2`, so `tcl spec upgrade` and the
  studio's rendered header move to 2.2 with it.
- **D84 — A scope's declaration is sealed after each of its rows.** Every
  `semantics`, `evaluate` or flagged `option` row recomputes the scope's
  declaration and replaces its `evaluate` hook, so the last row read is
  what the scope declares. A post-pass cannot reach a subcommand's hook
  list: `subcommand_from_parts` has none, and the evaluation loader's
  `fill_command_nodes` (D13) owns the closure that does.
- **D85 — A form's declared implementation is dropped, and the form
  declares no route.** Hook bodies bind at command and subcommand scope —
  a form option's `-arity-hook` is already dropped for that reason — and
  `HookOwner` gains no form variant (`tcl-mcp` matches it exhaustively).
  The form's declaration becomes `None { Unauthored }` with a notice,
  because a form that inherited would run an evaluator written for another
  shape. `semantics none` beside an `evaluate` statement abstains for the
  scope, and the body is not bound.
- **D86 — The `evaluate` family.** `HookFamily::Evaluate` joins
  `HOOK_FAMILIES`; its parameters are the declared inputs, one argument
  each; its programs are release-pinned; its call carries the declared
  targets (`HookCall::targets`), so `write` and `preserve` validate
  against them in the verb and `answer_of` applies the third rule without
  the host reading any declaration. The answer is
  `HookAnswer::Evaluation(EvaluationAnswer)`. `Emission::Fold` stays
  `Fold(String)`, the existing spelling. Two exhaustive family matches
  outside the lane's crates gained the one arm they need to compile:
  `tcl-mcp/src/spectcl.rs`'s `family_key` (the consumer-contracts lane's
  file, B-CC4) and `tcl-spec-studio/src/store.rs`'s (VT4.11's file).
- **D87 — A `-native` or `-direct` id installs nothing before VT4.10.**
  The id rule holds now: a short id is a notice naming `SCOPE::FIELD`, and
  a full id is a notice that nothing this build ships holds it. VT4.10's
  tables make the full ids resolve.
- **D88 — What a declared implementation declines with.**
  `DeclaredSemantics::evaluate` resolves every declared input before it
  asks the host, so an input that is not exact is its fact's stand-in
  (`Pending`, `NotExact`, `CorrelatedSets`) and never a placeholder. A `{*}`
  word anywhere in the call is `NotExact`, because a declaration's indices
  are positions; a missing operand is `Unsupported`, because the command
  raises; two targets sharing storage are `OverlappingTargets`. A profile
  naming no release is `Unsupported`: no engine can be pinned to it, and
  a body has no counterpart of the direct routes' unanimity. No bound
  slot, no host, or a quarantined slot is `Transient`, and so is an
  abstention after which the slot is no longer available — the call itself
  blew its budget or panicked and the host quarantined it — so none of
  these is a verdict on the inputs. A body that raises, stays silent or
  leaves a target unstated is `Unsupported` (the DSL's "error means
  abstain"). An answer in another family's shape, a preserve under a
  `write` outcome, or any verb under `unbind` is `MalformedAnswer`. A body
  that states its stores and no `fold` has an `Unavailable` result: silence
  is never an empty result. The evidence carries the route identity (the
  implementation id, its content hash as the revision), the pinned
  release, and the declared `binding NAME` dependencies.
- **D89 — An option input is a list of zero or one element.** `option
  -NAME exact` binds `{}` when no argument word is `-NAME` (every word
  exact) and the one-element list of the next word's value when exactly
  one is, rendered under the target's list rule
  (`ReleaseAmbiguous(ListRendering)` where that rule is the release's);
  `-NAME` twice, or as the last word, is `Unsupported`. The page's memo key
  keeps "an absent optional input … distinguishable from an empty one",
  and a bare string could not. The reading is positional, as `arg N` is: a
  declaration names words, not the command's option grammar.
- **D90 — Option declines carry the decline they record.**
  `OptionEvaluation::decline` turns a flag into the `DeclineReason` at
  load: `form_unsupported` and `callback` are `NoRoute` with that reason,
  and `release_ambiguous` is `ReleaseAmbiguous` on the option's own
  `-available` row (`Axis::Availability`); an option without one gets a
  notice and records a bare `-evaluate none`. The check
  (`DeclaredSemantics::option_decline`) reads every argument word exact,
  so an unknown word is its fact's stand-in and a word spelling the option
  in a value position declines too — the conservative direction. The
  driver asks it before the expression route, whose adapter never reads
  the declaration; `evaluate` asks it for every other route.
- **D91 — A declared budget narrows the host's for one call.**
  `HookCall::budget` reaches the host, which sets each field to the
  smaller of its own and the declared one before the call and restores
  its own after it. A narrowed budget the engine refuses is an abstention,
  never a wider run; an overrun is the host's ordinary budget blowout —
  recorded, quarantined, and `Transient` from then on (D88).
  `HookHost::is_available` is `!is_quarantined`, which also answers
  `false` for a slot the host does not serve. The rule lives in the host
  alone: the loader records a `budget` row as written (the review of
  slice 4 removed its second check, against `HostConfig::default()`,
  which disagreed with any host configured otherwise).
- **D92 — VT4.6 rides with the second checkpoint.** Its edits share
  `pack_hooks.rs`, `declared.rs` and `host.rs` with VT4.3 to VT4.5, and a
  partial stage of those files would commit a state no build verified;
  the third checkpoint holds VT4.7 to VT4.9.
- **D93 — Test pins follow `NEWEST_VOCABULARY_VERSION`.** The loader's
  two version notices and the upgrade tests' current-pack fixtures read
  2.2, as the 2.1 bump moved them (#1754); the CLI's `tcl spec upgrade`
  default target stays `2.0`, its own flag's default.
- **D94 — The evaluator generation is shared by plan and private to a
  quarantine.** `EvaluatorGeneration` (`value_transfer/context.rs`) is set
  by `pack_hooks`: `NO_HOST` (0) on a worker without a host; one
  generation per published plan, so every worker serving it shares its
  memoised answers (`install_plan_host`, which `tcl-spectcl`'s
  `ensure_thread_host` now calls with its plan generation); a fresh one
  for a host installed without a plan and for every quarantine
  (`note_quarantine`, which the host's crash path calls in place of
  `clear_cache`). A per-thread counter would give two workers in different
  states one number, and a fresh number per install would keep the
  server's `spawn_blocking` workers from ever sharing a lattice.
  `evaluator_generation` builds the host first, as `dispatch` would, so the
  key names the evaluators the analysis runs with, and
  `AnalysisContextKey::for_module` reads it.
- **D95 — A cached hook answer is proven on the hit.** A content-keyed
  entry keeps the call's content — the words, the constraints view, and
  the `evaluate` family's targets, budget and dependency list, which
  `HookCall::depends` now carries — and a hit compares it; the key's
  `content` hash is the bucket, and a colliding bucket holds the latest
  content's answer. The profile is the key's `dialect`, compared exactly,
  in place of the plan's `TargetDigest`: `TargetSemantics::of` is a
  function of the profile. An incoming target reaches a body only bound
  with an exact value (D88), so its word is its value and its existence.
  The values of `registry_generation` and `binding NAME` stay the
  driver's — the memo key and the trust check — and the hook cache is
  per thread and cleared with its host.
- **D96 — A registry carries its generation and its overlay.**
  `CommandRegistry::generation` is drawn from a process-wide counter at
  construction and again at every mutation; `overlay_generation` is the
  pack-set key `registry_for_profile_with_overlay` stamps.
  `AnalysisContextKey::for_module` takes both from the registry the unit
  resolved against, so a unit built against an overlay — or against the
  un-overlaid fallback of one not built yet — keys every lattice by
  exactly that registry.
- **D97 — The overlay reaches every per-procedure query.**
  `compilation_unit` and `proc_taint_solve` take the overlay
  (`AnalyserConfig::spec_pack_key`) as an argument, resolved by
  `unit_registry` (the shared registry for `0`, so a workspace without
  packs resolves exactly as before); `function_lattice`,
  `function_checks`, `function_optimisations`, `taint_cascade` and
  `proc_summary_cascade` resolve by the overlay their key's context
  carries (`lattice_registry`); `ProcBodyKey` gains the overlay, so a
  procedure body lowers against the unit's surface. The server's
  `document_compilation_unit(db, file)` has no config and keeps no
  overlay (the server is not this lane's file); the semantic-token
  queries read `document_compilation_unit_for(db, file, config)`, the
  diagnostics path's build, so their unit agrees with the overlaid
  registry they already pass.
- **D98 — Salsa does not see the evaluator generation.**
  `compilation_unit` is memoised on its inputs, and a thread's generation
  is not one, so a unit built on one worker is served to another whatever
  that worker's generation; the lattice keys inside carry the builder's.
  Answers a healthy host computed are not wrong on a worker whose host has
  since quarantined — a quarantine is no verdict on answers already
  given — and a transient decline served to a healthy worker is only
  imprecise until the file changes. A salsa input for the generation would
  close it; the server would set it, and the server is not this lane's
  file (open question below).
- **D99 — The three nested budgets.** `Budget::request()` is 50,000,000
  units and 64 MiB retained; `Budget::iteration` a tenth of the request's
  remaining work; `Budget::evaluation_within` an evaluation that charges
  through both.
  `charge_work` propagates — the evaluation's own exhaustion is
  `Budget(Fuel)`, an iteration's or the request's `Budget(Request)` — and
  `charge_result` also charges the request's retained bytes. The driver
  opens one request per run (`LatticeDriver::new`) and one iteration per
  sweep (`open_iteration`, beside the tally reset). A declared
  implementation charges the commands its host's engine dispatched, one
  unit each (`pack_hooks::record_commands_spent` and
  `take_commands_spent`), after the body ran: an exhausted request
  declines the evaluations after it, not the answer it paid for.
  What the work bound means depends on the route that spends it
  (corrected by the review of slice 4: this row said "200 ms at four
  milliseconds per million", which holds for the native routes only). A
  native route's unit is calibrated at about four milliseconds per
  million, so the native work a request pays for stays near 200 ms. A
  declared implementation's unit is one engine command, which is not
  calibrated to time: the request bounds the commands its bodies
  dispatch — about 500 calls that each spend the bounded host's whole
  per-call allowance of 100,000 — and each call's time is bounded by the
  host's own 250 ms clock, not by the request. A request whose bodies
  each spend their whole allowance can run for up to 500 × 250 ms, and a
  body that dispatches few commands but runs long is bounded only by the
  clock, which quarantines it the first time it is reached.
- **D100 — An exhausted request declines every re-evaluated statement.**
  Each sweep re-evaluates every executable statement with a tenth of what
  the request has left, and the join keeps a decline, so once a run's
  evaluations spend its request every route-evaluated definition of the
  function ends `Overdefined` with `declined: budget: Request`, not only
  those past the point of exhaustion (measured: twelve `string length`
  folds under a 500-unit request all decline). Sound; the witness pins
  only that the declines are the request's and each publishes
  `Overdefined` (open question below).
- **D101 — A confined engine reads no host environment.** The page's route
  contract says the engine does not read a host environment, and its
  per-evaluation state reads "a body that *reads* `::counter` reads the
  empty string, because nothing ever writes it"; but the VM's bootstrap
  seeds `::env` with the analysing machine's environment, `::tcl_platform`
  with its platform facts and `::tcl_library` / `::auto_path` with its
  paths, so a body could fold `$::env(USER)` into an answer about a
  program that runs elsewhere. Confining stores now also removes every
  global the host bootstrap wrote (`tcl_platform::bootstrap::HOST_ARRAYS`
  and `HOST_PATH_GLOBALS`), and again after a host swap; a read of one
  raises, which is a decline. `Engine::confine_stores`' contract says so.
  The safe-interpreter scrub keeps the portable `tcl_platform` keys; this
  keeps none, because `PLATFORM` is never satisfiable on this route.
- **D102 — A declared implementation reads its inputs before its
  target.** `DeclaredSemantics::evaluate` asked for the release and
  admitted the target before it read the places and the inputs, so under a
  profile that names no release a call whose argument the analysis cannot
  know declined `Unsupported` rather than `NotExact`, and one whose input
  had not settled declined at once rather than stayed pending. The lift
  reads the inputs (step 2) before it evaluates (step 5), as every direct
  route reads its operands before `ConstOps::admit`; the places and the
  inputs are now read first, then the release, the admission and the
  host. An answer under a profile with a release does not change, and one
  under a profile without one still declines, with the input's own reason
  where it has one. Found by VT4.13's completion test under the lenient
  `tcl` profile; `an_unknown_input_declines_before_the_release_is_asked`
  (`value_transfer/declared.rs`) pins it.
- **D103 — The completion test's shape.** The fixture lives in
  `rust/tcl-compiler/tests/fixtures/value_transfers/`, where the plan puts
  it, and the compiler reads it through the pack loader: `tcl-spectcl`
  becomes a dev-dependency of `tcl-compiler`, the cycle Cargo permits and
  `tcl-registry` already has for the shipped packs; only the witness
  binary that uses it links it. The plan's one test is two, because the
  compiler cannot reach the CLI binary, the renderer or the studio:
  `the_completion_test_needs_no_consumer_edit` (the analysis over seven
  profiles, the optimiser, and the `tclsh` oracle with a vendor runtime —
  the three spellings as ordinary procedures the analysis never sees,
  written for 8.4 onward) and `the_completion_test_reaches_every_surface`
  (the CLI, the export, the renderer and the studio). The body keeps the
  page's `string cat`, so the example folds from 8.6 and declines under
  8.4 and 8.5 exactly where `tclsh` has no `string cat`; the declared
  implementation runs in the analysed release, and the witness says so.
  The renderer writes a `-native` placeholder for a body it cannot draw
  (VT4.11) and says so in the draft; the studio's carry-forward keeps the
  author's bytes over it, which is the preservation the completion test
  names.

- **D104 — The evaluator epoch is a salsa input** (Q12). A process-wide
  `pack_hooks::evaluator_epoch` moves when a hook plan is published
  (`tcl_spectcl::hooks::publish` calls `advance_evaluator_epoch`) and when
  a hook is quarantined (`note_quarantine`, on whichever thread ran it).
  `tcl_lsp_db::EvaluatorEpoch` is a singleton salsa input every
  `TclDatabase` constructor creates at 0 — a query that read its absence
  would record no dependency — at salsa's default `LOW` durability like
  every input there. `build_unit_with_keys` reads it, so `compilation_unit`
  and `proc_taint_solve` depend on it, and `ValueTransferContext` carries
  it, so a new epoch re-keys every memoised lattice: the smallest input
  that does, because the lattice keys already name the building worker's
  generation (D94) and only the unit query above them was blind to it
  (D98). The server sets it (`sync_evaluator_epoch`, compare-then-set, so
  a sync that finds nothing moved writes nothing) in `reload_spec_packs`
  when the set changed, before anything re-analyses, and at the start of
  each diagnostics pass's post-publish refresh, the first place the server
  can see a quarantine on the worker that ran the pass. It reschedules
  nothing for a quarantine: a quarantine is no verdict on the answers
  already published (D98), and a pass on a healthy worker would run the
  crashing body again. After a quarantine the next analysis recomputes
  every lattice on whichever worker runs it — a healthy one answers, the
  quarantined one declines `transient` — and whichever computes first is
  the memo every worker reads until the next edit or epoch: a quarantined
  worker that recomputes first still serves its `transient` to a healthy
  one, so the epoch shortens that window to one analysis rather than
  closing it (corrected by the review of slice 4). A content
  edit to a pack moves the pack key as well, which alone re-keys the
  lattices (D97); the epoch adds the quarantine and the window in which a
  reload's plan reaches the workers before its key reaches the database,
  where an analysis memoises the new plan's answers under the old key.
  Only the server builds a `TclDatabase`; `tcl-lsp-db`'s own tests keep 0
  unless they set it.

Taken while slice 5's opus items were executed (§ *Slice 5* › *Record
(2026-09-23): the opus items of slice 5* has the witnesses):

- **D105 — The targets an outcome may name are the semantics' own.** The
  plan's `validate_outcome(plan, targets, outcome)` takes the targets from
  the driver, and `CommandSemantics::store_targets` answers them: every
  operand the resolver gives the `VarWrite` role and a cell-update plan's
  target by default, the `stores -targets` operands for a pack's
  `DeclaredSemantics`, whose words need no role. A `Body` plan's
  `WriteBackKeys` operand is a target too.
- **D106 — A traced target declines the whole answer; an escaping one
  widens its own definition.** A trace runs on the write and can observe or
  rewrite the other targets, so a store to a place the module names in a
  trace — or to an element of an array it names, or to anything under a
  dynamic trace — declines `TracedPlace` and every definition widens. An
  escaping place is the solver's own rule (`is_externally_mutable`, the
  exact predicate, no base check), which already widens that definition
  before any transfer; the driver applies the same rule so the answer does
  not depend on who asks first. An element and its array's base in one
  outcome decline `OverlappingTargets`; two targets on one place compose in
  execution order.
- **D107 — A non-normal completion declines in slice 5.** An outcome whose
  completion is `Error` or `Code` publishes nothing and every definition
  widens; slice 10's prefix rule is what publishes an error path's stores.
- **D108 — One evaluation per statement, joined per definition.** The
  solver evaluated a statement once per definition and gave every
  definition the one value; it now evaluates once per sweep (`DefValues`:
  one value a typed statement computes, or a call's value per definition)
  and reads each definition's own value. Across the members of a lifted
  evaluation a definition joins with `Overdefined` absorbing and a pending
  member keeping it pending — a preserved prior the solver has not reached
  is never taken for another member's constant — else the union of the
  constants. `evaluate_def` and `evaluate_def_with_folds` answer for the
  first variable a multi-definition call names.
- **D109 — Storage overlap has one owner.** `PlaceRef::{is_element, base,
  shares_storage_with, overlaps_as_element_and_base}` answer whether two
  places share storage — the same name, or an array and one of its
  elements, by kind or by the `base(key)` spelling — and `declared.rs`'s
  private `overlaps` went.

Taken while landing the review of slice 4 (§ *Slice 4* › *Record
(2026-09-23): the review of slice 4* has the witnesses):

- **D110 — The generator is refused where stores are confined; the other
  math functions stay.** `rand()` reads and `srand()` writes a seed that
  is interpreter state, so a body that calls either answers differently
  on the same inputs — the store confinement's own reason (D77). Under an
  8.4-based release the math functions are `expr` builtins, not commands,
  so no command restriction can remove them: the VM's `expr` refuses the
  two while `confined_stores` is set (`Vm::confine_generator`, beside
  `confine_store`, one check in each function), under every release.
  From 8.5 each function is a `tcl::mathfunc::NAME` command, and the
  engine's `restrict_commands` had stripped all of them, so `expr
  {abs(-1)}` folded under an 8.4-pinned body and declined under every
  later one; an allowed `expr` now keeps every `tcl::mathfunc::*` command
  except `rand` and `srand`, which the VM would refuse anyway.
- **D111 — A plan host follows the published plan; a direct host is
  kept.** `ensure_host` runs the registered installer before it reads
  `HOST` whenever the thread's host came from a plan (`FROM_PLAN`), and
  the installer rebuilds only when the published generation differs from
  the thread's, so a pool thread that last built its host under a
  superseded plan answers with the new one on its next call. It used to
  run the installer only for a thread with no host at all, so such a
  thread computed through the old plan's bodies and memoised the answer
  under the new plan's key and epoch. A host installed directly
  (`install_host`: a test's host, or one a caller manages) is not the
  plan's to replace and is returned as it is. The witness drives the
  reload's own sequence — the overlaid registry, `hooks::publish`, the
  pack key on the salsa config — in `tcl-lsp-db`'s parity module, where
  every test that publishes a plan holds `PUBLISHED_PACKS`: the plan is
  process-wide, and a server unit test that published one would race
  every other server test that reloads packs in the same process.
- **D112 — A `-native` const-fold id names a shipped folder.**
  `const_fold -native ID` and `const_fold_versioned -native ID` resolve
  through `native_fold` against `CONST_FOLD_NATIVE` and
  `CONST_FOLD_VERSIONED_NATIVE`, at command and subcommand scope; a short
  or another scope's id is a notice naming the full spelling, an id the
  tables do not hold is a notice, and either leaves the family's
  abstention installed. The tables hold every folder the shipped registry
  installs (47 and 3) rather than a chosen few, because the renderer
  writes `-native SCOPE::FIELD` for each of them and a rendered spec must
  reload as itself (VT4.11's round trip); the folders they name became
  `pub(crate)`. The other twelve body families' tables stay empty — no
  shipped hook of those families is written in Rust under a name yet —
  and the page's § *`-native ID`* and the slice status say so.
- **D113 — The viewport reads the document's unit under the pack
  overlay.** `semantic_tokens_range`'s enriched tier resolves against
  `registry_for_dialect`, which carries the workspace's packs, and read
  the unit built without them (`document_compilation_unit`), so a pack
  command's writes were missing from the unit its viewport coloured by.
  It now reads `document_compilation_unit_for` under the config
  `resolved_db_config(uri)` resolves — the unit the full-document query
  and the diagnostics path already read, so a viewport after a
  diagnostics pass is a cache hit.
- **D114 — A declared budget's cap has one rule, in the host** (amends
  D91). The loader capped a `budget` row against `HostConfig::default()`
  and dropped a value above it with a notice, while the host caps against
  its actual configuration: two rules that disagree for any host
  configured otherwise. The loader now records the row as written, and
  the host's per-call `narrowed` is the only cap. D99's bound is corrected
  in place: 50,000,000 units stay near 200 ms only for the native routes;
  for a declared implementation they bound commands, about 500 calls at
  the host's per-call allowance of 100,000, each call's time bounded by
  the host's own 250 ms clock.

Taken while slice 5's opus items were executed, after the review of
slice 4 (§ *Slice 5* › *Record (2026-09-23): the opus items of slice 5*
has the witnesses):

- **D115 — A folded type is the settled evaluation's, beside the value.**
  `FoldedType { intrep, shape, representation }` lives in
  `value_transfer.rs` and `SccpResult::folded_types` keys it by
  `ValueKey`, so `lattice_rebase.rs` is untouched. The driver's answer
  for a definition carries it (`DefAnswer`): a whole-word substitution's
  result states its type facts' result type and the result value's
  representation; a place states, store by store in execution order, a
  write's stated type, shape and value representation, a may-write's
  bounds with no representation, nothing after an unbind, and a preserve
  keeps what an earlier store left and states nothing of the prior value;
  a copy (`set m $n`) shares its source's. SCCP records each definition's
  on every evaluation, so the settled sweep's — the one over the final
  inputs — is what stays, and a widened, joined or escaping definition
  records none. A φ keeps what every executable incoming value states
  alike (an unreached one skipped, a live-in root stating nothing), and a
  barrier, which widens every value, forgets every folded type. Members
  of a lifted evaluation, and a φ's arms, join by agreement
  (`FoldedType::join`): the design page's "last written type wins is not
  a sound join". A literal states nothing.
- **D116 — Purity reads representation before the literal rule.** The
  shimmer passes judged a numeric value pure when SCCP held a constant
  for it — right for a literal push, wrong for a value a command built:
  since slice 2 folded `[string length $s]` and `incr` in the shared
  lattice, `set n [string length $s]; lindex $n 0` and `set i 0; incr i;
  lindex $i 0` lost the S100 they had before, though tclsh 8.6, 9.0 and
  9.1 all show the value an `int` before the `lindex` and a `list` after.
  `is_pure_value` reads the folded representation first — constructed as
  a string is pure, constructed as anything else is committed to that
  intrep, no evidence falls back to `is_pure_intrep` — and the commit
  facts' initial state and the use-site and expression checks
  (`is_free_first_conversion`) read it, so a computed constant neither
  hides nor manufactures a conversion (the examples page's S100–S110
  rule, the migration table's S100 row). The expression route states no
  representation, so `[expr {1 + 2}]`'s constant keeps the literal-push
  reading it had. No existing test's expectation moved.
- **D117 — The static type stands where it knows; a folded type fills in
  where it does not.** `type_infer` takes a definition's folded type —
  its stated intrep, else the one the route constructed — only where its
  own typing is unknown or overdefined: a pack command with no declared
  return type (`[tenant::label acme]` is `String` from its `result
  -semantic string`), and from VT5.5 a destructured target. A known
  static type stands because it can carry element facts (`List<…>`, an
  object's class) a folded type does not; where the two disagree the
  shimmer passes still read the representation (D116), which is where a
  computed value's intrep matters.
- **D118 — S110's representation source is additive.** The byte-array
  walk keeps its registry classification and adds one source: a
  definition it left untracked whose folded type says the route
  constructed a byte array is binary from there (`track_constructed_bytes`).
  `binary format`'s route (VT5.6) has a byte-array return type as well,
  so its S110s are unchanged by folding; the new source is what a route or
  a pack evaluation without that return type reaches.
  `representation_plan.rs`, which the plan's file list named, reads no
  `TclType` and no lattice, so it needed nothing.

- **D119 — The engine answers three ways; the plumbing's type is the
  page's.** `tcl-regex` stays dependency-free, so the engine's own answer
  is `ExecOutcome { Matched(groups), NoMatch, Stopped(ExecStop) }` with
  `ExecStop::{Fuel { spent }, Depth { limit }, Cancelled}`, and
  `AreEngine` maps it onto the plumbing's `RegexpPrecision<RegMatch>` and
  `PrecisionDecline`, which are the page's verbatim with `V` the span type
  of the layer (`RegMatch`; the page's `V` appears in no field). A
  pattern's compile error and an unsupported form are the plumbing's to
  say, so they are `PrecisionDecline` variants the engine never produces.
  The limits are the engine's (`ExecLimits { fuel, cancel }`) and the
  plumbing's (`MatchLimits`, through `RegexEngine::exec_within`); the token
  is an `AtomicBool`, read in `spend_fuel_n` and the backtracker's
  `spend_fuel` — the page's one cancellation point.
- **D120 — Nothing is approximated.** Dissection walked a repeat's
  iterations by recursion and fell back to an approximate span past 256,
  so `(x)*` over 300 `x` reported group 1 at `256 299` where every tclsh
  reports `299 299`. It now walks iterations and a concatenation's items in
  loops — an unbounded repeat's "can the rest finish at `hi` from here"
  answered once per position by a backward pass — so its depth is the
  pattern's structural nesting; past the cap it stops (`Depth`). A subtree
  without a capture is not dissected at all. `AreEngine` therefore always
  answers `captures_exact: true`; `ApproximateCapture` stays for another
  provider, and the analysis path names the first participating group
  with it, since `Exact` carries only the flag.
- **D121 — Exactness needs bounds that do not grow with the subject.**
  Measured on the engine as it stood (`885e5ca1`), three of the page's
  witnesses were wrong: `^a*(b)\1$` over 300 `a` and `bb` recursed past the
  backtracker's cap and answered no match; `(x)*` over 300 `x` reported
  group 1 at `256 299`; and `^(a+)+b$` over 300 `a` spent its fuel
  re-expanding a frontier per repetition count, after which `reach_seq`
  returned its partial frontier and the search reported a *match*
  (`[0, 300)`, in 568 ms) where every release answers 0 — the page's
  "opposite" answer. An unbounded repeat past its minimum now reaches the
  closure of its frontier with a worklist that expands each position once,
  and the backtracker matches a single character's repeat (`m_run`, in the
  order `m_repeat` and `m_star` try lengths) and a run of literal
  characters in loops. All three witnesses are exact and agree with tclsh
  8.4.20 to 9.1b0, well inside the budget (the witness test runs in a
  quarter of a second).
- **D122 — A decline is raised at run time and kept typed in analysis.**
  The runtime entry points keep their signatures — `regexp`, `regsub`,
  `regsub_eval`, `switch::select`, `lsearch`, the VM's match helper — and
  raise a decline as `error while matching regular expression: <reason>`,
  C Tcl's `TclRegError` prefix, rather than answer 0, the unsubstituted
  text, no arm, or no element; the C API's `TclReExec` returns
  `REG_ESPACE`. `regexp_analysis` and `regsub_analysis` run the same
  algorithm (`regexp_run`, `regsub_run`) and return
  `RegexFailure::{Error, Declined}`; a `-command` substitution that is due
  declines `FormUnsupported`, as the analysis path cannot run a script.
- **D123 — The pattern cache is the analysis path's, in the plumbing.**
  `PatternCacheKey` is the page's verbatim, its engine field
  `RegexEngine::IDENTITY` (`AreEngine`'s revision 2, bumped by this
  change) and its target the analysis context's pair, so
  `StringCharacterModel` and `ByteStringEncoding` gained `Hash`. The
  thread-local cache holds at most `PATTERN_CACHE_BYTES` (4 MiB) of
  `RegexEngine::retained_bytes` (`AreEngine` sizes its tree), evicts the
  coldest entry first, and keeps no compilation larger than the bound. A
  miss is charged the pattern's length squared through
  `AnalysisMatch::charge` before it compiles; a refused charge is
  `Cancelled` and caches nothing. The runtime compiles fresh, as it did.
- **D124 — `-about` is evaluated; the callback form has no route.** The
  plan's item says `-about` and `regsub -command` are
  `NoRoute(FormUnsupported)` and its test list has `regexp -about {a}`
  decline, but its upstream note, written against the merged core, says
  VT5.4 evaluates `-about` through the core (the engine port records the
  `re_info` bits as C does) and declares `regsub -command`
  `NoRoute(Callback)`. Built as the note: `-about` answers `{count
  infoList}` through `regexp_analysis`, witnessed on 8.4 to 9.1 for `{a}`
  (`0 {}`), `{(?:a)}` (`0 REG_UNONPOSIX`) and `{a(b)c}` (`1 {}`); a
  `regsub` whose option run names `-command` — read with the spec's own
  table by `regsub_::names_a_callback`, which the role and prefix
  resolvers now share — declines `NoRoute(Callback)` whether or not a
  substitution would be due, under every target (under 8.x the call is a
  `bad switch` error, a decline either way).
- **D125 — The routes' axes, and the `-start` rule.** `NEEDS` is the
  plan's `REGEXP_FEATURES | CHAR_INDEXING | SOURCE_ENCODING` plus
  `LIST_RENDERING`: the core builds the `-inline`, `-indices` and `-about`
  answers with `ValueOps::new_list`, which requires the axis, and the
  answer depends on it (`regexp -inline {#(a)} #a` is `#a a` on 8.4 and
  `{#a} a` from 8.5, a witness). The core reads `-start` with the 9.x
  index grammar, but tclsh reads it as an integer on 8.4 (`010` is 8,
  `end` an error), as an index with octal numerals on 8.5 and 8.6, and as
  a decimal index from 9.0 (measured), so the route evaluates a `-start`
  value only when it is a plain decimal integer — `-` its only sign, no
  leading zero — and declines `Unsupported` otherwise; reading it under
  the target's own grammar is a later refinement. Every operand must be
  `admissible_text`, so a non-ASCII pattern or subject (and with it the
  engine's case-folding and class tables) is reached only under a target
  that decodes source as UTF-8, where the engine is the shipped runtime's.
- **D126 — The stores are the resolver's targets, checked against the
  core.** The stores name the operands the resolver gives `VarWrite`
  (`store_targets`' default); the core names the variables it writes
  (`RegexpResult::Count { assign }`, `RegsubResult::var`), and the route
  publishes only when they are those targets, in order, spelt alike —
  otherwise `Unsupported` (a word the resolver took for the pattern
  because its value was not literal: `set o -nocase; regexp $o A a m`). A
  completed no-match, an `-inline` or `-about` answer and a `regsub`
  without a variable publish a `Preserve` for every declared target, since
  the command writes none whatever the roles said; `regsub` with a
  variable writes it whether or not anything matched (`regsub z abc X v`
  leaves `abc`, measured). The type facts give each write the type its
  value was built as (`String`, or `List` with `-indices`) and the result
  `Int`, or `List` for `-inline` and `-about`. A malformed pattern and any
  other error the command raises decline `WrongRepresentation`, the
  `ConstOps` convention until the completion slice.
- **D127 — The engine's work is metered and charged.** VT5.3's engine
  reported what it spent only when it ran out. `Regex::exec_metered`
  returns the fuel a search spent (the backtracker's included), and
  `MatchLimits` gained `spent: Option<&Cell<u64>>`: with a counter, `fuel`
  bounds every search charged to it together, each running under what the
  counter leaves, so `-all` cannot spend the budget once per match. The
  route's allowance is the evaluation's remaining work
  (`ConstOps::remaining_work`, new), each search's fuel at most
  `MATCH_FUEL`; the compile's length-squared charge draws on the same
  counter through `AnalysisMatch::charge`; the counter is charged to the
  budget when the run returns, whose own limit is the decline when it
  overran. The published bytes are charged as work (one unit per capture
  or output byte, the page's rule) and as result bytes
  (`ConstOps::take_all`, new, for a result and its written values), and
  `regsub`'s output bound — the subject's bytes plus one, times twice the
  spec's plus one — is charged as allocation before the run. The
  cancellation token is not wired: `ConstOps` holds the budget mutably,
  and no request sets `Budget::cancelled` today; the engine reads a token
  whenever a caller passes one. The route's revision folds in the
  engine's (`1 << 32 | IDENTITY.revision`), so an engine change is a new
  route identity.
- **D128 — `regsub`'s shipped folders are the route.**
  `fold_regsub_versioned(args, version)` is `evaluate_literal(&REGSUB, …)`
  and `fold_regsub` its no-release form, as `string range`'s are;
  `const_fold_versioned` is new on the spec, and
  `CONST_FOLD_VERSIONED_NATIVE` and its catalogue gain the row (4 rows;
  the page says so). Every folded answer for ASCII words stands; two
  answers the old folder gave are now declines — a non-ASCII operand with
  no release named or under 8.x (`SOURCE_ENCODING`), and a `-start` index
  other than a plain decimal integer (the old folder read `-start 010` as
  10 under every release).
- **D129 — The analyser hook waits for CC2.13.** The plan retires
  `AnalyserHookId::RegexPatternCapture` and has
  `handle_regex_pattern_capture` read the declared targets; the variant,
  the handler and `tests/analyser_hooks.rs` are the consumer-contracts
  lane's files, and the coordinator's rule is to retire a hook only after
  CC2.13 lands. It has not: the hook, the handler and the ledger row are
  unchanged, and the retirement is recorded as this item's remainder.

- **D130 — An element write names its key.** `array set`'s places are
  elements no operand spells, and the page's four outcomes name a place
  by its operand alone, so `StoreOutcome` gains `WriteElement { target,
  key, value }`: the element `key` of the array the target names.
  `validate_outcome` allows one outcome per target and key, and the
  driver resolves the store to the element place (`element_place`), which
  declines for an element of an element; an element beside its base in
  one outcome still declines `OverlappingTargets`, and a traced base
  `TracedPlace`. The type facts are per target, so an element write's
  value carries its own representation evidence.
- **D131 — A stated element write is definite.** The SSA fans a
  whole-array writer's base definition over every constant-keyed element
  the function names, as a may-write whose value SCCP joins with the
  element's prior version. An evaluated outcome that names the element's
  place makes that write definite: `DefAnswer::stated` says so, and SCCP
  takes the value as it stands (and its folded type). An element no store
  names keeps the join and widens, as it did, and a typed dynamic-key
  write (`set arr($i) x`) still joins. Without it, `array set arr {k v}`
  evaluated and left `arr(k)` overdefined.
- **D132 — The destructuring routes answer where every release agrees,
  measured.** `scan`: no positional or size-modified conversion; `%b` from
  8.6; a float conversion from 8.5 (8.4 spells `%.12g`), never over a
  subject spelling an infinity (`scan -inf %f` is `-Inf` from 8.5 where
  the matcher converts nothing) or to a negative zero (`scan -0 %f` is
  `0.0` from 8.5, which reads the integer spelling as an integer); no
  `%u`, which the matcher reads signed (`scan -1 %u` is
  `18446744073709551615` on every release); no `0x` input to a radix
  conversion under 8.4; integers within 32 bits (`scan 2147483648 %d` is
  `-2147483648` on 8.4, 9.0 and 9.1 and `2147483648` on 8.5 and 8.6).
  `binary scan`: a field letter or the `u` suffix from its release, a
  float field from 8.5, no character above `U+00FF`, and no format with
  more value fields than variables — the command raises once it reaches
  such a field with data left and answers when the data runs out first
  (`binary scan \x01 ccc a b` is 1), which the route does not follow.
  `lassign` from 8.5, a variable-less call from 8.6. `array set`: one
  write per key, the last value of a repeated key, an odd list the
  program's error.
- **D133 — The oracle harness spells non-ASCII as `\uXXXX`.** `tclsh`
  reads a piped script in the system encoding, which is not UTF-8
  without a locale, so a witness holding `U+00F0` reached 9.0 as two
  characters and was compared with the wrong answer (the regexp witnesses
  reach no non-ASCII answer, and stay green). `tcl_quoted_word` escapes
  every character from `U+0080` to `U+FFFF`, which every release reads
  as the one character it names.
- **D134 — One publication for the writing routes.** `regex.rs`'s
  `Publication` (its opening over exact words, its per-byte charge, its
  take-all and its typing of each write) moves to the private
  `value_transfer/publication.rs` with `open_words`, `targets_are` and
  `PendingStore`, so the regexp and destructuring routes publish one way;
  the route revision is the caller's.

- **D135 — A computed byte array is never written into the source.**
  The interface page's rule — "a computed `binary format` needs a
  lossless materialisation contract before it is emitted anywhere" — is
  `SccpResult::materialises`: false for a definition whose folded type
  says the route constructed a byte array (a copy shares it, D115). O100's
  per-version and name-keyed projections, O103's return read, O127's skip
  of an SCCP constant and the chain folds read it, so `set h [binary
  format …]; puts $h` keeps the command (O127 forwards it, as before the
  route existed) and `set h …; return $h` folds no caller. The analyses —
  branch decisions, the diagnostics, the types — still read the value. A
  φ whose arms disagree on representation states none (D115), so a join
  of a byte array with an equal literal is emittable: its text is the
  literal's, already in the source.
- **D136 — `binary format` answers what every release packs alike,
  measured.** A field letter from its release (`t n m r R q Q` from 8.5);
  an integer only as a plain decimal within 64 bits (`c 010` is 8 up to
  8.6 and 10 from 9.0, `0b`/`0o` arrive in 8.5 and `1_0` in 9.0, past 64
  bits 8.x raises where 9.x wraps); a float only as a plain decimal whose
  value is zero or a normal double, within `FLT_MAX` for a
  single-precision field (8.x clamps, 9.x packs an infinity), and whose
  integer spelling is exact in a double and not a negative zero (8.5 on
  reads it as an integer: `d -0` is -0.0 on 8.4 and 0.0 after, `d 010`
  8.0 on 8.5 and 8.6); no argument character above `U+00FF`; `x*` and a
  countless `@` decline, which C Tcl refuses and the packer does not; a
  countless float field takes its whole argument, where the packer
  would take a list's first element. The `u` suffix and every other
  packer error decline as the program's error. `NEEDS` is the plan's
  `BINARY_FIELDS | BYTE_STRINGS` plus `SOURCE_ENCODING`, as for
  `binary scan`.

- **D137 — The body plans' binders are a projection; an unknown
  dictionary names none.** `DictWithSemantics` reads the dictionary
  variable's prior exact value — and walks a key path into the nested
  dictionary, the last value of a repeated key winning — and binds its
  distinct keys, first occurrence first, as declared scalar binders. A
  dictionary the analysis does not know exactly binds keys no plan can
  name, so the plan declines `NotExact` and the generic transfer stands; a
  path key the dictionary lacks, or a value that is no dictionary, is the
  command's error (`WrongRepresentation`). `DictUpdateSemantics` binds its
  declared variable operands whatever the dictionary holds (a key the
  dictionary lacks leaves its variable unbound on entry, which a binder
  cannot say and a consumer must not assume). Both write the bound keys
  back into the dictionary operand, run the body in the caller's frame,
  complete as the body does, and declare no route: the command's value is
  the body's. Operands count from the resolved form's `argument_offset`,
  so the subcommand and the `::tcl::dict::` spelling, which
  `qualified_specs` gives the same `semantics`, share one declaration.
  `dict with` is still a barrier in the CFG, so the solver reads neither
  plan; the consumers do (VT5.18).
- **D138 — The loops' source layout is one var-list and one list.** The
  page's `IterationPlan` holds one iterable, so several var-list and list
  pairs, which step several iterables in lockstep, decline `Unsupported`;
  a var-list the analysis does not know names no binders (`NotExact`);
  an empty or malformed var-list, or a call without its body, is the
  command's error (`WrongRepresentation`).
- **D139 — A loop header binds each binder the elements it takes.** Binder
  `i` of `n` takes the elements at `i`, `i + n`, …, the empty string where
  the last iteration runs past the list's end, and a repeated binder its
  last position's; an empty list or one the analysis cannot read leaves
  every binder `Overdefined`, as before. So `foreach {a b} {1 10 2 20}`
  gives `a` `{1 2}` and `b` `{10 20}`, two distinct finite inputs, and the
  mirror pairs decline `CorrelatedSets` — the reason D64 deferred.
- **D140 — The `DictWith` hook waits for CC2.13, as D129's did.** The
  plan retires `AnalyserHookId::DictWith` and has
  `handle_dict_with_command` bind the plan's binders; the variant, the
  handler and `tests/analyser_hooks.rs` are the consumer-contracts lane's
  files, CC2.13 (their re-baseline, after CC2.12) has not landed, and the
  coordinator's rule is to retire a hook only after it does. The
  analyser's binding over a literal dictionary is unchanged; the walk has
  no lattice, so "a `dict with` over a lattice-constant dictionary binds
  its keys" is delivered where the lattice is, the per-function
  harvesters (VT5.18).

- **D141 — The branch fact records its kind, and I230 reads it.** The
  solver's decided branches are `Applied`; the existence post-pass's
  `[info exists X]` / `[array exists X]` folds are `Proven` — proven
  conditions the executable blocks do not reflect; `Selected` has no
  producer until slice 6. Each emitter reads the kind it owns, so the
  existence I230 is the stored fact's and the analyser no longer reruns
  the proof over a frame of its own. That removes one drift the page
  names: for an iRules event, `FunctionUnit` drops a fold that queries a
  variable another event sets (`drop_cross_event_existence_folds`, the
  optimiser's rule), while the rerun still reported it — `when
  HTTP_REQUEST {set ans_cleared 1}` beside `when HTTP_RESPONSE {if {[info
  exists ans_cleared]} …}` drew "always false". That false I230 is gone,
  the slice's one observable change here (mandate: § *Branch facts*, "the
  stored fact says which of the three it is, and emission never reruns
  the proof"). `compiler_checks.rs`, on the plan's file list, already
  reported every stored fact whatever its kind and needed no edit.

- **D142 — A preserved definition names the version it holds.** A
  `Preserve` leaves the place as it was, value and existence alike, which
  the lattice value alone cannot say: `SccpResult::preserved` maps each
  such definition to the version before its statement — the block's
  latest earlier definition, else the block's entry version, else the
  root — rather than to the statement's recorded use, because only the
  `CONDITIONAL_VARIABLE_WRITE` commands record one, and a pack command's
  declared `write_or_preserve` carries no trait. A `<cond>` statement
  gets no outcome of its own; its definitions it reads first are
  preserved when the shared engine decided the branch it immediately
  precedes, since that answer ran every nested command it reached without
  a store (any other declines `StatefulNested`) — a `for` loop's static
  summary evaluates no command and does not count, and a definition the
  statement does not read (an `upvar` or global writer's) is left alone.
  The lattice value of a `<cond>` definition stays widened; the fact is
  existence's, and the optimiser's inputs do not move.
- **D143 — W210 reads a preserved definition through the general
  pass.** The prover's own sweep went with it: the undef trace resolves a
  preserved definition to the version it holds, so a read after a
  no-match — a statement, a `return` (which the prover never read), a
  condition's no-match arm — is read before set exactly when that version
  can be unset, and a `set` before the call silences it where the prover
  reported it. Two guards keep the fact to where it holds: a read in a
  block the outcome makes unreachable (the match arm of a `regexp` that
  never matches) is not counted, and the name-level condition-write
  suppression, which exists because a condition's writes had no facts,
  does not cover a version the solver proved no substitution wrote. W213
  follows from the same trace: `unset v` after a no-match is "may not
  exist", as tclsh raises.
- **D144 — A nested conditional writer's targets are read.** The SSA's
  rule for a `regexp`, `scan` or `binary scan` statement records each
  target's prior version as read (#2051), but the same commands in a
  word or a condition wrote their targets with no read, so `tcl opt`
  deleted the store a no-match preserves: `set v before; if {[regexp {(x)}
  $s -> v]} {…}; puts $v` and `set v before; puts [regexp {x} y v]; puts
  $v` optimised to programs that fail with `can't read "v"`, where tclsh
  8.4 to 9.1 print `before`. `variable_write_effects_from_commands` now
  lists a conditional writer's targets among the names it reads, from the
  registry's trait over the recovered words; that is also the use D142's
  `<cond>` fact reads. The hunk is in `ir_helpers.rs`, a file the plan
  does not name, and is the whole of this lane's change there.

- **D145 — A proven word is the lattice's, read with the caller's
  grammar.** `proven_word_value` takes the document's `LexerConfig`
  beside the plan's three arguments: a function unit keeps no grammar,
  and a literal run's backslashes decode by release. It re-runs nothing —
  a word holding a command substitution is `None` even where the
  substitution is pure — so a consumer reports only what the lattice
  proved at that statement; a value computed by a command reaches it
  through the variable the command's result was stored in (`set o
  [string tolower -ALL]; lsearch $o …`), which the lattice already
  evaluated under the module's trust. `StatementId` is the statement's
  block and index, which the CFG and SSA blocks share, and
  `FunctionUnit::word_at` finds it from a word's source range or its
  representative token's, for the consumers (VT5.16, VT5.17) that hold a
  span rather than a statement.

- **D146 — One pass runs the literal-only checks again over proven
  words.** The plan has each check record the spans it abstains on; the
  checks share one call's words, so the walk records the call once instead
  (one hunk in `commands.rs`, the dispatch the checks already hang from)
  and the pass substitutes every proven word at once — its text as a
  braced literal at the word's own span — then runs each check the plan
  names. A finding is kept only at a proven word's exact span, which is
  what "no finding twice, none at a span the user did not write" comes to
  for a check that reports where it reads; the index and relation checks
  report at a literal index or over a pair of options, so they are diffed
  against the same check over the written words instead. A kept finding
  carries no fix, which would replace the user's substitution with a
  constant. W146, W147 and W152 are verdicts the arity flush settles
  against a shadowing user command; the flush has run by then, so the
  pass settles them with the same rule, now one method. A word inside a
  nested command substitution is not reached: the CFG has no statement of
  its own for it.
- **D147 — The iRules checks read the lattice where they already are.**
  IRULE4004, IRULE3101 and IRULE3103 run per function over the unit, so
  they read its values directly rather than through the walk's sites.
  IRULE4004 hoists a value that reads no variable and that the lattice
  proves (`set x [string range CONST 0 3]` is the same on every request);
  a value reading a variable stays unhoisted, since hoisting it alone
  would move a read of a request-local. IRULE3101's setter check takes the
  unit's values as a parameter, which its four callers pass — one of them
  `tcl-lsp-core`'s `graphs.rs`, a file the plan does not name — so #2055's
  `set p /a; HTTP::path $p` is clean where the taint colour could not
  prove it. IRULE3103 reads any proven constant, where it read only a
  `Const(String)`, and a condition's variable operand, which it had not
  read at all.
- **D148 — Three rows the lattice does not reach yet.** W141's one
  producer, `return -errorstack`, lowers as a barrier, and a barrier
  widens every value the function holds, so no proven value reaches it;
  the pass runs the check, and a command whose option arity is a hook
  would draw it. A computed subcommand word (`string $sub …`) makes the
  function's dynamic-name barrier widen every value, so W145 is reached
  through option words only. `[string tolower CONST]` has no route in the
  shared lattice yet, so the plan's "propagated or `[string tolower
  CONST]` option value" is met for the propagated value; a route that
  lands makes the other follow with no change here.
- **D149 — A dictionary body is found by its plan over a dictionary that
  holds its key path.** `DictWithSemantics` declines over a dictionary the
  analysis does not know (D137), so its plan alone cannot say that a `dict
  with` over a parameter is one — and the W210 harvest must know, to keep
  its unknown-shape stance there. The consumer asks the structure question
  twice. The first asks over a probe dictionary, the least one holding,
  each inside the one before, the exact words the plan read on its way to
  its dictionary: that settles the plan's shape (the dictionary operand,
  the body) and binds nothing, for `dict with` under any key path and for
  `dict update`, whose plan reads no dictionary. The second asks over the
  dictionary the lattice or a same-block literal gives, and reads each
  declared key's value at the key path the plan read. A dictionary lacking
  the key path is the command's error, and the harvest keeps the
  unknown-shape stance for it. The spelling harvest read the outer
  dictionary's keys under a key path; the plan's are the nested
  dictionary's, as tclsh binds them. A `dict update` variable is bound when
  its key word is literal and the dictionary holds it: the key operand
  before each variable, which a binder cannot say (D137), is the one piece
  of the form's layout the consumer keeps.
- **D150 — W307's element writes are the statements' own.** The lattice
  holds `set arr(k) v` and `array set` element values in a function
  without a barrier, but a barrier widens every value its function holds
  (`dict with` is one), so a lattice-only reader loses the element writes
  the spelling harvest read in such a function. The flow-insensitive
  constant sets read each statement's own write: the lowering's element
  assignment for `set arr(k) v`, and each `WriteElement` outcome a registry
  route states over the call's literal words — run only for a call with a
  declared store target, under the evaluation budget. A lattice-constant
  `array set` operand harvests its elements from the lattice where the
  function has no barrier. A `dict set` reaches the constant sets only
  through a `dict with` that reads the dictionary, at the version the
  statement reads: in the same function the barrier widens it, and the
  interprocedural pass fills a parameter only from a literal argument word
  (`set d {}; dict set d cmd puts; s $d` leaves `s`'s dictionary
  overdefined), so the plan's "a lattice-constant `dict set` operand
  harvests its keys" holds where the lattice carries the value, and follows
  with no change here when the interprocedural pass carries a constant
  argument.
- **D151 — The template plan reads the switch table the command
  declares, per proven spelling and per release.** `TemplateSemantics`
  holds `subst`'s own options, families and trailing reservation — the
  static sits beside the spec in `subst_.rs`, the one hunk there besides
  the `semantics` field, because `commands::tcl` keeps its specs private
  from `value_transfer` — and runs `option_effects` over each combination
  of the switches' proven spellings at each release the profile names: its
  own release for a plain or vendor profile, every modelled release for a
  profile that declares none (the lenient `tcl`). A spelling that raises at
  a release — a switch the release lacks, an ambiguous prefix, the two
  families together, a word the switch run stops at before the template —
  contributes nothing, and the rest join, a kind on in any being on; the
  releases reading one spelling differently decline `ReleaseAmbiguous`
  with the gated option's row (9.1's), and every spelling raising is the
  command's error, `WrongRepresentation`. A switch the lattice does not
  prove, or more than 64 combinations, reads as every kind. The
  declaration has no route, so the driver's `[subst …]` fold still falls
  back to the registry engine exactly as before.
- **D152 — A template is decomposed only when the source spells it, and
  its spans are the word's.** A braced word, or a bare or quoted one the
  parser leaves literal, is decomposed under the kinds by the lexer's one
  word-parts owner; a word the parser substitutes is `dynamic` and names
  no read, region or escape, since its value is computed. Every span is an
  offset into the word as `word_structure` reports it — a braced word's
  content from 1 — and so is a region's `base_offset`; the record carries
  the template word's token span (from `{` to the content's end, the span
  W102 anchors at today), so a rebase shifts the record alone. An array
  index substitutes every kind whatever the switches say
  (`Tcl_ParseVarName` parses it with `TCL_SUBST_ALL`: `subst -nocommands
  {$a([set b])}` runs `set b` and reads `a(5)` on 8.4 to 9.1), so its reads
  and scripts are recorded before the element read they key, and an index
  the template computes is its source text in `VariableRead::element`. The
  driver records a plan for a trusted `Call` to a command that performs
  substitution, in an executable block, over the settled lattice; a
  `[subst …]` inside another command's word is no statement and has no
  record yet. A template holding a construct `subst` rejects (`subst
  {a[set b}` raises `missing close-bracket` on 8.4 to 9.1) is the
  command's error: the plan declines `WrongRepresentation` rather than
  describing the part `subst` substitutes before it raises.
- **D153 — The folders read the plan over the call's literal words.**
  Lowering runs before SSA, so both folders ask the declaration through
  `value_transfer::literal_template_plan` over `LiteralInputs`, which now
  answers a brace-quoted word's structure (`with_braced`); every word
  reads as its own spelling, so a computed switch is a spelling no
  release accepts and the call answers no plan a folder acts on — the
  shape refused exactly as before. `subst_nocommands` renders the plan's
  reads and escapes and refuses an element read, a qualified name, and
  any script region, so an array index's `[…]`, which `subst -nocommands`
  runs, is refused as the scanner refused the `(` alone. A question with
  no profile at all — a profile-less registry, whose own surface query is
  surface-blind — reads every switch with no release gate, as the
  registry's `substitutions_performed` does, so
  `detects_factory_shape_with_tcl91_positive_switches` still detects over
  `build_default`; a profile that names no release (the permissive `tcl`
  sink, `plain_tcl`, which `DialectProfile::find` does not list) still
  reads every modelled release (D151). The factory call's statement span
  becomes the child's `Procedure::span`; the W123 half of #2143 (Q3) is
  not assigned here, so the landing pins the span half.
- **D154 — W102 stays in the walk and the lattice refines it.** The plan
  moves W102 to the per-function pass, but the records are the CFG's own
  `subst` calls, and the walk reaches bodies the CFG has no statement for —
  an `after` callback, a `dict with` or `namespace eval` body, a
  `[subst …]` nested in another command's word — where W102 fires today.
  So the walk emits W102 from the plan over the call's source words (a
  substituted switch unproven, so it runs every kind and advises nothing,
  as the registry's unreadable-call answer did), and the per-function pass
  re-reads each recorded call over the lattice and replaces the walk's
  finding at the template word: `set opt -novariables; subst $opt $x` then
  reports what `subst -novariables $x` reports, and proven switches that
  turn both kinds off report nothing. The advice needs the switch
  spellings, which the plan does not carry, so `TemplatePlanRecord` gains
  the command as spelled and each switch's proven spelling (`None` when one
  is unproven); the page's record shape is `{ span, plan }`. The walk, the
  barrier, extract-proc and VT5.9's folders ask one helper,
  `literal_template_plan`, which now takes each word's `SourceWord`
  (braced, literal, substituted) — `LiteralInputs` gains `with_structure`
  and `with_unproven` — so a substituted word is unproven everywhere rather
  than read as its own spelling.
- **D155 — A computed template blinds reads while it can run a command.**
  The mandate asks that `subst -novariables $t` stop blinding every read,
  and the plan says the barrier sets `reads` only for `dynamic &&
  kinds.variables`. That is unsound while command substitution runs: `proc
  f {t} {set x 1; return [subst -novariables $t]}; f {[set x]}` prints `1`
  on tclsh 8.4 to 9.1, and with `reads` clear the optimiser removed `set x
  1` as unused (O126), so the rewritten program raises. The barrier
  therefore sets `reads` when variable *or* command substitution runs over
  a computed template: `subst -nocommands -novariables $t`, which read a
  name no more than a literal does and blinded the function before, is
  what stops blinding; `subst -novariables $t` keeps it. A computed
  template's commands can write too (`[set x 2]`); the barrier never
  modelled that, and the lattice keeps `$x` unfolded after such a call on
  its own. Each script region of a braced template is scanned as script in
  the frame, which the generic bracket scan over every argument already
  did. Under this rule the plan's barrier test name,
  `a_novariables_subst_of_a_dynamic_template_does_not_blind_reads`, would
  state the opposite of what the test asserts, so the test is
  `a_computed_template_blinds_reads_while_it_substitutes`.

- **D156 — `NoRouteReason` gains `Platform`.** VT5.14's item text names
  `PLATFORM` as the route-decline reason for the `file` forms
  (`file stat`, `file lstat`, `file tempfile`), but the tree's
  `NoRouteReason` (`value_transfer/decline.rs`) had only `Declared`,
  `Unauthored`, `FormUnsupported`, and `Callback` — `PLATFORM` existed
  only as `Needs::PLATFORM` and `Axis::Platform`, a release-axis payload
  answering a different question (which axis a route cannot read, not why
  a command has no route at all). The item adds the variant it names
  rather than reusing one of the four that answers something else: a
  `stat` call or a temporary file's name is the host platform's to decide,
  never derivable from the source, which none of `Declared` (an author's
  abstention), `Unauthored` (unwritten), `FormUnsupported` (a form the
  route does not model) or `Callback` (a command prefix runs later) says.
  `MayWriteSemantics` (VT5.14, `value_transfer/builtins.rs`) declares it
  for the three `file` forms; `gets`, `chan gets`, `vwait`, and
  `tk_optionMenu` keep `Declared`, matching the item text's "the channel,
  event-loop and widget forms"; `trace add`/`remove`/`variable`/`vdelete`
  take `Callback` — the item text describes them separately
  ("declare `TransferAnswer::Generic` with no route") because they
  install a `commandPrefix` the same way `regsub -command` does (VT5.4,
  `NoRouteReason::Callback`), not because they need a second semantics
  type: `MayWriteSemantics` does not override `transfer`, so its inherited
  default is already `TransferAnswer::Generic`, and reusing the one type
  for all eleven `MayWrite`-shaped commands avoids a second, behaviourally
  identical struct (`store_targets`'s override, read only by an evaluated
  outcome's validation, is inert for every `EvalRoute::None` command
  either way — `call_defs` (`tcl-compiler/src/value_transfer.rs`) returns
  `widened(defs)` before it is ever read).
  *Amended by the slice 5 review (S1):* the item's "whose transfer is
  `MayWrite` on each declared target" holds now. `MayWriteSemantics`
  carries `kind: Option<BindingKind>` and overrides `transfer`: under
  `FactDomain::Existence`, one normal-completion path with a
  `MayBind(kind)` per `store_targets` target, as `unbind.rs` answers
  `Unbind`; every other domain stays `Generic`. The kinds: `Array` for
  `file stat` and `file lstat`; `Scalar` for `file tempfile`'s name
  variable, `gets`, `chan gets` and `tk_optionMenu`; `Either` for the four
  `trace` forms. Two depart from the review's note. `file tempfile`'s
  variable is a scalar, not an array: `set f [file tempfile p]; list
  [info exists p] [array exists p]` is `1 0` on 8.6 to 9.1 (8.4 and 8.5
  lack the subcommand). `vwait` declares no kind and keeps the generic
  transfer: its wait ends on an unset as on a write (`Tcl_VwaitObjCmd`
  traces `TCL_TRACE_UNSETS`; `set x 1; after 0 {unset x}; vwait x; info
  exists x` is 0 on 8.4 to 9.1), and a may-bind never loses a binding, so
  D159's widening to `MayBound` is its answer.

- **D157 — The special-variable faces are the registry's.** The
  consumer-contracts lane's hand-over: the existence rung's entry state,
  the taint seed and the existence post-pass read
  `CommandRegistry::special_vars_for_dialect`, `special_var_in_dialect`
  and the new `is_initially_bound` (VT8.1), and W210's startup read and
  W211 / W220's externally-read suppression read `is_readable_at_startup`,
  the new `is_lazily_readable` and `is_externally_read` (VT8.4) — each
  new face the shipped table less any name a pack row answers for, beside
  `is_readable_at_startup` in `registry.rs` — so a pack-declared special
  variable is honoured wherever a shipped one is. The free functions in
  `special_vars.rs` stay for the consumers outside this lane.
- **D158 — Existence is a forward fact per place, reported three ways.**
  The plan's `SccpResult::existence: HashMap<ValueKey, Existence>` alone
  cannot hold "after a barrier or a computed name, every place is
  `MayBound` from that statement on": neither statement defines a new
  version of the places it clobbers, so one version is read on both
  sides of it. The solver keeps a state per place and per point — the
  join over executable edges, advanced by storage outcomes and clobbers,
  held at the driver's cursor while a block is evaluated, so the routes
  read it through `prior_store(place, FactDomain::Existence)` — and
  reports it per version (`existence`, the plan's map: each definition's
  fact where it is established), per statement read (`existence_reads`)
  and per block exit (`existence_exits`, what a terminator reads). The
  per-version map is what S100 and the phi-arm consumers read; the other
  two are what a read at a point reads. `values` is untouched: a run with
  and without the rung computes the same values but for the cell updates
  the release rule decides.
- **D159 — The generic existence transfer widens.** A command with no
  evaluated outcome and no existence transfer of its own may bind or
  destroy its targets — `array unset` has neither until VT8.8 — so each
  of its definitions takes `MayBound`, as its value widens; a may-bind
  (`Join(Bound(_))`) would have claimed `array set a {k v}; array unset
  a; info exists a` is 1, where every release prints 0. An evaluated
  outcome's stores state per-place steps (a place no store names widens
  too), and a declaration's own `transfer(FactDomain::Existence)` on the
  normal completion states them for a declined or route-less evaluation
  (`set`, the cell updates, `unset`, the keyed updates).
- **D160 — The entry rules as built.** Three depart from the page's
  wording. A scope-alias local is `MayBound` from the entry, not from its
  declaration: the escaping set the values already read holds it
  flow-insensitively, and `namespace upvar` records no definition to hang
  the declaration's step on. A special variable enters bound as its
  registry kind — `env` and `tcl_platform` are arrays, and the page's
  `Bound(Scalar)` would answer 0 to `array exists env`. An iRules `when`
  handler enters every name any handler of the module binds `MayBound`,
  a superset of `ConnectionScope::cross_event_defs`: a handler's own
  names persist across its firings on one connection (the per-connection
  counter `if {![info exists n]} {set n 0}; incr n` must never fold), and
  the set rides on `AnalysisContextKey::connection_scoped`, so a
  memoised handler re-keys when another handler binds a new name; a
  computed-name write in any handler makes every name `MayBound` in all
  of them. `drop_cross_event_existence_folds` keeps filtering the
  post-pass until VT8.2 deletes both.
- **D161 — An exception edge carries every point of its handler's
  region.** The CFG builder sources a `try` or `catch` handler's edges at
  the block before the body, its tail and its explicit throws, but any
  command in the body may raise, so the state at the handler is the join
  over every point of the region between those sources on normal edges —
  each region block's points are kept for it. The per-edge exit alone
  would have read `try {set y 1; foo; unset y} on error {} {…}`'s
  handler as `y` unbound.
- **D162 — G1 scans past a test-only item, and the ratchet is measured
  from that scan** (the slice 5 review, item 6). The lint stopped at the
  first `#[cfg(test)]` line, so a test-only helper above a file's tests
  module hid the rest of the file — twenty scanned files had one. Only an
  inline test module ends the scan now; any other annotated item — a
  `mod name;` declaration, a `use`, a braced item — is skipped alone. The
  wider scan found sites in two files the ratchet held at zero,
  `analyser/commands.rs` (4) and `taint.rs` (3), and nowhere else;
  `RATCHET` and the ledger pin them, and "only lowered" is measured from
  this baseline. `commands.rs`'s four are the `set VAR [CLASS new]`
  instance tracking VT8.9 retires; `taint.rs`'s three — the `file`
  path-sink narrowing, the `string` guard parse and the `interp` / `proc`
  rebinding order — are registry axes (`side_effects`, `traits`) no slice
  8 item moves, so they are pinned rather than fixed here. The lint also
  sees a tuple-pattern `match` arm naming a literal under a subject that
  binds a head (S3), so `var_command.rs`'s three reviewed arms carry
  waivers.
- **D163 — `scan`'s conversion count is C's `nconversions`.** The
  shared core counted only non-suppressed, non-`%n` successes, and the
  underflow (`-1`, or the inline form's empty result) fires when the
  input runs out with that count at zero. `tclScan.c` increments
  `nconversions` in the `%n` arm whether or not it is suppressed, and
  after every successful conversion, suppressed or not, so `scan {}
  %n%d n a` is 1 and `scan 5 %*d%d a` is 0 on every release where the
  core answered -1 for both. The review named `%n`; the suppressed
  success is the same increment and the same wrong -1, so both are
  fixed in `scan_match` and the variable-mode count stays the
  conversions assigned. The route (`destructure.rs`) and the VM
  (`cmd_format.rs`) read the core's count and needed no edit.
- **D164 — A substituted word re-resolves the roles over its proven
  value** (the slice 5 review, S2). `view_of` runs a command's
  `arg_role_resolver` only over all-literal words and otherwise keeps the
  static roles, which for `regexp`, `regsub`, `scan`, `binary scan`,
  `lassign` and `array set` name no targets, so a route over a
  substituted subject declined `Unsupported` and a no-match lost its
  preserve. The driver's inputs re-run the resolver over the words' exact
  lattice texts when every word is exact, and take its roles when every
  operand they make a `VarWrite` is a literal word: the command itself
  reads the values, and a computed name is still no place.
- **D165 — An existence query decides from the rung, with the post-pass's
  two held abstentions.** `info exists` and `array exists` read the
  place's fact where they run (VT8.2), and the rung's entry rules and
  clobbers stand in for the post-pass's hand-kept exclusions. Two of that
  post-pass's abstentions are not rung facts and are kept at the query.
  In the initial global frame a registry special variable enters bound as
  its kind (D160), which the absent-cell rule and W210 read, but the host
  rather than the script binds it — a script sourced by an embedding host
  may not see `argv` — so a query about one decides nothing, as
  `i230_existence_fold_abstains_on_interpreter_globals_at_top_level`
  requires. An element query reads its array and decides only that an
  unbound array holds no element: a scalar parameter's element is 0 in
  every release, but the post-pass abstained on any touched or parameter
  base and `info_exists_element_fold_abstains_when_the_array_is_touched`
  pins that, so the query keeps it; a computed key leaves a bareword
  array fixed, so `info exists Params($k)` reads `Params` as the
  post-pass did. A name the function only asks about has no SSA symbol,
  so the run gives it a slot past the symbols; its version map is
  untouched.
- **D166 — The guard refines every place the solver tracks.** The guard
  refines every place the solver tracks, on both edges, with no exclusion
  list. The soundness argument for an externally mutable place is the
  barrier reset, not an exclusion: a TclOO instance variable, an iRules
  cross-event name and an interpreter-set special variable can only
  change without a visible statement across a barrier or an up-frame, and
  VT8.1 already turns every place `MayBound` there, so a refinement never
  survives the point at which another actor could act. Excluding those
  places would make W210 report the canonical `if {[info exists
  ::errorInfo]} {puts $::errorInfo}` and the TclOO `if {[info exists x]}
  {return $x}` idioms once slice 11 removes the old
  `collect_existence_guards` callers, which is the wrong trade.
- **D167 — An unbind reads its place's existence wherever it runs, and a
  nested one kills nothing yet.** The registry's read projection
  (`CommandRegistry::variable_read_projection`) names a
  `DESTROYS_VARIABLE` command's targets beside the `VarRead` words it
  already named, so the one consumer of that projection
  (`ir_helpers::variable_read_effects_from_commands`) records a nested
  `[unset x]` as a read of the version it observes in every position the
  lowering materialises — a condition's `<cond>` statement, a value
  word's or a `return` word's `<upvar-invalidate>`, a host call's own
  reads — and code sinking refuses to move a store past one. That is
  #2220's condition case and the statement rule in one implementation.
  The nested unbind gets no kill definition: a definition on the
  synthetic statement, which the lowering places before its host, would
  make the host word's own `$x` read the killed version, so the idiom
  `set y $x[unset x]` would draw W210 and read no value. The kill stays
  unmodelled (§ *Record (2026-09-24): the opus items of slice 8*, found
  and left). W210 treats the nested destroyer's target as the existence
  word it is (`existence_query_vars`, through
  `ir_helpers::destroyed_variables`), as it treats `[info exists x]`, so
  `puts [unset -nocomplain x]` of a never-set `x` draws nothing, as
  before.
- **D168 — `array unset` is a conditional write.** Every release leaves a
  scalar, an absent variable and every element the pattern misses in
  place (VT8.8's finding: `set x 1; array unset x; set x` is 1 on tclsh
  8.4.20 to 9.1b0), so the definition the SSA gives `array unset x` does
  not kill the one before it. `Traits::CONDITIONAL_VARIABLE_WRITE` states
  exactly that — the trait's own words: "the definition it appears to
  kill is not dead" — so the subcommand carries it rather than a new
  trait, and the SSA's statement rule and its nested twin
  (`ir_helpers::conditionally_writes`) read the prior version. The
  existence rung keeps reading VT8.8's `ArrayUnsetSemantics`, which no
  trait touches.
- **D169 — The hidden-read scan matches what the SSA records by span.**
  `collect_rmw_hidden_reads` drops a name the SSA records as read where
  the word runs: for a statement, its own uses and those of the synthetic
  statements the lowering pushed ahead of it for its words'
  substitutions; for a `return` word, those of the synthetic statements
  ahead of the terminator. Both kinds carry the host's span, which is
  what identifies them, so `return [info exists x]` and `return [incr n]`
  leave the scan along with every statement-level existence read; a
  braced `expr` body inside an `incr` amount, which nothing records,
  stays.
- **D170 — The typed existence read lives on the function unit.** The
  unit is what knows the tier its lattices ran at, so
  `FunctionUnit::existence` is the one typed read and `FunctionUnit::tier`
  its source: a request below the deep tier is a context key at that tier,
  whose build runs no rung, and a guarded unit's lattices are trivial.
  `Pending` answers a point the run never reached — a dead arm's read,
  which is the lattice's bottom, not an absence of analysis — and
  `Unavailable(tier)` answers only what the tier or the ceiling withheld.
  S100's producer takes the `SccpResult` alone, so it keeps the
  per-version map, where a missing entry is never `Unbound`: an arm the
  run did not type is not skipped as unbound, the conservative direction
  (R7). The optimiser's re-runs, which pass no rung, still answer an
  existence query `Unavailable(Deep)` from the driver — a deep run that
  computed none — and nothing reads their existence as a fact.
- **D171 — `set`'s binding reads the registry's `CellWrite` declaration.**
  The item moves the two-word form's definition onto the generic role
  binding, the constant-string environment onto the value word's
  `CellWrite` evaluation, and the `interp create` value binding onto the
  generic binding. One analyser method, `bind_value_word_assignment`,
  runs in the dispatch tail after the role binding for every invocation
  whose resolved semantics is the direct one-target write of a value word
  (`ResolvedSemantics::writes_value_word`: the route `Direct { CellWrite
  }`), so no consumer names `set`. It escalates the role binding's
  `warn_if_unused` to the assignment's `true`; for a one-token literal
  value word it evaluates the declaration over the call's literal words
  with the resolver's roles (`LiteralInputs`) and records the `Write`
  store's value — the word cooked as Tcl reads it, where the hook kept a
  bare or quoted token's raw text; otherwise it keeps the hook's two other
  arms on the value word (a created interpreter's key, a folded `[cmd]`)
  or clears both. The evaluation is the declaration's own, as the lattice
  driver runs it: a pack command declaring `evaluate -direct CellWrite`
  gets the assignment but no constant, since a pack's named route
  evaluates nothing yet. The one-word read form was already the walk's
  `VarRead`-role reference pass's. `set auto_path …`'s record rides on the
  same predicate, in the full analyser and in the signature scan. A
  computed name (`set $n 1`) is no longer defined: the role binding's
  `names_static_variable` rule, where the hook defined the name its `$`
  stripped away.
- **D172 — `set VAR [CLASS new]` reads the handle-binding layout.** The
  four sites that recognised the instance-creation shape by `cmd_name ==
  "set"` read `CommandRegistry::handle_binding`'s `ConstructionValue`
  layout — `set`'s own `SET_BINDS_HANDLE`, which the registry documents as
  exactly this value flow — for the bound variable, the value word and
  its position, so a rooted `::set` binds as `set` does. The ratchet
  ledger's row said the tracking "reads the value word's evaluation
  instead"; the layout is the registry's statement of which word's
  construction the variable receives, and the class still comes from the
  construction parsed out of that word, as before.

### Open questions for the owner

Each with the assumption the plan proceeds on.

- **Q1 — The three nested budgets.** The evaluation page lands them in
  slice 2; the hand-off deferred them. *Assumption*: slice 4 (D3).
- **Q2 — `command_substitution_is_none` "flips".** Its program, `[clock
  seconds] + 1`, reads the wall clock and can never fold. *Assumption*:
  slice 9 renames it `command_substitution_evaluates_through_the_nested_service`
  over `[incr x] + 1` and keeps the clock program as its negative case.
- **Q3 — #2143's W123 half.** The analyser's W123 pass runs over the
  source walk, which never sees `specialise_factories`' children; the
  template-word plan gives the child its span, not its registration.
  *Assumption*: slice 5 fixes the span half and its landing says "Pins the
  span half of #2143"; if the owner assigns the W123 half here, VT5.9
  grows a bridge carrying the unit's materialised procedures to the
  walk's W123 pass, and the landing says "Closes #2143".
- **Q4 — `ActivationStore`.** The page's mechanism cannot write the
  calling frame through `HostCommand`. *Assumption*:
  `Engine::confine_stores` (D10), an engine capability the page did not
  consider; the page's § *Per-evaluation state* is amended in VT4.15.
- **Q5 — The `Set` hook's `interp create` binding.** It is the
  interpreter-domain axis, not the value axis. *Assumption*: it moves with
  the hook in VT8.9, reached from the generic binding and keyed by the
  value word's resolved `interp create`.
- **Q6 — The shared lattice's stance on an unbounded binding transition.**
  Upstream's `ObservedBindings` keeps folding when one unresolved head
  raises the `dynamic` top, while the interface page's decline table reads
  "binding validity: … renamed, aliased, redefined, or in an opaque
  namespace". *Assumption*: the plan adopts upstream's two stances (D21),
  because every rewrite re-proves under `WholeModule`.
- **Q7 — Reclassified gap rows.** `const` to slice 8 and `lset`, `ledit`,
  `lpop` to slice 7 (D4, D5). *Assumption*: accepted.
- **Q8 — The regexp runtime delta.** *Assumption*: accepted (D18).
- **Q9 — #2214 in this lane.** *Assumption*: VT2.9 fixes it unless a
  `rust` pull request closes it first.
- **Q10 — Slice 5's size.** Twenty items in one slice. *Assumption*: one
  slice, six checkpoints, one landing, as the plan defines it.
- **Q11 — The diagnostic-policy lane's three questions to the
  `tcl-compiler` owner** (the file-directive fold, W305's self-filter, a
  single-bucket `line_suppressed`). *Assumption*: they are not this lane's;
  it neither answers nor blocks them.
- **Q12 — The evaluator generation in salsa** (D98). The per-procedure
  lattice key carries the building worker's generation, but the unit query
  above it is memoised on its inputs alone. *Assumption*: the server sets
  a salsa input for the generation when a worker's host changes, in a
  later slice or the consumer-contracts lane; until then a transient
  decline served to a healthy worker is imprecise, never wrong.
  **Answered on 2026-09-23 (D104)**, in this slice once the
  diagnostic-policy lane had freed `rust/tcl-lsp-server`: the evaluator
  epoch is a salsa input the server sets where it reloads packs and after
  each diagnostics pass.
- **Q13 — An exhausted request re-declines what it paid for** (D100).
  *Assumption*: the evaluation page's rule stands as built — a pass gets a
  tenth of what is left, and a re-evaluated statement the pass cannot pay
  for declines — and charging a pass only for work an earlier pass did not
  already pay for is the refinement if the degradation shows up in
  practice.
  **Accepted on 2026-09-23**, as built and with no change: sound, since a
  re-decline publishes `Overdefined` and never a stale constant. The
  precision it costs is per function: once one run's evaluations spend
  its request (`Budget::request()`, 50,000,000 units; D99 says what they
  bound on each route), every route-evaluated definition a later
  sweep re-evaluates declines with `declined: budget: Request` and ends
  `Overdefined`, the ones earlier sweeps folded included, so the function
  keeps none of its route folds rather than those made before the point
  of exhaustion (D100's measurement: all twelve `string length` folds of
  one procedure under a 500-unit request). What reads those constants — a
  constant condition (I230), a forwarded value (O100) — sees none for
  them. Only a function whose route evaluations together cost more than
  the request pays it; the refinement above stays the answer if that
  shows up in practice.

### Deltas flagged for the owner

- **F1 — Folding under a profile that names no release.** The hand-off's
  per-operation admission makes `incr x 5`, `append`, `lappend`, `string
  range` and every later direct route fold under every vendor dialect
  (iRules included) where each modelled release agrees, while the
  evaluation page's `admit` step 2 reads "a bit whose field is `None` is a
  decline". The page's per-axis rules support the reading; its literal
  step does not. CC9.2 later gives iRules a measured base (B-CC8).
  **Closed — the owner ruled on 2026-09-22:** "What's the most accurate
  thing to do with folding, that's the right answer." Unanimity decides: a
  route folds where its answer is proven identical under every release the
  profile can denote, any per-axis disagreement declines with
  `ReleaseAmbiguous(Axis)`, and a dialect that declares a base release
  (iRules on its 8.4-derived engine) evaluates under it, a vendor pack that
  diverges on an axis blocking the fold by declaring the axis. The
  per-operation admission stands as built; `value-evaluation.md`'s `admit`
  steps 1 and 2 and § *Target semantics* now say so, and
  `value-transfers.md` § *Rulings* 7 and 8 record it. The base-release half
  is built by VT4.1 (D72): `TargetSemantics::of` reads the profile's
  declared runtime (`DialectProfile::runtime_version`), so iRules evaluates
  under 8.4, and `TclVersion::from_profile` (CC9.2's) is untouched.
- **F2 — The budget deferral** (D3, Q1): a scope move, not a behaviour.
- **F3 — The inventory committed from a failing gate run** at the
  checkpoint: a process deviation, corrected by VT2.0.
- **F4 — The `ObservedBindings` stance** (D21, Q6): the shared lattice
  folds under an unbounded binding transition that the rewrite stance
  refuses.

No other delta in this plan lacks a mandate; each slice's table cites
its own.

### Item count and model split

| Slice | Items | opus | sonnet |
|---|---|---|---|
| 2 | 15 (VT2.0 landed, VT2.M, VT2.1–VT2.13) | 9 | 6 |
| 3 | 12 | 8 | 4 |
| 4 | 15 | 10 | 5 |
| 5 | 20 | 15 | 5 |
| 8 | 11 | 9 | 2 |
| 6 | 9 | 6 | 3 |
| 9 | 6 | 3 | 3 |
| 10 | 9 | 7 | 2 |
| 11 | 5 | 3 | 2 |
| 12 | 7 | 5 | 2 |
| 7a | 2 | 1 | 1 |
| 13 | 8 | 5 | 3 |
| 7 | 10 | 6 | 4 |
| **Total** | **129** | **87** | **42** |
