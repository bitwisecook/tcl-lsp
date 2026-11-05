# naming.analysis.selected-scope-alias-role-metadata

Kind: `implementation-contract`

## Problem statement

Scope-alias consumers can repeat command/subcommand policy or strip arbitrary root punctuation before querying the Registry. A type-widening caller can also use a command reporting label while losing the original selected invocation and source owner.

## Question

How do selected scope-alias traits and complete effective argument-role positions share the Registry owner, and how does actual type widening retain the original invocation/input/purpose without deriving a variable link or Native authority from a descriptor name?

## Conclusion

ResolvedInvocation exposes selected scope-alias local VarWrite positions and optional adjacent VarRead positions only after complete authored role/cardinality and selector checks. The adjacent position is role metadata, not an alias target. Authored public helpers use the Registry descriptor/root fallback and apply static source filters after selection; they carry no original Native operands. The sole production alias recogniser in type propagation joins the original tokens, complete source input and binding-owned Execution, Logical, Standalone or AuthoredSimulation purpose before reading selected traits. Logical uses its separate source owner; the other purposes require their existing checked selected invocation. Missing, changed or foreign ownership returns unavailable and widens relevant Call definitions. Explicit tokenless/unbound Standalone compatibility stays separately labelled. Fixed controls cover selected global/variable/upvar/namespace/my effective ordinals, lexical source-read filters, Registry descriptor root fallback, a real Logical aliased call and missing/foreign/changed original carriers, plus a held real Jim software entry whose raw Logical eligibility cannot select Logical policy. These are unexecuted software source definitions; no external Native/provider result is attached.

## Scope

Finite Registry selected-role query, public authored var_scoping helpers and the actual type-infer Call widening consumer. Argument positions and traits do not prove a completed alias, target cell, selected frame, physical contents, successful completion, optimisation equivalence or Native execution. The three public declaration-position helpers have no production callers in the current source; this contract does not claim a Native bypass in them or whole type soundness.

## Provider answers

### tcl8.4

Status: `not-tested`. Channel: Source-only selected metadata, actual retained entry/binding/vector and fixed authored or held-VM software controls. Version: tcl8.4 No external provider execution is attached to this source contract.

External tcl8.4 results are not tested here. The source-defined held-Jim software controls and explicit Logical/AuthoredSimulation drivers attach no external provider result, assertion outcome, Native frame/cell/value, dispatch, trace absence or executable rewrite proof. Independently recorded Native naming and BIG-IP/TMM evidence remains separately scoped.

### tcl8.5

Status: `not-tested`. Channel: Source-only selected metadata, actual retained entry/binding/vector and fixed authored or held-VM software controls. Version: tcl8.5 No external provider execution is attached to this source contract.

External tcl8.5 results are not tested here. The source-defined held-Jim software controls and explicit Logical/AuthoredSimulation drivers attach no external provider result, assertion outcome, Native frame/cell/value, dispatch, trace absence or executable rewrite proof. Independently recorded Native naming and BIG-IP/TMM evidence remains separately scoped.

### tcl8.6

Status: `not-tested`. Channel: Source-only selected metadata, actual retained entry/binding/vector and fixed authored or held-VM software controls. Version: tcl8.6 No external provider execution is attached to this source contract.

External tcl8.6 results are not tested here. The source-defined held-Jim software controls and explicit Logical/AuthoredSimulation drivers attach no external provider result, assertion outcome, Native frame/cell/value, dispatch, trace absence or executable rewrite proof. Independently recorded Native naming and BIG-IP/TMM evidence remains separately scoped.

### tcl9.0

Status: `not-tested`. Channel: Source-only selected metadata, actual retained entry/binding/vector and fixed authored or held-VM software controls. Version: tcl9.0 No external provider execution is attached to this source contract.

External tcl9.0 results are not tested here. The source-defined held-Jim software controls and explicit Logical/AuthoredSimulation drivers attach no external provider result, assertion outcome, Native frame/cell/value, dispatch, trace absence or executable rewrite proof. Independently recorded Native naming and BIG-IP/TMM evidence remains separately scoped.

### tcl9.1

Status: `not-tested`. Channel: Source-only selected metadata, actual retained entry/binding/vector and fixed authored or held-VM software controls. Version: tcl9.1 No external provider execution is attached to this source contract.

External tcl9.1 results are not tested here. The source-defined held-Jim software controls and explicit Logical/AuthoredSimulation drivers attach no external provider result, assertion outcome, Native frame/cell/value, dispatch, trace absence or executable rewrite proof. Independently recorded Native naming and BIG-IP/TMM evidence remains separately scoped.

### jim

Status: `not-tested`. Channel: Source-only selected metadata, actual retained entry/binding/vector and fixed authored or held-VM software controls. Version: jim No external provider execution is attached to this source contract.

External jim results are not tested here. The source-defined held-Jim software controls and explicit Logical/AuthoredSimulation drivers attach no external provider result, assertion outcome, Native frame/cell/value, dispatch, trace absence or executable rewrite proof. Independently recorded Native naming and BIG-IP/TMM evidence remains separately scoped.

### bigip

Status: `not-tested`. Channel: Source-only selected metadata, actual retained entry/binding/vector and fixed authored or held-VM software controls. Version: bigip No external provider execution is attached to this source contract.

External bigip results are not tested here. The source-defined held-Jim software controls and explicit Logical/AuthoredSimulation drivers attach no external provider result, assertion outcome, Native frame/cell/value, dispatch, trace absence or executable rewrite proof. Independently recorded Native naming and BIG-IP/TMM evidence remains separately scoped.

## Consumer bindings

- [rust/tcl-registry/src/resolved_invocation.rs](../../../../rust/tcl-registry/src/resolved_invocation.rs), `authored_source_scope_alias_arguments`: Selected complete role/cardinality projection; positions only, no target/link.
- [rust/tcl-compiler/src/var_scoping.rs](../../../../rust/tcl-compiler/src/var_scoping.rs), `scope_alias_declaration_indices`: Authored Registry selection followed by lexical static-name filters; no Native lookup.
- [rust/tcl-compiler/src/type_infer.rs](../../../../rust/tcl-compiler/src/type_infer.rs), `original_scope_alias_call`: Actual original vector/input/binding purpose and selected traits; unavailable widens.
- [rust/tcl-compiler/src/command_binding/origin_inventory.rs](../../../../rust/tcl-compiler/src/command_binding/origin_inventory.rs), `original_invocation_source_purpose`: Shared request-purpose issuer after complete original binding/input/source checks; no execution grant.
- [rust/tcl-compiler/src/type_infer.rs](../../../../rust/tcl-compiler/src/type_infer.rs), `type_infer::scope_alias_metadata_tests::original_alias_type_widening_requires_the_actual_invocation_and_input` (linked): The original selectedGlobal aliased call under a genuine Logical source driver reports the selected scope-alias trait. Missing metadata, foreign availability, withdrawn binding or a truncated original structured word vector makes the query unavailable. The separate production Call consumer widens unavailable definitions; this body asserts the query boundary, not a complete type/Native alias result.
- [rust/tcl-compiler/src/type_infer.rs](../../../../rust/tcl-compiler/src/type_infer.rs), `type_infer::scope_alias_metadata_tests::native_jim_alias_metadata_cannot_use_raw_logical_name_eligibility` (linked): A held real Jim software entry supplies no Native name policy, while its raw assistance metadata permits Logical names. The actual global call binding still issues Execution purpose and its unavailable checked alias-metadata query cannot take the Logical path. No physical alias link, current frame/cell/value or external Jim execution is asserted.
- [rust/tcl-compiler/src/var_scoping.rs](../../../../rust/tcl-compiler/src/var_scoping.rs), `var_scoping::tests::selected_alias_metadata_keeps_registry_root_fallback_and_source_read_filters` (linked): Authored Registry descriptor lookup accepts ::global and declines :global/::::global without a Native QName claim. Selected upvar source-read filters reject the dynamic source, variable value words do not remove unrelated local declarations, and my variable versus my touch follows the selected alias trait. No Native command lookup or completed link is asserted.
- [rust/tcl-registry/src/resolved_invocation.rs](../../../../rust/tcl-registry/src/resolved_invocation.rs), `resolved_invocation::scope_alias_metadata_tests::selected_scope_alias_positions_keep_complete_roles_and_effective_ordinals` (linked): Actual selected authored global/variable/upvar/namespace upvar/my variable/puts descriptors yield the fixed complete effective local VarWrite ordinals and optional preceding VarRead ordinals, including compound offsets. These are selected role positions only; the body supplies no alias-target identity, native link or frame.

## Evidence

- [rust/tcl-compiler](../../../../rust/tcl-compiler)
- [rust/tcl-compiler/src/command_binding](../../../../rust/tcl-compiler/src/command_binding)
- [rust/tcl-compiler/src](../../../../rust/tcl-compiler/src)
- [rust/tcl-compiler/src/optimiser/elimination](../../../../rust/tcl-compiler/src/optimiser/elimination)
- [rust/tcl-registry](../../../../rust/tcl-registry)
- [rust/tcl-syntax/src](../../../../rust/tcl-syntax/src)
