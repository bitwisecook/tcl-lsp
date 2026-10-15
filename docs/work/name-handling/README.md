# Centralised name handling on work

Current continuation: published `551f3b2c1`; fresh complete ordinary build succeeds, original #2286 first PASS68.978s/180s and both readonly Authored controls PASS. Full replays are running, unsealed, while the reviewed second batch awaits Rust validation. Read [current-state.md](current-state.md) before the historical capture descriptions below.

Complete one coherent set of name representations, lookup policies and
purpose-specific queries across the entire codebase. The starting report is
[#2286](https://github.com/bitwisecook/tcl-lsp/issues/2286): W210 must understand
the actual setup/body/cleanup frame of a resolved Tcltest invocation, while
still reporting genuine missing reads. The wider goal includes every command,
namespace, variable, array, link, procedure, class, method and callback consumer.

Central owners retain source bytes and spans, dialect/release, interpreter,
namespace/path/frame context, actual identity and relevant lifetimes. Consumers
select the query for their purpose and adapt its output. Presentation is not
identity. Conditional source advice, native allocation evidence, completed
execution and permission to edit or optimise are separate facts. Ordinary
dynamic Tcl idioms need useful shared analysis and explicit abstention when
evidence cannot establish a target.

## Read and continue

| Document | Purpose |
| --- | --- |
| [Continuation prompt](continue-prompt.md) | Complete instructions for a coordinator and sub-agents |
| [Current state](current-state.md) | Scoped measured acceptance, failures and exact evidence |
| [Completion plan](plan.md) | Every consumer group, inventory requirements and acceptance |
| [Implementation ledger](implementation-ledger.md) | Remaining obligations and next bounded work |
| [Call-site inventory](consumer-inventory.md) | Bounded inspected consumers, shared APIs, actual validation and unexamined obligations |
| [Related issues](issues.md) | Open and closed issues; GitHub status is separate from branch acceptance |
| [Issue action proposals](issue-actions.md) | Existing scoped acceptance requirements |
| [Name-handling contract](../../design/contracts/name-handling.md) | Durable entry point and representation/context rules |
| [Implementer guide](../../design/compiler/name-resolution-implementer-guide.md) | Existing purpose-specific APIs and consumer recipes |
| [Semantic queries](../../design/contracts/resolved-semantic-queries.md) | Shared evidence and query contracts |
| [Consumer matrix](../../design/contracts/naming-consumer-matrix.md) | Durable consumer integrations |
| [Canonical proofs](../../design/analysis/name-resolution-proofs/README.md) | Semantic questions and scoped provider evidence |
| [Oracle experiments](../../../experiments/name-handling/README.md) | Retained scripts, measurements and test promotion |
| [Measured engine gaps](../../../experiments/name-handling/results/RESULTS.md) | Original suite outcomes and limitations |
| [Pattern catalogue](../../../experiments/name-handling/patterns/README.md) | Corpus/Wiki idioms, positive/adversarial inputs and evidence |
| [BIG-IP request](../../../experiments/name-handling/f5/REQUEST.md) | Questions for the chat with appliance access |
| [BIG-IP answers](../../../experiments/name-handling/f5/answers/README.md) | Returned answers and outstanding operation audit |

Source238 is adopted on `work`; checkpoint `4e20920a5` is push-confirmed. Source232 owner clauses and the219 RHS catalogue are applied. Parent/Text sources237/238 are integrated; the fresh selected ordinary272 results below now cover source238. Native Runtime/VM acceptance remains separate. Ordinary271 failed its fresh build with exit101, Registry/Compiler rustc SIGKILL9, seven partial ELFs and zero tests; source drift{}. That failed271 attempt executed no tests. Fresh ordinary272 on actual source238 seals956 controls: 643 PASS, 312 FAIL, 1 TIMEOUT. Its single-job fresh build passes in348.483727 seconds with14 test ELFs, errors[] and unchanged authored source, executing zero tests during the build. All12 selected scopes retain unchanged captured source/binaries, exact single runs and no missing/empty filters. Every265929 selector remains with1 gain and0 prior-PASS regressions/drops; eight additions within retained scopes are7 PASS/1 FAIL; the independent IRules19 scope is16 PASS/3 FAIL. Original#2286 runs first and passes72.400995379s under the unchanged180-second budget. Runtime174/VM133 and formatter88/integration55/15 annexes remain unexecuted plans; normal owner CLI250 still197 diagnostics. The cgroup OOM counters are cumulative without a before/after cause baseline; do not claim measured causation. Runtime174 and VM133 are fully peered future plans, unexecuted.

TEMP270 built successfully and ran twelve exact diagnostic controls once: eight Compiler failures and four DB failures, no timeouts or source/binary drift. All three temporary files and97,819 authored inputs restored exactly at08:45:43.133493 UTC, changed inputs{}. Diagnostic results establish first guards only; they provide no ordinary acceptance.

The canonical continuation is [continue-prompt.md](continue-prompt.md). Portable unfinished work and exact future plans are indexed in [handoff/README.md](handoff/README.md). Prior source audits and issue/provider rosters retain their original capture dates. Current implementation, remaining naming-consumer owners and validation limits are recorded in [current-state.md](current-state.md) and [implementation-ledger.md](implementation-ledger.md).

## Acceptance and document lifecycle

Use C Tcl 8.4–9.1, the latest identified upstream JimTcl, and actually reached
F5 execution contexts as independent oracles. When using Tcltest, run the
original init.tcl and tcltest.tcl belonging to that exact C interpreter version,
with original suite/support inputs. Retain executable experiments, raw results,
provenance, answer documents and deterministic conformance tests derived from
those observed results.

This directory is temporary coordination material. Maintain a compact current
snapshot and unresolved obligations while work continues. The durable
architecture and API documentation must describe the current implementation,
with limitations made explicit. Before the final PR, remove this directory,
all temporary plans/prompts/ledgers/proposals elsewhere, their links and generated
copies, and the work-specific AGENTS push exception. Audit all documentation and
comments repository-wide, including pre-existing files. Keep minimal necessary
how/why comments, current public contracts and self-describing code.

Preserve reproducible semantic evidence and legal attribution. Do not restore
previously scrubbed lab/device/security material. Development history belongs
in Git; the final PR describes the resulting behaviour and validation.

Every consumer must be accounted for and verified, or have a measured,
documented unsupported boundary with tested refusal. An unexamined consumer,
unavailable required oracle, failing check or unanswered supported F5 operation
prevents completion. Regular reviewed development pushes are authorised on work;
all required checks must pass at the PR stage. No merge is requested.

Final source238 validation: [ordinary272 current snapshot](current-state.md#current272-selected-scopes); exact portable receipts and unfinished owner work are indexed in [handoff/README.md](handoff/README.md). Historical scope in this document remains bounded to its stated capture.
