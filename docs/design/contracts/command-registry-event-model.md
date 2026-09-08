# Command registry and event model contracts

What `rust/tcl-registry` declares and who depends on it: command signatures,
dialect availability, argument roles, event validation and flow metadata, and
type/taint hints. The analyser, the optimiser, the runtimes, the LSP
providers, and the CLI/MCP surfaces all read the same specs, so a command this
layer calls unknown is unknown everywhere, and an event-ordering fact it gets
wrong drifts through every consumer at once.

Specs are organised as per-dialect packs under
`rust/tcl-registry/src/commands/`: `tcl`, `stdlib`, `tcllib`, `tk`, `itcl`,
`expect`, `irules`, `iapps`, `bpf`, `ticklecharts`, and `argparse`.

`sdc_base` and the five EDA vendor libraries are **not** among them: they ship
as bundled `.tclspec` loadables under `specs/`, loaded by `tcl-spectcl` and
layered into the per-profile registry at workspace scope
([spec-packs.md](../spec-packs.md), [eda-library-packages.md](../eda-library-packages.md)).
The contracts below apply to a loaded pack's specs exactly as they do to a
compiled-in one — the loader builds the same `CommandSpec`.

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
10. **A command that raises an event declares it.** `event_requires` says where
    a command may be *written*; `event_emits` / `event_emission_forms` say what
    running it *starts*. They are different questions and neither implies the
    other. Any new event-raising command declares an emission, and a consumer
    that wants cross-event reachability, a diagram edge or a data-flow path
    reads `irules_event_emission_edges` — the one owner — rather than matching
    a command name (issue #1708).

    Three properties the descriptor exists to keep:

    - **A form that raises nothing says so.** `TCP::notify eom` declares an
      empty emission, which stops it inheriting `request`/`response`'s edge.
      An empty list is a positive fact; an absent declaration is not.
    - **Certainty is carried, not inferred.** `Possible` is not `Definite`:
      `TCP::notify request` raises `USER_REQUEST` *unless* an mblb
      message-boundary context consumes it, and nothing at the call site
      distinguishes them. `Asynchronous` is not same-frame fall-through —
      `NAME::lookup`'s handler runs when the DNS answer arrives.
    - **Only literal forms select.** A computed subcommand matches no
      `argument_prefix` and an unresolved head declares nothing, so neither
      invents an edge.
    - **A call that cannot run raises nothing.** A form prefix can match an
      invocation the runtime rejects outright — `TCP::notify request extra`
      carries the `request` prefix but breaks the command's declared arity —
      so a definite argument-count failure drops the edge rather than putting
      an unreachable handler into a consumer's reachability set. Only a
      *definite* failure: a release-dependent, form-owned, subcommand-owned,
      option-bearing or structurally checked shape is undecidable from the
      count and keeps its edge, since a wrong verdict would silently lose a
      real one.
    - **Every calling event owns its edge.** A procedure emits on behalf of
      each event that reaches it, so `irules_event_emission_edges` walks one
      closure per top-level event; a helper called from `CLIENT_ACCEPTED` and
      `HTTP_REQUEST` yields two edges, not whichever handler the file happens
      to declare first.

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
  `event_descriptions.rs` — the iRules event model, including
  `EventEmission` / `EventEmissionForm` / `EventEmissionCertainty`.
- `rust/tcl-irules/src/executable.rs` — `irules_event_emission_edges`, the one
  owner of the command-to-event relation, and `irules_event_reachable_closure`,
  which follows those edges.
- `rust/tcl-registry/src/profiles.rs`, `profile_queries.rs`,
  `profile_defaults/` — profiles and protocol namespaces.
- `rust/tcl-registry/src/taint.rs`, `types.rs` — taint and type hints.
- `rust/tcl-registry/src/model/declaration.rs` — the per-document declared surface (`# tcl-lsp: stub`)
  ([dialect-stubs.md](dialect-stubs.md)).

## Failure modes

- A per-feature hardcoded command list diverging from registry truth.
- Event-flow diagnostics regressing after an event-chain update, because the
  change bypassed central validation.
- A taint or type hint change silently altering downstream diagnostics.
- A protocol namespace left out of the table entirely, so its commands read as
  unknown rather than unconditionally available.
- An event-raising command added without an emission descriptor: no edge is
  built, and every cross-event consumer silently under-reports rather than
  failing.
- An emission naming an event the registry does not have — caught by
  `declared_event_emissions_name_known_events`, because an unresolvable target
  builds no edge and would otherwise look identical to "raises nothing".

## Test anchors

- `rust/tcl-registry/tests/registry_commands.rs` — presence and shape of every
  declared command.
- `rust/tcl-registry/tests/registry_sweep.rs` — the registry-wide sweep.
- `rust/tcl-compiler/tests/checks.rs` — analyser behaviour driven by spec data.
- `rust/tcl-irules/tests/event_emission.rs` — the command-to-event edges and
  the reachability they widen.

## Discoverability

- [Design doc index](../README.md)
- [command registry field reference](../compiler/command-registry.md)
- [registry contract tests](registry-contract-tests.md)
- [namespace models per dialect](namespace-model.md)
- [command spec studio](command-spec-studio.md)
- [shared utility contracts](shared-utility-contracts-rust.md)
