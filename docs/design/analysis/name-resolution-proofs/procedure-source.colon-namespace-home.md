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

### tcl8.4, tcl8.5, jim

Status: `observed`. Channel: Original complete UTF-8 ASCII wrapper fed to selected shell stdin; outer catch + binary-scan presenter. Version: tcl8.4 tcl8.4 (shell association; tcl8.5 tcl8.5 (shell association; jim jim (shell association.

Original stdout: '0 3a3a\n'; decoded outer result: '::'. Inner caught proc result is separate from outer presenter code0.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original complete UTF-8 ASCII wrapper fed to selected shell stdin; outer catch + binary-scan presenter. Version: tcl8.6 tcl8.6 (shell association; tcl9.0 tcl9.0 (shell association; tcl9.1 tcl9.1 (shell association.

Original stdout: '0 3a3a3a\n'; decoded outer result: ':::'. Inner caught proc result is separate from outer presenter code0.

### bigip

Status: `not-tested`.

No appliance execution of these procedure source controls is attached.

## Consumer bindings

- [runtime/rust/src/cmd_proc/native_name_tests.rs](../../../../runtime/rust/src/cmd_proc/native_name_tests.rs), `cmd_proc::native_name_tests::original_procedure_colon_names_match_all_18_native_home_and_error_controls` (linked): Compares the exact original source/case completion and result bytes; no unsampled native object getters or name/header allocation are granted.
- [rust/tcl-vm/src/command/native_procedure_name_tests.rs](../../../../rust/tcl-vm/src/command/native_procedure_name_tests.rs), `command::native_procedure_name_tests::original_procedure_colon_names_match_all_18_native_home_and_error_controls` (linked): Compares the exact original source/case completion and result bytes; no unsampled native object getters or name/header allocation are granted.

## Evidence

- [rust/tcl-syntax/tests/data/native_procedure_name/cases.json](../../../../rust/tcl-syntax/tests/data/native_procedure_name/cases.json)
- [rust/tcl-syntax/tests/data/native_procedure_name/manifest.json](../../../../rust/tcl-syntax/tests/data/native_procedure_name/manifest.json)
- [rust/tcl-syntax/tests/data/native_procedure_name/rows.txt](../../../../rust/tcl-syntax/tests/data/native_procedure_name/rows.txt)
