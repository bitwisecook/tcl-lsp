# f5-pattern-wiki-oo-3

## Question

Does this retained program establish that forward target observes command replacement?

## Answer

The table reports coverage of each requested original lookup in its recorded context. Partial coverage retains a measured setup/capability boundary; it does not answer an unreached final lookup. Investigation completion is separate.

| Context | Execution | Coverage | Investigation |
| --- | --- | --- | --- |
| tmsh-cli-script | unreached | partial | not separately recorded |
| iapp-implementation | unreached | partial | not separately recorded |
| tmm-RULE_INIT | unreached | partial | not separately recorded |
| tmm-procedure | unreached | partial | not separately recorded |
| tmm-traffic-event | unreached | partial | not separately recorded |
| appliance-stock-tclsh | unreached | partial | not separately recorded |
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

Load not-applicable (exit None); setup completion 0; lookup completion None; result "not captured".

Real tcltest skipped selected test: tcloo

Real tcltest skipped selected test: tcloo

### icall

Load accept (exit 0); setup completion 1; lookup completion None; result "not captured".

Setup failed before the intended operation.

Setup failed before the intended operation.

### iapp-apl

Load accept (exit 0); setup completion 1; lookup completion None; result "not captured".

Setup failed before the intended operation.

Setup failed before the intended operation.

## C Tcl and Jim Tcl controls

The [reviewed cross-provider proof](../../../../docs/design/analysis/name-resolution-proofs/wiki-oo-3.md) retains independent provider scope. C Tcl version results are not substituted for appliance results. Jim's inner-script observations are distinct from C Tcltest PASS/SKIP outcomes.

| Provider | Status | Scoped answer |
| --- | --- | --- |
| tcl8.4 | not-tested | ++++ wiki-oo-3 SKIPPED: tcloo. This case did not execute and supplies no oracle answer. |
| tcl8.5 | not-tested | ++++ wiki-oo-3 SKIPPED: tcloo. This case did not execute and supplies no oracle answer. |
| tcl8.6 | observed | wiki-oo-3 PASSED: actual result matched retained -result {replacement fixed}. |
| tcl9.0 | observed | wiki-oo-3 PASSED: actual result matched retained -result {replacement fixed}. |
| tcl9.1 | observed | wiki-oo-3 PASSED: actual result matched retained -result {replacement fixed}. |
| jim | not-tested | This authored Wiki reduction was not executed on this provider. No equivalent result is inferred from C. |

The following is the selected source range, not a standalone replacement for the real package/suite wrapper.

## Exact source

```tcl
::tcltest::test wiki-oo-3 {forward target observes command replacement} -constraints tcloo -body {
    isolated {
        proc take x {list old $x}
        oo::class create C {forward send ::take fixed}
        set obj [C new]
        rename take saved
        proc take x {list replacement $x}
        $obj send
    }
} -result {replacement fixed}
```

## Configuration, scripts and decoding

[f5-pattern-wiki-oo-3.json](f5-pattern-wiki-oo-3.json) records the exact source, context rows, selected results, original record selectors and replay recipes. Historical record selectors retain client/server/invocation/query/cleanup references, capture completeness and decoding fields without copying them into every answer.

Read an exact retained artefact or select a JSON measurement:

```sh
python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH
python3 experiments/oracles/f5/materialise.py read --path RECORD_PATH --json-pointer /ROW
```

The stored bytes retain their existing sanitisation and hash meaning. Reconstructed addressing is labelled separately. Null or unavailable fields are not inferred, and an adapted control cannot fill an original-source gap.
