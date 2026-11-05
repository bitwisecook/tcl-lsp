# naming.core.original-hint-effective-origins

Kind: `implementation-contract`

## Problem statement

An alias can insert arguments before written words, and a list expansion can supply several effective arguments from one parent word. Counting written tokens against a declaration signature attaches labels to the wrong argument; copying the head producer into a prefix or expansion child fabricates source geometry. The applicability depends on complete current effective arguments, selected descriptor and independently retained written or list-child origins.

## Question

Do parameter hints preserve effective prefix and expansion positions while anchoring only authentic original operands?

## Conclusion

The shared selected Registry words facade retains the complete original source/configuration, selected descriptor and effective origin vector. Captured prefix values advance argument positions without acquiring an anchor. Static original list children retain their own readonly extent; unavailable, computed or mismatching origins decline. Hints are declaration-signature advice and grant no execution, argument effect, edit or header authority.

## Scope

Core parameter hint and shared Registry invocation advice contract. Fixed alias, static expansion, opaque declaration, descriptor shadowing and stale-image selectors are validation obligations. No native provider result is claimed.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This Rust consumer contract has no native execution receipt.

## Consumer bindings

- [rust/tcl-lsp-core/src/original_invocation.rs](../../../../rust/tcl-lsp-core/src/original_invocation.rs), `selected_registry_words`: Reborrow complete current original arguments and selected descriptor with independent origins.
- [rust/tcl-lsp-core/src/inlay_hints.rs](../../../../rust/tcl-lsp-core/src/inlay_hints.rs), `emit_original_builtin_hints`: Map prototype roles through effective arguments rather than written token counts.
- [rust/tcl-lsp-core/src/inlay_hints.rs](../../../../rust/tcl-lsp-core/src/inlay_hints.rs), `inlay_hints::original_hint_tests::original_builtin_alias_and_expansion_hints_keep_effective_slots_and_original_extents` (linked): Alias prefix and expansion children preserve exact argument positions and source anchors.
- [rust/tcl-lsp-core/src/inlay_hints.rs](../../../../rust/tcl-lsp-core/src/inlay_hints.rs), `inlay_hints::original_hint_tests::original_builtin_hints_use_retained_descriptor_and_do_not_borrow_shadowed_prototype` (linked): Require the retained descriptor and refuse a replaced Registry implementation.

## Evidence

- [rust/tcl-lsp-core/src/original_invocation.rs](../../../../rust/tcl-lsp-core/src/original_invocation.rs)
- [rust/tcl-lsp-core/src/inlay_hints.rs](../../../../rust/tcl-lsp-core/src/inlay_hints.rs)
