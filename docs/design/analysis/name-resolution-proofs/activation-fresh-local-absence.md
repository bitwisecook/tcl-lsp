# naming.activation.fresh-local-absence

Kind: `native-observation`

## Problem statement

A root variable b is already PRESENT when f starts. An existence consumer could accidentally borrow that namespace cell for an unlinked procedure local b, or invent an array element Params(key) that was never created in the activation.

## Question

Does a fresh procedure activation report either unlinked b or never-created Params(key) present after root b was set?

## Conclusion

All six selected shell captures return 0 0, with process status0 and empty stderr. Root b does not make either queried procedure-local receiver present in this exact unlinked activation. This finite source observation does not establish arbitrary incoming frame state, compiler-local allocation, observers or an entered native variable object.

## Scope

One ASCII script sets ::b, defines zero-argument f, invokes it and prints two info-exists values. The exact source hash, selected executable hash, process status and embedded streams are retained. Full patchlevel/configuration and original shell channel are unrecorded. No raw NUL, opaque bytes, linked receiver, TclOO or BIG-IP run occurs.

## Provider answers

### tcl8.4

Status: `observed`. Version: Capture association 8.4; launched full patchlevel was not queried in this receipt.. Build: Recorded selected shell digest 551bef3a9b51f256f4ad9dec06bddc42ceab872a72528e83676d1d7df715b9d4; compiler, linked library/header digests and configure flags are unrecorded.. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded. Dialect: C Tcl.

Actual output is 0 0: unlinked local b and never-created Params(key) are both absent. Process status0 and empty stderr are retained independently of the two Boolean results.

### tcl8.5

Status: `observed`. Version: Capture association 8.5; launched full patchlevel was not queried in this receipt.. Build: Recorded selected shell digest 7a0fb4aa272a45662fb883295d0037a8c18e3e32c635736af8314fb835981935; compiler, linked library/header digests and configure flags are unrecorded.. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded. Dialect: C Tcl.

Actual output is 0 0: unlinked local b and never-created Params(key) are both absent. Process status0 and empty stderr are retained independently of the two Boolean results.

### tcl8.6

Status: `observed`. Version: Capture association 8.6; launched full patchlevel was not queried in this receipt.. Build: Recorded selected shell digest 0cc67113840d03c15c15d88188c16b8b019aa2a8c11ca6980f32723fbedacde0; compiler, linked library/header digests and configure flags are unrecorded.. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded. Dialect: C Tcl.

Actual output is 0 0: unlinked local b and never-created Params(key) are both absent. Process status0 and empty stderr are retained independently of the two Boolean results.

### tcl9.0

Status: `observed`. Version: Capture association 9.0; launched full patchlevel was not queried in this receipt.. Build: Recorded selected shell digest cc28851d04548dce27006a317d22b78c67e5491204f66f924c20d1f3febf6c18; compiler, linked library/header digests and configure flags are unrecorded.. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded. Dialect: C Tcl.

Actual output is 0 0: unlinked local b and never-created Params(key) are both absent. Process status0 and empty stderr are retained independently of the two Boolean results.

### tcl9.1

Status: `observed`. Version: Capture association 9.1; launched full patchlevel was not queried in this receipt.. Build: Recorded selected shell digest d4c2dbd11cd88994b7ca992eb87cdc855750f75d5d9f42b681b4dd4c321c791c; compiler, linked library/header digests and configure flags are unrecorded.. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded. Dialect: C Tcl.

Actual output is 0 0: unlinked local b and never-created Params(key) are both absent. Process status0 and empty stderr are retained independently of the two Boolean results.

### jim

Status: `observed`. Version: Capture association Jim0.84; launched full patchlevel was not queried in this receipt. Jim revision and UTF build configuration are unrecorded.. Build: Recorded selected shell digest d5b47eb75b271d50331ca5a38775492cb186f8feedd3c76bb2a0fbf1905671f0; compiler, linked library/header digests and configure flags are unrecorded.. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded. Dialect: Jim Tcl.

Actual output is 0 0: unlinked local b and never-created Params(key) are both absent. Process status0 and empty stderr are retained independently of the two Boolean results.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No appliance capture for this exact program is attached; C Tcl and Jim outcomes do not establish BIG-IP load or event behaviour.

## Exact evidence

- `source` (input): [rust/tcl-compiler/tests/data/native_activation_presence/source.tcl](../../../../rust/tcl-compiler/tests/data/native_activation_presence/source.tcl). SHA-256 `98c1c686dcc688818bc004f8a6ff0d6704a79219db83c8aaf473586c1b4db239`. Exact retained root assignment and two original procedure-local existence queries.
- `receipt` (provider): [rust/tcl-compiler/tests/data/native_activation_presence/manifest.json](../../../../rust/tcl-compiler/tests/data/native_activation_presence/manifest.json). SHA-256 `2ac19c5eedbe02f084e7d04ada23ef475681ecd9e964b4d4c80833d6007dd50b`. Six original shell/source/status/stream associations.
- `row-tcl8.4` (observation): [rust/tcl-compiler/tests/data/native_activation_presence/manifest.json](../../../../rust/tcl-compiler/tests/data/native_activation_presence/manifest.json). SHA-256 `2ac19c5eedbe02f084e7d04ada23ef475681ecd9e964b4d4c80833d6007dd50b`. JSON pointer `/rows/0`. Actual process0, stdout0 0 and empty stderr.
- `row-tcl8.5` (observation): [rust/tcl-compiler/tests/data/native_activation_presence/manifest.json](../../../../rust/tcl-compiler/tests/data/native_activation_presence/manifest.json). SHA-256 `2ac19c5eedbe02f084e7d04ada23ef475681ecd9e964b4d4c80833d6007dd50b`. JSON pointer `/rows/1`. Actual process0, stdout0 0 and empty stderr.
- `row-tcl8.6` (observation): [rust/tcl-compiler/tests/data/native_activation_presence/manifest.json](../../../../rust/tcl-compiler/tests/data/native_activation_presence/manifest.json). SHA-256 `2ac19c5eedbe02f084e7d04ada23ef475681ecd9e964b4d4c80833d6007dd50b`. JSON pointer `/rows/2`. Actual process0, stdout0 0 and empty stderr.
- `row-tcl9.0` (observation): [rust/tcl-compiler/tests/data/native_activation_presence/manifest.json](../../../../rust/tcl-compiler/tests/data/native_activation_presence/manifest.json). SHA-256 `2ac19c5eedbe02f084e7d04ada23ef475681ecd9e964b4d4c80833d6007dd50b`. JSON pointer `/rows/3`. Actual process0, stdout0 0 and empty stderr.
- `row-tcl9.1` (observation): [rust/tcl-compiler/tests/data/native_activation_presence/manifest.json](../../../../rust/tcl-compiler/tests/data/native_activation_presence/manifest.json). SHA-256 `2ac19c5eedbe02f084e7d04ada23ef475681ecd9e964b4d4c80833d6007dd50b`. JSON pointer `/rows/4`. Actual process0, stdout0 0 and empty stderr.
- `row-jim` (observation): [rust/tcl-compiler/tests/data/native_activation_presence/manifest.json](../../../../rust/tcl-compiler/tests/data/native_activation_presence/manifest.json). SHA-256 `2ac19c5eedbe02f084e7d04ada23ef475681ecd9e964b4d4c80833d6007dd50b`. JSON pointer `/rows/5`. Actual process0, stdout0 0 and empty stderr.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs](../../../../rust/tcl-compiler/src/analyser/diagnostics/dataflow.rs), `emit_existence_constant_branch_diagnostics`: Consumes independently retained frame existence facts rather than treating a root name as an unlinked local.
- [rust/tcl-compiler/src/analyser/diagnostics/tests.rs](../../../../rust/tcl-compiler/src/analyser/diagnostics/tests.rs), `analyser::diagnostics::tests::info_exists_fresh_activation_absence_does_not_borrow_namespace_contents` (linked): Binds the exact native source and checks I230 for the b predicate in an independently analysed source. This Rust assertion does not reproduce the Params element query or manufacture a native frame.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact script and original captured streams remain attached. This receipt does not retain the original invocation argument vector or whether the shell consumed a file or stdin. No exact original-channel replayer is claimed. A new run must record its selected shell/version/build, channel, exact input checksum, process status and separate streams; native disassembly addresses must not be mistaken for stable semantic coordinates. Retained executable digests do not reconstruct absent executables or establish the current provider. No new native run or Rust execution is claimed.
