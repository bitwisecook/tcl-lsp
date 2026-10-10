# naming.consumer.original-code-lens

Kind: `implementation-contract`

## Problem statement

Reference lenses must enumerate original declarations even when UI maps omit opaque or repeated names, and must resolve the same retained declaration after a client round trip.

## Question

How do source declaration lenses retain identity and reference purpose across opaque names, repeated declarations and stale editor data?

## Conclusion

Original code lenses enumerate current original declaration records and carry their sealed source identity. Counts use original reference matching and independent publication ownership. Resolution reselects the retained declaration against current source and obtains locations from canonical references; stale or forged wire data cannot issue a reference command.

## Scope

This is a readonly source lens contract. Named members, properties and lifecycle keywords keep their own declaration purpose. Retaining a property or lifecycle declaration does not invent generated accessor references, constructor calls or destructor dispatch. A reference count describes the identified source locations only; it is not native execution or an absence proof.

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

- `implementation-0` (implementation): [rust/tcl-lsp-core/src/code_lens.rs](../../../../rust/tcl-lsp-core/src/code_lens.rs). SHA-256 `259e633316966c9a8975285882a3a66b383c9245b5182f491ef16440d28049ed`. Current implementation source and fixed assertion bindings; no execution receipt.
- `implementation-1` (implementation): [rust/tcl-lsp-core/src/original_call_hierarchy.rs](../../../../rust/tcl-lsp-core/src/original_call_hierarchy.rs). SHA-256 `9d98ea0b91b59359c56acc93111907478f11a0576fcd5412f1f135a0604a2ff2`. Current implementation source and fixed assertion bindings; no execution receipt.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/code_lens.rs](../../../../rust/tcl-lsp-core/src/code_lens.rs), `code_lenses`: Enumerate genuine original source declarations and retain independent lens identities and reference purpose.
- [rust/tcl-lsp-core/src/original_call_hierarchy.rs](../../../../rust/tcl-lsp-core/src/original_call_hierarchy.rs), `references`: Gather canonical original reference locations under complete URI/source/configuration ownership and explicit duplicate-owner refusal.
- [rust/tcl-lsp-core/src/code_lens.rs](../../../../rust/tcl-lsp-core/src/code_lens.rs), `code_lens::tests::original_lenses_keep_repeated_and_opaque_source_declarations_without_ui_maps` (linked): Enumerate distinct opaque/repeated declarations with sealed identities after UI maps are cleared, and refuse stale complete source.

A named test is a coverage binding, not a claim that it executed.

## Replay

The named fixed Rust selectors are source coverage bindings. Their execution results are recorded separately; no native provider execution is asserted for this source contract.
