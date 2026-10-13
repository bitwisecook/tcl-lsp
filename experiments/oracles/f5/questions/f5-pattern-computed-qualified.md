# f5-pattern-computed-qualified

## Question

What lookup completion and result does the exact computed-qualified reduction produce after successful setup on each recorded provider?

## Answer

The table reports coverage of each requested original lookup in its recorded context. Partial coverage retains a measured setup/capability boundary; it does not answer an unreached final lookup. Investigation completion is separate.

| Context | Execution | Coverage | Investigation |
| --- | --- | --- | --- |
| tmsh-cli-script | observed | complete | not separately recorded |
| iapp-implementation | observed | complete | not separately recorded |
| tmm-RULE_INIT | unreached | partial | not separately recorded |
| tmm-procedure | observed | complete | not separately recorded |
| tmm-traffic-event | observed | complete | not separately recorded |
| appliance-stock-tclsh | observed | complete | not separately recorded |
| icall | observed | complete | not separately recorded |
| iapp-apl | observed | complete | not separately recorded |

## Measured evidence

### tmsh-cli-script

Load accept (exit 0); setup completion 0; lookup completion 1; result "invalid command name \"::nh::emit\"".

The byte-identical original program was evaluated sequentially. The complete recorded errorInfo names the failing command within the authored final operation and attributes it to that operation's line in eval $nh_operation. Reaching that line establishes that preceding authored top-level commands completed without propagating a nonzero completion. The code1/result belongs to evaluating the final operation, including its substitutions; it does not assert that the outer command was dispatched after an argument-substitution error. Successful setup here means the original prefix completed, not that it created every expected binding. This source-derived reachability review does not replace raw evidence, rewrite the original question, classify semantic unsupported behaviour, or apply to another context/invocation.

### iapp-implementation

Load accept (exit 0); setup completion 0; lookup completion 1; result "invalid command name \"::nh::emit\"".

The byte-identical original program was evaluated sequentially. The complete recorded errorInfo names the failing command within the authored final operation and attributes it to that operation's line in eval $nh_operation. Reaching that line establishes that preceding authored top-level commands completed without propagating a nonzero completion. The code1/result belongs to evaluating the final operation, including its substitutions; it does not assert that the outer command was dispatched after an argument-substitution error. Successful setup here means the original prefix completed, not that it created every expected binding. This source-derived reachability review does not replace raw evidence, rewrite the original question, classify semantic unsupported behaviour, or apply to another context/invocation.

### tmm-RULE_INIT

Load accept (exit 0); setup completion 1; lookup completion None; result "not captured".

Setup failed before the intended operation.

Setup failed before the intended operation.

### tmm-procedure

Load accept (exit 0); setup completion 0; lookup completion 0; result "hex:6669727374".

Complete byte-identical original source evaluated in the retained host wrapper's actual namespace/frame. Ownership guard precedes the entire program; cleanup deletes only original exact names proven absent. Whole-program completion does not by itself prove that a particular nested/final lookup was reached.

### tmm-traffic-event

Load accept (exit 0); setup completion 0; lookup completion 0; result "hex:6669727374".

Complete byte-identical original source evaluated in the retained host wrapper's actual namespace/frame. Ownership guard precedes the entire program; cleanup deletes only original exact names proven absent. Whole-program completion does not by itself prove that a particular nested/final lookup was reached.

### appliance-stock-tclsh

Load not-applicable (exit None); setup completion 0; lookup completion 0; result "first".

Complete original source in fresh stock child interpreter/process; source is exact, wrapper/isolation channel must still be retained.

### icall

Load accept (exit 0); setup completion 0; lookup completion 1; result "invalid command name \"::nh::emit\"".

The byte-identical original program was evaluated sequentially. The complete recorded errorInfo names the failing command within the authored final operation and attributes it to that operation's line in eval $nh_operation. Reaching that line establishes that preceding authored top-level commands completed without propagating a nonzero completion. The code1/result belongs to evaluating the final operation, including its substitutions; it does not assert that the outer command was dispatched after an argument-substitution error. Successful setup here means the original prefix completed, not that it created every expected binding. This source-derived reachability review does not replace raw evidence, rewrite the original question, classify semantic unsupported behaviour, or apply to another context/invocation.

### iapp-apl

Load accept (exit 0); setup completion 0; lookup completion 1; result "invalid command name \"::nh::emit\"".

The byte-identical original program was evaluated sequentially. The complete recorded errorInfo names the failing command within the authored final operation and attributes it to that operation's line in eval $nh_operation. Reaching that line establishes that preceding authored top-level commands completed without propagating a nonzero completion. The code1/result belongs to evaluating the final operation, including its substitutions; it does not assert that the outer command was dispatched after an argument-substitution error. Successful setup here means the original prefix completed, not that it created every expected binding. This source-derived reachability review does not replace raw evidence, rewrite the original question, classify semantic unsupported behaviour, or apply to another context/invocation.

## C Tcl and Jim Tcl controls

The [reviewed cross-provider proof](../../../../docs/design/analysis/name-resolution-proofs/corpus-pattern-computed-qualified.md) retains independent provider scope. C Tcl version results are not substituted for appliance results. Jim's inner-script observations are distinct from C Tcltest PASS/SKIP outcomes.

| Provider | Status | Scoped answer |
| --- | --- | --- |
| tcl8.4 | observed | Setup completed with code 0. Actual lookup completion code 0; result "first"; errorCode "". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. |
| tcl8.5 | observed | Setup completed with code 0. Actual lookup completion code 0; result "first"; errorCode "". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. |
| tcl8.6 | observed | Setup completed with code 0. Actual lookup completion code 0; result "first"; errorCode "". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. |
| tcl9.0 | observed | Setup completed with code 0. Actual lookup completion code 0; result "first"; errorCode "". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. |
| tcl9.1 | observed | Setup completed with code 0. Actual lookup completion code 0; result "first"; errorCode "". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. |
| jim | observed | Setup completed with code 0. Actual lookup completion code 0; result "first". No guest errorCode is reported in the captured return options. Full setup/question return options and errorInfo remain in the exact raw capture. This observes only the reached original question, not a C-equivalence or Rust consumer result. |

## Exact source

```tcl
namespace eval ::nh {proc emit {} {return first}}
set ns ::nh
set head ${ns}::emit
$head
```

## Configuration, scripts and decoding

[f5-pattern-computed-qualified.json](f5-pattern-computed-qualified.json) records the exact source, context rows, selected results, original record selectors and replay recipes. Historical record selectors retain client/server/invocation/query/cleanup references, capture completeness and decoding fields without copying them into every answer.

Read an exact retained artefact or select a JSON measurement:

```sh
python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH
python3 experiments/oracles/f5/materialise.py read --path RECORD_PATH --json-pointer /ROW
```

The stored bytes retain their existing sanitisation and hash meaning. Reconstructed addressing is labelled separately. Null or unavailable fields are not inferred, and an adapted control cannot fill an original-source gap.
