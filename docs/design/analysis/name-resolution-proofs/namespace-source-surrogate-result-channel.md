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

### tcl8.4, tcl8.5, tcl8.6

Status: `observed`. Channel: ASCII source-file shell ingress; result string length/index and scan %c are reported as decimal units in an ASCII-only row. No raw counted NativeString NUL, binary byte projection or physical object cache claim. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18.

Attempt 2 process 0, eleven complete rows; Tcl 8.x attempt 1 also completes.

### tcl9.0, tcl9.1

Status: `observed`. Channel: ASCII source-file shell ingress; result string length/index and scan %c are reported as decimal units in an ASCII-only row. No raw counted NativeString NUL, binary byte projection or physical object cache claim. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

Attempt 1 process 1, seven complete rows plus a partial eighth row and strict stdout encoding error; attempt 2 process 0, eleven complete rows.

### jim

Status: `observed`. Channel: ASCII source-file shell ingress; result string length/index and scan %c are reported as decimal units in an ASCII-only row. No raw counted NativeString NUL, binary byte projection or physical object cache claim. Version: jim 0.84-9-g5bac7c9.

Both attempts process 0 and eleven rows; no corresponding output-channel refusal.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2)
- [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt1](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt1)
