# naming.expression.braced-handler-and-observer-boundaries

Kind: `native-observation`

## Problem statement

A changed source word cannot select the handler or establish observer closure; user implementations may inspect their caller source.

## Question

What do the retained renamed, shadowed, execution-traced and caller-body-reading handler controls actually expose?

## Conclusion

The renamed builtin returns 3 in both variants and the simple shadow procedure returns the same argument list. C execution-trace callbacks receive the same canonical command argument expr {1 + 2} in both variants; the predicted difference in original source spelling is not observed there. Jim rejects trace as unavailable. A user procedure reading info body p returns expr "1 + 2" versus expr {1 + 2} on all six providers, demonstrating an actual source-observation difference. None of these controls authenticates an optimiser-selected handler or closes arbitrary observers.

Two linked compiler source controls preserve independent normality, original handler/argv/config and source-reflection obligations before literal expression bracing. Error-producing expressions and observable retained procedure/frame spelling withhold a rewrite even when a different literal calculation is finite and normal.

## Scope

Exact retained 23 source pairs, C8.4.20/C8.5.19/C8.6.18/C9.0.4/C9.1.0 and Jim 0.84-9-g5bac7c9; finite subquestion cases: shadowed-handler, renamed-builtin, execution-observer, execution-observer-source, shadowed-handler-body. Authored argv collector is an independent command. Primary/resident state is sampled before its public byte getter; Unicode getter follows separately. No body/Normal/cache/observer/equivalence/Rust execution grant.

These conditional source refusal controls neither replay nor replace original native errorInfo/options or observer measurements. They issue no handler/frame admission, completed native expression result, general rewrite equivalence or assertion outcome. All original versioned provider answers and evidence remain unchanged.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Counted Tcl_EvalEx source; separate Tcl_ReadChars on file UTF-8/auto then counted Tcl_EvalEx; independent authored collector and separate object-vector catch context. Version: tcl8.4 8.4.20.

The renamed builtin returns 3 in both variants and the simple shadow procedure returns the same argument list. C execution-trace callbacks receive the same canonical command argument expr {1 + 2} in both variants; the predicted difference in original source spelling is not observed there. Jim rejects trace as unavailable. A user procedure reading info body p returns expr "1 + 2" versus expr {1 + 2} on all six providers, demonstrating an actual source-observation difference. None of these controls authenticates an optimiser-selected handler or closes arbitrary observers. Tcl 8.4 GetReturnOptions and four-argument catch options are API-unavailable in this probe.

### tcl8.5, tcl8.6

Status: `observed`. Channel: Counted Tcl_EvalEx source; separate Tcl_ReadChars on file UTF-8/auto then counted Tcl_EvalEx; independent authored collector and separate object-vector catch context. Version: tcl8.5 8.5.19; tcl8.6 8.6.18.

The renamed builtin returns 3 in both variants and the simple shadow procedure returns the same argument list. C execution-trace callbacks receive the same canonical command argument expr {1 + 2} in both variants; the predicted difference in original source spelling is not observed there. Jim rejects trace as unavailable. A user procedure reading info body p returns expr "1 + 2" versus expr {1 + 2} on all six providers, demonstrating an actual source-observation difference. None of these controls authenticates an optimiser-selected handler or closes arbitrary observers.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Counted Tcl_EvalEx source; separate Tcl_ReadChars on file UTF-8/auto then counted Tcl_EvalEx; independent authored collector and separate object-vector catch context. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

The renamed builtin returns 3 in both variants and the simple shadow procedure returns the same argument list. C execution-trace callbacks receive the same canonical command argument expr {1 + 2} in both variants; the predicted difference in original source spelling is not observed there. Jim rejects trace as unavailable. A user procedure reading info body p returns expr "1 + 2" versus expr {1 + 2} on all six providers, demonstrating an actual source-observation difference. None of these controls authenticates an optimiser-selected handler or closes arbitrary observers. C080 DocumentReadChars original/candidate records read=-1, empty result and exact queried error options; direct evaluation not attempted for those two inputs.

### jim

Status: `observed`. Channel: Counted Jim_NewStringObj/Jim_EvalObj; independent authored argv collector; separate Jim_EvalObjVector catch context. Version: jim 0.84-9-g5bac7c9.

The renamed builtin returns 3 in both variants and the simple shadow procedure returns the same argument list. C execution-trace callbacks receive the same canonical command argument expr {1 + 2} in both variants; the predicted difference in original source spelling is not observed there. Jim rejects trace as unavailable. A user procedure reading info body p returns expr "1 + 2" versus expr {1 + 2} on all six providers, demonstrating an actual source-observation difference. None of these controls authenticates an optimiser-selected handler or closes arbitrary observers. Jim has counted Jim_EvalObj and separate object-vector catch only; C ReadChars is not tested and trace is guest unavailable.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

No implementation binding is claimed by this observation record.

- [rust/tcl-compiler/src/command_binding/original_literal_argv_rewrite.rs](../../../../rust/tcl-compiler/src/command_binding/original_literal_argv_rewrite.rs), `SourceInvocationBinding::original_literal_expression_bracing`: Require original selected handler/config/argv and independent argument, compiler-name, finite normality and isolated source-reflection premises before issuing a bounded literal bracing receipt.
- [rust/tcl-compiler/src/command_binding/original_literal_argv_rewrite.rs](../../../../rust/tcl-compiler/src/command_binding/original_literal_argv_rewrite.rs), `finite_native_literal_result`: Compute the bounded selected finite expression normality obligation independently of source reflection or native error-context equivalence.
- [rust/tcl-compiler/src/command_binding/original_literal_argv_rewrite.rs](../../../../rust/tcl-compiler/src/command_binding/original_literal_argv_rewrite.rs), `command_binding::original_literal_argv_rewrite::tests::original_literal_expression_bracing_declines_substitution_functions_and_handler_changes` (linked): The source bracing issuer refuses error-producing or empty expressions, substitutions/functions, procedure or frame observers, changed handler/unknown mutation and unsupported argv shape. This is refusal coverage independent of the original Native error/observer captures.
- [rust/tcl-compiler/src/command_binding/original_literal_argv_rewrite.rs](../../../../rust/tcl-compiler/src/command_binding/original_literal_argv_rewrite.rs), `command_binding::original_literal_argv_rewrite::tests::original_literal_bracing_normality_and_source_reflection_are_independent` (linked): A finite selected native-grammar calculation may establish 1+2 normality while 1/0 is non-normal; normality alone does not admit spelling changes observed by retained procedure/frame source or error context. Source rewrite receipts refuse those cases under each authored release grammar.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Evidence

- [rust/tcl-compiler/tests/data/native_brace_expression/v4/protocol](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/protocol)
- [rust/tcl-compiler/tests/data/native_brace_expression](../../../../rust/tcl-compiler/tests/data/native_brace_expression)
- [rust/tcl-compiler/tests/data/native_brace_expression/v1/protocol](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v1/protocol)
- [rust/tcl-compiler/tests/data/native_brace_expression/v3/protocol](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v3/protocol)
- [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture)
