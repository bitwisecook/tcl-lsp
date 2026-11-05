# naming.workspace.original-declaration-transport

Kind: `implementation-contract`

## Problem statement

Workspace presentation maps can collapse or omit original procedure/class names, including opaque source units. Equal source images can also occur in different documents. A cross-file consumer that derives declaration identity from displayed names or equal source alone can lose a declaration or attribute it to another file; candidate lookup must keep the original metadata and document owner separate.

## Question

Does workspace declaration inventory and lookup transport preserve original name bytes, canonical source slots and URI ownership independently of display maps and copied source images?

## Conclusion

The workspace stores complete original procedure/class declaration metadata separately from presentation rows and pairs every inventory/query result with its document URI. Shared original publication matching returns candidate declarations without collapsing opaque name bytes or identical-source documents. Indexed command occurrences retain their original input. Document removal and replacement withdraw the corresponding owned rows. These declaration/candidate projections do not prove loading, callability at an observation, normal completion, complete references or writable edit geometry.

## Scope

Current Core workspace transport implementation contract. The named regression removes display maps, indexes the same analysis under two URIs, distinguishes two surrogate-unit names, checks exact candidate/input bytes, then removes/replaces a document. It contains no native execution evidence or claimed Rust test outcome. Class inventory uses the same document-owned metadata transport; the concrete test assertions cover procedure declarations and command inputs.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This workspace declaration/URI transport invariant is a Rust implementation contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

## Consumer bindings

- [rust/tcl-lsp-core/src/workspace_index.rs](../../../../rust/tcl-lsp-core/src/workspace_index.rs), `DocumentRecords`: Owns original declaration metadata and source occurrences under the independent workspace document URI.
- [rust/tcl-lsp-core/src/workspace_index.rs](../../../../rust/tcl-lsp-core/src/workspace_index.rs), `original_procedure_declarations`: Projects complete original procedure metadata with its document owner.
- [rust/tcl-lsp-core/src/workspace_index.rs](../../../../rust/tcl-lsp-core/src/workspace_index.rs), `original_class_declarations`: Projects complete original class metadata with its document owner.
- [rust/tcl-lsp-core/src/workspace_index.rs](../../../../rust/tcl-lsp-core/src/workspace_index.rs), `original_procedure_candidates`: Uses the shared original publication matcher and preserves URI-qualified candidates.
- [rust/tcl-lsp-core/src/workspace_index.rs](../../../../rust/tcl-lsp-core/src/workspace_index.rs), `original_class_candidates`: Uses the same shared lookup purpose for document-owned class candidates.
- [rust/tcl-lsp-core/src/workspace_index.rs](../../../../rust/tcl-lsp-core/src/workspace_index.rs), `tests::original_declaration_inventory_retains_bytes_and_document_ownership` (linked): Opaque original declarations and invocation bytes survive removed display maps, remain independently URI-owned for identical-source documents, and withdraw on removal/replacement.

## Evidence

- [rust/tcl-lsp-core/src/workspace_index.rs](../../../../rust/tcl-lsp-core/src/workspace_index.rs)
