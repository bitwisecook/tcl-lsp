# naming.expression.scalar-reference-constant-source-projection

Kind: `implementation-contract`

## Problem statement

A source expression constant substitution must retain the exact selected scalar name. A same-prefix constant or array root cannot stand for an unmatched-parenthesis/literal-dollar scalar, and an array reference needs independent selected element-key evidence.

## Question

Which whole expression scalar reference may consult a retained source constant under actual grammar, and when must combined/indexed array syntax or missing execution operand evidence remain unchanged?

## Conclusion

The source emitter uses substitute_expr_constants_in_context with the actual ExprParseContext lexer grammar, operator grammar and hosted word axis. scalar_expression_reference_name checks the whole original variable token with that selected grammar and the shared Syntax variable_reference_root_bytes owner. Only complete scalar references join the supplied value-constant map: unmatched open parentheses, scalar tails, literal dollar and Unicode names keep their exact units. Separate or combined array references refuse because these scalar maps carry no selected element-key receipt. Selected Jim bare Unicode syntax and C ASCII syntax remain distinct even when the Jim point has no catalogue DialectProfile. The profile adapter explicitly creates its own chosen source context; it cannot replace a supplied actual context. The emitted source constants are chosen symbolic inputs, not executed Native cells or reads. Missing execution operand evidence keeps the original execution expression unchanged; no Native input/header/cache/frame, current storage, SSA runtime contents or erasure authority follows.

## Scope

Three marked fixed source/API controls compare conflicting scalar roots versus unmatched/tailed full names, literal-dollar and Unicode constants across actual C5/Jim expression points, array/malformed refusal and selected brace-close grammar. The actual-context discriminator retains Jim bare Unicode syntax without a catalogue profile and distinguishes the C9 source token grammar. Chosen constants and source output establish no Native runtime variable values or read success. The independent native expression public-value question covers its own fixed originals; all seven providers remain not tested for this source-emitter API.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

## Consumer bindings

- [rust/tcl-compiler/src/optimiser/helpers/expr_simplify.rs](../../../../rust/tcl-compiler/src/optimiser/helpers/expr_simplify.rs), `scalar_expression_reference_name`: Require authentic whole selected scalar source/grammar, refusing array references without independent element-key evidence.
- [rust/tcl-compiler/src/optimiser/helpers/expr_simplify.rs](../../../../rust/tcl-compiler/src/optimiser/helpers/expr_simplify.rs), `substitute_expr_constants`: Consult exact selected source scalar constants while preserving missing execution operands and unknown references unchanged.
- [rust/tcl-compiler/src/optimiser/helpers/expr_simplify.rs](../../../../rust/tcl-compiler/src/optimiser/helpers/expr_simplify.rs), `substitute_expr_constants_in_context`: Consume the actual complete expression grammar while retaining whole scalar-reference source and separate execution erasure obligations.
- [rust/tcl-compiler/src/optimiser/helpers/expr_simplify.rs](../../../../rust/tcl-compiler/src/optimiser/helpers/expr_simplify.rs), `optimiser::helpers::expr_simplify::tests::scalar_constant_source_substitution_keeps_exact_literal_names_and_array_refusal` (linked): Actual C5/Jim expression points retain complete scalar unmatched/tail/dollar/Unicode names against conflicting root constants; arrays and malformed source refuse, and absent execution operands preserve the original expression. Supplied constants are symbolic source inputs, not Native cell observations.
- [rust/tcl-compiler/src/optimiser/helpers/expr_simplify.rs](../../../../rust/tcl-compiler/src/optimiser/helpers/expr_simplify.rs), `optimiser::helpers::expr_simplify::tests::scalar_constant_reference_selection_uses_actual_brace_close_grammar` (linked): Actual selected C8/Jim first-close and C9 nested-close grammar determine the scalar source name, with malformed/compound or array reference refusal.
- [rust/tcl-compiler/src/optimiser/helpers/expr_simplify.rs](../../../../rust/tcl-compiler/src/optimiser/helpers/expr_simplify.rs), `optimiser::helpers::expr_simplify::tests::scalar_constant_context_uses_jim_variable_grammar_without_catalogue_profile` (linked): Actual Jim ExprParseContext selects bare Unicode scalar syntax without a catalogue profile; C9 ASCII variable grammar keeps the same Unicode text unprojected. Source constant selection grants no Native read/value or native341 Unicode result.

## Evidence

- [rust/tcl-compiler/src/optimiser/helpers/expr_simplify.rs](../../../../rust/tcl-compiler/src/optimiser/helpers/expr_simplify.rs)
