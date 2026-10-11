# naming.editor.logical-written-component-geometry

Kind: `implementation-contract`

## Problem statement

Logical namespace edits and source selection need exact written component positions. Collapsing a separator run loses byte geometry, and recovering namespace versus empty terminal entity meaning from a label loses the actual occurrence purpose. A failed component mapping cannot establish complete edit coverage.

## Question

Which exact original byte extents and selected occurrence purposes does the shared written-component owner retain for Logical namespace selection and edits, and when must component correspondence refuse?

## Conclusion

The shared Rust owner defines nonempty original byte extents, consuming each complete run of at least two colons and retaining lone colons, Unicode and NUL bytes. Logical consumers separately retain Namespace, Entity or configured Variable purpose, so trailing separators and actual braced headers keep their own meaning. Exact written suffix correspondence yields an original extent or an independently established inherited omission; failed correspondence remains terminal and never counts as occurrence coverage. Complete current source, metadata, grammar and edit guards remain required. The four finite software controls are defined but have no attached assertion result; all seven external provider executions are not tested for this question.

## Scope

Current Rust source/API definitions in tcl-syntax and tcl-lsp-core, actual parent module registrations, and four unexecuted software control definitions. Raw byte segmentation is separate from configured variable header selection and positively selected Logical source edit geometry. The direct mapper refuses escaped/computed geometry without an authentic container. Whole original input/configuration/metadata and coverage/hazard checks remain independently required. Authored C/Jim/F5 profiles in model controls are not original provider processes. No Native lookup, namespace lifetime, current command/cell identity, header/cache, body entry, source-to-runtime byte equivalence, applied external edit or measured performance outcome is inferred.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: No original external process for this Rust written-component contract. Channel: Current Rust source/API and finite model definitions. Dialect: tcl8.4.

The shared written byte geometry and selected Logical occurrence contract are inspected Rust definitions only. No original tcl8.4 execution or provider result is attached.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: No original external process for this Rust written-component contract. Channel: Current Rust source/API and finite model definitions. Dialect: tcl8.5.

The shared written byte geometry and selected Logical occurrence contract are inspected Rust definitions only. No original tcl8.5 execution or provider result is attached.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: No original external process for this Rust written-component contract. Channel: Current Rust source/API and finite model definitions. Dialect: tcl8.6.

The shared written byte geometry and selected Logical occurrence contract are inspected Rust definitions only. No original tcl8.6 execution or provider result is attached.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: No original external process for this Rust written-component contract. Channel: Current Rust source/API and finite model definitions. Dialect: tcl9.0.

The shared written byte geometry and selected Logical occurrence contract are inspected Rust definitions only. No original tcl9.0 execution or provider result is attached.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: No original external process for this Rust written-component contract. Channel: Current Rust source/API and finite model definitions. Dialect: tcl9.1.

The shared written byte geometry and selected Logical occurrence contract are inspected Rust definitions only. No original tcl9.1 execution or provider result is attached.

### jim

Status: `not-tested`. Version: not tested. Build: No original external process for this Rust written-component contract. Channel: Current Rust source/API and finite model definitions. Dialect: jim.

The shared written byte geometry and selected Logical occurrence contract are inspected Rust definitions only. No original jim execution or provider result is attached.

### bigip

Status: `not-tested`. Version: not tested. Build: No original external process for this Rust written-component contract. Channel: Current Rust source/API and finite model definitions. Dialect: bigip.

The shared written byte geometry and selected Logical occurrence contract are inspected Rust definitions only. No original bigip execution or provider result is attached.

## Exact evidence

- `written-geometry-0` (implementation): [rust/tcl-lsp-core/src/lib.rs](../../../../rust/tcl-lsp-core/src/lib.rs). SHA-256 `af601143313ec01d115f4c2f229269056e421cf18d702e33ce124e3266c95db7`. Current exact shared written-component/Logical consumer source or authentic module registration; source contract only, with no compiled assertion or external interpreter credit.
- `written-geometry-1` (implementation): [rust/tcl-lsp-core/src/namespace_rename.rs](../../../../rust/tcl-lsp-core/src/namespace_rename.rs). SHA-256 `f910036e09e51d73d3bba87025fa12762658998e9bc4874c7969374e5c949841`. Current exact shared written-component/Logical consumer source or authentic module registration; source contract only, with no compiled assertion or external interpreter credit.
- `written-geometry-2` (implementation): [rust/tcl-syntax/src/lib.rs](../../../../rust/tcl-syntax/src/lib.rs). SHA-256 `eba20677b3652b80fb4273e1e89aec569d5740cc4d3760fa8d42426988dcab45`. Current exact shared written-component/Logical consumer source or authentic module registration; source contract only, with no compiled assertion or external interpreter credit.
- `written-geometry-3` (implementation): [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs). SHA-256 `f2f4085e01013fd986beeabba9695c35e101aaa9ac0d486aef727d9a48c4bdcf`. Current exact shared written-component/Logical consumer source or authentic module registration; source contract only, with no compiled assertion or external interpreter credit.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs), `qualifier_component_extents`: Define exact nonempty original byte extents using complete runs of at least two colons; a lone colon remains a component byte. No trailing empty entity purpose, constructed-key inversion or source container is issued.
- [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs), `qualifier_segments`: Collect the same shared extents directly as original byte slices; retain the separately selected trailing-separator purpose at consumers.
- [rust/tcl-lsp-core/src/namespace_rename.rs](../../../../rust/tcl-lsp-core/src/namespace_rename.rs), `WrittenNamePurpose`: Retain independently issued Logical Namespace, Entity or Variable grammar purpose; a display label cannot supply this role.
- [rust/tcl-lsp-core/src/namespace_rename.rs](../../../../rust/tcl-lsp-core/src/namespace_rename.rs), `written_cell_occurrence`: Check the selected resolved suffix against the complete original written geometry and selected variable header grammar. Return exact Written, independently established Inherited, or terminal failed correspondence.
- [rust/tcl-lsp-core/src/namespace_rename.rs](../../../../rust/tcl-lsp-core/src/namespace_rename.rs), `lexical_namespace_rename_edits`: Require positively selected Logical advice and complete current source/metadata/configuration. Failed component mapping never counts as covered occurrence; existing coverage/hazard and source edit guards remain independent.
- [rust/tcl-lsp-core/src/namespace_rename.rs](../../../../rust/tcl-lsp-core/src/namespace_rename.rs), `original_namespace_rename_span_at_offset`: Retain the independent Native original operand, source container, current component path and checked original literal extent route. Logical geometry grants no authority to this separate route.
- [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs), `naming::tests::written_component_extents_share_segmentation_and_preserve_original_byte_geometry` (linked): The finite byte-vector control compares the shared component extents with qualifier_segments, including complete separator runs, lone colons, Unicode and NUL bytes. These are software definitions, not external interpreter or editable-container observations.
- [rust/tcl-lsp-core/src/namespace_rename.rs](../../../../rust/tcl-lsp-core/src/namespace_rename.rs), `namespace_rename::tests::logical_written_namespace_geometry_retains_complete_separator_runs_and_purpose` (linked): Direct Logical mapping keeps namespace/entity/variable purposes distinct, complete separator widths and original Unicode positions. It separates inherited omission from failed correspondence and refuses changed written names and escaped direct geometry.
- [rust/tcl-lsp-core/src/namespace_rename.rs](../../../../rust/tcl-lsp-core/src/namespace_rename.rs), `namespace_rename::tests::logical_braced_variable_geometry_uses_the_actual_header_span_and_close_rule` (linked): The genuine analyser-produced braced Var span ends before its closer. The selected original braced-variable grammar supplies contained geometry, retaining parentheses and nested versus first-close meaning; malformed or mismatched headers refuse.
- [rust/tcl-lsp-core/src/namespace_rename.rs](../../../../rust/tcl-lsp-core/src/namespace_rename.rs), `namespace_rename::tests::logical_namespace_rename_maps_source_and_refuses_failed_occurrence_coverage` (linked): The retained Logical analyser/source model defines exact selected replacement extents and keeps failed mappings outside occurrence coverage. Changed source, configuration, occurrence projection and missing original input refuse. Authored C/Jim/F5 model profiles are not external provider executions.

A named test is a coverage binding, not a claim that it executed.

## Replay

Executed software results require a matching independently successful source producer, actual executable inventory and separately closed selected assertion receipt. The shared written extent query does not establish Native operand/container/edit admission or external provider behavior. No such execution is attached here.

## Current source usage

Use `qualifier_component_extents` to retain original component positions.
`qualifier_segments` consumes that same iterator. Complete separator widths
stay in the source image; the iterator emits nonempty component extents only.

Keep the actual Namespace, Entity or configured Variable occurrence purpose.
A trailing separator names an empty terminal entity under the entity purpose,
while namespace purpose retains the namespace component. A variable occurrence
uses its selected original braced-header grammar and independently recorded
decoration; the actual Var span may stop immediately before its closer.

Keep `Written`, `Inherited` and failed correspondence distinct. A failed map
does not enter the covered-occurrence set. Logical rename and selection still
require complete current source/metadata/configuration and their existing
coverage and hazard checks. Native original input, authentic containers,
current component paths and checked literal extents use their own route.

The four linked controls define byte segmentation, purpose-aware direct
mapping, actual braced-header geometry and Logical source/edit refusals.
Their authored analyser profiles supply model ingress only. No matching
executable/assertion or external provider outcome is attached.
