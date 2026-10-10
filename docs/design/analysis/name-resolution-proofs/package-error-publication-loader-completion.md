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

### tcl8.4

Status: `observed`. Version: Full linked version not queried in this capture; recorded profile tcl8.4. Build: Executable SHA-256 551bef3a9b51f256f4ad9dec06bddc42ceab872a72528e83676d1d7df715b9d4; original header/library provenance remains in this row.. Channel: Exact script value retained; launch channel not recorded in this manifest.. Dialect: Tcl.

Exact script: "set ::errorCode SEED; catch {package ifneeded probe 1 {return -code break CALLBACK}; package require probe} m; list $m $::errorCode"

Captured result: {attempt to provide package probe 1 failed: bad return code: 2} NONE

Original process exit: 0; stderr: "".

### tcl8.5

Status: `observed`. Version: Full linked version not queried in this capture; recorded profile tcl8.5. Build: Executable SHA-256 7a0fb4aa272a45662fb883295d0037a8c18e3e32c635736af8314fb835981935; original header/library provenance remains in this row.. Channel: Exact script value retained; launch channel not recorded in this manifest.. Dialect: Tcl.

Exact script: "set ::errorCode SEED; catch {package ifneeded probe 1 {return -code break CALLBACK}; package require probe} m; list $m $::errorCode"

Captured result: {attempt to provide package probe 1 failed: bad return code: 2} NONE

Original process exit: 0; stderr: "".

### tcl8.6

Status: `observed`. Version: Full linked version not queried in this capture; recorded profile tcl8.6. Build: Executable SHA-256 0cc67113840d03c15c15d88188c16b8b019aa2a8c11ca6980f32723fbedacde0; original header/library provenance remains in this row.. Channel: Exact script value retained; launch channel not recorded in this manifest.. Dialect: Tcl.

Exact script: "set ::errorCode SEED; catch {package ifneeded probe 1 {return -code break CALLBACK}; package require probe} m; list $m $::errorCode"

Captured result: {attempt to provide package probe 1 failed: bad return code: 2} {TCL PACKAGE BADRESULT}

Original process exit: 0; stderr: "".

### tcl9.0

Status: `observed`. Version: Full linked version not queried in this capture; recorded profile tcl9.0. Build: Executable SHA-256 cc28851d04548dce27006a317d22b78c67e5491204f66f924c20d1f3febf6c18; original header/library provenance remains in this row.. Channel: Exact script value retained; launch channel not recorded in this manifest.. Dialect: Tcl.

Exact script: "set ::errorCode SEED; catch {package ifneeded probe 1 {return -code break CALLBACK}; package require probe} m; list $m $::errorCode"

Captured result: {attempt to provide package probe 1 failed: bad return code: 2} {TCL PACKAGE BADRESULT}

Original process exit: 0; stderr: "".

### tcl9.1

Status: `observed`. Version: Full linked version not queried in this capture; recorded profile tcl9.1. Build: Executable SHA-256 d4c2dbd11cd88994b7ca992eb87cdc855750f75d5d9f42b681b4dd4c321c791c; original header/library provenance remains in this row.. Channel: Exact script value retained; launch channel not recorded in this manifest.. Dialect: Tcl.

Exact script: "set ::errorCode SEED; catch {package ifneeded probe 1 {return -code break CALLBACK}; package require probe} m; list $m $::errorCode"

Captured result: {attempt to provide package probe 1 failed: bad return code: 2} {TCL PACKAGE BADRESULT}

Original process exit: 0; stderr: "".

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `row-3` (observation): [rust/tcl-registry/tests/data/native_package_error_publication/manifest.json](../../../../rust/tcl-registry/tests/data/native_package_error_publication/manifest.json). SHA-256 `1df5ba7ea8159bebc91afd0858fecb198ea6083152b56e408a1cf2aca1a7a437`. JSON pointer `/observations/3`. Exact script, result, original process status/stderr/budget and retained binary/header/library provenance for this case.
- `row-7` (observation): [rust/tcl-registry/tests/data/native_package_error_publication/manifest.json](../../../../rust/tcl-registry/tests/data/native_package_error_publication/manifest.json). SHA-256 `1df5ba7ea8159bebc91afd0858fecb198ea6083152b56e408a1cf2aca1a7a437`. JSON pointer `/observations/7`. Exact script, result, original process status/stderr/budget and retained binary/header/library provenance for this case.
- `row-11` (observation): [rust/tcl-registry/tests/data/native_package_error_publication/manifest.json](../../../../rust/tcl-registry/tests/data/native_package_error_publication/manifest.json). SHA-256 `1df5ba7ea8159bebc91afd0858fecb198ea6083152b56e408a1cf2aca1a7a437`. JSON pointer `/observations/11`. Exact script, result, original process status/stderr/budget and retained binary/header/library provenance for this case.
- `row-15` (observation): [rust/tcl-registry/tests/data/native_package_error_publication/manifest.json](../../../../rust/tcl-registry/tests/data/native_package_error_publication/manifest.json). SHA-256 `1df5ba7ea8159bebc91afd0858fecb198ea6083152b56e408a1cf2aca1a7a437`. JSON pointer `/observations/15`. Exact script, result, original process status/stderr/budget and retained binary/header/library provenance for this case.
- `row-19` (observation): [rust/tcl-registry/tests/data/native_package_error_publication/manifest.json](../../../../rust/tcl-registry/tests/data/native_package_error_publication/manifest.json). SHA-256 `1df5ba7ea8159bebc91afd0858fecb198ea6083152b56e408a1cf2aca1a7a437`. JSON pointer `/observations/19`. Exact script, result, original process status/stderr/budget and retained binary/header/library provenance for this case.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-vm/src/cmd_package.rs](../../../../rust/tcl-vm/src/cmd_package.rs), `tests::package_error_publication_matches_all_21_original_native_controls` (linked): Compares this question's original source/result rows within the complete 21-window fixture; the independent native captures remain the language evidence. No executed Rust result is attached.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact program strings and original provider/build/result rows are retained. No permanent standalone runner is attached here, and no new native execution is claimed. Reconfirmation must retain actual selected versions, input channel and separate status/streams.
