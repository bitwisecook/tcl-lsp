# naming.expression.variable-reference-boundaries

Kind: `native-observation`

## Problem statement

A whole expression variable reference needs its selected public brace/parenthesis boundary. Literal dollar bytes, scalar unmatched parentheses and a combined closed array spelling cannot share a sigil-stripping or first-close rule across all releases.

## Question

Which public expr lookup completion/result is observed for each original unmatched scalar parenthesis, closed combined/separate array spelling, repeated parentheses, literal dollar name and first-close versus nested-brace reference?

## Conclusion

All six providers return original public values 11, 12, 21, 21, 22 and 41 for unmatched_scalar_open, scalar_parenthesis_tail, combined_closed_element, separate_closed_element, combined_multiple_parentheses and literal_dollar_scalar respectively. C8.4, C8.5, C8.6 and Jim first_close_reference succeeds with31, while nested_close_reference catches its exact expression diagnostic. C9.0 and C9.1 first_close_reference catches the exact missing-close-brace diagnostic and nested_close_reference succeeds with32. Every exact expression hex, caught code and whole result hex is retained in the eight EXPR rows/provider; META separately reports actual patchlevel. All six fresh CLI processes exit0 with empty stderr. The original binary format H* calls explicitly construct the declared variable names and expr arguments; these public results do not establish opaque source AST, object/header/table/cache identity, physical Native read/frame/entry, arbitrary Unicode/NUL or host compiler behaviour.

## Scope

One unchanged ASCII LF source-file CLI is executed independently on each pinned provider. Eight original labelled expression arguments and declared variable bytes, complete stdout/stderr/receipt and required executable/archive/header/build/source pins remain exact. The source contains its own catch and actual META report; no added eval/catch/version envelope is used. BIG-IP is not tested. The selected-reference-root source/API contract remains independent, and a linked Runtime whole-source comparison does not assert that it has executed. The linked shared Syntax diagnostic owner compares the exact retained C8.5/C8.6 nested-reference bareword-c message before a later rejected brace. Text/profile, actual expression grammar, original counted-byte and checked parser paths share typed lexical-prefix selection; an unfinished prefix cannot claim EOF before a later lexical error. These are source/API message projections and do not add a Native provider execution, private AST/object/cache observer or generic diagnostic-order theorem.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Exact ASCII LF source-file CLI. binary format H* explicitly constructs the declared variable name and reached expr argument. Public catch completion/result are sampled; no private parsed AST, table/header/cache, opaque source-byte, execution-entry or BIG-IP claim. Version: tcl8.4 8.4.20.

Whole eight original public rows: unmatched_scalar_open code0 result=11; scalar_parenthesis_tail code0 result=12; combined_closed_element code0 result=21; separate_closed_element code0 result=21; combined_multiple_parentheses code0 result=22; first_close_reference code0 result=31; nested_close_reference code1 result=syntax error in expression "${a{b}c}": extra tokens at end of expression; literal_dollar_scalar code0 result=41. META actual patchlevel is separate. All fixed values are reached expr arguments explicitly constructed by the source binary-format calls, not opaque analytical AST/header/cell observations.

### tcl8.5, tcl8.6

Status: `observed`. Channel: Exact ASCII LF source-file CLI. binary format H* explicitly constructs the declared variable name and reached expr argument. Public catch completion/result are sampled; no private parsed AST, table/header/cache, opaque source-byte, execution-entry or BIG-IP claim. Version: tcl8.5 8.5.19; tcl8.6 8.6.18.

Whole eight original public rows: unmatched_scalar_open code0 result=11; scalar_parenthesis_tail code0 result=12; combined_closed_element code0 result=21; separate_closed_element code0 result=21; combined_multiple_parentheses code0 result=22; first_close_reference code0 result=31; nested_close_reference code1 result=invalid bareword "c"
in expression "${a{b}c}";
should be "$c" or "{c}" or "c(...)" or ...; literal_dollar_scalar code0 result=41. META actual patchlevel is separate. All fixed values are reached expr arguments explicitly constructed by the source binary-format calls, not opaque analytical AST/header/cell observations.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Exact ASCII LF source-file CLI. binary format H* explicitly constructs the declared variable name and reached expr argument. Public catch completion/result are sampled; no private parsed AST, table/header/cache, opaque source-byte, execution-entry or BIG-IP claim. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

Whole eight original public rows: unmatched_scalar_open code0 result=11; scalar_parenthesis_tail code0 result=12; combined_closed_element code0 result=21; separate_closed_element code0 result=21; combined_multiple_parentheses code0 result=22; first_close_reference code1 result=missing close-brace for variable name
in expression "${a{b}"; nested_close_reference code0 result=32; literal_dollar_scalar code0 result=41. META actual patchlevel is separate. All fixed values are reached expr arguments explicitly constructed by the source binary-format calls, not opaque analytical AST/header/cell observations.

### jim

Status: `observed`. Channel: Exact ASCII LF source-file CLI. binary format H* explicitly constructs the declared variable name and reached expr argument. Public catch completion/result are sampled; no private parsed AST, table/header/cache, opaque source-byte, execution-entry or BIG-IP claim. Version: jim 0.84-9-g5bac7c9.

Whole eight original public rows: unmatched_scalar_open code0 result=11; scalar_parenthesis_tail code0 result=12; combined_closed_element code0 result=21; separate_closed_element code0 result=21; combined_multiple_parentheses code0 result=22; first_close_reference code0 result=31; nested_close_reference code1 result=syntax error in expression: "${a{b}c}"; literal_dollar_scalar code0 result=41. META actual patchlevel is separate. All fixed values are reached expr arguments explicitly constructed by the source binary-format calls, not opaque analytical AST/header/cell observations.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Consumer bindings

- [runtime/rust/src/expr_reference_tests.rs](../../../../runtime/rust/src/expr_reference_tests.rs), `original_expression_source_matches_48_public_reference_boundaries`: Compare untouched original source and eight public EXPR completion/result rows on each actual Runtime NativeCore/Jim input, separately from META; no AST/cache/header/frame grant.
- [rust/tcl-syntax/src/expr/syntax_error.rs](../../../../rust/tcl-syntax/src/expr/syntax_error.rs), `ExprSyntaxError::diagnose_tokens`: Share typed selected lexical-prefix diagnostic order across original text and counted-byte grammar ingress, with unfinished-prefix refusal; native public message observations remain independent.
- [runtime/rust/src/expr_reference_tests.rs](../../../../runtime/rust/src/expr_reference_tests.rs), `expr::reference_root_tests::original_expression_source_matches_48_public_reference_boundaries` (linked): Whole unchanged original probe source and all48 public EXPR completion/result windows compare under authentic NativeCore/Jim bootstrap with host refusal excluded. META version is excluded from the behavioural comparison; no private object/header/AST claim.
- [rust/tcl-syntax/src/expr/syntax_error.rs](../../../../rust/tcl-syntax/src/expr/syntax_error.rs), `expr::syntax_error::tests::original_reference_bareword_precedes_later_lexical_rejection` (linked): The retained native341 C8.5/C8.6 original nested reference supplies its exact public bareword-c message. Text/profile/explicit grammar, counted bytes and checked syntax paths compare that message; API ingress parity supplies no new Native process or private object/frame evidence.

## Evidence

- [rust/tcl-registry/tests/data/native_expression_reference_root341/8.4.20](../../../../rust/tcl-registry/tests/data/native_expression_reference_root341/8.4.20)
- [rust/tcl-registry/tests/data/native_expression_reference_root341/8.5.19](../../../../rust/tcl-registry/tests/data/native_expression_reference_root341/8.5.19)
- [rust/tcl-registry/tests/data/native_expression_reference_root341/8.6.18](../../../../rust/tcl-registry/tests/data/native_expression_reference_root341/8.6.18)
- [rust/tcl-registry/tests/data/native_expression_reference_root341/9.0.4](../../../../rust/tcl-registry/tests/data/native_expression_reference_root341/9.0.4)
- [rust/tcl-registry/tests/data/native_expression_reference_root341/9.1.0](../../../../rust/tcl-registry/tests/data/native_expression_reference_root341/9.1.0)
- [rust/tcl-registry/tests/data/native_expression_reference_root341](../../../../rust/tcl-registry/tests/data/native_expression_reference_root341)
- [rust/tcl-registry/tests/data/native_expression_reference_root341/jim](../../../../rust/tcl-registry/tests/data/native_expression_reference_root341/jim)
- [runtime/rust/src](../../../../runtime/rust/src)
