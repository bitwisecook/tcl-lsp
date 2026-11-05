# naming.editor.original-own-object-member-navigation

Kind: `implementation-contract`

## Problem statement

An object own method can have the same native bytes as a class method. Joining its retained entry through the class member ledger loses the actual worker or selects advice from the wrong table.

## Question

How do readonly method candidates, declaration handles, references, code lenses and call hierarchy retain an original own-object worker independently of class-owned members?

## Conclusion

Own-object queries retain their independently issued configuration target, bounded allocation, canonical source worker and actual entry/generation receipt. Candidates and identities use that own owner under complete current source/configuration and URI currency. Class association and declaration side remain separate; private, missing, unknown or stale entries cannot obtain a class-table substitute. Source queries use the canonical class-or-object adapter for references, lenses and call edges.

## Scope

Rust readonly source ownership and editor consumer contract. Fixed C8.6/C9.0/C9.1 source-analysis controls discriminate own versus class declarations, independent object allocations, opaque siblings, replacement generations, visibility and current image/configuration. No native interpreter is executed by these tests. No live native dispatch, current runtime table, successful body execution, edit permission or constructor completion is granted by a candidate or wire handle.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This question is a Rust readonly consumer contract. No executed native observation is attached.

## Consumer bindings

- [rust/tcl-lsp-core/src/receiver_identity.rs](../../../../rust/tcl-lsp-core/src/receiver_identity.rs), `original_own_method_metadata`: Join the canonical own-table worker through its independently retained object allocation and actual entry declaration.
- [rust/tcl-lsp-core/src/method_symbol.rs](../../../../rust/tcl-lsp-core/src/method_symbol.rs), `temporal_query`: Preserve the own-object entry and dispatch-generation receipt separately from declaration side and class metadata.
- [rust/tcl-lsp-core/src/method_symbol.rs](../../../../rust/tcl-lsp-core/src/method_symbol.rs), `candidate_for_analysis`: Reselect the exact own configuration and worker under complete current source and grammar currency.
- [rust/tcl-lsp-core/src/method_symbol.rs](../../../../rust/tcl-lsp-core/src/method_symbol.rs), `candidate`: Check the independent indexed URI source/configuration before returning the sealed readonly own-object candidate.
- [rust/tcl-lsp-core/src/original_declaration.rs](../../../../rust/tcl-lsp-core/src/original_declaration.rs), `OriginalDeclarationIdentity::for_object_method`: Seal a readonly own-object identity with its complete original configuration, allocation, canonical source worker and URI; no class table or replacement geometry is supplied.
- [rust/tcl-lsp-core/src/original_declaration.rs](../../../../rust/tcl-lsp-core/src/original_declaration.rs), `OriginalDeclarationIdentity::for_member_candidate`: Reuse the canonical class-or-object member owner for wire locators and source navigation.
- [rust/tcl-lsp-core/src/original_call_hierarchy.rs](../../../../rust/tcl-lsp-core/src/original_call_hierarchy.rs), `edges`: Select genuine source queries directly and carry the independently owned own-object worker into source call edges and reference locations.
- [rust/tcl-lsp-core/src/method_symbol.rs](../../../../rust/tcl-lsp-core/src/method_symbol.rs), `method_symbol::own_object_tests::original_own_object_candidates_keep_allocation_and_canonical_worker_without_class_tables` (linked): Keep own and sibling/class declarations separate, distinguish opaque keys with UI inventories cleared, and refuse changed source/configuration.
- [rust/tcl-lsp-core/src/method_symbol.rs](../../../../rust/tcl-lsp-core/src/method_symbol.rs), `method_symbol::own_object_tests::original_own_object_generation_and_private_entries_block_class_advice` (linked): Reselect replacement own workers by canonical site under the same object allocation; private or unknown current entries cannot borrow an exported class worker.
- [rust/tcl-lsp-core/src/original_declaration.rs](../../../../rust/tcl-lsp-core/src/original_declaration.rs), `original_declaration::own_object_identity_tests::original_own_object_identity_keeps_uri_currency_and_wire_reselection` (linked): Retain independent URI ownership, verify readonly handles against the current full image, enumerate lenses, and select actual own-worker references.
- [rust/tcl-lsp-core/src/original_call_hierarchy.rs](../../../../rust/tcl-lsp-core/src/original_call_hierarchy.rs), `original_call_hierarchy::own_object_call_tests::original_own_object_calls_and_references_keep_the_canonical_object_worker` (linked): Prepare and group source calls/references under the actual own-object worker, with stale source withdrawal.

## Evidence

- [rust/tcl-lsp-core/src/receiver_identity.rs](../../../../rust/tcl-lsp-core/src/receiver_identity.rs)
- [rust/tcl-lsp-core/src/method_symbol.rs](../../../../rust/tcl-lsp-core/src/method_symbol.rs)
- [rust/tcl-lsp-core/src/original_declaration.rs](../../../../rust/tcl-lsp-core/src/original_declaration.rs)
- [rust/tcl-lsp-core/src/original_call_hierarchy.rs](../../../../rust/tcl-lsp-core/src/original_call_hierarchy.rs)
