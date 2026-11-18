# dataflow-optimiser: pause handover, 18 Nov 2026

**Area:** `rust/tcl-compiler`. This covers:
- lowering, CFG, SSA and SCCP;
- `tcl_expr_eval` and the optimiser;
- interprocedural and `compilation_unit`;
- GVN and the inliners;
- the integration tests for all of the above.

**Ledger:** `docs/work/test-scrub/ledger/dataflow-optimiser.tsv`.

## (a) Landed today (18 Nov), on `work`

| Commit | What |
| --- | --- |
| f9b9a80521 | Ledger gains `status`/`note` columns. |
| dcf5110be9 | A `string length` fold reads a non-ASCII literal only where the release reads source as UTF-8 (`StringCharacterModel::count_authored`). |
| d8402cd92f | Load forwarding counts a callback that writes a computed name (`TraceInputs::writes_any_variable_in`). |
| cd4fed81cd | Ledger: rows passing on `work`. |
| 88e66f2577 | An 8.4 constant fold declines a shift count outside 0..63 (`fold_ops.rs` only; the VM keeps `wide::shift`). |
| 3d0fbf74ed, 20751748d6, d53a399242, ade0ad2cfb, 3324ba2d75, 642c8f128e, d7b59fb57c, 2b6746b6bb | Pass and unit tests build their `PassContext` through `manager::build_pass_context` and a real release profile (rows cause `pass-context-unwired` / `dialect-blind-fixture`). |
| 6c0d92a6c8, 1eef5e110d | O112 tests: a sole `switch`/`for` is the script's result and is kept. |
| 72eb66a1b9 | An `if`'s else-body span runs through its closing brace. |
| 019cdef1a7, 44b7fd5dcd | A separate-word `switch` arm's body span runs through its closing brace; 44b7 corrects the 9.1 witness claim in 019c's message. |
| 21aabe1d68 | Ledger: the unknown-head O103 row is blocked on analyser-binding. |
| 9e1a47005a | Ledger statuses after the declared-frame collector. |
| ff48bc25ab | **Wrong rewrite fixed.** Under tcl8.4, `proc f {args} {return $args}; puts [f # a]` was rewritten to `puts {{#} a}`. Three `args` rest binders rendered with `join_list`: `command_binding/source_walk/called_bodies.rs` and `command_binding/formal_topology.rs` (now sharing `ResolveContext::render_rest_list`), and `optimiser/propagation/proc_folds.rs`. They now use `TargetSemantics::render_list`. `release_optimiser_safety::a_leading_hash_list_prints_as_the_release_does` holds it; the group is 69/69. |
| e28cff8dcb | Unset-variable fixtures: inputs are now assigned from `argv`, proc parameters or integer loop counters. 8 rows repaired, 8 exposed (see (c)). |

The NativeValueRefusal patch (typed native-gate refusal) went in through the values session as **57021886**, together with their driver side and their re-read of the pack-write row.

Earlier, before 18 Nov (see the ledger notes):
- 4696120a51: purity of positioned calls.
- 48aa8f83b4: the generic-entry ruling; the parameter seed producer is removed.

## (b) In flight

1. **`split ""` wrong rewrite.** I hold no record of it. Nothing in this session's messages routed a `split ""` rewrite to me, and I have made no diagnosis or patch. The successor should ask the coordinator for the reproducer and the diagnosis.
2. **`lifecycle-fixture-proc-not-installed`** (12 lib rows in `cfg_builder::execution_regions::tests`).
   - Patch: `patches/dataflow-optimiser-lifecycle-fixture.patch`. It is a one-hunk test-helper change; apply it with `git apply` from the repo root.
   - What it does: `trusted_lifecycle_body` used `fixture_id`, which panics before the helper's own declaration fallback runs. The patch switches it to `fixture_id_if_present`.
   - With the patch, all 12 rows fail one step later: `original procedure declaration retained` (`.expect` at line ~930).
   - Probe at e28cff8dcb (a temporary test, not kept):
     - `Lowerer::new` or `Lowerer::with_config` with `captured_native_entry` gives `procedures=0` even for `proc p {} {set x 1}; p` (`procedure_implementation_bodies=1`, `original_declaration_body_units=0`).
     - With `package require tcltest` first, the result is `impl=0` too.
     - `native_compilation` and the lexer config make no difference.
     - `captured_native_entry_with_owner` with `static_context_for("tcl8.6")` also gives `procedures=0`.
   - Remaining: find the lowering route that installs `::p` through this ingress, or take the body from `procedure_implementation_bodies` for the plain case. The tcltest case needs the stock provider to keep the declaration. Do not weaken the region assertions.

## (c) Queue, in order

The order follows the coordinator's standing rule: exposures first, then FIX-FIXTURE/FIX-EXPECTATION by cause size, then REWRITE/MERGE (RETIRE/MERGE in their own commits), then KEEP causes by rows.

1. **Exposures found today** (product declines that the old fixtures hid; each row's note names its cause):
   - `o128-hint-only-unknown-list`, 2 it_3 rows plus the `end_offset` lib tests. `end_offset.rs` reports O128 `hint_only` unless `closed_list_end_offset` proves a known valid list. This is likely deliberate, so these rows may need a re-read to the closed case rather than a product fix.
   - `o115-condition-unwrap-declines`, it_3 and it_5. The `branch_folding` O115 gate declines `if {[expr {$x + 1}]}` even for an integer loop counter.
   - `o120-sccp-const-operand-declines`, it_3 and it_5. `set a foo; if {$a == $b}` in a proc gives no O120; a literal operand still does.
   - `rename-join-both-arms-unknown`: the same `rename` on both arms joins to Unknown, not Opaque.
   - `switch-glob-not-retained-opaque` (lib `ssa::tests::effect_markers`): no `Statement::Switch` is retained once the subject is set.
2. **Older exposures:**
   - O114 `prospective_increment` declines;
   - no selection record for separate-word `switch $x`;
   - `-glob` over an appended value;
   - empty connection scope under f5-irules;
   - caller-store O109 missing in a called proc;
   - CFG proc-exit tests to be rewritten onto lowered source;
   - loop-header-may-return;
   - group-mate resurrection not exempt.
3. **FIX-FIXTURE by size:**
   - `lifecycle-fixture-proc-not-installed` (12, in flight);
   - `unbounded-self-recursion-fixture` (11);
   - `dialect-blind-fixture` (11 exposed);
   - `reads-unset-variable` (6);
   - `uncalled-body-withheld` (13);
   - `fixture-reads-unset-variable`;
   - `jim-entry-gained-name-policy` (4);
   - `trailing-statement-result-observed` (3).
4. **Rows queued from values:**
   - `ssa-nested-write-phantom-use`: `puts [set x 2]` records a use of x v1, so `set x 1` stays live.
   - `optimiser-nested-write-forwarding`: `puts $x` is not forwarded after `[incr x]` inside an expr.
   - `lifted-call-body-not-original` (2);
   - `o101-non-finite-render`;
   - `optimiser-module-top-level-blocks-fold`;
   - `dead-store-unreachable-use`;
   - `optimiser-handler-forwarding`;
   - `ssa-var-symbol-lookup-refused`.

   I told values that `unit-single-caller-param-seed` is the generic-entry ruling and that `optimiser-chain-fold-ordering` is intended ordering; they are re-reading both rows to the observable.
5. Then REWRITE/MERGE/RETIRE, then the KEEP causes ranked by rows: `uncalled-body-generic-entry` 31, `no-proc-transfer-summary` 15, `unknown-head-world-ends` 13, `method-body-not-entered` 13, `direct-proc-plan-declines-authored-calls` 13.

## (d) Waiting on / waited on

**Waiting on analyser-binding:**
- the O103 unknown-head wrong answer (`[answer]` → 42 in an uncalled `f` after `unknown_future_entry`); they ruled "don't gate O103";
- `proc_call_inside_expr_argument` (`return 7` from the walk retaining the caller's operands).

Both rows are `blocked`.

**Others waiting on me:**
- the values-queued rows in (c) item 4;
- the live-inlining 7e1d4fba review;
- the diagnostics W210 guard narrowing.

I have no record of a request to review taint's advisory solver mode. The successor should confirm the request and its scope with the coordinator.

**Reported to the coordinator, owner unknown:** two command_binding lib rows fail only in the parallel run and pass alone, with or without my change:
- `source_callable_static_tests::static_formals_update_one_retained_slot_across_actual_calls`
- `source_command_world::tests::original_opaque_jim_callable_name_does_not_gate_ascii_static_capture`

## (e) Rulings not yet carried out

- The coordinator's standing order of work (see (c)) is in progress.
- The "list every list-rendering site" part of the leading-hash order is done; the sites are named in the ff48bc25ab message.
- No other unexecuted ruling is held.

## (f) Last measured counts

**Ledger** (`python3 docs/work/test-scrub/tools/ledger_status.py docs/work/test-scrub/ledger`, at the handover commit): dataflow-optimiser has open 323, repaired 39, exposed 40, fixed 18, deleted 0, blocked 2, routed 2, out of 424 rows.

**Last full-suite baseline:** 1896 failing in tcl-compiler + tcl-registry, taken around 08:00 on 18 Nov (before 9e1a47005a), with
```
cargo nextest run -p tcl-compiler -p tcl-registry --no-fail-fast
```
run under the oracle env:
```
TCL_LSP_JIMSH=/home/user/tcl-lsp/tmp/jimtcl-oracle/jimsh TCL_LSP_REQUIRE_ALL_TCL_ORACLES=1 TCL_LSP_REQUIRE_JIM_ORACLE=1 TCL_LSP_REQUIRE_TCLSH=1 CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_PROFILE_TEST_DEBUG=0
```
The codegen residual was 15 failing. A fresh baseline at ff48bc25ab died when the disk filled, so no newer full count exists.

**Targeted runs at e28cff8dcb:**
- it_2 `value_transfer_witnesses`: 111 failing, against 112 before ff48bc25ab.
- `release_optimiser_safety` (tcl-codegen it_3): 69/69.

## (g) What a successor would get wrong

- **Generic-entry ruling:** a procedure body is never seeded by its callers; call-site evidence is advice only. Lattice rows expecting a caller-seeded parameter are re-reads, not bugs.
- **Unknown-head ruling:** an unknown command ends the premise.
- **Test contexts:** build them through `manager::build_pass_context` and a real release profile (`static_context_for("tcl8.6")`, `f5-irules` for iRules), never a bare `PassContext`.
- **Spans:** segmenter token spans stop before a braced word's closing brace. Body spans must go through `tcl_lexer::word_span_at` / `script_word_span`.
- **Disk:** it fills. `rm -rf target/debug/incremental` and use `CARGO_INCREMENTAL=0`.
- **Concurrent edits:** never edit sources or stash while a cargo build is compiling.
- **Formatting:** `cargo fmt -p` reformats others' files; use `rustfmt --edition 2024 <file>`.
- **List rendering:** a value printed by a program must render lists through `TargetSemantics::render_list`, because a leading `#` differs between 8.4 and 8.5+. `tcl_syntax::list::join_list` is release-agnostic.
- **`opt_codes`/`optimised` helpers:** a fixture that reads an unset variable at file level makes the optimiser fold nothing after it, correctly. Bind inputs from `argv` (`lassign $argv ...`, 8.5+) or put them in a proc.
