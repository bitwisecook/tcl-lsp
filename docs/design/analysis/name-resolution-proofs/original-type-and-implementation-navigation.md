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

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This precise Rust source contract has no native execution receipt.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This precise Rust source contract has no native execution receipt.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This precise Rust source contract has no native execution receipt.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This precise Rust source contract has no native execution receipt.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This precise Rust source contract has no native execution receipt.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

This precise Rust source contract has no native execution receipt.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

This precise Rust source contract has no native execution receipt.

## Exact evidence

- `implementation-0` (implementation): [rust/tcl-lsp-core/src/type_definition.rs](../../../../rust/tcl-lsp-core/src/type_definition.rs). SHA-256 `128350d2f210f4ad676c34f07b9ba686d0f910e1ab73b8e341126fec787776af`. Current Rust source and fixed assertion bindings; no execution receipt.
- `implementation-1` (implementation): [rust/tcl-lsp-core/src/implementation.rs](../../../../rust/tcl-lsp-core/src/implementation.rs). SHA-256 `55712f99f83f913f14f4fb7cb2a2366b03d53aa388797b1d70dc37054881b4f1`. Current Rust source and fixed assertion bindings; no execution receipt.
- `implementation-2` (implementation): [rust/tcl-lsp-core/src/method_symbol.rs](../../../../rust/tcl-lsp-core/src/method_symbol.rs). SHA-256 `bf5f62d0395ea9719c5e550fa8a6060e59f28e5adb2f4ac9fe9f8fea059870c5`. Current Rust source and fixed assertion bindings; no execution receipt.
- `implementation-3` (implementation): [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs). SHA-256 `23386c1e89d183e32bcb45e2852c9106bf69a1a37e91f673850983ff22405eb4`. Current Rust source and fixed assertion bindings; no execution receipt.
- `implementation-class-inventory` (implementation): [rust/tcl-lsp-core/src/original_declaration.rs](../../../../rust/tcl-lsp-core/src/original_declaration.rs). SHA-256 `eb71dcd56b7311d88e3c423b67dc90b57d1df0f4eb87057a2aa8674f00b527c2`. Current common readonly class inventory and meaningful fixed assertions; no native or Rust execution receipt.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

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

A named test is a coverage binding, not a claim that it executed.

## Replay

The named Rust selectors bind fixed source assertions. Their execution results are separate receipts; no native interpreter executes this editor contract.
