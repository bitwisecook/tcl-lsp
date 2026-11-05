# naming.server.original-reference-and-lens-source-selection

Kind: `implementation-contract`

## Problem statement

Cross-document references and reference lenses can disagree when one consumer selects original byte declarations and another reparses a displayed qualified name. Equal display text, namespace siblings and copied provider documents can then create unrelated locations or hide ambiguity.

## Question

How do workspace reference, declaration and reference-lens consumers select the same independently current original source owner without reparsing reporting names?

## Conclusion

The shared server selector prepares a sealed readonly declaration identity from the complete participating source inventory and genuine original caller lookup. Reference locations and resolved lenses use a reporting-only wrapper around the canonical source reference kernel. Original method source correspondences remain in this separate partial report; the strong hierarchy and transformation inventories retain their existing purposes. Declaration inclusion is explicit, and each returned span is lifted against its independent URI and rechecked current source after asynchronous reads. Duplicate providers, missing original selection and stale source decline the whole advice path; Native misses do not enter reporting-name resolution. Each reporting participant retains its actual URI, SourceFile, full checked input/configuration, original published SourcePath admission and actual source publication. Metadata is freshly issued under that same retained provider. A final joint snapshot holds input/factory guards, the complete open-document map, DB/file/publication and per-URI invalidation-map guards and the SourceStore provider lease through UTF-16 projection. Equal-path replacement, changed revisions or missing current owners withdraw advice. Partial positive method counts say "at least"; zero partial or unavailable coverage says "References unavailable" without a clickable empty census. Wire coverage tags are presentation only and resolve recomputes them from the retained identity. This selects source declarations and occurrences, not native dispatch.

## Scope

Rust server workspace source-selection and location transport. Fixed C8.6 source-analysis controls preserve opaque sibling keys, namespace priority, independent provider URI ownership, inclusion policy and duplicate-provider refusal. Missing operands and counterfactual report entries remain terminal. The linked tests compare the shared reference helper and cross-document declaration path; they do not execute an interpreter, assert native object state, or certify a wire handle from client labels.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This is a Rust source-consumer contract. No executed interpreter or appliance observation is attached.

## Consumer bindings

- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `original_declaration_identity_at`: Select one current document-owned readonly identity through genuine original prepare and complete participating source inventory.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `original_declaration_reference_locations`: Retain the strong canonical reference kernel for its existing consumers, keep declaration inclusion explicit, and recheck each URI/source before projecting UTF-16 spans.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `reference_locations`: Route original namespace, variable, method and procedure/class queries through their typed source owners before compatibility resolution.
- [rust/tcl-lsp-core/src/original_call_hierarchy.rs](../../../../rust/tcl-lsp-core/src/original_call_hierarchy.rs), `references`: Retain actual original lookups and canonical allocation/declaration owners for source occurrence enumeration; source advice grants no native dispatch.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `tests::original_workspace_references_and_lenses_share_owned_byte_declarations` (linked): The shared source/reference path selects pD800 under its provider URI, excludes pD801 and namespace siblings, respects include-declaration and declines duplicate authentic providers.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `tests::original_workspace_reference_misses_never_reparse_reporting_names` (linked): Counterfactual reporting entries cannot repair a missing original query; changed source cannot reissue an existing provider identity.

- [rust/tcl-lsp-server/src/readonly_reference_reports.rs](../../../../rust/tcl-lsp-server/src/readonly_reference_reports.rs), `Backend::readonly_original_declaration_reference_report`: Join source-only member positives only by genuine canonical identity and current independent original source owners; keep partial/unavailable coverage distinct from the strong inventory.
- [rust/tcl-lsp-server/src/readonly_reference_reports.rs](../../../../rust/tcl-lsp-server/src/readonly_reference_reports.rs), `Backend::capture_readonly_reference_document`: Retain the actual published SourceFile/admission and checked DB input before freshly issuing source metadata under that same provider.
- [rust/tcl-lsp-server/src/readonly_reference_reports.rs](../../../../rust/tcl-lsp-server/src/readonly_reference_reports.rs), `Backend::finish_readonly_reference_report`: Check the complete retained publication/source/input set without an intervening await and lift only inside the current provider lease.
- [rust/tcl-lsp-server/src/readonly_reference_reports.rs](../../../../rust/tcl-lsp-server/src/readonly_reference_reports.rs), `readonly_reference_reports::tests::readonly_member_wire_retains_owner_and_partial_coverage` (linked): Original source selectors reach their own real URI with lower-bound lens count; forged reporting labels and coverage tags cannot select an owner.
- [rust/tcl-lsp-server/src/readonly_reference_reports.rs](../../../../rust/tcl-lsp-server/src/readonly_reference_reports.rs), `readonly_reference_reports::tests::readonly_member_wire_withdraws_missing_or_replaced_source_owners` (linked): Independent copied providers, missing SourceFile handles and changed original source refuse the whole report.
- [rust/tcl-lsp-server/src/readonly_reference_reports.rs](../../../../rust/tcl-lsp-server/src/readonly_reference_reports.rs), `readonly_reference_reports::tests::readonly_member_wire_rejects_equal_path_provider_republication` (linked): Genuine MemoryStore equal-path ABA republication preserves bytes/input and SourceFile yet withdraws the old provider receipt; an independent URI invalidation and a same-byte document revision also withdraw the final set.
- [rust/tcl-lsp-server/src/readonly_reference_reports.rs](../../../../rust/tcl-lsp-server/src/readonly_reference_reports.rs), `readonly_reference_reports::tests::unavailable_member_wire_has_no_clickable_empty_census` (linked): Partial zero and unavailable coverage carry no reference-count absence or clickable empty location result; the existing inventory retains its old zero title.

## Evidence

- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs)
- [rust/tcl-lsp-core/src/original_call_hierarchy.rs](../../../../rust/tcl-lsp-core/src/original_call_hierarchy.rs)
