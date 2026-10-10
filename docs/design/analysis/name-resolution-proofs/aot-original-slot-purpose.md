# naming.variable.aot-original-slot-purpose

Kind: `implementation-contract`

## Problem statement

Escape summaries and reporting formal names carry String ordinals, while original native variable cells and compiled layouts retain counted names, anonymous slots, frame owners and layout incarnations. Matching the labels cannot establish a native slot or a proposed positional formal binding.

## Question

Which original formal or actual native layout receipt may select an AOT argument or materialisable slot without borrowing String escape ordinals?

## Conclusion

Original fixed scalar argument slots retain the actual ParamList input and counted names in argument order. Defaults, rest, caller links, duplicate bindings and compiler first-primary collisions withdraw positional argument selection. Retained native frame slots require the actual native compilation entry, matching procedure activation, selected byte primary, complete layout owner and incarnation, and actual ordinal including anonymous slots. Every represented point must retain that same selected local activation, exact namespace owner and independently selected entry name policy; quiet facts from a foreign frame cannot use its slot. Common AOT keeps source-formal, sealed-program and native-frame authorities distinct; typed cells cannot borrow escape-label ordinals. WASM direct-call parameter admission consumes original formal slots, and NativeLowering does not demote an actual native entry from a String escape summary and retains runtime trace checks when that native frame may have external observers absent from source reporting labels. These receipts supply neither current contents nor observer closure, Normal completion, physical source-local allocation, source edits or TMM values.

## Scope

Rust AOT and backend slot admission contract. Fixed original ParamList inputs cover six selected C/Jim protocols. Native layout controls are explicit Rust fixtures and are not native header captures. The separate compiled-versus-runtime-root-purpose question binds retained C8.6/9.0/9.1 comparison results only; it does not establish a physical layout or current activation. No execution receipt for the new Rust selectors is attached.

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

- [rust/tcl-compiler/src/var_escape/original_slots.rs](../../../../rust/tcl-compiler/src/var_escape/original_slots.rs), `OriginalScalarArgumentSlots`: Retain exact original fixed scalar formal bindings and source calling-convention ordinals without granting a physical compiled-local layout.
- [rust/tcl-compiler/src/var_escape/original_slots.rs](../../../../rust/tcl-compiler/src/var_escape/original_slots.rs), `OriginalNativeFrameSlot`: Select actual retained native layout primary, ordinal, activation owner and layout currency; check each represented point against the actual selected frame, exact namespace owner and independent entry policy without granting observer or value authority.
- [rust/tcl-compiler/src/common_aot_plan.rs](../../../../rust/tcl-compiler/src/common_aot_plan.rs), `materialisable_slot_allocation`: Project typed SSA cells through original formal, sealed-program or actual native slot owners; String ordinals remain authored analysis metadata.
- [rust/tcl-compiler/src/codegen/wasm/backend.rs](../../../../rust/tcl-compiler/src/codegen/wasm/backend.rs), `direct_proc_eligible`: Validate direct-call positional parameters through the original byte-formal owner before using reporting header labels.
- [rust/tcl-compiler/src/native_lowering/lower.rs](../../../../rust/tcl-compiler/src/native_lowering/lower.rs), `Lowerer`: Keep actual native entry storage and runtime observer checks separate from String escape-summary demotion and source trace-label inventories.
- [rust/tcl-compiler/src/var_escape/original_slots.rs](../../../../rust/tcl-compiler/src/var_escape/original_slots.rs), `var_escape::original_slots::tests::original_argument_slots_keep_counted_order_and_reject_nonpositional_bindings` (linked): Retains exact counted formal keys and source order; rejects authored escape spellings as keys, duplicates, defaults, rest and Jim caller-link bindings.
- [rust/tcl-compiler/src/var_escape/original_slots.rs](../../../../rust/tcl-compiler/src/var_escape/original_slots.rs), `var_escape::original_slots::tests::original_native_slots_require_selected_primary_frame_and_current_layout` (linked): Selects exact native primary ordinals including anonymous slots; rejects comparison aliases, authored labels, foreign activation, missing protocol, different owner, changed epoch and a separately declared procedure body.
- [rust/tcl-compiler/src/native_lowering/lower.rs](../../../../rust/tcl-compiler/src/native_lowering/lower.rs), `native_lowering::lower::original_entry_storage_tests::original_native_entry_keeps_cells_and_runtime_trace_guards_despite_escape_labels` (linked): An independently retained native entry ignores the authored String slot demotion and keeps its runtime trace guard, including when source labels name a trace; explicit authored synthetic IR retains its separate slot and unguarded path.
- [rust/tcl-compiler/src/var_escape/original_slots.rs](../../../../rust/tcl-compiler/src/var_escape/original_slots.rs), `var_escape::original_slots::tests::original_native_slot_usage_requires_the_actual_selected_activation` (linked): Uses an original source point under the captured entry frame; retained layout equality survives independent foreign activation, frame kind, frame selector, missing namespace and missing name-policy controls, while the slot refuses those uses. Changing presentation alone does not change activation correspondence.

A named test is a coverage binding, not a claim that it executed.

## Replay

Fixed Rust selectors are linked without an attached execution receipt. Physical native layout fixtures and separate empirical compiler comparison outcomes do not grant activation, observer, value, Normal, relocation or edit authority.
