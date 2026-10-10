# naming.database.original-header-arity

Kind: `implementation-contract`

## Problem statement

Body-free project declaration caches can drop opaque names or key unrelated declarations by their displayed tail. A wrong naming provider can then donate an arity result, while body-only edits needlessly invalidate signature assistance. The database consumer needs original declaration headers and a byte/policy key, with declaration assistance kept separate from actual dispatch.

## Question

Does the database header/arity projection preserve distinct opaque declaration slots and naming policy, remain unchanged across body-only edits, and update the selected arity when its formal header changes?

## Conclusion

Original declaration metadata reaches item signatures and file declarations. The project arity projection keys original source publications; per-slot lookup consumes exact bytes and naming policy through the shared matcher. Wrong-provider lookup remains absent. The concrete regression preserves two distinct opaque procedure slots, keeps their arities across changed body text, and updates only the changed formal-header arity. This cache/query result describes source candidates, without proving command loading, current occupancy, successful dispatch or execution.

## Scope

Current Salsa database declaration-header and arity consumer contract. The linked test uses file_decls/item_sigs and typed project/per-slot arity queries, with no native Runtime call. It covers two procedure headers and body/formal edits; no native-language outcome, Rust test result or class-dispatch completeness is claimed.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

This database declaration-header/arity invariant is a Rust implementation contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

This database declaration-header/arity invariant is a Rust implementation contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

This database declaration-header/arity invariant is a Rust implementation contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

This database declaration-header/arity invariant is a Rust implementation contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

This database declaration-header/arity invariant is a Rust implementation contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

This database declaration-header/arity invariant is a Rust implementation contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

This database declaration-header/arity invariant is a Rust implementation contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

## Exact evidence

- `database-source-owner` (implementation): [rust/tcl-lsp-db/src/lib.rs](../../../../rust/tcl-lsp-db/src/lib.rs). SHA-256 `489a9c177add18e04ee506fe11be00cce295bd7a004946483e3c05d8b623a8e2`. Current body-free header aggregation, exact byte/policy arity key, shared lookup matcher and concrete cache regression assertions.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-db/src/lib.rs](../../../../rust/tcl-lsp-db/src/lib.rs), `item_sigs`: Carries body-stripped original declaration headers across the signature cache boundary.
- [rust/tcl-lsp-db/src/lib.rs](../../../../rust/tcl-lsp-db/src/lib.rs), `file_decls`: Aggregates original source declaration metadata from item signatures.
- [rust/tcl-lsp-db/src/lib.rs](../../../../rust/tcl-lsp-db/src/lib.rs), `project_original_command_arities`: Builds original source-publication arity candidates independently of UI tails.
- [rust/tcl-lsp-db/src/lib.rs](../../../../rust/tcl-lsp-db/src/lib.rs), `OriginalCommandSlot`: Interns byte slot and naming policy; supplies no command-publication or execution grant.
- [rust/tcl-lsp-db/src/lib.rs](../../../../rust/tcl-lsp-db/src/lib.rs), `original_command_arity`: Projects exact original candidate arities through the shared byte/policy publication matcher.
- [rust/tcl-lsp-db/src/lib.rs](../../../../rust/tcl-lsp-db/src/lib.rs), `tests::original_project_headers_keep_opaque_slots_and_body_free_arity` (linked): Opaque original slots retain distinct arities; wrong provider is rejected; body-only edits preserve projection; formal-header edits update the selected slot.

A named test is a coverage binding, not a claim that it executed.

## Replay

The current regression and exact assertions are linked as a coverage obligation. No executed-test receipt is attached. Source candidate arity is independent from observed command lifetime, selected implementation, Native argv or normal completion.
