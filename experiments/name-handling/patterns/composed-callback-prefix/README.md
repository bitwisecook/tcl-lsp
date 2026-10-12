# Does a composed captured callback retain its source selector?

These new authored inputs compose an alias to the list builder, `namespace code`
with `my`, and a one-hop same-frame callback variable. They challenge the pair
proposed in the [callback acceptance review](../../../../docs/work/name-handling/evidence-review/callback-pattern-acceptance.md).
They are supplemental experiments, unregistered in the canonical proof owner;
all existing questions and answers retain their original identities and inputs.

The source question is whether the original `tick` selector can supply a
conditional readonly reference when all builder, callback role, receiver,
source/configuration and write premises are present, and whether an unknown
writer withdraws that reference. The separate native question is: **what setup
completion, scheduled callback error and final observed-variable lookup does
this exact source produce?** Native execution does not answer the static
consumer question or grant future lookup, rename or optimisation authority.

## Which sources motivate these inputs?

[Source receipts](sources.json) reuse maintained tcllib revision
`8a2e288fa02363639784c2f0a8ea6245896c2884`: `corpus-snit-alias` at
`modules/interp/interp.tcl:66–81` and `corpus-captured-my` at
`modules/httpd/build/server.tcl:361–385`. The local full files, selected ranges
and `license.terms` match the existing retained hashes. The Snit child-interpreter
builder and httpd socket/cron callback supply separate ingredients; neither
upstream site contains this exact composition. Snit/Clay definitions, actual
socket argument injection, cron lifecycle and child-interpreter context remain
unmeasured.

The same receipt links retained Wiki command-prefix, namespace-code,
interp-alias, TclOO and callback page metadata/example hashes. These are existing
retrieval snapshots; their displayed update times do not establish a verified
revision. The existing Wiki receipts contain no inspected licence, and this
experiment vendors no Wiki text. Inputs are authored repository fixtures. This
selection establishes no frequency or prevalence.

## What exact inputs were executed?

[Positive](probes/positive.tcl) stores the aliased list-produced `my tick {a b}`
under a namespace capture and schedules it with `after idle`. `tick` records its
fixed data so the final source line can read the observed value after `update`.
[Unknown writer](probes/unknown-writer.tcl) takes `destination` and executes
`set $destination {my unrelated}` between the stored prefix and its sink. Its
exact runtime invocation passes `cb`, replacing that cell and losing namespace
capture. The static method body has no literal destination and must withhold
the old stored selector. These fixtures use a normal exported lowercase method;
they exercise internal `my` dispatch but do not test private export enforcement.

Only the final physical source line is the native question; preceding lines are
setup. Both source sections execute unchanged on every provider. The driver
installs a `bgerror` observer before source setup and retains its message,
errorCode and errorInfo separately from setup/final completion and catch return
options. A successful `update` or process exit cannot turn a failed callback
into a successful callback observation. No Tcltest wrapper is part of these
inputs, and no mock callback host is used.

## What did native interpreters actually observe?

| Provider | Positive | Unknown writer |
| --- | --- | --- |
| C Tcl 8.4.20 | setup error `invalid command name "oo::class"`; question/callback unreached | same setup error; question/callback unreached |
| C Tcl 8.5.19 | same missing TclOO setup error; question/callback unreached | same setup error; question/callback unreached |
| C Tcl 8.6.18 | question `0: tick {a b}`; no background error | callback error `invalid command name "my"`; final question `1: can't read "::observed": no such variable` |
| C Tcl 9.0.4 | same reached positive value | same reached callback and final errors |
| C Tcl 9.1.0 | same reached positive value | same reached callback and final errors |
| Jim `0.84-9-g5bac7c9` | setup error `wrong # args: should be "interp"`; class/question/callback unreached | same alias-API setup error; class/question/callback unreached |
| BIG-IP/F5 | unanswered in each separately requested host/context | unanswered in each separately requested host/context |

[C5 records](results/records.json) retain the five native C binaries and their
matching `TCL_LIBRARY`, init and namespace-source hashes in
[provenance](results/provenance.json). Each case retains source copy, exact
wrapper bytes, raw stdout/stderr, catch options and separate background errors.
C8.4 has no three-variable catch-options API; its errorCode/errorInfo are retained.
All C processes exited zero, which says only that the capture driver completed.

[Final Jim records](jim-final-results/records.json) and
[provenance](jim-final-results/provenance.json) pin the previously retained
upstream revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, matching binary,
native `tests/testing.tcl`/`tcltest.tcl`, embedded init source and build
configuration hashes. No C Tcl startup library is supplied. This reuses the
prior upstream-HEAD check; no new remote HEAD check occurred. No Jim-native
alias or OO adaptation replaces these original failed setups.

Two failed Jim driver attempts remain explicit. The first C-style version
introspection refused at unsupported `package vcompare`, before source setup
([raw initial records](results/records.json),
[exact first driver](driver-portability-attempt.py)). The next attempt reached
source setup's alias error but then assumed global `::errorInfo` existed and
failed while serialising it ([raw records](jim-native-results/records.json),
[exact second driver](driver-jim-errorinfo-attempt.py)). Those driver failures
are not semantic answers. The final driver selects native catch capability
without C version commands and reads error globals independently; its exact
source inputs match both earlier attempts.

The corrected [runner](run.py) requires a fresh destination:

```sh
python3 experiments/name-handling/patterns/composed-callback-prefix/run.py --dest /tmp/composed-callback-new
```

Use `--provider jim` for the Jim-only channel. Existing capture directories must
not be overwritten. Inspect [consumer inspection and proposed independent
checks](CONSUMER-PLAN.md) separately from these native results, and the
[unanswered F5 source/context request](F5-REQUEST.md) for appliance boundaries.
