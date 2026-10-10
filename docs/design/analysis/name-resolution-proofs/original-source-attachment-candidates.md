# naming.consumer.original-source-attachment-candidates

Kind: `implementation-contract`

## Problem statement

Inferring an attachment kind from a displayed command or a broad SNAT effect can mistake an address for a SNAT-pool name, select the wrong argument after options or replace unknown original values with invented positional text.

## Question

Does source attachment advice require the actual authored invocation and the single object-reference table for both operand position and configuration object kind?

## Conclusion

original_source_object_operands consumes the actual authored ResolvedInvocation from the guarded source schema. It uses the existing declarative object-reference table for typed kinds and positions, preserving exact ordinary unknown operands while refusing unknown expansion cardinality and missing required selectors. Complete literal-only custom table resolvers receive actual original values; unknown values do not become placeholders. Possible Pool, Node and SnatPool attachments additionally require the corresponding authored selection write and exact object-kind template. A general SnatSelection write alone cannot produce a SNAT-pool operand; the snat pool selector and operand remain independently selected. This metadata supplies readonly configuration-reference candidates, without evaluated target contents, object identity, traffic effect, current availability, execution or editing permission. Object-reference consumers retain only supported literal source values and genuine original operand spans from this same descriptor; unsealed DTO labels cannot supply a schema and set statements do not prove later evaluated values. Diagram attachment reports preserve exact literal source candidates and make dynamic values unconstrained until independent composition, observer and evaluated-target premises exist. JSON states conditional source applicability and missing runtime reachability; it supplies no object deletion-safety proof.

## Scope

Current source-only object/attachment descriptor over the existing shared object-reference table. Fixed assertions distinguish pool/node/SNAT-pool purposes, dynamic ordinary slots from expansion, bare SNAT address from snat pool target and known falsey reference words. Seven native providers are not tested; no execution receipt or native observations are attached.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-irules/src/source_object.rs](../../../../rust/tcl-irules/src/source_object.rs), `original_source_object_operands`: Select source object kinds/ordinals from the existing table and actual authored invocation, preserving unknown ordinary slots without runtime or object identity.
- [rust/tcl-irules/src/source_object.rs](../../../../rust/tcl-irules/src/source_object.rs), `IrulesSourceAttachmentKind`: Keep Pool, Node and SnatPool advice distinct from broad possible selection effects.
- [rust/tcl-irules/src/walker.rs](../../../../rust/tcl-irules/src/walker.rs), `extract_original_irules_source_object_references`: Select genuine supported literal source references from the same guarded context/schema/table, without runtime set-value propagation.
- [rust/tcl-irules/src/walker.rs](../../../../rust/tcl-irules/src/walker.rs), `extract_irules_object_references_in_closure`: Treat compatibility DTO spans only as a filter over freshly validated actual source candidates; displayed labels do not select metadata.
- [rust/tcl-diagram/src/attach.rs](../../../../rust/tcl-diagram/src/attach.rs), `attach_reach_for_source_context`: Project exact literal source candidates and unconstrained dynamic target patterns through independently selected object-kind/ordinal purposes.
- [rust/tcl-irules/src/source_object.rs](../../../../rust/tcl-irules/src/source_object.rs), `source_object::tests::original_attachment_operands_keep_snat_pool_purpose_and_original_cardinality` (linked): Actual source schema and table retain dynamic ordinary target ordinals, reject unknown expansion, and distinguish SNAT-pool selectors from bare address/SNAT effects.
- [rust/tcl-irules/src/walker.rs](../../../../rust/tcl-irules/src/walker.rs), `walker::tests::original_object_reference_consumers_ignore_dto_labels_and_runtime_value_guesses` (linked): Changing DTO labels does not change actual literal source references; a source set/value guess supplies no evaluated target and a stale retained context refuses.
- [rust/tcl-diagram/src/attach.rs](../../../../rust/tcl-diagram/src/attach.rs), `attach::tests::original_attachment_reports_keep_dynamic_values_unconstrained` (linked): Substitution and set-value source patterns remain unconstrained while direct literal source values remain exact candidates.
- [rust/tcl-diagram/src/attach.rs](../../../../rust/tcl-diagram/src/attach.rs), `attach::tests::original_attachment_reports_require_actual_kind_ordinal_and_current_source` (linked): Actual Pool/Node/SnatPool templates and original ordinals remain distinct; bare SNAT addresses, source handler barriers and stale source cannot donate attachment metadata. Definite custom pool/event declarations block source advice; unavailable TMM rename retains unknown-transition obligations and an unconstrained MAY variable attachment rather than a selected move or object identity.

A named test is a coverage binding, not a claim that it executed.

## Replay

The named Rust selectors bind fixed source-consumer assertions. No Rust execution receipt or native experiment is attached. Whole-image/configuration/Registry currency and independently required purpose permissions remain mandatory.
