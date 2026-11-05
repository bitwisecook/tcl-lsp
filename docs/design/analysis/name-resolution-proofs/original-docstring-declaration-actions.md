# naming.core.original-docstring-declaration-actions

Kind: `implementation-contract`

## Problem statement

The reporting procedure map can collapse two opaque names or erase earlier redefinitions. A docstring action selected from that map can document the wrong declaration. Body insertion also cannot treat a computed script value as editable original body text merely because ProcDef carries a reporting range.

## Question

Do docstring actions select current original procedure declarations and separately authenticate interior braced-body editing?

## Conclusion

The action consumes procedure_symbol::declarations, preserving every original record and refusing mismatched complete source/configuration. Preceding comment actions use the selected declaration range. Interior body insertion additionally requires the retained whole original braced body word at the exact body token extent; a computed readonly body does not acquire this edit recipe.

## Scope

Original source procedure cards, including repeated and opaque declarations, and explicitly logical-only declaration advice. The body edit remains unavailable when its original grouped producer is missing or computed. A metadata declaration supplies no live publication, entered procedure or dispatch result.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This is a current Rust implementation contract. No native provider observation or test execution is claimed for this precise consumer question.

## Consumer bindings

- [rust/tcl-lsp-core/src/procedure_symbol.rs](../../../../rust/tcl-lsp-core/src/procedure_symbol.rs), `declarations`: Select the current complete original source declaration vector.
- [rust/tcl-lsp-core/src/code_actions.rs](../../../../rust/tcl-lsp-core/src/code_actions.rs), `original_literal_procedure_body`: Require the authentic grouped braced body word separately from declaration metadata.
- [rust/tcl-lsp-core/src/code_actions.rs](../../../../rust/tcl-lsp-core/src/code_actions.rs), `code_actions::original_docstring_tests::original_docstrings_keep_opaque_redefinitions_without_reporting_maps` (linked): Require all original redefinitions with their own formals after UI map removal; reject stale source and missing configuration.
- [rust/tcl-lsp-core/src/code_actions.rs](../../../../rust/tcl-lsp-core/src/code_actions.rs), `code_actions::original_docstring_tests::original_body_docstring_requires_its_braced_word_and_rejects_computed_body` (linked): Require a literal body insertion, keep computed-body preceding advice, and reject interior computed-body edits.

## Evidence

- [rust/tcl-lsp-core/src/code_actions.rs](../../../../rust/tcl-lsp-core/src/code_actions.rs)
