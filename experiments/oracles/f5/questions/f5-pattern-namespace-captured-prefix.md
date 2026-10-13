# f5-pattern-namespace-captured-prefix

## Question

What lookup completion and result does the exact namespace-captured-prefix reduction produce after successful setup on each recorded provider?

## Answer

The table reports coverage of each requested original lookup in its recorded context. Partial coverage retains a measured setup/capability boundary; it does not answer an unreached final lookup. Investigation completion is separate.

| Context | Execution | Coverage | Investigation |
| --- | --- | --- | --- |
| tmsh-cli-script | observed | complete | not separately recorded |
| iapp-implementation | observed | complete | not separately recorded |
| tmm-RULE_INIT | observed | complete | not separately recorded |
| tmm-procedure | observed | complete | not separately recorded |
| tmm-traffic-event | observed | complete | not separately recorded |
| appliance-stock-tclsh | observed | complete | not separately recorded |
| icall | observed | complete | not separately recorded |
| iapp-apl | observed | complete | not separately recorded |

## Measured evidence

### tmsh-cli-script

Load accept (exit 0); setup completion 0; lookup completion 0; result "global".

Complete byte-identical original source evaluated in the retained host wrapper's actual namespace/frame. Ownership guard precedes the entire program; cleanup deletes only original exact names proven absent. Whole-program completion does not by itself prove that a particular nested/final lookup was reached.

### iapp-implementation

Load accept (exit 0); setup completion 0; lookup completion 0; result "global".

Complete byte-identical original source evaluated in the retained host wrapper's actual namespace/frame. Ownership guard precedes the entire program; cleanup deletes only original exact names proven absent. Whole-program completion does not by itself prove that a particular nested/final lookup was reached.

### tmm-RULE_INIT

Load accept (exit 0); setup completion 0; lookup completion 1; result "hex:63616e2774207265616420223a3a6e683a3a707265666978223a206e6f2073756368207661726961626c65".

The byte-identical original program was evaluated sequentially. Complete recorded binary-scan errorInfo bytes contain reversible ASCII quoted-command and eval-body-line markers locating the authored final operation. Reaching that source line establishes that preceding authored top-level commands completed without propagating a nonzero completion. Code1/result belongs to evaluating the final operation, including substitutions; an argument-substitution failure does not imply outer command dispatch. ASCII is an explicit reversible byte projection for locating stack markers only. The original result/error encodings remain binary-scan-tcl-value; no UTF8 or Tcl Unicode projection is claimed. Successful setup means prior source evaluation completed, not that every expected binding exists. Ownership collisions and earlier program errors remain unpromoted; this review applies only to this exact identity.

### tmm-procedure

Load accept (exit 0); setup completion 0; lookup completion 1; result "hex:63616e2774207265616420223a3a6e683a3a707265666978223a206e6f2073756368207661726961626c65".

The byte-identical original program was evaluated sequentially. Complete recorded binary-scan errorInfo bytes contain reversible ASCII quoted-command and eval-body-line markers locating the authored final operation. Reaching that source line establishes that preceding authored top-level commands completed without propagating a nonzero completion. Code1/result belongs to evaluating the final operation, including substitutions; an argument-substitution failure does not imply outer command dispatch. ASCII is an explicit reversible byte projection for locating stack markers only. The original result/error encodings remain binary-scan-tcl-value; no UTF8 or Tcl Unicode projection is claimed. Successful setup means prior source evaluation completed, not that every expected binding exists. Ownership collisions and earlier program errors remain unpromoted; this review applies only to this exact identity.

### tmm-traffic-event

Load accept (exit 0); setup completion 0; lookup completion 1; result "hex:63616e2774207265616420223a3a6e683a3a707265666978223a206e6f2073756368207661726961626c65".

The byte-identical original program was evaluated sequentially. Complete recorded binary-scan errorInfo bytes contain reversible ASCII quoted-command and eval-body-line markers locating the authored final operation. Reaching that source line establishes that preceding authored top-level commands completed without propagating a nonzero completion. Code1/result belongs to evaluating the final operation, including substitutions; an argument-substitution failure does not imply outer command dispatch. ASCII is an explicit reversible byte projection for locating stack markers only. The original result/error encodings remain binary-scan-tcl-value; no UTF8 or Tcl Unicode projection is claimed. Successful setup means prior source evaluation completed, not that every expected binding exists. Ownership collisions and earlier program errors remain unpromoted; this review applies only to this exact identity.

### appliance-stock-tclsh

Load not-applicable (exit None); setup completion 0; lookup completion 0; result "local {a b} {{c d}}".

Complete original source in fresh stock child interpreter/process; source is exact, wrapper/isolation channel must still be retained.

### icall

Load accept (exit 0); setup completion 0; lookup completion 0; result "global".

Complete byte-identical original source evaluated in the retained host wrapper's actual namespace/frame. Ownership guard precedes the entire program; cleanup deletes only original exact names proven absent. Whole-program completion does not by itself prove that a particular nested/final lookup was reached.

### iapp-apl

Load accept (exit 0); setup completion 0; lookup completion 0; result "global".

Complete byte-identical original source evaluated in the retained host wrapper's actual namespace/frame. Ownership guard precedes the entire program; cleanup deletes only original exact names proven absent. Whole-program completion does not by itself prove that a particular nested/final lookup was reached.

## C Tcl and Jim Tcl controls

The [reviewed cross-provider proof](../../../../docs/design/analysis/name-resolution-proofs/corpus-pattern-namespace-captured-prefix.md) retains independent provider scope. C Tcl version results are not substituted for appliance results. Jim's inner-script observations are distinct from C Tcltest PASS/SKIP outcomes.

| Provider | Status | Scoped answer |
| --- | --- | --- |
| tcl8.4 | observed | Setup completed with code 0. Actual lookup completion code 0; result "local {a b} {{c d}}"; errorCode "". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. |
| tcl8.5 | observed | Setup completed with code 0. Actual lookup completion code 0; result "local {a b} {{c d}}"; errorCode "". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. |
| tcl8.6 | observed | Setup completed with code 0. Actual lookup completion code 0; result "local {a b} {{c d}}"; errorCode "". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. |
| tcl9.0 | observed | Setup completed with code 0. Actual lookup completion code 0; result "local {a b} {{c d}}"; errorCode "". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. |
| tcl9.1 | observed | Setup completed with code 0. Actual lookup completion code 0; result "local {a b} {{c d}}"; errorCode "". The complete recorded return options/errorInfo remain in the raw capture. This is only the reached lookup result. |
| jim | observed | Setup completed with code 0. Actual lookup completion code 1; result "can't read \"::nh::prefix\": no such variable". Guest errorCode is NONE. Full setup/question return options and errorInfo remain in the exact raw capture. This observes only the reached original question, not a C-equivalence or Rust consumer result. |

## Exact source

```tcl
namespace eval ::nh {
    proc emit {value args} {list local $value $args}
    set prefix [namespace code [list emit {a b}]]
}
proc emit {args} {return global}
eval [linsert $::nh::prefix end {c d}]
```

## Configuration, scripts and decoding

[f5-pattern-namespace-captured-prefix.json](f5-pattern-namespace-captured-prefix.json) records the exact source, context rows, selected results, original record selectors and replay recipes. Historical record selectors retain client/server/invocation/query/cleanup references, capture completeness and decoding fields without copying them into every answer.

Read an exact retained artefact or select a JSON measurement:

```sh
python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH
python3 experiments/oracles/f5/materialise.py read --path RECORD_PATH --json-pointer /ROW
```

The stored bytes retain their existing sanitisation and hash meaning. Reconstructed addressing is labelled separately. Null or unavailable fields are not inferred, and an adapted control cannot fill an original-source gap.
