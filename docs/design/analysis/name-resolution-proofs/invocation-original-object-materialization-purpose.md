# naming.invocation.original-object-materialization-purpose

Kind: `implementation-contract`

## Problem statement

A descriptive source profile may differ from the independently retained execution engine or an explicitly installed authored materialization provider. Letting list/dictionary conversion select a recipe from that profile can discard original compound members or fill a refused authored purpose from an unrelated physical engine. List length must retain the same purpose instead of selecting another profile.

## Question

How do list and dictionary constructors, checked getters and length retain distinct actual-engine, authored-logical and standalone compatibility purposes without borrowing source-profile authority, losing original members or bypassing a selected refusal?

## Conclusion

ObjectMaterialization keeps three purpose tags with one retained recipe: Native for an independently selected activation or actual engine, AuthoredLogical for an explicitly installed currently applicable F5 simulation, and StandaloneCompatibility for Distribution object operations. A selected actual activation supplies its own purpose first. Without it, a selected authored purpose is terminal when unavailable; an unrelated retained physical engine cannot fill it. If no authored purpose is selected, the independently retained actual engine remains valid even under an unknown descriptive source profile. Distribution compatibility is explicit and supplies no actual engine or entry. List/dictionary checked getters and checked dictionary constructors use that shared selection. Portable unchecked constructors may retain unsealed list/dictionary data when no recipe is selected; they confer no Native protocol. NativeObjectLengthProtocol::from_materialization purely retains the same recipe and logical tag. Cached List length checks the original live header and whole backing without rendering members; class and canonical-empty identity remain independent obligations. Foreign sealed compound backing stays refused.

An additional marked lower software control requires the authentic compound recipe before returning cached dictionary members. Foreign recipes refuse for both List and Dict carriers with or without resident text, preserving primary storage, member references and residency; the authentic recipe preserves original member identity.

Selected name inventory is separately governed by [original-variable-inventory-policy](invocation-original-variable-inventory-policy.md). General embedding transport keeps its own [host-publication-and-fact contract](embedding-original-host-publication-and-fact-transport.md). These source/API bindings do not change either question’s original provider status or independently pinned software outcome.

## Scope

Four marked VM software controls cover actual C86 engine with unknown Jim source, explicitly installed F5 authored materialization and terminal withdrawal despite a retained C90 physical engine, independently selected actual activation, separately tagged Distribution C/Jim compatibility and unknown-source refusal, and foreign C90 backing under actual C86. Constructors/getters/length retain the original member objects; failures occur before rendering untouched members. These are API/source definitions, not original external-provider comparisons or completed Rust assertion outcomes. Original engine/source/frame/header/object/cache authority, scalar/numeral/expression/character policies, Normal completion and artifact admission remain independent. The failed Source290 inventory-positive assertion remains separately archived; this definition does not replace it with a pass or imply whole Info186 comparison success.

These are constructed software carrier/cache assertions, not new original C-produced object observations or conversion/worker admission. The earlier Source295 foreign-dictionary failure remains independently archived. No assertion result, Native frame/handler or provider behavior is added by this source binding.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: No original external process for this software materialization-purpose contract. Dialect: tcl8.4.

The software tests select retained engine or authored/standalone purpose tags and original software members. No original C, Jim or BIG-IP process answers this API ownership contract; Native object/cache/frame behaviour remains independently measured where its own proof permits.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: No original external process for this software materialization-purpose contract. Dialect: tcl8.5.

The software tests select retained engine or authored/standalone purpose tags and original software members. No original C, Jim or BIG-IP process answers this API ownership contract; Native object/cache/frame behaviour remains independently measured where its own proof permits.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: No original external process for this software materialization-purpose contract. Dialect: tcl8.6.

The software tests select retained engine or authored/standalone purpose tags and original software members. No original C, Jim or BIG-IP process answers this API ownership contract; Native object/cache/frame behaviour remains independently measured where its own proof permits.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: No original external process for this software materialization-purpose contract. Dialect: tcl9.0.

The software tests select retained engine or authored/standalone purpose tags and original software members. No original C, Jim or BIG-IP process answers this API ownership contract; Native object/cache/frame behaviour remains independently measured where its own proof permits.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: No original external process for this software materialization-purpose contract. Dialect: tcl9.1.

The software tests select retained engine or authored/standalone purpose tags and original software members. No original C, Jim or BIG-IP process answers this API ownership contract; Native object/cache/frame behaviour remains independently measured where its own proof permits.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: No original external process for this software materialization-purpose contract. Dialect: jim.

The software tests select retained engine or authored/standalone purpose tags and original software members. No original C, Jim or BIG-IP process answers this API ownership contract; Native object/cache/frame behaviour remains independently measured where its own proof permits.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: No original external process for this software materialization-purpose contract. Dialect: bigip.

The software tests select retained engine or authored/standalone purpose tags and original software members. No original C, Jim or BIG-IP process answers this API ownership contract; Native object/cache/frame behaviour remains independently measured where its own proof permits.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-vm/src/interp/object_materialization.rs](../../../../rust/tcl-vm/src/interp/object_materialization.rs), `InterpState::object_materialization`: Require the actual selected activation, current explicitly installed authored provider, independently retained engine or tagged Distribution compatibility in that order; selected authored refusal is terminal.
- [rust/tcl-vm/src/interp/object_materialization.rs](../../../../rust/tcl-vm/src/interp/object_materialization.rs), `ObjectMaterialization::protocol`: Retain the selected recipe and distinct Native/AuthoredLogical/StandaloneCompatibility issuer purpose for compound constructors and checked getters.
- [rust/tcl-vm/src/interp/object_materialization.rs](../../../../rust/tcl-vm/src/interp/object_materialization.rs), `ObjectMaterialization::length_protocol`: Project length policy from the same retained materialization recipe without selecting another source profile or discarding its authored tag.
- [rust/tcl-registry/src/native_stock_list.rs](../../../../rust/tcl-registry/src/native_stock_list.rs), `NativeObjectLengthProtocol::from_materialization`: Purely compose the existing stock length policy and explicit logical origin from the selected recipe; this grants no engine, header, entry or Normal.
- [rust/tcl-vm/src/value_ops.rs](../../../../rust/tcl-vm/src/value_ops.rs), `<Vm as ValueOps>::list_len`: Require shared materialization purpose, checked original live header, independent stock class/canonical-empty identity and original whole cached backing before length.
- [rust/tcl-vm/src/value_ops.rs](../../../../rust/tcl-vm/src/value_ops.rs), `<Vm as ValueOps>::list_elements`: Use the retained purpose protocol for checked original list members and preserve foreign-header/backing refusal.
- [rust/tcl-vm/src/value_ops.rs](../../../../rust/tcl-vm/src/value_ops.rs), `<Vm as ValueOps>::dict_pairs`: Use the same retained purpose for checked dictionary members without reconstructing an engine from a source label.
- [rust/tcl-vm/src/value_ops.rs](../../../../rust/tcl-vm/src/value_ops.rs), `<Vm as ValueOps>::new_dict_with_hash_bucket_count_checked`: Require selected compound materialization before checked dictionary construction; original key/member objects remain the inputs.
- [rust/tcl-vm/src/value_ops.rs](../../../../rust/tcl-vm/src/value_ops.rs), `<Vm as ValueOps>::new_list`: Construct with the shared selected recipe, or retain portable unsealed data when unavailable; portable construction grants no Native materialization authority.
- [rust/tcl-vm/src/interp/object_materialization.rs](../../../../rust/tcl-vm/src/interp/object_materialization.rs), `interp::object_materialization::tests::actual_object_materialization_survives_unknown_source_without_losing_original_members` (linked): Actual software C86 materialization remains selected under unknown Jim source; list/dictionary getters and length retain original member identities, a foreign protocol cannot reseal the dictionary, and untouched member bytes remain unrendered.
- [rust/tcl-vm/src/interp/object_materialization.rs](../../../../rust/tcl-vm/src/interp/object_materialization.rs), `interp::object_materialization::tests::selected_authored_materialization_refuses_without_physical_fallback` (linked): An explicitly installed F5 authored C84 materialization/length purpose remains tagged; changing its source to an unsupported Jim point withdraws it before getters/checked construction despite a retained C90 engine. A separately selected actual activation remains independent and leaving it restores authored refusal.
- [rust/tcl-vm/src/interp/object_materialization.rs](../../../../rust/tcl-vm/src/interp/object_materialization.rs), `interp::object_materialization::tests::standalone_compatibility_is_separate_and_unknown_source_cannot_select_a_recipe` (linked): Distribution C/Jim object compatibility is separately tagged with no actual engine; current Jim opaque list data is retained, unknown Jim source refuses length/getters/checked construction, and unchecked portable List data grants no selected protocol.
- [rust/tcl-vm/src/interp/object_materialization.rs](../../../../rust/tcl-vm/src/interp/object_materialization.rs), `interp::object_materialization::tests::actual_object_materialization_preserves_foreign_compound_refusal` (linked): Actual C86 getters and length reject sealed foreign C90 list/dictionary backing without rendering original members or replacing the genuine foreign protocol.

A named test is a coverage binding, not a claim that it executed.

- [rust/tcl-vm/src/value.rs](../../../../rust/tcl-vm/src/value.rs), `Value::native_object_dict_pairs`: Check original header access and the independently sealed compound string recipe before any cached dictionary/list members; cached storage or resident text cannot repair a foreign recipe.
- [rust/tcl-vm/src/value.rs](../../../../rust/tcl-vm/src/value.rs), `value::tests::native_dictionary_getter_checks_foreign_compound_recipe_before_cached_members` (linked): Cached List and Dict software carriers, with and without resident text, refuse a foreign sealed compound string recipe before returning members; original primary/residency/member references stay unchanged and the authentic recipe retains original key/member identity.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Replay

Current source/API definitions only. No Rust assertion or original provider operation is attached; existing original compound object/list-length/cache proofs and separately pinned failed or passed software commands retain their own source, image and purpose. The Info186 opt-in diagnostic is instrumentation, not a new observation or success.
