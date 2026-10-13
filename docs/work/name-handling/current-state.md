# Current name-handling state

Branch: `work`; integrated cleanup base: `049c0fc7a`. Inspect Git for the current local/published commit. This is temporary coordination material: remove it and its references before merge. Root owns Git, issue updates and build/source leases. Receipts certify captured inputs, not later edits.

## Goal and boundaries

Central name representations, lookup context and typed evidence must serve every consumer: parser, registry/dialects, analysis/lowering/SSA/types, cells/arrays/links, classes/methods/aliases, LSP edits, database/import/package, VM/runtime, all codegen targets, CLI/tooling, AI/MCP/editors and frameworks. See [name handling](../../design/contracts/name-handling.md), [command resolution](../../design/contracts/command-resolution.md), [consumer integrations](../../design/analysis/name-resolution-proofs/consumer-integrations.md), [canonical proofs](../../design/analysis/name-resolution-proofs/README.md), [ledger](implementation-ledger.md) and [issues](issues.md).

Tcl source recognition is conditional advice. Preserve original input, configuration, context and store currency; decline missing, foreign, stale or dynamic evidence. Native object/frame/lifetime proof, completed execution and permission to rewrite remain separate. Test expectations come from measured C Tcl 8.4–9.1, pinned Jim and reached F5 contexts. Never run Tcltest in F5Tcl TMM/MCPD.

## Measured state and open work

| Surface | Measured acceptance | Remaining gap |
| --- | --- | --- |
| Source/split/trim | Source/split13 focused passes;18 original scripts and8 producer controls; Runtime/VM trim216 windows each. | Nested array-index/subst provenance, surrogate trim fixtures, physical VM split and other-release execution. |
| Real C86 naming suites | Original init/tcltest/all.tcl reaches67 var.test starts through var-13.2, with18 explicit skips. | Isolated original var-13.2 reproduces SIGABRT in Runtime; C86 passes1/skips109/fails0. Runtime has no final counts. Cause remains unproved; starts are not passes. |
| Variable diagnostics | Syntax2 policy controls and detached-element Runtime3 controls pass. | Retired-namespace Runtime3 controls still fail: constructed-script and ordinary original-vector metadata propagation. No broader acceptance. |
| Clock/glob | GetEnv7+host1+registry1; ParseFormatArgs11 rows; GetDateFields Runtime9/Core2/Registry1; active-call/glob batch9 passes. | Four clock workers, full constructor/pool/map lifetime, guest env sync, empty-table localtime and C9 signature. Glob qualifier/parser/platform gaps remain separate. |
| VM OO | Bootstrap constructor/namespace; reporting-owner/getter4 passes; SamePosition inner/outer cause fixture1 passes. | Earlier17=11/6 and29=18/11 are not fresh results. Two native expression boundaries and two forward/procedure accessor fixtures remain. |
| Callback/LSP | Public C86 source advice and issuer positive; source-selector preservation Compiler2 passes. | Historical Core4=1/3. Stored targets remain empty; after211 catalogue/transition evidence absent. Private builder schema, Logical hint, dynamic-formals and later-release/currentness checks remain distinct. |
| Registry/IRules/SpecTcl/Studio/MCP | Typed configuration-reference field parity; refinement4 passes; whole-expression source guard5 passes; generated field docs2 passes; unchanged MCP generator1 passes. | Wider consumer acceptance, middleware definition fixture and unrelated exact-name controls remain scoped. Earlier failures remain retained. |
| WASM/codegen | C5/Jim original root-colon publication distinction measured; original-argv proposal prepared. | Proposal unapplied. Actual compiler admission, Runtime, emitted WASM, strict ABI fingerprint and once-only argument/lifecycle controls needed. Hold source changes during allocator investigation. |
| Database/middleware | Structured workspace symbols and original package/autoload keys inspected. | Actual current Document literal-positive/computed-negative fallback fixture before any repair. |
| W210/#2286 | Cleanup9=8/1; graph20=17/3; four original programs pass outer/factory-prefix guards. | Original later declaration/formal/body-origin/unanimity proof unmeasured; two loops reproduce before-source. Issue open. |
| Scoped closed issues | #2287 packaged2/direct2/Logical5/import1; #2300 genuine NativeC86 forget3 passes and C5 chain controls. | Wider lifetimes, replacement/teardown and provider profiles remain separate. |
| Wild patterns | Pinned Snit/interp and Tcllib lazyset C85–C91 controls reached; separate lazyset core all C5 reached. | Real-package C84/Jim setup refusals retained; more corpus/Wiki/framework evidence and consumer heuristics needed. |
| Owner/proof/gates | Historical owner29/29 and158 public-entry obligations; last maintained proof check2606 records/296 digest errors across146 paths/99 records,0 pending-unlinked;35 reviewed renewals. | Fresh substantive acceptance/currency review; no blanket digest renewal. Full rust-check/prep-pr and required JetBrains SDK checks outstanding. |

## Exact receipts

- [GetDateFields Runtime9](../../../experiments/name-handling/results/runtime-c86-real-suites/get-date-fields-runtime/validation-02/validation.json), [Core2](../../../experiments/name-handling/results/runtime-c86-real-suites/get-date-fields-implementation/shared-validation-03/README.md), [Registry1](../../../experiments/name-handling/results/runtime-c86-real-suites/get-date-fields-implementation/shared-validation-02/README.md) and [original all.tcl zero-file selection](../../../experiments/name-handling/results/runtime-c86-real-suites/get-date-fields-runtime/orchestration-01/record.json). Earlier failed builds remain independently sealed; Registry guard does not directly assert backing.
- [Namespace-forget Runtime acceptance](../../../experiments/name-handling/results/namespace-forget-global-origin/runtime-validation-01/README.md) and [C5/Jim import-chain controls](../../../experiments/name-handling/results/namespace-forget-import-chain/README.md).
- [MCP static inventory/helper acceptance and datagroup failure](../../../experiments/name-handling/results/mcp-static-name-inventory/validation-01/README.md); default target exclusivity is coordinator-confirmed only, not independently attested by the capture.
- [MCP exact name acceptance](../../../experiments/name-handling/results/mcp-exact-command-name/validation-02/README.md) and [three existing controls](../../../experiments/name-handling/results/mcp-exact-command-name/existing-controls-01/README.md).
- [C86 source/split acceptance](../../../experiments/name-handling/results/split-native-implementation-c86/README.md), including13 tests,18+8 controls, original orchestration and source scopes; [reference study](../../../experiments/name-handling/results/split-native-reference/README.md).
- [Shared trim acceptance](../../../experiments/name-handling/results/split-native-trim-delegation-c86/README.md), both216-window controls; [GetDateFields operation contract](../../../experiments/name-handling/results/runtime-c86-real-suites/get-date-fields-question/operation-contract.md), retained separately from execution receipts and sealed oracles.
- [OO reporting repair](../../../experiments/name-handling/results/vm-oo-root-namespace-creation/questions/reporting-name-repair/acceptance-02/question.md), four controls passed; [sealed results](../../../experiments/name-handling/results/vm-oo-root-namespace-creation/questions/reporting-name-repair/acceptance-02/results.md). [Namespace-forget question/oracles](../../../experiments/name-handling/results/namespace-forget-global-origin/README.md).
- [VM bootstrap acceptance](../../../experiments/name-handling/results/vm-oo-root-namespace-creation/repair-02/results.md), [earlier target diagnostic](../../../experiments/name-handling/results/vm-oo-name-target-predicate/capture-01/results.md) and [earlier29 controls](../../../experiments/name-handling/results/vm-test-api-repairs/results.md).
- [Readonly reference integration/core4](../../../experiments/name-handling/patterns/composed-callback-prefix/results/composed-callback-consumer/source-only-reference-core-1/README.md): still1/3; [actual first empty-store diagnostic](../../../experiments/name-handling/patterns/composed-callback-prefix/results/composed-callback-consumer/source-only-reference-diagnostic-1/README.md), two temporary sources restored exactly; [actual store blockers](../../../experiments/name-handling/patterns/composed-callback-prefix/results/composed-callback-consumer/source-only-store-diagnostic-1/README.md), temporary logging restored exactly.
- [Callback public ingress/core4](../../../experiments/name-handling/patterns/composed-callback-prefix/results/composed-callback-consumer/production-ingress-core-1/README.md), [issuer controls](../../../experiments/name-handling/patterns/composed-callback-prefix/results/composed-callback-consumer/production-ingress-issuer-1/README.md) and [questions](../../../experiments/name-handling/patterns/composed-callback-prefix/questions/consumer-first-barrier-diagnostic.md).
- [Historical owner29/full158](../../../experiments/name-handling/results/owner-resolution-current/20261012T231605Z/README.md); no fresh owner/proof gate follows the latest code changes.
- [Last full proof check](../../../experiments/name-handling/results/proof-source-currency/20261012-current-consumer-check/README.md), [W210 actual factory-prefix probe](../../../experiments/name-handling/results/w210-original-program-admission/inner-predicate-question/actual/README.md), [suite census](../../../experiments/name-handling/results/RESULTS.md) and [real Snit experiment](../../../experiments/name-handling/patterns/corpus-snit-alias-original/README.md).

## Continuation audit and evidence-integrity obligations

A read-only issue/consumer/oracle audit on 13 October 2026 read remote work
at e0277ec42124d26b36cc06daa760de6b21a1decb. No fresh implementation tests ran.
The checkout is synchronised to work at 000c431f05. Rust stable 1.99.0, all five
source-built C references and Jim 5bac7c99 are provisioned. Latest upstream Jim
HEAD was checked on 13 October and matches that revision. Source /tmp/name-env.sh
and select an explicitly leased target; no full PR gate is green from setup.

- Original-suite runner admission now checks immutable upstream library/test
  inventories, shared manifest pin and source-built original init/Tcltest
  startup. Seven admission controls and all five C proc/rename captures pass.
  Overlays/debug settings remain separate; the VM driver is fixed to 9.0.
- The Jim pin 5bac7c99ad65864c87da513e22e2f01703fa4e03 reported
  0.84-9-g5bac7c9 and was checked against upstream on 12 October.
  Rediscover latest on the next run; provisioning that pin does not discover
  latest. Record build flags, extensions and operation-specific capabilities.
- The shared owner and resolution drift checks cover finite wiring patterns;
  neither certifies every consumer. Reconcile all call sites, backend/provider/
  editor rosters and generated owners with the completion inventory.
- Additional verified open dependencies include #1797 procedure generations,
  #2233/#2247 relative/qualified registry invocations, #2261 current-frame body
  effects, #2356 F5 when context, #2413 sharedvar and #2426 diagnostic subjects.
  See [issues](issues.md) for broader verified relations.
- Repository-wide comment cleanup still includes migration explanations in
  rust/tcl-compiler/src/codegen/mod.rs (artifact moves and former emitter state).
  Audit current source before editing; preserve actual how/why contracts.

## Centralisation scope and current validation

The primary goal is one reusable owner for name lookup and every consumer
delegating to it. Fix #2286 and issues entirely caused by naming: context,
identity, lookup order, roles, binding lifetime and consumer disagreement.
General runtime/clock/numeric/OO conformance is a dependency only where needed
to execute naming controls; it is not an independent completion programme.

[Central-owner controls](../../../experiments/name-handling/results/shared-name-owner-current-20261013/README.md) pass 12 tests: command 3, variable 2, namespace 2, Jim 5.
[Original harness integrity](../../../experiments/name-handling/upstream-integrity/README.md) has 7 passing admission controls and measured original proc/rename
suites for all five C references. Consumer migration still needs its own
public positive/negative controls. W210 fresh unchanged baseline remains 8/9
with original C8.4 control returns 0 instead of 2; later declaration refusal is under
bounded measurement. CLI exact-name migration and Runtime array-state blocker
repair are in progress. No fresh full rust-check/prep-pr/proof gate passes.

## Current execution priorities

1. Investigate the reproduced original var-13.2 compatibility failure. [Isolated results](../../../experiments/name-handling/results/runtime-c86-real-suites/array-search-isolation/var-13.2/results.md): Runtime SIGABRT after10.609s, C86 passes1/skips109/fails0. All2787 tracked compiled inputs match the original runner; the scrubbed noncompiled fixture remains untouched. No source cause or repair is claimed.
2. Test the [retired caller-context repair proposal](../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-runtime/caller-context-source-review.md): selected EVAL_STK enters generic List dispatch, and ordinary element receiver failures lose NameLookup context. These are source-review findings, not accepted fixes. [Callback catalogue question](../../../experiments/name-handling/patterns/composed-callback-prefix/results/composed-callback-consumer/after-consumer-catalogue-question.md) needs a fresh Core binary and existing bounded trace flags. Keep source advice separate from Native authority.
3. Continue bounded repository-wide docs/comments cleanup. Four installer/release/gate scripts now have comment-only cleanup with unchanged executable lines/legal headers and passing shell syntax checks. Root owns tracking; agents own disjoint files.
4. Resume the original-argv codegen repair after the allocator blocker is understood, then complete remaining consumers and current owner/proof checks.

Use `source /tmp/name-env.sh` and an explicitly coordinated target directory. Only one Cargo build uses a cache at a time. Preserve operational refusals and failed captures. Commit and push coherent progress regularly; development pushes before green gates are authorised, but required gates must pass before a PR.

## F5 and retained evidence

The original numbered snapshot retains400 F5 rows:93 observed/244 unreached/63 unavailable.
The current pattern matrix separately reports108 observed/25 unsupported/267 unreached
across50 questions/eight contexts. These scopes and status vocabularies differ;
retain each record's actual question IDs rather than merging counts. Supplementary40 derived variants/307 investigations/279 separate records do not promote originals. Suspended frames, restart persistence, other traffic units and earlier cleanup remain open. Preserve benign Tcl probes, actual reached channels and provenance. Never restore deleted lab/device/security material from historical snapshots. A further [transport projection](../../../scripts/dev/bigip-probes/resolution-2286/evidence/TRANSPORT_PROJECTION.md) removes remaining identities from910 plain files and42 archives;952 current manifest entries verify separately from historical execution seals.

The user's [scrub evidence](../../../experiments/name-handling/results/security-material-scrub/README.md) records current scans, redaction boundaries and reproduction procedures. Post-integration focused checks total18 passes; wider report15/17, Clippy assertion failures and absent Python engine remain scoped blockers. Historical executable identities are not hashes of redacted transport images; the retained Wasmtime input is a notice, not an executable replay source.

## Cleanup before merge

Audit authored/generated documentation, every code comment, tests, scripts, templates and links across the entire repository, including pre-existing material. Remove stale claims, obsolete guidance, completed TODOs and development narratives; retain minimal current how/why, actual user procedures, legal attribution and reproducible experiments/conformance tests. Remove docs/work, continuation/state/ledger documents, obsolete proposals and their references, plus the work-specific AGENTS push exception. No PR or merge is currently requested.
