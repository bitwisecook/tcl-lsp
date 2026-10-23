# CI 5206 worker-log triage

Read-only triage of decoded connector logs for run 39879737945, published
head `5a4182dfefc483b6446085bdabd9a1f7620e6a96` (product source `3b1a750`).
The 30-row live jobs roster snapshot was written at 2026-10-23T01:04:01Z.
The root later received connector metadata showing 7 success, 10 failure,
11 in progress, and 2 skipped; its wrapper did not prove roster completeness.
The captured logs are each below the 20 MiB cap; the compressed originals and
per-file hashes are in `ci5206-connector-logs.tar.gz` and
`hash-manifest.json`.

`classification.json` records the exact compiler errors and named test
outcomes. Runtime comparison uses only the `runtime-rust-test-no-tommath`
command section from the successful origin/rust CI 5203 job. The independent
`wasm_real_link` selector join uses only that target from the origin/rust CI
5203 job. The WASI host session join is recorded separately and preserves the
12 unexecuted shutdown repetitions as unknown. Those comparisons stay
separate. The runtime lib-test process aborted before producing a suite
summary, so its unreported selectors remain unknown. Test outcomes come from
log lines and not job state. A count-reconciliation correction is recorded in
`classification.json`; the initial classification is preserved, with its
SHA-256, in `classification-v1-20261023T011356Z.json`.
