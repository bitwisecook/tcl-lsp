# KCS: LSP feature providers (non-diagnostics)

## Symptom

A language feature (hover, completion, rename, references, symbols, semantic tokens, etc.) regresses even though parsing and diagnostics still look correct.

## Operational context

LSP features are implemented as focused providers under `server/features/`, each consuming shared semantic and compiler facts. Behaviour consistency depends on shared symbol resolution and workspace state.

## Decision rules / contracts

1. Feature providers should consume shared resolution/fact helpers instead of duplicating lookup logic.
2. Provider responses must be deterministic for unchanged document + workspace state.
3. Cross-feature behaviour changes should be validated across at least one navigation + one edit feature.
4. BIG-IP config (`bigip*.conf`) feature overlays must remain context-aware:
   semantic tokens should add BIG-IP value categories without regressing Tcl tokenisation, and definition should resolve cross-object references via the shared BIG-IP object registry (`dialects/f5/bigip/object_registry.py`) rather than hardcoded per-provider maps.
5. BIG-IP object and reference mappings are maintained in the in-repo registry catalogue (`dialects/f5/bigip/registry/`) and consumed through the registry facade (`dialects/f5/bigip/object_registry.py`).
5. Proc-oriented features must use shared proc-reference matching (`analyser/proc_lookup.py`) so definition/references/rename/call hierarchy/signature help stay precedence-consistent.
6. Package suggestions and iRules event-context inference must use shared helpers (`package_suggestions.py`, `irules_context.py`) to avoid action/command drift and insertion-line regressions.

## File-path anchors

- `server/features/completion.py`
- `server/features/hover.py`
- `server/features/definition.py`
- `server/features/references.py`
- `server/features/rename.py`
- `server/features/_semantic_tokens/`
- `server/features/call_hierarchy.py`
- `server/features/document_symbols.py`
- `server/features/document_links.py`
- `server/features/folding.py`
- `server/features/selection_range.py`
- `server/features/signature_help.py`
- `server/features/code_actions.py`
- `server/features/inlay_hints.py`
- `server/features/workspace_symbols.py`
- `server/features/symbol_resolution.py`
- `server/features/package_suggestions.py`
- `server/features/irules_context.py`
- `analyser/proc_lookup.py`
- `dialects/f5/bigip/object_registry.py`
- `dialects/f5/bigip/registry/data.py`
- `dialects/f5/bigip/registry/specs/`

## Failure modes

- Completion/hover diverge because one provider bypasses shared symbol-resolution rules.
- Rename updates incomplete ranges while references still resolve correctly.
- Semantic tokens or symbol providers drift after parser/scope changes.
- BIG-IP-specific overlays stop activating (or over-highlight unrelated Tcl files).

## Test anchors

- `tests/test_completion.py`
- `tests/test_hover.py`
- `tests/test_definition.py`
- `tests/test_bigip_object_registry.py`
- `tests/test_references.py`
- `tests/test_rename.py`
- `tests/test_semantic_tokens.py`
- `tests/test_call_hierarchy.py`
- `tests/test_document_symbols.py`
- `tests/test_document_links.py`
- `tests/test_folding.py`
- `tests/test_selection_range.py`
- `tests/test_signature_help.py`
- `tests/test_code_actions.py`
- `tests/test_inlay_hints.py`
- `tests/test_workspace_symbols.py`
- `tests/test_proc_lookup_lsp_features.py`
- `tests/test_package_suggestions.py`
- `tests/test_irules_context.py`

## Discoverability

- [KCS index](../../../docs/design/README.md)
- [LSP diagnostics publication](../../../docs/design/contracts/lsp-diagnostics-publication.md)
- [workspace/indexing contracts](../../../docs/design/contracts/workspace-indexing.md)
- [shared utility contracts](../../../docs/design/contracts/shared-utility-contracts-rust.md)
