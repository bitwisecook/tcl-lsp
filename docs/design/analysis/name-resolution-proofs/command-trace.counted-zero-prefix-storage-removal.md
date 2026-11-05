# naming.command-trace.counted-zero-prefix-storage-removal

Kind: `native-observation`

## Problem statement

The counted source contains raw zero inside a command target suffix and two different callback suffixes A/B. Retaining all original bytes for every operation or using one undifferentiated zero rule predicts different membership and reported prefixes.

## Question

What prefix bytes and membership follow registration with raw-zero suffix A and removal with raw-zero suffix B for the original command and execution trace controls?

## Conclusion

All five C programs report callback prefix6362ff and membership0 after removal for both trace kinds. These operation-specific report/removal outcomes do not prove a universal counted-name comparator or arbitrary source-zero behavior.

## Scope

Five C release associations with original Tcl_EvalEx counted source. The Syntax opaque driver initializes Tcl and forwards the file bytes; the Runtime result presenter uses a fresh uninitialized interpreter, evaluates each exact encoded source and prints code plus counted result hex. The independently retained programs differ in presentation and initial setup and are not asserted byte-identical. Raw FF/zero occur in their retained inputs. No original object-header, trace internal storage allocation, Jim or BIG-IP behavior is inferred.

## Provider answers

### tcl8.4, tcl8.5

Status: `observed`. Channel: Original counted Tcl_EvalEx source; separate direct stdout versus code/count-result-hex presenter. Version: tcl8.4 8.4.20 (Runtime capture association); Syntax8.4 only; full launched patchlevels unqueried; tcl8.5 8.5.19 (Runtime capture association); Syntax8.5 only; full launched patchlevels unqueried.

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

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original counted Tcl_EvalEx source; separate direct stdout versus code/count-result-hex presenter. Version: tcl8.6 8.6.18 (Runtime capture association); Syntax8.6 only; full launched patchlevels unqueried; tcl9.0 9.0.4 (Runtime capture association); Syntax9.0 only; full launched patchlevels unqueried; tcl9.1 9.1.0 (Runtime capture association); Syntax9.1 only; full launched patchlevels unqueried.

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

### jim, bigip

Status: `not-tested`.

No original command/execution trace program capture for this provider is attached.

## Consumer bindings

- [runtime/rust/src/cmd_trace/native_command_tests.rs](../../../../runtime/rust/src/cmd_trace/native_command_tests.rs), `cmd_trace::native_command_tests::original_command_traces_match_ten_native_byte_windows` (linked): Compares exact original source/code/result windows; named test execution is not asserted by the retained captures.
- [rust/tcl-vm/src/cmd_trace/native_original_tests.rs](../../../../rust/tcl-vm/src/cmd_trace/native_original_tests.rs), `cmd_trace::native_original_tests::counted_nul_command_trace_prefixes_preserve_native_storage_and_eval_extents` (linked): Compares the independently retained complete direct-output program for each C release, including callback log and removal fields.

## Evidence

- [runtime/rust/tests/data/native_command_traces/provenance.json](../../../../runtime/rust/tests/data/native_command_traces/provenance.json)
- [runtime/rust/tests/data/native_command_traces/windows.tsv](../../../../runtime/rust/tests/data/native_command_traces/windows.tsv)
- [rust/tcl-syntax/tests/data/native_command_trace](../../../../rust/tcl-syntax/tests/data/native_command_trace)
