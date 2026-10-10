# naming.runtime.original-root-command-context

Kind: `implementation-contract`

## Problem statement

A flat namespace engine has a real root command lookup context without a C namespace-tree root lookup. Reusing tree lookup in shared source-name alias origin traversal refuses Jim namespace helpers even though the genuine root object remains owned by the live interpreter. Returning an unconditional numeric root would lose that ownership and epoch check.

## Question

How can shared alias-origin traversal obtain a genuine root command context from each engine without treating a Jim retained namespace object as a C namespace-tree token?

## Conclusion

Namespaces::root_command_context_checked owns the purpose. Its default adapter retains checked namespace-root lookup for C adapters. The runtime Jim issuer independently requires Jim naming policy, a live native Jim object context and pointer equality between the actual retained top namespace holder and that context original empty object before returning the existing root command context. Checked and unchecked SourceName alias-origin hops reuse this same query. Equal bytes on a foreign holder and retired interpreter epochs refuse; the C namespace-tree Jim lookup stays unavailable. The VM Jim adapter independently checks its original top object identity and the live command world as well as its live object context. Both Runtime and VM satisfy the shared root-command purpose without issuing C namespace-tree ownership.

## Scope

Rust source contract and authored live/foreign-object/retired-context controls in the selected Jim runtime backend. The independent Root current Jim CLI script observation establishes only its exact guest completion/result, not holder pointers, internal header types, allocation epochs or the Rust object-control result. C tree lookup semantics and arbitrary namespace/frame liveness remain separate.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native interpreter observation establishes this Rust source-advice invariant.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native interpreter observation establishes this Rust source-advice invariant.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native interpreter observation establishes this Rust source-advice invariant.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native interpreter observation establishes this Rust source-advice invariant.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native interpreter observation establishes this Rust source-advice invariant.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No native interpreter observation establishes this Rust source-advice invariant.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native interpreter observation establishes this Rust source-advice invariant.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-runtime-api/src/lib.rs](../../../../rust/tcl-runtime-api/src/lib.rs), `Namespaces::root_command_context_checked`: Separate actual root command context purpose from C namespace-tree lookup.
- [runtime/rust/src/state_traits.rs](../../../../runtime/rust/src/state_traits.rs), `Namespaces for Interp::root_command_context_checked`: Require actual live Jim interpreter and exact original top object ownership before issuing its root command context; preserve C adapter lookup.
- [rust/tcl-cmd-core/src/namespace.rs](../../../../rust/tcl-cmd-core/src/namespace.rs), `origin_bytes`: Use the same purpose-specific context for source-name alias hops, retaining byte prefixes, cycle checks and checked command lookup.
- [rust/tcl-cmd-core/src/namespace.rs](../../../../rust/tcl-cmd-core/src/namespace.rs), `origin_bytes_checked`: Use the same purpose-specific context for source-name alias hops, retaining byte prefixes, cycle checks and checked command lookup.
- [rust/tcl-vm/src/interp.rs](../../../../rust/tcl-vm/src/interp.rs), `Namespaces for Vm::root_command_context_checked`: Use the same purpose-specific checked root command context in the VM; require independently selected Jim policy, live actual object context and live NameWorld plus exact original top object identity; retain default checked C tree adapter.
- [runtime/rust/src/interp/native_jim_namespace.rs](../../../../runtime/rust/src/interp/native_jim_namespace.rs), `interp::native_jim_namespace::tests::jim_root_command_context_requires_its_live_original_top_object` (linked): An actual selected Jim runtime context retains its original top object and supplies the root command context; C namespace-tree lookup refuses, equal-byte foreign holder refuses, restoring the original holder restores availability, and retiring the context refuses. This does not infer native Jim pointer or epoch behavior from the separate CLI row.
- [rust/tcl-vm/src/interp/native_namespace_root_tests.rs](../../../../rust/tcl-vm/src/interp/native_namespace_root_tests.rs), `interp::native_namespace_root_tests::jim_root_command_context_requires_its_live_original_top_object` (linked): Selected Jim VM supplies its genuine original root command context; an equal-byte foreign root object refuses, restoring the original object restores availability, and retiring either the original context or command world refuses. This is an authored Rust object-owner assertion and is not inferred from the separate native CLI guest result.

A named test is a coverage binding, not a claim that it executed.

## Replay

Linked Rust tests are coverage obligations, not executed interpreter observations.
