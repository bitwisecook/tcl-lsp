# Selected catalogue completion provider

## Question

How do command-head and definition-member completion render the descriptor selected by the actual document availability without allowing another same-name provider to decide a filter or its documentation?

## Conclusion

The existing DocumentFloor context and ResolvedContext::resolve_spec select each catalogue descriptor once. Builtin completion carries that descriptor through operator/member-only traits, the existing OoFrame requirement predicate, Tk advice, owning-package lifecycle floor and detail/documentation rendering. The old name-query OoFrame adapter and the selected-descriptor adapter share one private predicate; no frame classification or callability rule changes. Grammar membership still comes from the actual DefinitionBodyGrammar source_member_in and full authoring query. Its optional command metadata comes from resolve_spec in the same context; a genuine grammar-only row stays a vocabulary candidate with no invented descriptor documentation.

## Scope

Exact existing catalogue unit: lsp-completion-catalogue-selected-provider-filter, bounded builtin_completions, grammar_member_completions and registry_method_items caller facets. Actual unchanged full input/configuration and context ingress remain required. Metadata spelling and usage ranking are source advice and display. Registry inherited instance-method providers retain their actual owning CommandSpec and independent package floor. Linked editing, runtime command occupancy, successful dispatch, physical method frame, Normal, and edit permission remain independent. No Runtime/VM/BIG-IP source is changed or executed.

## Provider answers

### tcl8.4

Status: `not-tested`. Source-advice implementation only; no provider execution or ordinary Rust test result is claimed.

### tcl8.5

Status: `not-tested`. Source-advice implementation only; no provider execution or ordinary Rust test result is claimed.

### tcl8.6

Status: `not-tested`. Source-advice implementation only; no provider execution or ordinary Rust test result is claimed.

### tcl9.0

Status: `not-tested`. Source-advice implementation only; no provider execution or ordinary Rust test result is claimed.

### tcl9.1

Status: `not-tested`. Source-advice implementation only; no provider execution or ordinary Rust test result is claimed.

### jim

Status: `not-tested`. Source-advice implementation only; no provider execution or ordinary Rust test result is claimed.

### bigip

Status: `not-tested`. Source-advice implementation only; no provider execution or ordinary Rust test result is claimed.

## Source inspection

The prior builtin path resolved an available descriptor but discarded it, then asked raw registry.get for traits, Tk/package and lifecycle filters. Its detail branch resolved again. The prior grammar member path also used raw registry.get for documentation. CommandRegistry::get returns the last registered same-name descriptor; ResolvedContext::resolve_spec applies the retained authoring point, source realm, most-specific admitted provider, package and roster filtering. A last-registered Tcl9-only descriptor can differ from the selected Tcl8.6 descriptor. No native oracle is required to demonstrate this source catalogue selection mismatch.

## Consumer bindings

- [completion.rs](../../../../rust/tcl-lsp-core/src/completion.rs), `builtin_completions`: retains actual descriptor identity across every existing admission and item projection.
- [completion.rs](../../../../rust/tcl-lsp-core/src/completion.rs), `grammar_member_completions`: retains separate grammar vocabulary admission and optional selected documentation.
- [oo_dispatch.rs](../../../../rust/tcl-lsp-core/src/oo_dispatch.rs), `OoFrame::admits_selected_spec`: applies the same existing requirement predicate without another name lookup. Legacy hover keeps its original name-query adapter.
- [completion.rs](../../../../rust/tcl-lsp-core/src/completion.rs), `completion::selected_catalogue_provider_tests::builtin_filters_keep_the_actual_selected_provider_through_every_gate` (linked): genuine current source/context retains the admitted descriptor despite a later unavailable same-name row with competing operator/member/frame/package/version facts; original negative trait/frame gates and missing/full-config guards stay active.
- [completion.rs](../../../../rust/tcl-lsp-core/src/completion.rs), `completion::selected_catalogue_provider_tests::grammar_member_details_use_the_same_current_availability_context` (linked): actual grammar vocabulary preserves its own availability and grammar-only candidate while documentation comes from the admitted descriptor.

A named test is a coverage binding, not an execution claim. All pre-existing test source/assertions remain unchanged.

## Inherited provider ownership

registry_method_items passes the exact context-selected factory descriptor to CommandRegistry::instance_method_providers_for_descriptor_at. The existing Registry breadth-first owner walk retains each actual provider descriptor and obtains its document floor from that same descriptor; the receiver package cannot donate a version floor to a different superclass package. Actual context resolution and exact descriptor correspondence gate each provider, while row lifecycle/surface filtering, cycle prevention and declaration-order first-name precedence remain shared. A foreign factory descriptor refuses. An unavailable owner contributes neither rows nor a superclass traversal.

The legacy instance_methods_for_descriptor_at wrapper retains its original uniform-floor contract; other compiler/diagnostic/native consumers receive no semantic change. This caller projects conditional lexical catalogue advice only. Its existing allows_lexical_declaration_advice guard, actual context selection and configured source method-name rendering are unchanged; no Native receiver completion route opens.

- `registry::instance_method_provider_tests::inherited_method_provider_keeps_exact_descriptor_and_own_package_floor` (linked): genuine per-owner floors, exact provider pointer, foreign-descriptor and unavailable-provider refusal, cycle termination and unchanged legacy uniform-floor scope.
- `completion::inherited_registry_completion_provider_tests::inherited_completion_uses_selected_provider_package_instead_of_receiver_package` (linked): opposite receiver/provider package floors and a later foreign same-name descriptor cannot suppress or invent an inherited item.

A named selector is unexecuted coverage, not a provider or Rust result. All original test fixtures and assertions remain byte-identical.

## Replay

Root owns ordinary build, exact binary singleton listing, execution and count reconciliation. The original two and added three selectors are validation annexes to the one existing caller purpose; neither is a new semantic owner or per-helper count. Preserve all original completion operator, method-context, package, grammar, source/currentness and provider controls.

The additional `completion::inherited_registry_roster_completion_tests::inherited_completion_cannot_borrow_a_surface_only_provider_excluded_by_actual_roster` selector retains a genuine Jim 0.79 Logical editing input. A Tcl surface edge alone admits the superclass descriptor, while the actual inherited command roster excludes it. Full context resolution, including roster and invocation realm, must select that exact descriptor before any method or package floor is projected. This new control remains unexecuted.


## Current index binding

Stable proof ID: `naming.consumer.selected-catalogue-completion-provider`.

## Problem statement

A same-name provider selected by ambient Registry lookup can replace the descriptor selected by actual completion availability.

Existing Question, Conclusion and Scope above are the exact indexed fields. All seven provider rows are not-tested; named controls are linked, not executed by this update.

This is a source/API implementation contract. Index linkage supplies no provider observation, runtime execution or passed test result.
