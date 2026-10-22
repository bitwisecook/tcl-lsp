# Partial CI5203 to CI5205 named comparison

This receipt joins exact named outcomes from four scoped CI jobs. It is evidence for those jobs only, not a full-suite comparison or overall gate verdict.

- Baseline: run 5203, `rust`, SHA `fcb7927f02b5395753247b322fdd7be87ae5c731`.
- Current: run 5205, `work`, SHA `2b93470548313b99d29af108d8b38d52b2db7ceb`.
- The current and baseline job IDs, conclusions, raw log hashes, line counts, and test-status line references are in [partial-join-receipt.json](partial-join-receipt.json). All eight raw logs are copied here byte-for-byte and their hashes were reverified after copying.

The observed exact-name joins are:

- WASM `tests/wasm_real_link.rs`: five current failures match explicit baseline passes. Two other current failures (`original_argv_descriptor_requires_current_manifest_and_rejects_prior_fingerprint` and `root_colon_publication_runs_its_own_compiled_table_entry`) are absent from the baseline log and remain unknown.
- Browser WASM E2E: two definition checks and one pack-property hover check have baseline `ok` lines and current `FAIL` lines. The current host harness later aborts on a separate rename request; it is recorded as an error, not another named test row.
- WASI E2E: three named checks have baseline `ok` lines and current `FAIL` lines. The Rust unit suite separately reports 22/22 passing in both runs.
- VS Code multi-folder E2E: four exact Mocha titles have baseline checkmarks and current failure lines; the job summaries report 14 passing versus 10 passing/4 failing.

These are literal observed statuses from the logged test targets and E2E titles. They are not converted into nextest binary IDs, and they do not modify the 24,312-record CI5203 nextest baseline. New or unlogged selectors remain unknown.
