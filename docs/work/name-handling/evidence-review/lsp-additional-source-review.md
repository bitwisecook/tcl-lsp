# Additional LSP source review

All 26 joins in [lsp.json](lsp.json) were accounted for; the proposed allowlist
contains exactly 22 implementation joins across 21 records. Four joins belonging
to `naming.analysis.constructed-key-consumer-projections` and
`naming.core.original-dispatch-region-context` are excluded because their meaning
was separately reviewed and corrected.

Observed HEAD: `ef3bcdbfc3ee73f25d4ade4a182b8110ad943507`.
The five source owners matched that HEAD's tree bytes. Every audit
`current_sha256` was recovered exactly from loose/packed Git objects and checked
against its complete source bytes, without invoking Git.

| Owner | Additional delta from audited source |
| --- | --- |
| `completion.rs` | Six existing assertions wrapped; no token or literal change |
| `definition.rs` | Two blank lines removed |
| `references.rs` | Byte-identical |
| `rename.rs` | One blank line removed |
| server `lib.rs` | One blank line removed |

Full-file Rust token sequences, including comments and exact literal contents,
compare identical in all five files. The prior unchanged scoped invariants,
assertion bodies and refusals therefore remain unchanged by the additional
delta. Each allowed join matches the current manifest's exact evidence ID,
implementation role, path and old digest; the record question matches the
audited invariant. No old snapshot is missing and no additional semantic delta
was found.

[The machine allowlist](lsp-additional-source-review.json) contains every exact
`(record_id, evidence_id, path, oldhash, newhash)` tuple, recovered blob IDs,
audit hashes, token digests and the scoped reasoning for each invariant.
`oldhash` is the currently recorded implementation digest; the separate
`audit_current_sha256` identifies the already-reviewed intermediate snapshot.
These two values are intentionally distinct.

This review only establishes the additional delta and composes it with the
original scoped review. Earlier whole-file changes can include semantic work
outside these unchanged questions; they are not described as formatting only.
No Cargo, Rust test or native provider ran, and linked coverage remains
unexecuted. `diagnostic_subject.rs`, its active C obligations and all other
owners are outside this five-file review. No manifest or source was changed
while producing this review. Recheck source hashes and each exact old digest
before applying the allowlist; preserve all meanings, provider answers,
capture bytes and test statuses.
