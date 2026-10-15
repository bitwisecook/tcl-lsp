# Current centralised naming work

Development source `6d7081b73` is pushed on `work`; the next reviewed tranche is being checkpointed. The isolated validation worktree remains frozen at `8df9a91699c0907ae3e0aefa1b10396feb967e8f`. Source acceptance always names that exact commit. Incoming `rust` was inspected through `d0a644c69` without merging; concurrent upstream changes still require integration.

## Latest sealed ordinary result

F built all14 ordinary test ELFs successfully with one Cargo job (exit0, errors[], tracked source drift{}). Its complete selected replay ran992 controls: **677 PASS,314 FAIL,1 TIMEOUT**. All967 first-batch controls remain; exact comparison gives10 gains, **1 prior-PASS regression**,0 drops. The25 added controls give15PASS/10FAIL. Every selected scope retains original budgets, exact single-test checks, nonempty filters, unchanged source/ELFs and no missing selectors. Original#2286 ran first and PASS71.317924863s under its unchanged180-second budget; the issue remains OPEN.

| Scope | PASS | FAIL | TIMEOUT |
| --- | ---: | ---: | ---: |
| Compiler502 |295|206|1|
| Core251 |169|82|0|
| DB82 |70|12|0|
| iRules20 |19|1|0|
| MCP43 |33|10|0|
| Server15 |12|3|0|
| Registry15 |15|0|0|
| Syntax7 / Source42 / Studio3 / SpecTcl1 / Lexer11 |64|0|0|

The MCP regression is `irule_gen::tests::scaffold_deduplicates_events_and_sets_up_registry_classified_pools`; repair is assigned to LSP. All three predecessor iRules object reachability/tombstone/literal-candidate failures now pass. Four new Compiler collection controls fail; a separately captured original-ELF diagnostic identifies upfront source validation of three explicitly non-invocation markers as an additional refusal. The narrow shared-predicate correction is source reviewed but unexecuted. New Logical source/formal/frame positives and two original completion controls remain failing; no guard or assertion is relaxed.

Exact F channels and comparison are in `handoff/validation-8df-batch-f.tar.gz`; live capture `/workspace/name-session/replay-f-*`, comparison `ordinary-f-comparison.json`. No result accepts later605 source. B/C/D/E build failures retain their independent channels and repairs.

## Native execution and provider limits

F Runtime engine lib+namespace-query preflight builds2ELFs successfully (136.923226s,source{},0tests); full Runtime174 is **sealed171PASS/3FAIL**, errors[] and source/compiled-input/artifact drift maps{}, under its reviewed immutable source rebind. The VM133 original-plan replay is **sealed133PASS/0FAIL**, errors[] and all source/compiled-input/artifact drift maps{}: `/workspace/name-session/vm-f-rebind` and `vm-f-replay133`. It preserves all historical source/provider/artifact/compiled-input guards and all133 original selector bodies/assertions/budgets. VM133 retains all132 predecessor controls, including all six formerly failing controls now PASS, plus the original SharedEmpty control. Portable exact channels are in `handoff/validation-vm-8df-full133.tar.gz`. The three Runtime failures are original variable-inventory keys, namespace deletion and retained Import. The first and third stop at the same missing original variable-key owner before Import semantics are reached; DB owns that authentic producer repair. Do not advance the frozen worktree or share its Cargo target until its recorded target retirement. Both Native captures are now sealed; root retired4.3GiB of compiled artifacts (source/provider unchanged) to prepare an isolated successor build without disk exhaustion.

Original C5 binary/library/init/tcltest pairings are verified at `f-provider-preflight.json`. The sole provisioned untracked validation `tmp` symlink points exactly to `/workspace/tcl-lsp/tmp`; unknown extra paths are not generally admitted. Latest execution-time upstream Jim check11:39:05UTC identifies5bac7c99ad65864c87da513e22e2f01703fa4e03, binary c5b2b0c37c3bad8e97760350460310bde65e74cf9833979fe1c088e4dd2a4891. Original Source-provider capture01 seals211rows/7successful stages/27helper bodies, original namespace79/79 and unchanged89provider files. Static and raw helpers have four genuine body differences; byte offsets derived from original source are not physical runtime receipts.

## Reviewed implementation awaiting current-source execution

Next source integrates original manufacturer/factory/Registry-definer owners; original nested index and lexical header/return children; shared manifest declaration/append construction; actual Runtime nested getter holders and static Source bodies. Both Runtime and VM attach original Import provenance before publication and fail closed at the actual owned procedure entry until authentic finite traversal receipts exist. Rename retains that provenance; user replacements and copied source do not acquire it. This is an explicit unfinished positive path, not an implemented safe Import traversal. Compiler factory argument/layout joins, general Logical rename admission, source-only conditional body inventories, helper source-publication proposals, taint read ownership, UCS fidelity, every remaining host/backend/editor/generated consumer and all unexamined rows remain work.

The requested simplified private architecture/API Page is complete, including six diagrams, dialect, SSA/SCCP and array/dictionary/type explanations. The private link is not published in repository documentation. Studio web123/123 and TypeScript checks concern their captured earlier source; later Rust/DSL/generated changes need fresh drift validation. Independent audit now retains449 exact issue records (97OPEN/352CLOSED),329 explicit prior state rows with no drift and13 additional closed naming omissions; unclassified search hits remain unfinished. No issue was closed. F5 pending questions and unreached execution contexts remain unaccepted.

Formatter88, integration55/15, full Native/host suites, normal owner/proof CLI, generated drift, `make rust-check` and `make prep-pr` are not green or remain unrun. Development pushes are authorised; no PR or merge is justified. Root owns Git/shared tracking/build leases.

---

The following sections retain historical source/capture bounds; the snapshot above takes precedence for current activity.

# Current name-handling state

## Active implementation

The reviewed first migration batch is published as `551f3b2c1` on `work`, following requested checkpoint `2d456966b`. Root inspected incoming `rust` through `8c2138282` without merging it. The provisioned clean base `1890e99cf` is preserved locally; no untracked VSCode cache was present. All ten portable packets match `packets.json` and `SHA256SUMS`.

The second reviewed batch implements authentic Logical local/root occurrence witnesses, conservative shared dynamic-name patterns, occupancy-facet receipt merging, external workspace candidate selection and complete supplied header barriers. It also implements configured iRules body roles/event roots, Runtime original Import guards and exact/count-prefix selection, VM canonical original-object/getter transport and byte-preserving WASM Document ingress, plus Studio source-encoding provenance. The second batch is published as `a3e3d7949`; Rust acceptance for it is pending. The original controls and budgets remain.

## Fresh validation of frozen `551f3b2c1`

The isolated `/workspace/name-validation` worktree built all 14 ordinary test binaries with one Cargo job in 753.794629 seconds: exit0, errors[], zero tests during build, complete tracked source drift{}. The earlier setup attempt incorrectly included Runtime as a workspace member and failed exit101 with zero artifacts/tests; retained at `/workspace/name-session/batch-a-build`. Runtime is a separate Cargo root and requires its own build.

Compiler485 retains every ordinary272 Compiler480 selector and adds five controls; original #2286 ran first and passed in68.977646515 seconds under its unchanged180-second budget. Both readonly Authored positive and missing/foreign/mismatched-owner controls now PASS. The complete selected replay is sealed:967 controls,653 PASS/313 FAIL/1 TIMEOUT. All956 ordinary272 controls are retained, with3 improvements,0 prior-PASS regressions and0 drops. The11 added controls give7 PASS/4 FAIL. All twelve scopes retain source drift{}, unchanged ELFs and no missing/empty filters. This validates the frozen source only, not the newer active batch. Exact source/artifact captures are `/workspace/name-session/batch-a-build-ordinary`, `/workspace/name-session/replay-a-compiler` and `/workspace/name-session/replay-a-*`.

Portable exact build/replay channels and predecessor comparison are retained in `handoff/validation-551f-batch-a.tar.gz`. Compiler284/485, Core167/250, DB66/78, iRules16/19, MCP32/43, Server12/15; other selected scopes pass except Registry12/13. Two added source-occupancy controls fail. The minifier added six-provider control fails at Jim profile fixture ingress and the added Registry reference-body control has an incorrect Tcl9 expectation; both fixture defects were corrected in published source, assertions for predecessor controls unchanged. Runtime separate-root build failed E0277 in a new test assertion (0tests); its slice typing correction is in the active batch.

Studio web tests pass123/123 with npm12.2.0; both complete TypeScript projects typecheck. The previously exposed receiver-editor callback signature is corrected. Studio Rust marshalling controls remain unexecuted. These results concern active Studio source, independently of the frozen ordinary replay.

Rust1.99.0 is provisioned through `rustup`. All five pinned C releases and current upstream Jim `5bac7c99ad65864c87da513e22e2f01703fa4e03` were built using repository owners. Fresh VM Jim canonical SDK capture records80 rows and the exact original namespace suite79/79PASS; it supplies no Rust VM acceptance. New C source-name geometry and colon-boundary experiments preserve actual differing release answers, original init pairing, failed setup separately and exact raw channels. Runtime174/VM133 full plans, formatter88/integration55/15 annexes and required `make rust-check`/`make prep-pr` remain unexecuted for current source. #2286 remains OPEN pending complete acceptance.

## Third checkpoint source

The active tranche retains genuine Logical formal topology/cursor frames, configured bound substitutions, complete factory source-owner projection, typed list-loop source/CFG correspondence and runtime call completion dispatch. iRules nominal hazards and collection-flow consumers now use original source owners; composite/index/head child coverage and missing AssignValue carriers remain unfinished. Runtime/VM add distinct actual namespace table/cache/public-arity stages and VM typed Parent physical result handling. Specials, mutable diagnostic lsort and authentic VM helper installation remain unfinished. CLI transform/compile consumers now require original faithful decode reports before any output. These source peers are bounded, not execution acceptance. Original two Recorder completion assertions remain unchanged with a live producer-transport obligation.

## Successor build feedback

The isolated `a9fca77c1` ordinary build failed101 with4 generic error-type diagnostics in Runtime cache-install closures,0 test ELFs and0 tests, complete source drift{}. Capture: `/workspace/name-session/batch-b-build-ordinary`. Explicit `Result<(), E>` closure annotations repair inference without changing checks/order; successor rebuild is required. The missing libtommath setup warning belongs this failed attempt, not Native acceptance. Original assertions and selectors remain unchanged.

## Remaining implementation

| Owner | Next concrete obligations |
| --- | --- |
| Compiler | Genuine Logical formal/frame and alias-cell source inventories; configured bound-word substitution; remaining actual naming consumers in flow/effects/inference/optimisation/codegen. Preserve ambiguous namespace refusals and Native purposes. |
| DB/Core/Server | Complete supplied factory/token-hint ownership and collision policy; actual external source settlement and typed alias/navigation acceptance. Earlier unknown effects and missing absence premises remain barriers. |
| iRules/MCP | Source-only named mutation hazards for object-candidate withdrawal without fabricated execution tombstones; direct/getter/drop/HTTP flow consumers through selected invocations; actual literal/candidate regressions. |
| Runtime | Authentic original namespace subcommand table/cache/arity producer and mutable Import helper invocation with finite cycle/key/lifetime guards. Source metadata is not original cache ownership. |
| VM | Consume typed Parent result/storage recipe, original subcommand cache/dispatch producer, remaining byte/value/lifetime consumers and registered backends/hosts. Current-holder byte success is not full physical result parity. |
| Root/tooling/audit | CLI decode provenance before edit/compile artifacts, additional evaluated-name/template consumers, all remaining surfaces and issue/F5 admission. Unexamined consumers remain unfinished. |

The additive [audit](audit-next.md) includes254 exact GitHub issue reads (59OPEN/195CLOSED, zero state changes against explicit previous catalogue states), current source peers and further genuine consumers. Finite330 bindings remain historical source238 review only, not a current source or migration census. F5 unanswered/unreached operations retain their independent context/provider limits. Root owns Git, shared tracking and build leases. Development pushes remain authorised before green gates; no PR or merge is justified.

## Historical measured scopes

The following retained source238/prior captures are unchanged historical evidence, not acceptance of the active source.

## Actual validation

| Captured scope | Actual result | Limit |
| --- | --- | --- |
| Ordinary265, source221 |929 controls:619 PASS,309 FAIL,1 TIMEOUT |All eleven scopes sealed; unchanged captured source/binaries, exact single runs, no missing/empty filters. All258860 retained,9 gains,0 prior-PASS regressions/drops;69 additions37 PASS/32 FAIL. |
| Compiler265 |478:277 PASS,200 FAIL,1 TIMEOUT |Original#2286 first PASS70.901856808s under unchanged180s. Sole timeout: `optimiser::propagation::tests::depth_exhaustion_blocks_dynamic_name_load_forwarding`,180.051965968s. |
| Core265 / DB265 |243:161 PASS/82 FAIL;76:63 PASS/13 FAIL |Typed declaration/alias/workspace and positive Logical catalogue/header/unknown-effect premises remain live work. |
| MCP265 / Server265 |43:32 PASS/11 FAIL;15:12 PASS/3 FAIL |Later source238 IRules/MCP object controls remain failing; see272 below. |
| Other ordinary265 scopes |Registry12/12; Lexer11/11; Source42/42; Syntax5/5; Studio3/3; SpecTcl1/1 |Selected controls only; Source-unit success does not establish normal owner CLI success. |
| Runtime268, source221 |Build101/E0308;0 executed tests |Delete observer conversion fixed222, not replayed. Empty compiled/artifact maps after failed build do not validate coverage. Latest executed full selected Runtime is249158/158 on source133. |
| VM269, source221 |132:126 PASS/6 FAIL; errors[]; all three drift maps{} |All260131 retained,5 gains,0 prior-PASS regressions/drops,+1 new FAIL. Later fixture corrections and Parent/Text source are unexecuted. |
| Normal owner CLI250 |Exit1;197 diagnostic lines |Owner/checker/route changes since this capture require a fresh normal CLI. Full strict proof gates remain independent. |
| Ordinary271, source238 |Build101; Registry/Compiler rustc SIGKILL9;7 partial ELFs;0 tests; source{} |All956 replay controls were prevented. Empty errors[] does not override the failed build/raw compiler output. |
| Ordinary272, source238 |956: 643 PASS/312 FAIL/1 TIMEOUT; fresh build0/14 ELFs |All265929 retained;1 gain,0 prior-PASS regressions/drops. Selected scope only; Native/F5/fullowner gates independent. |

## Current272 selected scopes

| Scope | PASS | FAIL | TIMEOUT |
| --- | ---: | ---: | ---: |
| compiler (480) | 279 | 200 | 1 |
| core (247) | 165 | 82 | 0 |
| db (76) | 63 | 13 | 0 |
| syntax (7) | 7 | 0 | 0 |
| source (42) | 42 | 0 | 0 |
| studio (3) | 3 | 0 | 0 |
| spectcl (1) | 1 | 0 | 0 |
| registry (12) | 12 | 0 | 0 |
| mcp (43) | 32 | 11 | 0 |
| server (15) | 12 | 3 | 0 |
| lexer (11) | 11 | 0 | 0 |
| irules (19) | 16 | 3 | 0 |

## Latest source and build status

The next source tranche implements original Logical alias target/source-frame/template transport, shared evaluated array geometry and strict original CompoundArray word facts; complete token-source project projections; shared data-group suggestions; actual iRules getter/leaf-flow metadata with full SpecTcl/Studio parity; original namespace parser specials/mutable-sort/result stages and VM selected helper extension/token lifecycle. Bounded source peers pass. The raw-source versus statically loaded nshelper body distinction still requires its separate original-provider capture. Logical source geometry retains unconditional source-applicability obligations; it cannot unlock general rename edits.

Build feedback remains separate: C failed E0308 in a new fixture return annotation (3ELFs/0tests); D was terminated after root caught a wrong worktree target (-15/0tests); E failed3E0502 borrowed-key errors in a new DB test (4ELFs/0tests). Each retained source drift{}. Repairs preserve the actual fixture holder and reissue a current DB coordinate after mutations. F is rebuilding isolated source `8df9a9169` under an explicit worktree-target guard. Exact failed channels are in `handoff/successor-build-feedback.tar.gz`. No successor Rust execution acceptance yet.

The user-requested simplified architecture/API artifact is being prepared separately; implementation continues. F5 UCS/source provenance and remaining taint source/read consumers, structured-header substitutions and a genuine independent rename admission remain unfinished.

## Next source awaiting fresh build

The next reviewed tranche adds exact original constructor argument-value joins, conditional alias source schema agreement for MCP object candidates, actual VM static Source helper construction, shared three-engine compiled-unit proposal spelling, discovered manifest requirement construction and opaque F5 config/UCS source fidelity. Original Compiler/MCP assertions remain unchanged. Runtime Source/Script/Expression origin births are carried by their actual constructors; Import traversal remains refused while frame/iterator/alias receipt transport is implemented. These source peers and added controls are unexecuted. Runtime active `interp.rs` and `interp/native_script.rs` transport edits are excluded from the checkpoint until complete. Query file-provider values, host filesystem byte names and remaining unexamined surfaces remain separate unfinished obligations.

Runtime174 execution-time Jim check11:57:32UTC still identifies5bac7c99ad65864c87da513e22e2f01703fa4e03 with original binary/config/harness checks passing before the build. No final Runtime174 result is sealed yet. The latest original VM133 result is sealed in the portable archive, with no source/compiled-input/artifact drift.

## Current integration and next build

Runtime174 sealed12:11:35UTC on frozen8df:171PASS/3FAIL. All selected controls retain original assertions/budgets and exact source/provider/artifact guards; no timeouts. Portable full channels and reviewed source rebind are `handoff/validation-runtime-8df-full174.tar.gz`. This does not accept later05d source.

Ready source now includes actual Runtime invocation/frame/foreach Source transport (Import remains refused), the parent-horizon class relation API without analyser metadata donation, package init/freeze source rendering, genuine query file-provider provenance and the additional configured-event stage assertions. Active original-key command inventories and positive Import traversal are excluded until source reviewed; no source or provider drift is tolerated in ordinary builds. The next fresh isolated build will bind the new checkpoint and retain every992 predecessor control plus actually listed additions. Scoped CLI/package/F5 controls and owner/proof/generated gates remain separate required runs.

## Fresh G build feedback

The isolated `cb0ebe63e` ordinary build failed101 in82.754041288s with E0599 at `analyser/original_definer.rs:245`: the new actual source-value projection used nonexistent `Vec<u8>::as_bytes`. It produced0testELFs and ran0tests; complete source drift{}. The typed borrow repair preserves the same original byte vector and all source/argument checks. The successor must build afresh before any replay; exact G channels and target retirement record are in `handoff/successor-g-build-feedback.tar.gz`.
