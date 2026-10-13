# Typed metadata and consumer controls

Each listed exact control ran once on its pinned current library binary from
the [same selected build](../build/README.md). Full stdout/stderr and exact
invocations are retained beside [validation](validation.json).

| Exact control | Passed | Failed | Filtered |
| --- | --- | --- | --- |
| `source_configuration_reference::tests::selected_configuration_layout_keeps_unknown_item_and_original_ordinal` | 0 | 1 | 1758 |
| `source_configuration_reference::tests::selected_configuration_layout_declines_unknown_control_and_cardinality` | 1 | 0 | 1758 |
| `source_object::tests::original_data_group_operands_keep_unknown_item_and_refuse_unknown_reference` | 1 | 0 | 70 |
| `walker::tests::extracts_pool_snatpool_and_datagroup_refs` | 1 | 0 | 70 |
| `walker::tests::braced_expr_command_substitution_keeps_its_data_group_reference` | 0 | 1 | 70 |
| `walker::case_list_tests::original_object_reference_consumers_ignore_dto_labels_and_runtime_value_guesses` | 1 | 0 | 70 |
| `loader::tests::subcommand_source_configuration_reference_loads_and_omission_abstains` | 1 | 0 | 263 |
| `loader::tests::invalid_source_configuration_reference_withdraws_the_descriptor` | 1 | 0 | 263 |
| `loader::tests::source_configuration_reference_has_static_and_evaluated_loader_parity` | 1 | 0 | 263 |
| `catalogue::tests::enum_witnesses_compile_and_hold` | 1 | 0 | 263 |
| `catalogue::tests::catalogue_keys_are_unique` | 1 | 0 | 263 |
| `render_spectcl::tests::authored_source_configuration_reference_survives_render_load_and_draft` | 1 | 0 | 267 |

The Registry positive control fails at the DynamicNonOption ITEM case:
actual argument3/DataGroup is correct, but selection_unknown is true against
expected false. Its later operator loop was not reached. The initial failure
is preserved unchanged and that control was not repeated. The existing
braced-expression walker control fails at walker.rs424 with [] against
["/Common/braced_dg"]. All other listed metadata controls passed. The metadata
suite is not green. The original fixtures/assertions remain unchanged; new
controls retain their fixed expectations. New guard refinements and braced
first-loss diagnosis remain separate future work.

The passed loader and Studio controls establish declarative source metadata
load/absence/invalid withdrawal, static/interpreted/template parity, native/
rendered/reloaded declaration equality and typed readonly projection. They
establish no Native handler, actual accepted argv, live configuration object,
F5 worker or interpreter naming conformance. Standalone Studio integration
and MCP generator controls were not run in this capture.

All 11,803 conservative source/data hashes and selected binary hashes stayed
unchanged across the one build and all controls. Standalone Runtime is
excluded; no source edit, retry or full suite accompanied this capture.
