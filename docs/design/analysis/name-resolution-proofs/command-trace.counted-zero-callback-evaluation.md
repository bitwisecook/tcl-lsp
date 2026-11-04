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

Status: `observed`. Version: 8.4.20 (Runtime capture association); Syntax8.4 only; full launched patchlevels unqueried. Build: Runtime original observer and archive. Separate counted-zero Syntax driver/header/archive identity unrecorded. Compiler/configure and full launched patchlevel unqueried.. Channel: Original counted Tcl_EvalEx source; separate direct stdout versus code/count-result-hex presenter.. Dialect: C Tcl.

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

Status: `observed`. Version: 8.5.19 (Runtime capture association); Syntax8.5 only; full launched patchlevels unqueried. Build: Runtime original observer and archive. Separate counted-zero Syntax driver/header/archive identity unrecorded. Compiler/configure and full launched patchlevel unqueried.. Channel: Original counted Tcl_EvalEx source; separate direct stdout versus code/count-result-hex presenter.. Dialect: C Tcl.

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

Status: `observed`. Version: 8.6.18 (Runtime capture association); Syntax8.6 only; full launched patchlevels unqueried. Build: Runtime original observer and archive. Separate counted-zero Syntax driver/header/archive identity unrecorded. Compiler/configure and full launched patchlevel unqueried.. Channel: Original counted Tcl_EvalEx source; separate direct stdout versus code/count-result-hex presenter.. Dialect: C Tcl.

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

Status: `observed`. Version: 9.0.4 (Runtime capture association); Syntax9.0 only; full launched patchlevels unqueried. Build: Runtime original observer and archive. Separate counted-zero Syntax driver/header/archive identity unrecorded. Compiler/configure and full launched patchlevel unqueried.. Channel: Original counted Tcl_EvalEx source; separate direct stdout versus code/count-result-hex presenter.. Dialect: C Tcl.

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

Status: `observed`. Version: 9.1.0 (Runtime capture association); Syntax9.1 only; full launched patchlevels unqueried. Build: Runtime original observer and archive. Separate counted-zero Syntax driver/header/archive identity unrecorded. Compiler/configure and full launched patchlevel unqueried.. Channel: Original counted Tcl_EvalEx source; separate direct stdout versus code/count-result-hex presenter.. Dialect: C Tcl.

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

## Historical references

- `runtime-receipt` (provider): [runtime/rust/tests/data/native_command_traces/provenance.json](../../../../runtime/rust/tests/data/native_command_traces/provenance.json). Independent original observer/build/result associations.
- `runtime-windows` (observation): [runtime/rust/tests/data/native_command_traces/windows.tsv](../../../../runtime/rust/tests/data/native_command_traces/windows.tsv). Exact source hex, guest code and original result hex for both programs.
- `syntax-input` (input): [rust/tcl-syntax/tests/data/native_command_trace/counted_nul.tcl](../../../../rust/tcl-syntax/tests/data/native_command_trace/counted_nul.tcl). Exact independently retained direct-output source bytes.
- `syntax-receipt` (provider): [rust/tcl-syntax/tests/data/native_command_trace/counted-nul-provenance.json](../../../../rust/tcl-syntax/tests/data/native_command_trace/counted-nul-provenance.json). Separate original direct-output associations.
- `runtime-tcl8.4` (observation): [runtime/rust/tests/data/native_command_traces/provenance.json](../../../../runtime/rust/tests/data/native_command_traces/provenance.json). JSON pointer `/1`. Exact result-presenter guest result and original observer association.
- `syntax-tcl8.4` (observation): [rust/tcl-syntax/tests/data/native_command_trace/8.4.counted_nul.stdout](../../../../rust/tcl-syntax/tests/data/native_command_trace/8.4.counted_nul.stdout). Complete original direct-output stream.
- `runtime-tcl8.5` (observation): [runtime/rust/tests/data/native_command_traces/provenance.json](../../../../runtime/rust/tests/data/native_command_traces/provenance.json). JSON pointer `/3`. Exact result-presenter guest result and original observer association.
- `syntax-tcl8.5` (observation): [rust/tcl-syntax/tests/data/native_command_trace/8.5.counted_nul.stdout](../../../../rust/tcl-syntax/tests/data/native_command_trace/8.5.counted_nul.stdout). Complete original direct-output stream.
- `runtime-tcl8.6` (observation): [runtime/rust/tests/data/native_command_traces/provenance.json](../../../../runtime/rust/tests/data/native_command_traces/provenance.json). JSON pointer `/5`. Exact result-presenter guest result and original observer association.
- `syntax-tcl8.6` (observation): [rust/tcl-syntax/tests/data/native_command_trace/8.6.counted_nul.stdout](../../../../rust/tcl-syntax/tests/data/native_command_trace/8.6.counted_nul.stdout). Complete original direct-output stream.
- `runtime-tcl9.0` (observation): [runtime/rust/tests/data/native_command_traces/provenance.json](../../../../runtime/rust/tests/data/native_command_traces/provenance.json). JSON pointer `/7`. Exact result-presenter guest result and original observer association.
- `syntax-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_command_trace/9.0.counted_nul.stdout](../../../../rust/tcl-syntax/tests/data/native_command_trace/9.0.counted_nul.stdout). Complete original direct-output stream.
- `runtime-tcl9.1` (observation): [runtime/rust/tests/data/native_command_traces/provenance.json](../../../../runtime/rust/tests/data/native_command_traces/provenance.json). JSON pointer `/9`. Exact result-presenter guest result and original observer association.
- `syntax-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_command_trace/9.1.counted_nul.stdout](../../../../rust/tcl-syntax/tests/data/native_command_trace/9.1.counted_nul.stdout). Complete original direct-output stream.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/cmd_trace/native_command_tests.rs](../../../../runtime/rust/src/cmd_trace/native_command_tests.rs), `cmd_trace::native_command_tests::original_command_traces_match_ten_native_byte_windows` (linked): Compares exact original source/code/result windows; named test execution is not asserted by the retained captures.
- [rust/tcl-vm/src/cmd_trace/native_original_tests.rs](../../../../rust/tcl-vm/src/cmd_trace/native_original_tests.rs), `cmd_trace::native_original_tests::counted_nul_command_trace_prefixes_preserve_native_storage_and_eval_extents` (linked): Compares the independently retained complete direct-output program for each C release, including callback log and removal fields.

A named test is a coverage binding, not a claim that it executed.

## Replay

Exact source inputs and complete stdout are attached. The opaque Syntax driver/source hashes match its receipt. Counted-zero Syntax receipts do not identify a driver/build, while Runtime receipts identify original observer/archive hashes but no observer-source checksum or stderr. Reconfirmation must keep each observer/input pair separate, preserve initialization and counted source length, and retain configured build identity and separate process streams. No current native launch is claimed.
