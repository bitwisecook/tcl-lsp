# naming.consumer.original-declaration-wire-roundtrip

Kind: `implementation-contract`

## Problem statement

Server hierarchy and lens endpoints must not reconstruct original declarations from client supplied labels, ranges or qualified names.

## Question

How does the server round trip hierarchy and lens data while retaining authentic current source ownership?

## Conclusion

The server retains a bounded original declaration handle in originalDeclaration data and resolves it against the actual owning URI, complete current source, scanner configuration and original declaration inventory. Hierarchy preparation and edges use the shared original call graph. Lens resolution uses canonical declaration references and clears stale commands. Native source mode cannot fall back to a client supplied qualified name.

## Scope

This is an LSP transport and readonly consumer integration contract. The token is a locator for a retained receipt, not a serialization of original naming inputs or a native dispatch proof. Every returned item is rechecked in its own current source document. Native provider experiments and runtime behavior are not executed by this contract.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This precise Rust source contract has no native execution receipt.

## Consumer bindings

- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `retain_original_declaration_data`: Bind the owning URI and retain a bounded readonly declaration receipt before returning editor data.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `resolve_original_declaration_data`: Reselect a retained receipt against actual current source and metadata; no qualified-name recovery.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `original_declaration_documents`: Gather independently current document source inventories; a missing owner cannot silently remove an occupied declaration.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `original_declaration_reference_locations`: Obtain code-lens click locations from the canonical original reference owner.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `tests::original_hierarchy_and_lens_wire_data_require_retained_current_declarations` (linked): Round trip retained identities independently of labels/ranges; reject forged/missing/stale/foreign handles and retain distinct opaque lens data.

## Evidence

- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs)
