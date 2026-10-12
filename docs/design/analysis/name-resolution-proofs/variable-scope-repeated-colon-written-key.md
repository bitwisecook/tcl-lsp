# naming.variable.scope-repeated-colon-written-key

Kind: `native-observation`

## Problem statement

Writing a:::raw and reading it back succeeds, but testing a::raw can select a different simple key in a dialect with different separator parsing. Source path advice must preserve the selected home/name rule.

## Question

Does the actual a:::raw assignment also make a::raw exist?

## Conclusion

All C captures print written:/ONE and short:1; Jim prints written:/ONE and short:0. The written key and shorter receiver are equivalent for these C controls and distinct for Jim. This finite source program does not measure original binary name objects or a private variable-table allocation.

## Scope

One exact ASCII raw-qualified-key.tcl source across six selected shells. Source/output/executable digests, separate process/embedded streams and matching output files are retained. The manifest does not retain the shell argument vector, exact launched patchlevel, Jim revision, compiler or native library/header/configuration closure. Printed labels are actual source values/Boolean queries, not physical header/cell allocation observations. No NUL/opaque input, TclOO or BIG-IP capture.

## Provider answers

### tcl8.4

Status: `observed`. Version: Capture association tcl8.4; launched full patchlevel was not queried in this receipt.. Build: Recorded selected shell digest 551bef3a9b51f256f4ad9dec06bddc42ceab872a72528e83676d1d7df715b9d4; compiler, linked library/header digests and configure flags are unrecorded.. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded. Dialect: C Tcl.

Actual selected output:

```text
written:/ONE
short:1
```

Process status0 and empty stderr are recorded for this exact program.

### tcl8.5

Status: `observed`. Version: Capture association tcl8.5; launched full patchlevel was not queried in this receipt.. Build: Recorded selected shell digest 7a0fb4aa272a45662fb883295d0037a8c18e3e32c635736af8314fb835981935; compiler, linked library/header digests and configure flags are unrecorded.. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded. Dialect: C Tcl.

Actual selected output:

```text
written:/ONE
short:1
```

Process status0 and empty stderr are recorded for this exact program.

### tcl8.6

Status: `observed`. Version: Capture association tcl8.6; launched full patchlevel was not queried in this receipt.. Build: Recorded selected shell digest 0cc67113840d03c15c15d88188c16b8b019aa2a8c11ca6980f32723fbedacde0; compiler, linked library/header digests and configure flags are unrecorded.. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded. Dialect: C Tcl.

Actual selected output:

```text
written:/ONE
short:1
```

Process status0 and empty stderr are recorded for this exact program.

### tcl9.0

Status: `observed`. Version: Capture association tcl9.0; launched full patchlevel was not queried in this receipt.. Build: Recorded selected shell digest cc28851d04548dce27006a317d22b78c67e5491204f66f924c20d1f3febf6c18; compiler, linked library/header digests and configure flags are unrecorded.. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded. Dialect: C Tcl.

Actual selected output:

```text
written:/ONE
short:1
```

Process status0 and empty stderr are recorded for this exact program.

### tcl9.1

Status: `observed`. Version: Capture association tcl9.1; launched full patchlevel was not queried in this receipt.. Build: Recorded selected shell digest d4c2dbd11cd88994b7ca992eb87cdc855750f75d5d9f42b681b4dd4c321c791c; compiler, linked library/header digests and configure flags are unrecorded.. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded. Dialect: C Tcl.

Actual selected output:

```text
written:/ONE
short:1
```

Process status0 and empty stderr are recorded for this exact program.

### jim

Status: `observed`. Version: Capture association jim; launched full patchlevel was not queried in this receipt. Jim revision and UTF build configuration are unrecorded.. Build: Recorded selected shell digest d5b47eb75b271d50331ca5a38775492cb186f8feedd3c76bb2a0fbf1905671f0; compiler, linked library/header digests and configure flags are unrecorded.. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded. Dialect: Jim Tcl.

Actual selected output:

```text
written:/ONE
short:0
```

Process status0 and empty stderr are recorded for this exact program.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No appliance capture for this exact program is attached; C Tcl and Jim outcomes do not establish BIG-IP load or event behaviour.

## Exact evidence

- `source` (input): [rust/tcl-compiler/tests/data/native_path_constant_scopes/raw-qualified-key.tcl](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/raw-qualified-key.tcl). SHA-256 `59aeec8a4566c5ec4442598fc3ec00b76e76d77a0d6baa4c9fd6b61f5e4bba06`. Exact selected scope/path source; matched original complete SHA.
- `receipt` (provider): [rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json). SHA-256 `a8716f94efaaab63bffbd739f07526e8800699e6b6c07efaec36d11481789e51`. Full30-row five-program/six-shell source/output/executable/status association; selected question retains its own six rows.
- `row-tcl8.4` (observation): [rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json). SHA-256 `a8716f94efaaab63bffbd739f07526e8800699e6b6c07efaec36d11481789e51`. JSON pointer `/rows/24`. Selected original program/provider/process/stream row.
- `stdout-tcl8.4` (observation): [rust/tcl-compiler/tests/data/native_path_constant_scopes/raw-qualified-key.tcl8.4.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/raw-qualified-key.tcl8.4.txt). SHA-256 `9534209e4a78421eab6eadac402c1f65aa3f334615c88a94ce59179651d09108`. Exact original stdout file matched to the receipt full checksum and embedded text.
- `row-tcl8.5` (observation): [rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json). SHA-256 `a8716f94efaaab63bffbd739f07526e8800699e6b6c07efaec36d11481789e51`. JSON pointer `/rows/25`. Selected original program/provider/process/stream row.
- `stdout-tcl8.5` (observation): [rust/tcl-compiler/tests/data/native_path_constant_scopes/raw-qualified-key.tcl8.5.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/raw-qualified-key.tcl8.5.txt). SHA-256 `9534209e4a78421eab6eadac402c1f65aa3f334615c88a94ce59179651d09108`. Exact original stdout file matched to the receipt full checksum and embedded text.
- `row-tcl8.6` (observation): [rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json). SHA-256 `a8716f94efaaab63bffbd739f07526e8800699e6b6c07efaec36d11481789e51`. JSON pointer `/rows/26`. Selected original program/provider/process/stream row.
- `stdout-tcl8.6` (observation): [rust/tcl-compiler/tests/data/native_path_constant_scopes/raw-qualified-key.tcl8.6.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/raw-qualified-key.tcl8.6.txt). SHA-256 `9534209e4a78421eab6eadac402c1f65aa3f334615c88a94ce59179651d09108`. Exact original stdout file matched to the receipt full checksum and embedded text.
- `row-tcl9.0` (observation): [rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json). SHA-256 `a8716f94efaaab63bffbd739f07526e8800699e6b6c07efaec36d11481789e51`. JSON pointer `/rows/27`. Selected original program/provider/process/stream row.
- `stdout-tcl9.0` (observation): [rust/tcl-compiler/tests/data/native_path_constant_scopes/raw-qualified-key.tcl9.0.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/raw-qualified-key.tcl9.0.txt). SHA-256 `9534209e4a78421eab6eadac402c1f65aa3f334615c88a94ce59179651d09108`. Exact original stdout file matched to the receipt full checksum and embedded text.
- `row-tcl9.1` (observation): [rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json). SHA-256 `a8716f94efaaab63bffbd739f07526e8800699e6b6c07efaec36d11481789e51`. JSON pointer `/rows/28`. Selected original program/provider/process/stream row.
- `stdout-tcl9.1` (observation): [rust/tcl-compiler/tests/data/native_path_constant_scopes/raw-qualified-key.tcl9.1.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/raw-qualified-key.tcl9.1.txt). SHA-256 `9534209e4a78421eab6eadac402c1f65aa3f334615c88a94ce59179651d09108`. Exact original stdout file matched to the receipt full checksum and embedded text.
- `row-jim` (observation): [rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json). SHA-256 `a8716f94efaaab63bffbd739f07526e8800699e6b6c07efaec36d11481789e51`. JSON pointer `/rows/29`. Selected original program/provider/process/stream row.
- `stdout-jim` (observation): [rust/tcl-compiler/tests/data/native_path_constant_scopes/raw-qualified-key.jim.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/raw-qualified-key.jim.txt). SHA-256 `3ef909a71a2f7362dc0c23d6f86909176225c6a3a3baa0f560f4a62790d7fe51`. Exact original stdout file matched to the receipt full checksum and embedded text.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/auto_path_eval/path_constants.rs](../../../../rust/tcl-compiler/src/auto_path_eval/path_constants.rs), `constant_path_assignments`: Produces dialect-selected source path assignments under their source scope and explicit receiver rules.
- [rust/tcl-compiler/src/auto_path_eval/path_constants.rs](../../../../rust/tcl-compiler/src/auto_path_eval/path_constants.rs), `FoldedPathConstants::lookup_at`: Queries the position-selected source abstraction; its optional path value does not grant native storage or entered execution.
- [rust/tcl-compiler/src/auto_path_eval/path_constants.rs](../../../../rust/tcl-compiler/src/auto_path_eval/path_constants.rs), `auto_path_eval::path_constants::tests::original_native_scope_names_keep_six_engine_colon_and_home_rules` (linked): Checks exact written a:::raw versus a::raw presence without manufacturing an original native receiver.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact script and original captured streams remain attached. This receipt does not retain the original invocation argument vector or whether the shell consumed a file or stdin. No exact original-channel replayer is claimed. A new run must record its selected shell/version/build, channel, exact input checksum, process status and separate streams; native disassembly addresses must not be mistaken for stable semantic coordinates. Retained executable digests do not reconstruct absent executables or establish the current provider. No new native run or Rust execution is claimed.
