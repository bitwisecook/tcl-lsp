# CI 5203 nextest PASS-prefix boundary audit

Read-only follow-up to `/tmp/ci5203-nextest-console-binaryid-audit.md`. Baseline receipt SHA-256: `d02e90de0acac350316f822bab19448c048fa57fb194159991960d7ec245f39a`; 24,312 PASS records across 8 parsed jobs.

## Formatter boundary

At immutable nextest `cargo-nextest-0.9.148` source commit `cd1d6d5467322dbc0d39a80cb671d000ac302798`, `nextest-runner/src/reporter/displayer/imp.rs` `write_status_line_impl` lines 1651-1671 writes the status prefix and then formats `self.display_test_instance(...)` at lines 1663-1671. The helper at lines 1841-1862 constructs `DisplayTestInstance::new(...)` directly and does not call `.with_max_width(...)`. In `nextest-runner/src/helpers.rs`, `DisplayTestInstance::new` sets `max_width: None` (lines 183-213); trimming is conditional on `Some(max_width)` (starts line 234). The PASS status path therefore emits the full binary ID and test name without width truncation.

Versioned sources: `https://github.com/nextest-rs/nextest/blob/cd1d6d5467322dbc0d39a80cb671d000ac302798/nextest-runner/src/reporter/displayer/imp.rs` and `https://github.com/nextest-rs/nextest/blob/cd1d6d5467322dbc0d39a80cb671d000ac302798/nextest-runner/src/helpers.rs`.

## Roster and log check

Across all saved records, `package`, `binary_label_as_rendered` (10,218 non-null values), and `selector` contain zero literal `...` or Unicode `…` markers and zero explicit truncation-marker strings. All 24,312 records remain present; their per-job counts are unchanged. Re-fetched saved GitHub job logs for Rust shard 1 (job 119956196367; 1,515,575 chars; 6,760 PASS lines) and LSP e2e partition 1 (job 119957901640; 229,021 chars; 800 PASS lines) match their saved PASS counts and parse the expected status-line shape on every PASS line. Literal-ellipsis scan of the PASS lines found no truncation marker; the earlier broad word scan only matched legitimate test identifiers containing terms such as `ellipsis` and `truncated`, not formatter output markers.

This closes the presentation-truncation uncertainty for the canonical reporter path. It does not name skipped tests absent from the emitted PASS roster and does not establish complete selector coverage.
