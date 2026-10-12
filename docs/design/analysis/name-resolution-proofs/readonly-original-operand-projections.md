# Readonly original operand projections

Proof ID: `naming.source.readonly-original-operand-projections`

## Problem statement

A computed name, expanded list child or nested list element can have the same text as a written word while lacking its source geometry and compiler purpose. A consumer that remaps by text or offset may select the wrong argv ordinal, accept a stale document, or give a readonly value an editable word.

## Question

How do consumers retain exact written, computed, expanded and native list-child inputs without reconstructing identity from presentation?

## Scope

Implementation provenance contract for C and Jim source projections and readonly list consumers. Caller image and channel, full configuration, immutable producer currency and full-vector agreement are separate checks. Captured alias operands require their independently positioned allocation owner. No physical argv, interpreter success, compiler hook, formal slot, cell lifetime or editable child span is established.

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

The shared producer retains the complete original image, full LexerConfig and written vector. Effective operands follow the authenticated written/expanded origin map; expanded elements require native list parsing and retained cardinality agreement. Values remain readonly, variable roots cannot stand in for fetched lists, and static container ancestry describes native ordinals without child source spans or edit permission.

## Shared owners and tests

- `rust/tcl-compiler/src/command_binding/original_name_value.rs`: `original_variable_invocation` — Manufacture the complete effective operand vector from genuine written and expanded origins, with separate compiler-purpose receipts.
- `rust/tcl-compiler/src/command_binding/original_name_value.rs`: `original_written_name_input_at_span_in_source` — Require exact caller image/channel and retained word-vector configuration before projecting a current readonly input.
- `rust/tcl-compiler/src/signature_scan/name_value.rs`: `SignatureSourceNameInput` — Keep complete original words, readonly produced values and lexical variable roots distinct during shared list-child and container projections.
- `rust/tcl-compiler/src/command_binding/original_name_value.rs`: `OriginalWrittenSourceWords` — Retain the actual unchanged full parser vector at a dispatch independently of compiler selection and declaration advice.
- `rust/tcl-compiler/src/command_binding/frozen_arguments.rs`: `freeze_word` — Freeze authentic produced native bytes and parse expansion children under their selected native list protocol without logical-text re-encoding.

- `rust/tcl-compiler/src/command_binding/original_name_value.rs`: `command_binding::original_name_value::graph_tests::original_retained_span_input_requires_exact_image_config_and_word` — The same retained computed input is available only under the complete current caller image, input channel, full configuration and exact word extent. Changed image/config and partial spans decline.
- `rust/tcl-compiler/src/command_binding/original_name_value.rs`: `command_binding::original_name_value::graph_tests::original_effective_head_keeps_computed_and_expanded_producers` — Computed command heads remain readonly current values; native expanded heads use their actual list child, including skipped empty expansions, and cannot acquire a complete word. Unknown contents invalidate the computed head.
- `rust/tcl-compiler/src/signature_scan/name_value.rs`: `signature_scan::name_value::tests::original_input_list_children_keep_parent_and_reject_lexical_roots` — Native list children retain the exact static parent and nested ordinal path, share one child manufacturer, lack editable keys and reject lexical variable roots and out-of-range ordinals.

- `rust/tcl-compiler/src/command_binding/frozen_arguments.rs`: `command_binding::frozen_arguments::tests::frozen_native_argv_keeps_opaque_units_and_expansion_ordinals` — Static and expanded argv retain the exact original native bytes; opaque list children preserve full written parent and ordinal, remain readonly and are not converted to Unicode spelling.

## Reconfirmation

Run the exact named Rust selectors through the maintained workspace test setup. This record describes provenance assertions and contains no executed test or native interpreter result.

- `rust/tcl-compiler/src/command_binding/command_observers.rs`: `command_binding::command_observers::tests::original_trace_prefix_projection_requires_actual_script_argv_equivalence` — Authentic copied trace prefixes require counted script/list argv equivalence; arbitrary script source does not inherit list authority.

- `rust/tcl-compiler/src/command_binding/command_observers.rs`: `command_binding::command_observers::tests::original_trace_prefix_projection_requires_actual_script_argv_equivalence` — Authentic copied trace prefixes require counted script/list argv equivalence; arbitrary script source does not inherit list authority.
