# KCS: Dialects and events

## Symptom

A contributor needs to understand how commands are partitioned across Tcl
versions and tool contexts (dialects), how iRules event requirements work,
or is debugging why a command is reported as unknown (IRULE1001 / W102) in
a specific context.

## Context

Dialects partition command availability.  Every `CommandSpec` has an optional
`dialects` field; subcommands can override with their own set.  In iRules,
commands are further restricted by event context — `EventRequires` declares
transport, profile, and connection-side requirements.

Source: `compiler/registry/dialects.py`,
`compiler/registry/models.py`,
`compiler/registry/namespace_models.py`

## Content

### Known dialects

```python
KNOWN_DIALECTS = frozenset({
    "tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1",  # Tcl version dialects
    "f5-irules",                                 # F5 iRules
    "f5-iapps",                                  # F5 iApps
    "f5-tmsh",                                   # F5 tmsh scripts
    "f5-bigip",                                  # F5 BIG-IP config
    "synopsys-eda-tcl",                          # Synopsys EDA
    "cadence-eda-tcl",                           # Cadence EDA
    "xilinx-eda-tcl",                            # Xilinx/AMD EDA
    "intel-quartus-eda-tcl",                     # Intel Quartus
    "mentor-eda-tcl",                            # Mentor/Siemens EDA
    "expect",                                    # Expect
})
```

### Dialect base versions

Each non-standard dialect is based on a specific Tcl runtime version.
`DIALECT_BASE_VERSION` in `dialects.py` maps each dialect to its base.

**Rust port note**: `dialects.py` and `dialects_since()` below are the
retired Python implementation's names, kept here because the *table* of
base versions is still the source of truth. The Rust workspace has no
`dialects_since()` — per-command dialect gating uses the `DialectSet`
bitflags (`TCL85_PLUS` / `TCL86_PLUS` / `TCL90_PLUS`, `rust/tcl-registry/src/dialects.rs`)
directly on `CommandSpec.dialects`, and deliberately does **not** fold the
vendor dialects into those flags (most standard-library commands they
don't ship are missing because of their own restricted surface, not
version). For version-gated *expr grammar* features specifically — TIP 201
(`in`/`ni`) and TIP 461 (`lt`/`le`/`gt`/`ge`), the W003 diagnostic — where
every dialect here really does inherit a real embedded Tcl core's
operators wholesale, use `DialectSet::expr_grammar_base_version(name)`
instead, which encodes exactly this table (including `f5-irules`'s
runtime-vs-signature split below).

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

Note: a dialect's **signature base** (which commands are available) may
differ from its **runtime base** (how commands behave).  iRules loads
`tcl8.6` signatures for command availability, but its runtime is Tcl 8.4.6,
so version-dependent behaviour like `incr` on uninitialised variables
follows 8.4 semantics.

### dialects_since() -- version-gated traits

`dialects_since(min_version)` returns all dialects whose base Tcl version
is >= `min_version`.  Use this for version-dependent `CommandSpec` fields
instead of manually listing dialect names:

```python
from compiler.registry.dialects import dialects_since

# Returns {"tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "f5-iapps", "f5-tmsh",
#          "xilinx-eda-tcl", "intel-quartus-eda-tcl", "mentor-eda-tcl",
#          "synopsys-eda-tcl", "cadence-eda-tcl", "expect"}
dialects_since("tcl8.5")
```

Dialects not in `DIALECT_BASE_VERSION` (e.g. `f5-bigip`) are never
included -- safe default for non-Tcl contexts.

**Contract**: when adding a new dialect, add its entry to
`DIALECT_BASE_VERSION` in `dialects.py` so `dialects_since()` resolves
correctly.

### Dialect filtering

- `CommandSpec.dialects = None` → available in **all** dialects.
- `CommandSpec.dialects = frozenset({"f5-irules"})` → iRules-only
  (e.g. `HTTP::host`, `pool`, `table`).
- `SubCommand.supports_dialect()` checks the subcommand's own `dialects`
  set first, falling back to the parent command's.

### DialectStatus

```python
class DialectStatus(Enum):
    EXISTS       # available in this dialect
    DEPRECATED   # available but has a replacement
    DISALLOWED   # exists in some dialect, but not this one
    NOT_EXISTS   # not known anywhere
```

`DISALLOWED` produces diagnostic W102 with a hint about which dialect the
command belongs to.

### Event requirements (iRules)

`EventRequires` declares when a command is valid:

| Field | Purpose | Example |
|-------|---------|---------|
| `client_side` | Needs client-side connection | `true` for request-side commands |
| `server_side` | Needs server-side connection | `true` for response-side commands |
| `transport` | TCP or UDP | `"tcp"` for HTTP commands |
| `profiles` | Required profile set | `{"HTTP", "FASTHTTP"}` |
| `also_in` | Extra valid events | Events not matching other criteria |
| `init_only` | Only valid in RULE_INIT | Initialisation-only commands |
| `flow` | Needs active traffic flow | Flow-dependent commands |
| `capability` | Profile capability | `"sni"` for SNI-dependent |

**Example** — `HTTP::host`:
```python
event_requires=EventRequires(transport="tcp", profiles=frozenset({"HTTP", "FASTHTTP"}))
```

### Event validation

The validator matches `EventRequires` against `EventProps` (which describes
what each event provides: client/server side, transport, implied profiles).
Mismatches produce diagnostic `IRULE1001`.

`CommandSpec.excluded_events` lists events where a command is explicitly
forbidden.

### How dialects feed the compiler

| Stage | Effect |
|-------|--------|
| **Semantic analysis** | W102 for unknown/disallowed commands |
| **Variable analysis** | `safe_on_uninit` resolved per dialect via `DIALECT_BASE_VERSION` |
| **Completions** | Only show commands valid in the active dialect |
| **Taint** | Taint sources/sinks are dialect-specific (iRules HTTP commands) |
| **Side effects** | iRules-specific storage scopes (EVENT, CONNECTION) |
| **Connection scope** | Only applies to iRules multi-event scripts |

## Decision rule

- When adding a new iRules command, always set `dialects=frozenset({"f5-irules"})`
  and configure `event_requires` with the appropriate transport and profiles.
- If a command works in both iRules and standard Tcl, set
  `dialects=frozenset({"f5-irules", "tcl8.6", ...})`.
- For version-specific commands (e.g. `tcl9.0`-only), set
  `dialects=frozenset({"tcl9.0"})`.
- For version-dependent **behaviour** (not availability), use
  `dialects_since()` on the relevant `CommandSpec` field rather than
  listing dialect names.  Example: `safe_on_uninit=dialects_since("tcl8.5")`
  for `incr`.
- When adding a new dialect, add it to both `KNOWN_DIALECTS` and
  `DIALECT_BASE_VERSION` in `dialects.py`.
- If IRULE1001 fires incorrectly, check that the event's `EventProps`
  includes the required profiles and transport.

## Related docs

- [Command infrastructure — Dialects](../../../docs/design/example-script-walkthroughs.md#dialects)
- [Command infrastructure — Events](../../../docs/design/example-script-walkthroughs.md#events-irules-only)
- [kcs-command-registry.md](../../../docs/design/compiler/command-registry.md)
- [kcs-connection-scope.md](../../../docs/design/compiler/connection-scope.md)
