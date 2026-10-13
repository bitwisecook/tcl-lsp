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

| Work | Evidence and current limit | Next action |
| --- | --- | --- |
| Shared naming owners | [12 baseline controls](../../../experiments/name-handling/results/shared-name-owner-current-20261013/README.md): C command3, variable2, namespace2 and Jim5 pass. | Audit every actual consumer; owner vectors alone do not certify adapters. |
| Command lookup ordering | [Shared current/path/root capture](../../../experiments/name-handling/results/shared-command-lookup-bases-20261013/results.md):133 Syntax naming controls, C command3, Jim5 and115 actual Runtime slot comparisons pass. Shared primitive serves logical helpers, source metadata, compiler adapters, Runtime API snapshots, Runtime and VM. | The retained-descendant refusal now passes after routing Native contexts through the checked adapter; three other occupied-current/path controls pass. VM live209-slot comparisons plus7 publication/retained-namespace controls pass; CLI/other compiler tests remain pending. The two VM variable-geometry adapters compiled here, without separate variable-winner acceptance. |
| Original braced-source correspondence | Real C5/Jim LF/CRLF programs are retained under [braced correspondence](../../../experiments/name-handling/braced-source-correspondence/).25 Syntax escape/source mapping controls and6 Registry word controls pass. | Compiler mapped-read and original occurrence controls pass, and all four real W210 headers obtain read advice. The declaration-flow consumer still refuses; preserve separate execution/frame authority. |
| #2286/W210 | Original nine cleanup controls still include C8.4 result0 versus expected2. Four original factory programs pass earlier guards. First measured refusal was continuation-folded body origin; source correspondence addresses that geometry but has not fixed the public result. [Compiler capture](../../../experiments/name-handling/results/braced-source-correspondence-2026-10-13/compiler.log) retains current failures. | Actual next refusal is missing declaration-flow advice under dynamic writes: physical missing reads are0. Refreshed20 controls pass17/fail3 (original W210 and two Native-formal metadata controls). Trace the exact source-purpose producer and complete-input metadata, retaining all assertions. |
| Callback source catalogue | Shared conditional receiver/member-body source catalogue is integrated; ordinary execution layouts retain strict frame evidence. [Frozen consumer capture](../../../experiments/name-handling/patterns/composed-callback-prefix/results/composed-callback-consumer/member-catalogue-production-20261013/README.md) passes4 Compiler and12 Core controls, including all4 unchanged original callback assertions and hosted/export/colon controls. | Fresh DB8 and server8 controls pass after an assertion-preserving supported-flags correction to an existing DB test. [Current-input receipts](../../../experiments/name-handling/patterns/composed-callback-prefix/results/composed-callback-consumer/database-server-current-inputs-20261013/README.md) retain the initial zero-test compile failure separately. Shared source-frame selection and remaining LSP adapters are being migrated in an isolated checkout. No Native execution authority follows from source catalogue acceptance. |
| CLI and registry metadata | CLI uses exact `ResolvedContext::resolve_spec`; literal case/whitespace/empty-name C5/Jim observations retained in [CLI results](../../../experiments/name-handling/results/cli-command-name-literal/). MCP exact-name and static-source results remain separately scoped. Initial Registry129 scoped controls pass. [Broader descriptor/cache capture](../../../experiments/name-handling/results/registry-descriptor-root-cache/README.md) passes83 distinct Registry/store controls with unchanged frozen inputs, including exact original cache labels and over-root negatives. | Execute CLI exact-name and actual pack-discovery controls. Mixed original-head/constructed-label operand selection still needs explicit selected-registration layout; measured private-worker probes remain separate from compiler acceptance. Written names and metadata labels stay distinct. |
| WASM/codegen | Original evaluated-argv transport is implemented. Runtime6 and API18 controls pass. [Compiler/API/CLI/strict-WASM capture](../../../experiments/name-handling/results/codegen-root-colon-publication/compiler-wasm-validation-01/README.md) records49 effective Rust controls:34pass/15fail, including CLI11 exact-name cases passing. | [Writable-cache real-WASM capture](../../../experiments/name-handling/results/codegen-root-colon-publication/wasm-cache-validation-02/README.md) passes6/8 actual Runtime-linked controls: once-only name/body, inert body, unknown-name abort and general/native-tier replaced-definer fallback. Two compiled publication/table positives still fail before linking. Four selected Compiler positives/refusals, tail positive and two table-plan positives need exact producer repairs; #1778 map identity remains. No compiled-entry acceptance follows from generic execution. |
| Variables, aliases and lifetimes | Existing source/split/trim, detached-cell and namespace-forget receipts remain indexed in [results](../../../experiments/name-handling/results/RESULTS.md). #2287 and #2300 are closed only for their recorded scopes. New query-specific geometry/bases and typed empty-namespace handling pass16 owner/C5/Jim conformance controls. [Actual Runtime query capture](../../../experiments/name-handling/results/variable-namespace-query-bases-20261013/runtime/README.md) passes48 C5/Jim rows, with unchanged source/binary and complete compiled-input coverage. | [VM namespace/query capture](../../../experiments/name-handling/results/vm-namespace-query-20261013/README.md) executes13 selectors:6 pass/7 fail.12 C84 namespace rows and24 C84/C85/Jim query rows pass; four namespace tests incorrectly require global errorCode after direct object entry, and three C86+ query fixtures refuse compiler preflight. Correct the observation to actual completion metadata, investigate the genuine admission producer, then rerun. Core adapter tests, Runtime empty namespace-address migration, retained contexts, roles, links, generations and publication collisions remain. |
| Original oracle harness | [Immutable upstream inventories](../../../experiments/name-handling/upstream-integrity/README.md) and7 admission controls pass. All five original proc/rename suites complete with0 failures. | Use each release's original init.tcl/Tcltest/support files. Keep overlays separate and bind every new result to actual input/source/binary identities. |
| Naming-test runtime dependency | [Array iterator repair](../../../experiments/name-handling/results/runtime-array-search-var132-current/results.md): two regressions and exact original var13.2 Runtime/C86 replay pass. | Full var.test or runtime parity is not claimed; use scoped naming suites as needed. |
| Gates and proof currency | No fresh full rust-check/prep-pr, resolution drift or strict proof gate passes. Fresh strict checker reports2606 questions,7069 candidates and1739 errors:769 evidence-digest changes,952 scrub census-digest changes,2 missing annotation bindings,4 document assertions and12 census-path changes. | Review actual semantics and narrow proof bindings; never bulk-refresh hashes. Required formatting, generated drift and all gates must be green before PR. |

## References and build coordination

Source-built C8.4.20,8.5.19,8.6.18,9.0.4,9.1.0 are provisioned with matching
original libraries. Jim5bac7c99ad65864c87da513e22e2f01703fa4e03 reports
0.84-9-g5bac7c9; upstream HEAD was checked on13 October2026 and matches.
Rediscover latest when collecting new Jim results. Record flags and extensions.
Rust stable1.99.0 and repository-pinned Ruff0.16.10 are available. Source
`/tmp/name-env.sh` and select an explicitly leased target. Only one Cargo build
uses a cache at a time; source mutations must not invalidate another capture.
WASM tools are provisioned from verified upstream assets, separately from
the retained historical Wasmtime notice. Six scoped genuine Runtime-linked WASM controls pass; two compiled-entry/table positives still fail before linking.

The active cadence is a tracking-document update plus reviewed commit/push every
15 minutes, and a high-level user progress report every hour. It started at
16:05 BST on13 October2026. This checkpoint is17:50 BST; the next tracking
checkpoint is18:05 BST and the next hourly report18:05 BST. The17:20 checkpoint was delayed while recovering sparse checkout provisioning; subsequent checkpoints retain the original cadence. Root checks the clock between bounded operations;
source captures retain their exclusive build/cache leases. Checkpoints may be
red, but must state exact failures and unexecuted checks.

Both codegen captures are sealed; source/executable identities and selected
compiled-input coverage are retained, with post-build-only generated/third-party
inputs explicitly separate. The next shared edit window covers original source
declaration flow, private member catalogue advice, Runtime namespace geometry,
VM error metadata and the measured namespace.which compile-hook decline.
The [backend audit](codegen-consumers.md) identifies Tcl bytecode,
WASM and separate BPF emitters. Native/GPU/FPGA capability declarations have no
current production emitters found and must not be reported as migrated targets.

The expanded [Runtime consumer audit](runtime-consumers.md) identifies remaining
namespace-address, variable geometry, publication/rename and import adapters.
The [LSP consumer audit](lsp-consumers.md) records current shared delegation and
remaining source-frame, hosted helper/export bridge and primitive geometry
adapters. Its330 lexical candidates are pending reconciliation, not330 completed
consumers. Read-only audit does not establish fresh execution.

The refreshed [issue roster](issues.md) verifies the previously open58 rows,
adds the direct #2093 source/frame dispatch blocker and35 closed naming
regression issues. State and semantic acceptance remain separate; related
package/value/server issues are dependencies only for their naming subset.
[Verified issue reads](verified-issues.json) and
[open search results](open-issue-search.json) retain the compact observations.
The [surface roster](surface-roster.json) enumerates52 workspace and11 excluded
packages plus editor/script/AI surfaces; unexamined rows remain unfinished.

The current source-read flow, private member catalogue, descriptor-roster and
namespace.which hook adapters are applied and formatted. Compiler production
captures01/02 execute zero tests after a slice iterator error and an omitted
Core obligation conversion respectively; both errors are corrected. Production03
compiles and completes three controls, then its original #2286 program remains
busy. The bounded capture is terminated with SIGTERM after915 seconds total,
without a failed assertion or completed result for the remaining32 controls.
Its exact source and binary remain unchanged. Worklist convergence needs
measurement; #2286 remains unaccepted.

The follow-up four exact Compiler catalogue controls pass under that same
unchanged production binary: two new member-body source/authority/currency
controls and two existing conditional receiver catalogue controls. The fresh Core
callback/hosted/export capture passes12 exact controls, including all4 original
callback assertions. An initial incorrect roster selection executed zero tests
and is retained separately. Both accepted binaries were sealed before the
checkout provisioning disk exhaustion; DB/server builds had not started then.

Implementation priority is adoption by every audited consumer. Agents use
five isolated sparse source checkouts with exclusive build targets so Runtime variables,
Registry/VM, source-frame/LSP and compiler/codegen migrations can proceed in
parallel. A dedicated audit agent reconciles the remaining package/callsite
roster and routes concrete gaps to the implementation owners. The primary
checkout owns reviewed integration, tracking, regular commits/pushes and gates.

Runtime variables, Registry/VM and compiler descriptor adapters, codegen, original
source-read flow, and LSP source-frame selection each have separate implementation
owners and targets. The dedicated read-only audit feeds remaining callsites to
those owners; root owns tooling/AI integration. Sparse checkout omissions are
restored byte-exact from the shared baseline before fresh captures; failed builds
remain explicit. No concurrent worktrees share Cargo targets. Full checkout
provisioning exhausted disk and was stopped; unused copies were removed and
replaced with154 MiB sparse checkouts. Build/cache budgets remain bounded.

[Runtime namespace geometry](../../../experiments/name-handling/results/runtime-namespace-geometry-20261013/README.md)
passes five exact selectors:60 actual original-object namespace rows across C5,
independent Logical simulation/ownerless internal-creation authority, existing
variable-root geometry and cell-sensitive fallback, and the previous48 query
rows across C5/Jim.31,697 inputs and artifacts are unchanged;4,907 actual compiled
inputs are covered with no omission. Namespace Unicode/NUL rows remain outside
the60 ASCII observations. Other consumer captures are queued, not accepted.

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
27 bounded semantic handoffs. It establishes no exhaustive acceptance. Source
procedure identities still need propagation into compiler maps, DB cascades and
reporting schemas. Root prepares the exact simulator mock table and SpecTcl/CLI
tooling boundaries; Registry/VM, compiler descriptors, LSP and codegen are active
in isolated checkouts. Source-flow tracing terminates its34-node graph and
completes C8.4/8.5/8.6 original assertions before reaching C9.0; no infinite
worklist defect is established. Full original-program completion remains pending.
