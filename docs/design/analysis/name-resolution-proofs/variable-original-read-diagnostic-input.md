# naming.variable.original-read-diagnostic-input

Kind: `native-observation`

## Problem statement

A variable Read and a write to the same displayed receiver have different failure presenters. C array-root reads can fail where Jim dictionary-root reads succeed, and scalar-element failures must retain the original requested read name. Literal, dynamic-head and procedure source entries need separate recorded controls.

## Question

For the six original ASCII source forms, what exact caught Read result does each provider return for an array/dictionary root a and for b(k) when b is a scalar?

## Conclusion

All five C providers return code1 with can't read "a": variable is array for literal, dynamic-head and procedure array-root reads. Current Jim returns code0 with k OLD for those three dictionary-root reads. All six providers return code1 with can't read "b(k)": variable isn't array for the three scalar-element reads. These are Read presentations; write or captured-increment error recipes remain separate.

## Scope

Six fixed complete ASCII LF source-file CLI forms per actual C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and current Jim process. Actual queried versions identify each provider; outer process status is distinct from the six catches. No opaque/counting name channel, private receiver/object/header/cache identity, source compiler admission, captured increment, callback lifetime, general dictionary ordering or BIG-IP result is measured. The recreated-element callback question remains independent.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual executable; original SDK/header/library/source/environment pins are retained in the receipt.. Channel: Exact complete original ASCII LF source-file CLI; six variable Read controls.. Dialect: Tcl.

Exact original stdout:

```text
PATCHLEVEL 8.4.20
RESULT literal_array_read 1 {can't read "a": variable is array}
RESULT dynamic_array_read 1 {can't read "a": variable is array}
RESULT procedure_array_read 1 {can't read "a": variable is array}
RESULT literal_scalar_element 1 {can't read "b(k)": variable isn't array}
RESULT dynamic_scalar_element 1 {can't read "b(k)": variable isn't array}
RESULT procedure_scalar_element 1 {can't read "b(k)": variable isn't array}
```

Outer process status 0; stderr empty. These finite Read outcomes supply no write, callback, captured receiver, object/header/cache or compiler admission receipt.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual executable; original SDK/header/library/source/environment pins are retained in the receipt.. Channel: Exact complete original ASCII LF source-file CLI; six variable Read controls.. Dialect: Tcl.

Exact original stdout:

```text
PATCHLEVEL 8.5.19
RESULT literal_array_read 1 {can't read "a": variable is array}
RESULT dynamic_array_read 1 {can't read "a": variable is array}
RESULT procedure_array_read 1 {can't read "a": variable is array}
RESULT literal_scalar_element 1 {can't read "b(k)": variable isn't array}
RESULT dynamic_scalar_element 1 {can't read "b(k)": variable isn't array}
RESULT procedure_scalar_element 1 {can't read "b(k)": variable isn't array}
```

Outer process status 0; stderr empty. These finite Read outcomes supply no write, callback, captured receiver, object/header/cache or compiler admission receipt.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual executable; original SDK/header/library/source/environment pins are retained in the receipt.. Channel: Exact complete original ASCII LF source-file CLI; six variable Read controls.. Dialect: Tcl.

Exact original stdout:

```text
PATCHLEVEL 8.6.18
RESULT literal_array_read 1 {can't read "a": variable is array}
RESULT dynamic_array_read 1 {can't read "a": variable is array}
RESULT procedure_array_read 1 {can't read "a": variable is array}
RESULT literal_scalar_element 1 {can't read "b(k)": variable isn't array}
RESULT dynamic_scalar_element 1 {can't read "b(k)": variable isn't array}
RESULT procedure_scalar_element 1 {can't read "b(k)": variable isn't array}
```

Outer process status 0; stderr empty. These finite Read outcomes supply no write, callback, captured receiver, object/header/cache or compiler admission receipt.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual executable; original SDK/header/library/source/environment pins are retained in the receipt.. Channel: Exact complete original ASCII LF source-file CLI; six variable Read controls.. Dialect: Tcl.

Exact original stdout:

```text
PATCHLEVEL 9.0.4
RESULT literal_array_read 1 {can't read "a": variable is array}
RESULT dynamic_array_read 1 {can't read "a": variable is array}
RESULT procedure_array_read 1 {can't read "a": variable is array}
RESULT literal_scalar_element 1 {can't read "b(k)": variable isn't array}
RESULT dynamic_scalar_element 1 {can't read "b(k)": variable isn't array}
RESULT procedure_scalar_element 1 {can't read "b(k)": variable isn't array}
```

Outer process status 0; stderr empty. These finite Read outcomes supply no write, callback, captured receiver, object/header/cache or compiler admission receipt.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual executable; original SDK/header/library/source/environment pins are retained in the receipt.. Channel: Exact complete original ASCII LF source-file CLI; six variable Read controls.. Dialect: Tcl.

Exact original stdout:

```text
PATCHLEVEL 9.1.0
RESULT literal_array_read 1 {can't read "a": variable is array}
RESULT dynamic_array_read 1 {can't read "a": variable is array}
RESULT procedure_array_read 1 {can't read "a": variable is array}
RESULT literal_scalar_element 1 {can't read "b(k)": variable isn't array}
RESULT dynamic_scalar_element 1 {can't read "b(k)": variable isn't array}
RESULT procedure_scalar_element 1 {can't read "b(k)": variable isn't array}
```

Outer process status 0; stderr empty. These finite Read outcomes supply no write, callback, captured receiver, object/header/cache or compiler admission receipt.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual executable; original SDK/header/library/source/environment pins are retained in the receipt.. Channel: Exact complete original ASCII LF source-file CLI; six variable Read controls.. Dialect: Jim Tcl.

Exact original stdout:

```text
PATCHLEVEL 0.84-9-g5bac7c9
RESULT literal_array_read 0 {k OLD}
RESULT dynamic_array_read 0 {k OLD}
RESULT procedure_array_read 0 {k OLD}
RESULT literal_scalar_element 1 {can't read "b(k)": variable isn't array}
RESULT dynamic_scalar_element 1 {can't read "b(k)": variable isn't array}
RESULT procedure_scalar_element 1 {can't read "b(k)": variable isn't array}
```

Outer process status 0; stderr empty. These finite Read outcomes supply no write, callback, captured receiver, object/header/cache or compiler admission receipt.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native interpreter observation establishes this Rust source-advice invariant.

## Historical references

- `source` (input): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/probe.tcl](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/probe.tcl). Exact original complete ASCII LF source containing six caught literal/dynamic/procedure Read controls.
- `runner` (provider): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/capture.py](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/capture.py). Exact original Root capture runner; original command vectors and queue dependency retained.
- `queue` (provider): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/queue.json](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/queue.json). Exact provider executable/SDK/header/library/source/environment selection pins.
- `tcl8.4-receipt` (provider): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.4.20/receipt.json). Exact original process argv/status, queried version, source/runner/executable/SDK/environment and raw-stream digests.
- `tcl8.4-stdout` (observation): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.4.20/stdout). Exact original complete stdout bytes; six caught Read results are independent of outer process status.
- `tcl8.4-stderr` (observation): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.4.20/stderr). Exact original complete stderr bytes; six caught Read results are independent of outer process status.
- `tcl8.5-receipt` (provider): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.5.19/receipt.json). Exact original process argv/status, queried version, source/runner/executable/SDK/environment and raw-stream digests.
- `tcl8.5-stdout` (observation): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.5.19/stdout). Exact original complete stdout bytes; six caught Read results are independent of outer process status.
- `tcl8.5-stderr` (observation): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.5.19/stderr). Exact original complete stderr bytes; six caught Read results are independent of outer process status.
- `tcl8.6-receipt` (provider): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.6.18/receipt.json). Exact original process argv/status, queried version, source/runner/executable/SDK/environment and raw-stream digests.
- `tcl8.6-stdout` (observation): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.6.18/stdout). Exact original complete stdout bytes; six caught Read results are independent of outer process status.
- `tcl8.6-stderr` (observation): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.6.18/stderr). Exact original complete stderr bytes; six caught Read results are independent of outer process status.
- `tcl9.0-receipt` (provider): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/9.0.4/receipt.json). Exact original process argv/status, queried version, source/runner/executable/SDK/environment and raw-stream digests.
- `tcl9.0-stdout` (observation): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/9.0.4/stdout). Exact original complete stdout bytes; six caught Read results are independent of outer process status.
- `tcl9.0-stderr` (observation): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/9.0.4/stderr). Exact original complete stderr bytes; six caught Read results are independent of outer process status.
- `tcl9.1-receipt` (provider): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/9.1.0/receipt.json). Exact original process argv/status, queried version, source/runner/executable/SDK/environment and raw-stream digests.
- `tcl9.1-stdout` (observation): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/9.1.0/stdout). Exact original complete stdout bytes; six caught Read results are independent of outer process status.
- `tcl9.1-stderr` (observation): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/9.1.0/stderr). Exact original complete stderr bytes; six caught Read results are independent of outer process status.
- `jim-receipt` (provider): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/jim/receipt.json). Exact original process argv/status, queried version, source/runner/executable/SDK/environment and raw-stream digests.
- `jim-stdout` (observation): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/jim/stdout](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/jim/stdout). Exact original complete stdout bytes; six caught Read results are independent of outer process status.
- `jim-stderr` (observation): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/jim/stderr](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/jim/stderr). Exact original complete stderr bytes; six caught Read results are independent of outer process status.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `Interp::read_original_c_variable`: Use the original selected C input and Read purpose through the actual receiver owner; the captured source results do not issue private allocation identity.
- [rust/tcl-vm/src/interp/native_variable_names.rs](../../../../rust/tcl-vm/src/interp/native_variable_names.rs), `Interp::read_original_c_variable`: Use the original selected C input and Read purpose through the actual receiver owner; the captured source results do not issue private allocation identity.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `report_native_c_variable_value_name`: Present the original C compound input through the shared selected native name recipe independently of storage selection.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `Interp::original_c_variable_reporting_input`: Retain the original input separately from reached receiver geometry for late failure presentation.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `Interp::original_c_variable_failure_input`: Select the authentic failed operation input without changing Read into Write.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `Interp::original_c_variable_receiver_error`: Apply the selected operation-specific native error presenter to the authentic receiver failure.
- [runtime/rust/src/interp/native_body_artifact.rs](../../../../runtime/rust/src/interp/native_body_artifact.rs), `Interp::body_read_original_parts_using`: Retain original compound operands and the actual BodyObjectOperation stage through the shared Read worker and its Read-specific failure presenter; source observations supply no private allocation identity or compiled-body admission.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::variable_read_diagnostics_match_five_original_source_controls` (linked): Replay the identical complete original six-form source for each of five C providers and compare every Read result after separate PATCHLEVEL reporting. Literal, dynamic-head and procedure array-root and scalar-element failures retain their exact Read names; this is linked coverage, not an executed result.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::variable_read_input_preserves_current_jim_dictionary_results` (linked): Replay the identical complete original six-form Jim source and compare its successful dictionary-root reads and failing scalar-element reads after separate PATCHLEVEL reporting. This grants no private receiver or compiler admission and claims no executed result.
- [rust/tcl-vm/src/interp/native_variable_names.rs](../../../../rust/tcl-vm/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::variable_read_diagnostics_match_five_original_source_controls` (linked): Replay the identical complete original six-form source for each of five C providers and compare every Read result after separate PATCHLEVEL reporting. Literal, dynamic-head and procedure array-root and scalar-element failures retain their exact Read names; this is linked coverage, not an executed result.
- [rust/tcl-vm/src/interp/native_variable_names.rs](../../../../rust/tcl-vm/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::variable_read_input_preserves_current_jim_dictionary_results` (linked): Replay the identical complete original six-form Jim source and compare its successful dictionary-root reads and failing scalar-element reads after separate PATCHLEVEL reporting. This grants no private receiver or compiler admission and claims no executed result.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "/workspace/.proofs/consumer-native-variable-read149/capture.py"
]
```

The original runner retains capture-machine source/executable paths and its exact provider queue dependency. Replay requires independently verified matching SDK/source/executable/environment pins and fresh output paths. This review executes no native process or Rust test; these finite guest Read results issue no compiler or private receiver authority.
