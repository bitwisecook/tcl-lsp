# values: pause handoff, 18 November 2026

Session `values` (claude.ai/code session_017u5g3KpPQPTbtTswiHYkaw). Area: value-transfer
witnesses, `value_transfer`, `var_escape` (IR walk and CFG propagation), shimmer, `type_infer`,
`dynamic_names`, `cfg_builder::global_write_info`/`upvar_info`/`unit_scope`, and the
`tcl-codegen` `compiler_analysis_residual` tests. Ledger: `docs/work/test-scrub/ledger/values.tsv`
(220 rows), summary `docs/work/test-scrub/values.md`.

Nothing is in flight. The tree was clean at the pause: no uncommitted change, no parked patch, so
there is no `patches/values-*.patch`.

## (a) Landed today (on `work`, newest first; ids as rebased)

Product fixes:

- `57021886e` A source-gate refusal in value position carries its reason (`NativeValueRefusal`,
  dataflow-optimiser's gate half plus my driver half, ruling (a)): `stateful-nested` / `not-exact`
  instead of `unsupported`.
- `c7011ddb0` var_escape: a call the registry lowers as `eval` runs its joined words as a script
  (soundness: `eval set l2 $x` had recorded no dynamic barrier).
- `b79801f5a` value_transfer: a rebound head declines as `rebinding-suspected` before the source
  gate.
- `1e1e63b2b` A compiled literal `string length` carries the literal's representation
  (registry `NativeScalarOperation::compiled_constant`, consumed in
  `value_transfer/compiled_constants.rs`): no false S100 in compiled bodies 8.4 to 9.0.

Test corrections (each names its ruling or tclsh evidence in the message):
`14695691c` (chain fold, seeded call), `a76bfe7ff` (regexp inventory), `d135ff996` (try handler
fixture), `177a08b42` (expr_acceptance_list columns), `d1c09658f` (rooted `::set`),
`cf726dc31` (opaque catch fixture), `4202bbaa1` (clippy fix in taint's rule_procedures.rs).
Earlier today, before the shallow history I can list: the shimmer witnesses
`a_compiled_literal_length_is_a_pure_string_before_9_1` and the pack-write re-read.

Ledger commits: `267839830`, `6a729cd7b`, `3a06caf55`, `fd8fd7a62`, `72926acbc`, `6a5cd393e`, and
earlier ones the same day.

## (b) In flight

Nothing. One attempt was reverted and is NOT needed: a module-input check in
`OriginalDiagnosticValues::for_module_function`. The availability row it targeted fails on the
lowering (see `lowering-ignores-availability`, routed to class-facts), not on that function.

## (c) Queue, in order

The 6 rows still `open` with values, smallest risk first, then the passes the coordinator asked for:

1. `var_escape_cfg::cu_dynamic_set_name_resolved_to_literal_records_that_name_under_the_guard`:
   `set target real; set $target 99` should tag `real` at its SSA version through
   `CfgState::literal_assigns` (`var_escape/cfg_propagation/state.rs`); `ssa_tags` comes back
   empty. Mine (cfg_propagation walker).
2. `var_escape_cfg::cu_interprocedural_off_keeps_raw_per_proc_result`: `setit` should be
   `unbounded_upvar_source()` with interprocedural off. Mine; not yet diagnosed.
3. `cfg_builder::global_write_info::tests::custom_replacement_under_builtin_spelling_uses_its_own_descriptor`
   and `…original_global_alias_writes_keep_empty_local_and_remote_names`: mine; not yet
   re-measured today.
4. `type_infer::scope_alias_metadata_tests::native_jim_alias_metadata_cannot_use_raw_logical_name_eligibility`
   (KEEP-CONTRACT, panics before its assertion): mine; not yet diagnosed.
5. `lsp_shimmer_conditional_source::a_computed_length_is_an_integer_in_a_procedure` (new witness,
   `shimmer-computed-length-unrepresented`): `[string length $s]` builds an int in every release
   but the route declines a non-exact operand, so no representation is stated. Needs the route to
   state its result type/representation even when the value is unknown.
6. Re-run routed rows as their owners land (list in (d)), setting them `fixed` or back to `open`.
7. The coordinator's second pass: exact-copy and overlap clusters for `value_transfer`,
   `shimmer`, `type_infer`, `var_escape_cfg` from `docs/work/test-scrub/data/clusters_*.tsv`, into a
   `values-passing.tsv` with MERGE / KEEP / VACUOUS per test read against source, sent to the
   coordinator for sampling. Not started.

Order: own failing rows first (wrong answers before missed precision), then re-runs, then the
passing-test pass, as the coordinator ordered at 06:53.

## (d) Waiting on, and waited on by

Waiting on (each row's status names the owner; the evidence cell has the fixture):

- analyser-binding (122 rows): the lenient `tcl` profile binding (`lenient-tcl-logical-source-records`,
  about 18 rows), the walk widening after a raising catch body, caller-frame-opaque barriers,
  `binding-substitution-word-no-layout`, `unseen-call-marker-withdraws-every-procedure`, the
  declared-frame switch (`binding-binary-in-procedure-unproved`, `tenant::label` in a procedure
  condition), `embedded-write-projection-context-free` (queued at their position 9),
  `uncalled-body-generic-entry` (try handler), and a ruling on `binding-unavailable-command-unseen`
  (is an 8.4-unavailable `lassign` at the top level an unseen call because `unknown` may
  auto_load?).
- dataflow-optimiser (32): `wrong-rewrite-split-empty` (WRONG REWRITE, O129 `[split ""]` →
  `{{}}`), `o129-const-fold-past-declined-route`, `ssa-declared-may-write` (false W220),
  `ssa-nested-write-phantom-use`, `optimiser-nested-write-forwarding`, `o101-non-finite-render`,
  `lifted-call-body-not-original`, `optimiser-handler-forwarding`, `dead-store-unreachable-use`,
  `optimiser-module-top-level-blocks-fold`, `ssa-var-symbol-lookup-refused`.
- dataflow issue session (8): `summary-link-local-def`; they asked me not to change
  `lattice_inputs.rs` `place()` for the `unset` through `upvar 1 $name v` fact until their summary
  SSA answers `var_symbol("v")` for a link local.
- class-facts (5): `switch-name-roles-residual` (they confirmed it is theirs for 8.5+),
  `lowering-ignores-availability`, the object-element rows.
- taint (7): the f5 rule-procedure advice gap (completion test, registry entry-point escape rows,
  three rows they hold).
- diagnostics (1): `wrong-i230-body-role-stub` (WRONG DIAGNOSTIC: I230 always true after a
  body-role stub call where tclsh prints `other`).
- analysis-performance (3): `rebase-memoised-receipts`.

Others waiting on me: none that I know of. I have no record of a request to review taint's
advisory solver mode; the coordinator's pause message names it, so it may have reached a
predecessor or been sent while I read other queues. A successor should ask taint for the commit
and the question before assuming it.

## (e) Coordinator rulings not yet carried out

- The second pass over passing tests (item 7 of the queue).
- "Re-run the rows you routed when the owner lands": standing; nothing has landed since
  `ff48bc25ab` (args leading hash, marked fixed).

## (f) Last measured counts

All with the oracle environment (`TCL_LSP_JIMSH`, `TCL_LSP_REQUIRE_ALL_TCL_ORACLES=1`,
`TCL_LSP_REQUIRE_JIM_ORACLE=1`, `TCL_LSP_REQUIRE_TCLSH=1`; `bash scripts/dev/ensure-jim-oracle.sh`
first), `CARGO_INCREMENTAL=0`:

- At `57021886e` (09:35): `cargo nextest run -p tcl-compiler --test it_2 --no-fail-fast -E
  'test(/^value_transfer_witnesses::/)'`: 189 run, 78 passed, 111 failed.
  `cargo nextest run -p tcl-compiler --lib --no-fail-fast -E
  'test(/^(interprocedural|value_transfer|sccp)::/)'`: 374 run, 289 passed, 85 failed.
- At `c7011ddb0` (10:26): `cargo nextest run -p tcl-compiler --no-fail-fast --test it_2 --test
  it_5 -E 'test(/var_escape/)'`: 145 run, 128 passed, 17 failed; `--lib -E
  'test(/var_escape|inline/)'`: 288 run, 256 passed, 32 failed.
- Ledger at the pause (`ledger_status.py --check docs/work/test-scrub/ledger` passes): 220 rows,
  6 open, 23 exposed, 178 routed, 11 fixed, 1 repaired, 1 deleted.

## (g) What a successor would get wrong

- The ledger is generated. `values.tsv` and the summary paragraph of `values.md` are written by
  `values-ledger-tools/` in this directory: edit the `rows_*.tsv` (path, verdict, cause,
  evidence) and `status.tsv` (path, status; later lines win), then run
  `python3 build_ledger.py rows_gwi.tsv rows_esc.tsv rows_ti_vt.tsv rows_sh.tsv rows_res.tsv rows_wit.tsv`
  and `python3 sync_md.py` from that directory. Hand edits to `values.tsv` are overwritten; other
  sessions (taint did) sometimes edit it directly: fold their changes into `status.tsv` first.
- Status vocabulary (`docs/work/test-scrub/tools/ledger_status.py`): `repaired` means corrected
  and passing; a corrected test that still fails is `exposed`. `routed (owner)` is accepted (first
  word counts).
- Many failures that look like value-transfer faults are the walk: a `<caller-frame-opaque>`
  barrier or `<unseen-call>` after a command makes every variable overdefined. Probe the SSA
  statements before touching the driver (`TCL_LSP_TRACE_N17_FOLD=first-gate-v1` shows the native
  source gate's declines; `TCL_LSP_TRACE_VAR_ESCAPE_BODY_OWNERS=1` the escape walker's first
  pessimism).
- Explanations are keyed by statement span and the last pass wins; a missing explanation usually
  means `run_command` returned `None` (no original ownership, or the source gate refused).
- A top-level program is not an iRule: witnesses that call at the top level run
  `TCL_CORE_DIALECTS`, and expectation arrays must match that list's length (one slip found).
- Push only through the gated script: fetch at depth 200 (depth 50 stranded a rebase after a
  history rewrite), `cargo check` and `cargo clippy -p tcl-registry -p tcl-compiler -p
  tcl-codegen --all-targets -- -D warnings`, then `make xtask-gates`.
- A stash-based before/after comparison must not overlap a running background build; wait for it
  to finish before editing.
