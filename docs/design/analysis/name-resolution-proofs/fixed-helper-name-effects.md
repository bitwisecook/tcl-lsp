# naming.compiler.fixed-helper-name-effects

Kind: `implementation-contract`

## Problem statement

Unknown compiler admission cannot identify arbitrary binding mutations. Direct emitters can preserve names, while a BasicNArg emitter creates a fresh command-name literal whose getter can enter resolvers. A null compileProc cannot enter a private compiler worker even if that worker remains unavailable for handler or opcode admission.

## Question

Which independent source lookup proof closes a selected named emitter, and can absent compiler-worker admission change the effect of an authentic null compileProc?

## Conclusion

The Registry reports preserved names, a required fixed helper lookup, or unknown effects. The source consumer closes a required lookup against the exact current canonical absolute helper cell in an explicitly authored initial world or with independently captured same-entry native resolver absence. Missing, unknown, alias or conflicting helper cells refuse it; missing, present, unknown, foreign or stale native resolver observations also refuse it. The actual selected naming policy is retained rather than manufactured from a compiler release. Authentic null compileProc effects remain independent of worker admission and never supply body or Normal authority.

## Scope

Complete original vectors, exact source protocol and selected compiler descriptor/release. Direct named ensemble emitters under C8.6/C9 source protocols and actual arity are represented. The VM supplies its own interpreter/namespace resolver absence separately from command-table closure; opaque host snapshots remain residual without such a receipt. Dynamic/nested operands and recursive source compilers cannot borrow this fixed-helper closure. Runtime normal completion, object getter/retirement, opcode/cache admission, command observers and body entry remain separate.

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

- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `NativeCompilationSpec::original_argument_name_effects`: Select emitter effects and explicit fixed command-literal lookup separately from NativeCompilationSelection.
- [rust/tcl-compiler/src/command_binding/compiled_invocation.rs](../../../../rust/tcl-compiler/src/command_binding/compiled_invocation.rs), `original_fixed_compiler_lookup_preserves_names`: Close the actual canonical helper lookup only with independent authored or current native resolver evidence, preserving handler/compiler admission separately.
- [rust/tcl-compiler/src/command_binding/original_command_table.rs](../../../../rust/tcl-compiler/src/command_binding/original_command_table.rs), `original_registry_metadata_target`: One exact current canonical table/namespace projection checks fixed authored Registry helper proposals without creating source NameInputs.
- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `native_compilation::tests::original_named_compiler_effects_require_separate_fixed_lookup` (linked): Retain the required private helper lookup without opcode admission; refuse missing dialect/ordinal; keep authentic null hook effect independent.
- [rust/tcl-compiler/src/command_binding/compiled_invocation.rs](../../../../rust/tcl-compiler/src/command_binding/compiled_invocation.rs), `command_binding::compiled_invocation::tests::original_fixed_compiler_lookup_requires_current_authored_helper_and_resolver_scope` (linked): Accept the exact current direct helper under its selected policy in an authored world or with current VM resolver absence; refuse missing, unknown, alias, absent/foreign policy, unknown entry and missing/present/unknown/foreign/stale resolver observations.
- [rust/tcl-compiler/src/command_binding/compiled_invocation.rs](../../../../rust/tcl-compiler/src/command_binding/compiled_invocation.rs), `command_binding::compiled_invocation::tests::original_null_namespace_compiler_does_not_borrow_worker_admission` (linked): Retain tracked-name preservation for a selected null namespace compileProc without creating compiled proofs or body/native admission.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cargo",
  "test",
  "-p",
  "tcl-registry",
  "--lib",
  "original_named_compiler_effects_require_separate_fixed_lookup"
]
```

Run from rust/. Separately run the two named Compiler selectors. These are implementation contracts, not native launches. The VM resolver observation is a concrete Rust lookup-owner capability; pinned C compiler source definitions are retained in naming.compiler.literal-emitter-name-effects-source.
