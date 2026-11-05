# naming.package.error-publication-unknown-completion

Kind: `native-observation`

## Problem statement

A package unknown callback returns break rather than normal completion. The package consumer may reject that callback and overwrite an existing errorCode in a release-specific way. Callback completion is separate from package publication.

## Question

What result and errorCode follow the exact unknown callback returning break?

## Conclusion

All five captured C families reject the callback with bad return code3. Tcl8.4/8.5 publish NONE; Tcl8.6/9.0/9.1 publish TCL PACKAGE BADRESULT. No Jim callback row was captured, and this does not certify a source loader or normal callback receipt.

## Scope

Only the exact script strings, seeded global errorCode, captured selected executable/header/library artifacts, result/exit/stderr rows in this retained manifest. Full linked release versions and launch channel were not queried here; no BIG-IP or unlisted provider branch is inferred.

## Provider answers

### tcl8.4, tcl8.5

Status: `observed`. Channel: Exact script value retained; launch channel not recorded in this manifest. Version: tcl8.4 recorded profile tcl8.4; tcl8.5 recorded profile tcl8.5.

Exact script: "set ::errorCode SEED; catch {proc discover args {return -code break CALLBACK}; package unknown discover; package require missing} m; list $m $::errorCode"

Captured result: {bad return code: 3} NONE

Original process exit: 0; stderr: "".

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Exact script value retained; launch channel not recorded in this manifest. Version: tcl8.6 recorded profile tcl8.6; tcl9.0 recorded profile tcl9.0; tcl9.1 recorded profile tcl9.1.

Exact script: "set ::errorCode SEED; catch {proc discover args {return -code break CALLBACK}; package unknown discover; package require missing} m; list $m $::errorCode"

Captured result: {bad return code: 3} {TCL PACKAGE BADRESULT}

Original process exit: 0; stderr: "".

### jim

Status: `not-tested`.

No observation for this question.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [rust/tcl-vm/src/cmd_package.rs](../../../../rust/tcl-vm/src/cmd_package.rs), `tests::package_error_publication_matches_all_21_original_native_controls` (linked): Compares this question's original source/result rows within the complete 21-window fixture; the independent native captures remain the language evidence. No executed Rust result is attached.

## Evidence

- [rust/tcl-registry/tests/data/native_package_error_publication/manifest.json](../../../../rust/tcl-registry/tests/data/native_package_error_publication/manifest.json)
