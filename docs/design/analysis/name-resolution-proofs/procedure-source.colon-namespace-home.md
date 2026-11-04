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

Status: `observed`. Version: tcl8.4 (shell association; full patchlevel/revision unrecorded). Build: Original shell; process0/empty stderr. Compiler/configure/header/library identity unrecorded.. Channel: Original complete UTF-8 ASCII wrapper fed to selected shell stdin; outer catch + binary-scan presenter.. Dialect: C Tcl.

Original stdout: '0 3a3a\n'; decoded outer result: '::'. Inner caught proc result is separate from outer presenter code0.

### tcl8.5

Status: `observed`. Version: tcl8.5 (shell association; full patchlevel/revision unrecorded). Build: Original shell; process0/empty stderr. Compiler/configure/header/library identity unrecorded.. Channel: Original complete UTF-8 ASCII wrapper fed to selected shell stdin; outer catch + binary-scan presenter.. Dialect: C Tcl.

Original stdout: '0 3a3a\n'; decoded outer result: '::'. Inner caught proc result is separate from outer presenter code0.

### tcl8.6

Status: `observed`. Version: tcl8.6 (shell association; full patchlevel/revision unrecorded). Build: Original shell; process0/empty stderr. Compiler/configure/header/library identity unrecorded.. Channel: Original complete UTF-8 ASCII wrapper fed to selected shell stdin; outer catch + binary-scan presenter.. Dialect: C Tcl.

Original stdout: '0 3a3a3a\n'; decoded outer result: ':::'. Inner caught proc result is separate from outer presenter code0.

### tcl9.0

Status: `observed`. Version: tcl9.0 (shell association; full patchlevel/revision unrecorded). Build: Original shell; process0/empty stderr. Compiler/configure/header/library identity unrecorded.. Channel: Original complete UTF-8 ASCII wrapper fed to selected shell stdin; outer catch + binary-scan presenter.. Dialect: C Tcl.

Original stdout: '0 3a3a3a\n'; decoded outer result: ':::'. Inner caught proc result is separate from outer presenter code0.

### tcl9.1

Status: `observed`. Version: tcl9.1 (shell association; full patchlevel/revision unrecorded). Build: Original shell; process0/empty stderr. Compiler/configure/header/library identity unrecorded.. Channel: Original complete UTF-8 ASCII wrapper fed to selected shell stdin; outer catch + binary-scan presenter.. Dialect: C Tcl.

Original stdout: '0 3a3a3a\n'; decoded outer result: ':::'. Inner caught proc result is separate from outer presenter code0.

### jim

Status: `observed`. Version: jim (shell association; full patchlevel/revision unrecorded). Build: Original shell; process0/empty stderr. Compiler/configure/header/library identity unrecorded.. Channel: Original complete UTF-8 ASCII wrapper fed to selected shell stdin; outer catch + binary-scan presenter.. Dialect: Jim Tcl.

Original stdout: '0 3a3a\n'; decoded outer result: '::'. Inner caught proc result is separate from outer presenter code0.

### bigip

Status: `not-tested`. Version: not measured. Build: not measured. Channel: not tested. Dialect: F5 iRules.

No appliance execution of these procedure source controls is attached.

## Historical references

- `inputs` (input): [rust/tcl-syntax/tests/data/native_procedure_name/cases.json](../../../../rust/tcl-syntax/tests/data/native_procedure_name/cases.json). Exact three authored source payloads checked against original capture source fields.
- `observer` (input): [rust/tcl-syntax/tests/data/native_procedure_name/run.py](../../../../rust/tcl-syntax/tests/data/native_procedure_name/run.py). Actual retained original outer catch/binary-scan presenter construction and subprocess input channel.
- `receipt` (observation): [rust/tcl-syntax/tests/data/native_procedure_name/manifest.json](../../../../rust/tcl-syntax/tests/data/native_procedure_name/manifest.json). Complete original six-provider/case source, status, stdout/stderr and shell hashes.
- `rows` (observation): [rust/tcl-syntax/tests/data/native_procedure_name/rows.txt](../../../../rust/tcl-syntax/tests/data/native_procedure_name/rows.txt). Exact source/result projection independently matched to all original captures.
- `case-tcl8.4` (observation): [rust/tcl-syntax/tests/data/native_procedure_name/manifest.json](../../../../rust/tcl-syntax/tests/data/native_procedure_name/manifest.json). JSON pointer `/captures/0`. Exact original selected shell/case raw streams and status.
- `case-tcl8.5` (observation): [rust/tcl-syntax/tests/data/native_procedure_name/manifest.json](../../../../rust/tcl-syntax/tests/data/native_procedure_name/manifest.json). JSON pointer `/captures/3`. Exact original selected shell/case raw streams and status.
- `case-tcl8.6` (observation): [rust/tcl-syntax/tests/data/native_procedure_name/manifest.json](../../../../rust/tcl-syntax/tests/data/native_procedure_name/manifest.json). JSON pointer `/captures/6`. Exact original selected shell/case raw streams and status.
- `case-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_procedure_name/manifest.json](../../../../rust/tcl-syntax/tests/data/native_procedure_name/manifest.json). JSON pointer `/captures/9`. Exact original selected shell/case raw streams and status.
- `case-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_procedure_name/manifest.json](../../../../rust/tcl-syntax/tests/data/native_procedure_name/manifest.json). JSON pointer `/captures/12`. Exact original selected shell/case raw streams and status.
- `case-jim` (observation): [rust/tcl-syntax/tests/data/native_procedure_name/manifest.json](../../../../rust/tcl-syntax/tests/data/native_procedure_name/manifest.json). JSON pointer `/captures/15`. Exact original selected shell/case raw streams and status.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/cmd_proc/native_name_tests.rs](../../../../runtime/rust/src/cmd_proc/native_name_tests.rs), `cmd_proc::native_name_tests::original_procedure_colon_names_match_all_18_native_home_and_error_controls` (linked): Compares the exact original source/case completion and result bytes; no unsampled native object getters or name/header allocation are granted.
- [rust/tcl-vm/src/command/native_procedure_name_tests.rs](../../../../rust/tcl-vm/src/command/native_procedure_name_tests.rs), `command::native_procedure_name_tests::original_procedure_colon_names_match_all_18_native_home_and_error_controls` (linked): Compares the exact original source/case completion and result bytes; no unsampled native object getters or name/header allocation are granted.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact original payloads and presenter construction are retained. The retained runner depends on an unavailable checkpoint/lock setup and is not advertised as a currently executable replay. Reconfirmation must extract the unchanged complete wrapper for each source case, supply explicit selected shell build/channel and preserve inner result/outer code/status/streams; no fresh run or Rust pass is asserted.
