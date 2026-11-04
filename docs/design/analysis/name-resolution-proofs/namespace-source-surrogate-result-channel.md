# naming.namespace.source-surrogate-result-channel

Kind: `native-observation`

## Problem statement

A source operation error can contain opaque units that cannot be printed through a strict output channel. A truncated replay must not classify later unattempted cases as guest failures.

## Question

Does direct error-result output complete all controls, and can an ASCII numeric string-operation projection retain every result without printing the opaque unit?

## Conclusion

Attempt 1 completes on Tcl 8.x and Jim. Tcl 9.0/9.1 exit 1 while printing the eighth same-source-import error, retaining partial stdout and a strict output encoding error; the final three controls are unattempted. Attempt 2 completes all eleven controls on every provider using only numeric string-operation result output.

## Scope

The exact ASCII source-file controls are captured on Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and Jim 0.84-9-g5bac7c9. Backslash substitutions produce the tested adjacent surrogate units. Attempt 2 reports provider string length/index and scan %c results as decimal ASCII units; it does not assert raw bytes or a universal scalar representation. These observations grant no counted API, raw NUL, physical cache, compiler preparation, arbitrary glob equivalence or source execution authority. BIG-IP was not tested.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Executable. Channel: ASCII source-file shell ingress; result string length/index and scan %c are reported as decimal units in an ASCII-only row. No raw counted NativeString NUL, binary byte projection or physical object cache claim.. Dialect: Tcl.

Attempt 2 process 0, eleven complete rows; Tcl 8.x attempt 1 also completes.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Executable. Channel: ASCII source-file shell ingress; result string length/index and scan %c are reported as decimal units in an ASCII-only row. No raw counted NativeString NUL, binary byte projection or physical object cache claim.. Dialect: Tcl.

Attempt 2 process 0, eleven complete rows; Tcl 8.x attempt 1 also completes.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Executable. Channel: ASCII source-file shell ingress; result string length/index and scan %c are reported as decimal units in an ASCII-only row. No raw counted NativeString NUL, binary byte projection or physical object cache claim.. Dialect: Tcl.

Attempt 2 process 0, eleven complete rows; Tcl 8.x attempt 1 also completes.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Executable. Channel: ASCII source-file shell ingress; result string length/index and scan %c are reported as decimal units in an ASCII-only row. No raw counted NativeString NUL, binary byte projection or physical object cache claim.. Dialect: Tcl.

Attempt 1 process 1, seven complete rows plus a partial eighth row and strict stdout encoding error; attempt 2 process 0, eleven complete rows.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Executable. Channel: ASCII source-file shell ingress; result string length/index and scan %c are reported as decimal units in an ASCII-only row. No raw counted NativeString NUL, binary byte projection or physical object cache claim.. Dialect: Tcl.

Attempt 1 process 1, seven complete rows plus a partial eighth row and strict stdout encoding error; attempt 2 process 0, eleven complete rows.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Executable. Channel: ASCII source-file shell ingress; result string length/index and scan %c are reported as decimal units in an ASCII-only row. No raw counted NativeString NUL, binary byte projection or physical object cache claim.. Dialect: Jim Tcl.

Both attempts process 0 and eleven rows; no corresponding output-channel refusal.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Historical references

- `source` (input): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/cases.tcl](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/cases.tcl). Exact ASCII source program with eleven independent catch-result rows.
- `receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/receipt.json). All six actual process, reported version, source/executable/output hashes and row counts.
- `runner` (input): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/capture.py](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/capture.py). Retained capture program; its original workspace paths are recorded.
- `tcl8.4-output` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.4.20/stdout). Actual numeric result rows.
- `tcl8.4-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.4.20/receipt.json). Actual provider identity and invocation/output digests.
- `tcl8.4-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.4.20/stderr). Actual empty stderr stream.
- `tcl8.5-output` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.5.19/stdout). Actual numeric result rows.
- `tcl8.5-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.5.19/receipt.json). Actual provider identity and invocation/output digests.
- `tcl8.5-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.5.19/stderr). Actual empty stderr stream.
- `tcl8.6-output` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.6.18/stdout). Actual numeric result rows.
- `tcl8.6-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.6.18/receipt.json). Actual provider identity and invocation/output digests.
- `tcl8.6-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.6.18/stderr). Actual empty stderr stream.
- `tcl9.0-output` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.0.4/stdout). Actual numeric result rows.
- `tcl9.0-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.0.4/receipt.json). Actual provider identity and invocation/output digests.
- `tcl9.0-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.0.4/stderr). Actual empty stderr stream.
- `tcl9.1-output` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.1.0/stdout). Actual numeric result rows.
- `tcl9.1-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.1.0/receipt.json). Actual provider identity and invocation/output digests.
- `tcl9.1-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.1.0/stderr). Actual empty stderr stream.
- `jim-output` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/jim/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/jim/stdout). Actual numeric result rows.
- `jim-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/jim/receipt.json). Actual provider identity and invocation/output digests.
- `jim-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/jim/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/jim/stderr). Actual empty stderr stream.
- `first-source` (input): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt1/cases.tcl](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt1/cases.tcl). Unchanged direct-print attempt.
- `first-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt1/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt1/receipt.json). All six first-attempt process/version/output hashes, including C9 partial refusal.
- `tcl8.4-first-output` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt1/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt1/8.4.20/stdout). Original direct-print stdout, including incomplete C9 final line.
- `tcl8.4-first-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt1/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt1/8.4.20/stderr). Original direct-print stderr.
- `tcl8.5-first-output` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt1/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt1/8.5.19/stdout). Original direct-print stdout, including incomplete C9 final line.
- `tcl8.5-first-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt1/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt1/8.5.19/stderr). Original direct-print stderr.
- `tcl8.6-first-output` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt1/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt1/8.6.18/stdout). Original direct-print stdout, including incomplete C9 final line.
- `tcl8.6-first-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt1/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt1/8.6.18/stderr). Original direct-print stderr.
- `tcl9.0-first-output` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt1/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt1/9.0.4/stdout). Original direct-print stdout, including incomplete C9 final line.
- `tcl9.0-first-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt1/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt1/9.0.4/stderr). Original direct-print stderr.
- `tcl9.1-first-output` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt1/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt1/9.1.0/stdout). Original direct-print stdout, including incomplete C9 final line.
- `tcl9.1-first-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt1/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt1/9.1.0/stderr). Original direct-print stderr.
- `jim-first-output` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt1/jim/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt1/jim/stdout). Original direct-print stdout, including incomplete C9 final line.
- `jim-first-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt1/jim/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt1/jim/stderr). Original direct-print stderr.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The retained capture.py records exact retained workspace commands and hashes. The source fixtures and raw outputs are permanent; no portable executable provisioning replay is supplied. Provider answers remain scoped to the listed cases and actual channels.
