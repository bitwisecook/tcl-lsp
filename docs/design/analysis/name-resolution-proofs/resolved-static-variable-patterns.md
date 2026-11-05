# naming.analysis.resolved-static-variable-patterns

Kind: `implementation-contract`

## Problem statement

Generic static-variable advice can recognise only literal static:: prefixes, miss rooted/alias writes and mistake namespace display or F5 metadata for an actual worker-domain cell. Configurable name patterns and source locations must follow resolved storage ownership rather than select it.

## Question

How do generic-static source warnings use the same resolved namespace-place storage classifier as side effects before matching simple-key metadata, while retaining actual source/event/CFG/operand ownership and declining absent Native or physical TMM authority?

## Conclusion

The contextual namespace classifier has one worker shared by side effects and namespace_storage_scope_for_place. Native cell classification uses its selected storage domain; explicit authored source keeps separate descriptive policy. The namespace/root-only simple-key queries share the retained cell owner and real variable-link traversal. Generic-static warnings require the original Module/source/event/config and executable CFG point, consume def_places_at at the actual operand-selection phase, then classify storage and match checked simple-key/literal-element metadata. Source labels report the same assignment or join multi-output calls through the actual SSA cell map; absolute document spans are retained. Warnings deduplicate resolved namespace roots, including rooted and alias spellings. Missing original cells, unknown lifetimes, opaque unchecked UTF-8 or absent multi-output correspondence decline advice. Four genuine fixed software control definitions cover domain selection, retained key versus display/frame, rooted/alias/reachable pattern consumers and foreign Module source refusal. No assertion or external provider execution outcome is attached.

## Scope

Finite three-leaf Compiler implementation and four source-defined unexecuted software controls. Name patterns remain lexical metadata of already selected namespace keys and literal elements. StorageScope::Static supplies no physical worker, Native getter/header/cache, future command/variable lookup, value equality or cross-worker publication. Existing independent BIG-IP 21.1.0.1 four-worker measurements and selected authored RULE_INIT policy retain their own evidence; C Tcl/Jim answers are not donated. General dynamic/multi-output or unavailable original producers remain conservatively unavailable.

## Provider answers

### tcl8.4

Status: `not-tested`. Channel: Source-only authored event/CFG inputs and exact resolver/place fields; no actual executing worker or Native value header. Version: tcl8.4 No external provider execution is attached to this source/place classification question.

External tcl8.4 execution is not tested for this implementation question. These software controls grant no Native getter/cache, event execution, physical TMM identity, cross-worker equality or publication. The independent BIG-IP 21.1.0.1 four-worker observations remain separately scoped and unchanged.

### tcl8.5

Status: `not-tested`. Channel: Source-only authored event/CFG inputs and exact resolver/place fields; no actual executing worker or Native value header. Version: tcl8.5 No external provider execution is attached to this source/place classification question.

External tcl8.5 execution is not tested for this implementation question. These software controls grant no Native getter/cache, event execution, physical TMM identity, cross-worker equality or publication. The independent BIG-IP 21.1.0.1 four-worker observations remain separately scoped and unchanged.

### tcl8.6

Status: `not-tested`. Channel: Source-only authored event/CFG inputs and exact resolver/place fields; no actual executing worker or Native value header. Version: tcl8.6 No external provider execution is attached to this source/place classification question.

External tcl8.6 execution is not tested for this implementation question. These software controls grant no Native getter/cache, event execution, physical TMM identity, cross-worker equality or publication. The independent BIG-IP 21.1.0.1 four-worker observations remain separately scoped and unchanged.

### tcl9.0

Status: `not-tested`. Channel: Source-only authored event/CFG inputs and exact resolver/place fields; no actual executing worker or Native value header. Version: tcl9.0 No external provider execution is attached to this source/place classification question.

External tcl9.0 execution is not tested for this implementation question. These software controls grant no Native getter/cache, event execution, physical TMM identity, cross-worker equality or publication. The independent BIG-IP 21.1.0.1 four-worker observations remain separately scoped and unchanged.

### tcl9.1

Status: `not-tested`. Channel: Source-only authored event/CFG inputs and exact resolver/place fields; no actual executing worker or Native value header. Version: tcl9.1 No external provider execution is attached to this source/place classification question.

External tcl9.1 execution is not tested for this implementation question. These software controls grant no Native getter/cache, event execution, physical TMM identity, cross-worker equality or publication. The independent BIG-IP 21.1.0.1 four-worker observations remain separately scoped and unchanged.

### jim

Status: `not-tested`. Channel: Source-only authored event/CFG inputs and exact resolver/place fields; no actual executing worker or Native value header. Version: jim No external provider execution is attached to this source/place classification question.

External jim execution is not tested for this implementation question. These software controls grant no Native getter/cache, event execution, physical TMM identity, cross-worker equality or publication. The independent BIG-IP 21.1.0.1 four-worker observations remain separately scoped and unchanged.

### bigip

Status: `not-tested`. Channel: Source-only authored event/CFG inputs and exact resolver/place fields; no actual executing worker or Native value header. Version: bigip No external provider execution is attached to this source/place classification question.

External bigip execution is not tested for this implementation question. These software controls grant no Native getter/cache, event execution, physical TMM identity, cross-worker equality or publication. The independent BIG-IP 21.1.0.1 four-worker observations remain separately scoped and unchanged.

## Consumer bindings

- [rust/tcl-compiler/src/side_effects.rs](../../../../rust/tcl-compiler/src/side_effects.rs), `namespace_storage_scope_for_place`: Same canonical namespace-place classifier as side effects; Native storage domain precedes reporting/profile metadata, authored source advice remains separate.
- [rust/tcl-compiler/src/var_resolve.rs](../../../../rust/tcl-compiler/src/var_resolve.rs), `retained_namespace_variable_root`: Borrow the actual namespace root/key through existing owner or retained variable link; no display reparse.
- [rust/tcl-compiler/src/var_resolve.rs](../../../../rust/tcl-compiler/src/var_resolve.rs), `namespace_variable_simple_name`: Checked UTF-8 metadata of the already resolved namespace root; no namespace lookup, variable/cache or TMM grant.
- [rust/tcl-compiler/src/irules_checks.rs](../../../../rust/tcl-compiler/src/irules_checks.rs), `find_generic_static_name_warnings`: Actual original source/event/CFG guard then retained point def_places_at, cell/domain classification, source-label correspondence and configurable lexical pattern metadata.
- [rust/tcl-compiler/src/place_bridge.rs](../../../../rust/tcl-compiler/src/place_bridge.rs), `def_places_at`: Existing actual retained CFG tokens, conditional logical store and original variable operand-selection phase; missing original producer remains unavailable.
- [rust/tcl-registry/src/f5/storage.rs](../../../../rust/tcl-registry/src/f5/storage.rs), `namespace_storage_domain_in_path`: Existing separately selected host context and retained namespace component geometry; neither spelling nor a profile supplies a physical worker/publication.
- [rust/tcl-compiler/src/side_effects.rs](../../../../rust/tcl-compiler/src/side_effects.rs), `side_effects::tests::namespace_place_storage_scope_requires_the_selected_domain_before_static_reporting` (linked): Fixed genuine source/resolver/place-owner software inputs only. Source-defined control; no software outcome, Native object/header/getter observation or physical BIG-IP/TMM execution is attached.
- [rust/tcl-compiler/src/var_resolve.rs](../../../../rust/tcl-compiler/src/var_resolve.rs), `var_resolve::namespace_simple_projection_tests::namespace_simple_key_projection_keeps_resolved_bytes_separate_from_display_and_frames` (linked): Fixed genuine source/resolver/place-owner software inputs only. Source-defined control; no software outcome, Native object/header/getter observation or physical BIG-IP/TMM execution is attached.
- [rust/tcl-compiler/src/irules_checks.rs](../../../../rust/tcl-compiler/src/irules_checks.rs), `irules_checks::tests::generic_static_warning_uses_resolved_root_keys_aliases_and_original_source_locations` (linked): Fixed genuine source/resolver/place-owner software inputs only. Source-defined control; no software outcome, Native object/header/getter observation or physical BIG-IP/TMM execution is attached.
- [rust/tcl-compiler/src/irules_checks.rs](../../../../rust/tcl-compiler/src/irules_checks.rs), `irules_checks::tests::generic_static_warning_declines_foreign_module_source_without_spelling_fallback` (linked): Fixed genuine source/resolver/place-owner software inputs only. Source-defined control; no software outcome, Native object/header/getter observation or physical BIG-IP/TMM execution is attached.

## Evidence

- [rust/tcl-compiler/src](../../../../rust/tcl-compiler/src)
- [rust/tcl-compiler/src/var_resolve/cell_key.rs](../../../../rust/tcl-compiler/src/var_resolve/cell_key.rs)
- [rust/tcl-registry/src/f5/storage.rs](../../../../rust/tcl-registry/src/f5/storage.rs)
