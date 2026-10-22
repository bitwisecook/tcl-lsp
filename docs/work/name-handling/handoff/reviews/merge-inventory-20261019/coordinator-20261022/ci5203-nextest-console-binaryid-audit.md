# CI 5203 nextest console binary ID audit

Recorded for root review on 2026-10-22. Read-only; no baseline JSON or repository source was changed.

## Finding

The console prefix without `::` is the canonical nextest `RustBinaryId`, not an incomplete package-only label. In nextest 0.9.148, the human reporter prints `TestInstanceId.binary_id` directly, separated by a space from the test name. The stable `RustBinaryId::from_parts` contract says lib and proc-macro unit-test targets use exactly the package name as their binary ID; integration targets use `package::target_name`; other target kinds use `package::{kind}/{target_name}`. Therefore `binary_label_as_rendered: null` can be interpreted losslessly as `binary_id = package`, for these emitted PASS records. The null value means no suffix beyond the package, not unknown binary identity.

## Exact CI and reporter versions

- Baseline receipt `docs/work/name-handling/handoff/reviews/merge-inventory-20261019/coordinator-20261022/origin-rust-ci-5203-named-log-outcomes-20261022.json` records run 39816429638 / 5203, ref `rust`, head `fcb7927f02b5395753247b322fdd7be87ae5c731`; receipt SHA-256 `d02e90de0acac350316f822bab19448c048fa57fb194159991960d7ec245f39a`.
- At that exact repository head, `.github/workflows/ci.yml` installs `nextest@0.9.148` for Rust shards (lines 1384–1388), heavy tests (1717–1720), and LSP e2e archive/partitions (2209–2215, 2365–2371). Rust shard command runs through `scripts/dev/rust-test-binary-shard.sh run`; LSP partition command is a direct `cargo nextest run`.
- Fetched completed GitHub job logs for Rust shard 1 (job 119956196367) and native LSP e2e partition 1 (job 119957901640). Both print the installed version `cargo-nextest 0.9.148 (cd1d6d546 2026-10-07)` (Rust log line 680; LSP log line 563). This matches the immutable upstream tag `cargo-nextest-0.9.148`, whose tagged commit is `cd1d6d5467322dbc0d39a80cb671d000ac302798` (tag object `69f89204b7e5b8e2c97432216a616cbe79caf4ba`).

## Versioned source proof (tag commit cd1d6d5467322dbc0d39a80cb671d000ac302798)

- `nextest-runner/src/reporter/displayer/imp.rs`, lines 1663–1671: the status prefix is written, followed by duration and `self.display_test_instance(...)`; lines 1972–1981 use the same formatter for test info output.
- `nextest-runner/src/helpers.rs`, lines 183–213 and 234–235: `DisplayTestInstance` holds the full `TestInstanceId`; default `max_width` is `None`; formatting emits `instance.binary_id` followed by a space, then `instance.test_name` as a separate component. Thus the first whitespace-delimited identity field in these console records is the binary ID itself (after the counter/duration fields), not a package label that must be expanded.
- `nextest-metadata/src/test_list.rs`, lines 295–309 and 338–355: `RustBinaryId::from_parts` is documented as stable API. `lib` and `proc-macro` IDs are exactly the package name; integration target IDs are `package::target_name`; other kinds are `package::{kind}/{target_name}`. This fixes the no-`::` case.

Source URLs:

- https://github.com/nextest-rs/nextest/blob/cd1d6d5467322dbc0d39a80cb671d000ac302798/nextest-runner/src/reporter/displayer/imp.rs
- https://github.com/nextest-rs/nextest/blob/cd1d6d5467322dbc0d39a80cb671d000ac302798/nextest-runner/src/helpers.rs
- https://github.com/nextest-rs/nextest/blob/cd1d6d5467322dbc0d39a80cb671d000ac302798/nextest-metadata/src/test_list.rs

## Raw CI examples and reconciliation

The raw job-log status lines were ANSI-stripped only for inspection:

- Rust job 119956196367, line 1636: `PASS [0.028s] (1/6760) tcl-compiler alias::tests::a_dynamic_alias_target_is_a_typed_unknown_not_a_source_spelling`. The reporter prefix is `tcl-compiler`; the exact nextest version appears at line 680.
- LSP e2e job 119957901640, line 718: `PASS [0.016s] (1/800) tcl-lsp-server environment_notice::tests::an_existing_marker_means_already_dismissed`.
- LSP e2e job 119957901640, line 904: `PASS [0.568s] (187/800) tcl-lsp-server::e2e bigip::bare_set_arity_not_flagged_on_bigip_conf`; the exact nextest version appears at line 563.

The saved named-outcome receipt has 24,312 PASS rows, including 14,094 rows with `binary_label_as_rendered: null` and 10,218 with a rendered suffix. Reconstructing the exact binary ID as `package` when the label is null and `package::label` otherwise yields zero duplicate `(binary_id, selector)` keys across all 24,312 records. The receipt’s per-job PASS line counts agree with summary counts and reporter ordinals are contiguous. The known LSP examples above match the null-label lib form and the `e2e` integration-target form.

## Suggested receipt correction and remaining limit

Root can revise `limits.canonical_binary_id` to state the reconstruction rule above and populate/derive `binary_id` for all 24,312 emitted rows. Keep `binary_label_as_rendered` as the observed suffix field if desired; null is now an exact, meaningful case. This resolves binary identity for emitted named PASS records and enables exact selector matching for them.

This does not recover the complete nextest listing or named skip identities. The 4,625 summary skips still lack emitted per-selector lines, and selectors absent from the PASS roster remain unknown. Do not call those skips matched rows or interpret missing selectors as P2F. The upstream source proof also does not by itself prove coverage beyond tests represented in the saved PASS logs.
