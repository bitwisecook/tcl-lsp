# naming.variable.scope-root-qualified-declaration

Kind: `native-observation`

## Problem statement

At root, variable ::a::x can target the qualified namespace cell or a local tail x depending on the handler. Treating a rooted declaration as an ordinary set receiver would mispublish the path constant.

## Question

Which of root x and ::a::x exists after variable ::a::x /TOP at root?

## Conclusion

C8.4–9.1 print flat:0 and target:1; Jim prints flat:1 and target:0. The exact root-qualified variable declaration targets the qualified cell in these C captures and the local tail in Jim. No argument/formal installation, linked array or physical compiler-local claim follows.

## Scope

One exact ASCII root-qualified-variable.tcl source across six selected shells. Source/output/executable digests, separate process/embedded streams and matching output files are retained. The manifest does not retain the shell argument vector, exact launched patchlevel, Jim revision, compiler or native library/header/configuration closure. Printed labels are actual source values/Boolean queries, not physical header/cell allocation observations. No NUL/opaque input, TclOO or BIG-IP capture.

## Provider answers

### tcl8.4

Status: `observed`. Version: Capture association tcl8.4; launched full patchlevel was not queried in this receipt.. Build: Recorded selected shell digest 551bef3a9b51f256f4ad9dec06bddc42ceab872a72528e83676d1d7df715b9d4; compiler, linked library/header digests and configure flags are unrecorded.. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded. Dialect: C Tcl.

Actual selected output:

```text
flat:0
target:1
```

Process status0 and empty stderr are recorded for this exact program.

### tcl8.5

Status: `observed`. Version: Capture association tcl8.5; launched full patchlevel was not queried in this receipt.. Build: Recorded selected shell digest 7a0fb4aa272a45662fb883295d0037a8c18e3e32c635736af8314fb835981935; compiler, linked library/header digests and configure flags are unrecorded.. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded. Dialect: C Tcl.

Actual selected output:

```text
flat:0
target:1
```

Process status0 and empty stderr are recorded for this exact program.

### tcl8.6

Status: `observed`. Version: Capture association tcl8.6; launched full patchlevel was not queried in this receipt.. Build: Recorded selected shell digest 0cc67113840d03c15c15d88188c16b8b019aa2a8c11ca6980f32723fbedacde0; compiler, linked library/header digests and configure flags are unrecorded.. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded. Dialect: C Tcl.

Actual selected output:

```text
flat:0
target:1
```

Process status0 and empty stderr are recorded for this exact program.

### tcl9.0

Status: `observed`. Version: Capture association tcl9.0; launched full patchlevel was not queried in this receipt.. Build: Recorded selected shell digest cc28851d04548dce27006a317d22b78c67e5491204f66f924c20d1f3febf6c18; compiler, linked library/header digests and configure flags are unrecorded.. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded. Dialect: C Tcl.

Actual selected output:

```text
flat:0
target:1
```

Process status0 and empty stderr are recorded for this exact program.

### tcl9.1

Status: `observed`. Version: Capture association tcl9.1; launched full patchlevel was not queried in this receipt.. Build: Recorded selected shell digest d4c2dbd11cd88994b7ca992eb87cdc855750f75d5d9f42b681b4dd4c321c791c; compiler, linked library/header digests and configure flags are unrecorded.. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded. Dialect: C Tcl.

Actual selected output:

```text
flat:0
target:1
```

Process status0 and empty stderr are recorded for this exact program.

### jim

Status: `observed`. Version: Capture association jim; launched full patchlevel was not queried in this receipt. Jim revision and UTF build configuration are unrecorded.. Build: Recorded selected shell digest d5b47eb75b271d50331ca5a38775492cb186f8feedd3c76bb2a0fbf1905671f0; compiler, linked library/header digests and configure flags are unrecorded.. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded. Dialect: Jim Tcl.

Actual selected output:

```text
flat:1
target:0
```

Process status0 and empty stderr are recorded for this exact program.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No appliance capture for this exact program is attached; C Tcl and Jim outcomes do not establish BIG-IP load or event behaviour.

## Exact evidence

- `source` (input): [rust/tcl-compiler/tests/data/native_path_constant_scopes/root-qualified-variable.tcl](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/root-qualified-variable.tcl). SHA-256 `4d21af32c2d3ca0bc3ff2a712f1f4c038ab737ad0e75515011782e2fe64a165b`. Exact selected scope/path source; matched original complete SHA.
- `receipt` (provider): [rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json). SHA-256 `a8716f94efaaab63bffbd739f07526e8800699e6b6c07efaec36d11481789e51`. Full30-row five-program/six-shell source/output/executable/status association; selected question retains its own six rows.
- `row-tcl8.4` (observation): [rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json). SHA-256 `a8716f94efaaab63bffbd739f07526e8800699e6b6c07efaec36d11481789e51`. JSON pointer `/rows/6`. Selected original program/provider/process/stream row.
- `stdout-tcl8.4` (observation): [rust/tcl-compiler/tests/data/native_path_constant_scopes/root-qualified-variable.tcl8.4.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/root-qualified-variable.tcl8.4.txt). SHA-256 `d3f74aa0d86ac9770f6182f0120696197993a88c220b6597bb471fe8e0a85dc0`. Exact original stdout file matched to the receipt full checksum and embedded text.
- `row-tcl8.5` (observation): [rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json). SHA-256 `a8716f94efaaab63bffbd739f07526e8800699e6b6c07efaec36d11481789e51`. JSON pointer `/rows/7`. Selected original program/provider/process/stream row.
- `stdout-tcl8.5` (observation): [rust/tcl-compiler/tests/data/native_path_constant_scopes/root-qualified-variable.tcl8.5.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/root-qualified-variable.tcl8.5.txt). SHA-256 `d3f74aa0d86ac9770f6182f0120696197993a88c220b6597bb471fe8e0a85dc0`. Exact original stdout file matched to the receipt full checksum and embedded text.
- `row-tcl8.6` (observation): [rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json). SHA-256 `a8716f94efaaab63bffbd739f07526e8800699e6b6c07efaec36d11481789e51`. JSON pointer `/rows/8`. Selected original program/provider/process/stream row.
- `stdout-tcl8.6` (observation): [rust/tcl-compiler/tests/data/native_path_constant_scopes/root-qualified-variable.tcl8.6.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/root-qualified-variable.tcl8.6.txt). SHA-256 `d3f74aa0d86ac9770f6182f0120696197993a88c220b6597bb471fe8e0a85dc0`. Exact original stdout file matched to the receipt full checksum and embedded text.
- `row-tcl9.0` (observation): [rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json). SHA-256 `a8716f94efaaab63bffbd739f07526e8800699e6b6c07efaec36d11481789e51`. JSON pointer `/rows/9`. Selected original program/provider/process/stream row.
- `stdout-tcl9.0` (observation): [rust/tcl-compiler/tests/data/native_path_constant_scopes/root-qualified-variable.tcl9.0.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/root-qualified-variable.tcl9.0.txt). SHA-256 `d3f74aa0d86ac9770f6182f0120696197993a88c220b6597bb471fe8e0a85dc0`. Exact original stdout file matched to the receipt full checksum and embedded text.
- `row-tcl9.1` (observation): [rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json). SHA-256 `a8716f94efaaab63bffbd739f07526e8800699e6b6c07efaec36d11481789e51`. JSON pointer `/rows/10`. Selected original program/provider/process/stream row.
- `stdout-tcl9.1` (observation): [rust/tcl-compiler/tests/data/native_path_constant_scopes/root-qualified-variable.tcl9.1.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/root-qualified-variable.tcl9.1.txt). SHA-256 `d3f74aa0d86ac9770f6182f0120696197993a88c220b6597bb471fe8e0a85dc0`. Exact original stdout file matched to the receipt full checksum and embedded text.
- `row-jim` (observation): [rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/manifest.json). SHA-256 `a8716f94efaaab63bffbd739f07526e8800699e6b6c07efaec36d11481789e51`. JSON pointer `/rows/11`. Selected original program/provider/process/stream row.
- `stdout-jim` (observation): [rust/tcl-compiler/tests/data/native_path_constant_scopes/root-qualified-variable.jim.txt](../../../../rust/tcl-compiler/tests/data/native_path_constant_scopes/root-qualified-variable.jim.txt). SHA-256 `8e64b238d7deca4f50bb5e3c75409c61b31dc114a73c3a5481c6850165f206a0`. Exact original stdout file matched to the receipt full checksum and embedded text.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/auto_path_eval/path_constants.rs](../../../../rust/tcl-compiler/src/auto_path_eval/path_constants.rs), `constant_path_assignments`: Produces dialect-selected source path assignments under their source scope and explicit receiver rules.
- [rust/tcl-compiler/src/auto_path_eval/path_constants.rs](../../../../rust/tcl-compiler/src/auto_path_eval/path_constants.rs), `FoldedPathConstants::lookup_at`: Queries the position-selected source abstraction; its optional path value does not grant native storage or entered execution.
- [rust/tcl-compiler/src/auto_path_eval/path_constants.rs](../../../../rust/tcl-compiler/src/auto_path_eval/path_constants.rs), `auto_path_eval::path_constants::tests::original_native_jim_qualified_variable_at_root_writes_only_local_tail` (linked): Checks /TOP lookup at x versus ::a::x under the independently selected constant-path source projection.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact script and original captured streams remain attached. This receipt does not retain the original invocation argument vector or whether the shell consumed a file or stdin. No exact original-channel replayer is claimed. A new run must record its selected shell/version/build, channel, exact input checksum, process status and separate streams; native disassembly addresses must not be mistaken for stable semantic coordinates. Retained executable digests do not reconstruct absent executables or establish the current provider. No new native run or Rust execution is claimed.
