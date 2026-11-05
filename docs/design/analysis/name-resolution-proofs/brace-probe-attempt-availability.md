# naming.expression.brace-probe-attempt-availability

Kind: `native-observation`

## Problem statement

Probe build failures and an input-stage early exit must remain distinguishable from provider semantic answers and from the successful independently pinned capture.

## Question

Which original brace-probe attempts actually compiled/executed, and where did harness or input-stage failures prevent later observations?

## Conclusion

Attempt v1 executes C8.4/C8.5/C8.6 then fails C9.0 compilation because its Tcl_Size callback was passed to the legacy int-count command constructor. Attempt v2 fails in Python output-directory creation before any native compile. Attempt v3 executes C8.4/C8.5/C8.6 and stops C9.0 with process code 3 on the raw C080 ReadChars input. Attempt v4 uses Tcl_CreateObjCommand2 on C9 and retains ReadChars failure state plus EVAL_NOT_ATTEMPTED, then continues; all six v4 compile/process codes are zero. These are probe/process availability observations, not guest unsupported-command or optimiser claims.

## Scope

Exact retained 23 source pairs, C8.4.20/C8.5.19/C8.6.18/C9.0.4/C9.1.0 and Jim 0.84-9-g5bac7c9; finite subquestion cases: preserved harness/process attempts. Authored argv collector is an independent command. Primary/resident state is sampled before its public byte getter; Unicode getter follows separately. No body/Normal/cache/observer/equivalence/Rust execution grant.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Counted Tcl_EvalEx source; separate Tcl_ReadChars on file UTF-8/auto then counted Tcl_EvalEx; independent authored collector and separate object-vector catch context. Version: tcl8.4 8.4.20.

Attempt v1 executes C8.4/C8.5/C8.6 then fails C9.0 compilation because its Tcl_Size callback was passed to the legacy int-count command constructor. Attempt v2 fails in Python output-directory creation before any native compile. Attempt v3 executes C8.4/C8.5/C8.6 and stops C9.0 with process code 3 on the raw C080 ReadChars input. Attempt v4 uses Tcl_CreateObjCommand2 on C9 and retains ReadChars failure state plus EVAL_NOT_ATTEMPTED, then continues; all six v4 compile/process codes are zero. These are probe/process availability observations, not guest unsupported-command or optimiser claims. Tcl 8.4 GetReturnOptions and four-argument catch options are API-unavailable in this probe.

### tcl8.5, tcl8.6

Status: `observed`. Channel: Counted Tcl_EvalEx source; separate Tcl_ReadChars on file UTF-8/auto then counted Tcl_EvalEx; independent authored collector and separate object-vector catch context. Version: tcl8.5 8.5.19; tcl8.6 8.6.18.

Attempt v1 executes C8.4/C8.5/C8.6 then fails C9.0 compilation because its Tcl_Size callback was passed to the legacy int-count command constructor. Attempt v2 fails in Python output-directory creation before any native compile. Attempt v3 executes C8.4/C8.5/C8.6 and stops C9.0 with process code 3 on the raw C080 ReadChars input. Attempt v4 uses Tcl_CreateObjCommand2 on C9 and retains ReadChars failure state plus EVAL_NOT_ATTEMPTED, then continues; all six v4 compile/process codes are zero. These are probe/process availability observations, not guest unsupported-command or optimiser claims.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Counted Tcl_EvalEx source; separate Tcl_ReadChars on file UTF-8/auto then counted Tcl_EvalEx; independent authored collector and separate object-vector catch context. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

Attempt v1 executes C8.4/C8.5/C8.6 then fails C9.0 compilation because its Tcl_Size callback was passed to the legacy int-count command constructor. Attempt v2 fails in Python output-directory creation before any native compile. Attempt v3 executes C8.4/C8.5/C8.6 and stops C9.0 with process code 3 on the raw C080 ReadChars input. Attempt v4 uses Tcl_CreateObjCommand2 on C9 and retains ReadChars failure state plus EVAL_NOT_ATTEMPTED, then continues; all six v4 compile/process codes are zero. These are probe/process availability observations, not guest unsupported-command or optimiser claims. C080 DocumentReadChars original/candidate records read=-1, empty result and exact queried error options; direct evaluation not attempted for those two inputs.

### jim

Status: `observed`. Channel: Counted Jim_NewStringObj/Jim_EvalObj; independent authored argv collector; separate Jim_EvalObjVector catch context. Version: jim 0.84-9-g5bac7c9.

Attempt v1 executes C8.4/C8.5/C8.6 then fails C9.0 compilation because its Tcl_Size callback was passed to the legacy int-count command constructor. Attempt v2 fails in Python output-directory creation before any native compile. Attempt v3 executes C8.4/C8.5/C8.6 and stops C9.0 with process code 3 on the raw C080 ReadChars input. Attempt v4 uses Tcl_CreateObjCommand2 on C9 and retains ReadChars failure state plus EVAL_NOT_ATTEMPTED, then continues; all six v4 compile/process codes are zero. These are probe/process availability observations, not guest unsupported-command or optimiser claims. Jim has counted Jim_EvalObj and separate object-vector catch only; C ReadChars is not tested and trace is guest unavailable.

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
- [rust/tcl-compiler/tests/data/native_brace_expression/v2/protocol](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v2/protocol)
- [rust/tcl-compiler/tests/data/native_brace_expression/v1/capture/9.0.4](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v1/capture/9.0.4)
- [rust/tcl-compiler/tests/data/native_brace_expression/v2](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v2)
