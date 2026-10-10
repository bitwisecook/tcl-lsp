# naming.database.original-call-arity-diagnostics

Kind: `implementation-contract`

## Problem statement

Cross-file call diagnostics can join two opaque procedure names through the same display or suppress an unresolved root call because an unrelated namespace has a matching tail. The database must demand source candidate arities using the actual call lookup and retain an unknown original lookup, rather than substituting a bare-tail match.

## Question

Does project diagnostic resolution use each authentic original call lookup to distinguish opaque name slots and preserve an unresolved call whose matching tail exists only in another namespace?

## Conclusion

original_lookup_command_arities demands body-free per-slot arities for retained lookup candidates and uses the shared byte/policy publication matcher. project_diagnostics keeps original-input call sites separate from display-tail resolution, preserves W123 when their original lookup has no candidate, and checks a selected candidate arity at the original call span. The regression distinguishes p\uD800 with one formal from p\uD801 with two, then preserves only_here as an unresolved root call despite the N::only_here declaration. These are source diagnostic projections, without proving command loading, observed current dispatch or execution.

## Scope

Current database cross-file direct-call diagnostic consumer. The concrete regression checks distinct opaque original bytes, the original call spans, arity diagnostics and the typed unresolved subject. It tests unrelated-namespace exclusion; it does not exercise every namespace path/import/mutation alternative or assert a Rust test result. No native interpreter executes this question.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

This authentic-call diagnostic selection invariant is a Rust implementation contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

This authentic-call diagnostic selection invariant is a Rust implementation contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

This authentic-call diagnostic selection invariant is a Rust implementation contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

This authentic-call diagnostic selection invariant is a Rust implementation contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

This authentic-call diagnostic selection invariant is a Rust implementation contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

This authentic-call diagnostic selection invariant is a Rust implementation contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

This authentic-call diagnostic selection invariant is a Rust implementation contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

## Exact evidence

- `database-call-consumer` (implementation): [rust/tcl-lsp-db/src/lib.rs](../../../../rust/tcl-lsp-db/src/lib.rs). SHA-256 `489a9c177add18e04ee506fe11be00cce295bd7a004946483e3c05d8b623a8e2`. Current authentic-call arity demand, original-site diagnostic partition and exact opaque-slot/unrelated-namespace regression assertions.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-db/src/lib.rs](../../../../rust/tcl-lsp-db/src/lib.rs), `original_lookup_command_arities`: Demand original call lookup candidate arities through byte/policy publication matching.
- [rust/tcl-lsp-db/src/lib.rs](../../../../rust/tcl-lsp-db/src/lib.rs), `project_diagnostics`: Retain original call-site identity and unknown lookup in cross-file direct-call diagnostic settlement.
- [rust/tcl-lsp-db/src/lib.rs](../../../../rust/tcl-lsp-db/src/lib.rs), `tests::original_project_diagnostics_select_opaque_call_slots_and_namespace_priority` (linked): The first opaque call receives E003, the independently keyed two-formal call does not, and an unrelated namespace tail cannot suppress W123 with the original only_here subject.

A named test is a coverage binding, not a claim that it executed.

## Replay

The current regression is linked by its actual proof comment and exact assertions. No executed Rust test receipt is attached. Native behavior, command presence/currency and successful body entry require separate evidence.
