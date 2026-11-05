# naming.package.error-publication-loader-completion

Kind: `native-observation`

## Problem statement

An ifneeded script returns an abrupt completion while require expects package provision. Treating it as a successful loader or borrowing unknown-callback completion would change wrapper error and errorCode publication.

## Question

What result and errorCode follow the exact ifneeded loader returning break during require?

## Conclusion

The five captured C families retain the loader-failure wrapper with bad return code2. Tcl8.4/8.5 publish NONE; Tcl8.6/9.0/9.1 publish TCL PACKAGE BADRESULT. No Jim loader row was captured and no successful package execution/provision is proved.

## Scope

Only the exact script strings, seeded global errorCode, captured selected executable/header/library artifacts, result/exit/stderr rows in this retained manifest. Full linked release versions and launch channel were not queried here; no BIG-IP or unlisted provider branch is inferred.

## Provider answers

### tcl8.4, tcl8.5

Status: `observed`. Channel: Exact script value retained; launch channel not recorded in this manifest. Version: tcl8.4 recorded profile tcl8.4; tcl8.5 recorded profile tcl8.5.

Exact script: "set ::errorCode SEED; catch {package ifneeded probe 1 {return -code break CALLBACK}; package require probe} m; list $m $::errorCode"

Captured result: {attempt to provide package probe 1 failed: bad return code: 2} NONE

Original process exit: 0; stderr: "".

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Exact script value retained; launch channel not recorded in this manifest. Version: tcl8.6 recorded profile tcl8.6; tcl9.0 recorded profile tcl9.0; tcl9.1 recorded profile tcl9.1.

Exact script: "set ::errorCode SEED; catch {package ifneeded probe 1 {return -code break CALLBACK}; package require probe} m; list $m $::errorCode"

Captured result: {attempt to provide package probe 1 failed: bad return code: 2} {TCL PACKAGE BADRESULT}

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
