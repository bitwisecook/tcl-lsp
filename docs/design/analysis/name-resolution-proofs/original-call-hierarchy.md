# naming.consumer.original-call-hierarchy

Kind: `implementation-contract`

## Problem statement

Call hierarchy must preserve opaque declaration identities, terminal alias allocations and separate consuming and declaring document owners.

## Question

How are incoming and outgoing source call candidates selected without reconstructing a procedure or member from a displayed name?

## Conclusion

The original call graph selects genuine current consuming lookups and independently owned original declaration records. Direct references compare original slots and policies; call edges may follow a retained terminal implementation allocation. First occupied non-procedure headers, duplicate document owners, missing lookup and stale complete source remain terminal. Each hierarchy item retains a current declaration identity. Call-hierarchy preparation treats a retained original variable cursor as terminal before independently selecting a member declaration. A same-named reported method cannot turn that variable into a callable candidate; genuine canonical method declaration identity survives cleared reporting maps and still requires the exact current whole image.

## Scope

The graph supplies readonly source candidates, including independently owned member metadata through the shared method selector. Source declaration advice is separate from native dispatch. A missing original lookup does not acquire a candidate from a label. Current source/configuration and URI ownership are checked for every supplied document; no native call or optimizer result is measured.

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

- `implementation-0` (implementation): [rust/tcl-lsp-core/src/original_call_hierarchy.rs](../../../../rust/tcl-lsp-core/src/original_call_hierarchy.rs). SHA-256 `9d98ea0b91b59359c56acc93111907478f11a0576fcd5412f1f135a0604a2ff2`. Current implementation source and fixed assertion bindings; no execution receipt.
- `implementation-1` (implementation): [rust/tcl-lsp-core/src/references.rs](../../../../rust/tcl-lsp-core/src/references.rs). SHA-256 `e7aa59e2a45e24cd590da51fc3cf181733036619f768b8dd45872154762ea834`. Current implementation source and fixed assertion bindings; no execution receipt.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/original_call_hierarchy.rs](../../../../rust/tcl-lsp-core/src/original_call_hierarchy.rs), `prepare`: Select original declaration or readonly source call candidates under current source/configuration and actual lookup ownership.
- [rust/tcl-lsp-core/src/original_call_hierarchy.rs](../../../../rust/tcl-lsp-core/src/original_call_hierarchy.rs), `incoming`: Group source candidates by genuine caller declaration and URI rather than displayed names.
- [rust/tcl-lsp-core/src/original_call_hierarchy.rs](../../../../rust/tcl-lsp-core/src/original_call_hierarchy.rs), `outgoing`: Retain independently owned selected target declarations without borrowing native dispatch.
- [rust/tcl-lsp-core/src/references.rs](../../../../rust/tcl-lsp-core/src/references.rs), `invocation_calls_proc`: Use the shared original declaration predicate before explicit lexical compatibility advice.
- [rust/tcl-lsp-core/src/call_hierarchy.rs](../../../../rust/tcl-lsp-core/src/call_hierarchy.rs), `prepare_in_program`: Separate original variable cursor ownership from current canonical member declarations before admitting any independently Logical reporting compatibility.
- [rust/tcl-lsp-core/src/original_call_hierarchy.rs](../../../../rust/tcl-lsp-core/src/original_call_hierarchy.rs), `original_call_hierarchy::tests::original_call_edges_keep_opaque_declarations_and_independent_source_owners` (linked): Separate opaque incoming edges, independent URI owners and missing/stale original lookups after reporting maps are cleared.
- [rust/tcl-lsp-core/src/call_hierarchy.rs](../../../../rust/tcl-lsp-core/src/call_hierarchy.rs), `call_hierarchy::tests::original_hierarchy_preparation_keeps_variable_cursors_and_method_declarations_separate` (linked): A variable named like a method supplies no hierarchy item; a genuine original method declaration retains its exact identity after class report maps are cleared, and stale whole source refuses.

A named test is a coverage binding, not a claim that it executed.

## Replay

The named fixed Rust selectors are source coverage bindings. Their execution results are recorded separately; no native provider execution is asserted for this source contract.
