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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This precise Rust source contract has no native execution receipt.

## Consumer bindings

- [rust/tcl-lsp-core/src/original_call_hierarchy.rs](../../../../rust/tcl-lsp-core/src/original_call_hierarchy.rs), `prepare`: Select original declaration or readonly source call candidates under current source/configuration and actual lookup ownership.
- [rust/tcl-lsp-core/src/original_call_hierarchy.rs](../../../../rust/tcl-lsp-core/src/original_call_hierarchy.rs), `incoming`: Group source candidates by genuine caller declaration and URI rather than displayed names.
- [rust/tcl-lsp-core/src/original_call_hierarchy.rs](../../../../rust/tcl-lsp-core/src/original_call_hierarchy.rs), `outgoing`: Retain independently owned selected target declarations without borrowing native dispatch.
- [rust/tcl-lsp-core/src/references.rs](../../../../rust/tcl-lsp-core/src/references.rs), `invocation_calls_proc`: Use the shared original declaration predicate before explicit lexical compatibility advice.
- [rust/tcl-lsp-core/src/call_hierarchy.rs](../../../../rust/tcl-lsp-core/src/call_hierarchy.rs), `prepare_in_program`: Separate original variable cursor ownership from current canonical member declarations before admitting any independently Logical reporting compatibility.
- [rust/tcl-lsp-core/src/original_call_hierarchy.rs](../../../../rust/tcl-lsp-core/src/original_call_hierarchy.rs), `original_call_hierarchy::tests::original_call_edges_keep_opaque_declarations_and_independent_source_owners` (linked): Separate opaque incoming edges, independent URI owners and missing/stale original lookups after reporting maps are cleared.
- [rust/tcl-lsp-core/src/call_hierarchy.rs](../../../../rust/tcl-lsp-core/src/call_hierarchy.rs), `call_hierarchy::tests::original_hierarchy_preparation_keeps_variable_cursors_and_method_declarations_separate` (linked): A variable named like a method supplies no hierarchy item; a genuine original method declaration retains its exact identity after class report maps are cleared, and stale whole source refuses.

## Primary authored declaration product

A genuine Registry-described classmethod retains its ClassObject declaration and Instance forward as separate typed products sharing one written name. The existing original member owner exposes which product selects that authored declaration cursor. Declaration-only enumeration and method cursor selection use the primary product; the secondary forward remains available to independently selected call/reference lookup. This projection does not establish installation, receiver identity, body entry, runtime equivalence or editing authority. CALL labels render the independently retained class spelling and member label after identity selection; internal kind keys remain identity bookkeeping, not visible names.

Conditional source caller lineage is distinct from entered procedure/receiver frames. Actual retained member bodies join the complete original member argv and selected body ordinal against current class/member allocations; no surrounding span creates a receiver. Original Native procedure source calls retain the existing graph-selected declaration at their real call horizon, original argv, held lineage, unknown mutations and enclosing original procedure body. They supply possible readonly source relations only, never runtime target selection, observer absence, completed call census or edit equivalence. Unsupported body phases, self-selector targets and unavailable source issuers remain partial. Fixed source controls are unexecuted.

Original Native procedure-body command suggestions reborrow the existing retained body-children owner, exact original vectors, full input and source-analysis allocation. Inert braced data is not traversed as script. The unresolved-head adapter retains literal source head data and genuine declaration membership without inventing a qualified target or Logical candidate. It remains a positive partial inventory, including non-local built-in heads as before; it cannot prove unknown-body absence or physical dispatch.

## Evidence

- [rust/tcl-lsp-core/src/original_call_hierarchy.rs](../../../../rust/tcl-lsp-core/src/original_call_hierarchy.rs)
- [rust/tcl-lsp-core/src/references.rs](../../../../rust/tcl-lsp-core/src/references.rs)
