# naming.consumer.original-type-and-implementation-navigation

Kind: `implementation-contract`

## Problem statement

Opaque method declarations can lack reporting names, equal declarations can belong to different documents, and a class-object method does not follow instance superclass order. Type and implementation navigation needs the authentic selected member, exact source relations and independently current document owners.

## Question

Do type and implementation queries preserve original member and declaring-class identities across current source owners without borrowing reporting names or instance order for class-object methods?

## Conclusion

Type navigation projects the selected original method to its canonical declaring class under the current complete source and configuration. Implementation navigation retains its own declaration and exact instance descendant overrides through the shared original relation kernel, preserving each target URI and refusing ambiguous parent providers. Class-object methods inspect only the declaring class own table. Missing original selection, lexical variable operands and stale inventories remain terminal. These results are readonly source advice and do not establish installed objects, live native MRO or dispatch.

## Scope

Rust editor consumer contract over original source declaration metadata and genuine member inputs. Fixed tests clear reporting maps, distinguish opaque siblings, retain cross-document owners, reject duplicate parent providers and stale source, and separate instance from class-object member sides. Native method execution, object construction, compiler admission and writable edits have independent owners.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This precise Rust source contract has no native execution receipt.

## Consumer bindings

- [rust/tcl-lsp-core/src/type_definition.rs](../../../../rust/tcl-lsp-core/src/type_definition.rs), `type_definition`: Select a canonical declaring-class source range from a genuine current original method or independently retained object read.
- [rust/tcl-lsp-core/src/implementation.rs](../../../../rust/tcl-lsp-core/src/implementation.rs), `method_implementations_in_inventory`: Keep canonical own declarations and exact instance descendants under independent current URI/source/configuration; class-object candidates use their own class table.
- [rust/tcl-lsp-core/src/method_symbol.rs](../../../../rust/tcl-lsp-core/src/method_symbol.rs), `OriginalMethodCandidate::declaring_class_in`: Revalidate canonical declaration, metadata and receiver side in the owning current source, independently of effective moved selector spelling.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `goto_implementation`: Gather the complete current declaration inventory and return each implementation in its independently revalidated owning document.
- [rust/tcl-lsp-core/src/original_declaration.rs](../../../../rust/tcl-lsp-core/src/original_declaration.rs), `OriginalClassInventory::from_documents`: Validate complete current original class and occupied non-class headers under independent URI/source/configuration; duplicate URIs and stale headers refuse the inventory.
- [rust/tcl-lsp-core/src/type_definition.rs](../../../../rust/tcl-lsp-core/src/type_definition.rs), `type_definition::original_type_tests::original_method_type_navigation_uses_opaque_declaring_class_without_reporting_maps` (linked): Cleared class maps do not erase two opaque member selections; stale complete source refuses class navigation.
- [rust/tcl-lsp-core/src/implementation.rs](../../../../rust/tcl-lsp-core/src/implementation.rs), `implementation::original_implementation_tests::original_implementation_uses_opaque_method_identity_and_exact_descendant_relations` (linked): Separate two opaque member keys, include only the exact instance descendant override, and reject stale source.
- [rust/tcl-lsp-core/src/implementation.rs](../../../../rust/tcl-lsp-core/src/implementation.rs), `implementation::original_implementation_tests::original_method_implementations_keep_cross_document_owners_and_block_ambiguous_parents` (linked): Keep base and child URI owners; duplicate parent providers do not yield a child target and stale inventory refuses.
- [rust/tcl-lsp-core/src/implementation.rs](../../../../rust/tcl-lsp-core/src/implementation.rs), `implementation::original_implementation_tests::original_class_object_implementation_does_not_borrow_instance_superclass_order` (linked): Return the original class-object own declaration without adding the instance subclass method.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `tests::original_implementation_queries_keep_workspace_method_and_class_owners` (linked): Workspace queries keep exact opaque class and method owner URIs and exclude the unrelated same-name method.
- [rust/tcl-lsp-core/src/original_declaration.rs](../../../../rust/tcl-lsp-core/src/original_declaration.rs), `original_declaration::tests::original_class_inventory_keeps_equal_source_owners_and_current_occupied_headers` (linked): Preserve equal source under distinct URI owners and occupied procedure headers; reject duplicate URI owners, changed full source and changed strict-quoting configuration.

## Evidence

- [rust/tcl-lsp-core/src/type_definition.rs](../../../../rust/tcl-lsp-core/src/type_definition.rs)
- [rust/tcl-lsp-core/src/implementation.rs](../../../../rust/tcl-lsp-core/src/implementation.rs)
- [rust/tcl-lsp-core/src/method_symbol.rs](../../../../rust/tcl-lsp-core/src/method_symbol.rs)
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs)
- [rust/tcl-lsp-core/src/original_declaration.rs](../../../../rust/tcl-lsp-core/src/original_declaration.rs)
