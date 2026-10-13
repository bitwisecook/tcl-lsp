# f5-pattern-ensemble-prefix-map

## Question

What lookup completion and result does the exact ensemble-prefix-map reduction produce after successful setup on each recorded provider?

## Answer

The table reports coverage of each requested original lookup in its recorded context. Partial coverage retains a measured setup/capability boundary; it does not answer an unreached final lookup. Investigation completion is separate.

| Context | Execution | Coverage | Investigation |
| --- | --- | --- | --- |
| tmsh-cli-script | unreached | partial | not separately recorded |
| iapp-implementation | unreached | partial | not separately recorded |
| tmm-RULE_INIT | unavailable | partial | not separately recorded |
| tmm-procedure | unavailable | partial | not separately recorded |
| tmm-traffic-event | unavailable | partial | not separately recorded |
| appliance-stock-tclsh | observed | complete | not separately recorded |
| icall | unreached | partial | not separately recorded |
| iapp-apl | unreached | partial | not separately recorded |

## Measured evidence

### tmsh-cli-script

Load accept (exit 0); setup completion 0; lookup completion 1; result "bad option \"ensemble\": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which".

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

### iapp-implementation

Load accept (exit 0); setup completion 0; lookup completion 1; result "bad option \"ensemble\": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which".

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

### tmm-RULE_INIT

Load accept (exit 0); setup completion 0; lookup completion 1; result "hex:626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368".

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

### tmm-procedure

Load accept (exit 0); setup completion 0; lookup completion 1; result "hex:626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368".

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

### tmm-traffic-event

Load accept (exit 0); setup completion 0; lookup completion 1; result "hex:626164206f7074696f6e2022656e73656d626c65223a206d757374206265206368696c6472656e2c20636f64652c2063757272656e742c2064656c6574652c206576616c2c206578697374732c206578706f72742c20666f726765742c20696d706f72742c20696e73636f70652c206f726967696e2c20706172656e742c207175616c6966696572732c207461696c2c206f72207768696368".

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

### appliance-stock-tclsh

Load not-applicable (exit None); setup completion 0; lookup completion 0; result "{a b} {c d}".

Complete original source in fresh stock child interpreter/process; source is exact, wrapper/isolation channel must still be retained.

### icall

Load accept (exit 0); setup completion 0; lookup completion 1; result "bad option \"ensemble\": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which".

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

### iapp-apl

Load accept (exit 0); setup completion 0; lookup completion 1; result "bad option \"ensemble\": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which".

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

## C Tcl and Jim Tcl controls

The [reviewed cross-provider proof](../../../../docs/design/analysis/name-resolution-proofs/corpus-pattern-ensemble-prefix-map.md) retains independent provider scope. C Tcl version results are not substituted for appliance results. Jim's inner-script observations are distinct from C Tcltest PASS/SKIP outcomes.

| Provider | Status | Scoped answer |
| --- | --- | --- |
| tcl8.4 | not-tested | Intended lookup was not reached: setup completion code 1; setup result "bad option \"ensemble\": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which"; lookup_code not-run. No lookup, teardown or implementation-equivalence answer is supplied. This stock setup failure does not establish all extension/package contexts are unavailable. |
| tcl8.5 | observed | Setup completed with code 0. Actual lookup completion code 0; result "{a b} {c d}"; errorCode "". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. |
| tcl8.6 | observed | Setup completed with code 0. Actual lookup completion code 0; result "{a b} {c d}"; errorCode "". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. |
| tcl9.0 | observed | Setup completed with code 0. Actual lookup completion code 0; result "{a b} {c d}"; errorCode "". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. |
| tcl9.1 | observed | Setup completed with code 0. Actual lookup completion code 0; result "{a b} {c d}"; errorCode "". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. |
| jim | not-tested | This exact authored corpus reduction was not executed on this provider. C results establish no Jim or appliance answer. |

## Exact source

```tcl
namespace eval ::nh {
    proc emit {fixed value} {list $fixed $value}
    namespace ensemble create -command facade -map [list send [list ::nh::emit {a b}]]
}
::nh::facade send {c d}
```

## Configuration, scripts and decoding

[f5-pattern-ensemble-prefix-map.json](f5-pattern-ensemble-prefix-map.json) records the exact source, context rows, selected results, original record selectors and replay recipes. Historical record selectors retain client/server/invocation/query/cleanup references, capture completeness and decoding fields without copying them into every answer.

Read an exact retained artefact or select a JSON measurement:

```sh
python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH
python3 experiments/oracles/f5/materialise.py read --path RECORD_PATH --json-pointer /ROW
```

The stored bytes retain their existing sanitisation and hash meaning. Reconstructed addressing is labelled separately. Null or unavailable fields are not inferred, and an adapted control cannot fill an original-source gap.
