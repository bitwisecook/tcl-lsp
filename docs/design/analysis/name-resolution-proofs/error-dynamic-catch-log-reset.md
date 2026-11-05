# naming.error.dynamic-catch-log-reset

Kind: `native-observation`

## Problem statement

Two errors caught through a dynamic catch head can preserve the first result while resetting the active error log for the second. Reusing the first errorInfo would produce an observable stale completion.

## Question

What original result and errorInfo bytes remain after the two-error dynamic-catch controls?

## Conclusion

The C controls capture distinct FIRST/SECOND results and errorInfo for the second error. The Jim input deliberately omits the errorInfo query and establishes only FIRST/SECOND values; it supplies no errorInfo answer. No private return-options or object-cache ownership is measured.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates. The original C and Jim source hex differ at the final errorInfo read; their narrower outcomes are retained independently. The complete original catch/binary-scan input bytes are independently recovered in the attached wire-input artifact by exact agreement with each original source digest; this is an offline byte check and adds no native observation.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`provenance.json` (`file-0-row-1`):

```json
{
  "engine": "8.4",
  "exit": 0,
  "stdout": "0\t4649525354205345434f4e44207b5345434f4e440a202020207768696c6520657865637574696e670a226572726f72205345434f4e44227d"
}
```

`windows.tsv` (`file-7`):

```json
{
  "provider_rows": [
    "8.4\t73657420626f6479207b6572726f722046495253547d3b63617463682024626f64792066697273743b73657420626f6479207b6572726f72205345434f4e447d3b63617463682024626f6479207365636f6e643b6c6973742024666972737420247365636f6e6420243a3a6572726f72496e666f\t0\t4649525354205345434f4e44207b5345434f4e440a202020207768696c6520657865637574696e670a226572726f72205345434f4e44227d"
  ]
}
```

### tcl8.5

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`provenance.json` (`file-0-row-2`):

```json
{
  "engine": "8.5",
  "exit": 0,
  "stdout": "0\t4649525354205345434f4e44207b5345434f4e440a202020207768696c6520657865637574696e670a226572726f72205345434f4e44227d"
}
```

`windows.tsv` (`file-7`):

```json
{
  "provider_rows": [
    "8.5\t73657420626f6479207b6572726f722046495253547d3b63617463682024626f64792066697273743b73657420626f6479207b6572726f72205345434f4e447d3b63617463682024626f6479207365636f6e643b6c6973742024666972737420247365636f6e6420243a3a6572726f72496e666f\t0\t4649525354205345434f4e44207b5345434f4e440a202020207768696c6520657865637574696e670a226572726f72205345434f4e44227d"
  ]
}
```

### tcl8.6

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`provenance.json` (`file-0-row-3`):

```json
{
  "engine": "8.6",
  "exit": 0,
  "stdout": "0\t4649525354205345434f4e44207b5345434f4e440a202020207768696c6520657865637574696e670a226572726f72205345434f4e44227d"
}
```

`windows.tsv` (`file-7`):

```json
{
  "provider_rows": [
    "8.6\t73657420626f6479207b6572726f722046495253547d3b63617463682024626f64792066697273743b73657420626f6479207b6572726f72205345434f4e447d3b63617463682024626f6479207365636f6e643b6c6973742024666972737420247365636f6e6420243a3a6572726f72496e666f\t0\t4649525354205345434f4e44207b5345434f4e440a202020207768696c6520657865637574696e670a226572726f72205345434f4e44227d"
  ]
}
```

### tcl9.0

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`provenance.json` (`file-0-row-4`):

```json
{
  "engine": "9.0",
  "exit": 0,
  "stdout": "0\t4649525354205345434f4e44207b5345434f4e440a202020207768696c6520657865637574696e670a226572726f72205345434f4e44227d"
}
```

`windows.tsv` (`file-7`):

```json
{
  "provider_rows": [
    "9.0\t73657420626f6479207b6572726f722046495253547d3b63617463682024626f64792066697273743b73657420626f6479207b6572726f72205345434f4e447d3b63617463682024626f6479207365636f6e643b6c6973742024666972737420247365636f6e6420243a3a6572726f72496e666f\t0\t4649525354205345434f4e44207b5345434f4e440a202020207768696c6520657865637574696e670a226572726f72205345434f4e44227d"
  ]
}
```

### tcl9.1

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`provenance.json` (`file-0-row-5`):

```json
{
  "engine": "9.1",
  "exit": 0,
  "stdout": "0\t4649525354205345434f4e44207b5345434f4e440a202020207768696c6520657865637574696e670a226572726f72205345434f4e44227d"
}
```

`windows.tsv` (`file-7`):

```json
{
  "provider_rows": [
    "9.1\t73657420626f6479207b6572726f722046495253547d3b63617463682024626f64792066697273743b73657420626f6479207b6572726f72205345434f4e447d3b63617463682024626f6479207365636f6e643b6c6973742024666972737420247365636f6e6420243a3a6572726f72496e666f\t0\t4649525354205345434f4e44207b5345434f4e440a202020207768696c6520657865637574696e670a226572726f72205345434f4e44227d"
  ]
}
```

### jim

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`provenance.json` (`file-0-row-6`):

```json
{
  "engine": "jim",
  "exit": 0,
  "stdout": "0\t4649525354205345434f4e44"
}
```

`windows.tsv` (`file-7`):

```json
{
  "provider_rows": [
    "jim\t73657420626f6479207b6572726f722046495253547d3b63617463682024626f64792066697273743b73657420626f6479207b6572726f72205345434f4e447d3b63617463682024626f6479207365636f6e643b6c6973742024666972737420247365636f6e64\t0\t4649525354205345434f4e44"
  ]
}
```

### bigip

Status: `not-tested`.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [runtime/rust/tests/data/native_dynamic_catch_reset/provenance.json](../../../../runtime/rust/tests/data/native_dynamic_catch_reset/provenance.json)
- [runtime/rust/tests/data/native_dynamic_catch_reset/windows.tsv](../../../../runtime/rust/tests/data/native_dynamic_catch_reset/windows.tsv)
