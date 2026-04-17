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
5. When BIG-IP source data introduces profile aliases (`MSSQL`, `RADIUS_AAA`, `SIPSESSION`, `DIAMETERSESSION`, etc.), keep the shared profile/event/namespace tables aligned rather than hardcoding alias fixes in a consumer.
6. `VALID DURING` in BIG-IP command manpages is the source of truth for iRules command legality; avoid synthetic profile requirements unless some event in the shared model can actually satisfy them.
7. Utility/control iRules prefixes (`ILX`, `CRYPTO`, `URI`, `X509`, `PROFILE`, etc.) still need `ProtocolNamespaceSpec` entries even when they are not profile-backed; represent them with an empty `profiles` set instead of leaving the namespace table incomplete.
8. `LayerStack` must round-trip every registered `PROFILE_SPEC`; shared TLS helpers such as `PERSIST` belong in a shared TLS bucket instead of being dropped from the effective stack.
9. When a protocol namespace is profile-backed and all of its enabling profiles share one layer or side, keep `ProtocolNamespaceSpec.layer` and `ProtocolNamespaceSpec.side` aligned with that profile metadata.

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

- [KCS index](../../../docs/design/README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
- [shared utility contracts](../../../docs/design/contracts/core-lsp-shared-utility.md)
- [compiler pass/fact ownership matrix](../../../docs/design/compiler/pass-fact-ownership-matrix.md)
