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

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This Rust consumer contract has no native execution receipt.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This Rust consumer contract has no native execution receipt.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This Rust consumer contract has no native execution receipt.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This Rust consumer contract has no native execution receipt.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This Rust consumer contract has no native execution receipt.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

This Rust consumer contract has no native execution receipt.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

This Rust consumer contract has no native execution receipt.

## Exact evidence

- `implementation-0` (implementation): [rust/tcl-lsp-core/src/math_function_symbol.rs](../../../../rust/tcl-lsp-core/src/math_function_symbol.rs). SHA-256 `49039bfa844bba74fa181031a50f0f59a3144d61ddb9656999d97d1e2fa58726`. Current shared owner, consumer and fixed authored assertions. No Rust or native execution result is implied.
- `implementation-1` (implementation): [rust/tcl-lsp-core/src/rename.rs](../../../../rust/tcl-lsp-core/src/rename.rs). SHA-256 `5a81bb27113af62b4a90ef9e0dc3b1498ff28990b9d896286b2f1e08e9c4bbb6`. Current shared owner, consumer and fixed authored assertions. No Rust or native execution result is implied.
- `implementation-2` (implementation): [rust/tcl-lsp-core/src/original_declaration.rs](../../../../rust/tcl-lsp-core/src/original_declaration.rs). SHA-256 `32546953bd0a43636229eb9e7aa4646d897be43e9125ae3891e895167ae52172`. Current shared owner, consumer and fixed authored assertions. No Rust or native execution result is implied.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/math_function_symbol.rs](../../../../rust/tcl-lsp-core/src/math_function_symbol.rs), `procedure_references_in`: Retain readonly function references under independent complete current consumer and declaration owners.
- [rust/tcl-lsp-core/src/rename.rs](../../../../rust/tcl-lsp-core/src/rename.rs), `readonly_math_rename_refusal`: Refuse partial script-word rename plans when expression identifier edit ownership is absent.
- [rust/tcl-lsp-core/src/original_declaration.rs](../../../../rust/tcl-lsp-core/src/original_declaration.rs), `math_function_targets_declaration_in`: Match the genuine current function reference to the independently retained declaration allocation.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `Backend::original_procedure_expression_coverage_is_editable`: Inspect the complete current participating document inventory and canonical procedure allocation before the server issues any workspace rename edits; readonly expression references or unavailable correspondence refuse the whole edit set.
- [rust/tcl-lsp-core/src/math_function_symbol.rs](../../../../rust/tcl-lsp-core/src/math_function_symbol.rs), `math_function_symbol::tests::original_math_references_cannot_supply_a_command_word_rename` (linked): A genuine readonly function reference has no command-head input and prevents partial declaration/script-call edits even with reporting maps cleared; stale correspondence declines.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `tests::original_workspace_procedure_rename_requires_expression_edit_coverage` (linked): An independently retained current provider permits the complete command-only source coverage; adding a separate current Tcl document with expr {local(1)} withdraws editable coverage, and the public server rename returns no edit set. The readonly expression identifier supplies neither a command word nor an identifier edit receipt.

A named test is a coverage binding, not a claim that it executed.

## Replay

The named Rust selectors exercise the scoped implementation premises. No Rust execution receipt or native provider observation is attached. Readonly expression references supply no identifier edit permission, and missing required document correspondence refuses the edit set.
