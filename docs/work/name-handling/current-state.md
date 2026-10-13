# Current name-handling state

Temporary coordination document on `work`; inspect Git for the current commit.
Remove this directory and its references before the PR. Root owns integration,
issue status and source/build leases. Receipts certify their captured inputs.

## Goal

Centralise name lookup in one reusable owner and route every consumer through it.
Fix #2286 and issues entirely caused by naming: representation, context, lookup
order, binding roles, namespace identity, lifetime and consumer disagreement.
All surfaces are included: parser, registry, compiler and analyses, LSP, database,
middleware, runtimes, every codegen target, tooling, AI and editor integrations.
General runtime, numeric, clock or OO compatibility is only a bounded dependency
when it prevents a naming control from running.

Read the [owner contract](../../design/contracts/name-handling.md),
[consumer integrations](../../design/analysis/name-resolution-proofs/consumer-integrations.md),
[issue map](issues.md), [consumer inventory](consumer-inventory.md) and
[finishing plan](plan.md) and [structured procedure plan](structured-procedure-plan.md). Source advice, metadata, live binding and edit authority
remain distinct. Preserve original inputs, selected protocol, context, liveness
and store currency; refuse missing, foreign, stale or dynamic evidence.

## Current acceptance and gaps

Consumer migration remains incomplete. Counts below cover exact captured controls,
not the proportion of all consumers migrated. Integrated, prepared and validated
changes are separate states. Root owns Main integration and build leases; agents
implement independently while the consumer audit continues.

| Consumer group | Current implementation and evidence | Remaining implementation or validation |
| --- | --- | --- |
| Shared naming | A fresh Main capture passes139 exact Syntax naming controls; its harness-selected source inventory is stable, without a complete compiled-closure claim. Current/path/root owner serves Syntax, source metadata, Compiler, Runtime API, Runtime and VM.133 Syntax controls, C command3/Jim5, Runtime115, VM209 and7 publication/retained-holder controls pass. Original braced-source mapping passes25 Syntax/6 Registry controls with original C5/Jim LF/CRLF evidence. | Close every adapter in the [consumer inventory](consumer-inventory.md) and [outstanding audit](outstanding-consumer-audit.md); pure owner tests do not certify consumers. |
| Registry, Compiler selection and VM queries | Exact descriptors, original registration cache labels, typed role hooks and shared query geometry are integrated or isolated for review. Earlier Registry129 and descriptor/cache83 controls pass. Corrected VM namespace/query capture passes13 selectors, including60 C5 ASCII namespace and48 C5/Jim query rows. | The latest49 intended controls all pass across two honest frozen captures:43 first-capture passes with6 test-import build blocks, followed by6 exact Compiler passes after the test-only public import correction. Provider-specific Which parsing, Jim actual-object candidates, named-role/layout separation and ensemble selected-worker repairs are integrated. Fresh combined Main gates remain pending. Distinct Jim origin sequencer and VM caller/root adapters are integrated with original42 rows/provider retained; its original16-selector plan remains unexecuted. The measured C84 alias-registration owner and VM actual-token adapter are authored separately, with17 updated selectors queued. |
| #2286 and source reads | Conditional original procedure read-flow carrier is integrated. Original C5 factory assertions pass on unchanged instrumented source in two captures. Isolated production35 controls pass31/fail4; Main production run was incomplete. All C5 Native source records are present. | Finish original conditional-body source selection. The Jim formal control failed at a catalog-alias unwrap; central environment ingress correction is prepared. Main diagnostic4 controls execute1pass/3fail. Original source rows and reached metadata are present; repeated-iteration read selection loses source advice through an opaque lookup snapshot. The shared SourceReadScope/CompletePrefix source-purpose producer is authored in11 isolated owners, unbuilt, with38 selectors. Two fresh builds stop before tests at genuine issuer-type and documentation errors; corrections preserve both original issuer types and remove the unused method. Fifth frozen build and source/dependency closure pass;38 controls finish33pass/4assertionfail/1timeout. Original35=32pass/2assertionfail/1timeout; new3=1pass/2fail. Early results pass the original loop missing-read assertions but expose a runtime-applicability purpose leak, plus remaining Logical source-role and body-retention failures. Measured Logical ingress, exact bounded-vector, source-only applicability and immutable-seed memo repairs are prepared for fresh06 capture, unbuilt. Three other measured producer/layout failures remain. Preserve execution/frame guards and all original assertions. |
| Callback catalogue and database/server | Shared original conditional member/receiver catalogue is integrated. Frozen Compiler4/Core12 controls pass, including all4 unchanged callback assertions and hosted/export/colon controls. DB8/server8 fresh controls pass. | Finish remaining source-scope and class-navigation consumers; source catalogue acceptance issues no Native execution authority. |
| LSP | Definition, completion, workspace variable coordinates and rename consume shared LogicalVariableScopeAdvice. Root/current/unknown-caller distinction, exact source image/configuration, child-interpreter refusal and unknown namespace coordinates are integrated. | Combined Core capture01 compiled zero tests because the new measurement insertion duplicated two fixture definitions. Duplicate removal preserves assertions. Frozen capture02 executes22 selectors:15 pass/7 fail. The12 existing callback/vendor/export/colon controls still pass; three new source-scope controls pass, six fail, and the Native class-fallback measurement exposes deleted-head navigation. Fresh Main diagnostic10 controls execute3pass/7fail. Qualified-reference, checked original-write and current source-publication consumers are integrated in5 owners. Logical read recording and Core definition now share declaration-table selection; namespace-variable rename also checks the retained consumer image. Fresh13 original repair controls plus separate read-recording/rename controls remain unexecuted. Editor/extension inventory traversal is under audit. |
| Runtime variables and namespaces | Shared ordinary-variable bases, Jim global/cache handling, counted namespace geometry and destination selection are integrated. Ordinary variables16 controls pass; namespace geometry5 selectors include60 C5 rows and the48 C5/Jim query comparisons. | Retained aliases, generations, imports, traces and original publication contexts still require consumer closure. |
| Runtime publication and queries | Bootstrap/publication consumers use shared WrittenCommandParts and version-specific NativeOoVarnameLookup. Publication30 selectors pass29/fail1; the import-source failure reproduces on pre-change inputs. Runtime Which patch is integrated and sealed:5 selectors pass,1 existing info-commands failure reproduces unchanged. | Execute fresh combined Which/Runtime adapters with the integrated Registry helpers. Finish rename-window consumers by genuine generation/slot, without reparsing generated reports. Command-trace add/remove/info use the one checked original selection. Rename source selection now retains the actual token placement; the generated-report window map is removed. New Runtime controls await execution. |
| Bytecode, WASM and procedure identity | Original evaluated argv transport is integrated, with Runtime6/API18 controls passing. General Runtime-linked WASM controls pass, including two newer renamed-definer/materialised-name positives. Actual compiled-entry/table positives still refuse before linking. | Retain genuine source publication independently of physical compiler permission; migrate every procedure map, interprocedural/report/DB join to typed source identity, preserving same-display bodies (#1778). The primary ID slice is authored; first typecheck stops at2 Bytecode formatter errors before reaching Compiler. A second check stops at an isolated omission of Main’s shared glob helper; exact ingress is corrected. Third check reaches Compiler and reports67 remaining type errors. These are active typed-identity consumer repairs, with no primary acceptance. Native CPU/GPU/FPGA capability declarations have no current emitter found. |
| BPF | Production BPF consumers use shared exact CommandSpec descriptor keys and typed BpfOpKind/BpfDeclKind. Frozen isolated IR60/codegen28 controls pass; all attempts and source seals are retained under [BPF results](../../../experiments/name-handling/results/codegen-bpf-source-selection/). | Fresh combined workspace gates remain. The procedure identity migration must include this backend. |
| Tooling, SpecTcl and AI | Shared SimulatorMockName serves generator/pure-function consumers and exact authored mock publication; Registry owns native argument-role hook identity parsing. CLI pack presence uses provider-specific shared literal inventory plans.132 C5/Jim source-form observations pass. Shared original-coordinate snapshot support passes16 source-form controls. | Run actual Cargo consumer controls and full simulator generation/drift checks. CLI compiled controls and generator drift remain unexecuted. EngineWasm replay retains original bytes and authentic callback contexts; initial isolated Runtime23 executes9pass/14fail, followed by genuine ABI/compiled10pass and corrected query31 executing29pass/2fail. Both remaining failures expose the same C84 alias rename/replacement mismatch. Engine host/guest controls remain unexecuted. Extension-host inventory still reconstructs addresses from non-injective reports and needs migration. Remaining AI/tool/editor surfaces stay in the audit. |
| Oracle conformance | Original C8.4–9.1 release trees/libraries and current Jim source are provisioned. Original upstream proc/rename suites complete without failures; seven upstream admission controls pass. | Every new test must cite real scoped observations. Use the original matching init.tcl/tcltest.tcl when using Tcltest. F5 observations remain separately scoped below. |
| PR gates | Current Main formatting has been applied; no all-consumer, full rust-check/prep-pr, generated drift, resolution-drift or strict-proof acceptance. Previous strict checker reports1739 errors across evidence/scrub currency, annotations, docs and census paths. | Finish consumers first, review affected proof semantics and narrow bindings, then whole-repository docs/comment cleanup and every required green gate. Never bulk-refresh proof hashes. |

Receipts retain exact source/artifact identities, actual executed selector counts,
failed attempts and build limitations. The publication experiment's accidentally
copied test executable was removed; its reproducible source, logs and hash remain.

## References and build coordination

Source-built C8.4.20,8.5.19,8.6.18,9.0.4,9.1.0 are provisioned with matching
original libraries. Jim5bac7c99ad65864c87da513e22e2f01703fa4e03 reports
0.84-9-g5bac7c9; upstream HEAD was checked on13 October2026 and matches.
Rediscover latest when collecting new Jim results. Record flags and extensions.
Rust stable1.99.0 and repository-pinned Ruff0.16.10 are available. Source
`/tmp/name-env.sh` and select an explicitly leased target. Only one Cargo build
uses a cache at a time; source mutations must not invalidate another capture.
WASM tools are provisioned from verified upstream assets. No worktrees share a
Cargo target. Heavy Compiler/Core builds are serial under explicit leases; source
changes wait for capture completion. Main uses tmp/name-target-names, agents use
separate isolated targets. Build caches are expendable; sealed sources/results
and oracle provenance are retained. Sparse fixtures are checked against Git
before claiming an input absent.

Tracking documents and coherent development commits/pushes are updated every15
minutes, even while red. The current checkpoint is20:31 UTC on13 October2026;
next tracking checkpoint20:46 UTC and next hourly progress report21:06 UTC.
Root checks the clock between bounded operations. The last checkpoint's brief
integration delay is explicit; no missed check is represented as executed.

## F5

Consume [existing answers](../../../experiments/name-handling/f5/answers/README.md)
and the outstanding-operation audit before preparing new requests. The numbered
snapshot has400 rows:93 observed,244 unreached,63 unavailable. The pattern matrix
separately has108 observed,25 unsupported,267 unreached across50 questions/eight
contexts. Supplements do not promote unreached originals. Hosted fixtures must
use explicit owned publication names and prove the entered frame/lookup context.
Ask the BIG-IP chat only unanswered naming questions and have it commit actual
answer documents to `work`. Never run Tcltest in TMM/MCPD or infer F5 from C/Jim.
Suspended frames, restart/reload and other traffic-unit contexts remain unmeasured.

Preserve the user's [scrub](../../../experiments/name-handling/results/security-material-scrub/README.md)
and benign reproducible oracle observations. Do not restore deleted lab, device,
security or certificate material. Redacted transport hashes are not original
execution seals; a notice is not an executable replay source.

## Integration and final cleanup

Commit and push coherent reviewed batches regularly even while development checks
are red, recording exact failures and unexecuted gates. Before the PR, audit all
authored/generated documentation and all comments, including untouched files,
tests, scripts and templates. Remove stale claims, completed TODOs, abandoned
plans and process narratives; retain minimal current how/why, legal attribution,
experiments and oracle-derived tests. Remove temporary documents/proposals and
references plus the work-specific AGENTS push exception. Format/regenerate and
pass required gates, including any required JetBrains SDK checks. No merge is
authorised.


## Active implementation queue

The priority is moving every consumer to the shared owners. Runtime released its
serial build lease after sealed genuine10/10 replay controls and query31=29pass/2fail. Engine host compilation stops at an unchanged baseline error-completion variant; a raw guest-error transport correction and receiver-root replay guard are prepared but uncompiled. SourceReadScope
38-selector capture currently owns the lease, followed by Jim origin/alias17, LSP scope and
publication repair, revised typed procedure-ID typecheck, and combined Main
CLI/SpecTcl/generator gates. Editing proceeds in separate worktrees; no target is
shared. Root has an unapplied six-owner reporting draft for typed body-ID snapshots, IR rows, assembly source-range joins, diagram traversal and genuine dead-store producer/annotation joins. Primary Compiler typecheck and adapter controls remain pending. Root reviews and integrates bounded owned patches and keeps failed
attempts distinct from acceptance.

The initial Runtime23 capture executes9pass/14fail. The later genuine10
ABI/compiled controls all pass. The13 original public-output controls and18
followups are blocked by the capture host omitting NativeHost numeric/value
environment delegation, not an established production query defect. Complete
real Host delegation is being restored and original inputs/assertions rerun.
String-observer experiments are retained separately and match original oracle
stdout byte for byte; they do not replace the original controls. Jim counted
Engine creation has no selected Native recipe and remains an explicit genuine
refusal, independently of Legacy original holder capture.

The outstanding audit covers63 package boundaries,330 attributed LSP candidates
and36 semantic handoffs. Coverage is not an all-consumer completion certificate.
Root's extracted-procedure root projection and CLI minimiser qualification
filter now delegate the Syntax owner; their combined Cargo gates are pending.

Original command-trace/rename production uses genuine command generations and
physical slots. Actual Runtime acceptance for these Main changes remains queued.
A proposal for optional compiled body entry is still unapplied: genuine selected
definer, source-body identity and current Native permission remain mandatory.

C84 original alias creation retains registration-name-to-actual-token lifetime
independently of command placement. Reusing that name deletes the prior token
even after rename; C85+ keeps separate unique alias tokens. Shared purpose/version
policy and VM/Runtime physical token adapters are being implemented. This is an
active naming issue, not an excluded baseline exception.

The separate Logical read-recorder repair delegates declaration-table eligibility
and Core definition to the same scope owner, with real C5 root/current/caller
observations retained. Two-owner patch and its reference control are integrated,
unexecuted, and remain separate from the integrated13-selector LSP repair set.
Root's typed-ID drafts cover report rows, exact database memo ingresses and
dataflow graph envelopes; all are unapplied pending Compiler typecheck.

Runtime original query/origin adapters and typed replay transport are integrated
with their genuine10/10 and31=29/2 receipts retained. The raw Engine error-options
repair and extra actual-child-receiver guard are uncompiled post-capture changes.
Root reviewed those boundaries; no Engine host/guest acceptance is claimed.
Corrected VM17 capture executes16 tests:10pass/6fail, with1 separate test-only Registry build block. Jim origin and all150 Which observations pass; C aliases stop at pending-rename generation/preflight admission, and cycle storage assertion needs genuine returned-refusal matching. Purpose-specific transaction repair is authored. Scope15 finishes5pass/10fail with stable isolated source. Native class deletion/replacement/future/opaque publication controls and source/domain refusals pass. Logical producers and a Native-input-to-Logical-only rename test ingress fail. Supporting7 existing controls execute3pass/4fail, with literal apply and two-hop refusal passing; supplied-value lambda navigation fails. No new source acceptance is inferred from old binaries.

Root has repaired6 measured Compiler ID/name type joins in the isolated checkout,
including explicit actual-top-level selection and ID-authenticated event effects.
These remain uncompiled, with two temporary unique-display summary adapters
awaiting the strict analysis-ID maps. Main3 array-to-slice fixes preserve original
bytes and resolve the measured VM adapter conversion build failure.

Next queue: Scope15 completion, Lookup revised Compiler check, SourceReadScope06,
Runtime trace/alias/import/Engine controls, corrected VM aliases, and combined
Main tooling/generator gates. The extension inventory and SpecProbe/Studio
mixed-purpose consumers have bounded proposals, not acceptance.

SourceReadScope06 no-run and full original-source/dependency/library closure pass;
38 direct controls are running. First7 execute6pass/1fail: all3 new source
positive/adversarial controls, both original loop/zero-trip controls and Jim
formal controls pass. Source-only applicability now remains distinct from runtime
reachability. Missing original Unit::f body persists. Full result is pending.
Lookup fast05 owns the heavy lease;04 reached Compiler with1 remaining actual
resolved-target type mismatch, repaired through the shared allocation owner.

Extension-host structured command inventory is integrated, including shared
root-operand geometry and actual lookup slot/generation checks. Main RuntimeAPI
check passes; Runtime/Engine consumer controls remain unexecuted. Original
provider-specific root-context observations and Scope15/supporting receipts are
retained. SpecProbe/Studio mixed-purpose repairs are isolated and unvalidated.
