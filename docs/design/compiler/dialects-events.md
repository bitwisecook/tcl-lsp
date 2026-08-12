# KCS: Dialects and events

## Symptom

A contributor needs to understand how commands are partitioned across Tcl
versions and tool contexts (dialects), how iRules event requirements work,
or is debugging why a command is reported as unknown or disabled
(IRULE1001 / W002 / W123) in a specific context.

## Context

Dialects partition command availability.  Every `CommandSpec` has an
`Option<DialectSet>` `dialects` field; subcommands can override with their
own set.  In iRules,
commands are further restricted by event context — `EventRequires` declares
transport, profile, and connection-side requirements.

Source: `rust/tcl-dialect/src/dialect_set.rs`,
`rust/tcl-registry/src/dialects.rs`,
`rust/tcl-registry/src/spec.rs`,
`rust/tcl-registry/src/events.rs`

## Content

### Known dialects

`KNOWN_DIALECTS` (`rust/tcl-dialect/src/dialect_set.rs`, re-exported from
`rust/tcl-registry/src/dialects.rs`) is the pre-sorted list of canonical
dialect profile names — the single source of truth for the explorer's
dialect dropdown and the CLI's `--dialect` choices:

```rust
pub const KNOWN_DIALECTS: &[&str] = &[
    "bpf",
    "cadence-eda-tcl",
    "expect",
    "f5-bigip",
    "f5-iapps",
    "f5-irules",
    "f5-tmsh",
    "intel-quartus-eda-tcl",
    "mentor-eda-tcl",
    "synopsys-eda-tcl",
    "tcl8.4",
    "tcl8.5",
    "tcl8.6",
    "tcl9.0",
    "tcl9.1",
    "xilinx-eda-tcl",
];
```

Membership itself is carried by the `DialectSet` bitflags in the same
module — `TCL84`, `TCL85`, `TCL86`, `TCL90`, `TCL91`, `IRULES`, `IAPPS`,
`TK`, `EXPECT`, `BPF`, `TMSH`, `BIGIP`, plus the derived masks `ALL_TCL`,
`TCL8X`, `TCL85_PLUS`, `TCL86_PLUS`, `TCL90_PLUS`, and `TK_AND_TCL`.  The
EDA vendor shells deliberately have **no** bit of their own: they are
modelled as a base Tcl version plus `required_package`-gated command
libraries (see `eda-library-packages.md`), and bits 8–12 were freed when
that migration landed.

### Dialect base versions

Each non-standard dialect is based on a specific Tcl runtime version.
`DialectSet::expr_grammar_base_version(name)` maps a dialect name to that
base, returning `None` for names with no documented embedded Tcl core.

Per-command dialect gating does **not** go through this map — it uses the
`DialectSet` masks (`TCL85_PLUS` / `TCL86_PLUS` / `TCL90_PLUS`) directly on
`CommandSpec::dialects`, and deliberately does not fold the vendor dialects
into those masks (most standard-library commands they don't ship are missing
because of their own restricted surface, not version).  The base-version map
is for version-gated *runtime behaviour*: the `expr` grammar features TIP 201
(`in`/`ni`) and TIP 461 (`lt`/`le`/`gt`/`ge`) behind the W003 diagnostic, and
`DialectSet::namespace_var_global_fallback` (TIP 278), where every dialect
really does inherit a real embedded Tcl core wholesale.

| Dialect | Base version | Runtime |
|---------|-------------|---------|
| `f5-irules` | `TCL84` | TMOS embedded Tcl 8.4.6 |
| `f5-iapps` | `TCL85` | CentOS 7 system Tcl 8.5.13 |
| `f5-tmsh` | `TCL85` | CentOS 7 system Tcl 8.5.13 |
| `f5-bigip` | *(none)* | Custom parser, not Tcl |
| `bpf` | *(none)* | eBPF framework dialect |
| `synopsys-eda-tcl` | `TCL86` | Synopsys DC/PT/ICC2 |
| `cadence-eda-tcl` | `TCL84` | Cadence Genus/Innovus/Tempus (8.4-safe core) |
| `xilinx-eda-tcl` | `TCL85` | Xilinx Vivado |
| `intel-quartus-eda-tcl` | `TCL85` | Intel Quartus |
| `mentor-eda-tcl` | `TCL86` | Mentor ModelSim/Questa |
| `expect` | `TCL86` | Expect |

Note: a dialect's **signature base** (which commands are available) and its
**runtime base** (how commands behave) are modelled as two deliberately
separate fields (`docs/design/dialect-profile-model.md` §2.1).  For iRules
both are Tcl 8.4 — RATIFIED (D3): iRules embeds a genuine Tcl 8.4.6 and
nothing is backported at any BIG-IP version, so `dict`/`lassign` (8.5) and
`lmap`/`throw` (8.6) are never present, and version-dependent behaviour
like `incr` on uninitialised variables follows 8.4 semantics.  The F5
command surface (`HTTP::*`, `pool`, …) is a *versioned library* keyed by
BIG-IP version, orthogonal to the pinned Tcl base.

### Version-gated traits

Version-dependent `CommandSpec` fields take a `DialectSet` mask rather than
an enumerated list of dialect names.  `safe_on_uninit: Option<DialectSet>`
on `incr`, for example, is `Some(DialectSet::TCL85_PLUS)` — the derived
masks are `const`, so they compose at compile time:

```rust
/// Tcl 8.5 and later.
const TCL85_PLUS = Self::TCL85.bits() | Self::TCL86.bits()
                 | Self::TCL90.bits() | Self::TCL91.bits();
```

Because the masks are membership bits, not base versions, the vendor
dialects are *not* swept in by `TCL85_PLUS` — a dialect only intersects the
mask if the spec explicitly lists its bit.  `f5-bigip` has no Tcl command
surface at all, so it never intersects a version mask.

**Contract**: when adding a new dialect, add its name to `KNOWN_DIALECTS`,
give it a bit in the `DialectSet` bitflags, teach `DialectSet::parse` the
name, and — if it embeds a real Tcl core — add it to
`DialectSet::expr_grammar_base_version` so the runtime-behaviour gates
resolve.

### Dialect filtering

- `CommandSpec::dialects = None` → available in **all** dialects.
- `CommandSpec::dialects = Some(DialectSet::IRULES)` → iRules-only
  (e.g. `HTTP::host`, `pool`, `table`).
- `CommandSpec::supports_dialect(dialect)` is `true` when `dialects` is
  `None`, otherwise when the spec's set `intersects` the active one.
- The option/constraint layer's `supports_dialect(dialect, parent_dialects)`
  checks its own `dialects` gate first, falling back to the owning command
  or subcommand's set.

### Unavailable commands

A command that is known to the registry but whose `DialectSet` does not
intersect the active dialect is *not* reported as unknown: the analyser
distinguishes "exists elsewhere" from "not known anywhere" and emits
**W002** ("Command is disabled in active dialect profile") with an
"available in: …" hint derived from the spec's own dialect gate, rather
than the unresolved-command **W123**.  Deprecation and version floors are
separate axes, carried by `Lifecycle`
(`rust/tcl-registry/src/lifecycle.rs`) and consumed by
`rust/tcl-compiler/src/analyser/diagnostics/version_gate.rs`.

### Event requirements (iRules)

`EventRequires` (`rust/tcl-registry/src/events.rs`) declares when a command
is valid:

| Field | Rust type | Purpose | Example |
|-------|-----------|---------|---------|
| `client_side` | `bool` | Needs client-side connection | `true` for request-side commands |
| `server_side` | `bool` | Needs server-side connection | `true` for response-side commands |
| `transport` | `Option<&'static str>` | TCP or UDP | `Some("tcp")` for HTTP commands |
| `profiles` | `&'static [&'static str]` | Required profile types | `&["FASTHTTP", "HTTP"]` |
| `also_in` | `&'static [&'static str]` | Events where the command is unconditionally valid | Events not matching other criteria |
| `init_only` | `bool` | Only valid in RULE_INIT | Initialisation-only commands |
| `flow` | `bool` | Needs active traffic flow | Flow-dependent commands |
| `capability` | `Option<&'static str>` | Profile capability | `Some("sni")` for SNI-dependent |

**Example** — `HTTP::host`
(`rust/tcl-registry/src/commands/irules/http__host.rs`):

```rust
event_requires: Some(EventRequires {
    client_side: false,
    server_side: false,
    transport: Some("tcp"),
    profiles: &["FASTHTTP", "HTTP"],
    also_in: &[],
    init_only: false,
    flow: false,
    capability: None,
}),
```

A few commands have subforms with different event contracts.  Those declare
`event_requirement_forms: &[EventRequirementForm { argument_prefix, requires,
only_in }]`, and `CommandSpec::event_requirements_for_args` picks the
longest matching literal prefix, falling back to the command-level contract
for a dynamic or unmatched call.

### Event validation

The validator matches `EventRequires` against `EventProps` (which describes
what each event provides: client/server side, transport, implied profiles).
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

`EventProps::data_collect_protocols` lists the protocol alternatives for a data
event. `CLIENT_DATA` and `SERVER_DATA` include UDP as an implicit alternative,
so a standalone handler cannot be proved never to fire merely because it has
no `TCP::collect`. The analyser stays silent until the source gives stronger
evidence, such as a TCP payload access.

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
| **Semantic analysis** | W002 for dialect-disabled commands, W123 for unresolved ones |
| **Variable analysis** | `safe_on_uninit: Option<DialectSet>` intersected with the active dialect |
| **Completions** | Only show commands valid in the active dialect |
| **Taint** | Taint sources/sinks are dialect-specific (iRules HTTP commands) |
| **Side effects** | iRules-specific storage scopes (EVENT, CONNECTION) |
| **Connection scope** | Only applies to iRules multi-event scripts |

## Decision rule

- When adding a new iRules command, always set
  `dialects: Some(DialectSet::IRULES)` and configure `event_requires` with
  the appropriate transport and profiles.
- If a command works in both iRules and standard Tcl, union the bits:
  `dialects: Some(DialectSet::IRULES.union(DialectSet::TCL86))`.
- For version-specific commands (e.g. `tcl9.0`-only), set
  `dialects: Some(DialectSet::TCL90_PLUS)` — a `9.0` membership is inherited
  under `9.1`, so prefer the `_PLUS` mask over a bare `TCL90`.
- For version-dependent **behaviour** (not availability), put the mask on
  the relevant `CommandSpec` field rather than branching on dialect names.
  Example: `safe_on_uninit: Some(DialectSet::TCL85_PLUS)` for `incr`.
- When adding a new dialect, add its name to `KNOWN_DIALECTS`, give it a
  `DialectSet` bit, teach `DialectSet::parse` the name, and add it to
  `DialectSet::expr_grammar_base_version` if it embeds a real Tcl core.
- If IRULE1001 fires incorrectly, check that the event's `EventProps`
  includes the required profiles and transport.

## Related docs

- [Command infrastructure — Dialects](../../../docs/design/example-script-walkthroughs.md#dialects)
- [Command infrastructure — Events](../../../docs/design/example-script-walkthroughs.md#events-irules-only)
- [kcs-command-registry.md](../../../docs/design/compiler/command-registry.md)
- [kcs-connection-scope.md](../../../docs/design/compiler/connection-scope.md)
