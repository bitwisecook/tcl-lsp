# f5-pattern-string-prefix-near-miss

## Question

What lookup completion and result does the exact string-prefix-near-miss reduction produce after successful setup on each recorded provider?

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

Load accept (exit 0); setup completion 0; lookup completion 1; result "invalid command name \"::nh::two\"".

The byte-identical original program was evaluated sequentially. The complete recorded errorInfo names the failing command within the authored final operation and attributes it to that operation's line in eval $nh_operation. Reaching that line establishes that preceding authored top-level commands completed without propagating a nonzero completion. The code1/result belongs to evaluating the final operation, including its substitutions; it does not assert that the outer command was dispatched after an argument-substitution error. Successful setup here means the original prefix completed, not that it created every expected binding. This source-derived reachability review does not replace raw evidence, rewrite the original question, classify semantic unsupported behaviour, or apply to another context/invocation.

### iapp-implementation

Load accept (exit 0); setup completion 0; lookup completion 1; result "invalid command name \"::nh::two\"".

The byte-identical original program was evaluated sequentially. The complete recorded errorInfo names the failing command within the authored final operation and attributes it to that operation's line in eval $nh_operation. Reaching that line establishes that preceding authored top-level commands completed without propagating a nonzero completion. The code1/result belongs to evaluating the final operation, including its substitutions; it does not assert that the outer command was dispatched after an argument-substitution error. Successful setup here means the original prefix completed, not that it created every expected binding. This source-derived reachability review does not replace raw evidence, rewrite the original question, classify semantic unsupported behaviour, or apply to another context/invocation.

### tmm-RULE_INIT

Load accept (exit 0); setup completion 1; lookup completion None; result "not captured".

Setup failed before the intended operation.

Setup failed before the intended operation.

### tmm-procedure

Load accept (exit 0); setup completion 0; lookup completion 1; result "hex:696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f22".

The byte-identical original program was evaluated sequentially. Complete recorded binary-scan errorInfo bytes contain reversible ASCII quoted-command and eval-body-line markers locating the authored final operation. Reaching that source line establishes that preceding authored top-level commands completed without propagating a nonzero completion. Code1/result belongs to evaluating the final operation, including substitutions; an argument-substitution failure does not imply outer command dispatch. ASCII is an explicit reversible byte projection for locating stack markers only. The original result/error encodings remain binary-scan-tcl-value; no UTF8 or Tcl Unicode projection is claimed. Successful setup means prior source evaluation completed, not that every expected binding exists. Ownership collisions and earlier program errors remain unpromoted; this review applies only to this exact identity.

### tmm-traffic-event

Load accept (exit 0); setup completion 0; lookup completion 1; result "hex:696e76616c696420636f6d6d616e64206e616d6520223a3a6e683a3a74776f22".

The byte-identical original program was evaluated sequentially. Complete recorded binary-scan errorInfo bytes contain reversible ASCII quoted-command and eval-body-line markers locating the authored final operation. Reaching that source line establishes that preceding authored top-level commands completed without propagating a nonzero completion. Code1/result belongs to evaluating the final operation, including substitutions; an argument-substitution failure does not imply outer command dispatch. ASCII is an explicit reversible byte projection for locating stack markers only. The original result/error encodings remain binary-scan-tcl-value; no UTF8 or Tcl Unicode projection is claimed. Successful setup means prior source evaluation completed, not that every expected binding exists. Ownership collisions and earlier program errors remain unpromoted; this review applies only to this exact identity.

### appliance-stock-tclsh

Load not-applicable (exit None); setup completion 0; lookup completion 1; result "invalid command name \"::nh::two\"".

The unchanged original program was evaluated sequentially in the recorded fresh stock Tcl process. Its complete errorInfo locates the authored final command at outer eval body line 4, after the original definition prefix completed. The empty wrapper setup is only a successful no-op, not an ownership guard or proof of prefix semantics. Fresh-process invocation supplies isolation; source and stack establish prefix completion. This answer is the exact original final-operation completion/result in this stock context; no new execution, other interpreter context or unrecorded error options are inferred.

### icall

Load accept (exit 0); setup completion 0; lookup completion 1; result "invalid command name \"::nh::two\"".

The byte-identical original program was evaluated sequentially. The complete recorded errorInfo names the failing command within the authored final operation and attributes it to that operation's line in eval $nh_operation. Reaching that line establishes that preceding authored top-level commands completed without propagating a nonzero completion. The code1/result belongs to evaluating the final operation, including its substitutions; it does not assert that the outer command was dispatched after an argument-substitution error. Successful setup here means the original prefix completed, not that it created every expected binding. This source-derived reachability review does not replace raw evidence, rewrite the original question, classify semantic unsupported behaviour, or apply to another context/invocation.

### iapp-apl

Load accept (exit 0); setup completion 0; lookup completion 1; result "invalid command name \"::nh::two\"".

The byte-identical original program was evaluated sequentially. The complete recorded errorInfo names the failing command within the authored final operation and attributes it to that operation's line in eval $nh_operation. Reaching that line establishes that preceding authored top-level commands completed without propagating a nonzero completion. The code1/result belongs to evaluating the final operation, including its substitutions; it does not assert that the outer command was dispatched after an argument-substitution error. Successful setup here means the original prefix completed, not that it created every expected binding. This source-derived reachability review does not replace raw evidence, rewrite the original question, classify semantic unsupported behaviour, or apply to another context/invocation.

## C Tcl and Jim Tcl controls

The [reviewed cross-provider proof](../../../../docs/design/analysis/name-resolution-proofs/corpus-pattern-string-prefix-near-miss.md) retains independent provider scope. C Tcl version results are not substituted for appliance results. Jim's inner-script observations are distinct from C Tcltest PASS/SKIP outcomes.

| Provider | Status | Scoped answer |
| --- | --- | --- |
| tcl8.4 | observed | Setup completed with code 0. Actual lookup completion code 1; result "invalid command name \"::nh::two\""; errorCode "NONE". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. |
| tcl8.5 | observed | Setup completed with code 0. Actual lookup completion code 1; result "invalid command name \"::nh::two\""; errorCode "NONE". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. |
| tcl8.6 | observed | Setup completed with code 0. Actual lookup completion code 1; result "invalid command name \"::nh::two\""; errorCode "TCL LOOKUP COMMAND ::nh::two". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. |
| tcl9.0 | observed | Setup completed with code 0. Actual lookup completion code 1; result "invalid command name \"::nh::two\""; errorCode "TCL LOOKUP COMMAND ::nh::two". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. |
| tcl9.1 | observed | Setup completed with code 0. Actual lookup completion code 1; result "invalid command name \"::nh::two\""; errorCode "TCL LOOKUP COMMAND ::nh::two". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. |
| jim | observed | Setup completed with code 0. Actual lookup completion code 1; result "invalid command name \"::nh::two\"". Guest errorCode is NONE. Full setup/question return options and errorInfo remain in the exact raw capture. This observes only the reached original question, not a C-equivalence or Rust consumer result. |

## Exact source

```tcl
namespace eval ::nh {}
proc {::nh::two words} {fixed value} {list $fixed $value}
set target {::nh::two words}
eval "$target {a b} {c d}"
```

## Configuration, scripts and decoding

[f5-pattern-string-prefix-near-miss.json](f5-pattern-string-prefix-near-miss.json) records the exact source, context rows, selected results, original record selectors and replay recipes. Historical record selectors retain client/server/invocation/query/cleanup references, capture completeness and decoding fields without copying them into every answer.

Read an exact retained artefact or select a JSON measurement:

```sh
python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH
python3 experiments/oracles/f5/materialise.py read --path RECORD_PATH --json-pointer /ROW
```

The stored bytes retain their existing sanitisation and hash meaning. Reconstructed addressing is labelled separately. Null or unavailable fields are not inferred, and an adapted control cannot fill an original-source gap.
