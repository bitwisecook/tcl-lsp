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

Status: `observed`. Version: 8.4.20 (Runtime capture association); Syntax8.4 only; full launched patchlevels unqueried. Build: Runtime original observer and archive. Separate Syntax driver and archive. Compiler/configure and full launched patchlevel unqueried.. Channel: Original counted Tcl_EvalEx source; separate direct stdout versus code/count-result-hex presenter.. Dialect: C Tcl.

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

Status: `observed`. Version: 8.5.19 (Runtime capture association); Syntax8.5 only; full launched patchlevels unqueried. Build: Runtime original observer and archive. Separate Syntax driver and archive. Compiler/configure and full launched patchlevel unqueried.. Channel: Original counted Tcl_EvalEx source; separate direct stdout versus code/count-result-hex presenter.. Dialect: C Tcl.

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

Status: `observed`. Version: 8.6.18 (Runtime capture association); Syntax8.6 only; full launched patchlevels unqueried. Build: Runtime original observer and archive. Separate Syntax driver and archive. Compiler/configure and full launched patchlevel unqueried.. Channel: Original counted Tcl_EvalEx source; separate direct stdout versus code/count-result-hex presenter.. Dialect: C Tcl.

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

Status: `observed`. Version: 9.0.4 (Runtime capture association); Syntax9.0 only; full launched patchlevels unqueried. Build: Runtime original observer and archive. Separate Syntax driver and archive. Compiler/configure and full launched patchlevel unqueried.. Channel: Original counted Tcl_EvalEx source; separate direct stdout versus code/count-result-hex presenter.. Dialect: C Tcl.

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

Status: `observed`. Version: 9.1.0 (Runtime capture association); Syntax9.1 only; full launched patchlevels unqueried. Build: Runtime original observer and archive. Separate Syntax driver and archive. Compiler/configure and full launched patchlevel unqueried.. Channel: Original counted Tcl_EvalEx source; separate direct stdout versus code/count-result-hex presenter.. Dialect: C Tcl.

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

## Historical references

- `runtime-receipt` (provider): [runtime/rust/tests/data/native_command_traces/provenance.json](../../../../runtime/rust/tests/data/native_command_traces/provenance.json). Independent original observer/build/result associations.
- `runtime-windows` (observation): [runtime/rust/tests/data/native_command_traces/windows.tsv](../../../../runtime/rust/tests/data/native_command_traces/windows.tsv). Exact source hex, guest code and original result hex for both programs.
- `syntax-input` (input): [rust/tcl-syntax/tests/data/native_command_trace/opaque.tcl](../../../../rust/tcl-syntax/tests/data/native_command_trace/opaque.tcl). Exact independently retained direct-output source bytes.
- `syntax-receipt` (provider): [rust/tcl-syntax/tests/data/native_command_trace/opaque-provenance.json](../../../../rust/tcl-syntax/tests/data/native_command_trace/opaque-provenance.json). Separate original direct-output associations.
- `runtime-tcl8.4` (observation): [runtime/rust/tests/data/native_command_traces/provenance.json](../../../../runtime/rust/tests/data/native_command_traces/provenance.json). JSON pointer `/0`. Exact result-presenter guest result and original observer association.
- `syntax-tcl8.4` (observation): [rust/tcl-syntax/tests/data/native_command_trace/8.4.opaque.stdout](../../../../rust/tcl-syntax/tests/data/native_command_trace/8.4.opaque.stdout). Complete original direct-output stream.
- `runtime-tcl8.5` (observation): [runtime/rust/tests/data/native_command_traces/provenance.json](../../../../runtime/rust/tests/data/native_command_traces/provenance.json). JSON pointer `/2`. Exact result-presenter guest result and original observer association.
- `syntax-tcl8.5` (observation): [rust/tcl-syntax/tests/data/native_command_trace/8.5.opaque.stdout](../../../../rust/tcl-syntax/tests/data/native_command_trace/8.5.opaque.stdout). Complete original direct-output stream.
- `runtime-tcl8.6` (observation): [runtime/rust/tests/data/native_command_traces/provenance.json](../../../../runtime/rust/tests/data/native_command_traces/provenance.json). JSON pointer `/4`. Exact result-presenter guest result and original observer association.
- `syntax-tcl8.6` (observation): [rust/tcl-syntax/tests/data/native_command_trace/8.6.opaque.stdout](../../../../rust/tcl-syntax/tests/data/native_command_trace/8.6.opaque.stdout). Complete original direct-output stream.
- `runtime-tcl9.0` (observation): [runtime/rust/tests/data/native_command_traces/provenance.json](../../../../runtime/rust/tests/data/native_command_traces/provenance.json). JSON pointer `/6`. Exact result-presenter guest result and original observer association.
- `syntax-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_command_trace/9.0.opaque.stdout](../../../../rust/tcl-syntax/tests/data/native_command_trace/9.0.opaque.stdout). Complete original direct-output stream.
- `runtime-tcl9.1` (observation): [runtime/rust/tests/data/native_command_traces/provenance.json](../../../../runtime/rust/tests/data/native_command_traces/provenance.json). JSON pointer `/8`. Exact result-presenter guest result and original observer association.
- `syntax-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_command_trace/9.1.opaque.stdout](../../../../rust/tcl-syntax/tests/data/native_command_trace/9.1.opaque.stdout). Complete original direct-output stream.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/cmd_trace/native_command_tests.rs](../../../../runtime/rust/src/cmd_trace/native_command_tests.rs), `cmd_trace::native_command_tests::original_command_traces_match_ten_native_byte_windows` (linked): Compares exact original source/code/result windows; named test execution is not asserted by the retained captures.
- [rust/tcl-vm/src/cmd_trace/native_original_tests.rs](../../../../rust/tcl-vm/src/cmd_trace/native_original_tests.rs), `cmd_trace::native_original_tests::opaque_command_trace_operands_match_all_five_direct_native_engines` (linked): Compares the independently retained complete direct-output program for each C release, including callback log and removal fields.

A named test is a coverage binding, not a claim that it executed.

## Replay

Exact source inputs and complete stdout are attached. The opaque Syntax driver/source hashes match its receipt. Counted-zero Syntax receipts do not identify a driver/build, while Runtime receipts identify original observer/archive hashes but no observer-source checksum or stderr. Reconfirmation must keep each observer/input pair separate, preserve initialization and counted source length, and retain configured build identity and separate process streams. No current native launch is claimed.
