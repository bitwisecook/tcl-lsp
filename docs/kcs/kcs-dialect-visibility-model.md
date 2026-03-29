# KCS: Dialect visibility model

## Symptom

Commands are unexpectedly visible or invisible in certain dialects,
or dialect inheritance isn't working correctly.

## Operational context

`DialectFlags` is a bitmask in `native/include/tcl_lsp/registry/command_desc.hpp`.
`dialect_visibility()` in `command_registry.hpp` expands a single dialect
flag to its full visibility set.

## Decision rules / contracts

### Inheritance chains

| Dialect | Sees commands from |
|---------|-------------------|
| `TCL84` | `TCL84`, `ALL` |
| `TCL85` | `TCL85`, `ALL` |
| `TCL86` | `TCL86`, `ALL` |
| `TCL90` | `TCL90`, `ALL` |
| `IRULES` | `TCL84`, `IRULES`, `ALL` |
| `TMSH` | `TCL84`, `TCL85`, `TCL86`, `TMSH`, `ALL` |
| `EXPECT` | `TCL84`, `TCL85`, `TCL86`, `EXPECT`, `ALL` |
| `IAPPS` | `TCL84`, `TCL85`, `IAPPS`, `ALL` |
| `TK` | `TCL84`–`TCL90`, `TK`, `ALL` |
| `EDA_*` | `TCL84`, `TCL85`, `EDA_*`, `ALL` |

### Setting dialects on CommandDesc

- `DialectFlags::ALL` — universal command (e.g. `set`, `proc`).
  Visible in every dialect.
- `DialectFlags::IRULES` — iRules-only (e.g. `HTTP::header`, `when`).
- `DialectFlags::TCL86 | DialectFlags::TCL90` — tcl8.6+ only
  (e.g. `oo::define`, `lmap`).  Not visible in iRules (tcl8.4-based).
- `DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_CADENCE` — EDA commands
  shared between Synopsys and Cadence tools.

### Subcommand dialect narrowing

A `SubCmdDesc` can set its own `dialects` to restrict visibility further.
If `dialects == DialectFlags::ALL`, it inherits the parent command's
visibility.

## Gotchas

- iRules is based on Tcl 8.4, so commands introduced in 8.5+ (`lmap`,
  `oo::define`, `try`) are NOT visible in iRules mode.
- TMSH is based on Tcl 8.6, so it sees everything up to 8.6 but not 9.0.
- `dialect_visibility()` only expands single-flag inputs.  Multi-flag
  or `ALL` inputs pass through unchanged.
- Don't use `NONE` as a command's dialect — that means "visible nowhere".
  Use `ALL` for universal commands.
