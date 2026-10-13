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
| Shared naming | Current/path/root owner serves Syntax, source metadata, Compiler, Runtime API, Runtime and VM.133 Syntax controls, C command3/Jim5, Runtime115, VM209 and7 publication/retained-holder controls pass. Original braced-source mapping passes25 Syntax/6 Registry controls with original C5/Jim LF/CRLF evidence. | Close every adapter in the [consumer inventory](consumer-inventory.md) and [outstanding audit](outstanding-consumer-audit.md); pure owner tests do not certify consumers. |
| Registry, Compiler selection and VM queries | Exact descriptors, original registration cache labels, typed role hooks and shared query geometry are integrated or isolated for review. Earlier Registry129 and descriptor/cache83 controls pass. Corrected VM namespace/query capture passes13 selectors, including60 C5 ASCII namespace and48 C5/Jim query rows. | The latest49 intended controls all pass across two honest frozen captures:43 first-capture passes with6 test-import build blocks, followed by6 exact Compiler passes after the test-only public import correction. Provider-specific Which parsing, Jim actual-object candidates, named-role/layout separation and ensemble selected-worker repairs are integrated. Fresh combined Main gates remain pending. |
| #2286 and source reads | Conditional original procedure read-flow carrier is integrated. Original C5 factory assertions pass on unchanged instrumented source in two captures. Isolated production35 controls pass31/fail4; Main production run was incomplete. All C5 Native source records are present. | Finish original conditional-body source selection. The Jim formal control failed at a catalog-alias unwrap; central environment ingress correction is prepared. Three other measured producer/layout failures remain. Preserve execution/frame guards and all original assertions. |
| Callback catalogue and database/server | Shared original conditional member/receiver catalogue is integrated. Frozen Compiler4/Core12 controls pass, including all4 unchanged callback assertions and hosted/export/colon controls. DB8/server8 fresh controls pass. | Finish remaining source-scope and class-navigation consumers; source catalogue acceptance issues no Native execution authority. |
| LSP | Definition, completion, workspace variable coordinates and rename consume shared LogicalVariableScopeAdvice. Root/current/unknown-caller distinction, exact source image/configuration, child-interpreter refusal and unknown namespace coordinates are integrated. | Combined Core capture01 compiled zero tests because the new measurement insertion duplicated two fixture definitions. Duplicate removal preserves assertions. Frozen capture02 executes22 selectors:15 pass/7 fail. The12 existing callback/vendor/export/colon controls still pass; three new source-scope controls pass, six fail, and the Native class-fallback measurement exposes deleted-head navigation. Editor/extension inventory traversal is under audit. |
| Runtime variables and namespaces | Shared ordinary-variable bases, Jim global/cache handling, counted namespace geometry and destination selection are integrated. Ordinary variables16 controls pass; namespace geometry5 selectors include60 C5 rows and the48 C5/Jim query comparisons. | Retained aliases, generations, imports, traces and original publication contexts still require consumer closure. |
| Runtime publication and queries | Bootstrap/publication consumers use shared WrittenCommandParts and version-specific NativeOoVarnameLookup. Publication30 selectors pass29/fail1; the import-source failure reproduces on pre-change inputs. Runtime Which patch is integrated and sealed:5 selectors pass,1 existing info-commands failure reproduces unchanged. | Execute fresh combined Which/Runtime adapters with the integrated Registry helpers. Finish rename-window consumers by genuine generation/slot, without reparsing generated reports. Command-trace add/remove/info use the one checked original selection. Rename source selection now retains the actual token placement; the generated-report window map is removed. New Runtime controls await execution. |
| Bytecode, WASM and procedure identity | Original evaluated argv transport is integrated, with Runtime6/API18 controls passing. General Runtime-linked WASM controls pass, including two newer renamed-definer/materialised-name positives. Actual compiled-entry/table positives still refuse before linking. | Retain genuine source publication independently of physical compiler permission; migrate every procedure map, interprocedural/report/DB join to typed source identity, preserving same-display bodies (#1778). Native CPU/GPU/FPGA capability declarations have no current emitter found. |
| BPF | Production BPF consumers use shared exact CommandSpec descriptor keys and typed BpfOpKind/BpfDeclKind. Frozen isolated IR60/codegen28 controls pass; all attempts and source seals are retained under [BPF results](../../../experiments/name-handling/results/codegen-bpf-source-selection/). | Fresh combined workspace gates remain. The procedure identity migration must include this backend. |
| Tooling, SpecTcl and AI | Shared SimulatorMockName serves generator/pure-function consumers and exact authored mock publication; Registry owns native argument-role hook identity parsing. CLI pack presence uses shared literal glob projection.48 C5/Jim presence rows pass. | Run actual Cargo consumer controls and full simulator generation/drift checks. Snapshot/reporting adapter must retain original root words and context: real C5/Jim evidence proves report readdressing and universal qualified-prefix escaping invalid. EngineWasm root replay is prepared; authentic callback-context transport is being implemented separately from live receipt authority. Remaining AI/tool/editor surfaces stay in the audit. |
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
minutes, even while red. The current checkpoint is18:28 UTC on13 October2026;
next tracking checkpoint18:43 UTC and next hourly progress report19:06 UTC.
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

Parallel acceptance builds restore omitted sparse fixture/C inputs from exact
baseline blobs before fresh snapshots. A source-flow Compiler build was killed
by SIGKILL before tests; heavy Compiler/Core builds now use a sequential lease
and jobs1/test debug0. Code edits remain parallel. Runtime ordinary variable
controls have started after complete source/compiled-input closure validation.
No new completed VM, DB/server, source-flow or codegen acceptance is claimed.

[Runtime ordinary variable adoption](../../../experiments/name-handling/results/runtime-variable-adoption-20261013/validation.md)
is integrated:16 exact controls pass, including8 scalar,5 Jim/cache/activation,
2 ordinary rename and48 C5/Jim query comparisons. All9,154 captured sources and
4,907 compiled inputs match with no gaps; the3 owned variable files and inherited
rename-destination file match the tested hashes in the primary checkout. Further
publication/bootstrap and TclOO holder-selection consumers have separate owners.

The [outstanding consumer audit](outstanding-consumer-audit.md) reconciles63
package boundaries and330 current-hash/function-attributed LSP candidates, routing
36 bounded semantic handoffs. It establishes no exhaustive acceptance. Source
procedure identities still need propagation into compiler maps, DB cascades and
reporting schemas. Root prepares the exact simulator mock table and SpecTcl/CLI
tooling boundaries; Registry/VM, compiler descriptors, LSP and codegen are active
in isolated checkouts. Source-flow tracing terminates its34-node graph and
completes C8.4/8.5/8.6 original assertions before reaching C9.0; no infinite
worklist defect is established. Full original-program completion remains pending.

Two genuine linked WASM positives now pass on the unchanged isolated baseline:
renamed stock definer and constant materialised name. Native compiled-entry/table
authority remains separate. The instrumented source-flow capture passes31/35
controls, including the unchanged original #2286 assertion across C5;4 existing
flow/formal controls fail. Production trace owners are restored; exact first
producer diagnosis and fresh uninstrumented acceptance remain required. The
callback catalogue/fixtures are tracked at7aa200ed1; their isolated-checkout
absence was sparse materialisation, not missing Git content.

Root tooling consumers now use the shared simulator mock publication owner and
exact Tcl table, with authored C5/Jim observations and one pure Rust control.
Full generator union/regeneration/check and VM harness acceptance are pending.
SpecTcl native count/layout hook IDs use one Registry metadata resolver; fresh
Registry/loader controls are pending. CLI SpecTest presence delegates a shared quoted literal
global command-inventory pattern and original auto_load input; reporting
variable filters remain separately audited. Full workspace gates remain red.

[SpecTest actual presence observations](../../../experiments/name-handling/results/spec-test-exact-presence-20261013/README.md)
pass48 current extracted-helper rows across C5/Jim plus one pure glob control.
The rejected namespace-query recipe is retained: Jim namespace.which reports
no name for a lone-colon command that direct invocation can execute. A full
CLI/package regression is implemented but unexecuted. Codegen isolated04/05
receipts are retained in the primary checkout;05 passes two strict Runtime-linked
positives with complete selected-local inputs and explicit post-only SDK inputs.
Further Runtime publication/TclOO owner validation initially passes29/30; the
retained/recreated-import failure also reproduces before that migration and
without the inherited destination hunk. It remains an unresolved naming control.

Runtime publication/TclOO7-file migration and LSP shared scope/primitive/partial
array adapters are integrated. [Runtime receipt](../../../experiments/name-handling/results/runtime-publication-adoption-20261013/validation.md)
seals29/30 exact controls, with the retained/recreated-import failure reproduced
before the migration and without the destination hunk. Six new public LSP
controls and actual Logical array-completion ingress validation remain pending.
C5 frame/scope probes and distinct Jim refusals are retained in the source-scope
pattern; no C execution expectation transfers to Jim. BPF owner reports88
controls passing (IR60/eBPF28); source integration and receipt retention are next.

Registry/VM acceptance count audit corrects a zero-test selector: revised46
processes contain36 actual passes,4 assertion failures,5 Compiler build-blocked
selectors and1 zero-test attempt. The corrected VM13 group passes. Registry
opaqueWhich/named operand layout and C84/Jim query differences remain measured
repairs; no zero-test process establishes acceptance. Full gates remain pending.

Current consumer integration checkpoint (2026-10-13 17:58 UTC): BPF descriptor consumers are integrated with scoped IR60/codegen28 passes retained. Logical variable scope admission now declines child-interpreter roots and retains unknown-namespace body tables without inventing a namespace coordinate; the additional public controls await the combined Main build. Runtime Which is sealed and awaits the shared version-specific query helpers. Original C5/latest-Jim snapshot-pattern evidence is retained: qualified prefixes have different purposes across queries; neither universal escaping nor qualified report readdressing is valid. The accidentally copied Runtime test executable is removed from repository files; reproducible source, logs and artifact hash remain. Formatting and combined Main LSP/tooling validation are next. All-consumer acceptance and PR gates remain open.

Current integration: original command trace/rename consumers use retained generations and actual physical slots; their new Runtime checks are queued. Original C3 direct rename controls pass; C84/C85 fixture refusal and Jim trace unavailability remain explicit. Fresh isolated Registry/query capture22 exact controls pass; later Compiler test import build blockage is retained, remaining VM/Compiler results pending. #2286 SourceReadScope/body catalogue work continues under a fresh source_read_flow agent, with no source loss or new acceptance claim. The measured Native class fallback repair is prepared; remaining Logical scope producers are under direct audit.

Checkpoint18:28 UTC: shared Registry/Compiler/VM query and metadata adapters integrated;49 unique intended controls accepted across two frozen isolated receipts. New RuntimeWhich compiler adapter integrated with5passes/1reproducedbaselinefailure. The publication geometry carrier, original read-qualification consumers and measured class gate are integrated but their new Main controls remain pending. SourceReadScope one-owner diagnostic build is running isolated; Root Scope trace and fresh replay/Runtime validation are queued. Typed procedure ID primary propagation and snapshot coordinate implementation continue in parallel. Every validation state is scoped; no allconsumer acceptance claimed.
