# naming.optimiser.original-tail-call-inventory

Kind: `implementation-contract`

## Problem statement

A display name can denote a different procedure after rename, replacement, aliasing or a namespace change. A text scanner can also miss a quoted or escaped call, count inert brackets, or lose a non-tail recursive call in a decoded expression. Reconstructing the procedure header or reindenting original lines can change name identity and multiline literal values. These ambiguities affect whether a source loop rewrite is justified.

## Question

Does the tail-call optimiser select the exact retained procedure allocation and inventory original and readonly expression calls before editing the original body?

## Conclusion

The inventory joins the original evaluated body and complete procedure allocation, then selects source calls through independently retained current byte publications and original alias target producers. Checked expression grammar inventories original command spans; statically produced expression script words retain readonly parent-byte lineage and never acquire editable coordinates. Unknown target, operand effects or incomplete syntax withdraw the complete inventory. Fixed-arity scalar assignments consume the original native formal binding topology and exact source renderer. A loop edit preserves the entire original header and contiguous braced body, adds explicit continuation and normal fallthrough return, and retains multiline literal bytes. Captured prefixes, rest/reference bindings and missing original owners cannot donate written loop operands. An additional marked source control checks the conditional replacement text for unchanged zero-, one- and two-formal tail calls: continue is present and synthetic lassign or quoted set text is absent across the six selected source catalogues.

## Scope

Rust source-only naming and rewrite contracts; exact original source/configuration/frame/namespace and Registry transition receipts are required. Conditional quiet formal reads remain separate from unknown object rendering or arithmetic coercion. Native object caches, actual body entry and successful runtime completion are not supplied by an inventory or a source allocation. The assertions inspect replacement text only. Despite the test name, they do not observe object retention, release, rendering or reached stores. Native value/frame/completion, observer exclusion, safe edit and actual execution require their independent owners and receipts; no new software result is attached.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/optimiser/tail_call/inventory.rs](../../../../rust/tcl-compiler/src/optimiser/tail_call/inventory.rs), `Inventory::capture`: Exact implementation allocation and complete original command inventory.
- [rust/tcl-compiler/src/command_binding/declaration_lookup.rs](../../../../rust/tcl-compiler/src/command_binding/declaration_lookup.rs), `SourceInvocationBinding::original_declared_definition_reference`: Conditional current byte publication selection with independent operand-effect closure.
- [rust/tcl-compiler/src/command_binding/declaration_lookup.rs](../../../../rust/tcl-compiler/src/command_binding/declaration_lookup.rs), `SourceInvocationBinding::original_expression_command_definition`: Readonly static parent-owned expression script selection.
- [rust/tcl-compiler/src/command_binding/formal_topology.rs](../../../../rust/tcl-compiler/src/command_binding/formal_topology.rs), `OriginalFormalTopology::fixed_scalar_binding_names`: Original fixed-arity native scalar binding destinations.
- [rust/tcl-compiler/src/optimiser/tail_call.rs](../../../../rust/tcl-compiler/src/optimiser/tail_call.rs), `emit_loop_conversion`: Emit the conditional original tail-loop replacement while keeping unchanged formal transfer free of unnecessary assignment text; actual execution/edit prerequisites remain independent.
- [rust/tcl-compiler/src/optimiser/tail_call.rs](../../../../rust/tcl-compiler/src/optimiser/tail_call.rs), `optimiser::tail_call::tests::original_allocation_and_grouped_names_control_tail_rewrites` (linked): Grouped/non-ASCII original names retain header spelling; replaced and sibling allocations and expanded rest calls do not obtain a loop rewrite.
- [rust/tcl-compiler/src/optimiser/tail_call.rs](../../../../rust/tcl-compiler/src/optimiser/tail_call.rs), `optimiser::tail_call::tests::produced_expression_commands_retain_readonly_self_identity` (linked): Non-tail decoded expression self calls, including an alias target, prevent a tail-only loop edit.
- [rust/tcl-compiler/src/optimiser/tail_call.rs](../../../../rust/tcl-compiler/src/optimiser/tail_call.rs), `optimiser::tail_call::tests::unchanged_original_formal_objects_continue_without_releasing_or_rendering_them` (linked): For zero, one and two unchanged source formals across six catalogue selections, the conditional O122 candidate contains continue and omits synthetic lassign or quoted set text.
- [rust/tcl-compiler/src/optimiser/tail_call.rs](../../../../rust/tcl-compiler/src/optimiser/tail_call.rs), `optimiser::tail_call::original_tail_parser_purpose_tests::original_tail_command_slices_keep_bom_and_retained_return_geometry` (linked): Finite source-defined control: original tail command slices keep bom and retained return geometry. Genuine retained Return bracket source, full input/config and original absolute span; BOM command head and FirstClose arguments survive local parse. Expansion/multiple-command/missing parent binding refusals. No tail rewrite permission. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cargo",
  "test",
  "-p",
  "tcl-compiler",
  "optimiser::tail_call::tests::original_allocation_and_grouped_names_control_tail_rewrites",
  "--",
  "--exact"
]
```

These are Rust implementation-contract checks. Native provider answers remain not-tested for this precise Rust question; neighboring native observations establish separate byte, evaluator and object rules. A captured interpreter outcome is not a Rust test pass.
