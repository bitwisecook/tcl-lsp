# naming.package.error-publication-bad-member

Kind: `native-observation`

## Problem statement

A prior global errorCode value is seeded before an invalid package subcommand. Preserving SEED, publishing NONE, and publishing a structured lookup error are competing outcomes that affect callback/error consumers.

## Question

What result and errorCode does the exact caught package bad program publish?

## Conclusion

The captured family-specific results replace SEED. Tcl8.4 and Jim publish NONE; Tcl8.5/8.6/9.0/9.1 publish TCL LOOKUP INDEX option bad, with release-specific command usage text. This is only the recorded program and provider artifacts.

## Scope

Only the exact script strings, seeded global errorCode, captured selected executable/header/library artifacts, result/exit/stderr rows in this retained manifest. Full linked release versions and launch channel were not queried here; no BIG-IP or unlisted provider branch is inferred.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Exact script value retained; launch channel not recorded in this manifest. Version: tcl8.4 recorded profile tcl8.4.

Exact script: "set ::errorCode SEED; catch {package bad} m; list $m $::errorCode"

Captured result: {bad option "bad": must be forget, ifneeded, names, present, provide, require, unknown, vcompare, versions, or vsatisfies} NONE

Original process exit: 0; stderr: "".

### tcl8.5, tcl8.6

Status: `observed`. Channel: Exact script value retained; launch channel not recorded in this manifest. Version: tcl8.5 recorded profile tcl8.5; tcl8.6 recorded profile tcl8.6.

Exact script: "set ::errorCode SEED; catch {package bad} m; list $m $::errorCode"

Captured result: {bad option "bad": must be forget, ifneeded, names, prefer, present, provide, require, unknown, vcompare, versions, or vsatisfies} {TCL LOOKUP INDEX option bad}

Original process exit: 0; stderr: "".

### tcl9.0, tcl9.1

Status: `observed`. Channel: Exact script value retained; launch channel not recorded in this manifest. Version: tcl9.0 recorded profile tcl9.0; tcl9.1 recorded profile tcl9.1.

Exact script: "set ::errorCode SEED; catch {package bad} m; list $m $::errorCode"

Captured result: {bad option "bad": must be files, forget, ifneeded, names, prefer, present, provide, require, unknown, vcompare, versions, or vsatisfies} {TCL LOOKUP INDEX option bad}

Original process exit: 0; stderr: "".

### jim

Status: `observed`. Channel: Exact script value retained; launch channel not recorded in this manifest. Version: jim recorded profile jim.

Exact script: "set ::errorCode SEED; catch {package bad} m; list $m $::errorCode"

Captured result: {package, unknown command "bad": should be forget, names, provide, require} NONE

Original process exit: 0; stderr: "".

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [rust/tcl-vm/src/cmd_package.rs](../../../../rust/tcl-vm/src/cmd_package.rs), `tests::package_error_publication_matches_all_21_original_native_controls` (linked): Compares this question's original source/result rows within the complete 21-window fixture; the independent native captures remain the language evidence. No executed Rust result is attached.

## Evidence

- [rust/tcl-registry/tests/data/native_package_error_publication/manifest.json](../../../../rust/tcl-registry/tests/data/native_package_error_publication/manifest.json)
