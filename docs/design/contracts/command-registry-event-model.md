# KCS: Command registry and event model contracts

## Symptom

Known commands/events are flagged as unknown (or vice versa), event ordering checks drift, or command metadata-dependent features regress.

## Operational context

`rust/tcl-registry/` is the central contract layer for command signatures, dialect roles, event validation/flow metadata, and type/taint hints consumed across analysis and LSP features. Command knowledge lives on `CommandSpec` (and its attached descriptor types); the iRules event, profile, and protocol-namespace graphs live alongside it in the same crate.

## Decision rules / contracts

1. Command metadata belongs on `CommandSpec` and the registry's lookup layer, not in scattered hardcoded sets. A consumer that needs a new fact about a command gets a new spec field (or a typed hook ID it can dispatch on), never a `match cmd_name { … }` arm.
2. Event validity/ordering rules should be centralized in the event registry and flow-chain definitions.
3. Consumers should query registry APIs rather than duplicating command/event classification logic.
4. Parser/recovery known-command lookups must go through the shared helper in the analyser (`recovery_known_commands`, returning a `RecoveryKnownCommands`), which unions `CommandRegistry::command_names()` with the document's own definitions and the caller-supplied extra-command set. Dialect-agnostic existence questions use `CommandRegistry::known_in_any_dialect`, which is built from the same per-pack spec functions the registry itself loads.
5. When BIG-IP source data introduces profile aliases (`MSSQL`, `RADIUS_AAA`, `SIPSESSION`, `DIAMETERSESSION`, etc.), keep the shared profile/event/namespace tables aligned rather than hardcoding alias fixes in a consumer.
6. `VALID DURING` in BIG-IP command manpages is the source of truth for iRules command legality; avoid synthetic profile requirements unless some event in the shared model can actually satisfy them.
7. Utility/control iRules prefixes (`ILX`, `CRYPTO`, `X509`, `PROFILE`, etc.) still need `ProtocolNamespaceSpec` entries even when they are not profile-backed; represent them with an empty `profiles` set instead of leaving the namespace table incomplete.
8. `ProfileRegistry::expand_profile_stack` must round-trip every registered `ProfileSpec`; shared TLS helpers such as `PERSIST` belong in a shared TLS bucket instead of being dropped from the effective stack.
9. When a protocol namespace is profile-backed and all of its enabling profiles share one layer or side, keep `ProtocolNamespaceSpec.layer` and `ProtocolNamespaceSpec.side` aligned with that profile metadata.

## File-path anchors

- `rust/tcl-registry/src/spec.rs` — `CommandSpec`, `SubCommand`, `OptionSpec`
- `rust/tcl-registry/src/registry.rs` — `CommandRegistry` and its query surface
- `rust/tcl-registry/src/commands/` — the per-dialect spec packs
- `rust/tcl-registry/src/arity.rs`, `arg_role.rs`, `forms.rs` — signature shape
- `rust/tcl-registry/src/events.rs` — `EventRegistry`, `EventProps`, `FlowChain`, `EventRequires`
- `rust/tcl-registry/src/event_facts/` — generated event fact tables
- `rust/tcl-registry/src/profiles.rs` — `ProfileSpec`, `ProtocolNamespaceSpec`, `StackModification`, `ProfileRegistry`
- `rust/tcl-registry/src/taint.rs` — taint colours and transforms
- `rust/tcl-registry/src/types.rs` — the type lattice hints specs carry
- `rust/tcl-compiler/src/analyser/utils.rs` — `recovery_known_commands` / `RecoveryKnownCommands`

## Failure modes

- Per-feature hardcoded command lists diverge from registry truth.
- Event flow diagnostics regress after event-chain updates without central validation.
- Registry hint changes (taint/type) unintentionally alter downstream diagnostics.

## Test anchors

- `rust/tcl-registry/tests/registry_commands.rs` — curated, C-Tcl-anchored command/dialect/subcommand facts
- `rust/tcl-registry/tests/registry_sweep.rs` — broad accessor/data sweep over every command in every dialect plus the BIG-IP object specs
- `rust/tcl-registry/tests/dialect_profile.rs` — dialect profile and event/profile graph behaviour
- `rust/tcl-compiler/src/analyser/irules_event_checks.rs` — iRules event-scoping and ordering diagnostics (IRULE1001 / IRULE1002) and their unit tests
- `rust/tcl-compiler/tests/irules_spec_examples_self_consistent.rs` — every iRules spec's own examples analysed clean
- `rust/tcl-compiler/tests/irules_event_context.rs` — event-context threading through the analyser
- `rust/tcl-compiler/tests/recovery_positions.rs` — the recovery paths that consume the known-command helper

## Discoverability

- [KCS index](../../../docs/design/README.md)
- [Command registry field reference](../../../docs/design/compiler/command-registry.md)
- [Registry contract tests](../../../docs/design/contracts/registry-contract-tests.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
- [shared utility contracts](../../../docs/design/contracts/shared-utility-contracts-rust.md)
- [compiler pass/fact ownership matrix](../../../docs/design/compiler/pass-fact-ownership-matrix.md)
