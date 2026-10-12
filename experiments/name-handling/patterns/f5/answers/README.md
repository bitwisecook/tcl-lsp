# F5 wild-pattern answer records

The [current question bundle](../questions.json) distinguishes accepted original
answers from measured attempts and related variants. [Hosted original-source
review](hosted-original-source-review.md) covers actual CLI, iApp implementation,
iCall, APL and TMM observations. [Stock corpus review](stock-original-source-review.md)
and [stock Wiki review](stock-wiki-review.md) retain the stock interpreter results.
Every accepted pointer carries the sealed records file SHA-256 and exact JSON
pointer. `attempt_record` supplies measured boundary evidence with a null
`answer_record` when final-operation authority is absent.

Returned measurements and pending answers are indexed in the
[current coverage snapshot](nh20261012coverage2/answers.md), with its
[machine-readable receipts and input hashes](nh20261012coverage2/coverage.json).
It covers every original question/context row and preserves their existing
answer pointers. Original-source candidates require separate review; adapted
inputs and whole-program errors do not fill the original matrix. This current snapshot preserves accepted original pointers and measured attempts separately, including the completed persistence and namespace-owner controls. [Historical snapshot 1](nh20261012coverage1/answers.md) remains unchanged.

| Retained run | Scope and current limitations |
| --- | --- |
| [nh20261012cap2](nh20261012cap2/ANSWERS.md) | Individual dynamic capability controls; the combined traffic configuration exceeded the MCP string limit, so its traffic lookup did not run. |
| [nh20261012stock2](nh20261012stock2/ANSWERS.md) | Appliance stock 8.4/8.5 portable programs and real-tcltest suite selections. Whole-program errors and unavailable child error options retain their narrower scope. |
| [nh20261012pat1](nh20261012pat1/ANSWERS.md) | Adapted naming reductions in separate setup/final stages; earlier traffic load failure and uninvoked APL presentation callbacks remain explicit. |
| [nh20261012pat3](nh20261012pat3/ANSWERS.md) | Smaller traffic jobs after the earlier oversized configuration; original-name stock stage splits remain catch/frame adaptations. |
| [nh20261012literal1](../../../f5/answers/nh20261012literal1/ANSWERS.md) | Separate literal loader/capability inputs; their actual configuration acceptance remains distinct from dynamically evaluated controls. |
| [nh20261012aplexact1](nh20261012aplexact1/ANSWERS.md) | Actual browser-selected APL original-source callbacks; reviewed original final errors/successes and real-suite guard failures are independent of implementation callbacks. |
| [nh20261012aplcap1](nh20261012aplcap1/ANSWERS.md) | Actual browser-selected APL command capability inputs and own interpreter version. |
| [nh20261012app1](nh20261012app1/ANSWERS.md) | Rejected rule-call configuration attempts. These provide load diagnostics, not reached procedure-path, storage or persistence answers. |

The snapshot includes only retained report files available when it was
generated. Later literal controls, rule-call configurations, capability/traffic jobs and
actual APL rendering have separate reviewed run reports; a new immutable
coverage snapshot records only those reports available when generated.

Retain new measurements in immutable `<run-id>/` directories, including
`answers.md`, individual JSON records, actual materialised source/wrappers,
provider metadata and raw load/runtime/cleanup transcripts. Use the
[answer template](../answer-template.md) and
[main appliance protocol](../../../f5/REQUEST.md). Return the records to branch
`work`; update the corresponding bundle pointers only after verifying question,
input hashes, context and the actually reached operation.

Related appliance controls linked from the bundle README stay under their
original evidence paths. They are not copied here or relabelled as answers to
these portable cases.

[Native appliance-question and N11/N13 review](native-appliance-question-review.md) retains existing canonical controls, new bounded materialisations and explicit capability/persistence boundaries.

[Stock final-error review](stock-final-review.md) retains source-reviewed original near-error final operations from sealed stock2 captures without claiming a new execution.

[Bounded namespace-owner control](../../../f5/answers/nh20261012nsowner1/README.md)
and [actual APL callback](../../../f5/answers/nh20261012aplnsowner1/ANSWERS.md)
retain new F5-N02 inputs with guarded unique candidate owners. Their measured
owner distinction supplies no replacement original-source answer or total
interpreter-cleanup premise.
