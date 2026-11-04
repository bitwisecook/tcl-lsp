# naming.variable.original-array-root-write-diagnostic

Kind: `native-observation`

## Problem statement

An existing array root is a different variable receiver from an array element. Literal, dynamic and procedure source entries need their actual release-specific write result and errorCode; a read-failure tuple or presentation label cannot establish that boundary.

## Question

What catch result, errorCode and remaining array state follow the three fixed source entries that write an existing array root?

## Conclusion

C8.4.20 and C8.5.19 refuse all three writes with variable is array and errorCode NONE. C8.6.18, C9.0.4 and C9.1.0 refuse with the same result and TCL WRITE VARNAME; each array remains present. Current Jim succeeds with NEW, errorCode NONE and no remaining array. These eighteen source-file CLI results remain separate from original receiver identity, physical storage and compiler admission.

## Scope

One actual CLI interpreter per selected provider executes the same fixed ASCII LF source in literal, dynamic-command and procedure order; each form starts with a recreated array a containing k=OLD. Six separately printed patchlevels and eighteen complete result/error/state rows are retained. No arbitrary array names, opaque input, observers, physical cell/object/header/cache, original counted-argv ingress, compiler entry, Native Normal, pointer layout or BIG-IP answer is inferred.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Original CLI executable; exact SDK/header/library/build/source associations are retained in the receipt. No compilation command is executed by this CLI runner.. Channel: ASCII LF source-file CLI; three existing-array root variable write cases. Dialect: C Tcl.

All three writes return code1 with can't set "a": variable is array; array exists1. The recorded errorCode is NONE.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Original CLI executable; exact SDK/header/library/build/source associations are retained in the receipt. No compilation command is executed by this CLI runner.. Channel: ASCII LF source-file CLI; three existing-array root variable write cases. Dialect: C Tcl.

All three writes return code1 with can't set "a": variable is array; array exists1. The recorded errorCode is NONE.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Original CLI executable; exact SDK/header/library/build/source associations are retained in the receipt. No compilation command is executed by this CLI runner.. Channel: ASCII LF source-file CLI; three existing-array root variable write cases. Dialect: C Tcl.

All three writes return code1 with can't set "a": variable is array; array exists1. The recorded errorCode is TCL WRITE VARNAME.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Original CLI executable; exact SDK/header/library/build/source associations are retained in the receipt. No compilation command is executed by this CLI runner.. Channel: ASCII LF source-file CLI; three existing-array root variable write cases. Dialect: C Tcl.

All three writes return code1 with can't set "a": variable is array; array exists1. The recorded errorCode is TCL WRITE VARNAME.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Original CLI executable; exact SDK/header/library/build/source associations are retained in the receipt. No compilation command is executed by this CLI runner.. Channel: ASCII LF source-file CLI; three existing-array root variable write cases. Dialect: C Tcl.

All three writes return code1 with can't set "a": variable is array; array exists1. The recorded errorCode is TCL WRITE VARNAME.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Original CLI executable; exact SDK/header/library/build/source associations are retained in the receipt. No compilation command is executed by this CLI runner.. Channel: ASCII LF source-file CLI; three existing-array root variable write cases. Dialect: Jim.

All three writes return code0 and NEW, errorCode NONE and array exists0: the existing array becomes a scalar in these fixed source entries.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No BIG-IP attempt or corresponding source-file CLI answer is attached.

## Historical references

- `array-write-source` (input): [rust/tcl-registry/tests/data/native_variable_array_write/probe.tcl](../../../../rust/tcl-registry/tests/data/native_variable_array_write/probe.tcl). Exact ASCII LF source, including actual version query and the ordered literal, dynamic and procedure array-root write entries. Each entry creates and then removes its own test array.
- `array-write-runner` (input): [rust/tcl-registry/tests/data/native_variable_array_write/capture.py](../../../../rust/tcl-registry/tests/data/native_variable_array_write/capture.py). Retained original Root CLI runner; selected SDK inputs and new provider output directories are independent replay prerequisites.
- `array-write-sdk-inputs` (input): [rust/tcl-registry/tests/data/native_variable_array_write/sdk-queue.json](../../../../rust/tcl-registry/tests/data/native_variable_array_write/sdk-queue.json). JSON pointer `/providers`. Exact original queue bytes; only the providers array selects this CLI runner’s SDK/executable/environment inputs. Other queue fields supply no array-write observation.
- `array-write-8.4.20-receipt` (provider): [rust/tcl-registry/tests/data/native_variable_array_write/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_variable_array_write/8.4.20/receipt.json). Exact CLI command, queried patchlevel, original source/runner, executable and SDK pins, raw-stream hashes and process result.
- `array-write-8.4.20-stdout` (observation): [rust/tcl-registry/tests/data/native_variable_array_write/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_variable_array_write/8.4.20/stdout). The original PATCHLEVEL row and all three complete RESULT rows: catch status/result/errorCode and array-exists after the write.
- `array-write-8.4.20-stderr` (observation): [rust/tcl-registry/tests/data/native_variable_array_write/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_variable_array_write/8.4.20/stderr). Exact empty stderr from this process; no broader callback or exception absence is inferred.
- `array-write-8.5.19-receipt` (provider): [rust/tcl-registry/tests/data/native_variable_array_write/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_variable_array_write/8.5.19/receipt.json). Exact CLI command, queried patchlevel, original source/runner, executable and SDK pins, raw-stream hashes and process result.
- `array-write-8.5.19-stdout` (observation): [rust/tcl-registry/tests/data/native_variable_array_write/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_variable_array_write/8.5.19/stdout). The original PATCHLEVEL row and all three complete RESULT rows: catch status/result/errorCode and array-exists after the write.
- `array-write-8.5.19-stderr` (observation): [rust/tcl-registry/tests/data/native_variable_array_write/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_variable_array_write/8.5.19/stderr). Exact empty stderr from this process; no broader callback or exception absence is inferred.
- `array-write-8.6.18-receipt` (provider): [rust/tcl-registry/tests/data/native_variable_array_write/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_variable_array_write/8.6.18/receipt.json). Exact CLI command, queried patchlevel, original source/runner, executable and SDK pins, raw-stream hashes and process result.
- `array-write-8.6.18-stdout` (observation): [rust/tcl-registry/tests/data/native_variable_array_write/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_variable_array_write/8.6.18/stdout). The original PATCHLEVEL row and all three complete RESULT rows: catch status/result/errorCode and array-exists after the write.
- `array-write-8.6.18-stderr` (observation): [rust/tcl-registry/tests/data/native_variable_array_write/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_variable_array_write/8.6.18/stderr). Exact empty stderr from this process; no broader callback or exception absence is inferred.
- `array-write-9.0.4-receipt` (provider): [rust/tcl-registry/tests/data/native_variable_array_write/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_variable_array_write/9.0.4/receipt.json). Exact CLI command, queried patchlevel, original source/runner, executable and SDK pins, raw-stream hashes and process result.
- `array-write-9.0.4-stdout` (observation): [rust/tcl-registry/tests/data/native_variable_array_write/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_variable_array_write/9.0.4/stdout). The original PATCHLEVEL row and all three complete RESULT rows: catch status/result/errorCode and array-exists after the write.
- `array-write-9.0.4-stderr` (observation): [rust/tcl-registry/tests/data/native_variable_array_write/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_variable_array_write/9.0.4/stderr). Exact empty stderr from this process; no broader callback or exception absence is inferred.
- `array-write-9.1.0-receipt` (provider): [rust/tcl-registry/tests/data/native_variable_array_write/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_variable_array_write/9.1.0/receipt.json). Exact CLI command, queried patchlevel, original source/runner, executable and SDK pins, raw-stream hashes and process result.
- `array-write-9.1.0-stdout` (observation): [rust/tcl-registry/tests/data/native_variable_array_write/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_variable_array_write/9.1.0/stdout). The original PATCHLEVEL row and all three complete RESULT rows: catch status/result/errorCode and array-exists after the write.
- `array-write-9.1.0-stderr` (observation): [rust/tcl-registry/tests/data/native_variable_array_write/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_variable_array_write/9.1.0/stderr). Exact empty stderr from this process; no broader callback or exception absence is inferred.
- `array-write-jim-receipt` (provider): [rust/tcl-registry/tests/data/native_variable_array_write/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_variable_array_write/jim/receipt.json). Exact CLI command, queried patchlevel, original source/runner, executable and SDK pins, raw-stream hashes and process result.
- `array-write-jim-stdout` (observation): [rust/tcl-registry/tests/data/native_variable_array_write/jim/stdout](../../../../rust/tcl-registry/tests/data/native_variable_array_write/jim/stdout). The original PATCHLEVEL row and all three complete RESULT rows: catch status/result/errorCode and array-exists after the write.
- `array-write-jim-stderr` (observation): [rust/tcl-registry/tests/data/native_variable_array_write/jim/stderr](../../../../rust/tcl-registry/tests/data/native_variable_array_write/jim/stderr). Exact empty stderr from this process; no broader callback or exception absence is inferred.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `native_variable_failure_code`: Select only the measured C ValueWrite/IsArray report tuple. NameLookup, Read, current receiver identity and physical storage remain independent.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `naming::native::tests::c_array_root_write_failure_uses_the_measured_value_store_tuple` (linked): Rust-only site-table and mismatch coverage: the selected C ValueWrite/IsArray tuple retains actual NONE versus TCL WRITE VARNAME; Read and inappropriate NameLookup sites do not acquire this privilege. Native CLI outputs establish no private receiver layout.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::array_root_write_diagnostics_match_five_original_source_controls` (linked): Replay the identical complete ASCII source through an independently authentic C interpreter and compare all three original result/error/state rows. Only the separately identified process-version line is excluded; compiler/receiver/storage prerequisites are independent.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::array_root_write_preserves_current_jim_dictionary_replacement` (linked): Replay the same complete original Jim source and compare all three successful NEW/NONE/no-array rows; dictionary replacement is a finite source result, not a private physical-layout or cache observation.
- [rust/tcl-vm/src/interp/native_variable_names.rs](../../../../rust/tcl-vm/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::array_root_write_diagnostics_match_five_original_source_controls` (linked): Replay the identical complete ASCII source through an independently authentic C interpreter and compare all three original result/error/state rows. Only the separately identified process-version line is excluded; compiler/receiver/storage prerequisites are independent.
- [rust/tcl-vm/src/interp/native_variable_names.rs](../../../../rust/tcl-vm/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::array_root_write_preserves_current_jim_dictionary_replacement` (linked): Replay the same complete original Jim source and compare all three successful NEW/NONE/no-array rows; dictionary replacement is a finite source result, not a private physical-layout or cache observation.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "rust/tcl-registry/tests/data/native_variable_array_write/capture.py"
]
```

The retained runner reads its original absolute SDK-binding queue and expects new provider output directories. Replay requires independently matching SDK/environment/executable inputs and a fresh output location. Exact native source observations do not execute a linked Rust comparison or grant another variable receiver, storage or source-edit purpose.
