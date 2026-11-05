# naming.source.native-scalar-source-spelling

Kind: `implementation-contract`

## Problem statement

A Unicode reporting label or bare variable spelling cannot reproduce every selected native scalar operand. Qualification, array syntax, C-string clipping and incomplete lexical references can change which name a proposed source word denotes.

## Question

Can a pure source-rendering owner propose one literal name word and complete scalar reference while reproducing the exact original bytes under the selected channel, scanner configuration and native recipe?

## Conclusion

native_scalar_source_spelling accepts only an unchanged unqualified combined scalar operand, then uses the shared native literal source renderer and variable scanner to validate a complete braced reference and exact reproduced bytes. It declines qualification, array interpretation, clipping, incomplete references and source text the selected recipe cannot reproduce. The returned name word and reference supply source spelling only. Availability, cell identity, value, observers, lifetime, command selection and edit permission remain separate obligations.

## Scope

Pure Rust source proposal contract under the complete supplied source channel, LexerConfig and native name protocol. Fixed coverage checks literal café and complete ${café} for all five C recipes and Jim; rejects qualified, array, closing-brace and unrepresentable surrogate examples; raw zero follows each independently selected C/Jim purpose. Existing native braced-Unicode controls measure those exact source scripts separately and do not certify arbitrary proposals or edit permission. All seven empirical provider answers are not-tested for this Rust invariant.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This precise Rust source contract has no native execution receipt.

## Consumer bindings

- [rust/tcl-syntax/src/naming/variable_spelling.rs](../../../../rust/tcl-syntax/src/naming/variable_spelling.rs), `native_scalar_source_spelling`: Render and revalidate literal scalar source syntax through the shared byte ingress, source renderer and complete variable scanner without cell or edit authority.
- [rust/tcl-syntax/src/naming/variable_spelling.rs](../../../../rust/tcl-syntax/src/naming/variable_spelling.rs), `naming::variable_spelling::tests::scalar_source_proposals_preserve_native_operand_and_complete_reference` (linked): Six selected recipes preserve café as a complete braced reference, reject independently invalid scalar/source cases and keep raw-zero C/Jim purpose differences.

## Evidence

- [rust/tcl-syntax/src/naming/variable_spelling.rs](../../../../rust/tcl-syntax/src/naming/variable_spelling.rs)
