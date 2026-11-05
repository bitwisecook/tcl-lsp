# naming.procedure-source.colon-local-name

Kind: `native-observation`

## Problem statement

The local procedure name : under namespace : can fail before installation on older C while succeeding with a different creation result on Jim. A single colon-string policy would erase both release and producer-purpose differences.

## Question

What caught procedure creation code/result follows proc : {} under the original namespace :?

## Conclusion

C8.4/8.5 return inner code1 and the recorded non-global leading-colon name error. C8.6/9.0/9.1 return inner0 with empty result; Jim returns inner0 with :. The outer presenter completion/process remain0; the new procedure body is not invoked.

## Scope

Three exact ASCII source controls×six native shell associations; outer catch and binary-scan presenter are retained in run.py. Each shell/case original complete stdout/stderr/status/digest is attached, with an exact separate rows projection. Shell labels are recorded but full patchlevel/source revision/configure identities were not queried. Script versus native object-vector semantics and inner versus outer catch completion remain separate; BIG-IP not tested.

## Provider answers

### tcl8.4, tcl8.5

Status: `observed`. Channel: Original complete UTF-8 ASCII wrapper fed to selected shell stdin; outer catch + binary-scan presenter. Version: tcl8.4 tcl8.4 (shell association; tcl8.5 tcl8.5 (shell association.

Original stdout: '0 31207b63616e2774206372656174652070726f63656475726520223a2220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a227d\n'; decoded outer result: '1 {can\'t create procedure ":" in non-global namespace with name starting with ":"}'. Inner caught proc result is separate from outer presenter code0.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original complete UTF-8 ASCII wrapper fed to selected shell stdin; outer catch + binary-scan presenter. Version: tcl8.6 tcl8.6 (shell association; tcl9.0 tcl9.0 (shell association; tcl9.1 tcl9.1 (shell association.

Original stdout: '0 30207b7d\n'; decoded outer result: '0 {}'. Inner caught proc result is separate from outer presenter code0.

### jim

Status: `observed`. Channel: Original complete UTF-8 ASCII wrapper fed to selected shell stdin; outer catch + binary-scan presenter. Version: jim jim (shell association.

Original stdout: '0 30203a\n'; decoded outer result: '0 :'. Inner caught proc result is separate from outer presenter code0.

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
