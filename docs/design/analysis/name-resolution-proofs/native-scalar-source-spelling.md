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

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This precise Rust source contract has no native execution receipt.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This precise Rust source contract has no native execution receipt.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This precise Rust source contract has no native execution receipt.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This precise Rust source contract has no native execution receipt.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This precise Rust source contract has no native execution receipt.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

This precise Rust source contract has no native execution receipt.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

This precise Rust source contract has no native execution receipt.

## Exact evidence

- `implementation` (implementation): [rust/tcl-syntax/src/naming/variable_spelling.rs](../../../../rust/tcl-syntax/src/naming/variable_spelling.rs). SHA-256 `a3234a2b787d837586191c4c684cb39596d343be60f0d129aa0d25446440cd4b`. Current Rust source and fixed meaningful coverage selectors; no Rust or native execution receipt.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/naming/variable_spelling.rs](../../../../rust/tcl-syntax/src/naming/variable_spelling.rs), `native_scalar_source_spelling`: Render and revalidate literal scalar source syntax through the shared byte ingress, source renderer and complete variable scanner without cell or edit authority.
- [rust/tcl-syntax/src/naming/variable_spelling.rs](../../../../rust/tcl-syntax/src/naming/variable_spelling.rs), `naming::variable_spelling::tests::scalar_source_proposals_preserve_native_operand_and_complete_reference` (linked): Six selected recipes preserve café as a complete braced reference, reject independently invalid scalar/source cases and keep raw-zero C/Jim purpose differences.

A named test is a coverage binding, not a claim that it executed.

## Replay

The linked Rust selectors check this implementation contract. No native interpreter executes this editor or source-rendering owner, and no test execution receipt is asserted.
