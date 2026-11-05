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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This is a Rust ownership contract. No interpreter or appliance observation is attached.

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

## Evidence

- [rust/tcl-compiler/src/command_binding.rs](../../../../rust/tcl-compiler/src/command_binding.rs)
- [rust/tcl-compiler/src/command_binding/normal_result.rs](../../../../rust/tcl-compiler/src/command_binding/normal_result.rs)
