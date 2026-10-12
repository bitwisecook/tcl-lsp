# Jim pattern observations

These observations execute upstream Jim at `5bac7c99ad65864c87da513e22e2f01703fa4e03` (`0.84-9-g5bac7c9`). The upstream HEAD check is retained in [upstream-head.txt](upstream-head.txt). Binary, configuration and real Jim harness hashes are in [provenance.json](provenance.json). No C Tcl init library is substituted.

Each corpus reduction uses its exact setup and final question in a fresh process. Each Wiki question executes the exact inner script from its `isolated` body in a fresh Jim process; C interpreter isolation and release/TclOO constraints are removed explicitly. The original unmodified Wiki suite is also attempted: it stops at missing `::tcltest::configure` before a test runs. Its [capture](original-wiki-suite/record.json) is a harness refusal, not thirty semantic failures.

The driver sources upstream `tests/testing.tcl`, which sources upstream `tcltest.tcl`. Questions are observations, not assertions of C expected values on Jim. The lifecycle question executes the real Jim `test` procedure; its counters are retained. C/Jim API differences have four separate [Jim questions](../jim-probes/), which do not replace the original questions.

`returned-value` means the question completed normally, including questions that intentionally return a caught error code. `raised-error` retains an actual Tcl error. `capability-rejection` here is limited to the executed missing `oo::class` or incompatible `interp` interface. Setup failures leave the final corpus lookup unreached. `input.tcl`, `stdout.txt`, `stderr.txt` and `record.json` exist for every row.

| Question | Status | Observed value or error | Capture |
| --- | --- | --- | --- |
| `alias-prefix-lifetime` | setup-failed-question-unreached | `wrong # args: should be "interp"` | [record](corpus/alias-prefix-lifetime/record.json) |
| `computed-binding-mutation` | returned-value | `replacement` | [record](corpus/computed-binding-mutation/record.json) |
| `computed-qualified` | returned-value | `first` | [record](corpus/computed-qualified/record.json) |
| `ensemble-prefix-map` | setup-failed-question-unreached | `wrong # args: should be "ensemble command ?-automap prefix?"` | [record](corpus/ensemble-prefix-map/record.json) |
| `factory-body-near-miss` | raised-error | `can't read "name": no such variable` | [record](corpus/factory-body-near-miss/record.json) |
| `finite-factory` | returned-value | `red blue` | [record](corpus/finite-factory/record.json) |
| `frame-wrapper-near-miss` | returned-value | `original` | [record](corpus/frame-wrapper-near-miss/record.json) |
| `frame-wrapper` | returned-value | `changed` | [record](corpus/frame-wrapper/record.json) |
| `keyed-variable-link` | returned-value | `changed` | [record](corpus/keyed-variable-link/record.json) |
| `list-prefix` | returned-value | `{a b} {c d}` | [record](corpus/list-prefix/record.json) |
| `namespace-captured-prefix` | raised-error | `can't read "::nh::prefix": no such variable` | [record](corpus/namespace-captured-prefix/record.json) |
| `oo-prefix-deleted-near-miss` | setup-failed-question-unreached | `invalid command name "oo::class"` | [record](corpus/oo-prefix-deleted-near-miss/record.json) |
| `oo-self-prefix` | setup-failed-question-unreached | `invalid command name "oo::class"` | [record](corpus/oo-self-prefix/record.json) |
| `string-prefix-near-miss` | raised-error | `invalid command name "::nh::two"` | [record](corpus/string-prefix-near-miss/record.json) |
| `tcltest-shared-lifecycle` | raised-error | `can't read "::nh::observed": no such variable` | [record](corpus/tcltest-shared-lifecycle/record.json) |
| `alias-prefix-rebinding` | returned-value | `{::nh::emit {a b}} {replacement {a b} {c d}}` | [record](jim-native/alias-prefix-rebinding/record.json) |
| `ensemble-native-dispatch` | returned-value | `{old {a b} {c d}} {replacement {a b} {c d}}` | [record](jim-native/ensemble-native-dispatch/record.json) |
| `namespace-explicit-variable-prefix` | returned-value | `local {a b} {{c d}}` | [record](jim-native/namespace-explicit-variable-prefix/record.json) |
| `test-shared-lifecycle` | returned-value | `{{a b} {a b}} 0 1 0` | [record](jim-native/test-shared-lifecycle/record.json) |
| `wiki-prefix-1` | returned-value | `{x; error injected} {a b}` | [record](wiki/wiki-prefix-1/record.json) |
| `wiki-prefix-2` | returned-value | `extra script` | [record](wiki/wiki-prefix-2/record.json) |
| `wiki-prefix-3` | returned-value | `actual` | [record](wiki/wiki-prefix-3/record.json) |
| `wiki-code-1` | returned-value | `local fixed {a b}` | [record](wiki/wiki-code-1/record.json) |
| `wiki-code-2` | raised-error | `can't read "x": no such variable` | [record](wiki/wiki-code-2/record.json) |
| `wiki-code-3` | returned-value | `replacement` | [record](wiki/wiki-code-3/record.json) |
| `wiki-code-4` | returned-value | `1` | [record](wiki/wiki-code-4/record.json) |
| `wiki-code-5` | raised-error | `can't read "x": no such variable` | [record](wiki/wiki-code-5/record.json) |
| `wiki-code-6` | raised-error | `invalid command name "::namespace inscope ::n take"` | [record](wiki/wiki-code-6/record.json) |
| `wiki-code-7` | returned-value | `1` | [record](wiki/wiki-code-7/record.json) |
| `wiki-factory-1` | returned-value | `alpha beta` | [record](wiki/wiki-factory-1/record.json) |
| `wiki-factory-2` | returned-value | `1` | [record](wiki/wiki-factory-2/record.json) |
| `wiki-alias-1` | capability-rejection | `wrong # args: should be "interp"` | [record](wiki/wiki-alias-1/record.json) |
| `wiki-alias-2` | capability-rejection | `wrong # args: should be "interp"` | [record](wiki/wiki-alias-2/record.json) |
| `wiki-rename-1` | returned-value | `two` | [record](wiki/wiki-rename-1/record.json) |
| `wiki-upvar-1` | returned-value | `changed` | [record](wiki/wiki-upvar-1/record.json) |
| `wiki-upvar-2` | returned-value | `0` | [record](wiki/wiki-upvar-2/record.json) |
| `wiki-upvar-3` | returned-value | `changed` | [record](wiki/wiki-upvar-3/record.json) |
| `wiki-upvar-4` | returned-value | `old` | [record](wiki/wiki-upvar-4/record.json) |
| `wiki-variable-1` | returned-value | `b 0` | [record](wiki/wiki-variable-1/record.json) |
| `wiki-global-1` | returned-value | `global` | [record](wiki/wiki-global-1/record.json) |
| `wiki-autoload-1` | raised-error | `invalid command name "::late"` | [record](wiki/wiki-autoload-1/record.json) |
| `wiki-unknown-1` | returned-value | `{} {absent fixed}` | [record](wiki/wiki-unknown-1/record.json) |
| `wiki-oo-1` | capability-rejection | `invalid command name "oo::class"` | [record](wiki/wiki-oo-1/record.json) |
| `wiki-oo-2` | capability-rejection | `invalid command name "oo::class"` | [record](wiki/wiki-oo-2/record.json) |
| `wiki-oo-3` | capability-rejection | `invalid command name "oo::class"` | [record](wiki/wiki-oo-3/record.json) |
| `wiki-oo-4` | capability-rejection | `invalid command name "oo::class"` | [record](wiki/wiki-oo-4/record.json) |
| `wiki-oo-5` | capability-rejection | `invalid command name "oo::class"` | [record](wiki/wiki-oo-5/record.json) |
| `wiki-oo-6` | capability-rejection | `invalid command name "oo::class"` | [record](wiki/wiki-oo-6/record.json) |
| `wiki-oo-7` | capability-rejection | `invalid command name "oo::class"` | [record](wiki/wiki-oo-7/record.json) |

The native alias question shows name rebinding with a preserved list prefix. The native ensemble question shows dispatch observing command replacement. The explicit `variable prefix` question distinguishes Jim namespace variable creation from an ordinary `set` inside `namespace eval`. The native lifecycle question returns `{{a b} {a b}} 0 1 0`: setup/body/cleanup see the payload, cleanup removes it, and the real harness records one pass and no failures.

Reproduce with:

```sh
python3 experiments/name-handling/patterns/jim-run-patterns.py --dest experiments/name-handling/patterns/jim-results-new
```

The destination must be new. Retained [runner-original.txt](runner-original.txt) is the exact capture runner; formatting the working runner does not change the existing capture. [Initial experiment](../jim-discovery-5bac7c9/README.md) is retained with its corrected interpretation.
