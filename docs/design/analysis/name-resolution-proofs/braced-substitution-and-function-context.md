# naming.expression.braced-substitution-and-function-context

Kind: `native-observation`

## Problem statement

Bracing moves variable/script substitution into expression evaluation, while function lookup remains an independent execution dependency.

## Question

What do the retained variable-read, script callback and registered-function controls measure in the original and braced evaluation contexts?

## Conclusion

These finite direct controls have matching completion/result, and the script callback increments ticks once in both original expression executions. The separately authored collector increments ticks for the quoted original but leaves it zero for the braced candidate; it therefore does not substitute for measuring expression execution. The observed abs function result supplies no independent function resolver/observer closure. General substitution-bearing or function-bearing rewrite permission is not inferred.

## Scope

Exact retained 23 source pairs, C8.4.20/C8.5.19/C8.6.18/C9.0.4/C9.1.0 and Jim 0.84-9-g5bac7c9; finite subquestion cases: variable-substitution, script-substitution, registered-function. Authored argv collector is an independent command. Primary/resident state is sampled before its public byte getter; Unicode getter follows separately. No body/Normal/cache/observer/equivalence/Rust execution grant.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Counted Tcl_EvalEx source; separate Tcl_ReadChars on file UTF-8/auto then counted Tcl_EvalEx; independent authored collector and separate object-vector catch context. Version: tcl8.4 8.4.20.

These finite direct controls have matching completion/result, and the script callback increments ticks once in both original expression executions. The separately authored collector increments ticks for the quoted original but leaves it zero for the braced candidate; it therefore does not substitute for measuring expression execution. The observed abs function result supplies no independent function resolver/observer closure. General substitution-bearing or function-bearing rewrite permission is not inferred. Tcl 8.4 GetReturnOptions and four-argument catch options are API-unavailable in this probe.

### tcl8.5, tcl8.6

Status: `observed`. Channel: Counted Tcl_EvalEx source; separate Tcl_ReadChars on file UTF-8/auto then counted Tcl_EvalEx; independent authored collector and separate object-vector catch context. Version: tcl8.5 8.5.19; tcl8.6 8.6.18.

These finite direct controls have matching completion/result, and the script callback increments ticks once in both original expression executions. The separately authored collector increments ticks for the quoted original but leaves it zero for the braced candidate; it therefore does not substitute for measuring expression execution. The observed abs function result supplies no independent function resolver/observer closure. General substitution-bearing or function-bearing rewrite permission is not inferred.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Counted Tcl_EvalEx source; separate Tcl_ReadChars on file UTF-8/auto then counted Tcl_EvalEx; independent authored collector and separate object-vector catch context. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

These finite direct controls have matching completion/result, and the script callback increments ticks once in both original expression executions. The separately authored collector increments ticks for the quoted original but leaves it zero for the braced candidate; it therefore does not substitute for measuring expression execution. The observed abs function result supplies no independent function resolver/observer closure. General substitution-bearing or function-bearing rewrite permission is not inferred. C080 DocumentReadChars original/candidate records read=-1, empty result and exact queried error options; direct evaluation not attempted for those two inputs.

### jim

Status: `observed`. Channel: Counted Jim_NewStringObj/Jim_EvalObj; independent authored argv collector; separate Jim_EvalObjVector catch context. Version: jim 0.84-9-g5bac7c9.

These finite direct controls have matching completion/result, and the script callback increments ticks once in both original expression executions. The separately authored collector increments ticks for the quoted original but leaves it zero for the braced candidate; it therefore does not substitute for measuring expression execution. The observed abs function result supplies no independent function resolver/observer closure. General substitution-bearing or function-bearing rewrite permission is not inferred. Jim has counted Jim_EvalObj and separate object-vector catch only; C ReadChars is not tested and trace is guest unavailable.

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
