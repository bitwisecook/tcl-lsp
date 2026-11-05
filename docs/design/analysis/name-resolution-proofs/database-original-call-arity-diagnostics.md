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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This authentic-call diagnostic selection invariant is a Rust implementation contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

## Consumer bindings

- [rust/tcl-lsp-db/src/lib.rs](../../../../rust/tcl-lsp-db/src/lib.rs), `original_lookup_command_arities`: Demand original call lookup candidate arities through byte/policy publication matching.
- [rust/tcl-lsp-db/src/lib.rs](../../../../rust/tcl-lsp-db/src/lib.rs), `project_diagnostics`: Retain original call-site identity and unknown lookup in cross-file direct-call diagnostic settlement.
- [rust/tcl-lsp-db/src/lib.rs](../../../../rust/tcl-lsp-db/src/lib.rs), `tests::original_project_diagnostics_select_opaque_call_slots_and_namespace_priority` (linked): The first opaque call receives E003, the independently keyed two-formal call does not, and an unrelated namespace tail cannot suppress W123 with the original only_here subject.

## Evidence

- [rust/tcl-lsp-db/src/lib.rs](../../../../rust/tcl-lsp-db/src/lib.rs)
