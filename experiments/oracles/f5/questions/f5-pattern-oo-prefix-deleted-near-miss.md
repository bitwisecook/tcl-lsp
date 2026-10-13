# f5-pattern-oo-prefix-deleted-near-miss

## Question

What lookup completion and result does the exact oo-prefix-deleted-near-miss reduction produce after successful setup on each recorded provider?

## Answer

The table reports coverage of each requested original lookup in its recorded context. Partial coverage retains a measured setup/capability boundary; it does not answer an unreached final lookup. Investigation completion is separate.

| Context | Execution | Coverage | Investigation |
| --- | --- | --- | --- |
| tmsh-cli-script | unreached | partial | not separately recorded |
| iapp-implementation | unreached | partial | not separately recorded |
| tmm-RULE_INIT | unreached | partial | not separately recorded |
| tmm-procedure | unavailable | partial | not separately recorded |
| tmm-traffic-event | unavailable | partial | not separately recorded |
| appliance-stock-tclsh | unreached | partial | not separately recorded |
| icall | unreached | partial | not separately recorded |
| iapp-apl | unreached | partial | not separately recorded |

## Measured evidence

### tmsh-cli-script

Load accept (exit 0); setup completion 0; lookup completion 1; result "invalid command name \"oo::class\"".

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

### iapp-implementation

Load accept (exit 0); setup completion 0; lookup completion 1; result "invalid command name \"oo::class\"".

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

### tmm-RULE_INIT

Load accept (exit 0); setup completion 1; lookup completion None; result "not captured".

Setup failed before the intended operation.

Setup failed before the intended operation.

### tmm-procedure

Load accept (exit 0); setup completion 0; lookup completion 1; result "hex:696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322".

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

### tmm-traffic-event

Load accept (exit 0); setup completion 0; lookup completion 1; result "hex:696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322".

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

### appliance-stock-tclsh

Load not-applicable (exit None); setup completion 0; lookup completion 1; result "invalid command name \"oo::class\"".

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

Whole original program measured. Intended final-operation reachability is false due to an earlier unsupported TclOO setup operation.

### icall

Load accept (exit 0); setup completion 0; lookup completion 1; result "invalid command name \"oo::class\"".

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

### iapp-apl

Load accept (exit 0); setup completion 0; lookup completion 1; result "invalid command name \"oo::class\"".

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

Observed code 1 belongs to evaluation of the entire portable program. It is not an accepted answer to its canonical final-lookup question.

## C Tcl and Jim Tcl controls

The [reviewed cross-provider proof](../../../../docs/design/analysis/name-resolution-proofs/corpus-pattern-oo-prefix-deleted-near-miss.md) retains independent provider scope. C Tcl version results are not substituted for appliance results. Jim's inner-script observations are distinct from C Tcltest PASS/SKIP outcomes.

| Provider | Status | Scoped answer |
| --- | --- | --- |
| tcl8.4 | not-tested | Intended lookup was not reached: setup completion code 1; setup result "invalid command name \"oo::class\""; lookup_code not-run. No lookup, teardown or implementation-equivalence answer is supplied. This stock setup failure does not establish all extension/package contexts are unavailable. |
| tcl8.5 | not-tested | Intended lookup was not reached: setup completion code 1; setup result "invalid command name \"oo::class\""; lookup_code not-run. No lookup, teardown or implementation-equivalence answer is supplied. This stock setup failure does not establish all extension/package contexts are unavailable. |
| tcl8.6 | observed | Setup completed with code 0. Actual lookup completion code 0; result "1"; errorCode "". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. |
| tcl9.0 | observed | Setup completed with code 0. Actual lookup completion code 0; result "1"; errorCode "". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. |
| tcl9.1 | observed | Setup completed with code 0. Actual lookup completion code 0; result "1"; errorCode "". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. |
| jim | not-tested | This exact authored corpus reduction was not executed on this provider. C results establish no Jim or appliance answer. |

## Exact source

```tcl
oo::class create ::nhclass {
    method emit {value} {list ok $value}
    method prefix {} {list [self] emit {a b}}
}
set object [::nhclass new]
set prefix [$object prefix]
$object destroy
catch {eval $prefix} result
string match {invalid command name *} $result
```

## Configuration, scripts and decoding

[f5-pattern-oo-prefix-deleted-near-miss.json](f5-pattern-oo-prefix-deleted-near-miss.json) records the exact source, context rows, selected results, original record selectors and replay recipes. Historical record selectors retain client/server/invocation/query/cleanup references, capture completeness and decoding fields without copying them into every answer.

Read an exact retained artefact or select a JSON measurement:

```sh
python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH
python3 experiments/oracles/f5/materialise.py read --path RECORD_PATH --json-pointer /ROW
```

The stored bytes retain their existing sanitisation and hash meaning. Reconstructed addressing is labelled separately. Null or unavailable fields are not inferred, and an adapted control cannot fill an original-source gap.
