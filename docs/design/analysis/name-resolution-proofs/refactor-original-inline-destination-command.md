# naming.refactor.original-inline-destination-command

Kind: `implementation-contract`

## Problem statement

A body command can be a stock builtin in its declaring namespace while the same proposed spelling resolves to a user procedure in the caller namespace. Moving the body without rechecking its destination changes the implementation. The body occurrence and caller proposal are different purposes and cannot share an invented original head.

## Question

Does inlining independently validate each proposed body builtin at the actual call destination?

## Conclusion

The bounded inline subset admits only an exact original body head matching the selected authored ASCII Registry identifier, then independently checks that implementation at the actual call point through the shared proposed Registry command facade. Shadowed, replaced, observed, qualified or opaque unsupported proposals decline. The facade supplies conditional command selection only, without insertion, frame relocation, store or Native preparation authority.

## Scope

Native single-command procedure inline destination check. General byte command proposals, alias body heads and independent expression-function relocation obligations are outside this facade.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This Rust consumer contract has no native execution receipt.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/declaration_lookup.rs](../../../../rust/tcl-compiler/src/command_binding/declaration_lookup.rs), `original_proposed_registry_commands`: Validate proposed authored Registry spellings against original point snapshots and observer/operand closure.
- [rust/tcl-lsp-core/src/refactor/inline_proc.rs](../../../../rust/tcl-lsp-core/src/refactor/inline_proc.rs), `original_inline_body`: Keep original body naming and caller proposal selection separate.
- [rust/tcl-lsp-core/src/refactor/inline_proc.rs](../../../../rust/tcl-lsp-core/src/refactor/inline_proc.rs), `refactor::inline_proc::original_inline_destination_tests::original_inline_refuses_a_body_builtin_shadowed_at_the_call_destination` (linked): Caller namespace shadowing refuses the rewrite with no edits.

## Evidence

- [rust/tcl-compiler/src/command_binding/declaration_lookup.rs](../../../../rust/tcl-compiler/src/command_binding/declaration_lookup.rs)
- [rust/tcl-lsp-core/src/refactor/inline_proc.rs](../../../../rust/tcl-lsp-core/src/refactor/inline_proc.rs)
