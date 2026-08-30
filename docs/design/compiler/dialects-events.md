# Dialects and events

How commands are partitioned across Tcl versions and tool contexts, and how
iRules event requirements narrow availability further. Read this when adding a
dialect-specific command, or when a command is reported as unknown
(IRULE1001 / W002) in one context but not another.

Dialects partition command availability.  Every `CommandSpec` has an optional
`dialects` field; subcommands can override with their own set.  In iRules,
commands are further restricted by event context — `EventRequires` declares
transport, profile, and connection-side requirements.

Source: `rust/tcl-dialect/src/dialect_set.rs` (`DialectSet`),
`rust/tcl-dialect/src/profile.rs` (`DialectProfile`, `LexerGrammar`),
`rust/tcl-registry/src/spec.rs` (`CommandSpec::dialects`),
`rust/tcl-registry/src/events.rs` (`EventRequires`)

### Known dialects

Dialects are bits of the `DialectSet` bitflags type
(`rust/tcl-dialect/src/dialect_set.rs`); `DialectSet::canonical_name` is
the name table, and `DialectSet::parse` its inverse:

```rust
bitflags! {
    pub struct DialectSet: u64 {
        const TCL84  = 1 << 0;   // "tcl8.4"
        const TCL85  = 1 << 1;   // "tcl8.5"
        const TCL86  = 1 << 2;   // "tcl8.6"
        const TCL90  = 1 << 3;   // "tcl9.0"
        const TCL91  = 1 << 14;  // "tcl9.1"
        const IRULES = 1 << 4;   // "f5-irules"
        const IAPPS  = 1 << 5;   // "f5-iapps"
        const TK     = 1 << 6;   // "tk"
        const EXPECT = 1 << 7;   // "expect"
        const BPF    = 1 << 13;  // "bpf"
        const TMSH   = 1 << 15;  // "f5-tmsh"
        const BIGIP  = 1 << 16;  // "f5-bigip"
        // …plus the composite constants ALL_TCL, TCL85_PLUS, …
    }
}
```

The EDA shells (Synopsys, Cadence, Xilinx/AMD, Intel Quartus,
Mentor/Siemens) have no surface of their own: they are modelled as a base
Tcl release plus `required_package`-gated command libraries, installed by
profile name from the bundled `specs/*.tclspec` packs.

### Dialect base versions

Each non-standard dialect is based on a specific Tcl runtime version.
`DialectSet::expr_grammar_base_version(name)`
(`rust/tcl-dialect/src/dialect_set.rs`) maps each dialect name to that base
and encodes exactly the table below, including `f5-irules`'s
runtime-vs-signature split.

The two axes are deliberately separate. Per-command *availability* gating
uses the `DialectSet` bitflags (`ALL_TCL`, `TCL85_PLUS`, `TCL90_PLUS`, …)
directly on `CommandSpec.dialects`, and does **not** fold the vendor
dialects into those flags — most standard-library commands they don't ship
are missing because of their own restricted surface, not version. Version-
gated *behaviour* uses `expr_grammar_base_version`, because every dialect
in the table is either plain Tcl or a vendor shell embedding a real Tcl
core and inherits that core's semantics wholesale. The *expr grammar*
features — TIP 201 (`in`/`ni`) and TIP 461 (`lt`/`le`/`gt`/`ge`), the W003
diagnostic — are the canonical consumers.

| Dialect | Base version | Runtime |
|---------|-------------|---------|
| `f5-irules` | `tcl8.4` | TMOS embedded Tcl 8.4.6 |
| `f5-iapps` | `tcl8.5` | CentOS 7 system Tcl 8.5.13 |
| `f5-tmsh` | `tcl8.5` | CentOS 7 system Tcl 8.5.13 |
| `f5-bigip` | *(omitted)* | Custom parser, not Tcl |
| `synopsys-eda-tcl` | `tcl8.6` | Synopsys DC/PT/ICC2 |
| `cadence-eda-tcl` | `tcl8.6` | Cadence Genus/Innovus/Tempus |
| `xilinx-eda-tcl` | `tcl8.5` | Xilinx Vivado |
| `intel-quartus-eda-tcl` | `tcl8.5` | Intel Quartus |
| `mentor-eda-tcl` | `tcl8.5` | Mentor ModelSim/Questa |
| `expect` | `tcl8.6` | Expect |

Note: a dialect's **signature base** (which commands are available) and its
**runtime base** (how commands behave) are modelled as two deliberately
separate fields (`docs/design/dialect-profile-model.md` §2.1).  For iRules
both are Tcl 8.4 — RATIFIED (D3): iRules embeds a genuine Tcl 8.4.6 and
nothing is backported at any BIG-IP version, so `dict`/`lassign` (8.5) and
`lmap`/`throw` (8.6) are never present, and version-dependent behaviour
like `incr` on uninitialised variables follows 8.4 semantics.  The F5
command surface (`HTTP::*`, `pool`, …) is a *versioned library* keyed by
BIG-IP version, orthogonal to the pinned Tcl base.

### Version-floor constants

`DialectSet` carries composite constants for the common version floors, so
a version-dependent `CommandSpec` field never lists dialect names:

```rust
const ALL_TCL    = TCL84 | TCL85 | TCL86 | TCL90 | TCL91;
const TCL85_PLUS =         TCL85 | TCL86 | TCL90 | TCL91;
```

Use them directly:

```rust
// incr: safe on an uninitialised variable in Tcl 8.5+, an error in 8.4
safe_on_uninit: Some(DialectSet::TCL85_PLUS),
```

`expr_grammar_base_version` returns `None` for a name it has no documented
base version for (`f5-bigip`, a config parser rather than Tcl at all;
`tk`; `bpf`). Callers treat `None` as "cannot reason about this dialect's
grammar version", not "assume plain Tcl".

**Contract**: when adding a new dialect, add its bit to `DialectSet`, its
name to `canonical_name` / `parse`, and its base version to
`expr_grammar_base_version`.

### Dialect filtering

- `dialects: None` → available in **all** dialects.
- `dialects: Some(DialectSet::IRULES)` → iRules-only (e.g. `HTTP::host`,
  `pool`, `table`).
- A `SubCommand`'s own `dialects` is consulted first; `None` there means
  "inherit the parent command's set".

### Availability outcomes

The analyser reports a head word that does not resolve in the active
dialect through three distinct codes:

| Code | Meaning |
|------|---------|
| `W002` | The command exists, but is disabled in the active dialect profile. The message carries an "available in: …" suffix built by `dialect_availability_suffix` from the spec's own dialect gate |
| `W123` | Unresolved command — not found in the registry, user procs, or an `unknown` handler |
| `W001` | The command resolved, but its first word is not a recognised subcommand. Suppressed when the spec sets `allow_unknown_subcommands` |

The emitters live in
`rust/tcl-compiler/src/analyser/diagnostics/validity.rs`.

### Event requirements (iRules)

`EventRequires` declares when a command is valid:

| Field | Purpose | Example |
|-------|---------|---------|
| `client_side` | Needs client-side connection | `true` for request-side commands |
| `server_side` | Needs server-side connection | `true` for response-side commands |
| `transport` | TCP or UDP | `Some("tcp")` for HTTP commands |
| `profiles` | Required profile list | `&["FASTHTTP", "HTTP"]` |
| `also_in` | Extra valid events | Events not matching other criteria |
| `init_only` | Only valid in RULE_INIT | Initialisation-only commands |
| `flow` | Needs active traffic flow | Flow-dependent commands |
| `capability` | Profile capability | `"sni"` for SNI-dependent |

**Example** — `HTTP::host`:
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

### Event validation

The validator matches `EventRequires` against `EventProps` (which describes
what each event provides: client/server side, transport, implied profiles).
Mismatches produce diagnostic `IRULE1001`.

`CommandSpec.excluded_events` lists events where a command is explicitly
forbidden.

### Data collection and side context

The command registry also records a `DataCollectionOperation` for commands
that participate in an iRules payload lifecycle. A consumer does not infer
meaning from a name ending in `::collect`, `::release`, or `::payload`.
That distinction matters: TCP, HTTP, and SSL payloads need collection; UDP
and ASM payloads are supplied by BIG-IP without an explicit collect command.
HTTP collection is released implicitly when its matching data event completes,
whereas TCP and SSL require an explicit release.

`EventProps.data_collect_protocols` lists the protocol alternatives for a data
event. `CLIENT_DATA` and `SERVER_DATA` include UDP as an implicit alternative,
so a standalone handler cannot be proved never to fire merely because it has
no `TCP::collect`. The analyser stays silent until the source gives stronger
evidence, such as a TCP payload access.

Nesting-script side changes are likewise declared through
`CommandSpec.side_switch_target` (`Client`, `Server`, or `Peer`). The flow
checker uses the descriptor while it recurses, so adding a side-switch command
does not require a compiler command-name branch.

Finally, `EventHandlerPriority` records whether an event handler has a runtime
default. BIG-IP's `when` default is priority 500; omitting `priority` is valid
and only an explicitly stricter dialect policy can request IRULE1004.

### How dialects feed the compiler

| Stage | Effect |
|-------|--------|
| **Semantic analysis** | W002 for disallowed commands, W123 for unresolved ones |
| **Variable analysis** | `safe_on_uninit` resolved per dialect against the spec's `DialectSet` |
| **Completions** | Only show commands valid in the active dialect |
| **Taint** | Taint sources/sinks are dialect-specific (iRules HTTP commands) |
| **Side effects** | iRules-specific storage scopes (EVENT, CONNECTION) |
| **Connection scope** | Only applies to iRules multi-event scripts |

## Decision rule

- When adding a new iRules command, always set
  `dialects: Some(DialectSet::IRULES)` and configure `event_requires` with
  the appropriate transport and profiles.
- If a command works in both iRules and standard Tcl, union the bits:
  `dialects: Some(DialectSet::ALL_TCL.union(DialectSet::IRULES))`.
- For version-specific commands (e.g. `tcl9.0`-only), set
  `dialects: Some(DialectSet::TCL90)`.
- For version-dependent **behaviour** (not availability), use a
  version-floor constant on the relevant `CommandSpec` field rather than
  listing dialect names.  Example:
  `safe_on_uninit: Some(DialectSet::TCL85_PLUS)` for `incr`.
- When adding a new dialect, add its bit to `DialectSet` and its base
  version to `DialectSet::expr_grammar_base_version`.
- If IRULE1001 fires incorrectly, check that the event's `EventProps`
  includes the required profiles and transport.

## Related docs

- [Command infrastructure — Availability](../../../docs/design/example-script-walkthroughs.md#availability)
- [Command infrastructure — Events](../../../docs/design/example-script-walkthroughs.md#events-irules-only)
- [kcs-command-registry.md](../../../docs/design/compiler/command-registry.md)
- [kcs-connection-scope.md](../../../docs/design/compiler/connection-scope.md)
