# naming.variable.original-destruction-operand-keys

Kind: `implementation-contract`

## Problem statement

A conditional unset diagnostic can retain the correct written operand yet lose its alias target if SSA reconstructs a key from the reporting name against a different context. The spelling linked can designate the original local cell at the source point while a later table or logical reparse denotes another activation slot.

## Question

Does conditional SSA destruction advice resolve exact original operand ordinals through each retained source-point alias context?

## Conclusion

The declaration flow retains the actual written argument ordinal. The key projection authenticates each original declaration layout, selected naming recipe and exact whole-word input, then resolves its original alias context; all retained alternatives must agree. SSA keeps the existing conditional-advice flag and independent closed-lookup/frame checks. The fixed test follows linked to the original activation key and rejects a missing operand. This is conditional diagnostic key correspondence, not a Must destruction, normal unset, physical cell or caller-current alias grant.

## Scope

Compiler original declaration advice and SSA destruction metadata under all six stock authoring profiles. Byte/compiler-primary ambiguity, absent original operands, missing policy and disagreeing source alternatives withdraw the projection. Authored-only logical diagnostic paths and executed statement destruction remain independent.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This question specifies a Rust implementation contract; no native provider observation or Rust execution result is attached.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This question specifies a Rust implementation contract; no native provider observation or Rust execution result is attached.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This question specifies a Rust implementation contract; no native provider observation or Rust execution result is attached.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This question specifies a Rust implementation contract; no native provider observation or Rust execution result is attached.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This question specifies a Rust implementation contract; no native provider observation or Rust execution result is attached.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

This question specifies a Rust implementation contract; no native provider observation or Rust execution result is attached.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

This question specifies a Rust implementation contract; no native provider observation or Rust execution result is attached.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/registry_invocation/declaration_flow.rs](../../../../rust/tcl-compiler/src/registry_invocation/declaration_flow.rs), `DeclarationInvocationFlow`: Retains exact removal operand positions independently of diagnostic names.
- [rust/tcl-compiler/src/command_binding/declaration_layout.rs](../../../../rust/tcl-compiler/src/command_binding/declaration_layout.rs), `declaration_variable_argument_keys`: Authenticates original declaration snapshots and unanimously projects byte keys through their alias context.
- [rust/tcl-compiler/src/place_bridge.rs](../../../../rust/tcl-compiler/src/place_bridge.rs), `ssa_destruction_keys`: Uses typed original keys while preserving the conditional-advice distinction and frame/lookup guards.
- [rust/tcl-compiler/src/command_binding/declaration_layout.rs](../../../../rust/tcl-compiler/src/command_binding/declaration_layout.rs), `command_binding::declaration_layout::tests::original_destruction_operand_keys_follow_the_retained_alias_point` (linked): Requires the original activation key after upvar and rejects an out-of-bounds operand ordinal.

A named test is a coverage binding, not a claim that it executed.

## Replay

The named fixed Rust tests bind this implementation contract. Execute the corresponding crate selectors against one complete current source snapshot. This record supplies no Rust execution receipt, native test result, physical object, normal-completion certificate or cache proof.
