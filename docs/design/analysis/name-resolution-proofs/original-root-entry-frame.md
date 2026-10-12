# naming.variable.original-root-entry-frame

Kind: `implementation-contract`

## Problem statement

A genuine Native namespace wrapper at the top-level analysis entry can differ from the initial model frame. Treating that difference as a nested call restores incoming unknown variables after independently completed root stores.

## Question

Can an analysis retain its supplied entry frame through root completion while keeping nested caller restoration and unknown observer withdrawal independent?

## Conclusion

The source analysis entry selects its supplied namespace-wrapped frame before walking the root chunk and binds incoming formals in that selected frame. Root completion therefore retains the selected variable context instead of restoring an unselected model parent. Nested body entry and caller restoration are unchanged. Literal contents remain available only through the existing current cell, successful access and observer kernels; an unknown callback still withdraws them. This frame selection supplies no new Native frame, successful invocation, cell contents, source edit or TMM value authority.

## Scope

Rust source analysis entry and retained variable-state contract. The fixed C8.6 captured-entry control compares exact before-read and final namespace/frame identity, a known scalar store/read and an unknown-command counterexample. Its independently captured namespace table and observer premises are not a general fresh global or caller-state assumption. No execution result for this new Rust selector or native experiment is attached.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding.rs](../../../../rust/tcl-compiler/src/command_binding.rs), `analyse_source_in_frame_with_options`: Establish the supplied analysis entry frame before root traversal, separately from nested-call restoration and handler completion.
- [rust/tcl-compiler/src/var_resolve.rs](../../../../rust/tcl-compiler/src/var_resolve.rs), `restore_execution_frame`: Preserve the existing nested caller and outward-effects restoration boundary without applying it to an unselected model entry parent.
- [rust/tcl-compiler/src/command_binding/native_result.rs](../../../../rust/tcl-compiler/src/command_binding/native_result.rs), `command_binding::native_result::tests::original_root_entry_retains_selected_frame_and_withdraws_after_unknown_observer` (linked): Retains exact selected Native root namespace/frame and completed scalar contents at final publication; an unknown command still leaves dynamic bindings and refuses the value.

A named test is a coverage binding, not a claim that it executed.

## Replay

The fixed Rust selector is linked without an attached execution receipt. Existing native address, frame and alias observations remain separate questions and do not prove this implementation contract.
