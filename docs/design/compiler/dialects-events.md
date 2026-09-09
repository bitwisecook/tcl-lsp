# Dialects and events

How command availability is partitioned across Tcl releases and tool
surfaces, and how iRules event requirements narrow it further. Read this
when adding a dialect-specific command, or when a command is reported as
unknown (IRULE1001 / W002) in one context but not another.

Source: `rust/tcl-dialect/src/model/authored_surface.rs` (`SpecSurface`),
`rust/tcl-dialect/src/profile.rs` (`DialectProfile`),
`rust/tcl-registry/src/spec.rs` (`CommandSpec::surface`),
`rust/tcl-registry/src/events.rs` (`EventRequires`, `EventProps`)

### Surfaces

Availability is `surface: Option<&'static [SpecSurface]>` on `CommandSpec`,
`SubCommand`, `FormSpec`, `OptionSpec`, and `SideEffect`. Each `SpecSurface`
names a provider — `SpecProvider::Core(Family)` for a language family
(`Family::Tcl`, `F5Tcl`, `F5Irules`, `Jim`) or `SpecProvider::Package(name)`
— with optional version windows. Shorthands cover the usual rows:

| Constant | Means |
|---|---|
| `SpecSurface::ALL_TCL` | core Tcl, every release |
| `TCL84` … `TCL91`, `TCL8X` | one release, or the 8.x series |
| `TCL85_PLUS`, `TCL86_PLUS`, `TCL90_PLUS` | a version floor |
| `IRULES`, `JIM` | another core family |
| `TK`, `EXPECT`, `IAPPS`, `TMSH`, `BPF`, `BIGIP`, `SPECTCL`, `SSLICTCL` | a package |
| `ALL_TCL_AND_IRULES`, `TK_AND_TCL` | unions |

- `surface: None` → available everywhere.
- `surface: Some(SpecSurface::IRULES)` → iRules only (`HTTP::host`, `pool`,
  `table`).
- A `SubCommand`'s own `surface` is consulted first; `None` there inherits
  the parent's.

The EDA shells have no surface of their own: each is a base Tcl release plus
`required_package`-gated command libraries, shipped as bundled
`specs/*.tclspec` packs and installed by profile name.

### Profiles and base versions

A document resolves to one `DialectProfile`: `tcl8.4`–`tcl9.1`, `tk`,
`expect`, `f5-irules`, `f5-iapps`, `f5-tmsh`, `f5-bigip`, `bpf`, `spectcl`,
`sslictcl`, or an EDA shell (`synopsys-`, `cadence-`, `xilinx-`,
`intel-quartus-`, `mentor-`, `microchip-libero-eda-tcl`). Besides its surface
layers and packages, a profile carries two deliberately separate base
versions ([dialect-profile-model.md](../registry/dialect-profile-model.md) §2.1):

| Field | Decides |
|---|---|
| `signature_base` | which command / subcommand / option *signatures* the profile exposes — the release half of every surface query |
| `runtime_base` (always equal to `expr_grammar_base`) | version-gated *behaviour*: `incr` on an uninitialised variable, TIP 201 `in`/`ni`, TIP 461 `lt`/`le`/`gt`/`ge`, the W003 diagnostic |

| Profile | Base |
|---|---|
| `f5-irules`, `f5-iapps`, `f5-tmsh`, `cadence-eda-tcl` | 8.4 |
| `xilinx-eda-tcl`, `intel-quartus-eda-tcl`, `microchip-libero-eda-tcl` | 8.5 |
| `expect`, `synopsys-eda-tcl`, `mentor-eda-tcl` | 8.6 |
| `bpf`, `spectcl`, `sslictcl` | 9.0 |
| `f5-bigip` (a configuration parser, not Tcl), `tk` until projected onto a Tcl point | `None` |

iRules embeds a genuine Tcl 8.4.6 with nothing backported at any BIG-IP
version, so `dict`/`lassign` (8.5) and `lmap`/`throw` (8.6) are never
present and `incr` on an unset variable follows 8.4 semantics. The F5 command
surface (`HTTP::*`, `pool`, …) is a versioned library keyed by BIG-IP
version, orthogonal to the pinned Tcl base. A `None` base means "cannot
reason about this profile's grammar version", never "assume plain Tcl".

Version-gated *behaviour* on a spec is a `SpecSurface` row, never a dialect
name:

```rust
// incr: safe on an uninitialised variable in Tcl 8.5+, an error in 8.4
safe_on_uninit: Some(SpecSurface::TCL85_PLUS),
```

**Contract**: a new dialect is a `DialectProfile` in `profile.rs` (name,
aliases, layers, packages, base versions, grammar); a new language family is
also a `Family` variant.

### Availability outcomes

The analyser reports a head word that does not resolve in the active
profile through three distinct codes:

| Code | Meaning |
|------|---------|
| `W002` | The command exists, but is disabled in the active dialect profile. The message carries an "available in: …" suffix built by `dialect_availability_suffix` from the spec's own `surface` |
| `W123` | Unresolved command — not found in the registry, user procs, or an `unknown` handler |
| `W001` | The command resolved, but its first word is not a recognised subcommand. Suppressed when the spec sets `allow_unknown_subcommands` |

The emitters live in
`rust/tcl-compiler/src/analyser/diagnostics/validity.rs`.

### Event requirements (iRules)

`EventRequires` declares when a command is valid:

| Field | Purpose | Example |
|-------|---------|---------|
| `client_side` | Needs a client-side connection | `true` for request-side commands |
| `server_side` | Needs a server-side connection | `true` for response-side commands |
| `transport` | TCP or UDP | `Some("tcp")` for HTTP commands |
| `profiles` | Required profile list | `&["FASTHTTP", "HTTP"]` |
| `also_in` | Extra valid events | Events not matching other criteria |
| `flow` | Needs an active traffic flow | Flow-dependent commands |

`event_requirement_forms` (`EventRequirementForm`) overrides the contract for
one literal argument prefix — `FIX::tag map set` configures a mapping and is
valid outside the event `FIX::tag get` needs.

**Example** — `HTTP::host`:
```rust
event_requires: Some(EventRequires {
    client_side: false,
    server_side: false,
    transport: Some("tcp"),
    profiles: &["FASTHTTP", "HTTP"],
    also_in: &[],
    flow: false,
}),
```

### Event validation

The validator matches `EventRequires` against `EventProps` (what each event
provides: client/server side, transport, implied profiles, flow).
Mismatches produce diagnostic `IRULE1001`.

`CommandSpec::excluded_events` lists events where a command is explicitly
forbidden.

### Data collection and side context

The command registry also records a `DataCollectionOperation` for commands
that participate in an iRules payload lifecycle. A consumer does not infer
meaning from a name ending in `::collect`, `::release`, or `::payload`.
That distinction matters: TCP, HTTP, and SSL payloads need collection; UDP
and ASM payloads are supplied by BIG-IP without an explicit collect command.
HTTP collection is released implicitly when its matching data event completes,
whereas TCP and SSL require an explicit release.

`EventProps::data_collect_protocols` lists the protocol alternatives for a
data event. `CLIENT_DATA` and `SERVER_DATA` include UDP as an implicit
alternative, so a standalone handler cannot be proved never to fire merely
because it has no `TCP::collect`. The analyser stays silent until the source
gives stronger evidence, such as a TCP payload access.

Nesting-script side changes are likewise declared through
`CommandSpec::side_switch_target` (`Client`, `Server`, or `Peer`). The flow
checker uses the descriptor while it recurses, so adding a side-switch command
does not require a compiler command-name branch.

Finally, `EventHandlerPriority` records whether an event handler has a runtime
default. BIG-IP's `when` default is priority 500; omitting `priority` is valid
and only an explicitly stricter dialect policy can request IRULE1004.

### How dialects feed the compiler

| Stage | Effect |
|-------|--------|
| **Semantic analysis** | W002 for disabled commands, W123 for unresolved ones |
| **Variable analysis** | `safe_on_uninit` resolved at the profile's point against the spec's `SpecSurface` rows |
| **Completions** | Only show commands valid in the active profile |
| **Taint** | Taint sources/sinks are dialect-specific (iRules HTTP commands) |
| **Side effects** | iRules-specific `SideEffectTarget`s (`SessionTable`, `PersistenceTable`, `ConnectionControl`, …) |
| **Connection scope** | Only applies to iRules multi-event scripts |

## Decision rule

- When adding a new iRules command, set `surface: Some(SpecSurface::IRULES)`
  and configure `event_requires` with the appropriate transport and profiles.
- If a command works in both iRules and standard Tcl, use
  `SpecSurface::ALL_TCL_AND_IRULES` (or list the rows).
- For version-specific commands, use a floor or release row —
  `Some(SpecSurface::TCL90_PLUS)`, `Some(SpecSurface::TCL90)`.
- For version-dependent **behaviour** (not availability), put a
  `SpecSurface` row on the relevant `CommandSpec` field rather than naming a
  dialect: `safe_on_uninit: Some(SpecSurface::TCL85_PLUS)` for `incr`.
- When adding a new dialect, add its `DialectProfile`.
- If IRULE1001 fires incorrectly, check that the event's `EventProps`
  includes the required profiles and transport.

## Related docs

- [Command infrastructure — Availability](example-walkthroughs.md#availability)
- [Command infrastructure — Events](example-walkthroughs.md#events-irules-only)
- [command-registry.md](command-registry.md)
- [connection-scope.md](connection-scope.md)
