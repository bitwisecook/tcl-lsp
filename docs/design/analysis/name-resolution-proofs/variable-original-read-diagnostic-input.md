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

Status: `observed`. Channel: Exact complete original ASCII LF source-file CLI; six variable Read controls. Version: tcl8.4 8.4.20.

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

Status: `observed`. Channel: Exact complete original ASCII LF source-file CLI; six variable Read controls. Version: tcl8.5 8.5.19.

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

Status: `observed`. Channel: Exact complete original ASCII LF source-file CLI; six variable Read controls. Version: tcl8.6 8.6.18.

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

Status: `observed`. Channel: Exact complete original ASCII LF source-file CLI; six variable Read controls. Version: tcl9.0 9.0.4.

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

Status: `observed`. Channel: Exact complete original ASCII LF source-file CLI; six variable Read controls. Version: tcl9.1 9.1.0.

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

Status: `observed`. Channel: Exact complete original ASCII LF source-file CLI; six variable Read controls. Version: jim 0.84-9-g5bac7c9.

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

Status: `not-tested`.

No native interpreter observation establishes this Rust source-advice invariant.

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

## Evidence

- [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original)
- [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.4.20](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.4.20)
- [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.5.19](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.5.19)
- [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.6.18](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.6.18)
- [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/9.0.4](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/9.0.4)
- [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/9.1.0](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/9.1.0)
- [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/jim](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/jim)
