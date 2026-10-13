# f5-pattern-tcltest-shared-lifecycle

## Question

What lookup completion and result does the exact tcltest-shared-lifecycle reduction produce after successful setup on each recorded provider?

## Answer

The table reports coverage of each requested original lookup in its recorded context. Partial coverage retains a measured setup/capability boundary; it does not answer an unreached final lookup. Investigation completion is separate.

| Context | Execution | Coverage | Investigation |
| --- | --- | --- | --- |
| tmsh-cli-script | unavailable | partial | not separately recorded |
| iapp-implementation | unavailable | partial | not separately recorded |
| tmm-RULE_INIT | unavailable | partial | not separately recorded |
| tmm-procedure | unavailable | partial | not separately recorded |
| tmm-traffic-event | unavailable | partial | not separately recorded |
| appliance-stock-tclsh | observed | complete | not separately recorded |
| icall | unavailable | partial | not separately recorded |
| iapp-apl | unavailable | partial | not separately recorded |

## Measured evidence

### tmsh-cli-script

Load accept (exit 0); setup completion 0; lookup completion 1; result "can't read \"debug\": no such variable".

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

### iapp-implementation

Load accept (exit 0); setup completion 0; lookup completion 1; result "can't read \"debug\": no such variable".

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

### tmm-RULE_INIT

Load accept (exit 0); setup completion 0; lookup completion 1; result "hex:63616e27742066696e64207061636b6167652074636c74657374".

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

### tmm-procedure

Load accept (exit 0); setup completion 0; lookup completion 1; result "hex:63616e27742066696e64207061636b6167652074636c74657374".

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

### tmm-traffic-event

Load accept (exit 0); setup completion 0; lookup completion 1; result "hex:63616e27742066696e64207061636b6167652074636c74657374".

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

### appliance-stock-tclsh

Load not-applicable (exit None); setup completion 0; lookup completion 0; result "{{a b} {a b}} 0".

Complete original source in fresh stock child interpreter/process; source is exact, wrapper/isolation channel must still be retained.

### icall

Load accept (exit 0); setup completion 0; lookup completion 1; result "can't read \"debug\": no such variable".

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

### iapp-apl

Load accept (exit 0); setup completion 0; lookup completion 1; result "can't read \"debug\": no such variable".

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

## C Tcl and Jim Tcl controls

The [reviewed cross-provider proof](../../../../docs/design/analysis/name-resolution-proofs/corpus-pattern-tcltest-shared-lifecycle.md) retains independent provider scope. C Tcl version results are not substituted for appliance results. Jim's inner-script observations are distinct from C Tcltest PASS/SKIP outcomes.

| Provider | Status | Scoped answer |
| --- | --- | --- |
| tcl8.4 | observed | Setup completed with code 0. Actual lookup completion code 0; result "{{a b} {a b}} 0"; errorCode "". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. |
| tcl8.5 | observed | Setup completed with code 0. Actual lookup completion code 0; result "{{a b} {a b}} 0"; errorCode "". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. |
| tcl8.6 | observed | Setup completed with code 0. Actual lookup completion code 0; result "{{a b} {a b}} 0"; errorCode "". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. |
| tcl9.0 | observed | Setup completed with code 0. Actual lookup completion code 0; result "{{a b} {a b}} 0"; errorCode "". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. |
| tcl9.1 | observed | Setup completed with code 0. Actual lookup completion code 0; result "{{a b} {a b}} 0"; errorCode "". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. |
| jim | observed | Setup completed with code 0. Actual lookup completion code 1; result "can't read \"::nh::observed\": no such variable". Guest errorCode is NONE. Full setup/question return options and errorInfo remain in the exact raw capture. This observes only the reached original question, not a C-equivalence or Rust consumer result. Jim harness counters separately report one passed test and no failures; those counters do not make the failed final qualified variable read succeed. |

## Exact source

```tcl
package require tcltest
namespace eval ::nh {
    namespace import ::tcltest::*
    set observed {}
    test name-corpus-1 {} -setup {
        set payload {a b}
    } -body {
        lappend observed [set payload]
    } -cleanup {
        lappend observed [set payload]
        unset payload
    } -result {{a b}}
}
list $::nh::observed [info exists ::nh::payload]
```

## Configuration, scripts and decoding

[f5-pattern-tcltest-shared-lifecycle.json](f5-pattern-tcltest-shared-lifecycle.json) records the exact source, context rows, selected results, original record selectors and replay recipes. Historical record selectors retain client/server/invocation/query/cleanup references, capture completeness and decoding fields without copying them into every answer.

Read an exact retained artefact or select a JSON measurement:

```sh
python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH
python3 experiments/oracles/f5/materialise.py read --path RECORD_PATH --json-pointer /ROW
```

The stored bytes retain their existing sanitisation and hash meaning. Reconstructed addressing is labelled separately. Null or unavailable fields are not inferred, and an adapted control cannot fill an original-source gap.
