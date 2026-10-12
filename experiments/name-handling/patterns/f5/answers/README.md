# F5 wild-pattern answer records

Eleven appliance stock Tcl rows in [the current question bundle](../questions.json)
have accepted exact original-source answers. The
[stock review](stock-original-source-review.md) links 21 successful receipts
across the 8.4/8.5 channels and documents the eleven primary matrix pointers.
The remaining 389 exact question/context rows have no accepted answer yet.

Returned measurements and pending answers are indexed in the
[coverage snapshot](nh20261012coverage1/answers.md), with its
[machine-readable receipts and input hashes](nh20261012coverage1/coverage.json).
It covers every original question/context row and preserves their existing
answer pointers. Original-source candidates require separate review; adapted
inputs and whole-program errors do not fill the original matrix. This immutable
snapshot predates the eleven accepted stock rows; the current question bundle
and stock review carry those updates.

| Retained run | Scope and current limitations |
| --- | --- |
| [nh20261012cap2](nh20261012cap2/ANSWERS.md) | Individual dynamic capability controls; the combined traffic configuration exceeded the MCP string limit, so its traffic lookup did not run. |
| [nh20261012stock2](nh20261012stock2/ANSWERS.md) | Appliance stock 8.4/8.5 portable programs and real-tcltest suite selections. Whole-program errors and unavailable child error options retain their narrower scope. |
| [nh20261012pat1](nh20261012pat1/ANSWERS.md) | Adapted naming reductions in separate setup/final stages; earlier traffic load failure and uninvoked APL presentation callbacks remain explicit. |
| [nh20261012pat3](nh20261012pat3/ANSWERS.md) | Smaller traffic jobs after the earlier oversized configuration; original-name stock stage splits remain catch/frame adaptations. |
| [nh20261012app1](nh20261012app1/ANSWERS.md) | Rejected rule-call configuration attempts. These provide load diagnostics, not reached procedure-path, storage or persistence answers. |

The snapshot includes only retained report files available when it was
generated. Ongoing literal controls, corrected rule-call configurations,
additional capability/traffic jobs and actual APL rendering need their own
reviewed reports and a new immutable coverage snapshot.

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
