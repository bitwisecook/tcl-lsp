# naming.procedure-source.name-before-malformed-formals

Kind: `native-observation`

## Problem statement

A leading-colon name and malformed formal list compete for the first creation error. Parsing formals before release-selected name validation would emit a different diagnostic.

## Question

Which caught creation error appears for proc : with unmatched-open-brace formal text under namespace :?

## Conclusion

C8.4/8.5 report the leading-colon name rejection; C8.6/9.0/9.1 report unmatched open brace in list; Jim reports argument with no name. Outer catch completion and process are0. No original formal/body object getter, callback or allocation state is sampled.

## Scope

Three exact ASCII source controls×six native shell associations; outer catch and binary-scan presenter are retained in run.py. Each shell/case original complete stdout/stderr/status/digest is attached, with an exact separate rows projection. Shell labels are recorded but full patchlevel/source revision/configure identities were not queried. Script versus native object-vector semantics and inner versus outer catch completion remain separate; BIG-IP not tested.

## Provider answers

### tcl8.4, tcl8.5

Status: `observed`. Channel: Original complete UTF-8 ASCII wrapper fed to selected shell stdin; outer catch + binary-scan presenter. Version: tcl8.4 tcl8.4 (shell association; tcl8.5 tcl8.5 (shell association.

Original stdout: '0 31207b63616e2774206372656174652070726f63656475726520223a2220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a227d\n'; decoded outer result: '1 {can\'t create procedure ":" in non-global namespace with name starting with ":"}'. Inner caught proc result is separate from outer presenter code0.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original complete UTF-8 ASCII wrapper fed to selected shell stdin; outer catch + binary-scan presenter. Version: tcl8.6 tcl8.6 (shell association; tcl9.0 tcl9.0 (shell association; tcl9.1 tcl9.1 (shell association.

Original stdout: '0 31207b756e6d617463686564206f70656e20627261636520696e206c6973747d\n'; decoded outer result: '1 {unmatched open brace in list}'. Inner caught proc result is separate from outer presenter code0.

### jim

Status: `observed`. Channel: Original complete UTF-8 ASCII wrapper fed to selected shell stdin; outer catch + binary-scan presenter. Version: jim jim (shell association.

Original stdout: '0 31207b617267756d656e742077697468206e6f206e616d657d\n'; decoded outer result: '1 {argument with no name}'. Inner caught proc result is separate from outer presenter code0.

### bigip

Status: `not-tested`.

No appliance execution of these procedure source controls is attached.

## Consumer bindings

- [runtime/rust/src/cmd_proc/native_name_tests.rs](../../../../runtime/rust/src/cmd_proc/native_name_tests.rs), `cmd_proc::native_name_tests::original_procedure_colon_names_match_all_18_native_home_and_error_controls` (linked): Compares the exact original source/case completion and result bytes; no unsampled native object getters or name/header allocation are granted.
- [rust/tcl-vm/src/command/native_procedure_name_tests.rs](../../../../rust/tcl-vm/src/command/native_procedure_name_tests.rs), `command::native_procedure_name_tests::original_procedure_colon_names_match_all_18_native_home_and_error_controls` (linked): Compares the exact original source/case completion and result bytes; no unsampled native object getters or name/header allocation are granted.

## Evidence

- [rust/tcl-syntax/tests/data/native_procedure_name/cases.json](../../../../rust/tcl-syntax/tests/data/native_procedure_name/cases.json)
- [rust/tcl-syntax/tests/data/native_procedure_name/run.py](../../../../rust/tcl-syntax/tests/data/native_procedure_name/run.py)
- [rust/tcl-syntax/tests/data/native_procedure_name/manifest.json](../../../../rust/tcl-syntax/tests/data/native_procedure_name/manifest.json)
- [rust/tcl-syntax/tests/data/native_procedure_name/rows.txt](../../../../rust/tcl-syntax/tests/data/native_procedure_name/rows.txt)
