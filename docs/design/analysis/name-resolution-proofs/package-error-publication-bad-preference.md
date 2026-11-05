# naming.package.error-publication-bad-preference

Kind: `native-observation`

## Problem statement

An invalid preference can fail because the prefer command door is absent or because its argument fails validation. The stage changes the diagnostic and errorCode, so consumers cannot borrow the later-release validation result.

## Question

What result and errorCode does the exact caught package prefer bad program publish?

## Conclusion

Tcl8.4 rejects the prefer subcommand and publishes NONE. The captured Tcl8.5/8.6/9.0/9.1 preference-validation rows publish TCL LOOKUP INDEX preference bad. No Jim preference row was captured.

## Scope

Only the exact script strings, seeded global errorCode, captured selected executable/header/library artifacts, result/exit/stderr rows in this retained manifest. Full linked release versions and launch channel were not queried here; no BIG-IP or unlisted provider branch is inferred.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Exact script value retained; launch channel not recorded in this manifest. Version: tcl8.4 recorded profile tcl8.4.

Exact script: "set ::errorCode SEED; catch {package prefer bad} m; list $m $::errorCode"

Captured result: {bad option "prefer": must be forget, ifneeded, names, present, provide, require, unknown, vcompare, versions, or vsatisfies} NONE

Original process exit: 0; stderr: "".

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Exact script value retained; launch channel not recorded in this manifest. Version: tcl8.5 recorded profile tcl8.5; tcl8.6 recorded profile tcl8.6; tcl9.0 recorded profile tcl9.0; tcl9.1 recorded profile tcl9.1.

Exact script: "set ::errorCode SEED; catch {package prefer bad} m; list $m $::errorCode"

Captured result: {bad preference "bad": must be latest or stable} {TCL LOOKUP INDEX preference bad}

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
