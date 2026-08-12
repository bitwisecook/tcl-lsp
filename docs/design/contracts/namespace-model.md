# Namespace models per dialect

"Namespace" means four different things across the dialects this stack
supports. They share a word and almost nothing else, so each has its own model
and its own owner.

## 1. Standard Tcl namespaces

`namespace eval ::foo { proc bar {} {…} }` creates `::foo::bar`. Command names
resolve through the current namespace, `namespace path`, imports, and
`namespace unknown`; packages install into namespaces; OO classes live in them.

This is **not** described here. There is one canonical algorithm, one Rust
home, and a conformance-vector gate:

- [command-resolution.md](command-resolution.md) — the `Tcl_FindCommand` rule,
  its single implementation (`tcl_syntax::naming::resolve_command_with`), every
  consumer, and the anti-drift gates.
- [../name-resolution.md](../name-resolution.md) — the model across all four
  name kinds, and its deliberate abstentions.
- [../name-resolution-c-conformance.md](../name-resolution-c-conformance.md) —
  the C ground truth, pinned per release.
- [runtime-variable-frame-model.md](runtime-variable-frame-model.md) — the
  *variable* side, which deliberately does not follow the command rule.

## 2. F5 iRules protocol namespaces

`HTTP::`, `SSL::`, `TCP::` and friends are **not** Tcl `namespace eval`
namespaces. They are command prefixes whose availability depends on which
profiles are attached to the virtual server and which events have fired. The
model is static data in `rust/tcl-registry/src/profiles.rs`, read through
`ProfileRegistry`.

### `ProfileSpec` — a profile type

One row per profile type (`HTTP`, `CLIENTSSL`, `DNS`, …) carrying:

| Field | Meaning |
|---|---|
| `layer` | protocol stack layer (`transport`, `tls_shared`, …) |
| `side` | `client`, `server`, `both`, or `global` |
| `requires` | parent profiles this one needs |
| `conflicts` | profiles it is mutually exclusive with |
| `capabilities` | what it makes available (`sni`, `cipher`, `cert`, …) |
| `lifecycle` | introduction / deprecation / retirement on the BIG-IP release axis, defaulting to the axis baseline |

Derived predicates read the data rather than restating it:
`is_infrastructure_profile` is *"this profile's layer is `transport` or
`tls_shared`"*, not a hardcoded list, so the `# Profiles:` header code action
filters the right set as soon as a new transport profile is declared.
`profile_available_at(name, version)` answers the release-axis question from
`lifecycle`.

### `ProtocolNamespaceSpec` — a command prefix

One row per prefix, carrying the profiles that provide it, its layer, its
default connection side, and whether `clientside` / `serverside` qualifiers
apply (`side_selectable`).

**Every prefix gets a row, including the ones that are not profile-backed.**
Utility and control prefixes (`ILX`, `CRYPTO`, `URI`, `X509`, `PROFILE`, …)
are represented with an empty `profiles` set rather than being left out of the
table — an absent row and an unconditionally-available row are different
facts, and only the second is true of them.

Where a prefix *is* profile-backed and all of its enabling profiles share one
layer or side, the spec's `layer` and `side` must agree with that profile
metadata.

### `StackModification` — runtime stack changes

`SSL::disable`, `HTTP::disable` and friends change the active profile stack
mid-connection. Each is a row naming the command, the connection side it
affects, and the profile it removes or adds — so the effective stack after a
given event is derived, not guessed.

### Events

Which profile *types* make an event available is
`events::EventProps::implied_profiles` — hand-curated, and the single source of
truth. `event_facts` records only structural facts (which protocol
*categories* an event belongs to) and deliberately does **not** duplicate the
enabling-profile relation; the inverse view (`events_for_profile`) reads
`implied_profiles` directly so the schema-derived table cannot drift from it.

`VALID DURING` in the BIG-IP command manpages is the source of truth for
command legality per event. Do not invent a synthetic profile requirement that
no event in the model can satisfy.

### Availability queries

Every availability consumer resolves commands through
`ProfileQueries` (`profile_queries.rs`), the extension trait that gives a
resolved `DialectProfile` its registry-backed availability API. `DialectProfile`
lives in the lower `tcl-dialect` crate and cannot know about `CommandSpec`, so
this is where the two meet. Going through the trait — rather than a bare
availability-mask query — is what keeps every consumer applying the same rules.

## 3. F5 iRules proc namespaces

A `proc` defined in an iRule named `my_irule` becomes
`::my_irule::my_helper`, callable from any other iRule:

```tcl
call my_helper                              ;# same iRule
call other_rule::my_helper $arg             ;# another iRule, same partition
call /other_partition/other_rule::my_helper ;# another partition
```

The iRule *name* is the namespace. `call` is an ordinary registry command
(`rust/tcl-registry/src/commands/irules/call.rs`) carrying
`Traits::INVOKES_USER_PROC`, an `ArgRole::Name` on its first argument, and a
`SideEffectTarget::ProcDefinition` read — so navigation, references, and the
call graph reach the target through the same machinery as any other user-proc
call. Cross-file visibility comes from the workspace index's proc aggregation
([workspace-indexing.md](workspace-indexing.md)), not from a bespoke iRules
path.

Where the iRule name is not known (no partition path in the call, nothing to
anchor against), procs are treated as local. That is a graceful degradation,
not a wrong answer.

## 4. EDA tool namespaces

EDA dialects have their own namespace conventions. Support is deliberately
minimal and conservative; the extension point for tool-specific commands is
[dialect-stubs.md](dialect-stubs.md), and per-tool command availability is
`required_package`-gated library data rather than namespace machinery.

## Property naming convention

All boolean and set properties in these tables are expressed in **positive
form**: `EventProps`'s `flow`, `hot`, `common`, `client_side`, `server_side`,
and `implied_profiles`; `ProtocolNamespaceSpec`'s `profiles` and
`side_selectable`; `ProfileSpec`'s `requires`, `conflicts`, and
`capabilities`. Never `no_flow`, `excluded_dialects`, or `never_inline_body`.
A positive property means "this thing is true or present", so a consumer
reads `if props.flow` rather than `if !props.no_flow`, and a negative set
never has to be stored.

`conflicts` is the one name that reads negative and is not: it is a
*positive* set of the profiles this profile conflicts with, not a negation of
`requires`.

## Key files

| File | Role |
|---|---|
| `rust/tcl-registry/src/profiles.rs` | `ProfileSpec`, `ProtocolNamespaceSpec`, `StackModification`, `ProfileRegistry` |
| `rust/tcl-registry/src/profile_queries.rs` | `ProfileQueries` — the one availability API |
| `rust/tcl-registry/src/profile_defaults/` | per-profile default settings |
| `rust/tcl-registry/src/events.rs` | `EventProps`, `implied_profiles`, flow chains |
| `rust/tcl-registry/src/event_facts/` | structural event → category facts |
| `rust/tcl-registry/src/commands/irules/call.rs` | the `call` spec |
| `rust/tcl-irules/src/walker.rs` | the iRules body walker |

## Discoverability

- [Design doc index](../README.md)
- [command registry and event model](command-registry-event-model.md)
- [dialect detection](dialect-detection.md), [dialect stubs](dialect-stubs.md)
