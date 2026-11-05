# naming.list.original-lassign-target-arity

Kind: `native-observation`

## Problem statement

A target-free lassign invocation can be a valid guest operation in one release and an arity error in another. Ordinary handler grammar, procedure-script results and native compiler admission are separate purposes; Jim also reports a different usage synopsis.

## Question

For the seven exact original ASCII lassign source forms, which captured providers accept a target-free list, what error precedes malformed-list inspection, and what exact usage or unavailable-command result does each provider return?

## Conclusion

C8.5 and current Jim require at least one target: all target-free forms return the recorded arity error before the malformed list is inspected. C8.6, C9.0 and C9.1 accept literal, dynamic-head and procedure target-free lists and return A B; the malformed list then reports unmatched open brace in list and an empty target-free list returns empty. The one-target form returns B in all five accepting providers. C8.4 returns invalid command name "lassign" for all seven forms. C8.5 usage is lassign list varName ?varName ...?; modern C usage is lassign list ?varName ...?; current Jim usage is lassign varList list ?varName ...?. The shared ordinary-invocation floor and usage metadata are selected from the actual InvocationDialect; both ports check that arity before list conversion. The Compiler admission rule remains independent.

## Scope

Seven finite forms in one fixed ASCII LF source-file CLI probe per actual C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and current Jim process: no operands, literal/dynamic-head/procedure target-free, malformed target-free, one target and empty target-free. A successful procedure result does not establish a native compiler hook or target-free ListAssignment admission. No original object/header/cache identity, opaque/zero input, callback behavior, Tcllib shim or BIG-IP result is measured. The original receipts contain a copied input_channel sentence about trace registration; the exact probe, argument vectors and boundary fields identify these lassign-only source controls.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Fixed original complete ASCII LF lassign-only source-file CLI argument. Version: tcl8.4 8.4.20.

Exact original stdout:

```text
PATCHLEVEL 8.4.20
RESULT no_operands 1 {invalid command name "lassign"}
RESULT literal_no_target 1 {invalid command name "lassign"}
RESULT dynamic_no_target 1 {invalid command name "lassign"}
RESULT compiled_no_target 1 {invalid command name "lassign"}
RESULT malformed_no_target 1 {invalid command name "lassign"}
RESULT literal_one_target 1 {invalid command name "lassign"}
RESULT empty_no_target 1 {invalid command name "lassign"}
```

Outer process status 0; stderr empty. These finite guest results supply no target-free native compiler admission or physical object/cache receipt.

### tcl8.5

Status: `observed`. Channel: Fixed original complete ASCII LF lassign-only source-file CLI argument. Version: tcl8.5 8.5.19.

Exact original stdout:

```text
PATCHLEVEL 8.5.19
RESULT no_operands 1 {wrong # args: should be "lassign list varName ?varName ...?"}
RESULT literal_no_target 1 {wrong # args: should be "lassign list varName ?varName ...?"}
RESULT dynamic_no_target 1 {wrong # args: should be "lassign list varName ?varName ...?"}
RESULT compiled_no_target 1 {wrong # args: should be "lassign list varName ?varName ...?"}
RESULT malformed_no_target 1 {wrong # args: should be "lassign list varName ?varName ...?"}
RESULT literal_one_target 0 B
RESULT empty_no_target 1 {wrong # args: should be "lassign list varName ?varName ...?"}
```

Outer process status 0; stderr empty. These finite guest results supply no target-free native compiler admission or physical object/cache receipt.

### tcl8.6

Status: `observed`. Channel: Fixed original complete ASCII LF lassign-only source-file CLI argument. Version: tcl8.6 8.6.18.

Exact original stdout:

```text
PATCHLEVEL 8.6.18
RESULT no_operands 1 {wrong # args: should be "lassign list ?varName ...?"}
RESULT literal_no_target 0 {A B}
RESULT dynamic_no_target 0 {A B}
RESULT compiled_no_target 0 {A B}
RESULT malformed_no_target 1 {unmatched open brace in list}
RESULT literal_one_target 0 B
RESULT empty_no_target 0 {}
```

Outer process status 0; stderr empty. These finite guest results supply no target-free native compiler admission or physical object/cache receipt.

### tcl9.0

Status: `observed`. Channel: Fixed original complete ASCII LF lassign-only source-file CLI argument. Version: tcl9.0 9.0.4.

Exact original stdout:

```text
PATCHLEVEL 9.0.4
RESULT no_operands 1 {wrong # args: should be "lassign list ?varName ...?"}
RESULT literal_no_target 0 {A B}
RESULT dynamic_no_target 0 {A B}
RESULT compiled_no_target 0 {A B}
RESULT malformed_no_target 1 {unmatched open brace in list}
RESULT literal_one_target 0 B
RESULT empty_no_target 0 {}
```

Outer process status 0; stderr empty. These finite guest results supply no target-free native compiler admission or physical object/cache receipt.

### tcl9.1

Status: `observed`. Channel: Fixed original complete ASCII LF lassign-only source-file CLI argument. Version: tcl9.1 9.1.0.

Exact original stdout:

```text
PATCHLEVEL 9.1.0
RESULT no_operands 1 {wrong # args: should be "lassign list ?varName ...?"}
RESULT literal_no_target 0 {A B}
RESULT dynamic_no_target 0 {A B}
RESULT compiled_no_target 0 {A B}
RESULT malformed_no_target 1 {unmatched open brace in list}
RESULT literal_one_target 0 B
RESULT empty_no_target 0 {}
```

Outer process status 0; stderr empty. These finite guest results supply no target-free native compiler admission or physical object/cache receipt.

### jim

Status: `observed`. Channel: Fixed original complete ASCII LF lassign-only source-file CLI argument. Version: jim 0.84-9-g5bac7c9.

Exact original stdout:

```text
PATCHLEVEL 0.84-9-g5bac7c9
RESULT no_operands 1 {wrong # args: should be "lassign varList list ?varName ...?"}
RESULT literal_no_target 1 {wrong # args: should be "lassign varList list ?varName ...?"}
RESULT dynamic_no_target 1 {wrong # args: should be "lassign varList list ?varName ...?"}
RESULT compiled_no_target 1 {wrong # args: should be "lassign varList list ?varName ...?"}
RESULT malformed_no_target 1 {wrong # args: should be "lassign varList list ?varName ...?"}
RESULT literal_one_target 0 B
RESULT empty_no_target 1 {wrong # args: should be "lassign varList list ?varName ...?"}
```

Outer process status 0; stderr empty. These finite guest results supply no target-free native compiler admission or physical object/cache receipt.

### bigip

Status: `not-tested`.

No native interpreter observation establishes this Rust source-advice invariant.

## Consumer bindings

- [rust/tcl-registry/src/native_list_assignment.rs](../../../../rust/tcl-registry/src/native_list_assignment.rs), `InvocationDialect::list_assignment_invocation`: Select the ordinary handler arity/usage at the actual point; unknown, hosted and conflicting points decline.
- [rust/tcl-registry/src/native_list_assignment.rs](../../../../rust/tcl-registry/src/native_list_assignment.rs), `NativeListAssignmentInvocation::arity`: Expose the selected required-target floor independently of Compiler admission.
- [rust/tcl-registry/src/native_list_assignment.rs](../../../../rust/tcl-registry/src/native_list_assignment.rs), `NativeListAssignmentInvocation::accepts`: Check actual operation cardinality before conversion.
- [rust/tcl-registry/src/native_list_assignment.rs](../../../../rust/tcl-registry/src/native_list_assignment.rs), `NativeListAssignmentInvocation::usage`: Retain exact selected C/Jim usage.
- [rust/tcl-registry/src/native_list_assignment.rs](../../../../rust/tcl-registry/src/native_list_assignment.rs), `operation_arity`: Share ordinary operation arity with selected/legacy Registry metadata.
- [rust/tcl-registry/src/resolved_invocation.rs](../../../../rust/tcl-registry/src/resolved_invocation.rs), `resolve_invocation_semantics`: Use the shared ordinary operation floor in the authentic selected semantics.
- [rust/tcl-registry/src/registry.rs](../../../../rust/tcl-registry/src/registry.rs), `ResolvedCall::arity_for_arguments`: Expose the same ordinary operation arity for its retained actual context.
- [runtime/rust/src/cmd_list.rs](../../../../runtime/rust/src/cmd_list.rs), `lassign`: Check the actual selected ordinary handler floor before list conversion.
- [rust/tcl-vm/src/cmd_list.rs](../../../../rust/tcl-vm/src/cmd_list.rs), `cmd_lassign`: Check the actual selected ordinary handler floor before list conversion.
- [rust/tcl-registry/src/native_list_assignment.rs](../../../../rust/tcl-registry/src/native_list_assignment.rs), `native_list_assignment::tests::selected_handler_floor_and_usage_preserve_measured_native_rows` (linked): Actual C8.5/Jim required target and C8.6–9.1 optional-target floors and exact usage are selected; C8.4/unknown/hosted/conflicting inputs decline. Linked implementation obligation only.
- [rust/tcl-registry/src/native_list_assignment.rs](../../../../rust/tcl-registry/src/native_list_assignment.rs), `native_list_assignment::tests::selected_and_legacy_signature_views_share_operation_arity` (linked): The typed selected and legacy signature projections share the same ordinary operation arity without donating Compiler admission.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::list_assignment_arity_matches_five_original_source_controls` (linked): Replay the exact seven original complete source forms for each of five C providers; compare full output after independent PATCHLEVEL reporting. No native Compiler admission or new native launch is inferred.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::list_assignment_arity_matches_current_jim_original_source_controls` (linked): Replay the exact original seven Jim source forms and compare its actual required-target usage and results after independent PATCHLEVEL reporting; no current Rust pass is inferred.
- [rust/tcl-vm/src/interp/native_variable_names.rs](../../../../rust/tcl-vm/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::list_assignment_arity_matches_five_original_source_controls` (linked): Replay the exact seven original complete source forms for each of five C providers; compare full output after independent PATCHLEVEL reporting. No native Compiler admission or new native launch is inferred.
- [rust/tcl-vm/src/interp/native_variable_names.rs](../../../../rust/tcl-vm/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::list_assignment_arity_matches_current_jim_original_source_controls` (linked): Replay the exact original seven Jim source forms and compare its actual required-target usage and results after independent PATCHLEVEL reporting; no current Rust pass is inferred.

## Evidence

- [rust/tcl-registry/tests/data/native_lassign_target_arity_original](../../../../rust/tcl-registry/tests/data/native_lassign_target_arity_original)
- [rust/tcl-registry/tests/data/native_lassign_target_arity_original/8.4.20](../../../../rust/tcl-registry/tests/data/native_lassign_target_arity_original/8.4.20)
- [rust/tcl-registry/tests/data/native_lassign_target_arity_original/8.5.19](../../../../rust/tcl-registry/tests/data/native_lassign_target_arity_original/8.5.19)
- [rust/tcl-registry/tests/data/native_lassign_target_arity_original/8.6.18](../../../../rust/tcl-registry/tests/data/native_lassign_target_arity_original/8.6.18)
- [rust/tcl-registry/tests/data/native_lassign_target_arity_original/9.0.4](../../../../rust/tcl-registry/tests/data/native_lassign_target_arity_original/9.0.4)
- [rust/tcl-registry/tests/data/native_lassign_target_arity_original/9.1.0](../../../../rust/tcl-registry/tests/data/native_lassign_target_arity_original/9.1.0)
- [rust/tcl-registry/tests/data/native_lassign_target_arity_original/jim](../../../../rust/tcl-registry/tests/data/native_lassign_target_arity_original/jim)
