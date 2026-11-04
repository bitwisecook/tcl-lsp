# External issue validation comments

Inventory rechecked 2026-11-04 18:27 UTC: the same 20 external issues remain open. Evidence revisions and measured scope remain in the comments. All **19 requested issue comments** were posted and
reread on GitHub. **#2021 was excluded and received no comment from this sweep.**
No external issue was closed by this work. Revisions, commands, native-oracle
versions, controls and scope limits are in the linked comments.

A quiet reported example is not called fixed when its genuine positive control
also fails. The top-level S100 LSP path now has an authenticated positive and
quiet controls, but the reopened S102 report in #940 remains unresolved.
#2498, #2500 and #2504 retain their earlier inconclusive assessments;
#2286, #2501 and #2502 retain missing-read gaps, and #2181 has unverified UI and
broader lifecycle paths. The LSP is not release-ready on this evidence alone.

| Issue | Validation outcome | Evidence comment |
| --- | --- | --- |
| #940 | Partial: authenticated top-level S100 publication and quiet controls pass; reopened conditional S102 report remains unresolved. | [Proof](https://github.com/bitwisecook/tcl-lsp/issues/940#issuecomment-6629102912) |
| #2181 | Partial: routed-environment edit/reopen and focused role changes pass; UI and broader lifecycle paths remain unverified. | [Proof](https://github.com/bitwisecook/tcl-lsp/issues/2181#issuecomment-6629241452) |
| #2242 | Verified native LSP arity publication and repair under Tcl 8.6/9.0. | [Proof](https://github.com/bitwisecook/tcl-lsp/issues/2242#issuecomment-6629219439) |
| #2265 | Verified exact Core token extents and geometry control. | [Proof](https://github.com/bitwisecook/tcl-lsp/issues/2265#issuecomment-6629091417) |
| #2286 | Partial: reported W210s absent, but genuine Tcltest missing-read controls are also missed. | [Proof](https://github.com/bitwisecook/tcl-lsp/issues/2286#issuecomment-6629095522) |
| #2288 | Verified additive bind diagnostic and active W105 control. | [Proof](https://github.com/bitwisecook/tcl-lsp/issues/2288#issuecomment-6628779207) |
| #2289 | Scoped non-reproduction on current linked input and reduced creator/call; full RBC execution not tested. | [Proof](https://github.com/bitwisecook/tcl-lsp/issues/2289#issuecomment-6629096965) |
| #2290 | Scoped non-reproduction on current linked input; historical September input not established. | [Proof](https://github.com/bitwisecook/tcl-lsp/issues/2290#issuecomment-6629098284) |
| #2497 | Verified minimal inherited my reference/lens agreement; broader partial-coverage label tests still fail. | [Proof](https://github.com/bitwisecook/tcl-lsp/issues/2497#issuecomment-6629092872) |
| #2498 | Inconclusive: genuine shimmer controls also emit no warning. | [Proof](https://github.com/bitwisecook/tcl-lsp/issues/2498#issuecomment-6629104505) |
| #2499 | Verified dynamic read diagnostic and active W212 write-position control. | [Proof](https://github.com/bitwisecook/tcl-lsp/issues/2499#issuecomment-6628780552) |
| #2500 | Inconclusive: genuine shimmer controls also emit no warning; full example supported only on Tcl 8.5–9.1. | [Proof](https://github.com/bitwisecook/tcl-lsp/issues/2500#issuecomment-6629106024) |
| #2501 | Partial: reported W210 absent; a reached control with lassign removed still misses a genuine read. | [Proof](https://github.com/bitwisecook/tcl-lsp/issues/2501#issuecomment-6629099746) |
| #2502 | Partial: reported W210 absent; a procedure-local control without dict with still misses a genuine read. | [Proof](https://github.com/bitwisecook/tcl-lsp/issues/2502#issuecomment-6629101423) |
| #2503 | Verified quoted nested expr diagnostic and active W114 control. | [Proof](https://github.com/bitwisecook/tcl-lsp/issues/2503#issuecomment-6628781788) |
| #2504 | Inconclusive: genuine shimmer controls also emit no warning. | [Proof](https://github.com/bitwisecook/tcl-lsp/issues/2504#issuecomment-6629107328) |
| #2505 | Verified multiline/single-line list callbacks and active W301 control. | [Proof](https://github.com/bitwisecook/tcl-lsp/issues/2505#issuecomment-6628783425) |
| #2506 | Verified boolean-derived operands and direct untrusted-input T100 control. | [Proof](https://github.com/bitwisecook/tcl-lsp/issues/2506#issuecomment-6629094133) |
| #2507 | Verified W100/O111/scan recommendation and safe action behavior; seven tests pass. | [Proof](https://github.com/bitwisecook/tcl-lsp/issues/2507#issuecomment-6629220744) |

The full linked inputs, hashes, upstream licences and compiler receipts are under
`rust/tcl-compiler/tests/data/lsp-external-bindings/`. Values and protocol details
remain in their `LSP_RELEASE_*.md` lane records. A machine-readable comment audit
is in [LSP_RELEASE_ISSUE_COMMENTS_2026-11-04.json](LSP_RELEASE_ISSUE_COMMENTS_2026-11-04.json).
