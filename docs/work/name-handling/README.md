# Centralised name handling on work

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

The continuation audit read remote work at
e0277ec42124d26b36cc06daa760de6b21a1decb on 13 October 2026.
Inspect the actual branch head before implementing: source, executable and
oracle receipts certify their captured inputs rather than every later commit.
The audit used read-only sub-agents for consumers, issues and oracle integrity.
The newly started local checkout is older than remote work and Git fetch could
not reach its configured proxy. Updates use the GitHub connection; no fresh
implementation build or test is claimed.

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
