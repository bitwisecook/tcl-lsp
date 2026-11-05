# naming.expression.braced-word-and-arity-boundaries

Kind: `native-observation`

## Problem statement

A braced candidate can change whole-word parsing or collapse the original argument vector even when a C expression result would otherwise match.

## Question

Do the malformed-brace and multiple-operand controls preserve the actual source parse and expression argument-vector behavior?

## Conclusion

The malformed candidate errors with extra characters after close-brace while its original returns a}b on all six providers. The multiple-operand original and candidate both return 3 on C Tcl; Jim rejects the original with a wrong-argument-count error while the candidate returns 3. The authored collector retains the distinct argument counts. No single-operand receipt may infer permission from the C result equality.

## Scope

Exact retained 23 source pairs, C8.4.20/C8.5.19/C8.6.18/C9.0.4/C9.1.0 and Jim 0.84-9-g5bac7c9; finite subquestion cases: braced-boundary, multiple-operands. Authored argv collector is an independent command. Primary/resident state is sampled before its public byte getter; Unicode getter follows separately. No body/Normal/cache/observer/equivalence/Rust execution grant.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Counted Tcl_EvalEx source; separate Tcl_ReadChars on file UTF-8/auto then counted Tcl_EvalEx; independent authored collector and separate object-vector catch context. Version: tcl8.4 8.4.20.

The malformed candidate errors with extra characters after close-brace while its original returns a}b on all six providers. The multiple-operand original and candidate both return 3 on C Tcl; Jim rejects the original with a wrong-argument-count error while the candidate returns 3. The authored collector retains the distinct argument counts. No single-operand receipt may infer permission from the C result equality. Tcl 8.4 GetReturnOptions and four-argument catch options are API-unavailable in this probe.

### tcl8.5, tcl8.6

Status: `observed`. Channel: Counted Tcl_EvalEx source; separate Tcl_ReadChars on file UTF-8/auto then counted Tcl_EvalEx; independent authored collector and separate object-vector catch context. Version: tcl8.5 8.5.19; tcl8.6 8.6.18.

The malformed candidate errors with extra characters after close-brace while its original returns a}b on all six providers. The multiple-operand original and candidate both return 3 on C Tcl; Jim rejects the original with a wrong-argument-count error while the candidate returns 3. The authored collector retains the distinct argument counts. No single-operand receipt may infer permission from the C result equality.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Counted Tcl_EvalEx source; separate Tcl_ReadChars on file UTF-8/auto then counted Tcl_EvalEx; independent authored collector and separate object-vector catch context. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

The malformed candidate errors with extra characters after close-brace while its original returns a}b on all six providers. The multiple-operand original and candidate both return 3 on C Tcl; Jim rejects the original with a wrong-argument-count error while the candidate returns 3. The authored collector retains the distinct argument counts. No single-operand receipt may infer permission from the C result equality. C080 DocumentReadChars original/candidate records read=-1, empty result and exact queried error options; direct evaluation not attempted for those two inputs.

### jim

Status: `observed`. Channel: Counted Jim_NewStringObj/Jim_EvalObj; independent authored argv collector; separate Jim_EvalObjVector catch context. Version: jim 0.84-9-g5bac7c9.

The malformed candidate errors with extra characters after close-brace while its original returns a}b on all six providers. The multiple-operand original and candidate both return 3 on C Tcl; Jim rejects the original with a wrong-argument-count error while the candidate returns 3. The authored collector retains the distinct argument counts. No single-operand receipt may infer permission from the C result equality. Jim has counted Jim_EvalObj and separate object-vector catch only; C ReadChars is not tested and trace is guest unavailable.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-compiler/tests/data/native_brace_expression/v4/protocol](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/protocol)
- [rust/tcl-compiler/tests/data/native_brace_expression](../../../../rust/tcl-compiler/tests/data/native_brace_expression)
- [rust/tcl-compiler/tests/data/native_brace_expression/v1/protocol](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v1/protocol)
- [rust/tcl-compiler/tests/data/native_brace_expression/v3/protocol](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v3/protocol)
- [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture)
