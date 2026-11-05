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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original counted Tcl_EvalEx source; separate direct stdout versus code/count-result-hex presenter. Version: tcl8.4 8.4.20 (Runtime capture association); Syntax8.4 only; full launched patchlevels unqueried; tcl8.5 8.5.19 (Runtime capture association); Syntax8.5 only; full launched patchlevels unqueried; tcl8.6 8.6.18 (Runtime capture association); Syntax8.6 only; full launched patchlevels unqueried; tcl9.0 9.0.4 (Runtime capture association); Syntax9.0 only; full launched patchlevels unqueried; tcl9.1 9.1.0 (Runtime capture association); Syntax9.1 only; full launched patchlevels unqueried.

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

### jim, bigip

Status: `not-tested`.

No original command/execution trace program capture for this provider is attached.

## Consumer bindings

- [runtime/rust/src/cmd_trace/native_command_tests.rs](../../../../runtime/rust/src/cmd_trace/native_command_tests.rs), `cmd_trace::native_command_tests::original_command_traces_match_ten_native_byte_windows` (linked): Compares exact original source/code/result windows; named test execution is not asserted by the retained captures.
- [rust/tcl-vm/src/cmd_trace/native_original_tests.rs](../../../../rust/tcl-vm/src/cmd_trace/native_original_tests.rs), `cmd_trace::native_original_tests::opaque_command_trace_operands_match_all_five_direct_native_engines` (linked): Compares the independently retained complete direct-output program for each C release, including callback log and removal fields.

## Evidence

- [runtime/rust/tests/data/native_command_traces/provenance.json](../../../../runtime/rust/tests/data/native_command_traces/provenance.json)
- [runtime/rust/tests/data/native_command_traces/windows.tsv](../../../../runtime/rust/tests/data/native_command_traces/windows.tsv)
- [rust/tcl-syntax/tests/data/native_command_trace](../../../../rust/tcl-syntax/tests/data/native_command_trace)
