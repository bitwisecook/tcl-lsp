# naming.bootstrap.original-root-cells

Kind: `native-observation`

## Problem statement

A name catalogue cannot prove that a root variable cell is allocated or defined. Native constructors can allocate undefined special cells, and C arrays differ from Jim dictionaries. The model must distinguish allocation, definition and representation.

## Question

Which root cells and definition/array states are recorded immediately after each actual native interpreter constructor?

## Conclusion

The retained core-roots table records constructor root allocations, including undefined C special cells and C-array versus Jim-dictionary storage. It does not establish complete command registration, cell contents or allocation chronology from bucket order. The marked Runtime child-constructor definition separately checks logical embedding and explicitly selected software native-core child root-cell/command presence under the actual parent recipe.

## Scope

Native bootstrap constructor root tables only. The source control does not launch original CreateInterp, Tcl_Init or Jim constructors and attaches no assertion outcome. Selected software profile/runtime correspondence and bounded root-cell checks do not establish the complete external command roster, private headers, cell contents or allocation chronology. The retained original constructor table remains independent.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `observed`. Channel: Compiled native constructor/header-table probe. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Constructor allocation/definition rows are retained in core-roots.tsv; this is not a Tcl_Init, shell startup or complete command/module inventory.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [rust/tcl-registry/src/special_vars.rs](../../../../rust/tcl-registry/src/special_vars.rs), `NativeBootstrapProtocol`: Native bootstrap constructor root tables only.
- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `Interp::create_child`: Carry the actual selected parent child-constructor purpose and root-cell initialization into the newly allocated software interpreter; retain logical embedding separately.
- [rust/tcl-registry/src/special_vars.rs](../../../../rust/tcl-registry/src/special_vars.rs), `special_vars::native_bootstrap_tests::constructor_root_order_and_undefined_cells_match_native_capture` (linked): The shared Rust purpose owner checks the exact retained native rows independently of their original capture.
- [runtime/rust/src/interp/native_children.rs](../../../../runtime/rust/src/interp/native_children.rs), `interp::native_children::tests::child_constructor_uses_the_explicit_parent_profile_recipe` (linked): Logical child construction retains the actual parent assistance profile and logical argv cell. Six explicitly selected software native-core parents retain profile/runtime-version correspondence while child argv/argc/argv0/tcl_library are absent and set/binary are present; this is an API constructor control, not an original constructor replay.
- [runtime/rust/src/interp/bootstrap_context_tests.rs](../../../../runtime/rust/src/interp/bootstrap_context_tests.rs), `interp::bootstrap_context_tests::original_native_constructor_keeps_selected_inventory_and_checked_host_values` (linked): Finite source-defined control: original native constructor keeps selected inventory and checked host values. Real Host callback, original bootstrap/currency/storage/report/retirement boundary; no external provider or all-Host closure. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.

## Exact original-source349 software outcomes

These finite results belong to the original source349 body/question/document pairs and the owned Runtime executable. Passing, failing, aborted/incomplete and unstarted selections retain their separate meanings. They establish no outcome for another source image or external provider.

| Original selection | Passed | Failed | Aborted or incomplete | Not started |
| --- | ---: | ---: | ---: | ---: |
| `grouped` | 1 | 0 | 0 | 0 |
| `isolated` | 1 | 0 | 0 | 0 |

Exact original body hashes and process outcomes join the unchanged original receipts, streams, frozen source, own inventory/admission and complete executable bytes. The grouped abort is independent of the isolated diagnostics.

## Evidence

- [rust/tcl-registry/tests/data/native_bootstrap/probe.c](../../../../rust/tcl-registry/tests/data/native_bootstrap/probe.c)
- [rust/tcl-registry/tests/data/native_bootstrap/capture-manifest.json](../../../../rust/tcl-registry/tests/data/native_bootstrap/capture-manifest.json)
- [rust/tcl-registry/tests/data/native_bootstrap/core-roots.tsv](../../../../rust/tcl-registry/tests/data/native_bootstrap/core-roots.tsv)
