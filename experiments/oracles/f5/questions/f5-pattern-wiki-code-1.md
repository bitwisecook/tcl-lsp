# f5-pattern-wiki-code-1

## Question

Does this retained program establish that namespace code captures command lookup namespace?

## Answer

The table reports coverage of each requested original lookup in its recorded context. Partial coverage retains a measured setup/capability boundary; it does not answer an unreached final lookup. Investigation completion is separate.

| Context | Execution | Coverage | Investigation |
| --- | --- | --- | --- |
| tmsh-cli-script | unreached | partial | not separately recorded |
| iapp-implementation | unreached | partial | not separately recorded |
| tmm-RULE_INIT | unreached | partial | not separately recorded |
| tmm-procedure | unreached | partial | not separately recorded |
| tmm-traffic-event | unreached | partial | not separately recorded |
| appliance-stock-tclsh | observed | complete | not separately recorded |
| icall | unreached | partial | not separately recorded |
| iapp-apl | unreached | partial | not separately recorded |

## Measured evidence

### tmsh-cli-script

Load accept (exit 0); setup completion 1; lookup completion None; result "not captured".

Setup failed before the intended operation.

Setup failed before the intended operation.

### iapp-implementation

Load accept (exit 0); setup completion 1; lookup completion None; result "not captured".

Setup failed before the intended operation.

Setup failed before the intended operation.

### tmm-RULE_INIT

Load accept (exit 0); setup completion 1; lookup completion None; result "not captured".

Setup failed before the intended operation.

Setup failed before the intended operation.

### tmm-procedure

Load accept (exit 0); setup completion 1; lookup completion None; result "not captured".

Setup failed before the intended operation.

Setup failed before the intended operation.

### tmm-traffic-event

Load accept (exit 0); setup completion 1; lookup completion None; result "not captured".

Setup failed before the intended operation.

Setup failed before the intended operation.

### appliance-stock-tclsh

Load not-applicable (exit None); setup completion 0; lookup completion 0; result "local fixed {a b}".

Observer captures isolated leave completion/result; child inner error options are unavailable.

### icall

Load accept (exit 0); setup completion 1; lookup completion None; result "not captured".

Setup failed before the intended operation.

Setup failed before the intended operation.

### iapp-apl

Load accept (exit 0); setup completion 1; lookup completion None; result "not captured".

Setup failed before the intended operation.

Setup failed before the intended operation.

## C Tcl and Jim Tcl controls

The [reviewed cross-provider proof](../../../../docs/design/analysis/name-resolution-proofs/wiki-code-1.md) retains independent provider scope. C Tcl version results are not substituted for appliance results. Jim's inner-script observations are distinct from C Tcltest PASS/SKIP outcomes.

| Provider | Status | Scoped answer |
| --- | --- | --- |
| tcl8.4 | observed | wiki-code-1 PASSED: actual result matched retained -result {local fixed {a b}}. |
| tcl8.5 | observed | wiki-code-1 PASSED: actual result matched retained -result {local fixed {a b}}. |
| tcl8.6 | observed | wiki-code-1 PASSED: actual result matched retained -result {local fixed {a b}}. |
| tcl9.0 | observed | wiki-code-1 PASSED: actual result matched retained -result {local fixed {a b}}. |
| tcl9.1 | observed | wiki-code-1 PASSED: actual result matched retained -result {local fixed {a b}}. |
| jim | not-tested | This authored Wiki reduction was not executed on this provider. No equivalent result is inferred from C. |

The following is the selected source range, not a standalone replacement for the real package/suite wrapper.

## Exact source

```tcl
::tcltest::test wiki-code-1 {namespace code captures command lookup namespace} -body {
    isolated {
        proc take args {return global}
        namespace eval n {
            proc take args {return [linsert $args 0 local]}
            set ::prefix [namespace code [list take fixed]]
        }
        eval [linsert $prefix end {a b}]
    }
} -result {local fixed {a b}}
```

## Configuration, scripts and decoding

[f5-pattern-wiki-code-1.json](f5-pattern-wiki-code-1.json) records the exact source, context rows, selected results, original record selectors and replay recipes. Historical record selectors retain client/server/invocation/query/cleanup references, capture completeness and decoding fields without copying them into every answer.

Read an exact retained artefact or select a JSON measurement:

```sh
python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH
python3 experiments/oracles/f5/materialise.py read --path RECORD_PATH --json-pointer /ROW
```

The stored bytes retain their existing sanitisation and hash meaning. Reconstructed addressing is labelled separately. Null or unavailable fields are not inferred, and an adapted control cannot fill an original-source gap.
