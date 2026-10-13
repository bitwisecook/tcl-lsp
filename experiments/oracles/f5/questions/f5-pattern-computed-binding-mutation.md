# f5-pattern-computed-binding-mutation

## Question

What lookup completion and result does the exact computed-binding-mutation reduction produce after successful setup on each recorded provider?

## Answer

The table reports coverage of each requested original lookup in its recorded context. Partial coverage retains a measured setup/capability boundary; it does not answer an unreached final lookup. Investigation completion is separate.

| Context | Execution | Coverage | Investigation |
| --- | --- | --- | --- |
| tmsh-cli-script | unavailable | partial | not separately recorded |
| iapp-implementation | unavailable | partial | not separately recorded |
| tmm-RULE_INIT | unreached | partial | not separately recorded |
| tmm-procedure | observed | complete | not separately recorded |
| tmm-traffic-event | observed | complete | not separately recorded |
| appliance-stock-tclsh | observed | complete | not separately recorded |
| icall | unavailable | partial | not separately recorded |
| iapp-apl | unavailable | partial | not separately recorded |

## Measured evidence

### tmsh-cli-script

Load accept (exit 0); setup completion 0; lookup completion 1; result "can't rename \"::nh::emit\": command doesn't exist".

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

### iapp-implementation

Load accept (exit 0); setup completion 0; lookup completion 1; result "can't rename \"::nh::emit\": command doesn't exist".

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

### tmm-RULE_INIT

Load accept (exit 0); setup completion 1; lookup completion None; result "not captured".

Setup failed before the intended operation.

Setup failed before the intended operation.

### tmm-procedure

Load accept (exit 0); setup completion 0; lookup completion 0; result "hex:7265706c6163656d656e74".

Complete byte-identical original source evaluated in the retained host wrapper's actual namespace/frame. Ownership guard precedes the entire program; cleanup deletes only original exact names proven absent. Whole-program completion does not by itself prove that a particular nested/final lookup was reached.

### tmm-traffic-event

Load accept (exit 0); setup completion 0; lookup completion 0; result "hex:7265706c6163656d656e74".

Complete byte-identical original source evaluated in the retained host wrapper's actual namespace/frame. Ownership guard precedes the entire program; cleanup deletes only original exact names proven absent. Whole-program completion does not by itself prove that a particular nested/final lookup was reached.

### appliance-stock-tclsh

Load not-applicable (exit None); setup completion 0; lookup completion 0; result "replacement".

Complete original source in fresh stock child interpreter/process; source is exact, wrapper/isolation channel must still be retained.

### icall

Load accept (exit 0); setup completion 0; lookup completion 1; result "can't rename \"::nh::emit\": command doesn't exist".

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

### iapp-apl

Load accept (exit 0); setup completion 0; lookup completion 1; result "can't rename \"::nh::emit\": command doesn't exist".

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

## C Tcl and Jim Tcl controls

The [reviewed cross-provider proof](../../../../docs/design/analysis/name-resolution-proofs/corpus-pattern-computed-binding-mutation.md) retains independent provider scope. C Tcl version results are not substituted for appliance results. Jim's inner-script observations are distinct from C Tcltest PASS/SKIP outcomes.

| Provider | Status | Scoped answer |
| --- | --- | --- |
| tcl8.4 | observed | Setup completed with code 0. Actual lookup completion code 0; result "replacement"; errorCode "". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. |
| tcl8.5 | observed | Setup completed with code 0. Actual lookup completion code 0; result "replacement"; errorCode "". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. |
| tcl8.6 | observed | Setup completed with code 0. Actual lookup completion code 0; result "replacement"; errorCode "". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. |
| tcl9.0 | observed | Setup completed with code 0. Actual lookup completion code 0; result "replacement"; errorCode "". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. |
| tcl9.1 | observed | Setup completed with code 0. Actual lookup completion code 0; result "replacement"; errorCode "". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. |
| jim | observed | Setup completed with code 0. Actual lookup completion code 0; result "replacement". No guest errorCode is reported in the captured return options. Full setup/question return options and errorInfo remain in the exact raw capture. This observes only the reached original question, not a C-equivalence or Rust consumer result. |

## Exact source

```tcl
namespace eval ::nh {proc emit {} {return first}}
set ns ::nh
set head ${ns}::emit
rename ::nh::emit ::nh::old
proc ::nh::emit {} {return replacement}
$head
```

## Configuration, scripts and decoding

[f5-pattern-computed-binding-mutation.json](f5-pattern-computed-binding-mutation.json) records the exact source, context rows, selected results, original record selectors and replay recipes. Historical record selectors retain client/server/invocation/query/cleanup references, capture completeness and decoding fields without copying them into every answer.

Read an exact retained artefact or select a JSON measurement:

```sh
python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH
python3 experiments/oracles/f5/materialise.py read --path RECORD_PATH --json-pointer /ROW
```

The stored bytes retain their existing sanitisation and hash meaning. Reconstructed addressing is labelled separately. Null or unavailable fields are not inferred, and an adapted control cannot fill an original-source gap.
