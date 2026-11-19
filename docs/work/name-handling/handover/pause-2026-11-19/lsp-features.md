# LSP features, 19 November 2026

Paused on the owner’s instruction at 14:19 UTC. The clean measurement checkout was on `work` at `a6c66517c7`. The active product checkout was on `work` at `ca48b18add` in `/tmp/bd-lsp-integration`. This handoff parks its only three modified production files as a single patch. No worker push or pull request occurred.

## Clean integration evidence

With C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 and Jim 0.84-15-g2f8e155, all three REQUIRE flags, `cargo nextest` 0.9.148 and no patches at a6c66517c7:

| tcl-lsp-core binary | Run | Passed | Failed | Skipped |
| --- | ---: | ---: | ---: | ---: |
| it_2 | 294 | 268 | 26 | 0 |
| it_3 | 289 | 264 | 25 | 0 |
| it_4 | 322 | 275 | 47 | 0 |
| it_5 | 349 | 268 | 81 | 0 |

The full 179-name failing list is in `../../lists/lsp-features-integration-failing-2026-11-19.txt`. Raw terminal summaries are in `measurement/all-newpin.log.txt`. No fresh clean tcl-lsp-core **lib** run was completed here. These numbers describe the clean baseline, not any subsequent fix. Diagnostics owns the strict variable-rename coverage witness and its e2e control.

## Product work and parser inventory

The coordinator’s combined checkout includes the shared native regsub replacement grammar, scanf grammar and editor directives, native C/Jim regexp and string glob source components, selected switch flags, cursor word geometry, expression-tree comparisons, SCF stanza and shell token spans, and extracted-language fixes. Reconcile cherry-picked SHA equivalents against current `work` before applying anything. Source candidates from the legacy feature checkout run from `e7de366043` to `1863245272`. The post-extraction counted-glob type and selected quote geometry keep their lower owners.

- TclOO method rename at the public entry edited two occurrences while leaving the old punctuation-bearing selector in `set callback [list object {m;n}]`. Spaced and parenthesized selectors also reproduced. C8.6, C9.0 and C9.1 demonstrate that the stored invocation fails after such an edit. Local candidate `d5041259dc` became `1d23f7717e` in the combined checkout. The related complete-word superclass/mixin fix `db18fd072d` became `731daac63f`. The probes and original product output are retained under `measurement/member-*` and `measurement/relation.tcl.txt`. Run the new public candidate controls and the strict rename e2e test before closing the rows. The punctuation splitter still protects other unresolved source words and needs a separate completeness review.
- `tcl-lsp.extractLinkedObjects` lost the header and the final brace at EOF. Original `BlockSpan` byte bounds plus document-relative LSP UTF-16 conversion, including an astral-prefix control, are in `5c5ac84d45` (combined `4fe4b5ff0b`). No F5 device result is implied.
- The 18 configured-reader dispositions and two occurrence-specific index-call proofs are in the parser review records. The 831-row structural pass still classified 238 as structural leads at pause. The compact list in `measurement/remaining-lsp-structural-leads.tsv` retains actual sites. Execution has C Unix filesystem `c_glob_head` in `ab95c8766f` and a Jim whole-pattern tokenizer pending integration. File expansion depends on reached directories and selected OS. Filesystem hover remains distinct from string glob matching. Unsigned function scopes remain open.

## Switch and dictionary patch chain

On a fresh origin/work baseline, verify existing changes first, then apply the following to the area checkout only. Equivalent cherry-picks need no reapplication.

1. `patches/lsp-if-literal-table.patch` (`31e02d2ac7`, integration copy `cec1caee5c`) preserves complete list/body data, the literal final pattern `default`, and Tcl’s special body `-`. `measurement/switch-control.tcl.txt` is the portable oracle witness.
2. `patches/analyser-prospective-comparison-heads.patch` is analyser-binding’s `0b3386b24c` (integration copy `cf08cd8b4a`). It selects proposed exact switch, dict get/getwithdefault, set and return heads against the current command world including the ensemble worker. Keep it with the analyser owner.
3. `patches/lsp-per-site-comparison-wip.patch` is the uncompiled working diff against `ca48b18add`. It consumes the above issuer, actual selected native source, and positioned closed scalar read facts. A static mapping can use a literal dictionary with no temporary when all known subjects are covered. Explicit default lookup uses the selected dialect’s command. A stored operand name or a reached procedural caller cannot authorize an edit in every future activation. Only a bounded document-level, side-effect-free subset has been attempted. Do not publish the patch as a validated product fix.

At the actual compiler input, `set k a` followed by an if comparing `$k` twice produced two distinct closed reads, both with value a. Switch produced one. The evidence is in `measurement/rewrite-points.rs.txt` and its log. These probes precede the proposed-head consumer and do not establish a successful edit. First type-check and run the public action on genuine defined-scalar and exhaustive-literal cases. Verify that nested conditional-arm dispatch receipts are available for selected set/return and strip incidental restrictions only where proven. Keep observer, shadowed/rebound `::switch`, `::dict`, private `::tcl::dict::get`, changed ensemble map, occupied would-be temp, dead destination plus reachable missing key, duplicate key, fallthrough, and literal final-default controls. Validate Tcl 8.4 without dict, Tcl 8.5/8.6 without getwithdefault, and current Jim. Preserve historical expectations in the scrub ledger, amend the stale contract and address the nine owner-approved rows individually. The WIP has not updated that contract or those nine rows. The F5 when-body selector stays with the device ledger.

Rebuild Jim, export the three REQUIRE flags, take a new clean-head baseline before citing a count, and coordinate all large Rust work and pushes with the coordinator. Use cargo fmt, clippy --all-targets and make xtask-gates before a push.
