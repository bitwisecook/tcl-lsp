# naming.source.namespace-inventory-selection-sharing

Kind: `implementation-contract`

## Problem statement

Selecting the same authentic source body and namespace can request mutable access to collections even when their retained contents do not change. Source inventory sharing must preserve exact namespace filtering, original views and refusal boundaries while avoiding unnecessary collection detachment. Repeated body queries can also filter the same completed immutable inventory. Reusing a selection from a different source origin, namespace incarnation or ancestor can restore observations that the selected child does not own; sharing must leave per-body ranges and current execution guards independent.

## Question

How can an exact original source/namespace selection retain shared inventory storage when its contents are unchanged, while changed facets still detach and unrelated namespaces gain no dispatch or preparation proof? Within one completed immutable inventory, do complete origin and typed namespace keys share the exact cold structural selection while filtered children and independent body ranges retain their own observations?

## Conclusion

SharedSourceInventory::update_if_needed inspects the whole current collection before requesting Arc::make_mut; an unchanged selected facet retains its original shared storage. SourceCommandBindings::selected_source_in_context selects the authentic original script first, then applies the existing exact namespace-incarnation filters. Points and dispatch indexes rebuild only if point membership changes; the retained compiler and implicit-math visits, expression preparations, unrepresented entries, pre-handler failures and other collection fields keep their independent selected origin/namespace conditions. Empty alternative removal and nonempty variable-read alternative selection keep their existing shared context owner and refusal rules. Changed facets detach before mutation, leaving the original view intact. The deterministic storage assertions count eight collection identities across 32 identical whole-source namespace selections and require zero detachments; mixed-namespace selection requires changed point/preparation storage to detach, repeated unchanged selection to share it, and unrelated selection to remain unknown with no expression preparation. These are software collection correspondence controls, not measured elapsed-time improvement, native namespace identity, reached child/body entry, dispatch, variable read, compiler admission or execution. RetainedSourceBodyInventory keeps a completed immutable SourceCommandBindings owner and memoizes only its existing namespace-wide structural selector under full source-origin and typed namespace equality. Matching entries share the selected immutable inventory. Each selected child has its own memo over its filtered owner, so it cannot recover ancestor or sibling observations. Three additional finite linked controls compare complete cold-selector output, actual Arc sharing, one software capture for two requests, independent per-body preparation ranges, foreign/filtered empty selections and distinct held Rust VM namespace incarnations. These definitions attach no matching owned assertion outcome. They establish neither measured time or memory improvement nor original C namespace identity, physical reference counts or present Native admission.

## Scope

Current conditional source inventory sharing. Two marked fixed controls assert exact shared collection identities and unchanged invocation correspondence, plus mixed-namespace filtering, original-view preservation and unrelated-namespace refusal. Collection detach counts concern these eight selected Rust inventories only; they make no universal allocation, wall-clock speedup or whole deep-test performance claim. All seven native providers are not tested; actual Rust command outcomes remain separate validation receipts. The readonly memo is confined to one immutable owner retaining its complete source interpretation, input/store/configuration/origin and typed namespace receipts. It performs no fresh parse or registry reconstruction and executes no callback under the memo lock. Foreign and filtered selections retain the existing cold-selector empty-inventory behavior. Original body mapping/ranges, full current metadata/input, selected handler policy and Native generation/admission remain independently checked. The separate complete Logical-input/withdrawn-owner control belongs to original-analysis-metadata-context and is not rebound here.

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

- `naming-source-namespace-inventory-selection-sharing-origin_inventory.rs` (implementation): [rust/tcl-compiler/src/command_binding/origin_inventory.rs](../../../../rust/tcl-compiler/src/command_binding/origin_inventory.rs). SHA-256 `3ae89834cca6d621f64291636489d8a6417b436ee42f2fd52ffb4729d2601e23`. Current source/API owner and fixed marked assertion body; no executed Rust or native provider observation.
- `naming-source-namespace-inventory-selection-sharing-pre_handler_failure.rs` (implementation): [rust/tcl-compiler/src/command_binding/pre_handler_failure.rs](../../../../rust/tcl-compiler/src/command_binding/pre_handler_failure.rs). SHA-256 `82f1e3fc7dfd08dc4a5506007b5f2a25024ac7a364d7e0cf1ab1b4c6f54d1ab7`. Current source/API owner and fixed marked assertion body; no executed Rust or native provider observation.
- `naming-source-namespace-inventory-selection-sharing-shared_inventory.rs` (implementation): [rust/tcl-compiler/src/command_binding/shared_inventory.rs](../../../../rust/tcl-compiler/src/command_binding/shared_inventory.rs). SHA-256 `ca79b99c5450149e0eb269d5d05f09e57dd597535f003149f4151ba45be02a49`. Current source/API owner and fixed marked assertion body; no executed Rust or native provider observation.
- `implementation-source-0` (implementation): [rust/tcl-compiler/src/command_binding/retained_body_inventory.rs](../../../../rust/tcl-compiler/src/command_binding/retained_body_inventory.rs). SHA-256 `936c422c9abe44c837a665ac04e877edaf94cb7c56b2b9350a51c10528d291c6`. Exact retained source inventory, cold selector, typed namespace or lowering consumer. This source/API evidence supplies no matching owned assertion outcome, measured time/allocation gain, external observation or present Native admission.
- `implementation-source-1` (implementation): [rust/tcl-compiler/src/command_binding/namespace_context.rs](../../../../rust/tcl-compiler/src/command_binding/namespace_context.rs). SHA-256 `f10f446e0076bac46970a2c63d4c28562a16b6d70a6e064734fad17d5aad4268`. Exact retained source inventory, cold selector, typed namespace or lowering consumer. This source/API evidence supplies no matching owned assertion outcome, measured time/allocation gain, external observation or present Native admission.
- `implementation-source-2` (implementation): [rust/tcl-compiler/src/lowering/mod.rs](../../../../rust/tcl-compiler/src/lowering/mod.rs). SHA-256 `9ed2cb7441b9e4ce0abdb457a39c3a8b3d7b3aa72ee8c296b663d64b6a079915`. Exact retained source inventory, cold selector, typed namespace or lowering consumer. This source/API evidence supplies no matching owned assertion outcome, measured time/allocation gain, external observation or present Native admission.
- `implementation-source-3` (implementation): [rust/tcl-compiler/src/command_binding.rs](../../../../rust/tcl-compiler/src/command_binding.rs). SHA-256 `b457884e90c6a20e065b5e07235eed843683dae103366495d20905721ce31928`. Exact retained source inventory, cold selector, typed namespace or lowering consumer. This source/API evidence supplies no matching owned assertion outcome, measured time/allocation gain, external observation or present Native admission.

- `own-source347-software-outcomes` (observation): [docs/design/analysis/name-resolution-proofs/rust-validation/compiler-core-runtime-owned347-experiments.json](../../../../docs/design/analysis/name-resolution-proofs/rust-validation/compiler-core-runtime-owned347-experiments.json). SHA-256 `903b68fc411ccd67d42d0756077e0fe6df85453fcec7cc3d7619374e33cc9f4c`. JSON pointer `/exact_question_body_outcomes`. Exact original-source347 body/question joins, closed owning producers, strict actual inventories/admissions and per-selector results. Current implementation source and unselected/new controls receive no result from these images.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/shared_inventory.rs](../../../../rust/tcl-compiler/src/command_binding/shared_inventory.rs), `SharedSourceInventory::update_if_needed`: Inspect exact collection contents before requesting copy-on-write mutation; unchanged selected facets preserve storage identity.
- [rust/tcl-compiler/src/command_binding/origin_inventory.rs](../../../../rust/tcl-compiler/src/command_binding/origin_inventory.rs), `SourceCommandBindings::selected_source_in_context`: Filter authentic original source receipts by exact namespace and retain independent variable-read/context refusals; rebuild changed facets only.
- [rust/tcl-compiler/src/command_binding/pre_handler_failure.rs](../../../../rust/tcl-compiler/src/command_binding/pre_handler_failure.rs), `SourceCommandBindings::retain_pre_handler_failure_namespace`: Preserve exact source-origin/namespace failure alternatives and empty-row removal while sharing unchanged inventories.
- [rust/tcl-compiler/src/command_binding/retained_body_inventory.rs](../../../../rust/tcl-compiler/src/command_binding/retained_body_inventory.rs), `RetainedSourceBodyInventory`: Keep one completed immutable SourceCommandBindings owner with a readonly derived projection memo. Changed source interpretation constructs an independent owner; sharing does not cache current operation validity.
- [rust/tcl-compiler/src/command_binding/retained_body_inventory.rs](../../../../rust/tcl-compiler/src/command_binding/retained_body_inventory.rs), `RetainedSourceBodyInventory::selected_source_in_context`: Memoize only the existing structural namespace selector under complete origin and typed namespace equality within this owner. Each selected child receives its own memo over its filtered inventory; hashes choose buckets only.
- [rust/tcl-compiler/src/command_binding/namespace_context.rs](../../../../rust/tcl-compiler/src/command_binding/namespace_context.rs), `SourceNamespaceKey`: Keep semantic authored/typed namespace identity, including actual Native namespace incarnation, rather than using a displayed namespace label as a cache identity.
- [rust/tcl-compiler/src/lowering/mod.rs](../../../../rust/tcl-compiler/src/lowering/mod.rs), `Lowerer::selected_source_bindings_boxed_in_context`: Use the retained immutable source owner for the actual selected body, with original body mapping and lowering checks kept outside the namespace-wide projection memo.
- [rust/tcl-compiler/src/command_binding/origin_inventory.rs](../../../../rust/tcl-compiler/src/command_binding/origin_inventory.rs), `command_binding::origin_inventory::tests::unchanged_namespace_selection_reuses_original_inventory_without_detachments` (linked): Eight genuine retained collection identities remain shared across 32 identical whole-source namespace selections; exact invocation correspondence and original view remain unchanged. Zero detachments supplies no native frame/body admission or elapsed-time result.
- [rust/tcl-compiler/src/command_binding/origin_inventory.rs](../../../../rust/tcl-compiler/src/command_binding/origin_inventory.rs), `command_binding::origin_inventory::tests::namespace_selection_detaches_changed_points_and_preserves_original_views` (linked): Mixed authentic namespaces detach changed point/preparation facets, retain only the selected namespace, then share a repeated unchanged projection. Original view stays intact; unrelated namespace returns unknown dispatch and no preparation proof.
- [rust/tcl-compiler/src/command_binding/retained_body_inventory.rs](../../../../rust/tcl-compiler/src/command_binding/retained_body_inventory.rs), `command_binding::retained_body_inventory::tests::original_body_projection_shares_exact_namespace_inventory_and_keeps_body_ranges` (linked): Two different original body ranges select one exact origin/namespace entry from a completed immutable inventory. The finite control compares full cold-selector output, actual shared Arc storage and one recorded capture for two requests, while each body keeps its independent expression-preparation range. Software counters are not elapsed-time or memory measurements. This source-defined software control has no attached matching owned assertion outcome; it establishes no external-provider observation or Native execution grant.
- [rust/tcl-compiler/src/command_binding/retained_body_inventory.rs](../../../../rust/tcl-compiler/src/command_binding/retained_body_inventory.rs), `command_binding::retained_body_inventory::tests::original_body_projection_children_cannot_restore_filtered_or_foreign_origins` (linked): Different namespace keys keep distinct selected inventories. A child selects from its already filtered owner and cannot recover the sibling points or expression preparations; a foreign original source selection remains empty under the existing cold-selector semantics. The control does not require structural selection to authenticate current execution. This source-defined software control has no attached matching owned assertion outcome; it establishes no external-provider observation or Native execution grant.
- [rust/tcl-compiler/src/command_binding/retained_body_inventory.rs](../../../../rust/tcl-compiler/src/command_binding/retained_body_inventory.rs), `command_binding::retained_body_inventory::tests::original_body_projection_uses_actual_native_namespace_incarnation` (linked): Two held genuine Rust VM entries have equal namespace display and different typed namespace identities. The exact cold-selector parity and shared same-key view are preserved, while the other actual namespace incarnation selects an empty point/preparation inventory. Holding software entry metadata grants no original C execution, current handler admission or physical reference-count parity. This source-defined software control has no attached matching owned assertion outcome; it establishes no external-provider observation or Native execution grant.

A named test is a coverage binding, not a claim that it executed.

## Replay

Named Rust coverage is linked, not an executed-result claim. Exact source/executable receipts retain actual outcomes separately. Native evidence and runtime/effect/frame/edit obligations remain independent.


## Exact original-source Compiler software outcomes

The table reports only admitted selectors attached to this question by exact comments in the retained original test bodies. Each result is limited to its named source snapshot, owned Compiler image and selector invocation. No other source/image, unselected assertion, external-provider observation or performance conclusion follows.

| Original operation | Passed | Failed | Incomplete |
| --- | ---: | ---: | ---: |
| `integration-compiler-focused-consumer-tests344` | 3 | 0 | 0 |

[Exact original-source body hashes, receipts, streams, inventories and owned images](rust-validation/compiler-owned341345-experiments.json) preserve the per-selector events and independent producer/admission limits. The existing provider answers retain their own experimental scope.


## Exact original-source347 software outcomes

These results belong only to the retained source347 body/question pairs and their owning Compiler or Core image. They do not establish an outcome for current source, unselected controls or external providers.

| Original operation | Passed | Failed |
| --- | ---: | ---: |
| `integration-compiler-focused-consumer-tests347` | 3 | 0 |

[Exact original body hashes, producer/list/admission receipts, raw logs and owned images](rust-validation/compiler-core-runtime-owned347-experiments.json) retain the finite per-selector results and independent failed Runtime producer.
