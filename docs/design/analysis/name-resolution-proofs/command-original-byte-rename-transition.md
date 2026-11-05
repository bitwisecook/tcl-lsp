# naming.command.original-byte-rename-transition

Kind: `implementation-contract`

## Problem statement

A known native byte rename destination has no Unicode literal facet. Treating that absence as an unknown destination discards a represented move before the independent intrinsic handler can retain its source allocation and normal successor.

## Question

Does the Registry rename transition resolver select the actual C destination input extent while retaining complete original byte operands and their effective argument positions?

## Conclusion

The resolver independently selects the retained C rename destination recipe, derives namespace spelling from its selected CString extent, and emits a move with both original byte operands and ordinals intact. A selected empty destination emits deletion; deletion and its residual effects do not borrow the intrinsic move completion receipt. Missing policies and the context-dependent Jim byte case remain unknown. Actual occupancy, destination existence, observer absence and post-transfer allocation are checked by the separate move owner.

## Scope

Rust Registry transition selection for known native byte operands under C Tcl 8.4 through 9.1. Pure tests distinguish opaque units, modified NUL, raw NUL extent, qualifier geometry, operand ordinals and missing-policy refusal. The modeled source move test uses original document surrogate escapes, not a counted native interpreter call. This record asserts no native guest execution, namespace creation, deletion completion, body entry or physical table grant.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

No guest execution of this Rust implementation question is claimed.

## Consumer bindings

- [rust/tcl-registry/src/state_transition.rs](../../../../rust/tcl-registry/src/state_transition.rs), `command_binding::RENAMES_COMMANDS`: One canonical move/delete destination transition resolver retaining effective native byte operands and independently selected input extent.
- [rust/tcl-registry/src/state_transition.rs](../../../../rust/tcl-registry/src/state_transition.rs), `namespace_qualifiers_bytes`: Shared counted qualifier geometry after the caller independently selects the operation input extent.
- [rust/tcl-compiler/src/command_binding/original_command_mutation.rs](../../../../rust/tcl-compiler/src/command_binding/original_command_mutation.rs), `OriginalCommandMove`: Independent actual occupied source, vacant existing destination, quiet intrinsic handler and preserved post-transfer allocation completion checks.
- [rust/tcl-registry/src/state_transition.rs](../../../../rust/tcl-registry/src/state_transition.rs), `state_transition::tests::original_byte_rename_transitions_select_destination_extent_and_keep_ordinals` (linked): Selected C rename extent preserves opaque and modified-NUL destinations, qualifier ordinals and complete original operands; raw NUL selects deletion and absent or Jim context remains unknown.
- [rust/tcl-compiler/src/command_binding/original_command_mutation.rs](../../../../rust/tcl-compiler/src/command_binding/original_command_mutation.rs), `command_binding::original_command_mutation::tests::original_byte_command_move_keeps_the_original_definition_and_complete_world` (linked): The separately closed source intrinsic move retains the genuine original procedure allocation at an opaque destination for every selected C release.
