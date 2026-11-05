# naming.core.readonly-member-source-candidates

Kind: `implementation-contract`

## Problem statement

A literal loop can produce several member names at one method worker word such as $item. Those readonly list-child values are known names but are not original static words or independently editable declaration spans. Treating the worker text as the name hides candidates; fabricating a Key retags variable syntax and invents edit authority.

## Question

Do completion and outline preserve readonly produced member names while static semantic declaration roles remain conditional on a real original word occurrence?

## Conclusion

OriginalSourceMemberDeclaration retains the actual worker site and NativeWord separately from the canonical produced Input and optional static occurrence. Source method rendering and outline use the retained worker channel/configuration and produced units. Semantic declaration roles consume static_occurrence only, so a readonly $item worker does not become a static member token. These are source candidates, without installation, winning provider or edit grants.

## Scope

Registry-selected source member grammar and closed readonly name producers, including bounded literal-loop list children. Dynamic/unknown producer coverage remains unavailable. Canonical names and moved route Inputs remain distinct. Source cards may share worker geometry; they do not imply a writable name occurrence.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This is a current Rust implementation contract. No native provider observation or test execution is claimed for this precise consumer question.

## Consumer bindings

- [rust/tcl-compiler/src/analyser/types/original_members.rs](../../../../rust/tcl-compiler/src/analyser/types/original_members.rs), `OriginalSourceMemberDeclaration`: Separate canonical readonly name from authentic worker syntax and optional static naming occurrence.
- [rust/tcl-lsp-core/src/original_oo.rs](../../../../rust/tcl-lsp-core/src/original_oo.rs), `method_word`: Render source candidates under the retained channel and full configuration.
- [rust/tcl-lsp-core/src/semantic_tokens/original.rs](../../../../rust/tcl-lsp-core/src/semantic_tokens/original.rs), `OriginalTokenRoles::capture`: Require independent static occurrence for a declaration token role.
- [rust/tcl-lsp-core/src/original_oo.rs](../../../../rust/tcl-lsp-core/src/original_oo.rs), `original_oo::tests::original_readonly_loop_method_candidates_keep_values_separate_from_worker_geometry` (linked): Require two opaque readonly values and exact emitted-word byte roundtrip, without static declaration occurrences.
- [rust/tcl-lsp-core/src/document_symbols/original.rs](../../../../rust/tcl-lsp-core/src/document_symbols/original.rs), `document_symbols::original::tests::original_readonly_loop_names_are_outline_cards_without_static_declaration_keys` (linked): Require both source cards after class reporting maps are cleared.
- [rust/tcl-lsp-core/src/semantic_tokens/original.rs](../../../../rust/tcl-lsp-core/src/semantic_tokens/original.rs), `semantic_tokens::original::readonly_member_tests::original_readonly_worker_value_does_not_retag_variable_syntax_as_member_declaration` (linked): Require readonly worker values to remain absent from static member declaration role overrides.

## Evidence

- [rust/tcl-lsp-core/src/document_symbols/original.rs](../../../../rust/tcl-lsp-core/src/document_symbols/original.rs)
- [rust/tcl-lsp-core/src/original_oo.rs](../../../../rust/tcl-lsp-core/src/original_oo.rs)
- [rust/tcl-lsp-core/src/semantic_tokens/original.rs](../../../../rust/tcl-lsp-core/src/semantic_tokens/original.rs)
