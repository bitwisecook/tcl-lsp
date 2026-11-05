# naming.editor.original-namespace-symbol-selection

Kind: `implementation-contract`

## Problem statement

An opaque namespace operand can collide with a reporting spelling, and a parent namespace may be introduced only as part of a longer written declaration. An editor could use the wrong symbol or project a decoded byte length into unrelated source characters.

## Question

Do the shared readonly namespace selector and Core providers preserve exact byte scope/policy, complete source currency and genuine implicit-parent source geometry?

## Conclusion

The shared namespace selector retains byte scope and naming policy under the complete original image and configuration. Stale sources and retained unknown namespace operands are terminal before reporting-name lookup. An authentic lexical variable root routes to the variable selector even when its value is an unknown namespace operand; unavailable variable identity then remains terminal in that selector. Canonical declaration projection uses the actual written producer and ordinal mapping, including the written prefix of an implicit parent. Core definition, references, hover and rename share this selection; a readonly symbol supplies neither native namespace existence nor a substring editing grant.

## Scope

Core source namespace selection and readonly declaration/reference projection. Two fixed C8.6 assertions distinguish surrogate D800/D801 siblings after reporting names are removed, reject stale whole source, route an unresolved lexical variable to the variable selector, refuse an unresolved command-produced namespace operand, and project the written implicit-parent prefix. Rename still requires its separate checked source edit plan; these source assertions supply no native provider execution.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This readonly editor integration is a Rust implementation invariant; no native provider observation answers this question.

## Consumer bindings

- [rust/tcl-lsp-core/src/namespace_symbol.rs](../../../../rust/tcl-lsp-core/src/namespace_symbol.rs), `select_at_offset`: Issue only current original namespace symbols; authentic lexical variables retain their separate selector priority, and stale/unknown namespace operands are terminal.
- [rust/tcl-lsp-core/src/namespace_symbol.rs](../../../../rust/tcl-lsp-core/src/namespace_symbol.rs), `original_namespace_declaration_spans`: Project genuine declaration geometry, including independently mapped implicit-parent prefixes.
- [rust/tcl-lsp-core/src/namespace_symbol.rs](../../../../rust/tcl-lsp-core/src/namespace_symbol.rs), `original_namespace_spans`: Project typed source occurrences independently of qualified-name reporting strings.
- [rust/tcl-lsp-core/src/namespace_symbol.rs](../../../../rust/tcl-lsp-core/src/namespace_symbol.rs), `namespace_symbol::original_namespace_surface_tests::original_namespace_selection_is_shared_and_terminal_for_stale_or_unknown_inputs` (linked): Clears namespace reports, distinguishes opaque siblings through typed scope/policy, checks definition/reference providers, refuses stale source and unresolved command-produced namespace operands, and routes an unresolved lexical variable to its terminal variable selector.
- [rust/tcl-lsp-core/src/namespace_symbol.rs](../../../../rust/tcl-lsp-core/src/namespace_symbol.rs), `namespace_symbol::original_namespace_surface_tests::original_namespace_definition_uses_written_implicit_parent_geometry` (linked): Projects the actual written escaped prefix for an implicit namespace parent from the authentic source producer.

## Evidence

- [rust/tcl-lsp-core/src/namespace_symbol.rs](../../../../rust/tcl-lsp-core/src/namespace_symbol.rs)
