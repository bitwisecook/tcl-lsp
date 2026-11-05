# naming.expression.braced-static-literal-completion

Kind: `native-observation`

## Problem statement

Changing the original word delimiters can change the value supplied to expression evaluation; successful literal controls must be measured in each actual source channel.

## Question

For the seven retained static literal pairs, what completion and result does each actual provider return before and after bracing?

## Conclusion

Each retained pair returns matching OK code and result in all six counted-native providers and in every measured C ReadChars channel. The separate authored collector also records matching operand bytes within each pair/channel. This finite comparison grants no optimiser equivalence, handler identity, object lifetime, observer closure or permission for unmeasured literals.

## Scope

Exact retained 23 source pairs, C8.4.20/C8.5.19/C8.6.18/C9.0.4/C9.1.0 and Jim 0.84-9-g5bac7c9; finite subquestion cases: arithmetic-literal, bare-numeric-literal, numeric-source-escape, string-literal-bmp, string-literal-astral, continuation-decoded, continuation-written-verbatim. Authored argv collector is an independent command. Primary/resident state is sampled before its public byte getter; Unicode getter follows separately. No body/Normal/cache/observer/equivalence/Rust execution grant.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Counted Tcl_EvalEx source; separate Tcl_ReadChars on file UTF-8/auto then counted Tcl_EvalEx; independent authored collector and separate object-vector catch context. Version: tcl8.4 8.4.20.

Each retained pair returns matching OK code and result in all six counted-native providers and in every measured C ReadChars channel. The separate authored collector also records matching operand bytes within each pair/channel. This finite comparison grants no optimiser equivalence, handler identity, object lifetime, observer closure or permission for unmeasured literals. Tcl 8.4 GetReturnOptions and four-argument catch options are API-unavailable in this probe.

### tcl8.5, tcl8.6

Status: `observed`. Channel: Counted Tcl_EvalEx source; separate Tcl_ReadChars on file UTF-8/auto then counted Tcl_EvalEx; independent authored collector and separate object-vector catch context. Version: tcl8.5 8.5.19; tcl8.6 8.6.18.

Each retained pair returns matching OK code and result in all six counted-native providers and in every measured C ReadChars channel. The separate authored collector also records matching operand bytes within each pair/channel. This finite comparison grants no optimiser equivalence, handler identity, object lifetime, observer closure or permission for unmeasured literals.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Counted Tcl_EvalEx source; separate Tcl_ReadChars on file UTF-8/auto then counted Tcl_EvalEx; independent authored collector and separate object-vector catch context. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

Each retained pair returns matching OK code and result in all six counted-native providers and in every measured C ReadChars channel. The separate authored collector also records matching operand bytes within each pair/channel. This finite comparison grants no optimiser equivalence, handler identity, object lifetime, observer closure or permission for unmeasured literals. C080 DocumentReadChars original/candidate records read=-1, empty result and exact queried error options; direct evaluation not attempted for those two inputs.

### jim

Status: `observed`. Channel: Counted Jim_NewStringObj/Jim_EvalObj; independent authored argv collector; separate Jim_EvalObjVector catch context. Version: jim 0.84-9-g5bac7c9.

Each retained pair returns matching OK code and result in all six counted-native providers and in every measured C ReadChars channel. The separate authored collector also records matching operand bytes within each pair/channel. This finite comparison grants no optimiser equivalence, handler identity, object lifetime, observer closure or permission for unmeasured literals. Jim has counted Jim_EvalObj and separate object-vector catch only; C ReadChars is not tested and trace is guest unavailable.

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
