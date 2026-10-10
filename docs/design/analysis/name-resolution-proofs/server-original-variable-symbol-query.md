# naming.server.original-variable-symbol-query

Kind: `implementation-contract`

## Problem statement

Two documents can have equal source and opaque sibling spellings while their editor locations remain document-owned. Cross-document query consumers could join by presentation or fall back to reporting names after source currency fails.

## Question

Do Server cross-document variable definition/reference/hover queries use typed opaque symbols, URI ownership and actual current source?

## Conclusion

The fixed Server integration assertion inserts two distinct URI-owned copies, clears all reporting qualified names, and selects only the typed D800 variable: one foreign declaration, two selected references, four typed occurrences including declarations, and source-only hover counts. Changed source withdraws both direct selection and cross-document definition instead of reviving a reporting-name fallback. These are implementation assertions whose execution must be verified separately; no native resolution/cell/value grant follows.

## Scope

Server editor integration contract for two documents under a C8.6 source-analysis profile. Typed global naming correspondence can relate documents, while returned locations retain their own URI/range. Equal source alone is not occurrence ownership. No runtime currentness, alias execution, type/value result or rename coverage is asserted.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This readonly editor integration is a Rust implementation invariant; no native provider observation answers this question.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This readonly editor integration is a Rust implementation invariant; no native provider observation answers this question.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This readonly editor integration is a Rust implementation invariant; no native provider observation answers this question.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This readonly editor integration is a Rust implementation invariant; no native provider observation answers this question.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This readonly editor integration is a Rust implementation invariant; no native provider observation answers this question.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

This readonly editor integration is a Rust implementation invariant; no native provider observation answers this question.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

This readonly editor integration is a Rust implementation invariant; no native provider observation answers this question.

## Exact evidence

- `actual-consumer-source` (implementation): [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs). SHA-256 `d554d436ef2d376901ce4b4e33440a992770198683fd144ad293e393aea498e3`. Exact current consumer and fixed test source; not an execution receipt or native observation.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `cross_document_variable_definition`: Uses the authentic selected variable symbol and foreign document-owned declaration locations.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `cross_document_variable_references`: Queries exact typed symbol occurrences while retaining each URI/range.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `cross_document_variable_hover`: Renders readonly original-name and source-count facts under actual consumer currency.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `tests::original_variable_queries_keep_opaque_slots_across_documents` (linked): Checks opaque sibling separation across two URIs after clearing reports, exact definition/reference/occurrence counts, readonly hover and stale-source refusal.

A named test is a coverage binding, not a claim that it executed.

## Replay

The named fixed Rust test is a source coverage binding. No Rust test result or native provider capture is attached to this contract; C Tcl, Jim and BIG-IP have not tested this Rust invariant.
