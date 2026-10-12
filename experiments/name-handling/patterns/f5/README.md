# Wild naming patterns: F5 oracle questions

[questions.json](questions.json) maps 45 retained corpus/Wiki cases to their
44 canonical questions, plus five appliance-specific questions from two
pinned public rule-call sites. Each question records CLI, iApp implementation,
TMM `RULE_INIT`, TMM procedure, traffic event, appliance stock tclsh, iCall and
APL separately. Accepted original-source answers and measured attempts are distinct
in the [current matrix](questions.json). The [hosted review](answers/hosted-original-source-review.md),
[stock corpus review](answers/stock-original-source-review.md) and
[stock Wiki review](answers/stock-wiki-review.md) identify exact accepted inputs
and primary pointers. `attempt_record` retains measured guard/setup/program
boundaries while `answer_record` remains null when the original final lookup
is unreached or its reachability is unproved. Related adapted controls have
independent variant scope.

The [original 400-row matrix](questions.json) retains **93 observed, 244 unreached
and 63 unavailable** question/context rows with their exact source, attempt and
answer identities. The [answer directory](answers/README.md) indexes retained
controls and reports. The [supplemental investigation index](answers/finish-admission-supplement.json)
links 307 completed investigation classifications without changing original
coverage or claiming new original-program execution.

The supplement gives 15 native composite observations and 25 measured native
`call`-absence controls distinct derived question IDs, exact source/record
selectors and bounded context scope. The composite source/configuration/case
hashes differ from the original materialisations. Absence controls establish
only the required native dispatch prerequisite in the measured contexts.
The 267 classifications whose final operations are unreached retain null
final answers; diagnostic children and adapted controls remain separate.

The sealed upstream [readable conclusions](answers/nh20261012finish1/ANSWERS.md),
[resolutions](answers/nh20261012finish1/resolutions.json),
[proposal](answers/nh20261012finish1/proposed-questions.json) and
[validation](answers/nh20261012finish1/validation.json) are preserved as derived
claims. Their proposed 108/267/25 matrix is not admitted as fixed original-input
coverage. Follow original `answer_record` or `attempt_record` pointers in
`questions.json`, and supplemental `resolution` pointers for investigation scope.
The [historical coverage snapshot 2](answers/nh20261012coverage2/answers.md) and
[JSON inventory](answers/nh20261012coverage2/coverage.json) remain unchanged.
The [portable report index](../../f5/answers/README.md#portable-report-index)
links retained input reports and raw records.
[Historical snapshot 1](answers/nh20261012coverage1/answers.md) predates these updates.

At commit `755f78207b993cdfa4b0fd230abcafb1f3f606bc`, all 30 retained
`MANIFEST.sha256` seals across the two F5 answer trees verified: 26,097 listed
entries, with no missing or mismatching blobs. The
[numbered-question snapshot](../../f5/answers/nh20261012coverage2/answers.md)
retains F5-N01–F5-N16 bounded variants and remaining questions. Suspended
frames, restart/reload after deletion, other traffic contexts and cleanup of
earlier state remain separate evidence limits.

Use the [main request](../../f5/REQUEST.md),
[answer protocol](../../f5/ANSWER-TEMPLATE.md), and
[isolated appliance infrastructure](../../../../scripts/dev/bigip-probes/README.md).
Literal loader controls and actual APL callbacks have separate [retained reports](answers/README.md); dynamic source evaluation does not stand in for literal acceptance. Return answer documents and raw inputs/results to this directory's
[answers](answers/README.md) on branch `work`. The appliance chat owns actual
execution; this bundle contains no appliance runner or deployment instruction.

## Exact questions and inputs

Keep the `canonical_question_id` and `case_id` unchanged. The corpus entries
identify complete authored reductions. Wiki entries identify the complete
suite and a selected test range; the range depends on `isolated`, real tcltest,
constraints and suite setup. It is not a standalone iRule. The `wiki-code-6`
and `wiki-code-7` cases share the canonical `naming.wiki.code-6` question and
measure different release constraints. Both need independent records.

Retain each actual input and its SHA-256, including the complete wrapper,
encoded script, generated configuration and invocation. When adapting a
portable case, give the adaptation a separate `variant_id` and describe the
changed isolation, frame, namespace, availability or input channel. An
adaptation cannot fill the exact portable-input answer merely because it has
the same expected value. The five `corpus-f5-*` questions need newly authored,
retained provider/caller definitions and isolated object names; the public
object names in the source links are evidence of an idiom, not deployment names.

## Context and execution boundaries

`tcltest` cannot run in TMM or MCPD processes hosting F5 Tcl. F5 host evidence
uses small native probes in the actual event, rule or hosted interpreter
context; do not inject a compatibility harness. Standalone C Tcl controls use
their real initialisation and `tcltest`; Jim uses equivalent independent
controls. Appliance stock C interpreters remain distinct from F5 host Tcl.
Historical harness/helper failures remain attempt evidence: an unreached
generic helper is not proof that the underlying Tcl feature is absent.

Probe each feature independently before materialising a context-specific
case. Do not embed unsupported `interp`, TclOO or tcltest branches in a TMM
rule under an assumption that an unexecuted branch or `catch` avoids the loader.
Keep literal load controls separate from dynamically constructed command/script
controls. A dynamic variant has different source bytes and must be retained.

The prepared `campaign.py --exact-hosted` mode keeps original operation bytes
and names, guards their absence, and retains each whole-source wrapper. CLI,
iApp implementation and iCall are defaults; TMM `RULE_INIT`, procedure and
traffic are explicitly selectable. TMM setup and original source travel as
retained ASCII hex decoded with `binary format H*` and evaluated in the actual
host frame. This dynamic source channel supplies no literal iRule loader
answer. Each case is a separate configuration job, and runtime cleanup deletes
only exact original names whose absence was proved before execution. Missing
guard capabilities, collisions or Wiki package failure leave the source
unreached. A traffic request supplies its own unit/invocation receipt, not a
claim that every TMM executed.

An iRule load result is independent of `RULE_INIT` or a reached traffic event.
For a TMM procedure record its invoking event or `RULE_INIT`, caller/provider
rule identities and exact `call` route. A CLI wrapper's execution namespace,
iApp execution action, iCall invocation and APL reachability are separate
inputs. Appliance stock tclsh is a control provider, not a TMM answer.

Record version/build/hotfix, profiles, attachment, TMM roster and actual unit
IDs. Preserve repeated state-dependent results and missing responses. Never
infer all-TMM execution from one response or a successful load. Keep cleanup
proof and configuration-save status with the run as the main protocol requires.

## Answer states

Use `not-tested`, `observed`, `unsupported`, `unreached`, `unavailable`, or
`infrastructure-failure`. An `observed` runtime error is an observation if the
intended operation ran; it is not automatically `unsupported`. A failed setup
leaves the intended lookup `unreached`. A command-availability control may prove
`unsupported` for that exact version/context/input channel; a constraint skip
alone does not. Keep load status, reached status, completion code, result bytes
and error options in separate fields. Mark unavailable error options explicitly.

The [answer template](answer-template.md) includes an individual machine-readable
record shape. Add immutable run directories rather than overwriting prior
answers. Update only the answered question/context's `answer_record` pointer
and status; retain non-answering measured attempts in a separate `attempt_record` in `questions.json`; contradictory measurements remain separate.
Before promoting a canonical provider answer, verify exact question, original
input/wrapper hashes, version, context, load and reached operation. A successful
reduction cannot establish corpus prevalence or static recogniser coverage.

## Related measured controls

These records provide useful controls and appliance methods. They do not
answer the bundle's exact portable cases or its unmaterialised variants.

| Existing proof | Relevant boundary |
| --- | --- |
| [Procedure owner route](../../../../docs/design/analysis/name-resolution-proofs/bigip-procedure-owner-route.md) | Owner-local and partition/folder `call` routes, provider configuration deletion and event frames. |
| [Activated procedure replacement](../../../../docs/design/analysis/name-resolution-proofs/bigip-activated-procedure-replacement.md) | Fresh calls after accepted deletion/replacement and rejected replacement. |
| [Namespace/command currency](../../../../docs/design/analysis/name-resolution-proofs/bigip-namespace-command-currency.md) | Exact dynamic namespace/proc/rename controls and recorded context differences. |
| [Dynamic resolution controls](../../../../docs/design/analysis/name-resolution-proofs/bigip-dynamic-resolution-controls.md) | Literal loader versus dynamic runtime lookup; repeats, links, aliases and namespace path capability. |
| [Literal head loader boundary](../../../../docs/design/analysis/name-resolution-proofs/bigip-literal-head-loader-boundary.md) | Accepted/rejected canonical sources and never-executed cases. |
| [Static state locality](../../../../docs/design/analysis/name-resolution-proofs/bigip-static-state-tmm-locality.md) | Actually reached TMMs, write/unset/recreate and rule recreation. |

The existing controls report BIG-IP 21.1.0.1 build 0.0.26, Point Release 1 in
their explicitly recorded contexts. Do not extrapolate to another build,
context or source. Desktop C Tcl and Jim observations supply no F5 answer.
