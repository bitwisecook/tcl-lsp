# naming.command-trace.counted-zero-callback-evaluation

Kind: `native-observation`

## Problem statement

A copied trace prefix with raw-zero suffix can report the same short prefix across releases while its callback evaluation receives different arguments. Prefix reporting alone cannot explain what a reached callback evaluates.

## Question

What callback log follows invoking the original traced command with a raw-zero callback prefix on each C release?

## Conclusion

C8.4/8.5 record two empty callback observations, rendered {} {}; C8.6/9.0/9.1 record enter leave. The exact programs retain the same registration/report/removal outputs, so the reached callback-result difference is a separate release axis.

## Scope

Five C release associations with original Tcl_EvalEx counted source. The Syntax opaque driver initializes Tcl and forwards the file bytes; the Runtime result presenter uses a fresh uninitialized interpreter, evaluates each exact encoded source and prints code plus counted result hex. The independently retained programs differ in presentation and initial setup and are not asserted byte-identical. Raw FF/zero occur in their retained inputs. No original object-header, trace internal storage allocation, Jim or BIG-IP behavior is inferred.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (Runtime capture association); Syntax8.4 only; full launched patchlevels unqueried. Build: Runtime original observer SHA256 03bf2af50be51d7a9c7760a01c6c25bba0519c8bff147ba4754bc346a6d2e0c4 and archive SHA256 d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011. Separate counted-zero Syntax driver/header/archive identity unrecorded. Compiler/configure and full launched patchlevel unqueried.. Channel: Original counted Tcl_EvalEx source; separate direct stdout versus code/count-result-hex presenter.. Dialect: C Tcl.

Direct-output stream:

```text
6362ff
0
6362ff
0
{} {}
```

Independent presenter stdout:

```text
0	3633363266662030203633363266662030207b7b7d207b7d7d
```
Original programs and observer assumptions remain separate.

### tcl8.5

Status: `observed`. Version: 8.5.19 (Runtime capture association); Syntax8.5 only; full launched patchlevels unqueried. Build: Runtime original observer SHA256 69cb3b0b02418fe33d414cc0e89d424aed4dd35e8dfa86f36d3f22b6cd64b1df and archive SHA256 99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df. Separate counted-zero Syntax driver/header/archive identity unrecorded. Compiler/configure and full launched patchlevel unqueried.. Channel: Original counted Tcl_EvalEx source; separate direct stdout versus code/count-result-hex presenter.. Dialect: C Tcl.

Direct-output stream:

```text
6362ff
0
6362ff
0
{} {}
```

Independent presenter stdout:

```text
0	3633363266662030203633363266662030207b7b7d207b7d7d
```
Original programs and observer assumptions remain separate.

### tcl8.6

Status: `observed`. Version: 8.6.18 (Runtime capture association); Syntax8.6 only; full launched patchlevels unqueried. Build: Runtime original observer SHA256 95e72b2b226a5bc2da91f415588f86b00e58bb4e3a32d18b90faf6841f99e4e4 and archive SHA256 a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6. Separate counted-zero Syntax driver/header/archive identity unrecorded. Compiler/configure and full launched patchlevel unqueried.. Channel: Original counted Tcl_EvalEx source; separate direct stdout versus code/count-result-hex presenter.. Dialect: C Tcl.

Direct-output stream:

```text
6362ff
0
6362ff
0
enter leave
```

Independent presenter stdout:

```text
0	3633363266662030203633363266662030207b656e746572206c656176657d
```
Original programs and observer assumptions remain separate.

### tcl9.0

Status: `observed`. Version: 9.0.4 (Runtime capture association); Syntax9.0 only; full launched patchlevels unqueried. Build: Runtime original observer SHA256 7da959c6f9f4c9c04cf113fac0846a41d9a70f353584451bab9b20537be36a22 and archive SHA256 2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04. Separate counted-zero Syntax driver/header/archive identity unrecorded. Compiler/configure and full launched patchlevel unqueried.. Channel: Original counted Tcl_EvalEx source; separate direct stdout versus code/count-result-hex presenter.. Dialect: C Tcl.

Direct-output stream:

```text
6362ff
0
6362ff
0
enter leave
```

Independent presenter stdout:

```text
0	3633363266662030203633363266662030207b656e746572206c656176657d
```
Original programs and observer assumptions remain separate.

### tcl9.1

Status: `observed`. Version: 9.1.0 (Runtime capture association); Syntax9.1 only; full launched patchlevels unqueried. Build: Runtime original observer SHA256 dbfe9edd5b7611d49a9c2f3da50fe418ee8a2c94bb0dfc057b406a15bbcaa8ef and archive SHA256 8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33. Separate counted-zero Syntax driver/header/archive identity unrecorded. Compiler/configure and full launched patchlevel unqueried.. Channel: Original counted Tcl_EvalEx source; separate direct stdout versus code/count-result-hex presenter.. Dialect: C Tcl.

Direct-output stream:

```text
6362ff
0
6362ff
0
enter leave
```

Independent presenter stdout:

```text
0	3633363266662030203633363266662030207b656e746572206c656176657d
```
Original programs and observer assumptions remain separate.

### jim

Status: `not-tested`. Version: not measured. Build: not measured. Channel: not tested. Dialect: Jim Tcl.

No original command/execution trace program capture for this provider is attached.

### bigip

Status: `not-tested`. Version: not measured. Build: not measured. Channel: not tested. Dialect: F5 iRules.

No original command/execution trace program capture for this provider is attached.

## Exact evidence

- `runtime-receipt` (provider): [runtime/rust/tests/data/native_command_traces/provenance.json](../../../../runtime/rust/tests/data/native_command_traces/provenance.json). SHA-256 `5eae5fcd8bf8025aba2f96b43de5502ad08924c8540f2bb6fc2a97d5519f445d`. Independent original observer/build/result associations.
- `runtime-windows` (observation): [runtime/rust/tests/data/native_command_traces/windows.tsv](../../../../runtime/rust/tests/data/native_command_traces/windows.tsv). SHA-256 `960870cfc8d3b0588c51f130c407ead4a052f900cd41cf0aedcea3343b0a8ec2`. Exact source hex, guest code and original result hex for both programs.
- `syntax-input` (input): [rust/tcl-syntax/tests/data/native_command_trace/counted_nul.tcl](../../../../rust/tcl-syntax/tests/data/native_command_trace/counted_nul.tcl). SHA-256 `70d3477e2958679fe3bf1b329e5ee84d70a0a39c197c3a02c75b5d7253d397e3`. Exact independently retained direct-output source bytes.
- `syntax-receipt` (provider): [rust/tcl-syntax/tests/data/native_command_trace/counted-nul-provenance.json](../../../../rust/tcl-syntax/tests/data/native_command_trace/counted-nul-provenance.json). SHA-256 `29005d8e9a718552b73eea82391c76231c3600bfaf4b8a8af4c0ffcbe22c9806`. Separate original direct-output associations.
- `runtime-tcl8.4` (observation): [runtime/rust/tests/data/native_command_traces/provenance.json](../../../../runtime/rust/tests/data/native_command_traces/provenance.json). SHA-256 `5eae5fcd8bf8025aba2f96b43de5502ad08924c8540f2bb6fc2a97d5519f445d`. JSON pointer `/1`. Exact result-presenter guest result and original observer association.
- `syntax-tcl8.4` (observation): [rust/tcl-syntax/tests/data/native_command_trace/8.4.counted_nul.stdout](../../../../rust/tcl-syntax/tests/data/native_command_trace/8.4.counted_nul.stdout). SHA-256 `30a245fca1786e87a097912e38b268dca8187f6edc70dd83a146cd9f0921ef83`. Complete original direct-output stream.
- `runtime-tcl8.5` (observation): [runtime/rust/tests/data/native_command_traces/provenance.json](../../../../runtime/rust/tests/data/native_command_traces/provenance.json). SHA-256 `5eae5fcd8bf8025aba2f96b43de5502ad08924c8540f2bb6fc2a97d5519f445d`. JSON pointer `/3`. Exact result-presenter guest result and original observer association.
- `syntax-tcl8.5` (observation): [rust/tcl-syntax/tests/data/native_command_trace/8.5.counted_nul.stdout](../../../../rust/tcl-syntax/tests/data/native_command_trace/8.5.counted_nul.stdout). SHA-256 `30a245fca1786e87a097912e38b268dca8187f6edc70dd83a146cd9f0921ef83`. Complete original direct-output stream.
- `runtime-tcl8.6` (observation): [runtime/rust/tests/data/native_command_traces/provenance.json](../../../../runtime/rust/tests/data/native_command_traces/provenance.json). SHA-256 `5eae5fcd8bf8025aba2f96b43de5502ad08924c8540f2bb6fc2a97d5519f445d`. JSON pointer `/5`. Exact result-presenter guest result and original observer association.
- `syntax-tcl8.6` (observation): [rust/tcl-syntax/tests/data/native_command_trace/8.6.counted_nul.stdout](../../../../rust/tcl-syntax/tests/data/native_command_trace/8.6.counted_nul.stdout). SHA-256 `59255d4044c9ca9d3f2224f04c5f54fd84539f2c112b59a7163e95496975b2e7`. Complete original direct-output stream.
- `runtime-tcl9.0` (observation): [runtime/rust/tests/data/native_command_traces/provenance.json](../../../../runtime/rust/tests/data/native_command_traces/provenance.json). SHA-256 `5eae5fcd8bf8025aba2f96b43de5502ad08924c8540f2bb6fc2a97d5519f445d`. JSON pointer `/7`. Exact result-presenter guest result and original observer association.
- `syntax-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_command_trace/9.0.counted_nul.stdout](../../../../rust/tcl-syntax/tests/data/native_command_trace/9.0.counted_nul.stdout). SHA-256 `59255d4044c9ca9d3f2224f04c5f54fd84539f2c112b59a7163e95496975b2e7`. Complete original direct-output stream.
- `runtime-tcl9.1` (observation): [runtime/rust/tests/data/native_command_traces/provenance.json](../../../../runtime/rust/tests/data/native_command_traces/provenance.json). SHA-256 `5eae5fcd8bf8025aba2f96b43de5502ad08924c8540f2bb6fc2a97d5519f445d`. JSON pointer `/9`. Exact result-presenter guest result and original observer association.
- `syntax-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_command_trace/9.1.counted_nul.stdout](../../../../rust/tcl-syntax/tests/data/native_command_trace/9.1.counted_nul.stdout). SHA-256 `59255d4044c9ca9d3f2224f04c5f54fd84539f2c112b59a7163e95496975b2e7`. Complete original direct-output stream.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/cmd_trace/native_command_tests.rs](../../../../runtime/rust/src/cmd_trace/native_command_tests.rs), `cmd_trace::native_command_tests::original_command_traces_match_ten_native_byte_windows` (linked): Compares exact original source/code/result windows; named test execution is not asserted by the retained captures.
- [rust/tcl-vm/src/cmd_trace/native_original_tests.rs](../../../../rust/tcl-vm/src/cmd_trace/native_original_tests.rs), `cmd_trace::native_original_tests::counted_nul_command_trace_prefixes_preserve_native_storage_and_eval_extents` (linked): Compares the independently retained complete direct-output program for each C release, including callback log and removal fields.

A named test is a coverage binding, not a claim that it executed.

## Replay

Exact source inputs and complete stdout are attached. The opaque Syntax driver/source hashes match its receipt. Counted-zero Syntax receipts do not identify a driver/build, while Runtime receipts identify original observer/archive hashes but no observer-source checksum or stderr. Reconfirmation must keep each observer/input pair separate, preserve initialization and counted source length, and retain configured build identity and separate process streams. No current native launch is claimed.
