# naming.namespace.original-counted-path-transfer

Kind: `implementation-contract`

## Problem statement

A namespace command path is a whole native List whose elements address existing namespaces. Reporting Strings and partial list parsing cannot establish counted namespace identity or complete path replacement.

## Question

How does the selected C8.5–9.1 source transfer retain one complete original path operand, resolve every element to a unique existing namespace identity and install only a complete path?

## Conclusion

The selected namespace SetPath transition retains its genuine original operand through OriginalCommandOperands. Its independently selected C8.5–9.1 name policy supplies native list decoding and namespace-address purposes under the current retained namespace geometry. Every decoded element must select exactly one existing namespace incarnation before the source path is replaced. Order and duplicate elements remain original list order; an empty complete list clears the source path. Unknown, missing, ambiguous, stale or malformed operands leave the path unknown. No namespace, runtime token or storage is created.

## Scope

Rust source-state transfer with a genuine complete original word and current whole source, selected C8.5–9.1 policy and existing namespace identities. C8.4 and Jim provide no path transfer through this owner. The separately recorded ASCII command-path-precedence observation establishes only its original script result; it does not establish opaque native operands, actual table identity, cache installation, runtime path execution or Normal completion for these Rust controls.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

No interpreter execution is claimed for this Rust source-transfer invariant.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/original_command_table.rs](../../../../rust/tcl-compiler/src/command_binding/original_command_table.rs), `OriginalCommandOperands::capture`: Retain the complete actual SetPath List operand at its selected Registry transition ordinal.
- [rust/tcl-compiler/src/command_binding/original_namespace_path.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_path.rs), `select`: Decode the whole original List with the independently selected C policy and resolve each unique existing namespace identity before any source-path update.
- [rust/tcl-compiler/src/command_binding/namespace_slots.rs](../../../../rust/tcl-compiler/src/command_binding/namespace_slots.rs), `ModuleCommandBindings::original_namespace_key_for_path`: Select existing namespace incarnations by exact retained geometry; absent or ambiguous identities supply no path element.
- [rust/tcl-compiler/src/command_binding.rs](../../../../rust/tcl-compiler/src/command_binding.rs), `apply_namespace_path`: Install the whole resolved vector atomically or retain explicit unknown path state, without allocating namespaces.
- [rust/tcl-compiler/src/command_binding/original_namespace_path.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_path.rs), `command_binding::original_namespace_path::tests::original_namespace_path_keeps_whole_lists_and_existing_identities` (linked): Four C source contexts retain the exact B,A,B element sequence over actual prior namespace allocations, and a reached empty List clears the complete path.
- [rust/tcl-compiler/src/command_binding/original_namespace_path.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_path.rs), `command_binding::original_namespace_path::tests::original_namespace_path_declines_missing_malformed_and_dynamic_elements` (linked): Four C source contexts retain uncertainty for a missing element, malformed List or dynamic operand; no missing namespace is allocated. Changed whole source refuses the retained source image.
- [rust/tcl-compiler/src/command_binding/original_namespace_path.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_path.rs), `command_binding::original_namespace_path::tests::original_namespace_path_requires_current_original_policy_and_unique_incarnations` (linked): An actual C8.6 complete original word resolves its prior allocated namespace. Counterfactual duplicate incarnation, unknown selected namespace, foreign C9.1 policy, C8.4/Jim policy and changed original source refuse without installing a path.
