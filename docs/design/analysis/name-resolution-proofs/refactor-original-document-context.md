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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This Rust consumer contract has no native execution receipt.

## Consumer bindings

- [rust/tcl-lsp-core/src/code_actions.rs](../../../../rust/tcl-lsp-core/src/code_actions.rs), `extract_inline_actions`: Use the actual document Registry without an unconditional F5 overlay.
- [rust/tcl-lsp-core/src/code_actions.rs](../../../../rust/tcl-lsp-core/src/code_actions.rs), `refactor_engine_actions`: Preserve complete source/configuration before dispatching refactor APIs.
- [rust/tcl-lsp-core/src/code_actions.rs](../../../../rust/tcl-lsp-core/src/code_actions.rs), `code_actions::original_refactor_context_tests::original_refactor_actions_keep_document_registry_and_full_source` (linked): Counterfactual reporting dialect, actual F5 source, stale image and missing configuration are distinguished.

## Evidence

- [rust/tcl-lsp-core/src/code_actions.rs](../../../../rust/tcl-lsp-core/src/code_actions.rs)
