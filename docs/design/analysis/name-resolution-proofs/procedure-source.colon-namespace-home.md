# naming.procedure-source.colon-namespace-home

Kind: `native-observation`

## Problem statement

Constructing namespace : can select a different home than reconstructing a displayed namespace string. A procedure called from that original namespace must report its actual namespace home.

## Question

What namespace current result follows defining and invoking p under namespace eval :?

## Conclusion

C8.4/8.5 and Jim return ::; C8.6/9.0/9.1 return :::. All outer catch completions are0. No native namespace primary/header or arbitrary same-display command correspondence is sampled.

## Scope

Three exact ASCII source controls×six native shell associations; outer catch and binary-scan presenter are retained in run.py. Each shell/case original complete stdout/stderr/status/digest is attached, with an exact separate rows projection. Shell labels are recorded but full patchlevel/source revision/configure identities were not queried. Script versus native object-vector semantics and inner versus outer catch completion remain separate; BIG-IP not tested.

## Provider answers

### tcl8.4

Status: `observed`. Version: tcl8.4 (shell association; full patchlevel/revision unrecorded). Build: Original shell SHA256 551bef3a9b51f256f4ad9dec06bddc42ceab872a72528e83676d1d7df715b9d4; process0/empty stderr. Compiler/configure/header/library identity unrecorded.. Channel: Original complete UTF-8 ASCII wrapper fed to selected shell stdin; outer catch + binary-scan presenter.. Dialect: C Tcl.

Original stdout: '0 3a3a\n'; decoded outer result: '::'. Inner caught proc result is separate from outer presenter code0.

### tcl8.5

Status: `observed`. Version: tcl8.5 (shell association; full patchlevel/revision unrecorded). Build: Original shell SHA256 7a0fb4aa272a45662fb883295d0037a8c18e3e32c635736af8314fb835981935; process0/empty stderr. Compiler/configure/header/library identity unrecorded.. Channel: Original complete UTF-8 ASCII wrapper fed to selected shell stdin; outer catch + binary-scan presenter.. Dialect: C Tcl.

Original stdout: '0 3a3a\n'; decoded outer result: '::'. Inner caught proc result is separate from outer presenter code0.

### tcl8.6

Status: `observed`. Version: tcl8.6 (shell association; full patchlevel/revision unrecorded). Build: Original shell SHA256 0cc67113840d03c15c15d88188c16b8b019aa2a8c11ca6980f32723fbedacde0; process0/empty stderr. Compiler/configure/header/library identity unrecorded.. Channel: Original complete UTF-8 ASCII wrapper fed to selected shell stdin; outer catch + binary-scan presenter.. Dialect: C Tcl.

Original stdout: '0 3a3a3a\n'; decoded outer result: ':::'. Inner caught proc result is separate from outer presenter code0.

### tcl9.0

Status: `observed`. Version: tcl9.0 (shell association; full patchlevel/revision unrecorded). Build: Original shell SHA256 cc28851d04548dce27006a317d22b78c67e5491204f66f924c20d1f3febf6c18; process0/empty stderr. Compiler/configure/header/library identity unrecorded.. Channel: Original complete UTF-8 ASCII wrapper fed to selected shell stdin; outer catch + binary-scan presenter.. Dialect: C Tcl.

Original stdout: '0 3a3a3a\n'; decoded outer result: ':::'. Inner caught proc result is separate from outer presenter code0.

### tcl9.1

Status: `observed`. Version: tcl9.1 (shell association; full patchlevel/revision unrecorded). Build: Original shell SHA256 d4c2dbd11cd88994b7ca992eb87cdc855750f75d5d9f42b681b4dd4c321c791c; process0/empty stderr. Compiler/configure/header/library identity unrecorded.. Channel: Original complete UTF-8 ASCII wrapper fed to selected shell stdin; outer catch + binary-scan presenter.. Dialect: C Tcl.

Original stdout: '0 3a3a3a\n'; decoded outer result: ':::'. Inner caught proc result is separate from outer presenter code0.

### jim

Status: `observed`. Version: jim (shell association; full patchlevel/revision unrecorded). Build: Original shell SHA256 d5b47eb75b271d50331ca5a38775492cb186f8feedd3c76bb2a0fbf1905671f0; process0/empty stderr. Compiler/configure/header/library identity unrecorded.. Channel: Original complete UTF-8 ASCII wrapper fed to selected shell stdin; outer catch + binary-scan presenter.. Dialect: Jim Tcl.

Original stdout: '0 3a3a\n'; decoded outer result: '::'. Inner caught proc result is separate from outer presenter code0.

### bigip

Status: `not-tested`. Version: not measured. Build: not measured. Channel: not tested. Dialect: F5 iRules.

No appliance execution of these procedure source controls is attached.

## Exact evidence

- `inputs` (input): [rust/tcl-syntax/tests/data/native_procedure_name/cases.json](../../../../rust/tcl-syntax/tests/data/native_procedure_name/cases.json). SHA-256 `54d1ea302fd422ea940a6515829534de798a4966bcb849d78cab715f2a2c68dd`. Exact three authored source payloads checked against original capture source fields.
- `observer` (input): [rust/tcl-syntax/tests/data/native_procedure_name/run.py](../../../../rust/tcl-syntax/tests/data/native_procedure_name/run.py). SHA-256 `7e0801f7c1b18e7624e760a0bc7e972e0a5c8797b00caaef9a11d5b1b297023d`. Actual retained original outer catch/binary-scan presenter construction and subprocess input channel.
- `receipt` (observation): [rust/tcl-syntax/tests/data/native_procedure_name/manifest.json](../../../../rust/tcl-syntax/tests/data/native_procedure_name/manifest.json). SHA-256 `0427ea0b076aa02ff6426538b6fcad4970a29ddfa2989386b7deea9702d86495`. Complete original six-provider/case source, status, stdout/stderr and shell hashes.
- `rows` (observation): [rust/tcl-syntax/tests/data/native_procedure_name/rows.txt](../../../../rust/tcl-syntax/tests/data/native_procedure_name/rows.txt). SHA-256 `0bea27980e49d6c525e3ac59c944965bd3b04ee4ed41841a8dc67b1c24b2fc7c`. Exact source/result projection independently matched to all original captures.
- `case-tcl8.4` (observation): [rust/tcl-syntax/tests/data/native_procedure_name/manifest.json](../../../../rust/tcl-syntax/tests/data/native_procedure_name/manifest.json). SHA-256 `0427ea0b076aa02ff6426538b6fcad4970a29ddfa2989386b7deea9702d86495`. JSON pointer `/captures/0`. Exact original selected shell/case raw streams and status.
- `case-tcl8.5` (observation): [rust/tcl-syntax/tests/data/native_procedure_name/manifest.json](../../../../rust/tcl-syntax/tests/data/native_procedure_name/manifest.json). SHA-256 `0427ea0b076aa02ff6426538b6fcad4970a29ddfa2989386b7deea9702d86495`. JSON pointer `/captures/3`. Exact original selected shell/case raw streams and status.
- `case-tcl8.6` (observation): [rust/tcl-syntax/tests/data/native_procedure_name/manifest.json](../../../../rust/tcl-syntax/tests/data/native_procedure_name/manifest.json). SHA-256 `0427ea0b076aa02ff6426538b6fcad4970a29ddfa2989386b7deea9702d86495`. JSON pointer `/captures/6`. Exact original selected shell/case raw streams and status.
- `case-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_procedure_name/manifest.json](../../../../rust/tcl-syntax/tests/data/native_procedure_name/manifest.json). SHA-256 `0427ea0b076aa02ff6426538b6fcad4970a29ddfa2989386b7deea9702d86495`. JSON pointer `/captures/9`. Exact original selected shell/case raw streams and status.
- `case-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_procedure_name/manifest.json](../../../../rust/tcl-syntax/tests/data/native_procedure_name/manifest.json). SHA-256 `0427ea0b076aa02ff6426538b6fcad4970a29ddfa2989386b7deea9702d86495`. JSON pointer `/captures/12`. Exact original selected shell/case raw streams and status.
- `case-jim` (observation): [rust/tcl-syntax/tests/data/native_procedure_name/manifest.json](../../../../rust/tcl-syntax/tests/data/native_procedure_name/manifest.json). SHA-256 `0427ea0b076aa02ff6426538b6fcad4970a29ddfa2989386b7deea9702d86495`. JSON pointer `/captures/15`. Exact original selected shell/case raw streams and status.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/cmd_proc/native_name_tests.rs](../../../../runtime/rust/src/cmd_proc/native_name_tests.rs), `cmd_proc::native_name_tests::original_procedure_colon_names_match_all_18_native_home_and_error_controls` (linked): Compares the exact original source/case completion and result bytes; no unsampled native object getters or name/header allocation are granted.
- [rust/tcl-vm/src/command/native_procedure_name_tests.rs](../../../../rust/tcl-vm/src/command/native_procedure_name_tests.rs), `command::native_procedure_name_tests::original_procedure_colon_names_match_all_18_native_home_and_error_controls` (linked): Compares the exact original source/case completion and result bytes; no unsampled native object getters or name/header allocation are granted.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact original payloads and presenter construction are retained. The retained runner depends on an unavailable checkpoint/lock setup and is not advertised as a currently executable replay. Reconfirmation must extract the unchanged complete wrapper for each source case, supply explicit selected shell build/channel and preserve inner result/outer code/status/streams; no fresh run or Rust pass is asserted.
