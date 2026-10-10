# naming.consumer.original-linked-editing

Kind: `implementation-contract`

## Problem statement

LSP linked editing mirrors replacement text verbatim into every range. A canonical procedure match is insufficient when a call uses a qualifier, alias, computed name, or a different escape spelling.

## Question

Can linked editing mirror identical literal source components while preserving canonical procedure identity and qualifiers?

## Conclusion

The consumer selects the retained original procedure and independently matches direct self-call allocations. Native command component extents map through the shared literal source geometry. Each accepted range contains exactly the same plain source text; qualified heads retain their prefixes. Computed heads, linked aliases, escape-dependent components, unrelated arguments, stale source and missing scanner configuration cannot borrow a writable declaration range. ASCII and Unicode components use explicit validating patterns.

## Scope

Current Rust consumer and source geometry contract. Native naming observations retained elsewhere are independently scoped; they do not prove this editor integration. Fixed Rust tests bind the assertions without claiming a native consumer result.

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

- `implementation-source` (implementation): [rust/tcl-lsp-core/src/linked_editing_range.rs](../../../../rust/tcl-lsp-core/src/linked_editing_range.rs). SHA-256 `2f5e010c9ed9558cd5ab2e80acad85d935edd7cfc6918f3e6af708abb41cfa59`. Current Rust source geometry and consumer assertions; no native or Rust execution receipt.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/linked_editing_range.rs](../../../../rust/tcl-lsp-core/src/linked_editing_range.rs), `linked_editing_ranges`: Use genuine current procedure identity and identical plain written terminal components for linked editing.
- [rust/tcl-lsp-core/src/linked_editing_range.rs](../../../../rust/tcl-lsp-core/src/linked_editing_range.rs), `original_command_tail_span`: Map independently selected native command component extents through the genuine source word; qualifiers and escape-dependent components do not gain edits.
- [rust/tcl-lsp-core/src/linked_editing_range.rs](../../../../rust/tcl-lsp-core/src/linked_editing_range.rs), `linked_editing_range::original_linked_tests::original_linked_editing_keeps_qualified_unicode_tail_and_refuses_stale_source` (linked): The consumer selects the retained original procedure and independently matches direct self-call allocations. Native command component extents map through the shared literal source geometry. Each accepted range contains exactly the same plain source text; qualified heads retain their prefixes. Computed heads, linked aliases, escape-dependent components, unrelated arguments, stale source and missing scanner configuration cannot borrow a writable declaration range. ASCII and Unicode components use explicit validating patterns.
- [rust/tcl-lsp-core/src/linked_editing_range.rs](../../../../rust/tcl-lsp-core/src/linked_editing_range.rs), `linked_editing_range::original_linked_tests::original_linked_editing_does_not_mirror_escapes_computed_heads_or_unrelated_body_words` (linked): The consumer selects the retained original procedure and independently matches direct self-call allocations. Native command component extents map through the shared literal source geometry. Each accepted range contains exactly the same plain source text; qualified heads retain their prefixes. Computed heads, linked aliases, escape-dependent components, unrelated arguments, stale source and missing scanner configuration cannot borrow a writable declaration range. ASCII and Unicode components use explicit validating patterns.

A named test is a coverage binding, not a claim that it executed.

## Replay

The named fixed Rust selectors are reproducible source contract checks. Their execution is not asserted by this record. Native experiments are independently scoped.
