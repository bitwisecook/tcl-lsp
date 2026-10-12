# naming.command-trace.opaque-prefix-report-and-removal

Kind: `native-observation`

## Problem statement

An opaque FF command and callback can be damaged by host Unicode presentation. Reporting or removing its registered prefix must preserve the original byte operand rather than replacing it with decoded display.

## Question

What prefix hex and trace membership remain after registering, reporting and removing opaque command/execution callbacks around command rename and deletion?

## Conclusion

All five C captures report callback hex6362ff for both trace kinds, execution membership0 after removal, command membership1 after rename and0 after removal. The final callback log is enter leave rename delete. The independent result-presenter and direct-puts programs agree on these finite fields.

## Scope

Five C release associations with original Tcl_EvalEx counted source. The Syntax opaque driver initializes Tcl and forwards the file bytes; the Runtime result presenter uses a fresh uninitialized interpreter, evaluates each exact encoded source and prints code plus counted result hex. The independently retained programs differ in presentation and initial setup and are not asserted byte-identical. Raw FF/zero occur in their retained inputs. No original object-header, trace internal storage allocation, Jim or BIG-IP behavior is inferred.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (Runtime capture association); Syntax8.4 only; full launched patchlevels unqueried. Build: Runtime original observer SHA256 03bf2af50be51d7a9c7760a01c6c25bba0519c8bff147ba4754bc346a6d2e0c4 and archive SHA256 d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011. Separate Syntax driver SHA256 4da9875515935c7079297c214bf8174242c3ebe2654064fc26a64dcc63196b92 and archive SHA256 d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011. Compiler/configure and full launched patchlevel unqueried.. Channel: Original counted Tcl_EvalEx source; separate direct stdout versus code/count-result-hex presenter.. Dialect: C Tcl.

Direct-output stream:

```text
6362ff
6362ff
0
1
0
enter leave rename delete
```

Independent presenter stdout:

```text
0	36333632666620363336326666203020312030207b656e746572206c656176652072656e616d652064656c6574657d
```
Original programs and observer assumptions remain separate.

### tcl8.5

Status: `observed`. Version: 8.5.19 (Runtime capture association); Syntax8.5 only; full launched patchlevels unqueried. Build: Runtime original observer SHA256 69cb3b0b02418fe33d414cc0e89d424aed4dd35e8dfa86f36d3f22b6cd64b1df and archive SHA256 99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df. Separate Syntax driver SHA256 61f1cc83c541e3e4474d5b5397f29998eb3443fcf46b43b5e20cfe05a0955607 and archive SHA256 99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df. Compiler/configure and full launched patchlevel unqueried.. Channel: Original counted Tcl_EvalEx source; separate direct stdout versus code/count-result-hex presenter.. Dialect: C Tcl.

Direct-output stream:

```text
6362ff
6362ff
0
1
0
enter leave rename delete
```

Independent presenter stdout:

```text
0	36333632666620363336326666203020312030207b656e746572206c656176652072656e616d652064656c6574657d
```
Original programs and observer assumptions remain separate.

### tcl8.6

Status: `observed`. Version: 8.6.18 (Runtime capture association); Syntax8.6 only; full launched patchlevels unqueried. Build: Runtime original observer SHA256 95e72b2b226a5bc2da91f415588f86b00e58bb4e3a32d18b90faf6841f99e4e4 and archive SHA256 a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6. Separate Syntax driver SHA256 93aeb8d04a7b7c8d33fab7be1065ab551cf57e1f103e3e28209b25ab8f7baddb and archive SHA256 a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6. Compiler/configure and full launched patchlevel unqueried.. Channel: Original counted Tcl_EvalEx source; separate direct stdout versus code/count-result-hex presenter.. Dialect: C Tcl.

Direct-output stream:

```text
6362ff
6362ff
0
1
0
enter leave rename delete
```

Independent presenter stdout:

```text
0	36333632666620363336326666203020312030207b656e746572206c656176652072656e616d652064656c6574657d
```
Original programs and observer assumptions remain separate.

### tcl9.0

Status: `observed`. Version: 9.0.4 (Runtime capture association); Syntax9.0 only; full launched patchlevels unqueried. Build: Runtime original observer SHA256 7da959c6f9f4c9c04cf113fac0846a41d9a70f353584451bab9b20537be36a22 and archive SHA256 2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04. Separate Syntax driver SHA256 c0bbf57a19f7e1c17cd2600d9d4a00f4ecae7ae86dd56ac40b9409dba98c8b0e and archive SHA256 2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04. Compiler/configure and full launched patchlevel unqueried.. Channel: Original counted Tcl_EvalEx source; separate direct stdout versus code/count-result-hex presenter.. Dialect: C Tcl.

Direct-output stream:

```text
6362ff
6362ff
0
1
0
enter leave rename delete
```

Independent presenter stdout:

```text
0	36333632666620363336326666203020312030207b656e746572206c656176652072656e616d652064656c6574657d
```
Original programs and observer assumptions remain separate.

### tcl9.1

Status: `observed`. Version: 9.1.0 (Runtime capture association); Syntax9.1 only; full launched patchlevels unqueried. Build: Runtime original observer SHA256 dbfe9edd5b7611d49a9c2f3da50fe418ee8a2c94bb0dfc057b406a15bbcaa8ef and archive SHA256 8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33. Separate Syntax driver SHA256 de35afb57f0da3378851542d5c7e9d2327b27296028e550daeb06efc77287cc9 and archive SHA256 8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33. Compiler/configure and full launched patchlevel unqueried.. Channel: Original counted Tcl_EvalEx source; separate direct stdout versus code/count-result-hex presenter.. Dialect: C Tcl.

Direct-output stream:

```text
6362ff
6362ff
0
1
0
enter leave rename delete
```

Independent presenter stdout:

```text
0	36333632666620363336326666203020312030207b656e746572206c656176652072656e616d652064656c6574657d
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
- `syntax-input` (input): [rust/tcl-syntax/tests/data/native_command_trace/opaque.tcl](../../../../rust/tcl-syntax/tests/data/native_command_trace/opaque.tcl). SHA-256 `aa84654b2f09a81fe7c934703ea2e331ca3baedcdc2f213db4b57bde5d474bf0`. Exact independently retained direct-output source bytes.
- `syntax-receipt` (provider): [rust/tcl-syntax/tests/data/native_command_trace/opaque-provenance.json](../../../../rust/tcl-syntax/tests/data/native_command_trace/opaque-provenance.json). SHA-256 `0022622eb2984069c024b8fa814d07bdd3748cccaaa3835c7bc1a7b438796e31`. Separate original direct-output associations.
- `runtime-tcl8.4` (observation): [runtime/rust/tests/data/native_command_traces/provenance.json](../../../../runtime/rust/tests/data/native_command_traces/provenance.json). SHA-256 `5eae5fcd8bf8025aba2f96b43de5502ad08924c8540f2bb6fc2a97d5519f445d`. JSON pointer `/0`. Exact result-presenter guest result and original observer association.
- `syntax-tcl8.4` (observation): [rust/tcl-syntax/tests/data/native_command_trace/8.4.opaque.stdout](../../../../rust/tcl-syntax/tests/data/native_command_trace/8.4.opaque.stdout). SHA-256 `50089aae4dfaec3f946767e13d18414f8eea5105c7fd43a09559c05bb5d1948c`. Complete original direct-output stream.
- `runtime-tcl8.5` (observation): [runtime/rust/tests/data/native_command_traces/provenance.json](../../../../runtime/rust/tests/data/native_command_traces/provenance.json). SHA-256 `5eae5fcd8bf8025aba2f96b43de5502ad08924c8540f2bb6fc2a97d5519f445d`. JSON pointer `/2`. Exact result-presenter guest result and original observer association.
- `syntax-tcl8.5` (observation): [rust/tcl-syntax/tests/data/native_command_trace/8.5.opaque.stdout](../../../../rust/tcl-syntax/tests/data/native_command_trace/8.5.opaque.stdout). SHA-256 `50089aae4dfaec3f946767e13d18414f8eea5105c7fd43a09559c05bb5d1948c`. Complete original direct-output stream.
- `runtime-tcl8.6` (observation): [runtime/rust/tests/data/native_command_traces/provenance.json](../../../../runtime/rust/tests/data/native_command_traces/provenance.json). SHA-256 `5eae5fcd8bf8025aba2f96b43de5502ad08924c8540f2bb6fc2a97d5519f445d`. JSON pointer `/4`. Exact result-presenter guest result and original observer association.
- `syntax-tcl8.6` (observation): [rust/tcl-syntax/tests/data/native_command_trace/8.6.opaque.stdout](../../../../rust/tcl-syntax/tests/data/native_command_trace/8.6.opaque.stdout). SHA-256 `50089aae4dfaec3f946767e13d18414f8eea5105c7fd43a09559c05bb5d1948c`. Complete original direct-output stream.
- `runtime-tcl9.0` (observation): [runtime/rust/tests/data/native_command_traces/provenance.json](../../../../runtime/rust/tests/data/native_command_traces/provenance.json). SHA-256 `5eae5fcd8bf8025aba2f96b43de5502ad08924c8540f2bb6fc2a97d5519f445d`. JSON pointer `/6`. Exact result-presenter guest result and original observer association.
- `syntax-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_command_trace/9.0.opaque.stdout](../../../../rust/tcl-syntax/tests/data/native_command_trace/9.0.opaque.stdout). SHA-256 `50089aae4dfaec3f946767e13d18414f8eea5105c7fd43a09559c05bb5d1948c`. Complete original direct-output stream.
- `runtime-tcl9.1` (observation): [runtime/rust/tests/data/native_command_traces/provenance.json](../../../../runtime/rust/tests/data/native_command_traces/provenance.json). SHA-256 `5eae5fcd8bf8025aba2f96b43de5502ad08924c8540f2bb6fc2a97d5519f445d`. JSON pointer `/8`. Exact result-presenter guest result and original observer association.
- `syntax-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_command_trace/9.1.opaque.stdout](../../../../rust/tcl-syntax/tests/data/native_command_trace/9.1.opaque.stdout). SHA-256 `50089aae4dfaec3f946767e13d18414f8eea5105c7fd43a09559c05bb5d1948c`. Complete original direct-output stream.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/cmd_trace/native_command_tests.rs](../../../../runtime/rust/src/cmd_trace/native_command_tests.rs), `cmd_trace::native_command_tests::original_command_traces_match_ten_native_byte_windows` (linked): Compares exact original source/code/result windows; named test execution is not asserted by the retained captures.
- [rust/tcl-vm/src/cmd_trace/native_original_tests.rs](../../../../rust/tcl-vm/src/cmd_trace/native_original_tests.rs), `cmd_trace::native_original_tests::opaque_command_trace_operands_match_all_five_direct_native_engines` (linked): Compares the independently retained complete direct-output program for each C release, including callback log and removal fields.

A named test is a coverage binding, not a claim that it executed.

## Replay

Exact source inputs and complete stdout are attached. The opaque Syntax driver/source hashes match its receipt. Counted-zero Syntax receipts do not identify a driver/build, while Runtime receipts identify original observer/archive hashes but no observer-source checksum or stderr. Reconfirmation must keep each observer/input pair separate, preserve initialization and counted source length, and retain configured build identity and separate process streams. No current native launch is claimed.
