# Continue centralised name handling with sub-agents

Continue the existing work branch of bitwisecook/tcl-lsp. Complete centralised
name handling across the entire codebase, starting from #2286, and then complete
the repository-wide documentation/comment cleanup. Use sub-agents for bounded,
independent work. Implement and validate the work; do not stop after another plan.

## Establish the current branch and obligations

Read AGENTS.md, docs/work/name-handling/README.md, current-state.md,
implementation-ledger.md, plan.md and issues.md. Read the durable entry points:

- docs/design/contracts/name-handling.md
- docs/design/compiler/name-resolution-implementer-guide.md
- docs/design/contracts/resolved-semantic-queries.md
- docs/design/contracts/shared-utility-contracts-rust.md
- docs/design/contracts/naming-consumer-matrix.md
- docs/design/analysis/name-resolution-proofs/README.md and consumer-integrations.md
- experiments/name-handling/README.md, results/RESULTS.md and patterns/README.md

Inspect Git status, branch, HEAD, remote and incoming changes. Stay on work,
preserve other edits and coordinate concurrent chats, including the BIG-IP chat.
The documentation audit read e0277ec42124d26b36cc06daa760de6b21a1decb;
it supplies no fresh execution result. Re-read current tracking and source before
acting. Old target directories, /tmp/name-env.sh, provisioned binaries and
absolute paths in receipts may be absent in a new environment. Discover and
validate them using the repository's supported environment/oracle owners.
Do not restore scrubbed lab, device, credential or certificate material.

## Goal and semantic rules

Create a clean, coherent set of central name tools that every surface uses:
grammar/lexer/parser, dialect and registry metadata, signature scanning,
analyser, compiler/lowering, CFG/SSA/memory SSA, value/type/object inference,
effects/taint/interprocedural summaries, optimisation/inlining, database and
workspace indexing, LSP providers, server middleware, runtimes, all registered
codegen targets, native/WASI/browser hosts, every editor, CLI/debugger/explorer,
F5/EDA/framework tooling, MCP and AI templates/generated guidance.

Reuse existing owner crates and purpose-specific queries. Extend an owner only
where a measured requirement needs it. No second resolver, copied normaliser,
consumer-local command knowledge or catch-all name utility crate. Registry
CommandSpec metadata and overlays/stubs own command-specific roles/semantics;
DSL, renderer, Studio and generated consumers must agree.

Keep written source/evaluated input, namespace identity, command slot, live
command identity, variable place/cell and display text distinct. Flat rendered
FQNs are non-injective: a:/p and a/:p can both display ::a:::p. Never reconstruct
identity, publication operands, cache keys or editable geometry from display
text. Retain exact bytes, spans, provider/release, original source/channel/config,
namespace/path/frame/interpreter context, lifetimes and relevant generations.

Lookup order, variable operation, aliases versus imports, frame reach,
callbacks, namespace deletion/recreation, unknown handlers, method dispatch and
compiler admission are different semantic obligations. Shared policy must
preserve each runtime's actual token/byte/cell storage adapters. A successful
source recogniser is not an entered native frame, selected execution, normal
completion, writable reference or optimisation proof. Missing/stale/foreign/
ambiguous evidence retains a typed residual or justified abstention.

Support familiar dynamic Tcl idioms through common facts: constant and finite
computed names, list-built prefixes, namespace code/inscope, factories,
autoload/source/package, aliases/imports/ensembles, upvar/uplevel links,
TclOO/frameworks and F5 call contexts. Bound recognisers and invalidate their
facts on relevant effects. Keep candidate advice separate from definite
navigation and transformation. Test useful positives and adversarial near misses.

## Coordinate sub-agents

Root owns Git, shared tracking, issue mutations, cross-owner integration and
final review. Give the user regular high-level progress reports: current outcome,
material blockers and next priorities, with failures and unrun checks explicit. At least these responsibilities must be assigned, splitting
further only when independence and capacity permit:

1. Shared names, registry, static source/command/frame producers and #2286.
2. Runtime/VM physical names, cells, arrays, aliases, traces and lifecycle.
3. Compiler middle end and every registered codegen/backend boundary.
4. LSP, workspace/database/middleware, hosts, editors, CLI/MCP/AI consumers.
5. Oracle integrity, retained experiments, pattern coverage and F5 handoffs.
6. Repository-wide documentation/comments cleanup after semantic boundaries
   settle; independent comment-only audits may proceed earlier.

Give each agent a narrow deliverable, exact file ownership, dependency freeze,
expected evidence and permitted edits. No simultaneous writes to shared files.
Allocate exclusive Cargo target caches/worktrees; follow agent-build-env.sh.
Serialize builds sharing a target or source owner. Do not trust a build after
ENOSPC until a clean current rebuild. Freeze compiled dependencies during
captures; restore temporary instrumentation byte-exact and mark its binary
noncurrent. Agents return changed paths, precise acceptance, failures,
unexecuted checks, evidence paths and remaining obligations. Root reviews and
integrates coherent batches, updates current snapshots, commits and pushes
regularly. Development pushes before green gates are already authorised; all
required checks must pass at the PR stage. Do not merge.

## Current priorities to revalidate

- Original C8.6 var-13.2 aborts the Runtime with SIGABRT; C8.6 reports
  passed 1/skipped 109/failed 0. The larger orchestration reached 67 starts
  and 18 skips with no final counts. Inspect the isolated experiment under
  experiments/name-handling/results/runtime-c86-real-suites/array-search-isolation/var-13.2/.
  Investigate actual array-search/element-unset storage/lifetime cause first;
  a start marker or absent failure banner is not success. Keep the proposed
  original-argv WASM change on hold until this blocker is understood.
- #2286 remains OPEN. Current original C8.4 program is one versus two W210
  reads at bytes160–167; namespace import remains zero versus one. Alias setup
  retains all forty cases and shared undefined-read controls pass. Diagnostic79
  times out the unchanged program after180 seconds and grants no acceptance.
  Diagnose actual retained input/read/body producer refusals; preserve every
  reported source, both '$absent' expectations and matching original init/tcltest
  files. Do not fabricate Native witnesses or suppress the original assertions.
- Retired-namespace variable-diagnostic Runtime controls still fail 3/3.
  The caller-context source review proposes preserving selected EVAL_STK
  compilation and ordinary element NameLookup context. A proposal is not a fix.
  Detached-write and expression-source passes have narrower scopes.
- Callback namespace-code selector preservation has two compiler passes;
  after211 catalogue/transition advice and stored callback targets remain
  unresolved. Keep conditional readonly advice independent of execution.
- #2287 and #2300 are CLOSED with scoped published acceptance. Retain their
  fixtures and test broader mutations, release profiles and consumers separately.
  Remove obsolete plan claims that those accepted packaged controls still fail.
- Finish VM OO native-expression/forward/procedure boundaries, middleware
  literal-positive/computed-negative public fixtures, evaluated publication argv
  versus entry key, actual compiler admission, emitted WASM and ABI/lifecycle
  controls, remaining clock/bootstrap dependencies and all inventory rows.
  Avoid turning adjacent dependency repair into an unaudited scope expansion.
- Historical owner counts and proof-digest counts require current review.
  Renew only bindings whose meaning and current substantive acceptance were
  reviewed; never obtain green checks by bulk digest refresh or weaker assertions.

## Inventory and finish every consumer

Use Cargo.toml members, runtime/rust, backend registrations, public provider
rosters, editor integrations, scripts and generators to establish the universe.
Search with rg for colon splitting/trimming, qualified names, tail matching,
proc/class scans, alias walks, reconstructed variable refs, direct registry
lookups, caches and copied resolvers. Inspect semantic purpose and callers:
presentation and ingress operations can be legitimate adapters.

For each call site record file/function, semantic operation, input/output
representations, shared owner/API, full context, relevant lifetime/invalidation,
source spans, consumer purpose, oracle question/result IDs, deterministic tests,
and disposition. Dispositions: verified owner/adapter, migration required,
migrated and verified, measured unsupported with tested rejection, or blocked
on a named question. Unexamined rows are unfinished. Reconcile searches and
every workspace/member/backend/provider/editor/generator against the inventory.
Durable consumer docs retain current integrations; temporary scheduling stays here.

For a consumer, first establish its real public-path positive and discriminating
negative, then repair the producer/owner or adapter where context is lost.
Cover collision, same-tail siblings, empty/literal/qualified names, colon runs,
binary/Unicode extents, links/arrays, source order, shadowing, rename/import
replacement, namespace recreation, stale/foreign source and uncertain effects.
Optimised, interpreted and compiled execution must preserve results and effects.
Test exact spans and refusal through LSP edits, CLI/MCP and middleware where
affected. No source-shape-only test can stand in for the missing semantic result.

## Oracles, experiments and conformance tests

C Tcl: use all five exact release lines 8.4/8.5/8.6/9.0/9.1, pinned by
rust/tcl-dialect/data/reference-toolchains.tsv and validated through
tcl-test-support. At this audit the pins are 8.4.20, 8.5.19, 8.6.18, 9.0.4,
9.1.0. Use required_tclshs(TclVersion::ALL) or the complete-matrix requirement;
absence/invalid configuration is not a green matrix.

If using Tcltest, MUST execute the ORIGINAL init.tcl and tcltest.tcl for the
exact interpreter version, retaining matching all.tcl, selected original test
definitions, support dependencies and constraints. Record paths, source ref,
hashes, binary/library pairing and actual loaded harness. Do not substitute a
minimal startup, rewritten test function, another release's harness, or changed
expected answers. Existing runner file-existence/patchlevel checks alone do not
certify original harness integrity. Validate that boundary and add meaningful
harness integrity checks where needed. Distinguish backend overlays, debugging
and launch contexts from an unchanged upstream baseline. Use --no-overlay for
8.4 with the current run_suites.py; its VM driver is currently fixed to 9.0.
An eventual release-selectable driver requires real original-library proof.

JimTcl: discover latest upstream release/revision at execution time, record exact
ref/commit, reported version, source/binary hashes, configure/build flags and
enabled extensions. The existing pinned 5bac7c99ad65864c87da513e22e2f01703fa4e03
or 0.84 capture does not establish current latest. Use Jim's original
tcltest/testing.tcl wrapper and relevant upstream suites. Record unsupported
operations and setup failures separately from reached lookup answers.

F5 Tcl: read experiments/name-handling/f5/REQUEST.md and answers/README.md,
including returned answers and outstanding-operation-audit.md. Reuse answered
questions; do not rerun the initial request blindly. Prepare exact bounded Tcl
inputs, controls, questions and expected competing outcomes for another chat
with BIG-IP access. Keep appliance tclsh, TMM iRules, MCPD/CLI scripts and iApps
as separate contexts, including release/hotfix/build and actual execution phase.
Never assume appliance tclsh answers TMM, or run Tcltest in TMM/MCPD.

Maintain the request, question IDs, answer template and unanswered context rows
on work. The appliance chat fetches the same branch, runs appropriate isolated
native/event probes, records accepted load versus reached invocation, and
commits answer documents plus safe raw/machine-readable evidence under
experiments/name-handling/f5/answers/. It returns commit/path references.
Before either chat pushes, inspect incoming branch changes and integrate without
overwriting the other's edits. Root verifies exact script/hash/context and
imports measured answers. Continue independent work while F5 is pending.
Original numbered matrices and supplements have different scopes; do not promote
unreached rows or reconcile their counts by copying newer aggregate totals.

Each experiment retains a stable question ID, competing outcomes, exact source,
controls, invocation, hashes, oracle/context/build identity, stdout/stderr,
process exit and Tcl completion/options, results and limitations. Separate
setup rejection, unsupported capability, unreached operation, infrastructure
failure and observed answer. Reduce suspicious results in isolation. Preserve
original evidence; do not overwrite old measurements, capture unrelated session
diaries or infer a native cause from a software control.

Promote deterministic tests only from actual oracle results. Cite observation
ID, input hash, provider/version/context and raw output. Reuse shared vector
formats and prove affected owner/consumer paths, including true negatives and
justified static abstentions. An implementation must never generate its own
expected answer. Retained F5 fixtures must support future regression tests
without live BIG-IP access. Keep generator/fuzz campaigns in the manual tier.

## Documentation and comments; final review

After consumers settle, audit the ENTIRE tracked repository, including files
outside this change: design/KCS/README/glossary, generated docs/prompts,
API/source/test/script comments, templates, editors and fixtures. Correct stale
API claims and obsolete instructions against current code. Consolidate durable
central APIs into useful current contracts and implementer recipes. Update
indexes and generator owners, regenerate and check drift.

Remove development narratives, migration stories, milestones/agent lanes,
completed TODOs, abandoned proposals, temporary state/ledger/continuation files
and links/generated copies. In particular remove docs/work/name-handling and
the work-specific AGENTS exception before the final PR. Keep actual user
procedures and terms that describe current semantics. Comments are minimal
necessary how/why or precise API preconditions; prefer clear names/types/small
functions to paraphrasing code. Preserve legal attribution, executable
experiments, raw measurements, provenance and oracle-derived conformance tests.
Experimental input identity and measured limitations are evidence to retain;
implementation chronology belongs in Git.

Run appropriate scoped checks, owner-resolution, resolution-drift and the naming
proof checker with current substantive acceptance, generated drift/link checks,
make rust-check and make prep-pr. Carry existing host obligations, including
JetBrains SDK checks and Emacs checks when touched. CI carries deep suites;
reproduce failures and fix forward. Record actual failures/unexecuted checks
until resolved. Do not use absent oracles, skips, instrumentation, old binaries
or softened guards as acceptance.

Finish only when every inventory row is accounted for, all supported semantics
have required oracle evidence, positive/negative public-path controls pass,
there is no competing lookup authority or semantic display-name key, and all
required gates pass. Unavailable required evidence stays an explicit blocker.
Review both open and closed issue regressions; GitHub state alone proves neither
a current fix nor a current failure. Refresh issue metadata before any scoped
issue action; avoid misleading closes.

Complete the temporary-doc removal and repository-wide comment/link audit,
then review the final tree AND full branch diff again. A final PR contains
current behaviour, relevant design choices and validation only; no development
history, handoff or progress narrative. No merge is authorised.
