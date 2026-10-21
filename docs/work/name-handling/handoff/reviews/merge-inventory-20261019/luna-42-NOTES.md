# Luna-42: BIG-IP portions of the LSP (CI blockers + deferred BIG-IP units)

Branch `work`, HEAD 75c67fa555. Nothing committed, staged, stashed, reset or
checked out. All changes are in the working tree; `luna-42.patch` is
`git diff` of exactly the paths listed under "Patch scope" plus the two new
files (`git diff --no-index /dev/null …`). Other agents' uncommitted edits
(tcl-compiler, tcl-spectcl, runtime/rust, tcl-lsp-db, tcl-lsp-core
`tests/name_resolution.rs`, `zz_*_probe.rs`, naming-consumer-matrix.md) were not
touched and are not in the patch.

## Step 0 — CI blockers (run 39616304947 on f6c4694f14)

| Blocker | Fix | Files |
|---|---|---|
| TS6133 `serverIsRunning` (extension.ts:927) | Removed: dead since 91a253315f moved every caller to `client?.isRunning()` with captured client identity. | `editors/vscode/src/extension.ts` |
| TS6133 `effectiveDialectFor` (extension.ts:977) | Removed: its last caller (`runRuntimeValidationForDocument`) moved to the report-owned dialect in 212542632d; `scheduleDialectRefresh` reads `effectiveConfigFor` directly. | same |
| TS6133 `request` (currentDiagnostics.test.ts:970) | Used as intended: the test now records every proposal request and asserts the single request carried the original uri/version/source/proposal (the old `calls === 1` assertion is kept as the length of that exact list). Strengthened, not weakened. | `editors/vscode/src/test/currentDiagnostics.test.ts` |
| Prettier rejects `nameGeneration.ts` | `prettier --write` (two line joins, no semantic change). The file is hand-written, not generated. | `editors/vscode/src/nameGeneration.ts` |

Checks run (the CI commands): `npm run lint` (prettier --check src/**/*.ts) PASS;
`make copy-canonical && npx tsc -p ./` PASS (0 errors); esbuild node + browser
bundles PASS; `make check-vscode-test-partitions` PASS (after the count update
below). Not run locally: the VS Code extension host suites (`make
test-ext-partition`, `npm run test:web`) — they need a release `tcl-lsp-server`
build and a downloaded VS Code; there was neither CPU nor disk (≈2 GB free,
shared) for that.

## Units (35) — status

Legend: DONE = required change implemented with tests; PARTIAL = implemented
as far as stated, remainder named; ALREADY = the cited site was already
migrated upstream (no edit); REMAINING = not started, reason given.

### translation.document-input-transport (XC)

| Unit | Status | Change | Tests |
|---|---|---|---|
| lsp-server-xc-diagnostic-input | DONE | `f5_xc::translate_irule_for_document` / `get_xc_diagnostics_for_document` walk the document's own `AnalysisResult`, its retained registry, and a unit built from that analysis (`tcl_lsp_db::compilation_unit_uncached_from_analysis`); refuse (None) for another source, a context-unavailable analysis or a non-iRules unit profile. Server `xc_findings(source, analysis)` and `published_findings(.., xc_document: Option<&AnalysisResult>, ..)`: the push, pull and code-action paths pass their actual analysis; no fixed-default reparse remains. | f5-xc: `document_translation_walks_the_documents_retained_registry`, `document_translation_refuses_other_source_or_a_non_irules_input` (new, PASS). Server: `xc_findings_refuse_an_analysis_of_other_source` (new); `xc_findings_surface_and_the_report_filters_codes`, `published_xc_codes_are_known_diag_codes_issue_2121` now build the analysis through the server ingress (assertions unchanged) — compiled, not executed (see Measurements). |
| lsp-node-xc-document-full-input-transport | DONE | New server command `tcl-lsp.xcTranslateDocument [{uri,version,source}, format]`: snapshot must equal the server's copy, dialect must be iRules, translation under the document analysis, re-checked current before publication; reply carries `source:{purpose:"document",uri,version,dialect}`; refusals are `{unavailable: request|notOpen|stale|notIrules|analysis}`. Node `translateXc`/`translateXcConsole` use it via `bigipSourceActions.translateXcDocument`; a non-iRules/unanswerable document falls back to the explicit standalone `tcl-lsp.xcTranslate` and the message says which ("Translated the open document (version N, f5-irules) under its own analysis." / "...as standalone iRule source: <reason>"); `stale` is never reparsed. | Server `xc_translate_document_uses_the_current_open_document` (new, compiled). TS `bigipSourceActions.test.ts` (4 XC cases). |
| lsp-browser-xc-document-input-transport | DONE | Same as Node, via the shared module (browser `executeServerCommand` → `serverCommandSender`). | same TS tests |
| lsp-editor-xc-document-context | DONE | `/xc` uses `resolveIruleSource`: a whole current document snapshot goes through the document ingress, a selection/attachment-selection/inline code through standalone (`sourceOnly`); the reply states which. Gap/fallback prompts now say the mappings are declared recommendations, not outcomes observed on a BIG-IP/XC deployment. | covered by the shared-module tests; chat handler itself has no unit harness |

### host.original-rule-source-action / BIG-IP wire

| Unit | Status | Change | Tests |
|---|---|---|---|
| lsp-server-write-rule-back-currency | DONE | `writeRuleBack [ruleReceipt, newBody]`: rejoins the stanza in the current document by header + full path + original body (exactly one match, wherever it moved), splices with the shared `rule_extract::replace_rule_body`, requires the result to reparse as the same rule with the new body at the same start, applies a **versioned** `documentChanges` edit at the server's document version, and returns `{applied, rule}` with the post-edit geometry from the parser (or `{applied:false, reason: request|notOpen|stale|body|rejected}`). The legacy `[uri,start,end,body]` form keeps its boolean reply and unversioned edit for other callers. `rule_info` now carries `header` and `documentVersion`. | server code compiled; TS registry tests below. No server unit test for the client round-trip (`apply_edit` needs a client). |
| lsp-node-rule-scratch-source-byte-and-lifetime | DONE | Node uses `RuleScratchRegistry`: the receipt is the server's object, no UTF-16 `newBody.length` arithmetic, saves of one scratch document are serialised so each write sends the previous answer's receipt; listeners moved from module top level into `activate` (`context.subscriptions`), sender read per save (rejoin is content-based, so a restarted server is still correct). | `bigipSourceActions.test.ts`: write-back receipt round trip, serialised saves, refusal keeps receipt, no server. |
| lsp-browser-rule-scratch-source-and-byte-currency | DONE | Same registry in the browser entry. | same |
| lsp-node-browser-bigip-cursor-byte-boundary | DONE | `extractRule` / `extractLinkedObjects` accept `{position:{line,character}, version}` (UTF-16 LSP position) per cursor; the server converts with the document's LSP line index only at that version (else `{unavailable:"stale"}`); bare numbers stay the legacy byte offset (JetBrains). Both VS Code entries send typed positions. | TS `cursor positions travel as versioned LSP positions`; server compiled. |
| lsp-server-bigip-linked-object-report | DONE | Reply adds `sources:[{uri,version}]` (the document versions the graph was read from); typed cursors as above. `sourceOrigin` stays null; BFS node ids unchanged. | compiled |
| lsp-editor-workspace-edit-command-wire | DONE | `LspWorkspaceEdit` models `documentChanges`; `workspaceEditFromLsp(edit, versionOf?)` converts a versioned change only while the open document is at that version (else `undefined`), unversioned `changes` as before. `renamePartition` captures the document version before the input box and sends it as a 4th argument; the server refuses a different version and answers with a versioned `documentChanges` edit (3-argument callers unchanged). | TS `a versioned server edit converts only at its own document version`, `a partition rename names its document version...` |
| lsp-code-actions-bigip-rename-command-routing | PARTIAL | Partition argument now comes from the shared `tcl_bigip::object_names::object_path_partition` (bare identifier verbatim, or the single component of `/<p>`; `//x`, `/a/b` offer no partition action) instead of `trim_start_matches('/')`. Version identity is captured at invocation (above), not carried in the code action itself: `bigip_code_actions` has no document version and `ActionCommand` has only string/u32 arguments. | `partition_argument_comes_from_the_shared_object_path_owner` (new, integration test, compiled, not executed) |

### tcl-lsp-core BIG-IP

| Unit | Status | Change | Tests |
|---|---|---|---|
| lsp-bigip-basename-uri-consumer | DONE | `bigip::is_bigip_conf_uri(uri)`: the URI path's last segment, strictly percent-decoded (fluent-uri), any scheme; encoded `.` matches, a literal backslash stays name data, undecodable/unparsable → false. The server's `dialect_for_open_sync` uses it; `is_bigip_conf_name` (path/basename contract) is unchanged for its existing callers. | core `a_protocol_uri_is_classified_by_its_decoded_last_segment` (PASS); server `dialect_for_open_reads_the_decoded_uri_basename` (compiled). |
| lsp-hover-configured-valid-event-guidance | DONE | `EventRelease::of(context)` = the actual context's `placement_floor("f5-irules-cmds")` (the same owner completion uses); **Valid events** are filtered by `EventRegistry::event_available_at` at that release and labelled "(BIG-IP x.y.z)", or "(event requirements only; no BIG-IP release configured)". Wired through the original-registry renderer and the test-only builtin renderer. | `valid_events_apply_the_configured_release_lifecycle`, `hover_valid_events_follow_the_documents_configured_release` (new, PASS); existing valid-events tests PASS (signature-only adaptation: `EventRelease::Unconfigured`). |
| lsp-semantic-embedded-bigip-body-owner | DONE | BIG-IP config semantic tokens walk the bodies `tcl_bigip::rule_extract::find_embedded_rules` reports (iRules brace grammar), not the presentation lexer's brackets; `conf_tokens::embedded_rule_bodies` (the config lexer's exclusion set) puts those parser bodies first and keeps a presentation span only where it overlaps none, so no overlapping tokens. | tcl-bigip `an_escaped_brace_does_not_end_the_rule_body` (new, PASS); core integration `bigip_conf_embedded_rule_body_ends_where_the_rule_parser_ends_it` (new, compiled). |
| lsp-datagroup-command-and-name-source-construction | DONE | Data-group and target-variable operands are built with the shared `word_rules::lexical_unicode_source_word` and each generated `class match` / `class lookup` / `set` command is re-parsed to check the operand word's static value equals the intended data (`generated_operand_values`). Supplied names stay verbatim in the definition (not sanitised). `membership_extraction` takes a `MembershipArms` struct (no new `#[allow]`). | `supplied_names_are_emitted_as_exact_literal_operands` (new, PASS); all other datagroup unit tests unchanged and PASS except `inside_when_body` (see failures). One test call adapted to the new signature only (`membership_extraction` in `decoded_membership_script_indentation_tests`), assertions unchanged. |
| lsp-ilx-navigation-sibling-input-transport | PARTIAL | `OpenDocuments::analysis(path)` (default None). Sibling rule files (forward call sites, source candidates, and the JS reverse route) are scanned under the sibling's own analysis when the host supplies one; otherwise the route's existing rule applies (no borrowed initiating facts; closed siblings are unchanged — still no facts under a document analysis). | `an_open_sibling_rule_is_scanned_under_its_own_analysis` (new; first run exposed a fixture flaw — identical sibling bytes — fixed; re-run not done, see Measurements). |
| lsp-server-ilx-per-document-source-and-candidate-ingress | PARTIAL | `OpenDocumentTexts` now also holds each open iRules document's own `analysis_for` result. Remaining: closed sibling files (store-read) still have no per-document input; that needs a retained analysis owner for closed files (workspace index) — not built. | compiled |

### iRules metadata / AI chat

| Unit | Status | Change | Tests |
|---|---|---|---|
| lsp-server-irules-metadata-commands | DONE | `describeIruleEvent` keeps `deprecated:false` (existing contract) but labels it `deprecatedSource:"eventCompletionDetail"`, adds `lifecycle:{axis:"BIG-IP",introduced,deprecated,retired}` from `EventRegistry::event_lifecycle` (declared data) and `catalogue:"standaloneIrules"`. | e2e `describe_event_*` unchanged; no new server test |
| lsp-ai-event-command-query-context | DONE | `/event` and the context pack stop presenting the completion flag as "Event deprecated"; they show the declared lifecycle, say the facts are the standalone registry catalogue, and state the active document's configured `tclLsp.bigipVersion` with whether the catalogue declares the event there (or that no release is configured). The natural-language query grammar is unchanged and grants no identity. | none (chat has no unit harness) |
| lsp-ai-event-new-scaffold | DONE | `/scaffold` reads words through the generated registry catalogue (`catalogueEventName`), drops events not declared at the configured `tclLsp.bigipVersion` (reported), tells the model the target release or that none is configured, and uses the honest clean-report line. | `parseScaffoldEvents` exported; covered indirectly by skeleton tests |
| lsp-editor-ai-create-clean-report | DONE | "Validated clean" replaced by `cleanReportLine`: names the completed report's dialect, "this exact source", and "static analysis only; not run on a BIG-IP"; without a completed report it claims nothing. The fallback template is labelled an authored, unvalidated recommendation. | none |
| lsp-ai-diagram-source-purpose | DONE | Uses `resolveIruleSource`; the reply states "Diagram of `<uri>` (version N) / the supplied text, analysed as standalone iRule source; it shows what the source may do, not what runs". Prompt: structured data is authoritative for structure only; events ordered by declared lifecycle order with dashed "may follow" edges, never a guaranteed sequence. | `diagramPrompt.test.ts` assertion (`## Structured flow data (authoritative`) still holds |
| lsp-editor-event-skeleton-construction | DONE | `buildIruleEventSkeletonFor(events, {bigipVersion})`: only names in the generated registry catalogue are written (exact, else upper-cased match), others are rejected and reported, never interpolated; with a configured release, events the catalogue's lifecycle does not declare there are rejected. `buildIruleEventSkeleton` keeps its string API. Both entries pass `tclLsp.bigipVersion` and warn on rejections. | `iruleSkeleton.test.ts` +2 tests |
| lsp-node-irule-skeleton-source-inference | DONE | Preselection now reads `vscode.executeDocumentSymbolProvider` (the server's `when` Event symbols from the actual analysis) instead of a regex over raw text; no outline, no preselection; version must not change during the read. | none in TS (needs the host) |
| lsp-generated-editor-event-catalogue | ALREADY | `editors/vscode/src/generated/iruleEvents.json` already has `HTTP_REQUEST.introducedVersion = "9.0.0"` (regenerated in 3d92614629), matching `events.rs` `Lifecycle::introduced_in("9.0.0")`. Static parity only; `cargo xtask gen-editor-catalogs --check` not run (xtask build). | — |

### Zed / TextMate projections

| Unit | Status | Reason |
|---|---|---|
| lsp-zed-irules-variable-catalogue | ALREADY | `languages/irules/highlights.scm` `@variable.builtin` is generated from `reg.special_vars()` filtered by the language's providers (`gen_zed_queries::variable_spellings`), regenerated in e0612d3736 together with the Tcl sibling. Drift test (`committed_queries_match_generated`) not run (xtask build). |
| lsp-zed-iapps-variable-catalogue | ALREADY | same generator, `languages/iapps/highlights.scm` |
| lsp-zed-tmsh-context-routing | REMAINING | Needs a split into a `BIG-IP` config language (grammar `bigip`, `tcl-bigip`) and a Tcl-grammar `TMSH` language on `.tmsh` (`tcl-tmsh`): new `ZED_LANGUAGES` row in `rust/xtask/src/editor_extensions.rs`, a `gen_zed_queries` target for f5-tmsh, moved `highlights.scm`/`injections.scm`, regenerated `extension.toml`, and `zed-query-check`. xtask is outside my ownership and needs an xtask build + drift gates. |
| lsp-zed-apl-static-name-presentation | REMAINING | Needs a generator projecting the APL field types/validators (and BIG-IP typed property keys) from their owners into `highlights.scm`; no such projection exists in xtask. |
| lsp-zed-tmsh-static-name-presentation | REMAINING | same as above (BIG-IP config vocabulary) |
| lsp-vscode-apl-static-name-projection | REMAINING | same, for `syntaxes/apl.tmLanguage.json` |
| lsp-vscode-bigip-static-name-projection | REMAINING | same, for `syntaxes/bigip.tmLanguage.json` |

### Other

| Unit | Status | Change / reason |
|---|---|---|
| lsp.validation.00.00.fn0 | DONE | `walker_matches_golden` now asserts, before pairing: non-empty cases, one golden row per case, each row exactly `CASE i<TAB>expected`. Existing fixtures/expected assertions unchanged. Census PASS; the pre-existing golden comparison (line 85) still fails as in CI-5201 (`FAIL tcl-irules::walker walker_matches_golden`). |
| lsp-completion-event-clause-template | REMAINING | The `timing`-only rule for argument ≥2 needs a registry-owned partial layout for `when` (the `arg_values` table on `when.rs` cannot say "values only after `timing`"; `irules_event_declaration_shape` validates complete shapes only). That is the realm-aware iRules world in `tcl-registry` (Sol-49). LSP side: once the registry exposes it, `completion/original_context.rs:97` should ask it instead of comparing the previous word. |
| lsp-semantic-irules-original-input-overlay | REMAINING | `OriginalIrulesSourceContext::from_source_analysis` requires `has_original_vendor_source_names()`, so switching the object overlay to the document's analysis could drop highlighting for analyses without vendor names; not done without a measured baseline. The process-wide `when` fallback should become an explicit generic-Tcl display purpose used only when the document registry has no `when`. |

Totals (35): DONE 22, PARTIAL 3, ALREADY 3 (no edit needed), REMAINING 7.

## Shared-file hunks (for reconciliation with Luna-41)

`rust/tcl-lsp-server/src/lib.rs` (all BIG-IP/F5 routing):
- `OpenDocumentTexts` → struct `{texts, analyses}` + `OpenDocuments::analysis`; `open_document_texts` collects open iRules analyses after releasing the documents lock.
- `xc_findings(source, analysis)`; `published_findings(.., xc_document: Option<&AnalysisResult>, ..)` and its 4 callers (push lift, pull `full_diagnostics_for`, `code_action_report`, one test).
- `dialect_for_open_sync`: `is_bigip_conf_uri` (+3 comment references).
- `describe_irule_event_command`: lifecycle/catalogue fields.
- `rule_info(.., version)`, `source_position_offset`, `list_rules_command`, `extract_rule_command`, `write_rule_back_command` + `write_rule_back_receipt` + `rejoin_rule` + `apply_rule_body_edit`, `extract_linked_objects_command` (typed cursors, `sources`).
- `rename_partition_command` (optional version, versioned edit).
- `xc_translate_document_command` + dispatch arm + `executeCommandProvider` entry `tcl-lsp.xcTranslateDocument`.
- tests: `irules_document_analysis` helper, 2 XC tests made async over a real analysis, 3 new tests (`xc_findings_refuse_an_analysis_of_other_source`, `xc_translate_document_uses_the_current_open_document`, `dialect_for_open_reads_the_decoded_uri_basename`).

`rust/tcl-lsp-core`: `hover.rs` (`EventRelease`, `append_command_event_metadata`/`append_valid_events`/`valid_events` signatures, tests), `hover/original_registry.rs` (one call), `code_actions.rs` (`bigip_code_actions` partition only), `semantic_tokens.rs` (`bigip_conf_entries` body source only), `bigip.rs`, `refactor/datagroup.rs`, `ilx_navigation.rs`; tests `bigip_code_actions.rs`, `semantic_tokens.rs` (one new test each).

Other: `rust/tcl-lsp-server/tests/e2e/commands.rs` (advertised list), `docs/kcs/features/kcs-feature-xc-translation.md`, `docs/kcs/features/kcs-feature-irule-extraction.md`, and the VS Code test-count gates `editors/vscode/test-partitions.json` (+`bigipSourceActions.test.js` in partition 2; single-root identities 999→1011, passed 998→1010), `scripts/dev/test-vscode-test-partitions.sh` (108→109 files, counts), `scripts/dev/verify-vscode-test-partitions.mjs`, `scripts/dev/verify-vscode-test-results.mjs` (counts). The +12 is 10 tests in the new file and 2 in `iruleSkeleton.test.ts`; it assumes the 999/998 baseline was accurate — confirm on the first hosted run.

## Measurements

Shared tree: Sol-48/Sol-49/Luna-41 were editing tcl-compiler, tcl-spectcl,
tcl-registry and tcl-lsp-db throughout, so every Rust run here includes their
uncommitted state. A clean-HEAD baseline build was not possible (disk at
1.3–2.6 GB free; reverting shared tcl-lsp-core files to build a baseline would
have changed Luna-41's builds). Baselines below are therefore the CI-5201
failure lists (ci-5201/*.txt) or code-path reasoning, stated per row.

| Target | Before | After | Passed-before→failed-after |
|---|---|---|---|
| `cargo test -p f5-xc --lib` | 28 tests; 5 fail in CI-5201 (report×2, translator getter×2, supplied_registry) | 30 tests: 25 PASS, 5 FAIL — exactly the CI-5201 five; 2 new PASS | 0 |
| `cargo test -p tcl-bigip --lib` | 317 | 318: 315 PASS, 3 FAIL (`lint::irule_empty_when_and_deprecated`, `lint::unknown_event_fires`, `tls::factory_profile_materialises_available_default_certificate`) | 0 attributable: `lint.rs`/`tls.rs` do not reach `conf_tokens` (the only tcl-bigip file I changed; no other module calls it) |
| `cargo test -p tcl-irules --test walker` | 1 test, FAIL in CI-5201 | 1 test, FAIL at the unchanged golden comparison (line 85); new census passes | 0 |
| `cargo test -p tcl-lsp-core --lib -- hover bigip datagroup ilx_navigation semantic_tokens code_actions` | not measurable (see above) | 613 run: 478 PASS, 135 FAIL. New tests: 4 PASS, 1 FAIL (ILX fixture flaw, fixed, not re-run) | Reviewed every failure in a touched module: hover OO/regex/class/special-var, semantic-token OO/snit/regex/receiver, code_actions package/inline/check-action, completion version, bigip folding, original_registry conditional — none reaches the code I changed (event metadata, bigip_conf_entries, bigip_code_actions, datagroup rendering); the matching integration cases are in CI-5201 (`hover_on_builtin_command_shows_summary_and_synopsis`, `nested_if_inside_when_body_resolves_at_cursor`, folding). `datagroup::inside_when_body` = the CI-5201 nested-`when` failure; `extract_datagroup_surfaces_and_carries_definition` fails in `code_actions` gating while every analysis-route datagroup rendering test passes. Not proven by a baseline run. |
| tcl-lsp-core integration (`bigip_code_actions`, `semantic_tokens`, `file_ops_bigip_residual`) | — | compiled (`cargo check --tests`), not executed | — |
| `cargo check -p tcl-lsp-server --tests` | — | PASS, 0 warnings (also f5-xc, tcl-lsp-core, tcl-bigip, tcl-irules) | — |
| tcl-lsp-server unit/e2e tests | — | not executed (the 67k-line test binary did not fit the disk budget) | — |
| VS Code: tsc / prettier / bundles / partition gate | tsc FAIL (3×TS6133), prettier FAIL | PASS | host suites not run |

fmt: rustfmt clean on every changed Rust file; prettier clean. `cargo check
--tests` is warning-free for every changed crate. Clippy was started on the
changed crates and stopped at the coordinator's disk request before it
finished — not run; no new `#[allow]` was added.

## Next actions

1. Re-run `tcl-lsp-core --lib ilx_navigation` (fixture fix) and the server tests
   (`xc_`, `dialect_for_open_reads_the_decoded_uri_basename`,
   `code_action_handler_bigip_conf_offers_partition_rename`, e2e `commands`,
   `bigip`) when disk allows.
2. Hosted VS Code partitions: confirm 1011/1010 identities and the 10+2 new tests.
3. Coordinate with Sol-49 for the `when` partial layout (completion unit) and with
   the xtask owner for the Zed/TextMate projection units.
