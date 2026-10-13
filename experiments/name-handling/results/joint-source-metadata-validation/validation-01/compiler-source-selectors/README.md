# Compiler source-selector controls

Each listed exact control ran once on its pinned current library binary from
the [same selected build](../build/README.md). Full stdout/stderr and exact
invocations are retained beside [validation](validation.json).

| Exact control | Passed | Failed | Filtered |
| --- | --- | --- | --- |
| `registry_invocation::source_structure::tests::original_source_schema_keeps_named_selector_and_handler_argument_shape` | 1 | 0 | 9562 |
| `registry_invocation::source_structure::tests::original_source_schema_withholds_dynamic_and_shadowed_selector_claims` | 1 | 0 | 9562 |

Both new controls pass for the current source-purpose composer. They retain
actual named selectors in source assistance while preserving the separate
private handler-argument shape, and decline unsupported dynamic/shadowed
selector claims. These two controls do not validate the original Core after211
reference fixture, native handler acceptance or every source consumer. The
common compiled source also includes metadata and corrected VM fixture
changes. No canonical proof digest or provider status is renewed here.

All 11,803 conservative source/data hashes and selected binary hashes stayed
unchanged across the one build and all controls. Standalone Runtime is
excluded; no source edit, retry or full suite accompanied this capture.
