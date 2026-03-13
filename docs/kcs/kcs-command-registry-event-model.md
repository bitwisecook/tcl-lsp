# KCS: Command registry and event model contracts

## Symptom

Known commands/events are flagged as unknown (or vice versa), event ordering checks drift, or command metadata-dependent features regress.

## Operational context

`core/commands/registry/` is the central contract layer for command signatures, dialect roles, event validation/flow metadata, and type/taint hints consumed across analysis and LSP features.

## Decision rules / contracts

1. Command metadata belongs in registry models/runtime, not in scattered hardcoded sets.
2. Event validity/ordering rules should be centralized in event registry/flow definitions.
3. Consumers should query registry APIs rather than duplicating command/event classification logic.
4. Parser/recovery known-command lookups must use the shared cache helper (`core/parsing/known_commands.py`) backed by registry command names.

## File-path anchors

- `core/commands/registry/models.py`
- `core/commands/registry/command_registry.py`
- `core/commands/registry/runtime.py`
- `core/commands/registry/signatures.py`
- `core/commands/registry/namespace_registry.py`
- `core/commands/registry/namespace_data.py`
- `core/commands/registry/namespace_models.py`
- `core/commands/registry/taint_hints.py`
- `core/commands/registry/type_hints.py`
- `core/parsing/known_commands.py`

## Failure modes

- Per-feature hardcoded command lists diverge from registry truth.
- Event flow diagnostics regress after event-chain updates without central validation.
- Registry hint changes (taint/type) unintentionally alter downstream diagnostics.

## Test anchors

- `tests/test_command_registry.py`
- `tests/test_registry_validation.py`
- `tests/test_event_registry.py` (tests NamespaceRegistry via EVENT_REGISTRY alias)
- `tests/test_event_flow_chains.py` (tests ordering from namespace_data)
- `tests/test_event_tree.py` (tests data types and validation from namespace_data/namespace_models)
- `tests/test_irules_checks.py`
- `tests/test_parsing_helpers.py`

## Discoverability

- [KCS index](README.md)
- [LSP feature providers](kcs-lsp-feature-providers.md)
- [shared utility contracts](kcs-core-lsp-shared-utility-contracts.md)
- [compiler pass/fact ownership matrix](compiler/kcs-pass-fact-ownership-matrix.md)
