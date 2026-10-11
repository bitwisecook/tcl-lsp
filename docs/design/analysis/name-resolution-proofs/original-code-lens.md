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

- `implementation-0` (implementation): [rust/tcl-lsp-core/src/code_lens.rs](../../../../rust/tcl-lsp-core/src/code_lens.rs). SHA-256 `3ba67c08c9d3379d221166ee2d85c6e47947c2c749f66a59153caf91d372284a`. Current implementation source and fixed assertion bindings; no execution receipt.
- `implementation-1` (implementation): [rust/tcl-lsp-core/src/original_call_hierarchy.rs](../../../../rust/tcl-lsp-core/src/original_call_hierarchy.rs). SHA-256 `1e72c30cdc65eb27bc88a2ff4f1234959f22bf34ff72a13db2c19fe117407be1`. Current implementation source and fixed assertion bindings; no execution receipt.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/code_lens.rs](../../../../rust/tcl-lsp-core/src/code_lens.rs), `code_lenses`: Enumerate genuine original source declarations and retain independent lens identities and reference purpose.
- [rust/tcl-lsp-core/src/original_call_hierarchy.rs](../../../../rust/tcl-lsp-core/src/original_call_hierarchy.rs), `references`: Gather canonical original reference locations under complete URI/source/configuration ownership and explicit duplicate-owner refusal.
- [rust/tcl-lsp-core/src/code_lens.rs](../../../../rust/tcl-lsp-core/src/code_lens.rs), `code_lens::tests::original_lenses_keep_repeated_and_opaque_source_declarations_without_ui_maps` (linked): Enumerate distinct opaque/repeated declarations with sealed identities after UI maps are cleared, and refuse stale complete source.

A named test is a coverage binding, not a claim that it executed.

- [rust/tcl-lsp-core/src/code_lens.rs](../../../../rust/tcl-lsp-core/src/code_lens.rs), `code_lens::tests::original_logical_lenses_keep_complete_source_currency_and_actual_profile` (linked): Logical code-lens source advice keeps the actual full input/profile and current original source owner; its source candidates confer no callable or Native entry authority.

Current source linkage supplies no execution or assertion credit.

## Replay

The named fixed Rust selectors are source coverage bindings. Their execution results are recorded separately; no native provider execution is asserted for this source contract.
