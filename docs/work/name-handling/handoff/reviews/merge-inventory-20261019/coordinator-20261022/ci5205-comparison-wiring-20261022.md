# CI5205 exact nextest outcome wiring

This is an input contract, not a current-run outcome report. CI5205 is run `39845788481` (#5205), ref `work`, actual head `2b93470548313b99d29af108d8b38d52b2db7ceb`. The `active_source_snapshot` value in its dispatch receipt is provenance only; it does not replace the run's actual head SHA.

The comparator is the already-reviewed `/tmp/ci5203_exact_selector_compare.py` with SHA-256 `65cb36e14d9145039546e4b5a0b0de02b01e9feb749bc05f2d846d7c3625eaf2`. Its baseline is the 24,312 observed PASS records in `origin-rust-ci-5203-named-log-outcomes-20261022.json` (SHA-256 `8f0640329c7c60180099d61fe5c2d58ff13b76230b6046ef169f610b3ffa7a58`), run #5203 at `fcb7927f02b5395753247b322fdd7be87ae5c731`.

For each completed CI5205 nextest job, retain its actual job name/ID, conclusion, raw-log path and SHA-256, nextest version, profile/features, summary totals, parsed status-line totals, and exact emitted status records. Parse the full canonical `RustBinaryId` as printed by the status line; preserve the selector text verbatim. Do not derive the ID from a guessed package/binary suffix. Reconcile named status-line counts against the log's emitted totals. Summary-only skipped/ignored counts do not create named records.

The normalized current receipt should use schema `nextest-human-reporter-observed-identities-v1`, carry the exact CI5205 run metadata above, and preserve each raw record's `job`, `package`/rendered-label evidence when available, `binary_id`, `selector`, `status`, and ordinal. Run the existing comparator with that receipt and the immutable CI5203 baseline. Duplicate `(binary_id, selector)` keys fail closed. Baseline PASS matched to current FAIL/ABORT/TIMEOUT is a regression; current PASS is a match; current SKIP or missing is unknown; current PASS rows absent from baseline stay in the separate extra-current set.

Do not promote this comparison to suite completeness or an overall CI verdict. CI5205 aggregate-only runtime/WASM summaries and non-test gate logs are not selector outcomes. As of this inventory, the nextest test-shard and LSP e2e raw logs were still pending from root; no current normalized receipt or comparison exists.
