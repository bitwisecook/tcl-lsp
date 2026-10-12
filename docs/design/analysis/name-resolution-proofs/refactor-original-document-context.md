# naming.refactor.original-document-context

Kind: `implementation-contract`

## Problem statement

A code action can resolve its Registry from a mutable dialect label or add F5 commands to a non-F5 source. Reconstructing a lexer from a profile string also loses actual overrides. The proposed transformation must use the document-owned Registry and full original parser configuration; changing a reporting label cannot supply either.

## Question

Do refactor actions use the retained document Registry, full configuration and current original image?

## Conclusion

Code-action refactoring consumes the analysis resolved Registry and full body configuration after complete source-image correspondence. Reporting dialect changes do not change that selection. Counterfactual F5 metadata, missing configuration and displaced source cannot supply a transformation context.

## Scope

Core code-action entry contract for current source analysis. Explicit F5 authoring advice remains separate from C/Jim native engine authority and from individual transformation permissions.

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

- `implementation-0` (implementation): [rust/tcl-lsp-core/src/code_actions.rs](../../../../rust/tcl-lsp-core/src/code_actions.rs). SHA-256 `9791cc7d94fa2847c053ad6cf02696be82c95467865e735f7350b67f470df2e8`. Current shared owner, consumer and fixed validation assertions. No executed Rust or native result is implied.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/code_actions.rs](../../../../rust/tcl-lsp-core/src/code_actions.rs), `extract_inline_actions`: Use the actual document Registry without an unconditional F5 overlay.
- [rust/tcl-lsp-core/src/code_actions.rs](../../../../rust/tcl-lsp-core/src/code_actions.rs), `refactor_engine_actions`: Preserve complete source/configuration before dispatching refactor APIs.
- [rust/tcl-lsp-core/src/code_actions.rs](../../../../rust/tcl-lsp-core/src/code_actions.rs), `code_actions::original_refactor_context_tests::original_refactor_actions_keep_document_registry_and_full_source` (linked): Counterfactual reporting dialect, actual F5 source, stale image and missing configuration are distinguished.

A named test is a coverage binding, not a claim that it executed.

## Replay

Named fixed Rust selectors bind validation obligations. No Rust execution receipt is attached to this implementation contract. No native interpreter experiment or overall passing suite is claimed. Missing authority and unsupported source purposes remain explicit in the scope and conclusion.
