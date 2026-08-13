# Command registry and event model contracts

What `rust/tcl-registry` declares and who depends on it: command signatures,
dialect availability, argument roles, event validation and flow metadata, and
type/taint hints. The analyser, the optimiser, the runtimes, the LSP
providers, and the CLI/MCP surfaces all read the same specs, so a command this
layer calls unknown is unknown everywhere, and an event-ordering fact it gets
wrong drifts through every consumer at once.

Specs are organised as per-dialect packs under
`rust/tcl-registry/src/commands/`: `tcl`, `stdlib`, `tcllib`, `tk`, `itcl`,
`expect`, `irules`, `iapps`, `bpf`, `sdc_base`, the five EDA vendor packs,
`ticklecharts`, and `argparse`.

## Decision rules / contracts

1. **Command metadata lives on the `CommandSpec`, never in a scattered
   hardcoded set.** Arity, subcommands, options, argument roles, traits,
   lifecycle, owning package, shimmer hints, and side effects are all spec
   fields. A consumer matching on a command name is a review defect — the fix
   is to add or extend the declaration.
2. **Event validity and ordering are centralised** in the event registry and
   its flow definitions (`events.rs`, `event_facts/`), not re-derived per
   consumer.
3. Consumers query registry APIs rather than duplicating classification
   logic. `CommandRegistry` owns lookup, dialect masking, and visibility;
   `ProfileQueries` owns profile-aware availability
   ([namespace-model.md](namespace-model.md)).
4. **`VALID DURING` in the BIG-IP command manpages is the source of truth**
   for iRules command legality. Do not invent a synthetic profile requirement
   that no event in the shared model can actually satisfy.
5. When BIG-IP source data introduces a profile alias (`MSSQL`, `RADIUS_AAA`,
   `SIPSESSION`, `DIAMETERSESSION`, …), align the shared profile / event /
   namespace tables rather than patching the alias in one consumer.
6. **Every protocol-namespace prefix gets a row**, including the ones that are
   not profile-backed (`ILX`, `CRYPTO`, `URI`, `X509`, `PROFILE`, …), with an
   empty `profiles` set. An absent row and an unconditionally-available row
   are different facts.
7. Where a protocol namespace is profile-backed and all its enabling profiles
   share one layer or side, keep `ProtocolNamespaceSpec`'s `layer` and `side`
   aligned with that profile metadata.
8. **Spec data is reload-safe.** A spec describes a command; it does not reach
   into compiler internals such as codegen or the optimiser
   ([project-layout.md](project-layout.md) rule 3).
9. **The registry is both the generator of test inputs and the oracle for the
   expected outputs** — see [registry-contract-tests.md](registry-contract-tests.md).

## File-path anchors

- `rust/tcl-registry/src/spec.rs` — `CommandSpec`, `SubCommand`, and the
  nested descriptor types.
- `rust/tcl-registry/src/registry.rs`, `command_table.rs`,
  `command_snapshot.rs` — lookup, masking, visibility, snapshots.
- `rust/tcl-registry/src/commands/` — the per-dialect spec packs.
- `rust/tcl-registry/src/dialects.rs`, `version.rs`, `version_range.rs`,
  `lifecycle.rs` — dialect and release-axis gating.
- `rust/tcl-registry/src/arg_role.rs`, `traits.rs`, `arity.rs`, `forms.rs`,
  `hover.rs` — the per-argument and per-command vocabularies.
- `rust/tcl-registry/src/events.rs`, `event_facts/`,
  `event_descriptions.rs` — the iRules event model.
- `rust/tcl-registry/src/profiles.rs`, `profile_queries.rs`,
  `profile_defaults/` — profiles and protocol namespaces.
- `rust/tcl-registry/src/taint.rs`, `types.rs` — taint and type hints.
- `rust/tcl-registry/src/stub_overlay.rs` — the per-document user-stub overlay
  ([dialect-stubs.md](dialect-stubs.md)).

## Failure modes

- A per-feature hardcoded command list diverging from registry truth.
- Event-flow diagnostics regressing after an event-chain update, because the
  change bypassed central validation.
- A taint or type hint change silently altering downstream diagnostics.
- A protocol namespace left out of the table entirely, so its commands read as
  unknown rather than unconditionally available.

## Test anchors

- `rust/tcl-registry/tests/registry_commands.rs` — presence and shape of every
  declared command.
- `rust/tcl-registry/tests/registry_sweep.rs` — the registry-wide sweep.
- `rust/tcl-compiler/tests/checks.rs` — analyser behaviour driven by spec data.

## Discoverability

- [Design doc index](../README.md)
- [command registry field reference](../compiler/command-registry.md)
- [registry contract tests](registry-contract-tests.md)
- [namespace models per dialect](namespace-model.md)
- [command spec studio](command-spec-studio.md)
- [shared utility contracts](shared-utility-contracts-rust.md)
