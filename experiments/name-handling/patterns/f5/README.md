# Wild naming patterns: F5 oracle questions

[questions.json](questions.json) maps 45 retained corpus/Wiki cases to their
44 canonical questions, plus five appliance-specific questions from two
pinned public rule-call sites. Each question records CLI, iApp implementation,
TMM `RULE_INIT`, TMM procedure, traffic event, appliance stock tclsh, iCall and
APL separately. All 400 context rows are `not-tested`: there is no accepted
exact-case F5 answer for this bundle. A null `reached` means no measurement;
it does not mean an observed failure to reach the case.

Use the [main request](../../f5/REQUEST.md),
[answer protocol](../../f5/ANSWER-TEMPLATE.md), and
[isolated appliance infrastructure](../../../../scripts/dev/bigip-probes/README.md).
Return answer documents and raw inputs/results to this directory's
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

Probe each feature independently before materialising a context-specific
case. Do not embed unsupported `interp`, TclOO or tcltest branches in a TMM
rule under an assumption that an unexecuted branch or `catch` avoids the loader.
Keep literal load controls separate from dynamically constructed command/script
controls. A dynamic variant has different source bytes and must be retained.

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
and status in `questions.json`; contradictory measurements remain separate.
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
context or source. C Tcl and Jim observations supply no F5 answer.
