# Narrow CLI fixture corrections

The three selectors were baseline-absent failures in CI5206; they are not
counted as baseline gains. Original assertions are retained. Current execution
found the additional fixture-shape corrections below; final rerun is pending.

- `source_transforms_refuse_invalid_name_bytes_before_artifacts`: keep every
  mutation refusal, exact unchanged-input check and artifact-absence assertion.
  Its readonly report asserted `/Common/old` but selected `.name`, whose
  documented fixture value is the leaf `old`. Select `.full-path` instead.
- `mutating_queries_require_faithful_named_and_structured_donors`: keep every
  invalid-byte refusal and exact unchanged target. The BIG-IP donor likewise
  selects `first($donor.ltm.pool[].full-path)` so scalar `rename` receives the
  one qualified path before the actual byte-fidelity gate; `.name` is a leaf, not a full rename destination. The JSON
  donor case is unchanged.
The archive-content regression keeps both original byte cases and every
assertion. Its selected config now puts `description old` on its own line:
`edit_plan::splice_edits` requires an actual single-line FieldSlot for editing,
and the inline one-line stanza had none. That unrelated source-geometry error
prevented reaching the UTF-8 gate. The query expression is unchanged.

The user chose
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

Fresh production-binary controls confirm both corrections: scalar `first`
reaches the invalid-UTF-8 refusal instead of the stream-type error; the
multiline archive fixture displays identical previews, refuses invalid source
with empty stdout, accepts literal U+FFFD, and leaves each archive unchanged.
Raw controls: `named-donor-scalar-probe-20261023.json` and
`archive-multiline-probe-20261023.json`. `bi_first` and scalar `bi_rename`
contract are in `builtins/mod.rs` and `builtins/rename.rs`. No assertion or
admission guard is relaxed.
