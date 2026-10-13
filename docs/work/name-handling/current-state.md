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
[finishing plan](plan.md). Source advice, metadata, live binding and edit authority
remain distinct. Preserve original inputs, selected protocol, context, liveness
and store currency; refuse missing, foreign, stale or dynamic evidence.

## Current acceptance and gaps

| Work | Evidence and current limit | Next action |
| --- | --- | --- |
| Shared naming owners | [12 baseline controls](../../../experiments/name-handling/results/shared-name-owner-current-20261013/README.md): C command3, variable2, namespace2 and Jim5 pass. | Audit every actual consumer; owner vectors alone do not certify adapters. |
| Command lookup ordering | [Shared current/path/root capture](../../../experiments/name-handling/results/shared-command-lookup-bases-20261013/results.md):133 Syntax naming controls, C command3, Jim5 and115 actual Runtime slot comparisons pass. Shared primitive serves logical helpers, source metadata, compiler adapters, Runtime API snapshots, Runtime and VM. | Compiler validation has a retained-descendant refusal failure; determine regression versus existing refusal. VM live209-slot vector and CLI/other compiler tests remain pending. |
| Original braced-source correspondence | Real C5/Jim LF/CRLF programs are retained under [braced correspondence](../../../experiments/name-handling/braced-source-correspondence/).25 Syntax escape/source mapping controls and6 Registry word controls pass. | Compiler original-read acceptance is incomplete. Use genuine declaration receipts; do not grant executed-frame authority to source-only advice. |
| #2286/W210 | Original nine cleanup controls still include C8.4 result0 versus expected2. Four original factory programs pass earlier guards. First measured refusal was continuation-folded body origin; source correspondence addresses that geometry but has not fixed the public result. [Compiler capture](../../../experiments/name-handling/results/braced-source-correspondence-2026-10-13/compiler.log) retains current failures. | Measure the next exact refusal; retain original assertions, foreign/stale/input/channel negatives, factory aliases and failed-publication controls. |
| Callback source catalogue | Conditional receiver-source preview implementation is prepared; ordinary execution layouts retain strict frame evidence. Existing composed callback acceptance is incomplete. | Fresh two Compiler catalogue controls and four Core callback controls, then occupied-current/path, currency and source-barrier negatives. |
| CLI and registry metadata | CLI uses exact `ResolvedContext::resolve_spec`; literal case/whitespace/empty-name C5/Jim observations retained in [CLI results](../../../experiments/name-handling/results/cli-command-name-literal/). MCP exact-name and static-source results remain separately scoped. | Execute CLI exact-name and actual pack-discovery controls. Audit registry geometry and descriptor adapters without conflating written names and metadata labels. |
| WASM/codegen | C5/Jim root-colon publication observations show why a reporting FQN cannot be a fresh publication operand. Original evaluated-argv transport implementation is in progress. | Real compiler admission, once-only arguments, actual Runtime object/lifetime controls, strict ABI and emitted WASM linking; structured map identity remains separate #1778 work. |
| Variables, aliases and lifetimes | Existing source/split/trim, detached-cell and namespace-forget receipts remain indexed in [results](../../../experiments/name-handling/results/RESULTS.md). #2287 and #2300 are closed only for their recorded scopes. | Retired-context propagation, variable purpose/roles, links, generations, publication collisions and all remaining consumers in the issue map. |
| Original oracle harness | [Immutable upstream inventories](../../../experiments/name-handling/upstream-integrity/README.md) and7 admission controls pass. All five original proc/rename suites complete with0 failures. | Use each release's original init.tcl/Tcltest/support files. Keep overlays separate and bind every new result to actual input/source/binary identities. |
| Naming-test runtime dependency | [Array iterator repair](../../../experiments/name-handling/results/runtime-array-search-var132-current/results.md): two regressions and exact original var13.2 Runtime/C86 replay pass. | Full var.test or runtime parity is not claimed; use scoped naming suites as needed. |
| Gates and proof currency | No fresh full rust-check/prep-pr, resolution drift or strict proof gate passes. Last maintained proof check had2606 records and296 digest errors across146 paths/99 records. | Review actual semantics and narrow proof bindings; never bulk-refresh hashes. Required formatting, generated drift and all gates must be green before PR. |

## References and build coordination

Source-built C8.4.20,8.5.19,8.6.18,9.0.4,9.1.0 are provisioned with matching
original libraries. Jim5bac7c99ad65864c87da513e22e2f01703fa4e03 reports
0.84-9-g5bac7c9; upstream HEAD was checked on13 October2026 and matches.
Rediscover latest when collecting new Jim results. Record flags and extensions.
Rust stable1.99.0 and repository-pinned Ruff0.16.10 are available. Source
`/tmp/name-env.sh` and select an explicitly leased target. Only one Cargo build
uses a cache at a time; source mutations must not invalidate another capture.
WASM tools are being provisioned from verified upstream assets, separately from
the retained historical Wasmtime notice.

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
