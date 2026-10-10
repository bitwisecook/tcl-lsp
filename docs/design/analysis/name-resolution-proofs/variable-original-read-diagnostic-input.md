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

Status: `observed`. Version: 8.4.20. Build: Actual executable SHA-256 f0e742ccc08a150afd9a0a236ce24c8e12098dae38c0efd46e59f9b1a28bf58a; original SDK/header/library/source/environment pins are retained in the receipt.. Channel: Exact complete original ASCII LF source-file CLI; six variable Read controls.. Dialect: Tcl.

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

Status: `observed`. Version: 8.5.19. Build: Actual executable SHA-256 e7eb17821aeb7b1f84619f431e89ecff667bc5461fa8e00d67498b6b7d5dd61a; original SDK/header/library/source/environment pins are retained in the receipt.. Channel: Exact complete original ASCII LF source-file CLI; six variable Read controls.. Dialect: Tcl.

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

Status: `observed`. Version: 8.6.18. Build: Actual executable SHA-256 9b156375d068a58fcfb82a2a0afd170afd2b03912cbb6e588676c7688118c8ff; original SDK/header/library/source/environment pins are retained in the receipt.. Channel: Exact complete original ASCII LF source-file CLI; six variable Read controls.. Dialect: Tcl.

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

Status: `observed`. Version: 9.0.4. Build: Actual executable SHA-256 f4748357cdbf2762f1c5c05916fa53e54718d087120ae9de9c403bbdfaf764b1; original SDK/header/library/source/environment pins are retained in the receipt.. Channel: Exact complete original ASCII LF source-file CLI; six variable Read controls.. Dialect: Tcl.

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

Status: `observed`. Version: 9.1.0. Build: Actual executable SHA-256 2a82f721b68f2ed2347ca4e3daa9a56c49c9e3efce77d00f22fc3b2012d8814d; original SDK/header/library/source/environment pins are retained in the receipt.. Channel: Exact complete original ASCII LF source-file CLI; six variable Read controls.. Dialect: Tcl.

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

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual executable SHA-256 e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806; original SDK/header/library/source/environment pins are retained in the receipt.. Channel: Exact complete original ASCII LF source-file CLI; six variable Read controls.. Dialect: Jim Tcl.

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

## Exact evidence

- `source` (input): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/probe.tcl](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/probe.tcl). SHA-256 `ea7d2eff456aeaab5a31784731649f59ac049f73a08434db375aa8b58c2dd56f`. Exact original complete ASCII LF source containing six caught literal/dynamic/procedure Read controls.
- `runner` (provider): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/capture.py](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/capture.py). SHA-256 `dbe21a5970d52155c2b6c94fcbfe478b10ba7789e273e22815f538a73c86b403`. Exact original Root capture runner; original command vectors and queue dependency retained.
- `queue` (provider): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/queue.json](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/queue.json). SHA-256 `3eb6871f7e4def2770d65bde01d20ccbe7e7840b3128da03ba072c473292db7e`. Exact provider executable/SDK/header/library/source/environment selection pins.
- `tcl8.4-receipt` (provider): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.4.20/receipt.json). SHA-256 `9da9435e9c88eb4239c6044e99b66bb6ffbeadb25f32b4c08b7142ba6a5b534d`. Exact original process argv/status, queried version, source/runner/executable/SDK/environment and raw-stream digests.
- `tcl8.4-stdout` (observation): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.4.20/stdout). SHA-256 `c56afd467378baba61cf7a96b8fa7e48d7af8533f6bf51907d79c4a9629f3acb`. Exact original complete stdout bytes; six caught Read results are independent of outer process status.
- `tcl8.4-stderr` (observation): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original complete stderr bytes; six caught Read results are independent of outer process status.
- `tcl8.5-receipt` (provider): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.5.19/receipt.json). SHA-256 `049c2659834326aa57a3d8d3e51ede1b46cda2b87a40496f1630d71d547ecff0`. Exact original process argv/status, queried version, source/runner/executable/SDK/environment and raw-stream digests.
- `tcl8.5-stdout` (observation): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.5.19/stdout). SHA-256 `72fea88f4d9010454ee048a57d18bfb42593b8fb9e6b47bbf413bc2f6861f4bd`. Exact original complete stdout bytes; six caught Read results are independent of outer process status.
- `tcl8.5-stderr` (observation): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original complete stderr bytes; six caught Read results are independent of outer process status.
- `tcl8.6-receipt` (provider): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.6.18/receipt.json). SHA-256 `28f24f068dd4ec8a8fdf583c2c573380eeb21ec31288740ae25dc2985cac362d`. Exact original process argv/status, queried version, source/runner/executable/SDK/environment and raw-stream digests.
- `tcl8.6-stdout` (observation): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.6.18/stdout). SHA-256 `1c24f6ecd2de10a2fe5c49c4a347f9ce726a84baa569d830346e0a496e8165ff`. Exact original complete stdout bytes; six caught Read results are independent of outer process status.
- `tcl8.6-stderr` (observation): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original complete stderr bytes; six caught Read results are independent of outer process status.
- `tcl9.0-receipt` (provider): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/9.0.4/receipt.json). SHA-256 `26a23bea8245553357ad976abc8718efc193ae7b8e9ac125ee62a6232ce8727d`. Exact original process argv/status, queried version, source/runner/executable/SDK/environment and raw-stream digests.
- `tcl9.0-stdout` (observation): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/9.0.4/stdout). SHA-256 `f50895b154f2faa8d38e91033829bd29b5b8ea3a3dbfbf506b873b86fb3ea070`. Exact original complete stdout bytes; six caught Read results are independent of outer process status.
- `tcl9.0-stderr` (observation): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original complete stderr bytes; six caught Read results are independent of outer process status.
- `tcl9.1-receipt` (provider): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/9.1.0/receipt.json). SHA-256 `9015c451f01838921df671230464fbf3e4c0e16fd1d594dac0332d1678e23897`. Exact original process argv/status, queried version, source/runner/executable/SDK/environment and raw-stream digests.
- `tcl9.1-stdout` (observation): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/9.1.0/stdout). SHA-256 `6e90cf5d75046393ecbec1e85009c3904138aa9a56ac0199880371b5c795d136`. Exact original complete stdout bytes; six caught Read results are independent of outer process status.
- `tcl9.1-stderr` (observation): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original complete stderr bytes; six caught Read results are independent of outer process status.
- `jim-receipt` (provider): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/jim/receipt.json). SHA-256 `3c054bcf3b09aef4e95e48bd45402b242bbeb3907fc9471b34ce79dababfcce0`. Exact original process argv/status, queried version, source/runner/executable/SDK/environment and raw-stream digests.
- `jim-stdout` (observation): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/jim/stdout](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/jim/stdout). SHA-256 `bb5ff3aa389ed3321b169ec777fb8a8877f635f8240b5101d699b831286986a9`. Exact original complete stdout bytes; six caught Read results are independent of outer process status.
- `jim-stderr` (observation): [rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/jim/stderr](../../../../rust/tcl-registry/tests/data/native_variable_read_diagnostic_original/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original complete stderr bytes; six caught Read results are independent of outer process status.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `Interp::read_original_c_variable`: Use the original selected C input and Read purpose through the actual receiver owner; the captured source results do not issue private allocation identity.
- [rust/tcl-vm/src/interp/native_variable_names.rs](../../../../rust/tcl-vm/src/interp/native_variable_names.rs), `Interp::read_original_c_variable`: Use the original selected C input and Read purpose through the actual receiver owner; the captured source results do not issue private allocation identity.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `report_native_c_variable_value_name`: Present the original C compound input through the shared selected native name recipe independently of storage selection.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `Interp::original_c_variable_reporting_input`: Retain the original input separately from reached receiver geometry for late failure presentation.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `Interp::original_c_variable_failure_input`: Select the authentic failed operation input without changing Read into Write.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `Interp::original_c_variable_receiver_error`: Apply the selected operation-specific native error presenter to the authentic receiver failure.
- [runtime/rust/src/interp/native_body_artifact.rs](../../../../runtime/rust/src/interp/native_body_artifact.rs), `Interp::body_read_original_parts`: Retain original compound operands through the actual body Read owner rather than using a write-only presenter.
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
