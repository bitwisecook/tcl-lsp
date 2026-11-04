# naming.variable.dictionary-scope-trace-phase-order

Kind: `native-observation`

## Problem statement

A dictionary body can read or replace its receiver and mapped variables, fail, return, or retire the receiver. Applying update/with writeback before these callbacks or borrowing local-cell opcode ordering would change both results and trace chronology. These controls select the generic worker through a variable-held command head.

## Question

In the sixteen exact generic dict update/with scripts, which read/write callbacks occur and what completion/result/receiver survive normal, error, return, replacement and retirement paths?

## Conclusion

The retained generic workers have the exact callback and writeback order shown below. C85+ provide 64 measured callback windows; C84 lacks dict and Jim lacks trace in these builds. Those unsupported guest results are preserved, and no compiled-local, physical cell, or normal-completion capability follows.

## Scope

Six original shell builds, sixteen ASCII scripts each, outer catch presenter and exact result bytes. Generic dynamic command head only. Native output and current Rust comparisons are independent; no custom object release/header or arbitrary callback closure is measured.

## Provider answers

### tcl8.4

Status: `unsupported`. Version: 8.4.20. Build: See historical references.. Channel: ASCII script bytes to shell stdin; exact outer catch/binary scan presenter. Dialect: Tcl.

All sixteen fresh processes exit 0 with empty stderr; outer caught code and result are shown, including the inner code/result/events/receiver tuple. The selected dict command is unavailable; these are actual guest errors, not trace-order successes.

| case | outer code | outer result (JSON escaped) |
| --- | --- | --- |
| update-read-order | 0 | "1 {invalid command name \"dict\"} {} {k OLD}" |
| update-full-order | 0 | "1 {invalid command name \"dict\"} {} {k OLD}" |
| update-root-replacement | 0 | "1 {invalid command name \"dict\"} {} {k OLD}" |
| update-read-error | 0 | "1 {invalid command name \"dict\"} {} {k OLD}" |
| update-body-error | 0 | "1 {invalid command name \"dict\"} {} {k OLD}" |
| update-body-return | 0 | "1 {invalid command name \"dict\"} {} {k OLD}" |
| update-mapped-read | 0 | "1 {invalid command name \"dict\"} {} {k OLD}" |
| update-root-retirement | 0 | "1 {invalid command name \"dict\"} {} {k OLD}" |
| with-read-order | 0 | "1 {invalid command name \"dict\"} {} {k OLD}" |
| with-full-order | 0 | "1 {invalid command name \"dict\"} {} {k OLD}" |
| with-root-replacement | 0 | "1 {invalid command name \"dict\"} {} {k OLD}" |
| with-read-error | 0 | "1 {invalid command name \"dict\"} {} {k OLD}" |
| with-body-error | 0 | "1 {invalid command name \"dict\"} {} {k OLD}" |
| with-body-return | 0 | "1 {invalid command name \"dict\"} {} {k OLD}" |
| with-mapped-read | 0 | "1 {invalid command name \"dict\"} {} {k OLD}" |
| with-root-retirement | 0 | "1 {invalid command name \"dict\"} {} {k OLD}" |

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: See historical references.. Channel: ASCII script bytes to shell stdin; exact outer catch/binary scan presenter. Dialect: Tcl.

All sixteen fresh processes exit 0 with empty stderr; outer caught code and result are shown, including the inner code/result/events/receiver tuple.

| case | outer code | outer result (JSON escaped) |
| --- | --- | --- |
| update-read-order | 0 | "0 NEW {read read} {k NEW}" |
| update-full-order | 0 | "0 NEW {read write write read read write} {k NEW}" |
| update-root-replacement | 0 | "0 NEW {read read} {fresh KEPT k NEW}" |
| update-read-error | 0 | "0 NEW {read read} {k OLD}" |
| update-body-error | 0 | "1 BODY {read read} {k NEW}" |
| update-body-return | 0 | "2 BODY {read read} {k NEW}" |
| update-mapped-read | 0 | "0 NEW read {k REACHED}" |
| update-root-retirement | 0 | "0 NEW {read read} {fresh KEPT k NEW}" |
| with-read-order | 0 | "0 NEW {read read} {k NEW}" |
| with-full-order | 0 | "0 NEW {read write write read read write} {k NEW}" |
| with-root-replacement | 0 | "0 NEW {read read} {fresh KEPT k NEW}" |
| with-read-error | 0 | "0 NEW {read read} {k OLD}" |
| with-body-error | 0 | "1 BODY {read read} {k NEW}" |
| with-body-return | 0 | "2 BODY {read read} {k NEW}" |
| with-mapped-read | 0 | "0 NEW read {k REACHED}" |
| with-root-retirement | 0 | "0 NEW {read read} {fresh KEPT k NEW}" |

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: See historical references.. Channel: ASCII script bytes to shell stdin; exact outer catch/binary scan presenter. Dialect: Tcl.

All sixteen fresh processes exit 0 with empty stderr; outer caught code and result are shown, including the inner code/result/events/receiver tuple.

| case | outer code | outer result (JSON escaped) |
| --- | --- | --- |
| update-read-order | 0 | "0 NEW {read read} {k NEW}" |
| update-full-order | 0 | "0 NEW {read write write read read write} {k NEW}" |
| update-root-replacement | 0 | "0 NEW {read read} {fresh KEPT k NEW}" |
| update-read-error | 0 | "0 NEW {read read} {k OLD}" |
| update-body-error | 0 | "1 BODY {read read} {k NEW}" |
| update-body-return | 0 | "2 BODY {read read} {k NEW}" |
| update-mapped-read | 0 | "0 NEW read {k REACHED}" |
| update-root-retirement | 0 | "0 NEW {read read} {fresh KEPT k NEW}" |
| with-read-order | 0 | "0 NEW {read read} {k NEW}" |
| with-full-order | 0 | "0 NEW {read write write read read write} {k NEW}" |
| with-root-replacement | 0 | "0 NEW {read read} {fresh KEPT k NEW}" |
| with-read-error | 0 | "0 NEW {read read} {k OLD}" |
| with-body-error | 0 | "1 BODY {read read} {k NEW}" |
| with-body-return | 0 | "2 BODY {read read} {k NEW}" |
| with-mapped-read | 0 | "0 NEW read {k REACHED}" |
| with-root-retirement | 0 | "0 NEW {read read} {fresh KEPT k NEW}" |

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: See historical references.. Channel: ASCII script bytes to shell stdin; exact outer catch/binary scan presenter. Dialect: Tcl.

All sixteen fresh processes exit 0 with empty stderr; outer caught code and result are shown, including the inner code/result/events/receiver tuple.

| case | outer code | outer result (JSON escaped) |
| --- | --- | --- |
| update-read-order | 0 | "0 NEW {read read} {k NEW}" |
| update-full-order | 0 | "0 NEW {read write write read read write} {k NEW}" |
| update-root-replacement | 0 | "0 NEW {read read} {fresh KEPT k NEW}" |
| update-read-error | 0 | "0 NEW {read read} {k OLD}" |
| update-body-error | 0 | "1 BODY {read read} {k NEW}" |
| update-body-return | 0 | "2 BODY {read read} {k NEW}" |
| update-mapped-read | 0 | "0 NEW read {k REACHED}" |
| update-root-retirement | 0 | "0 NEW {read read} {fresh KEPT k NEW}" |
| with-read-order | 0 | "0 NEW {read read} {k NEW}" |
| with-full-order | 0 | "0 NEW {read write write read read write} {k NEW}" |
| with-root-replacement | 0 | "0 NEW {read read} {fresh KEPT k NEW}" |
| with-read-error | 0 | "0 NEW {read read} {k OLD}" |
| with-body-error | 0 | "1 BODY {read read} {k NEW}" |
| with-body-return | 0 | "2 BODY {read read} {k NEW}" |
| with-mapped-read | 0 | "0 NEW read {k REACHED}" |
| with-root-retirement | 0 | "0 NEW {read read} {fresh KEPT k NEW}" |

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: See historical references.. Channel: ASCII script bytes to shell stdin; exact outer catch/binary scan presenter. Dialect: Tcl.

All sixteen fresh processes exit 0 with empty stderr; outer caught code and result are shown, including the inner code/result/events/receiver tuple.

| case | outer code | outer result (JSON escaped) |
| --- | --- | --- |
| update-read-order | 0 | "0 NEW {read read} {k NEW}" |
| update-full-order | 0 | "0 NEW {read write write read read write} {k NEW}" |
| update-root-replacement | 0 | "0 NEW {read read} {fresh KEPT k NEW}" |
| update-read-error | 0 | "0 NEW {read read} {k OLD}" |
| update-body-error | 0 | "1 BODY {read read} {k NEW}" |
| update-body-return | 0 | "2 BODY {read read} {k NEW}" |
| update-mapped-read | 0 | "0 NEW read {k REACHED}" |
| update-root-retirement | 0 | "0 NEW {read read} {fresh KEPT k NEW}" |
| with-read-order | 0 | "0 NEW {read read} {k NEW}" |
| with-full-order | 0 | "0 NEW {read write write read read write} {k NEW}" |
| with-root-replacement | 0 | "0 NEW {read read} {fresh KEPT k NEW}" |
| with-read-error | 0 | "0 NEW {read read} {k OLD}" |
| with-body-error | 0 | "1 BODY {read read} {k NEW}" |
| with-body-return | 0 | "2 BODY {read read} {k NEW}" |
| with-mapped-read | 0 | "0 NEW read {k REACHED}" |
| with-root-retirement | 0 | "0 NEW {read read} {fresh KEPT k NEW}" |

### jim

Status: `unsupported`. Version: not recorded (manifest label jim). Build: See historical references.. Channel: ASCII script bytes to shell stdin; exact outer catch/binary scan presenter. Dialect: Jim Tcl.

All sixteen fresh processes exit 0 with empty stderr; outer caught code and result are shown, including the inner code/result/events/receiver tuple. Trace registration fails before dict scope execution; these do not measure Jim callback order.

| case | outer code | outer result (JSON escaped) |
| --- | --- | --- |
| update-read-order | 1 | "invalid command name \"trace\"" |
| update-full-order | 1 | "invalid command name \"trace\"" |
| update-root-replacement | 1 | "invalid command name \"trace\"" |
| update-read-error | 1 | "invalid command name \"trace\"" |
| update-body-error | 1 | "invalid command name \"trace\"" |
| update-body-return | 1 | "invalid command name \"trace\"" |
| update-mapped-read | 1 | "invalid command name \"trace\"" |
| update-root-retirement | 1 | "invalid command name \"trace\"" |
| with-read-order | 1 | "invalid command name \"trace\"" |
| with-full-order | 1 | "invalid command name \"trace\"" |
| with-root-replacement | 1 | "invalid command name \"trace\"" |
| with-read-error | 1 | "invalid command name \"trace\"" |
| with-body-error | 1 | "invalid command name \"trace\"" |
| with-body-return | 1 | "invalid command name \"trace\"" |
| with-mapped-read | 1 | "invalid command name \"trace\"" |
| with-root-retirement | 1 | "invalid command name \"trace\"" |

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Historical references

- `e0` (provider): [runtime/rust/tests/data/native_dictionary_scope_traces/manifest.json](../../../../runtime/rust/tests/data/native_dictionary_scope_traces/manifest.json). 96 separate shell executions with command/binary/source/observer hashes, exit status, empty stderr and exact stdout.
- `e1` (input): [runtime/rust/tests/data/native_dictionary_scope_traces/cases.json](../../../../runtime/rust/tests/data/native_dictionary_scope_traces/cases.json). Retained exact cases.json; original source/object inputs and reporter boundary, independent of Rust implementation.
- `e2` (input): [runtime/rust/tests/data/native_dictionary_scope_traces/run.py](../../../../runtime/rust/tests/data/native_dictionary_scope_traces/run.py). Retained exact run.py; original source/object inputs and reporter boundary, independent of Rust implementation.
- `e3` (observation): [runtime/rust/tests/data/native_dictionary_scope_traces/controls.tsv](../../../../runtime/rust/tests/data/native_dictionary_scope_traces/controls.tsv). 96 exact source/code/result rows; inner completion tuple remains part of the outer result.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/cmd_dict/native_scope_trace_tests.rs](../../../../runtime/rust/src/cmd_dict/native_scope_trace_tests.rs), `dictionary_scope_reads_match_64_native_callback_windows`: Independent current Rust comparison at the stated semantic boundary.
- [runtime/rust/src/cmd_dict/native_scope_trace_tests.rs](../../../../runtime/rust/src/cmd_dict/native_scope_trace_tests.rs), `cmd_dict::native_scope_trace_tests::dictionary_scope_reads_match_64_native_callback_windows` (linked): Compares the 64 C85+ callback/code/result windows; C84 and Jim unsupported branches are not included in that count.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "runtime/rust/tests/data/native_dictionary_scope_traces/run.py"
]
```

The retained runner is executable source and constructs the exact outer presenter. Authenticate the six manifested native binaries at its recorded /tmp/2286-oracles paths; run from an isolated copied fixture directory because it rewrites manifest.json and controls.tsv. Require all 96 captured source/code/result rows to match the retained controls, including unavailable dict/trace errors, fresh process exit 0 and empty stderr. Reconstructed source and observer hashes must match the original manifest. No Rust execution is implied.
