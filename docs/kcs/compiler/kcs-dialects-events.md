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

Source: [`core/commands/registry/dialects.py`](../../../core/commands/registry/dialects.py),
[`core/commands/registry/models.py`](../../../core/commands/registry/models.py),
[`core/commands/registry/namespace_models.py`](../../../core/commands/registry/namespace_models.py)

## Content

### Known dialects

```python
KNOWN_DIALECTS = frozenset({
    "tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0",   # Tcl version dialects
    "f5-irules",                                 # F5 iRules
    "f5-iapps",                                  # F5 iApps
    "synopsys-eda-tcl",                          # Synopsys EDA
    "cadence-eda-tcl",                           # Cadence EDA
    "xilinx-eda-tcl",                            # Xilinx/AMD EDA
    "intel-quartus-eda-tcl",                     # Intel Quartus
    "mentor-eda-tcl",                            # Mentor/Siemens EDA
})
```

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
- If IRULE1001 fires incorrectly, check that the event's `EventProps`
  includes the required profiles and transport.

## Related docs

- [Command infrastructure — Dialects](../../example-script-walkthroughs.md#dialects)
- [Command infrastructure — Events](../../example-script-walkthroughs.md#events-irules-only)
- [kcs-command-registry.md](kcs-command-registry.md)
- [kcs-connection-scope.md](kcs-connection-scope.md)
