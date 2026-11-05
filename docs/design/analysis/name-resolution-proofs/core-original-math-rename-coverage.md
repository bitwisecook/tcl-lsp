# naming.core.original-math-rename-coverage

Kind: `implementation-contract`

## Problem statement

Readonly expression-function references do not carry original script command words. A procedure rename that edits its declaration and script calls but ignores genuine expression identifiers emits a partial rename; wrapping NAME( in synthetic command syntax cannot authenticate those identifiers.

## Question

Does Native procedure rename refuse readonly expression identifier references without a separate identifier edit owner?

## Conclusion

The shared math reference query validates complete independent consumer and declaration source/configuration ownership and matches the retained actual function allocation to the canonical procedure declaration. Core rename checks this query before constructing edits and refuses the whole Native procedure rename when such readonly identifiers exist or required correspondence is unavailable. Starting at an original function cursor also refuses. No command-head word, source edit or expression identifier permission is synthesized from a readonly function reference. The server applies the same coverage query to each independent participating document before issuing workspace edits; a readonly expression use refuses the complete rename.

## Scope

Core atomic procedure rename coverage, the reusable per-document query for independent workspace owners, and Server complete-participant coverage before a workspace edit set. Readonly navigation and reference inventory are separate purposes; this contract supplies no writable expression identifier geometry.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This Rust consumer contract has no native execution receipt.

## Consumer bindings

- [rust/tcl-lsp-core/src/math_function_symbol.rs](../../../../rust/tcl-lsp-core/src/math_function_symbol.rs), `procedure_references_in`: Retain readonly function references under independent complete current consumer and declaration owners.
- [rust/tcl-lsp-core/src/rename.rs](../../../../rust/tcl-lsp-core/src/rename.rs), `readonly_math_rename_refusal`: Refuse partial script-word rename plans when expression identifier edit ownership is absent.
- [rust/tcl-lsp-core/src/original_declaration.rs](../../../../rust/tcl-lsp-core/src/original_declaration.rs), `math_function_targets_declaration_in`: Match the genuine current function reference to the independently retained declaration allocation.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `Backend::original_procedure_expression_coverage_is_editable`: Inspect the complete current participating document inventory and canonical procedure allocation before the server issues any workspace rename edits; readonly expression references or unavailable correspondence refuse the whole edit set.
- [rust/tcl-lsp-core/src/math_function_symbol.rs](../../../../rust/tcl-lsp-core/src/math_function_symbol.rs), `math_function_symbol::tests::original_math_references_cannot_supply_a_command_word_rename` (linked): With the procedure reporting map cleared, the source-defined control checks that a readonly function reference has no original command-head input, that rename refuses a partial declaration/script-call edit, and that displaced consumer source declines correspondence. No test execution or native provider result is attached.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `tests::original_workspace_procedure_rename_requires_expression_edit_coverage` (linked): An independently retained current provider permits the complete command-only source coverage; adding a separate current Tcl document with expr {local(1)} withdraws editable coverage, and the public server rename returns no edit set. The readonly expression identifier supplies neither a command word nor an identifier edit receipt.

## Evidence

- [rust/tcl-lsp-core/src/math_function_symbol.rs](../../../../rust/tcl-lsp-core/src/math_function_symbol.rs)
- [rust/tcl-lsp-core/src/rename.rs](../../../../rust/tcl-lsp-core/src/rename.rs)
- [rust/tcl-lsp-core/src/original_declaration.rs](../../../../rust/tcl-lsp-core/src/original_declaration.rs)
