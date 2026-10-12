# Current state

Branch: `work`. Latest published checkpoint at this snapshot: `9bcc2c449`.
Inspect git for subsequent commits and uncommitted ownership. This document is
current coordination material; delete it and its references before merge.

## Semantic authorities

The durable entry point is [name-handling.md](../../design/contracts/name-handling.md),
with purpose-specific [command resolution](../../design/contracts/command-resolution.md),
[consumer integrations](../../design/analysis/name-resolution-proofs/consumer-integrations.md)
and the [canonical proof index](../../design/analysis/name-resolution-proofs/README.md).
Source advice, native lookup/object lifetime, completed worlds and edit permission
retain independent authority. Complete original source/channel/configuration/store
currency matters; labels cannot reconstruct slots, frames or recreated objects.

## Executed checks and remaining defects

| Surface | Current evidence | Remaining obligation |
| --- | --- | --- |
| Shared variable declarations | VM/runtime zero/three/four-operand matrices pass against C5/Jim; shared release-selected grammar | Broader native variable/frame/link conformance |
| Shared array modes | 200 oracle comparisons pass; ten populated-regexp refusals explicit | Original regex worker; old-release trace/arity order |
| Real C8.4 suites | Native var/proc/rename pass; six original suites retained | Ten namespace assertion failures; upvar/interp refusals; remaining interp failures |
| Real C8.5 suites | Frozen native var69 pass/17 skip/0 fail; compiled alias controls pass | Namespace failures and unreached cases; broader VM/runtime suite census |
| C8.6 clock registration | Runtime scalar and registry controls each pass1; original map/hooks/epochs retained | Real init refuses; mathfunc topology/hooks, missing support workers and map/worker ordering |
| Native compiled foreach | Same exact input aborts SIGABRT; stock C8.5–9.1/Jim complete; C8.4 missing namespace stops publication | Original iterator ownership; no runtime conformance claim |
| Packaged LSP | Both complete default-C8.6 imported Tcltest regressions pass; two direct C-source controls pass | Post-offset packaged checks pass2/2; #2287 closed against published `c7b94e8fb`; broader consumer census |
| Tcltest diagnostics | Final frozen source: six controls pass; three genuine-missing-read controls fail at C8.4 | Repair actual declared-read/selection boundary; #2286 open |
| Compiler residual fixtures | Three constructors now forward their genuine source inputs; build completes; six seeded-parameter controls fail | Shared procedure lookup fails before SCCP/handler assertions; baseline status unknown; no fallback |
| Backing/generation | 392-command selected-release backing gate,12 backing/21 owner controls, AI drift pass | 165 manifest scope/gate obligations; full required Rust/PR gates |
| Naming proof bindings | Merged checker: 2,327 records, zero pending candidates/unlinked tests | 315 implementation-digest errors across 154 paths; no other error class; scoped review required |

Exact raw results and limits:

- [Original suite census](../../../experiments/name-handling/results/RESULTS.md).
- [Clock, mathfunc and compiled-foreach questions](../../../experiments/name-handling/clock-bootstrap-epoch/registration-repair/README.md).
- [Source namespace consumer](../../../experiments/name-handling/results/c-source-namespace-advice/README.md), including failed attempts and current validation status.
- [Static advice/lifecycle failures](../../../experiments/name-handling/results/static-name-advice/README.md).
- [Real Tcltest caller-frame question](../../../experiments/name-handling/tcltest-shared-frame/README.md).
- [Frozen backing/owner results](../../../experiments/name-handling/results/gate-validation/20261012T161110Z/README.md).
- [Merged proof checker](../../../experiments/name-handling/results/proof-source-currency/20261012-merged-f5-three-renewals/README.md).
- [Reviewed source bindings](evidence-review/); source inspection is not execution.

## Discovery, appliance answers and cleanup

[Wild patterns](../../../experiments/name-handling/patterns/README.md) retain pinned
corpus/Wiki source, exact C/Jim observations and question-led inputs. Eleven
reached original Jim corpus answers are registered canonically; four setup
failures remain unanswered. Adapted Wiki inputs and separate controls do not
answer unchanged original questions. Composed held-out callback acceptance,
Snit/Clay/framework provenance and broader heuristics remain required.

The pushed [F5 bundle](../../../experiments/name-handling/patterns/f5/README.md)
has50 questions/eight contexts:93 observed,244 unreached,63 unavailable.
[Returned coverage](../../../experiments/name-handling/patterns/f5/answers/nh20261012coverage2/answers.md)
and [numbered-question answers](../../../experiments/name-handling/f5/answers/nh20261012coverage2/answers.md)
are integrated from `755f78207`. Review verified 30 seals/26,097 entries and 93
matching input/context pointers with no missing files or hash mismatches. General
[requests](../../../experiments/name-handling/f5/REQUEST.md) and prior BIG-IP
controls remain separate. Admit only exact release/input/channel/reached-event
observations; unreached/unavailable rows remain unanswered. Suspended frames,
restart persistence, other traffic/units and earlier-state cleanup remain open.

Five verified stale F5CLI/editor prose passages and five correction groups in
three F5 architecture documents have been corrected. The final
cleanup remains repository-wide: authored docs, comments, scripts, templates,
generated copies and links, including pre-existing material outside this issue.
Preserve current contracts, minimal how/why, experiments, raw oracle provenance
and conformance tests. Remove obsolete claims, abandoned plans, completed TODOs,
process narratives and this entire temporary work directory before merge.

Use the [ledger](implementation-ledger.md) for ownership/next actions and the
[compact prompt](continue-prompt.md) for resuming small subagent jobs.
