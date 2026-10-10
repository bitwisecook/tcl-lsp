# naming.source.recursive-driver-state-transport

Kind: `implementation-contract`

## Problem statement

A deterministic 80-level namespace source requires bounded recursive stack storage. Complete retained states and invocation-result temporaries cannot be dropped or replaced by identity hashes to meet that bound.

## Question

How can the common source driver move complete retained state off its recursive stack without weakening ownership or depth guards?

## Conclusion

The driver transports complete branch states, invocation-result bases and intrinsic completion premises in heap containers. A non-inlined capture leaf finishes large receipt construction temporaries before retaining recursive descent. Cold typed body-flow branches remain separate from the recursive sequence and concatenated-script path. Values retain their full structural identity; no digest or reporting projection substitutes for retained state. The recursive execution view borrows its evaluator-owned original variable-compilation facade. Argument evaluation retains the updated facade in an owned heap holder for the entire consumer lifetime; the short view preserves all source, configuration, namespace and evaluated-word receipts without an extra semantic owner.

## Scope

Rust storage and recursion invariant. The existing depth limit and deterministic 80-level input remain exact. This implementation contract issues no measured stack reduction or execution result; independently retained Rust validation receipts keep their own frozen source and assertion scope.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust ownership contract. No interpreter or appliance observation is attached.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust ownership contract. No interpreter or appliance observation is attached.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust ownership contract. No interpreter or appliance observation is attached.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust ownership contract. No interpreter or appliance observation is attached.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust ownership contract. No interpreter or appliance observation is attached.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

This is a Rust ownership contract. No interpreter or appliance observation is attached.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

This is a Rust ownership contract. No interpreter or appliance observation is attached.

## Exact evidence

- `implementation-0` (implementation): [rust/tcl-compiler/src/command_binding.rs](../../../../rust/tcl-compiler/src/command_binding.rs). SHA-256 `79a7eea36eb850a3dc3e2702b5d4b398d317ee192c4bd95dce026b24c1d17989`. Reviewed Rust owner for this implementation invariant; this file is not an executed provider capture.
- `implementation-1` (implementation): [rust/tcl-compiler/src/command_binding/normal_result.rs](../../../../rust/tcl-compiler/src/command_binding/normal_result.rs). SHA-256 `d6a41edd775c2bec3bb62f1a4cdc0128bc22d1049dc3e4a7da2b5b1881b6c10f`. Reviewed Rust owner for this implementation invariant; this file is not an executed provider capture.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding.rs](../../../../rust/tcl-compiler/src/command_binding.rs), `SourceCommandBindings::walk_unknown_compiler_handler`: Move complete stable and residual branch states through the existing boxed branch helper.
- [rust/tcl-compiler/src/command_binding.rs](../../../../rust/tcl-compiler/src/command_binding.rs), `SourceCommandBindings::record_evaluated_source_point`: Return the complete result basis in a heap container across recursive dispatch.
- [rust/tcl-compiler/src/command_binding.rs](../../../../rust/tcl-compiler/src/command_binding.rs), `SourceCommandBindings::walk_non_loop_body_flow`: Keep hot sequence/concatenated-script traversal separate from cold typed body-flow temporaries.
- [rust/tcl-compiler/src/command_binding/normal_result.rs](../../../../rust/tcl-compiler/src/command_binding/normal_result.rs), `SourceCommandBindings::record_invocation_normal_result`: Consume the complete boxed basis and retain it unchanged in the immutable result observation.
- [rust/tcl-compiler/src/command_binding.rs](../../../../rust/tcl-compiler/src/command_binding.rs), `boxed_source_completion_receipt`: Retain the complete same capture result on the heap across child descent; capture temporaries end in a non-inlined leaf without changing premise/currentness/completion checks.
- [rust/tcl-compiler/src/command_binding.rs](../../../../rust/tcl-compiler/src/command_binding.rs), `SourceArgumentExecutionContext`: Own the actual evaluated compilation facade while a short execution view borrows it.
- [rust/tcl-compiler/src/command_binding.rs](../../../../rust/tcl-compiler/src/command_binding.rs), `SourceArgumentExecutionContext::context`: Return a short-lived view borrowing the same complete evaluator-owned receipt.
- [rust/tcl-compiler/src/command_binding.rs](../../../../rust/tcl-compiler/src/command_binding.rs), `source_argument_context_boxed`: Update the selected original compilation with the actual frozen evaluated-name vector and retain it across recursive descent.
- [rust/tcl-compiler/src/command_binding/native_list_assignment.rs](../../../../rust/tcl-compiler/src/command_binding/native_list_assignment.rs), `evaluate_assignment_operand`: Keep the argument-context owner alive through genuine sequential assignment evaluation and exact target projection.
- [rust/tcl-lsp-core/src/refactor/inline_variable.rs](../../../../rust/tcl-lsp-core/src/refactor/inline_variable.rs), `refactor::inline_variable::tests::deeply_nested_namespaces_survive_scope_walk` (linked): The unchanged deterministic 80-level source exercises genuine analyser/source-driver nesting under the standard test-thread stack. No success or elapsed-time claim is inferred from this coverage binding.
- [rust/tcl-compiler/src/command_binding/native_list_assignment.rs](../../../../rust/tcl-compiler/src/command_binding/native_list_assignment.rs), `command_binding::native_list_assignment::tests::original_argument_context_borrows_its_evaluator_owned_compilation` (linked): The view borrows the owned selected compilation, retains actual evaluated vector correspondence, rejects wrong cardinality/configuration and stays within the established small execution-view size budget.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact linked selectors exercise complete original state and immutable completion-receipt transport. Linked coverage does not establish a stack bound, timing result or whole-suite success. Executed assertion and process outcomes remain separately recorded with their exact frozen source inventories in the Rust validation ledger.
