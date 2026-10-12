# Scoped LSP implementation digest renewal

Applied exactly the 22 joins across 21 records in
[the reviewed allowlist](lsp-additional-source-review.json). Every observed old
digest and all five current source digests were verified before mutation.
The two excluded meaning-reviewed records were preserved exactly.

All 1,179 record objects compare identical after reverting only the 22 allowed
implementation hash fields. Meanings, provider answers, capture references,
test statuses and all other evidence are unchanged. The 21 affected pages were
rendered with `update-name-resolution-proof-index.py:render`; their only changes
are the allowlisted evidence digest replacements. Source files were not edited.

Selected-record schema checks pass; all renewed hashes match current source.
[The exact check report](lsp-scoped-renewal-check.json) retains input/output
digests and the scope of verification. No Cargo, Rust test or native provider
ran. This is a scoped renewal check, not a complete proof audit or an execution
receipt.
