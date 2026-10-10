# naming.editor.original-declaration-wire-identity

Kind: `implementation-contract`

## Problem statement

Editor consumers need a declaration identity that keeps opaque names, repeated declarations, member sides and independent document ownership separate.

## Question

How can readonly declaration metadata and compact editor wire locators retain complete source ownership without constructing an original naming input or runtime capability?

## Conclusion

OriginalDeclarationIdentity is issued from current original metadata and retains its URI, complete source image, scanner configuration, role and allocation sites. A bounded OriginalDeclarationRegistry resolves a compact token only by rechecking that full receipt against current metadata; labels, ranges and deserialized tokens supply no naming or dispatch authority.

## Scope

This is a Rust source metadata and wire identity contract. Procedure and class publications, static or computed member declarations, property declarations and nameless lifecycle syntax retain distinct purposes. Literal API values use selected native document ingress solely for readonly publication matching. No interpreter execution, installed command, writable edit, lifecycle absence or native object-header result is asserted.

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

- `implementation-0` (implementation): [rust/tcl-lsp-core/src/original_declaration.rs](../../../../rust/tcl-lsp-core/src/original_declaration.rs). SHA-256 `d113b984e87601ade6ea5c03fe31ec43391afb1033b05c27aa2f8e293e91191e`. Current implementation source and fixed assertion bindings; no execution receipt.
- `implementation-1` (implementation): [rust/tcl-compiler/src/command_binding/command_reference.rs](../../../../rust/tcl-compiler/src/command_binding/command_reference.rs). SHA-256 `bef3e3d1c6d1fc6911b5dd869738ccb6fba347f386b4c8ba4617b9698911d0cc`. Current implementation source and fixed assertion bindings; no execution receipt.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/original_declaration.rs](../../../../rust/tcl-lsp-core/src/original_declaration.rs), `OriginalDeclarationIdentity`: Retain independently current URI/image/configuration and genuine original declaration allocation/side/producer metadata for readonly source consumers.
- [rust/tcl-lsp-core/src/original_declaration.rs](../../../../rust/tcl-lsp-core/src/original_declaration.rs), `OriginalDeclarationRegistry`: Resolve bounded wire locators by full receipt revalidation; token parsing grants no naming or runtime authority.
- [rust/tcl-lsp-core/src/original_declaration.rs](../../../../rust/tcl-lsp-core/src/original_declaration.rs), `invocation_targets_declaration_in`: Match authentic independent consuming and declaring sources using positioned original input/lookup and selected allocation; direct references additionally retain called slot and policy.
- [rust/tcl-lsp-core/src/original_declaration.rs](../../../../rust/tcl-lsp-core/src/original_declaration.rs), `literal_name_matches_publication`: Apply selected native document ingress to literal Unicode API values for readonly root publication matching without fabricating an original operand.
- [rust/tcl-compiler/src/command_binding/command_reference.rs](../../../../rust/tcl-compiler/src/command_binding/command_reference.rs), `matches_original_invocation_site`: Expose only the retained reference issuance/site correspondence, without widening source bindings.
- [rust/tcl-lsp-core/src/definition.rs](../../../../rust/tcl-lsp-core/src/definition.rs), `resolve_proc_target_at`: Select the actual current original procedure publication before projecting readonly metadata; reporting names and spans cannot choose it.
- [rust/tcl-lsp-core/src/original_declaration.rs](../../../../rust/tcl-lsp-core/src/original_declaration.rs), `original_declaration::tests::original_declaration_handles_reselect_complete_source_uri_and_scanner` (linked): Retain opaque/repeated source declarations and reject stale source/configuration, foreign URI and forged wire handles after display tables are cleared.
- [rust/tcl-lsp-core/src/original_declaration.rs](../../../../rust/tcl-lsp-core/src/original_declaration.rs), `original_declaration::tests::original_declaration_reference_matching_keeps_opaque_slots_and_actual_lookup_owners` (linked): Select exact opaque declaration allocations under actual lookup/input ownership and refuse missing lookup or stale complete source.
- [rust/tcl-lsp-core/src/definition.rs](../../../../rust/tcl-lsp-core/src/definition.rs), `definition::tests::original_proc_and_class_selection_uses_current_typed_publications` (linked): Authentic opaque procedure and class selection across the selected C/Jim models uses current original publications despite cleared/counterfeit reporting maps; complete source or scanner changes withdraw advice.

A named test is a coverage binding, not a claim that it executed.

## Replay

The named fixed Rust selectors are source coverage bindings. Their execution results are recorded separately; no native provider execution is asserted for this source contract.
