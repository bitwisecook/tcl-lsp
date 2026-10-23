# Clean LSP release candidate plan

Prepared 23 October 2026. Planning and validation records stay on `work`; this
file is excluded from the shipping candidate. Nothing below is a claim that an
unexecuted gate passed.

The deliverable is a reduced branch targeting `rust`, with current product
source, supported contracts, documentation and regression tests, ready for a
high-quality squash merge. All functional LSP fixing, continuing migration work
and development evidence remain on `work`. The intermediate branch is for cleanup, extraction and
validation only. The user's intermediate-branch permission is recorded in
`handoff/reviews/resume-20261017-diagnosis/user-decisions-20261023T084659Z.json`.
Follow-up fixing-location and iRules-performance decisions: `handoff/reviews/resume-20261017-diagnosis/user-decisions-20261023T084954Z.json`.
The user subsequently requested implementation and reaching the PR within six
hours, starting08:56UTC on23October (target14:56UTC). Opening the PR is now
authorised after the existing gates pass; no further opening approval is needed.
Merge, tagging and release publication remain subsequent unauthorised actions.

## Acceptance contract

All of these conditions are necessary:

- Every release checklist item in review section F is satisfied, with a
  defensible disposition for every section B finding. No enabled known
  wrong-target or unsafe-edit case remains.
- Supported static cases retain authoritative family/release/environment-aware
  identities through providers, compiler facts and edits. Dynamic or incomplete
  cases expose uncertainty and retain fail-closed guards; supported functionality
  is not replaced with blanket refusal.
- All applicable required checks pass on the exact reduced shipping commit.
  Complete test inventories establish zero pass-to-fail against the selected
  `origin/rust`; omitted, skipped, cancelled or unreached cases remain unknown.
  All newly introduced tests pass. Baseline failures do not make a required
  shipping check green.
- Release-build corpus, interactive, cancellation and memory measurements meet
  the existing contract: a regression beyond measured run noise blocks the
  change. Typical first semantic tokens target single-digit milliseconds. The
  40 ms fast-tier race is not a universal request SLA.
- Documentation and comments describe the actual supported implementation.
  No development evidence, implementation ledgers, probes, temporary switches,
  process narratives or broken references to removed material remain in the
  candidate diff. Tests keep their necessary inputs and expected answers.
- The tree is formatted, generated files are current, strict checks have zero
  warnings, packaging works, and running generation/gates leaves no unexplained
  tracked changes. Squash-tree verification uses the same product tree.

## Current position and immediate sequence

The six-hour target passed at14:56UTC with required gates still red. Work and
the reduced release branch remain separate; no PR is open. Current detailed
outcomes and owner routing live in `current-state.md` and `outstanding/`.

The latest executed startup on2d8 passes cold tokens in124.536ms but fails
the unchanged30-second workspace barrier. Analysis of one system tclIndex
alone takes21.011seconds for Tcl8.4; Tcl8.5 starts without completing before
shutdown. Context's two original assertions pass, but16Cancelled rounds and a
nonsettlement warning remain. The old capture did not seal those actual system
inputs; the fresh driver seals and retains them and identifies the internal
analysis phase and cancellation reason. This needs a producer fix and a fresh
successful protocol run, not a larger budget or skipped Tcl release.

Fresh7-mode diagnostics were dispatched at23:29 on exact WORK
`b52ff0b6c7ef223da6b920031be4d6b314346311`; full CI including native build
proof runs on clean `a26ab0ffd2b1dc1681710dbf14e518025545ec33`
([CI40039952540](https://github.com/bitwisecook/tcl-lsp/actions/runs/40039952540)).
Keep these immutable source identities distinct from subsequent repairs.
The active documentation checkout has frozen older product bytes and must not
be built or reported as the shipping source. All functional changes first land
in the publisher's `work`, then reviewed counterparts carry to release.

Actually observed gains are narrow: DB's requested strict all-target/all-feature
command passes without warnings; O129 folding and the older Jim class case
pass. Core4 and Server131 old lint findings require fresh measurement after
later fixes. Spec's original derived42/xy still fail; Jim method and nested-body
ownership remain unresolved. Runtime append times out, numeric Jim retains
ERANGE, and the separate fullTom baseline has14PASS-to-FAIL plus382unknown.
The main24312namedPASS baseline comparison remains unknown. No full required
CI, sectionF or Tcl corpus performance gate passes yet.

1. Authenticate the current7diagnostic runs and clean CI without canceling
   healthy runs. Preserve failures, resource stops and unreached tests. Required
   source pins now pass before dispatch; actual source/provider/artifact and
   process seals remain necessary when interpreting results.
2. Fix the measured startup/cancellation producers, compiler ILX/RBS/SH and
   ReadFlow owner failures, Jim method/nested-world ownership, Spec derived
   execution and runtime baseline regressions/stack failure. Keep all original
   assertions, currentness and fail-closed guards; handle unanswered new policy
   questions only when the user's decision arrives.
3. Complete strict Core/Server checks and every required native, browser,
   WASI, editor, generated and SpecTcl gate on the integrated shipping tree.
   Reconcile every selected baseline identity; unknown is not pass.
4. Finish the existing perf owner's complete authenticated old/mixed Tcl
   corpus and client proof. Accept no optimisation or realm patch before its
   actual proof; then measure the final shipping source under the recorded
   latency/cancellation/memory contract. Dedicated iRules timing alone is waived.
5. Complete repository-wide documentation/comment cleanup and generated drift,
   retain test-consumed fixtures, run final formatting/rust-check/prep-pr and
   packaging, then verify the squash tree matches the validated product.
   Open the already-authorised PR only after every acceptance condition holds.

## Functional workstreams and owners

| Workstream | Required completion | Owner |
|---|---|---|
| LSP providers and transformations | Fix nested braced-expression minification under the recorded whitespace decision; resolve original syntax/body and argument-replacement failures; validate all affected provider answers, UTF-16 edits, revision handling, complete rename/refusal and actual transformed execution. Correct the BOM fixture only through its genuine Skip configuration, retaining Content coverage. | lsp; compiler-names for compiler ownership |
| Compiler identities and diagnostics | Resolve remaining source/installed/hosted report correspondence, aliases, foreign-body ownership, source syntax alternatives, nested lowering, C6/G4 lifecycle outcomes, SH/RBS diagnostics, ILX producer lookup, OO/itcl/snit and iRules realm cases. Preserve actual Error/abrupt outcomes and the loaded-world decision. Keep test corrections narrowly oracle/decision justified. | compiler and compiler-names |
| Codegen and live hooks | Repair original native preflight, reference-body and release-sensitive derived execution failures; prove the live const-fold hook actually runs and returns the right answer in supported DSL versions. Hostless/budget/unknown-context refusals stay valid. Remove N17 switches/probes only after the repaired positive and refusal tests pass. | compiler-codegen |
| SpecTcl functionality | Complete C.1–C.2: real pack evaluation and wrong-registration regression, supported 1.x/2.x compatibility, authoring, all supported live hook families, Jim core loading/body analysis, all six EDA environments, every UPF hook slot, cross-worker publication and add/change/remove reload coherence. Test native filesystem and shipped browser VFS. General VM migration stays deferred; baseline execution regressions remain mandatory. | vm-runtime with compiler/codegen and lsp |
| BIGIP/editor consumers | Finish original consumer tests under the stop-after-opaque ILX decision; strict Clippy, generated editor catalogues, Zed/tree-sitter and shipped integration follow-ups. Route generic compiler/LSP blockers to those owners. | existing separate bigip-lsp session |
| Performance patches | Finish authenticated full-report API/carrier correctness, then have the existing perf owner re-run the entire all-old/mixed corpus and pack equality proof. Accept only complete Equal, never Different/Unavailable. Forward realm/interprocedural only after the owner records the exact patch proven. Integrate clean patches without switches, then remeasure shipping performance. | compiler/names for API; existing separate perf session for proof |

No duplicate BIGIP, perf or review-triage roles are started. Notify the user when
external role files exceed60 minutes; do not silently take them over. Review
triage is marked complete; its old header does not establish incomplete triage.

## Integrated server and oracle acceptance

Owner-level helpers complement these six review-D scenarios; they do not replace
them. Reuse the real server/e2e and editor harnesses:

1. Package versions in one process: A→B→A and B→A→B, unavailable registrations,
   Unicode autoload, changed requirements/library paths and retired providers.
2. Inherited source state and deep source chains, unopened children, multiple
   namespace views, constants after hop5, composition after hop4 and edge edits.
   Await readiness independently and assert one completed reconciliation.
3. Warm/fresh lifecycle answers after open-document and watched closed-file
   changes; compare the same committed semantic revision.
4. Environment changes with unchanged text: pack/stub/dialect remove/restore,
   source-seeded and closed files, class consumers, cross-worker hook answers
   and absence of late old-generation publication.
5. Real protocol edits from declaration and call origins, imports/aliases,
   method/collision cases, Unicode ranges and open/closed files. Apply the
   returned workspace edits and execute with the matching oracle. Include
   optimiser/minifier output and compiler quick fixes.
6. Shipped native JSON-RPC, browser/WASM, WASI and editor hosts, including VFS
   updates, delivery, shutdown, pack configuration and edit application.

Run the required shared C Tcl8.4–9.1 and pinned Jim oracle vectors, preserving
family differences. Complete dedicated Jim/EDA/UPF tests beyond
`make test-spectcl-compat`; its inventory alone does not include Jim core packs
or prove every executable hook slot. Use the recorded capability split: the full-TomMath suite is the semantic
release gate with original bignum assertions; strict intentional no-TomMath
checks cover capability and refusal, including the approved explicit absence
assertion. Both gates require zero warnings.

## Corpus performance acceptance

This has two separate proofs: semantic equivalence of proposed optimisations,
and user-visible performance of the final shipping build. Neither implies the
other. The distinct authenticated report comparison must cover every field and
typed subject with complete unique current correspondence; global/runtime Eq,
Salsa cutoffs and native eligibility remain strict.

### Comparable builds and workloads

- Pin an exact `origin/rust` baseline, candidate SHA/tree, harness revision,
  toolchain/features, dependencies, corpus revision/pins, host/OS/CPU and runtime
  versions. Build both release servers comparably. An upstream advance triggers
  baseline/diff review and reruns as appropriate.
- Use `scripts/perf/MANIFEST.toml` and `fetch_corpus.py --verify-only`; retain
  the corpus revision. Run the existing fixed15-check sequence for small,
  medium and full Tcl scopes, not only the default small set. Preserve anchors,
  request counts, selected documents and checks; do not compare different work.
- Dedicated iRules performance benchmarking is excluded from this release by
  the follow-up user decision. This is not a measured iRules performance claim
  and does not waive iRules correctness or safety checks. Existing `irules` and
  `everything` scopes are not substitutes for comparable Tcl timing scopes.
  Include representative Jim and EDA pack workloads and source/import/package
  graphs in the Tcl/SpecTcl performance coverage. Report the actual inventory
  and any missing corpus access explicitly.
- Run isolated paired/interleaved baseline and candidate repetitions on the
  same host, separating cold startup/index/load from warm requests and repeated
  edits/reloads. Establish the host's noise before judging a change. Keep
  timeouts, assertion outcomes and incomplete checks visible.
- Capture per-project/per-operation medians, tail latency and variation; use
  enough repeated requests/runs to support any percentile or confidence claim.
  Compare meaningful returned results and full diagnostics completion, so
  omitted analysis or perpetual cheap provisional answers cannot appear faster.

### Metrics and blockers

Measure open→first semantic tokens, edit→response, full diagnostic convergence,
workspace scan/index, hover/definition/references/completion/symbols, code lens,
rename/file rename, and close retention. Include cancellation and overlapping
requests under edit storms: stale publication, cancellation delay, work/CPU
left behind and starvation. Include peak/retained RSS or host-appropriate heap,
CPU, allocations where helpful, and repeated open/edit/close, source/package,
pack/environment change and reload cycles; detect unbounded retained growth.
Measure cold pack load, warm reuse and first-worker hook installation separately.

Use representative sizes plus targeted scaling for body/scope/index scans,
import-heavy updates, source depth and reference/edit deduplication. An aggregate
improvement cannot hide a slow project, tail regression, stall or leak. Baseline
stalls do not excuse candidate stalls on supported representative workloads.

Run native and browser shipping builds. Where macOS-specific code paths matter,
obtain paired macOS measurements; Linux CI is not their substitute. The current
Performance workflow is informational and does not enforce acceptance: explicitly
review each check and fail the release decision on incomplete/failed workloads
or regressions beyond measured noise.

No fixed percentage slowdown, numeric memory ceiling, maximum supported
workspace size or long-request bound has been agreed. Define and record the
workload envelopes and any additional numeric policy before using it for a
passing judgement; ask the user if a genuinely new acceptance choice is needed.
Do not invent5%/10% as an existing decision or relax the existing noise rule.

## Candidate construction and separation from work

Use a separate shallow checkout and candidate branch based on the then-current
`origin/rust` (suggested name `release/lsp-naming`). Never switch or reset the
shared work checkout. Fetch only the branch/commit/history actually required.
Do not merge work wholesale or cherry-pick evidence-bearing development commits.

1. Freeze a named work snapshot and record the exact rust base. Inventory the
   **entire** work→rust difference, including earlier migration commits, not
   just the latest46/66-path development delta.
2. Produce a reviewed path/hunk inclusion manifest with production dependency
   closure: all LSP-required compiler, registry, lexer/syntax, SpecTcl runtime,
   build/CI, assets and generated dependencies. Separate unrelated deferred VM
   work; do not drop necessary behavior because its crate is called runtime.
3. Assemble approved final file/hunk contents onto rust in the candidate
   checkout. Keep only actual shipping tests, minimal consumed fixtures,
   supported docs/contracts and legitimate build/check tooling. Record every
   omission or candidate-only cleanup in work-side extraction records.
4. Exclude added `docs/work/name-handling`, migration experiments, development
   proof-output trees, raw logs, archives/binaries, reports, timers, patches,
   scratchpad mirrors, profiling output and benchmark result evidence. Do not
   add new evidence/graphs to the PR to demonstrate green; keep validation
   records externally/on work for the later evidence delivery.
5. Audit references and build closure after exclusions. Classify proof checkers
   and manifests that read development evidence: remove development-only
   machinery, and retain/migrate real source invariants into durable executable
   conformance checks. Never leave a required checker pointing at absent files,
   skip it, weaken assertions or fabricate success to make extraction pass.
6. Review the candidate diff and dependency/feature/lockfile changes against
   rust. Avoid unrelated churn and generated files that contain removed records.
   Later work fixes enter by explicit reviewed deltas; never automatic merges
   from the moving work branch.

All functional fixing happens on work. If candidate validation discovers a
functional defect, diagnose and repair it on work, run the appropriate checks
there, then carry the explicit reviewed product delta to the candidate. The
intermediate branch handles cleanup/extraction and validation only; it is not a
second functional development branch. Preserve other sessions' changes and do
not reintroduce removed ledgers while carrying fixes across. Candidate cleanup
does not authorise deleting evidence from work. Any relevant candidate change
invalidates affected validation.

## Fixture reduction without coverage loss

Recompute the current consumers; the21October fixture review is a starting
inventory, not a final allowlist. It found roughly1.46GB of added data, mostly
unread evidence, and208 captured release expectations not asserted. Resolve its
count discrepancies from actual paths; do not copy inconsistent totals.

Retain all `include_*`, runtime-loaded, generated-path/macro, directory-enumerated
and packaging inputs genuinely consumed by candidate tests. Remove unread
captures, copied repository/vendor sources, old attempts, probe executables and
archives from the candidate. Merge byte-identical/overlapping fixtures and
convert the identified51 families to tables where it improves reviewability.
Assert all intended release vectors, including the208 currently unasserted
expectations, or explicitly justify their scope under existing decisions.
Preserve case identities or a reviewed mapping, all old assertions and positive
plus adversarial behavior. Prove the reduced harnesses read the intended rows
and measure test cost before/after. Required oracle inputs are tests, not bulk
development evidence, and must remain reproducible without work-only paths.

## Repository-wide documentation and comments

The user explicitly reaffirmed that development-process chaff is removed from
the release and may remain on work. Cleanup therefore affects the entire
release tree; no evidence/process deletion from work is required.

Audit every relevant repository surface, including pre-existing text, not just
added lines. Owners review statements against the final candidate source:

- README/feature and platform support, installation/configuration, editor
  settings, supported dialects/releases and SpecTcl authoring/user guidance.
- Architecture and naming/registry/identity/source/package/invalidation contracts,
  provider agreement, transformation safety, trust/budgets and reload behavior.
- Performance/concurrency/cancellation/memory descriptions and claims: remove
  stale numbers or qualify them to their actual measured workload.
- Glossary, KCS, design indexes, command/diagnostic catalogues, examples,
  generated copies, API docs, tests, scripts, templates and workflow comments.
- Release notes describe observable behavior and any approved temporary
  restrictions; explicitly bound deferred VM/backend migration. Do not claim
  complete supported LSP migration before the contract is met. Do not hand-edit
  generated release performance sections or insert development benchmark evidence.

Remove temporary implementation narratives, phase/agent/probe references,
completed or obsolete TODOs and abandoned guidance. Preserve terminology that
actually describes user procedures or runtime semantics, licence attribution,
durable rationale and reproducible conformance inputs. Replace decision-file
references in necessary comments with the actual invariant/oracle rationale.
Check relative links/anchors, generated indexes and examples after removals;
require human/context review rather than blind keyword deletion.

## Validation order on the reduced candidate

Use prescribed stable Rust and Node26+, pinned C Tcl/Jim and required native,
WASM/WASI/editor tooling. Use isolated target directories when builds overlap
between checkouts; under current disk constraints root allocates one build at
a time. Keep logs bounded20MB, stop below2GB, obey artifact age rules and never
delete under target while cargo/rustc exists. Preserve existing budgets.

1. Formatting/code generation and cheap syntax/link/config checks; run
   `make prep-pr`, commit legitimate generated changes and rerun until clean.
2. `make rust-check`: workspace/runtime/BIGIP strict Clippy, rustfmt and all
   xtask/generated/owner-resolution/proof/source invariants. No new allows,
   warning suppression, target skipping or reduced gate coverage.
3. Targeted correctness plus shared oracle matrix; `make test-spectcl-compat`
   and additional Jim/EDA/UPF/section-D regressions.
4. Complete native workspace shards, heavy selections and doctests; native LSP
   archive producer and all3 e2e partitions, with complete exact-once inventory.
5. Desktop3 partitions plus multi-folder, browser extension host, WASM server,
   WASI runtime/shutdown and relevant Zed/tree-sitter/editor checks. Run Emacs
   or JetBrains gates if their integrations are changed. Include Python,
   cargo-deny, runtime-rust, real-link, frontends, installer/settings, smoke
   VSIX and standalone EDA where required by the affected shipping closure.
6. Full corpus/performance/retention/cancellation acceptance above, using the
   same final release configuration. Independent semantic equivalence proofs
   must already permit any accepted optimisation patches.
7. Obtain complete final-SHA pre-PR CI validation for all required aggregates
   by dispatching on the candidate without opening a PR. Ensure path
   classification does not silently skip the affected closure. A skipped
   condition is not a pass. Workflow-dispatch checks do not count towards
   required PR checks: after the user asks to open the PR, its actual required
   checks must also pass on the PR/merge candidate before squash merge.

The current baseline has24,312 exact named PASS identities in CI5203, plus
separate host/heavy inventories. Join canonical binary ID and test selector,
features, toolchain and backend. Add separate matching baseline coverage for
unlisted shipping gates; never add overlapping ledger totals. Report all new,
removed, renamed, skipped and unreached identities. Require every relevant
baseline PASS to have a candidate PASS, with narrowly justified test correction
mapping where allowed. No retry, larger timeout, assertion weakening or default
stack change can erase an original failure.

Fix CI result retrieval before relying on another long run: current connector
logs return Transport closed and artifact redirects hit proxy CONNECT403.
The reviewed bounded Nextest capture wrapper still needs a real reporter pilot
before integration; preserve bounded logs, complete selectors and final outcomes
without changing test assertions/budgets. Do not rely on cancelled or partial
jobs to close the ledger. Manual-only fuzz/exhaustive campaigns are not added
to normal gates unless the user explicitly invokes them by name.

## Final review, squash readiness and hand-off

Require an independent review of identity/edit safety and of the full reduced
diff, including removed checks/fixtures, public APIs, docs and comments. Close
all actionable release-scope findings. On the final candidate:

- Refresh origin/rust with minimal history. Reconcile upstream changes; rerun
  affected gates and ensure the selected baseline still matches the merge base.
- Verify no conflict markers, whitespace errors, secrets, probe binaries,
  unintended large files, process ledgers, stale generated copies or links to
  excluded files. Check `git diff --check`, format/check gates, a clean status,
  and regeneration idempotence. Review the actual PR diff/file count/size.
- Validate install/packaging and smoke the built artifact rather than an older
  server found on PATH. Check that required runtime fixtures/assets are shipped.
- Construct a local prospective squash commit/tree on the current rust base
  without merging rust. Verify tree equality with the reviewed candidate,
  no work/evidence ancestry in the proposed change and source-only build
  reproducibility. Required final gates apply to that exact prospective squash
  SHA; reuse tree-equivalent validation only where the applicable gate expressly
  permits it. Always rerun SHA-sensitive artifact/packaging checks. Record
  final-SHA results and the squash-tree mapping on work, and retain the separate
  requirement for actual PR/merge-candidate checks after PR authorisation.
- Prepare a concise PR title/body explaining the user-visible problem, resulting
  behavior, supported scope, restrictions and validation commands/results.
  Keep raw evidence and process history out. The user has now authorised opening the PR once readiness is demonstrated;
  open it only after completing the work and existing release gates.

Only then open the requested PR into rust and attach its URL to this task. Do not merge
or publish. Once the LSP ships under subsequent authorisation, reconcile any durable
release cleanup needed by work and continue the parked migration from its
recorded role briefs without losing in-progress work. Functional fixes will
already have originated on work.

## Checkpoints and remaining decisions

Keep30-minute work snapshots, role freshness, current-state, reports and disk
checks while active. Candidate snapshots require their own clear branch/SHA
mapping; never mix source identities or green claims between the two branches.
All validation records remain on work or external CI, excluded from candidate.

Current external blockers are stale BIGIP/perf handoffs and CI artifact access.
Potential genuinely new decisions are a numeric supported-workload/memory or
long-request envelope, or an unrecorded functional restriction; ask and record
only when needed. Existing naming, ILX, after-load, minify and authenticated
report decisions are settled and are not reopened. Do not promise a date until
the current failure inventory and full corpus measurements establish the work.
