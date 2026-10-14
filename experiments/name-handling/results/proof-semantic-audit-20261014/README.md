# Naming proof and ownership audit

This read-only audit ran against `work` starting at `73766b2d56064bef4cac5571cc421635adc1d416`. Authored Main files changed during the check, so the output is a failure inventory rather than a source-stable acceptance receipt. No Cargo build, Rust assertion, native interpreter or appliance execution was performed. No source implementation, provider observation, evidence seal, implementation digest or census digest was renewed.

## Results

The strict checker exits1 with2606 questions,31924 retained corpus files,7069 JSON candidates, zero pending classifications, zero unlinked test definitions, and1918 errors:

| Failure | Count |
| --- | --- |
| Evidence digest changed | 942 |
| Scrub-census digest changed | 952 |
| Census path drift | 12 |
| Retired owner declaration | 6 |
| Real test annotation missing catalogue binding | 2 |
| Exact document conclusion/scope omitted | 4 |

The942 digest errors affect165 questions and616 distinct paths. The question kinds are127 implementation contracts,34 native observations, and4 source anchors. Role counts are519 implementation,383 provider,38 observation,3 input and1 limitation entries. Several provider/observation mismatches refer to sanitised old F5 transport. Their current projected bytes must not be presented as unchanged original transport or fresh appliance observations. Restore neither removed security material nor historical executable input bytes.

`currency-review-worklist.json` binds every affected question to its actual test selectors and changed evidence paths. Current applicable assertions and exact semantic diff review are required before renewing implementation bindings. The six retired declarations and their question contracts are retained in `retired-owner-review.json`; renaming a declaration does not settle a changed contract. In particular, opaque callback body IDs, original hierarchy identities and structured host inventories require their own current ownership review.

## Bounded proposals

`owner-contract.patch` names genuine public catalogue ingress instead of crate-private implementation types, keeps private Module source-selection/ScanCtx helpers explicit, and registers the actual checked namespace, source-value, analysis-cache, event-body and APL convention owners. It preserves the newly integrated F5 object-name row. The strict visibility rules and source APIs stay unchanged. This is static declaration review; current `cargo xtask owner-resolution` remains required. The coarse196 restricted-declaration candidates include possible trait/re-export false positives and are not an executed gate result.

`bounded-proof-bindings.patch` links two real source-structure test definitions, explicitly without an execution claim; aligns two authored-contract conclusions with existing current documents; and restores the two exact Wasmtime redaction scope boundaries. Provider and evidence fields remain identical. `git apply --check` passes. Manifest files record proposal hashes and boundaries.

## Untracked W210 evidence

The untracked source-flow-production directory contains29 files. Twenty-two are byte-identical tracked captures; three compressed logs decompress exactly to the tracked source-read-flow logs. Its README, producer patch, ownership snapshot and artifact index are unique. All28 artifact index entries verify. The producer patch and ownership snapshot have no identical tracked file anywhere in the repository.

Preserve these unique supplementary producer artifacts and link the existing capture paths rather than duplicating25 capture/log files. They establish no additional result: three controls completed, the unchanged original oracle-program regression received SIGTERM before completion, and no completed35-test total or #2286 acceptance follows. Comparison receipts retain paths and hashes only, without sensitive contents.
