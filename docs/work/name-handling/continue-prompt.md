# Continuation prompt

Copy the following prompt into the chat that will implement the remaining work.
It authorises subagents and keeps one coordinating agent responsible for the
complete result.

---

Continue name-handling centralisation in `bitwisecook/tcl-lsp` on branch `work`.
Work started from #2286. Deliver a clean, coherent set of central name-handling
APIs used across the entire codebase: lexer/syntax, registry/declarations,
analyser, compiler/middle end, every codegen target, both runtimes, LSP and its
middleware/workspace/hosts, editor integrations, dialects/F5, CLI/tooling/MCP,
AI sources and generated prompts. Use subagents actively. Finish the complete
consumer inventory, rather than stopping after a small related fix.

Read `AGENTS.md`, `CONTRIBUTING.md`, the layout/engineering guide and these
branch documents first:

- `docs/design/contracts/name-handling.md`
- `docs/design/contracts/shared-utility-contracts-rust.md`
- `docs/design/analysis/name-resolution.md`
- `docs/design/analysis/name-resolution-c-conformance.md`
- `docs/design/contracts/command-resolution.md`
- `docs/design/contracts/command-binding-and-aliasing.md`
- `docs/design/contracts/runtime-variable-frame-model.md`
- `docs/design/contracts/compiled-scope-and-name-lowering.md`
- `docs/work/name-handling/{current-state,issues,plan}.md`
- `experiments/name-handling/README.md`
- `experiments/name-handling/results/RESULTS.md`
- `experiments/name-handling/f5/REQUEST.md`

The user's `work` instruction overrides the stale AGENTS statement that only
`rust` is active. Preserve existing work and oracle answers. Inspect branch,
status and incoming changes before editing. Refresh the source audit and all
related open/closed issue relationships. Do not interpret an issue marked
closed as proof that every consumer conforms, or an open issue as proof its
reproduction is still broken on this branch. Do not close issues or send other
external messages merely because you have read them.

The key contract is parse source/evaluated written names at ingress with the
selected dialect, retain structured namespace/command/variable identity and
source spans, resolve through shared policy with actual context, render only at
output. Different slots can have the same rendered FQN: `a:` / `p` versus
`a` / `:p`. Never repair that collision with another string canonicaliser.
Do not parse displays back into identities or prove a target by a simple-name
fallback. Uncertain static facts must abstain consistently.

Tcl is a highly dynamic metaprogramming language. Do not make the tool useful
only for literal declarations and direct calls. Build solid shared heuristics
for ordinary Tcl naming idioms: constant/list/format-built names and callbacks,
namespace code/inscope, aliases/imports/ensembles, frame-forwarding wrappers,
package/autoload/source conventions, finite procedure factories, object-handle
provenance and supported framework definers. Use the registry, binding model,
value provenance and shared name/frame owners, with context, effects, budgets
and invalidation. A dynamic source shape is not automatically unknowable.

Distinguish proved identity, conditional identity with explicit assumptions,
plausible candidates and unresolved/ambiguous results. Useful heuristics may
guide completion and explicit candidate navigation; they cannot silently
authorise rename, reference rewriting, constant folding or direct binding.
Prove the constructed value and the call-time target separately. Do not infer
uniqueness from an incomplete index or report a definitely missing variable
when its frame/effects are unknown. Demur with a recorded reason where proof
is unavailable, preserving unaffected facts rather than suppressing everything.
Use the dynamic-pattern acceptance matrix in
`experiments/name-handling/case-catalogue.md` to test both useful coverage and
false target selection. This is required completion work, not optional polish.

Use and extend existing owner crates, not a second name system. Preserve
command slots versus live command objects, interpreter domains, frames and
variable links, imports versus late-bound aliases, mutation ordering,
namespace path, retained/dead namespace lifetime, generation/cache guards,
array/literal/empty names, byte values where supported and source span widths.
Relative-qualified `n::x` is not a frame local solely because it lacks leading
`::`. Top-level `x` and `::x` may identify the same storage place. Runtime
arenas and byte namespace trees need state adapters, not flattening to FQNs.
Command-specific roles and knowledge stay in the registry, including overlays
and stubs; consumers apply the declared facts generically.

First reproduce #2286's real `tcltest` setup/body/cleanup scope and binding
problem. Keep both genuine missing-read and false-positive cases. Do not add
blanket W210 suppression or a command-name special case. Then carry the shared
identity/context through every consumer listed in the plan, including older
closed issues' regressions and the unresolved structural identity work in
#1778. Validate actual direct/AOT/native code paths before claiming all codegen
uses runtime dispatch.

Coordinate subagents in bounded, non-overlapping lanes:

1. Owner/API and identity lane: written parsing, structured slots/paths,
   variable places, registry boundary, policy adapters and enforcement.
2. Static producer/compiler lane: signature scan/analyser, binding facts,
   scope/alias/summary propagation, SSA/dataflow/optimisers/type inference and
   #2286 regression. Split producers and middle end once interfaces settle.
3. Runtime/backend lane: VM and WASM command/namespace/frame stores, lifecycle,
   traces/imports/aliases/OO/mathfunc, all codegen targets and invalidation.
4. LSP/middleware/tooling lane: providers, workspace/server settlement,
   rename/refactors/spans, hosts/editor consumers, CLI/MCP/AI guidance.
5. Oracle lane: C matrix, latest Jim, retained experiments, expected-vector
   promotion and prepared BIG-IP questions/returned-answer integration.
6. Independent review/docs lane: adversarial collision/mutation/context tests,
   complete consumer reconciliation and present-state documentation scrub.

The coordinator owns shared API edits and integration. Start independent audits
in parallel, agree interfaces before dependent edits, assign explicit file
ownership and prevent concurrent edits to shared files. Respect slot limits;
reuse agents for later lanes. Never share Cargo targets across concurrently
building worktrees; use `scripts/dev/agent-build-env.sh`. Worktrees may isolate
lanes, but integrate all accepted changes into `work`. A subagent reports
findings, tests, remaining consumer rows and evidence paths; its report does
not by itself establish completion. Independently review each integrated lane.

Maintain a complete per-call-site ledger as described in `plan.md`. Audit every
workspace crate, `runtime/rust`, all registered backends, scripts/editor/AI
sources and generator templates. Classify raw colon operations by their input
representation and purpose; grep counts and existing narrow drift gates are
not proof of centralisation. A tested restricted feature may be unsupported;
an unaudited consumer is unfinished.

Oracles are real C Tcl 8.4–9.1, the latest upstream JimTcl, and F5 Tcl measured
on BIG-IP. C pins come from `rust/tcl-dialect/data/reference-toolchains.tsv`:
require the complete five-release matrix using shared test support. Set
`TCL_LSP_REQUIRE_TCLSH=1` for witness-based callers. Current syntax vector
runners use `available_tclshs()` and skip missing/invalid binaries without
honouring that variable: add an explicit all-five preflight/assertion or use
`witness_tclsh` for every `TclVersion::ALL` entry. Determine and record the
latest Jim revision and
build configuration rather than treating an old 0.84 claim as current. Preserve
exact programs, provenance, stdout/stderr bytes, completion/error details and
capability status. Different releases/dialects/contexts can differ; record
observed expectations explicitly. Never bless our own implementation by using
its output as the oracle.

Use C Tcl's upstream `tcltest` cases on each matching real C release, or Jim's
equivalent upstream suite on the identified Jim build, as oracle evidence
where useful. Reuse relevant cases before inventing bespoke probes. Retain
suite revision, file/case IDs, exact definitions, setup/cleanup, dependencies,
constraints and raw pass/fail/skip transcripts. Skipped cases are not measured
answers. Use `tcl-test-support::upstream_test_definition` and validated
source-tree execution for C cases where applicable; preserve Jim's own harness
and expectations. Running the harness on our implementation validates a
consumer and does not create an independent oracle. Reduce measured suite
cases into shared vectors without changing their semantic question.

Keep all experiments and measurements. Use the existing vector renderers and
BIG-IP probe infrastructure where suitable. Promote deterministic regression
expectations only from identified real-oracle records; cover owner and each
consumer plus public LSP/CLI/MCP/compiled outcomes. No oracle available means
pending/unavailable, not a pass. Manual generated/fuzz campaigns follow AGENTS.

This chat does not have BIG-IP access. Prepare executable probes and exact
questions in `experiments/name-handling/f5/REQUEST.md`, using its IDs and answer
protocol. The user will give them to the chat with appliance access. That chat
returns answer documents/raw transcripts to `work` under
`experiments/name-handling/f5/answers/`. Ingest them without overwriting their
provenance. Appliance tclsh is a control; TMM, CLI script and iApp are separate
contexts. Distinguish load acceptance from runtime semantics and named-command
from dynamically constructed lookup. Continue independent work while waiting;
report the precise unanswered questions instead of inventing F5 behaviour.

Update durable contracts as implementation changes. Then scrub the entire
codebase's documentation and comments so they describe only current semantics,
ownership, public preconditions, actual limitations and necessary how/why.
Comments must be minimal; make the code itself clear and self describing.
Remove development process/history, milestones, phase diaries, past-bug
narratives, handoffs and completed TODOs from final shipped docs and comments.
Retain legal/machine markers and reproducible experimental evidence. Regenerate
all generated docs, prompts and catalogues from their owners. Resolve the
specific contradictory claims listed in `current-state.md` using code/oracles.

Before final delivery reconcile every consumer row, related issue reproduction,
required oracle record, API contract and gate. Run focused deterministic tests
for each semantic change, `make rust-check`, and `make prep-pr` before each push;
fix failures and include generated/formatting changes. CI owns deep tiers;
address its failures when a PR is requested or already exists. Do not run
unnecessary full-suite loops after appropriate checks pass.

When all obligations are complete, remove `docs/work/name-handling/` and its
temporary references from the final diff. Preserve
`docs/design/contracts/name-handling.md`, accurate existing contracts, retained
experiments, BIG-IP answer documents and real-oracle conformance tests.
Development history belongs in git. Any final PR describes the concrete final
problem/behaviour, material design tradeoffs and validation, never a sequence
of phases, slices, agents or abandoned approaches. Do not open or merge a PR
unless requested. Report what actually passed and any exact remaining blocker;
do not label the programme complete with unexplored consumers or missing oracle
answers. Persist autonomously through all independent authorised work.
