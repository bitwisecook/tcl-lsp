# naming.source.namespace-inventory-selection-sharing

Kind: `implementation-contract`

## Problem statement

Selecting the same authentic source body and namespace can request mutable access to collections even when their retained contents do not change. Source inventory sharing must preserve exact namespace filtering, original views and refusal boundaries while avoiding unnecessary collection detachment.

## Question

How can an exact original source/namespace selection retain shared inventory storage when its contents are unchanged, while changed facets still detach and unrelated namespaces gain no dispatch or preparation proof?

## Conclusion

SharedSourceInventory::update_if_needed inspects the whole current collection before requesting Arc::make_mut; an unchanged selected facet retains its original shared storage. SourceCommandBindings::selected_source_in_context selects the authentic original script first, then applies the existing exact namespace-incarnation filters. Points and dispatch indexes rebuild only if point membership changes; the retained compiler and implicit-math visits, expression preparations, unrepresented entries, pre-handler failures and other collection fields keep their independent selected origin/namespace conditions. Empty alternative removal and nonempty variable-read alternative selection keep their existing shared context owner and refusal rules. Changed facets detach before mutation, leaving the original view intact. The deterministic storage assertions count eight collection identities across 32 identical whole-source namespace selections and require zero detachments; mixed-namespace selection requires changed point/preparation storage to detach, repeated unchanged selection to share it, and unrelated selection to remain unknown with no expression preparation. These are software collection correspondence controls, not measured elapsed-time improvement, native namespace identity, reached child/body entry, dispatch, variable read, compiler admission or execution.

## Scope

Current conditional source inventory sharing. Two marked fixed controls assert exact shared collection identities and unchanged invocation correspondence, plus mixed-namespace filtering, original-view preservation and unrelated-namespace refusal. Collection detach counts concern these eight selected Rust inventories only; they make no universal allocation, wall-clock speedup or whole deep-test performance claim. All seven native providers are not tested; actual Rust command outcomes remain separate validation receipts.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

## Exact evidence

- `naming-source-namespace-inventory-selection-sharing-origin_inventory.rs` (implementation): [rust/tcl-compiler/src/command_binding/origin_inventory.rs](../../../../rust/tcl-compiler/src/command_binding/origin_inventory.rs). SHA-256 `479a813919f0408628d242082b462156efc8e4bb56fe24c026aa61e3d594892a`. Current source/API owner and fixed marked assertion body; no executed Rust or native provider observation.
- `naming-source-namespace-inventory-selection-sharing-pre_handler_failure.rs` (implementation): [rust/tcl-compiler/src/command_binding/pre_handler_failure.rs](../../../../rust/tcl-compiler/src/command_binding/pre_handler_failure.rs). SHA-256 `91688e2f36e1b036ba357cbb442ef6a33a6f8c21d91c463b9c5c1bdd7b672459`. Current source/API owner and fixed marked assertion body; no executed Rust or native provider observation.
- `naming-source-namespace-inventory-selection-sharing-shared_inventory.rs` (implementation): [rust/tcl-compiler/src/command_binding/shared_inventory.rs](../../../../rust/tcl-compiler/src/command_binding/shared_inventory.rs). SHA-256 `ca79b99c5450149e0eb269d5d05f09e57dd597535f003149f4151ba45be02a49`. Current source/API owner and fixed marked assertion body; no executed Rust or native provider observation.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/shared_inventory.rs](../../../../rust/tcl-compiler/src/command_binding/shared_inventory.rs), `SharedSourceInventory::update_if_needed`: Inspect exact collection contents before requesting copy-on-write mutation; unchanged selected facets preserve storage identity.
- [rust/tcl-compiler/src/command_binding/origin_inventory.rs](../../../../rust/tcl-compiler/src/command_binding/origin_inventory.rs), `SourceCommandBindings::selected_source_in_context`: Filter authentic original source receipts by exact namespace and retain independent variable-read/context refusals; rebuild changed facets only.
- [rust/tcl-compiler/src/command_binding/pre_handler_failure.rs](../../../../rust/tcl-compiler/src/command_binding/pre_handler_failure.rs), `SourceCommandBindings::retain_pre_handler_failure_namespace`: Preserve exact source-origin/namespace failure alternatives and empty-row removal while sharing unchanged inventories.
- [rust/tcl-compiler/src/command_binding/origin_inventory.rs](../../../../rust/tcl-compiler/src/command_binding/origin_inventory.rs), `command_binding::origin_inventory::tests::unchanged_namespace_selection_reuses_original_inventory_without_detachments` (linked): Eight genuine retained collection identities remain shared across 32 identical whole-source namespace selections; exact invocation correspondence and original view remain unchanged. Zero detachments supplies no native frame/body admission or elapsed-time result.
- [rust/tcl-compiler/src/command_binding/origin_inventory.rs](../../../../rust/tcl-compiler/src/command_binding/origin_inventory.rs), `command_binding::origin_inventory::tests::namespace_selection_detaches_changed_points_and_preserves_original_views` (linked): Mixed authentic namespaces detach changed point/preparation facets, retain only the selected namespace, then share a repeated unchanged projection. Original view stays intact; unrelated namespace returns unknown dispatch and no preparation proof.

A named test is a coverage binding, not a claim that it executed.

## Replay

Named Rust coverage is linked, not an executed-result claim. Exact source/executable receipts retain actual outcomes separately. Native evidence and runtime/effect/frame/edit obligations remain independent.
