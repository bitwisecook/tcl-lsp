# Narrow CLI fixture corrections

No production query/forensic behavior changes. These three selectors were
baseline-absent failures in CI5206; they are not counted as baseline gains.
Fresh execution is pending in this continuation.

- `source_transforms_refuse_invalid_name_bytes_before_artifacts`: keep every
  mutation refusal, exact unchanged-input check and artifact-absence assertion.
  Its readonly report asserted `/Common/old` but selected `.name`, whose
  documented fixture value is the leaf `old`. Select `.full-path` instead.
- `mutating_queries_require_faithful_named_and_structured_donors`: keep every
  invalid-byte refusal and exact unchanged target. The BIG-IP donor likewise
  selects `.full-path` so `rename` receives the qualified path before the actual
  byte-fidelity gate; `.name` is a leaf, not a full rename destination. The JSON
  donor case is unchanged.
The archive-content regression remains byte-for-byte unchanged. The user chose
"Expand explicit file reads to support lossy previews" in this chat on
2026-10-23. The implementation now routes `file(path)` through an exact archive
member operation, retaining original member bytes and metadata before preview
decoding. `files`, `glob` and `grep` retain their forensic discovery inventory.
No leaf-name fallback or lossy path alias can substitute for the explicit path.
Duplicate exact paths refuse rather than selecting one arbitrarily.

The additional provider test
`commands::query::provider_source_tests::archive_content_donors_retain_original_bytes_before_mutation`
uses a real UCS, the production reader, and the same
`QueryProviderSources::require_faithful` gate called before `emit_mutation`.
Both inputs display identical `new`+U+FFFD; the test asserts their distinct
original bytes, original size/text metadata, invalid-byte refusal (including
diagnostic), literal-U+FFFD acceptance, and an unchanged archive.

Repository authorities for the two operand-only fixture corrections:
`tcl-bigip-query/tests/fixtures/projection.json` has leaf `.name` golden values;
`projection.rs::project_pool` exposes `full-path`. A fresh production-library
probe confirms inline/multiline `.name` is `old`, not `/Common/old`; raw output
is in the continuation scratch directory. No refusal assertion is weakened.
