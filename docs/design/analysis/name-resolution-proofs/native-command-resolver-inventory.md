# naming.compiler.native-command-resolver-inventory

Kind: `implementation-contract`

## Problem statement

A complete native command and namespace snapshot can select a fixed compiler helper while saying nothing about interpreter or namespace resolver callbacks. Treating table completeness as callback absence would let opaque host resolvers mutate command bindings during lookup. Refusing every native snapshot also discards the VM lookup owner's independently known callback-free table traversal.

## Question

Which independently captured native resolver observation permits a fixed-helper name-effect closure, and which entry or target differences must still reject it?

## Conclusion

Only the concrete VM lookup owner captures interpreter and namespace resolver absence for its same interpreter and mutation epoch. The Compiler consumes that facet with the actual selected naming policy and independently current canonical Registry helper. Missing, present, unknown, foreign or stale resolver evidence remains residual; opaque handlers still have no compiler identity. The observation supplies no object getter/release, handler execution, command-execution trace, Normal completion, frame or opcode authority.

## Scope

Immutable actual VM entry observations and source compiler fixed-helper effect queries. The public provider constructor represents an explicit inspected observation; it does not infer absence from a profile or retained table. Synthetic fixtures default to missing inventory. This is a Rust implementation contract, not a C Tcl, Jim or BIG-IP native observation.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-runtime-api/src/native_compilation.rs](../../../../rust/tcl-runtime-api/src/native_compilation.rs), `NativeCommandResolverInventory::permits_no_callbacks`: Check that the independently captured Absent observation belongs to this exact entry interpreter and mutation epoch.
- [rust/tcl-vm/src/interp.rs](../../../../rust/tcl-vm/src/interp.rs), `Vm::native_compilation_entry_for_namespace_token`: Capture VM lookup resolver absence from its concrete table/namespace owner without borrowing a command catalogue or variable observer state.
- [rust/tcl-compiler/src/command_binding/compiled_invocation.rs](../../../../rust/tcl-compiler/src/command_binding/compiled_invocation.rs), `original_fixed_compiler_lookup_preserves_names`: Join independent resolver closure with the actual current direct Registry helper and selected naming policy.
- [rust/tcl-runtime-api/src/native_compilation.rs](../../../../rust/tcl-runtime-api/src/native_compilation.rs), `NativeCompilationEntry::same_compilation_world`: Preserve resolver inventory as an independent compilation-cache currency axis.
- [rust/tcl-vm/src/interp/native_command_resolver_tests.rs](../../../../rust/tcl-vm/src/interp/native_command_resolver_tests.rs), `interp::native_command_resolver_tests::native_command_resolver_inventory_requires_exact_entry_and_no_target_donation` (linked): Inspect actual VM entries under six selected core fixtures; retain opaque handler identity without executing it; refuse missing, foreign or stale resolver inventory; preserve independent variable-observer and compilation-world differences.
- [rust/tcl-compiler/src/command_binding/compiled_invocation.rs](../../../../rust/tcl-compiler/src/command_binding/compiled_invocation.rs), `command_binding::compiled_invocation::tests::original_fixed_compiler_lookup_requires_current_authored_helper_and_resolver_scope` (linked): Accept the exact current direct helper under its selected policy in an authored world or with current VM resolver absence; refuse missing, unknown, alias, absent/foreign policy, unknown entry and missing/present/unknown/foreign/stale resolver observations.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cargo",
  "test",
  "-p",
  "tcl-vm",
  "--lib",
  "native_command_resolver_inventory_requires_exact_entry_and_no_target_donation"
]
```

Run from rust/. Separately run the Compiler fixed-helper selector. These Rust test bindings do not claim execution here or establish a native engine resolver observation.
