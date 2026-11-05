# naming.editor.original-declaration-document-participation

Kind: `implementation-contract`

## Problem statement

A server workspace can contain Tcl sources, JavaScript ILX buffers, BIG-IP configuration and APL presentations. Treating every buffer as a Tcl declaration provider can create a false duplicate or disable unrelated Tcl navigation; dropping an unknown relevant Tcl source can erase an earlier occupied provider.

## Question

Does the shared editor declaration collector admit only Tcl source participants while retaining unknown relevant providers as a terminal inventory obligation?

## Conclusion

The collector classifies file and contributed language identity before asking for Tcl analysis. Identity-only environments and APL presentations are outside the Tcl declaration inventory; unrelated non-Tcl buffers contribute no headers. Relevant Tcl sources retain their independently resolved environment, complete source and configuration. An unknown relevant dialect or missing current source refuses the whole inventory. Source classification supplies neither name policy nor cross-dialect declaration authority; original header and relation owners check those independently.

## Scope

Rust server document participation and readonly source inventory contract. A fixed mixed workspace keeps two original Tcl implementation owners while JavaScript with counterfactual Tcl-looking text, BIG-IP configuration and APL are open. A relevant unknown Tcl provider then refuses the inventory. Native Tcl, Jim and logical iRules participation remains independent of each document naming policy and actual current analysis.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This precise Rust source contract has no native execution receipt.

## Consumer bindings

- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `original_declaration_document_participates`: Classify source participation through shared environment, contributed language identity and workspace source-extension owners without granting original name or execution authority.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `original_declaration_documents`: Gather complete current relevant document owners; exclude unrelated surfaces before Tcl analysis and preserve unknown relevant provider refusal.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `tests::original_declaration_inventory_excludes_foreign_buffers_and_keeps_unknown_tcl_terminal` (linked): Mixed JS/BIG-IP/APL buffers do not erase or duplicate two exact Tcl implementation owners; an unknown relevant Tcl source refuses the whole collector and known Native/logical dialects retain independent participation.

## Evidence

- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs)
