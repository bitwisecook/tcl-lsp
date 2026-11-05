# naming.consumer.original-code-lens

Kind: `implementation-contract`

## Problem statement

Reference lenses must enumerate original declarations even when UI maps omit opaque or repeated names, and must resolve the same retained declaration after a client round trip.

## Question

How do source declaration lenses retain identity and reference purpose across opaque names, repeated declarations and stale editor data?

## Conclusion

Original code lenses enumerate current original declaration records and carry their sealed source identity. Counts use original reference matching and independent publication ownership. Resolution reselects the retained declaration against current source and obtains locations from canonical references; stale or forged wire data cannot issue a reference command.

Logical lens source advice retains the actual complete input/profile and current source owner independently of callable installation or Native entry.

## Scope

This is a readonly source lens contract. Named members, properties and lifecycle keywords keep their own declaration purpose. Retaining a property or lifecycle declaration does not invent generated accessor references, constructor calls or destructor dispatch. A reference count describes the identified source locations only; it is not native execution or an absence proof.

Current direct test attributes and real module parents attach these source/API contracts only. They do not certify an executable inventory, passed assertion, complete Native comparison or changed-current-image result. Each original provider answer and independently closed software command retains its exact input/channel/source purpose.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This precise Rust source contract has no native execution receipt.

## Consumer bindings

- [rust/tcl-lsp-core/src/code_lens.rs](../../../../rust/tcl-lsp-core/src/code_lens.rs), `code_lenses`: Enumerate genuine original source declarations and retain independent lens identities and reference purpose.
- [rust/tcl-lsp-core/src/original_call_hierarchy.rs](../../../../rust/tcl-lsp-core/src/original_call_hierarchy.rs), `references`: Gather canonical original reference locations under complete URI/source/configuration ownership and explicit duplicate-owner refusal.
- [rust/tcl-lsp-core/src/code_lens.rs](../../../../rust/tcl-lsp-core/src/code_lens.rs), `code_lens::tests::original_lenses_keep_repeated_and_opaque_source_declarations_without_ui_maps` (linked): Enumerate distinct opaque/repeated declarations with sealed identities after UI maps are cleared, and refuse stale complete source.

- [rust/tcl-lsp-core/src/code_lens.rs](../../../../rust/tcl-lsp-core/src/code_lens.rs), `code_lens::tests::original_logical_lenses_keep_complete_source_currency_and_actual_profile` (linked): Logical code-lens source advice keeps the actual full input/profile and current original source owner; its source candidates confer no callable or Native entry authority.

Current source linkage supplies no execution or assertion credit.

## Evidence

- [rust/tcl-lsp-core/src/code_lens.rs](../../../../rust/tcl-lsp-core/src/code_lens.rs)
- [rust/tcl-lsp-core/src/original_call_hierarchy.rs](../../../../rust/tcl-lsp-core/src/original_call_hierarchy.rs)
