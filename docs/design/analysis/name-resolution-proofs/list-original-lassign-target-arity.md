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

Status: `observed`. Version: 8.4.20. Build: Actual executable SHA-256 f0e742ccc08a150afd9a0a236ce24c8e12098dae38c0efd46e59f9b1a28bf58a; original SDK/header/library/Makefile/source pins, environment and process argv are in the receipt.. Channel: Fixed original complete ASCII LF lassign-only source-file CLI argument.. Dialect: Tcl.

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

Status: `observed`. Version: 8.5.19. Build: Actual executable SHA-256 e7eb17821aeb7b1f84619f431e89ecff667bc5461fa8e00d67498b6b7d5dd61a; original SDK/header/library/Makefile/source pins, environment and process argv are in the receipt.. Channel: Fixed original complete ASCII LF lassign-only source-file CLI argument.. Dialect: Tcl.

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

Status: `observed`. Version: 8.6.18. Build: Actual executable SHA-256 9b156375d068a58fcfb82a2a0afd170afd2b03912cbb6e588676c7688118c8ff; original SDK/header/library/Makefile/source pins, environment and process argv are in the receipt.. Channel: Fixed original complete ASCII LF lassign-only source-file CLI argument.. Dialect: Tcl.

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

Status: `observed`. Version: 9.0.4. Build: Actual executable SHA-256 f4748357cdbf2762f1c5c05916fa53e54718d087120ae9de9c403bbdfaf764b1; original SDK/header/library/Makefile/source pins, environment and process argv are in the receipt.. Channel: Fixed original complete ASCII LF lassign-only source-file CLI argument.. Dialect: Tcl.

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

Status: `observed`. Version: 9.1.0. Build: Actual executable SHA-256 2a82f721b68f2ed2347ca4e3daa9a56c49c9e3efce77d00f22fc3b2012d8814d; original SDK/header/library/Makefile/source pins, environment and process argv are in the receipt.. Channel: Fixed original complete ASCII LF lassign-only source-file CLI argument.. Dialect: Tcl.

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

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual executable SHA-256 e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806; original SDK/header/library/Makefile/source pins, environment and process argv are in the receipt.. Channel: Fixed original complete ASCII LF lassign-only source-file CLI argument.. Dialect: Jim Tcl.

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

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native interpreter observation establishes this Rust source-advice invariant.

## Exact evidence

- `source` (input): [rust/tcl-registry/tests/data/native_lassign_target_arity_original/probe.tcl](../../../../rust/tcl-registry/tests/data/native_lassign_target_arity_original/probe.tcl). SHA-256 `03dd0d973f283518d7a1dc2a9d17332ccfe82f3ad62ce973b5a4dde200ae0cc8`. Exact original fixed complete ASCII LF source-file script containing seven independently caught forms.
- `capture-runner` (provider): [rust/tcl-registry/tests/data/native_lassign_target_arity_original/capture.py](../../../../rust/tcl-registry/tests/data/native_lassign_target_arity_original/capture.py). SHA-256 `6e739313b414a3ca71c6aa2b27023440e00b7c0a31f77a957796a7e4da301f73`. Exact Root runner, with original absolute queue dependency preserved.
- `provider-queue` (provider): [rust/tcl-registry/tests/data/native_lassign_target_arity_original/queue.json](../../../../rust/tcl-registry/tests/data/native_lassign_target_arity_original/queue.json). SHA-256 `3eb6871f7e4def2770d65bde01d20ccbe7e7840b3128da03ba072c473292db7e`. Exact selected provider SDK/source/library/header/build/environment queue; actual receipts identify the six launched processes.
- `tcl8.4-receipt` (provider): [rust/tcl-registry/tests/data/native_lassign_target_arity_original/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_lassign_target_arity_original/8.4.20/receipt.json). SHA-256 `83db3c856c754815b49064ecafe0ad817d0d54aafa36a248344628754f3ec5e8`. Original process argv/exit, actual patchlevel, source/runner/executable/SDK/environment and raw stream digests; copied input_channel label is not a trace observation.
- `tcl8.4-stdout` (observation): [rust/tcl-registry/tests/data/native_lassign_target_arity_original/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_lassign_target_arity_original/8.4.20/stdout). SHA-256 `55f1169b22e1e3734ecaa6651c06b2b64b49e8b3b333cc70e95207fecb868501`. Exact complete original stdout bytes; seven caught source results remain distinct from outer process status.
- `tcl8.4-stderr` (observation): [rust/tcl-registry/tests/data/native_lassign_target_arity_original/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_lassign_target_arity_original/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact complete original stderr bytes; seven caught source results remain distinct from outer process status.
- `tcl8.5-receipt` (provider): [rust/tcl-registry/tests/data/native_lassign_target_arity_original/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_lassign_target_arity_original/8.5.19/receipt.json). SHA-256 `ceb8224e9ce9dce9839beeac3c6ff689dc0d67681d5cd1ad1c7390e3ead65b3e`. Original process argv/exit, actual patchlevel, source/runner/executable/SDK/environment and raw stream digests; copied input_channel label is not a trace observation.
- `tcl8.5-stdout` (observation): [rust/tcl-registry/tests/data/native_lassign_target_arity_original/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_lassign_target_arity_original/8.5.19/stdout). SHA-256 `6f6a80cfbee81c3b118db0ba3d690679d2b68d969c884490f361f30bcf0d3ba8`. Exact complete original stdout bytes; seven caught source results remain distinct from outer process status.
- `tcl8.5-stderr` (observation): [rust/tcl-registry/tests/data/native_lassign_target_arity_original/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_lassign_target_arity_original/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact complete original stderr bytes; seven caught source results remain distinct from outer process status.
- `tcl8.6-receipt` (provider): [rust/tcl-registry/tests/data/native_lassign_target_arity_original/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_lassign_target_arity_original/8.6.18/receipt.json). SHA-256 `304b86184e13b4bcaef255e973067d604f5a9f0ec24b924070c5a8fd4f239488`. Original process argv/exit, actual patchlevel, source/runner/executable/SDK/environment and raw stream digests; copied input_channel label is not a trace observation.
- `tcl8.6-stdout` (observation): [rust/tcl-registry/tests/data/native_lassign_target_arity_original/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_lassign_target_arity_original/8.6.18/stdout). SHA-256 `4fcb152f0c53b3f51b5d2d074c6c582cf122a74455f24b1e6037c7156a379dcf`. Exact complete original stdout bytes; seven caught source results remain distinct from outer process status.
- `tcl8.6-stderr` (observation): [rust/tcl-registry/tests/data/native_lassign_target_arity_original/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_lassign_target_arity_original/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact complete original stderr bytes; seven caught source results remain distinct from outer process status.
- `tcl9.0-receipt` (provider): [rust/tcl-registry/tests/data/native_lassign_target_arity_original/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_lassign_target_arity_original/9.0.4/receipt.json). SHA-256 `69825e3f66165845155ebf7ba64ef7d0f74f380d4704e4f63965303c63a4b8f9`. Original process argv/exit, actual patchlevel, source/runner/executable/SDK/environment and raw stream digests; copied input_channel label is not a trace observation.
- `tcl9.0-stdout` (observation): [rust/tcl-registry/tests/data/native_lassign_target_arity_original/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_lassign_target_arity_original/9.0.4/stdout). SHA-256 `3b2903d117e6ebd528e6e21dc8eebd42f7930ff0e628a646d26329c5cf19b298`. Exact complete original stdout bytes; seven caught source results remain distinct from outer process status.
- `tcl9.0-stderr` (observation): [rust/tcl-registry/tests/data/native_lassign_target_arity_original/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_lassign_target_arity_original/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact complete original stderr bytes; seven caught source results remain distinct from outer process status.
- `tcl9.1-receipt` (provider): [rust/tcl-registry/tests/data/native_lassign_target_arity_original/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_lassign_target_arity_original/9.1.0/receipt.json). SHA-256 `3a79bdab31550641b385595e4c8187ebb12792cba4377373533b6b82bb320eb2`. Original process argv/exit, actual patchlevel, source/runner/executable/SDK/environment and raw stream digests; copied input_channel label is not a trace observation.
- `tcl9.1-stdout` (observation): [rust/tcl-registry/tests/data/native_lassign_target_arity_original/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_lassign_target_arity_original/9.1.0/stdout). SHA-256 `a3d0588d44776524fc6bbc9864405d8e281fc0584eca998b35968d2a147b6da0`. Exact complete original stdout bytes; seven caught source results remain distinct from outer process status.
- `tcl9.1-stderr` (observation): [rust/tcl-registry/tests/data/native_lassign_target_arity_original/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_lassign_target_arity_original/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact complete original stderr bytes; seven caught source results remain distinct from outer process status.
- `jim-receipt` (provider): [rust/tcl-registry/tests/data/native_lassign_target_arity_original/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_lassign_target_arity_original/jim/receipt.json). SHA-256 `a98e06dc4911d1fddba397124deca1c268eb59724991840e0a951c12f12fea0f`. Original process argv/exit, actual patchlevel, source/runner/executable/SDK/environment and raw stream digests; copied input_channel label is not a trace observation.
- `jim-stdout` (observation): [rust/tcl-registry/tests/data/native_lassign_target_arity_original/jim/stdout](../../../../rust/tcl-registry/tests/data/native_lassign_target_arity_original/jim/stdout). SHA-256 `e0b5980957f59a9517f7cd0025699ec45e885659a7d63578b63806580c4561ed`. Exact complete original stdout bytes; seven caught source results remain distinct from outer process status.
- `jim-stderr` (observation): [rust/tcl-registry/tests/data/native_lassign_target_arity_original/jim/stderr](../../../../rust/tcl-registry/tests/data/native_lassign_target_arity_original/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact complete original stderr bytes; seven caught source results remain distinct from outer process status.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

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

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "/workspace/.proofs/native-list-arity146/capture.py"
]
```

The original runner/process argument vectors retain their capture-machine paths and provider queue dependency. Source, runner, queue and all raw capture bytes are archived unchanged. Replay requires independently verified matching SDK/executable/environment pins and fresh output paths. This offline review launches no native process or Rust tests. Guest script results do not issue a compiler admission, object/header/cache, callback or appliance receipt.
