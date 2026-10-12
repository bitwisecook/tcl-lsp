# Name-handling work on `work`

Complete centralised name handling across every supported surface, with real
C Tcl 8.4–9.1, latest JimTcl and F5 oracle evidence. The starting report is
[#2286](https://github.com/bitwisecook/tcl-lsp/issues/2286), concerning W210
across `tcltest` setup/body/cleanup execution. Its reproduction must be covered
alongside the broader command, namespace, variable and object identity work.

Current source basis: merged HEAD `c49ef6fd5`, 12 October 2026. Earlier
source audits and retained execution receipts remain separate baselines;
current execution must be recorded before calling their gaps resolved.

| Document | Purpose |
| --- | --- |
| [Shared contract](../../design/contracts/name-handling.md) | Durable entry point for agents and all consumers; existing APIs and representation rules |
| [Current state](current-state.md) | Verified source audit, gaps and misleading claims |
| [Issues](issues.md) | Related open/closed issues, dependencies and branch evidence rules |
| [Issue action proposals](issue-actions.md) | Exact closure/update text and current validation requirements |
| [Canonical naming proofs](../../design/analysis/name-resolution-proofs/README.md) | Sole question/evidence index and provider-specific proof authority |
| [Consumer integration contracts](../../design/analysis/name-resolution-proofs/consumer-integrations.md) | Purpose-specific source, lookup, runtime and edit obligations |
| [Corpus findings](../../../experiments/name-handling/patterns/corpus-findings.md) | Pinned idioms, authored probes, exact observations and limits |
| [Completion plan](plan.md) | Every consumer group, ordered work and acceptance criteria |
| [Continuation prompt](continue-prompt.md) | Complete prompt for a coordinating agent using subagents |
| [Measured engine gaps](../../../experiments/name-handling/results/RESULTS.md) | Real upstream suite outcomes and prioritised live-engine work |
| [Oracle protocol](../../../experiments/name-handling/README.md) | Durable scripts, measurements, provenance and test promotion |
| [Wild-pattern catalogue](../../../experiments/name-handling/patterns/README.md) | Maintained real-world patterns, exact experiments and C/Jim/F5 evidence matrix |
| [BIG-IP handoff](../../../experiments/name-handling/f5/REQUEST.md) | Questions and return format for the chat with appliance access |

## Lifecycle

This directory is temporary coordination material on `work`. Keep the current
state and remaining obligations accurate while work is active. Do not append
session narratives, completed-step diaries or abandoned approaches.

At completion remove this directory and its temporary links from the final
branch diff. Keep the current architecture/API contract, executable
experiments, raw measurements, BIG-IP answers and conformance fixtures.
Experiments preserve reproducible evidence, not development history. Git owns
implementation history. The final PR describes the resulting behaviour,
relevant design choices and validation; it contains no migration diary,
agent handoff, phase history or progress bookkeeping.

An issue's GitHub state is separate from implementation evidence on `work`.
An open issue may have a branch fix; a closed issue may still have a missed
consumer. Reproduce, inspect and test before making either conclusion.
