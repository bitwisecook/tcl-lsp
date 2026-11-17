# Test scrub: analysis performance and incremental analysis

Ledger: [`ledger/analysis-performance.tsv`](ledger/analysis-performance.tsv), 46 failing tests: the per-item and incremental tests in `tcl-compiler` (library `per_item`, `per_item_corpus`, the per-item agreement tests in `it_4`, the `phi_undef_memo` stage budgets, the `depth_guard` stack budget) and the `tcl-lsp-db` memo and memory tests that wait on per-body incremental analysis. Failing status taken on `work` at `59d31ef6` (compiler) and from the 17 November list (`tcl-lsp-db`).

## Verdicts

| Verdict | Tests |
| --- | ---: |
| `KEEP-CONTRACT` | 41 |
| `REWRITE` | 4 |
| `UNDECIDED` | 1 |

## Causes, by tests blocked

| Cause | Tests | Behaviour |
| --- | ---: | --- |
| `per-item-source-ownership-gate` | 22 | Every document with a procedure re-analyses the whole file on each edit, because `fill_deferred_bodies` falls back (`OriginalSourceOwnership`) for any body carrying naming receipts. Fix in progress: `patches/handover-analysis-performance-body-wrapper-realm.patch`. |
| `per-item-result-differs-from-full` | 6 | The incremental result is not the whole-file result for the same text. |
| `lattice-memo-keys-carry-document-receipts` | 6 | A procedure's memoised lattice is re-keyed by any edit, because its key holds whole-document naming receipts. |
| `per-item-drops-diagnostic` | 4 | The incremental path loses a diagnostic the whole-file walk issues (W240, W129). |
| `phi-budget-absolute-seconds` | 2 | Absolute wall-clock budget; see below. |
| `computed-head-qualified-literally` | 1 | The whole-file walk records `::${ns}::setdef` as a resolution candidate for `${ns}::setdef x y`: a wrong answer, analyser-binding's. |
| `full-walk-misses-factory-class` | 1 | The whole-file analysis misses the factory-made class (class-facts area). |
| `per-item-instance-replay` | 1 | No W308 on a named object created in a deferred body (#1312). |
| `per-item-jim-callback-error-diagnostic` | 1 | A Jim control fixture falls back with `ErrorDiagnostic` where the fast path is expected. |
| `per-item-asserts-fallback-reason` | 1 | Pins the old design's fallback reason rather than the result. |
| `stack-budget-test-reads-ir-shape` | 1 | Counts `Statement::Foreach` in lowered IR to measure depth. |

## The observable

Every per-item test's observable is "the incremental analysis equals the whole-file analysis for the same text" (the correctness contract in `docs/design/rust/incremental-analysis.md`), or, for the fast-path tests, "and it got there without re-walking the whole file". Both are product behaviour under the 16 November ruling that per-body incremental analysis is restored and the memo tests are not re-scoped, so they are kept. The one exception is `private_source_advice_requires_whole_document_owner`, whose first assertion names the old design's fallback reason: rewrite to its second assertion, the equality. Four more tests of the same shape pass today and go to the second pass (`nested_opaque_declarations_use_full_original_source_owner`, `original_lexical_member_body_keeps_the_whole_source_owner`, `original_next_advice_reconstructs_whole_source_after_preceding_edits`, `computed_body_head_keeps_genuine_whole_document_value_owner`).

## Budgets

- `phi_undef_memo::nested_if_stages_analyse_in_linear_time` (10 s) and `…_in_a_loop_…` (10 s): the number is arbitrary. The test says so ("deliberately generous"); it guards issue #2021, path enumeration in the W210 phi trace, which would take forever, not a user-visible latency. Today they take 10.4 s and 13.3 s at `opt-level 1` (28 s and 37 s at CI's `opt-level 0`) because of the per-command constant, not enumeration. `REWRITE`: keep the W210 verdict and bound the phi-trace visit count; `nested_if_stages_do_not_grow_exponentially` already holds the ratio for the first.
- `depth_guard::tests::unrelated_command_observer_keeps_structured_bodies_within_stack_budget`: the budget protects a real crash (#996, a stack overflow aborts the server; `docs/design/compiler/recursive-descent-depth-limits.md`), so the budget stands, but the test measures depth by counting structured `Foreach` statements in the lowered IR, and under a command trace those loops no longer lower to that shape (0 counted). `REWRITE` to "lowering N nested bodies under an observer completes on the budget thread" plus the observer-depth witness.
- The server's `issue996_stack_overflow` tests (survive and answer within the harness timeout) are user-visible and belong to `server-e2e-db`'s ledger.

## Distinct failing behaviours

1. Editing one procedure re-analyses the whole file (22 tests).
2. A procedure's lattice and checks are recomputed after any edit (6).
3. The incremental result differs from the whole-file result on some sources (6), including lost W240 and W129 (4) and W308 (1).
4. A computed command head is recorded as a literal qualified name (1, wrong answer).
5. A factory-made class is missing from the whole-file analysis (1).
