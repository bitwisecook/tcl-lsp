# taint: pause handover, 18 Nov 2026

Session: session_017u5g3HGxE6CxLNW1DgfVr9. Branch `work`. Tree base at pause: `9839f8212`. Ledger: `docs/work/test-scrub/ledger/taint.tsv` (and `taint-passing-wrong.tsv`, 3 rows).

## (a) Landed (17–18 Nov)

| Commit | What |
| --- | --- |
| `bb93c4f54` | Tk widget getters are instance taint sources again (tk-fixture-without-tk cause: `leading = 0` for Instance publication, `instance_source_colour`, unseen-call marker skipped in instance classes, `it_3/taint/object_instance_sources.rs` runs under `tk`) |
| `5e0f10ab0` | IRULE5001: a flag set in a setup event is read from the handler under the document input |
| `0ac07f9fc` | IRULE5001: an event body the index cannot see opens the debug-flag index |
| `68952f29c` | Analyser: the body of an event handler the rule loader rejects is not walked (IRULE3102 wrong answer) |
| `050bb134d` | BIG-IP check 074 (a `-`-leading `switch` subject without `--`) |
| `0d71c9a2e`, `099f65afb` | value-transfer witnesses: top-level calls leave f5-irules; new `it_2/value_transfer_witnesses/rule_procedures.rs` (9 witnesses, 1 passes, 8 wait on rule-procedure advice) |
| `290e8b135`, `af2c64c9c`, `3f360ed5d`, `9839f8212` | ledger status commits |
| 17 Nov | `0c5048ed6` T102/T103 handler option and pattern positions, `cfdb5da68`, `9370b8fa4`, `d16f051e0`, `0db7cd310`, `a2eba15f4`, `dd16696a7`, ledger parts 1–4 |

## (b) In flight

All patches are in `patches/`, made against `9839f8212`. Apply with `git apply docs/work/name-handling/handover/pause-2026-11-18/patches/<file>` and run `git apply --check` first.

1. **irules-checks-dialect-blind-unit (34 rows), diagnosis only.**
   - **Patch:** `taint-irules-checks-census-probe.patch`. It holds eprintln probes and a `zz_probe_source_flow` test, not a fix. Do not push it.
   - **Finding 1, the registry fixture:** switching `irules_checks/tests.rs::registry()` to `static_context_for("f5-irules").commands()` changed nothing: 81 pass / 36 fail before and after. Also, `source_unit` already builds through the f5 environment, so the ledger's cause label is suspect.
   - **Finding 2, opaque handler-body builtins:** in a `when` body, `catch`, `switch`, `while`, `foreach`, `eval` and `after` lower as an opaque `Statement::Call` with no `child_scripts()`. The decline chain:
     1. `IrulesSourceQuery::retain_pairing_handler_commands` (`irules_checks/source_query.rs`) takes them down the leaf branch.
     2. `leaf_handler_source_completion` declines on their script arguments.
     3. So `conditional_pairing_advice` is `None` and every flow check reports nothing.

     The walker (`flow_walk.rs`, `evaluated_body` and structured statements) has the same dependency.
   - **Finding 3:** `HTTP::header Cache-Control no-cache` in `irule1201_http_after_respond_fires` also declines, because the subcommand is not in the registry. Its expectation likely needs a real form (`HTTP::header insert ...`); check with BIG-IP before changing it.
   - **Where this points:** analyser-binding's ranked causes 6 and 9 (a builtin not proved at its site; F5 handler bodies). The fix is either to lower structured builtins in handler bodies as advice, or to make the census and walker read hosted body structure from the registry. I traced it only for the IRULE1201/1202/5002/5004/1007 flow tests. The generic_static (IRULE4002), IRULE4004, IRULE1005 and original_event_owner rows are not traced.
2. **Rule-procedure advice (11 rows blocked), the coordinator's ruling (A).**
   - **Patch:** `taint-rule-procedure-advice.patch`. It is a WIP with probes.
   - **Contents:**
     - `advisory_execution_target()` in `execution_proofs.rs`;
     - `advisory_tokens_invocation_with_metadata_context` in `registry_invocation.rs`;
     - `advisory_normal_transfer_invocation_with_metadata_context` in `normal_transfer.rs`;
     - an advisory flag threaded through `evaluate_def_with_math` and `fold_normal_store_value` in `sccp/original_value_facts`.
   - **Remaining:**
     - strip the probes;
     - meet the 6 conditions in (e);
     - get values to review;
     - tell dataflow-optimiser.
   - The 3 legal-rule witnesses' f5 arm reads strict `value_at`, so it needs a FIX-EXPECTATION to read semantic advice.
3. **Tk callback replay (14 Tk rows plus audit's 6 lib tests).**
   - **Patch:** `taint-tk-callback-replay.patch`. It applies cleanly.
     - (a) `callback_replay_unit` uses `cu.ir_module.dialect_profile.or(dialect)`;
     - (b) `is_actual_formal_entry` accepts `VariableCellKey::Authored(_)` when `declared_formal`.
   - **Blocked on:** registry completion contracts for `after` (analyser-binding) and the Tk widget constructors `entry`/`label` (tk-oo). Until those land, the Unknown completion route withdraws the after-load world.
   - **Once they land:** apply the patch, re-run the 14 rows and audit's 6 tests, tell audit before pushing, and name the unblocking commits in the ledger.
   - `taint-tk-entry-after-probes.patch` holds the diagnosis probes (`first_possible_invocation`, `SourceBindingTimeline::entry_after`) and is not for push.
4. **Older scratchpad patches, kept for reference.**
   - `taint-held-hosted-t102-t103.patch` (stale): superseded by `0c5048ed6`.
   - `taint-action-tier.patch` (stale): a `ContextActionTier` sketch in `compiler_checks.rs`. It was never ruled on. Drop it unless lsp-features wants it.
   - `taint-trace-read-references-probe.patch` and `taint-quickfix-probe.patch`: debugging probes only.

## (c) Queue, in order

1. Remaining wrong answers: the duplicate eval-sink warning, then IRULE3101 on a proven `/` path. Wrong answers come first per the sign-off order.
2. irules-checks-dialect-blind-unit (34). Re-label the cause from (b)1 and route it to analyser-binding if the lowering is theirs. Trace the untraced rows first. These are the biggest fixture cause.
3. profileless-registry-fixture (11). This is the next big fixture cause.
4. Rule-procedure advice under ruling (A) (11 blocked rows, 8 witnesses).
5. Tk callback replay once its contracts land.
6. f5-xc switch lowering in hosted event bodies (irules-registry-tooling's 10 tests). First check whether `f6d942f3` already changes the `tcl explore` output.
7. Remaining exposed causes, taken by size.

## (d) Waiting on, and waited on by

- **Waiting on analyser-binding:** the `after` completion contract (Tk replay), and the handler-body structured lowering, if they own it ((b)1).
- **Waiting on tk-oo:** widget constructor completion contracts. The `.caption configure` false positive is routed to them.
- **Waiting on values:** review of the rule-procedure advice change. Do not edit `values.tsv`; tell values which rows change.
- **audit is waiting on me:** the 6 callback_replay lib tests after the Tk contracts land.
- **irules-registry-tooling is waiting on me:** the f5-xc switch lowering question (queue item 6).
- **dataflow-optimiser:** must be told when rule-procedure advice lands.

## (e) Rulings not yet carried out

Coordinator ruling, 18 Nov 11:08: **(A)**, a single engine reading the registry data. (B) would write every transfer twice. The current WIP patch threads a runtime boolean, which the ruling does not accept as is.

1. **The resolution mode lives in the types, not in a boolean.**
   - The engine is parameterised by its resolution purpose.
   - The advisory instantiation yields a different result type from the strict one, so no expression that stores an advisory value into `sccp.values` compiles.
   - If the code shape makes that impossible, report the cost to the coordinator before falling back to a runtime flag.
2. **Prototype first:** one transfer (`incr`) and one witness (`a_rule_procedure_folds_its_cell_updates_as_advice`). Measure, then send the coordinator (a)–(c) and a half-page on the shape:
   - (a) that witness is green;
   - (b) a full tcl-compiler run with the oracle flags, from a clean `origin/work` baseline, shows 0 new failures;
   - (c) a byte-for-byte differential of the strict `sccp.values`, dumped for every function over the test corpus and the samples, before and after the change.
3. **Scope:** the second run applies only to hosted-advice bodies (rule procedures, handlers). Plain Tcl gets no second run. Count solver runs (not wall-clock) on the iRules samples before and after and report the ratio. If it more than doubles, report that.
4. **Ownership:**
   - taint writes the change.
   - dataflow-optimiser owns `sccp/solver` and must approve the diff before push; being informed is not enough.
   - class-facts is told, since the mode threads through `registry_invocation`.
   - values reviews the witness expectations.
5. **Earlier conditions still stand:**
   - a negative test on rewrites under f5-irules (`release_optimiser_safety`);
   - registry-data transfers only;
   - formals Bound only;
   - BIG-IP checks cited for 8.4 behaviour;
   - the 11 rows turn `fixed` only as they pass;
   - `blocked` stays the status until the prototype is measured.

Also received before the pause:

- **audit (11:23), on the Tk replay:** the second `entry .two` goes missing because `entry .one -validatecommand {…}` trips `requires_reentry_guard` (`command_binding/source_walk/completion_receipts.rs:299`). Its `source_callback_retention` clause withholds the command world after any registration that retains a callback. Controls without a callback resolve `.two`. Real Tk 9.0.3 does not run `-validatecommand` at registration.
  - Proposed split, awaiting a ruling: the registry marks deferred-only surfaces (audit), and the walk guard honours that (binding owner).
  - Until the guard changes, the batch test and the batches-past-the-first-batch test stay short a candidate, even with `taint-tk-callback-replay.patch` and the completion contracts.
- **coordinator (11:04), on disk:** run `rm -rf /tmp/tcl-spectcl-cache-test-*` when the disk fills, before deleting `target/`. `cargo fmt` and clippy must finish before a push. Quote fixed counts only against a clean `origin/work` baseline.

## (f) Last measured counts

- **`irules_checks::` lib tests at `9839f8212`:** 81 pass / 36 fail, with and without the fixture switch. The probe build showed 82/36, the extra pass being the probe test. Command:
  ```
  cd rust && CARGO_INCREMENTAL=0 TCL_LSP_JIMSH=/home/user/tcl-lsp/tmp/jimtcl-oracle/jimsh \
    TCL_LSP_REQUIRE_ALL_TCL_ORACLES=1 TCL_LSP_REQUIRE_JIM_ORACLE=1 TCL_LSP_REQUIRE_TCLSH=1 \
    cargo test -p tcl-compiler --lib irules_checks::
  ```
- **Ledger `taint.tsv` (185 rows):** 132 open, 5 repaired, 26 exposed, 10 fixed, 11 blocked, 1 routed. Check with `python3 docs/work/test-scrub/tools/ledger_status.py docs/work/test-scrub/ledger --check`.

## (g) Things a successor would get wrong

- **The irules_checks cause label is wrong.** Changing `registry()` in `irules_checks/tests.rs` is not the fix (measured). The 34 rows' failures are mostly empty results from declined advice, not from a wrong registry.
- **Handler bodies are advice, never entered.** Hosted facts never feed `sccp.values` or rewrites, and a rule proc has no proved execution target at load. Any fix that makes a handler body "proved" breaks the handler-store ruling.
- **`TCL_CORE_DIALECTS` has 4 entries**, not 5 (f5 removed). Do not zip it against 5-column expectation tables.
- **Disk fills fast.** Use `CARGO_INCREMENTAL=0` and prune `target/debug/incremental`.
- **Push checks:** run clippy `-D warnings` on touched crates and `make xtask-gates` (39/39) before any push touching compiler, codegen, syntax, lexer, registry or xtask.
- **A broad `git checkout -- rust/` is refused** by the session safety check. Restore named files with `git restore <file>`.
