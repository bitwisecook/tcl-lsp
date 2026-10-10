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

- `implementation-0` (implementation): [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs). SHA-256 `2a5c8e5f3df024c66d56d92fb833c66359fc21dc387c8fc772fcdbea1c73d572`. Current implementation source and fixed assertion bindings; no execution receipt.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `retain_original_declaration_data`: Bind the owning URI and retain a bounded readonly declaration receipt before returning editor data.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `resolve_original_declaration_data`: Reselect a retained receipt against actual current source and metadata; no qualified-name recovery.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `original_declaration_documents`: Gather independently current document source inventories; a missing owner cannot silently remove an occupied declaration.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `original_declaration_reference_locations`: Obtain code-lens click locations from the canonical original reference owner.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `tests::original_hierarchy_and_lens_wire_data_require_retained_current_declarations` (linked): Round trip retained identities independently of labels/ranges; reject forged/missing/stale/foreign handles and retain distinct opaque lens data.

A named test is a coverage binding, not a claim that it executed.

## Replay

The named fixed Rust selectors are source coverage bindings. Their execution results are recorded separately; no native provider execution is asserted for this source contract.
