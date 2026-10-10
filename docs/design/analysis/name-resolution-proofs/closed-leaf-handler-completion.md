# Closed leaf handler completion

Proof ID: `naming.source.closed-leaf-handler-completion`

## Problem statement

A native leaf may pass its exact before-operation checks while the source driver discards its completion. Conversely, a normal child body or retained RHS bytes can be mistaken for successful completion of the enclosing handler. Procedure installation and variable writes therefore need separate operation-owned certificates.

## Question

Which exact invocation and before-write obligations allow the shared source driver to preserve a closed leaf Normal completion without borrowing a child or value receipt?

## Scope

Implementation source-transfer contract for the existing closed procedure installation and selected Set handlers. Before-write checks require the current receiver, selected container policy, namespace or activation, observer closure and absent old contents or an independently closed ordinary producer release. Namespace wrappers, arbitrary callback release and opaque procedure storage need their own owners; no physical interpreter or actual native execution is established.

## Provider answers

| Provider | Status | Answer |
| --- | --- | --- |
| tcl8.4 | not-tested | This is an implementation contract; no interpreter or appliance observation is attached. |
| tcl8.5 | not-tested | This is an implementation contract; no interpreter or appliance observation is attached. |
| tcl8.6 | not-tested | This is an implementation contract; no interpreter or appliance observation is attached. |
| tcl9.0 | not-tested | This is an implementation contract; no interpreter or appliance observation is attached. |
| tcl9.1 | not-tested | This is an implementation contract; no interpreter or appliance observation is attached. |
| jim | not-tested | This is an implementation contract; no interpreter or appliance observation is attached. |
| bigip | not-tested | This is an implementation contract; no interpreter or appliance observation is attached. |

## Conclusion

A private native leaf certificate retains the exact original source site, complete words, LexerConfig and semantic operation. A valid closed procedure installation or an independently rechecked before-store write receipt may issue it. The outer driver consumes and clears this certificate; a normal child, known bytes, catalogue footprint or possible Ok route alone cannot issue it.

## Shared owners and tests

- `rust/tcl-compiler/src/command_binding.rs`: `SourceNativeNormalCompletion` — Retain and consume a completion certificate only at the exact original native leaf invocation.
- `rust/tcl-compiler/src/command_binding/native_result.rs`: `original_store_receiver` — Select the Set receiver from actual compiler operands or the independently frozen Generic name, without claiming write success.
- `rust/tcl-compiler/src/var_resolve/original_name_write.rs`: `OriginalNormalValueWrite` — Seal before-store receiver, container, namespace, observer and old-content release obligations independently of RHS data.

- `rust/tcl-compiler/src/command_binding/normal_result.rs`: `command_binding::normal_result::tests::original_closed_leaf_completion_keeps_definition_and_store_obligations_separate` — A deferred error body does not run during the valid procedure definition; nested selected stores require closed writes. Invalid formal topology, scalar/array incompatibility and argument errors cannot complete the whole root.
- `rust/tcl-compiler/src/command_binding/normal_result.rs`: `command_binding::normal_result::tests::original_jim_fresh_store_completion_uses_the_selected_write_kernel` — An authentic Jim environment point selects its own fresh before-write kernel; a missing RHS read cannot provide the same completion.

## Reconfirmation

Run the exact named Rust selectors through the maintained workspace test setup. This page records implementation obligations, not an executed Rust result or an interpreter observation.
