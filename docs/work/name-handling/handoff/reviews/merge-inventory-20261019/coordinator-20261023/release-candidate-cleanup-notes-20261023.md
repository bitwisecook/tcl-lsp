# Release candidate cleanup notes — 2026-10-23

Candidate: `/workspace/naming-lsp-release/tcl-lsp`, base `fcb7927f02b5395753247b322fdd7be87ae5c731`.

The candidate-only edits removed the three development evidence-census scripts (`check-name-resolution-proofs.py`, `test_name_resolution_proofs.py`, `update-name-resolution-proof-index.py`), their Makefile target and `xtask-check` prerequisite, and the proof-specific owner-resolution route/helper/tests. The generic owner-manifest validator, internal-manifest validator, source-owner checks, and all semantic owner rows remain. Four semantic rows previously naming the removed evidence checker now name the existing `xtask-owner-resolution` gate. This was a static source edit only; no Cargo or test run was performed.

The removed proof-document tree had 5,143 source/test comment lines pointing to it across 1,061 candidate files. Those exact comment-only lines were replaced with blank lines that retain their newlines. The off-tree receipt `release-source-proof-comment-cleanup-20261023.json` records file byte counts and before/after SHA-256 values; it verifies all changed source lines were comment-only and all other source bytes stayed identical. Two coordinator-ruling comments in `references_residual.rs` were rewritten as durable explanations of the real-source test inputs. No implementation code changed as part of this cleanup.

Authored Markdown no longer contains links to the omitted proof tree. The visible titles were kept as ordinary text where their underlying descriptions remain useful; two evidence-index entries were removed from `docs/design/README.md`. A follow-up link scan found 25 additional local links into omitted experiments/probe/evidence paths; those links were also changed to their visible labels. This preserves their surrounding technical explanation but leaves those labels unlinked. They were not all retargeted to contracts/tests, so a reviewer may choose further specific mappings. The static link scan over AGENTS, both top-level READMEs, and `docs/**/*.md` reported zero missing local links after cleanup.

The candidate `AGENTS.md` remains a general developer guide. It now describes `rust` as the development branch with reviewed `release/*` candidates, keeps the name-handling design-contract link, and drops work-specific authorization/handoff text and stale catalogue references.

`git diff --check` passed for the reviewed documentation, Makefile, and owner-resolution files. No formatting/build/tests, Git mutation, or source integration was done. Functional fixes remain pending, so product claims still need final cross-check against the completed source candidate.

Separate source comment inventories: `release-source-comments-triage-20261023.json` (broad heuristic; noisy) and `release-source-proof-comment-links-20261023.json` (exact original reference inventory).
# Follow-up pass, 2026-10-23

The candidate now has no missing local Markdown links in the repository-wide
scan. The BIG-IP report formerly referenced by the authored static-group test
was reduced to the exact asserted `static_group` row in
`rust/tcl-vm/tests/data/authored_static_group_reinitialization.txt`; the test
still checks its four initialization records and original rule behavior.

This pass removed unexecuted-control inventories and capture/evidence-process
text from the import-order and name-handling design docs, and removed stale
proof-record wording from the retained-realm, container-index, TclOO linked
member and consumer-matrix docs. `git diff --check` passes in the release
candidate. The text patch against current work content and per-path manifest
are in `/tmp/naming-candidate-cleanup-review-20261023/`; binary deltas are
represented in the manifest by size and SHA-256 rather than included in the
text patch.
