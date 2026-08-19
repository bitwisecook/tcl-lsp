# KCS: feature — Dialect Selection

> **Audience:** User
> **Type:** Functionality

## Summary

Switch between Tcl versions and iRules/iApps/BIG-IP/EDA dialects to get dialect-specific analysis.

## Applies to

MCP, all-editors

## Availability

| Context | How |
|---------|-----|
| VS Code | `Tcl: Select Dialect` command palette, or automatic from file extension |
| Any LSP editor | `tclLsp.dialect` workspace setting |
| MCP | `set_dialect` tool |
| CLI | `--dialect` flag on the `tcl` binary |

## How to use

- **VS Code**: Run `Tcl: Select Dialect` from the command palette and pick from the eighteen profiles: tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, f5-irules, f5-iapps, f5-tmsh, f5-bigip, bpf, expect, spectcl, cadence-eda-tcl, intel-quartus-eda-tcl, mentor-eda-tcl, microchip-libero-eda-tcl, synopsys-eda-tcl, xilinx-eda-tcl.
- **Other editors**: Set `tclLsp.dialect` in workspace settings.
- **MCP**: Call `set_dialect` with the dialect name.
- **Automatic**: each profile owns its file extensions — `.irul`/`.irule`/`.irules` → f5-irules; `.iapp`/`.iappimpl`/`.impl` → f5-iapps; `.tmsh` → f5-tmsh; `.scf` (and `bigip.conf`) → f5-bigip; `.exp`/`.expect` → expect; `.tclspec` → spectcl; `.globals` → cadence-eda-tcl; `.qsf`/`.qpf`/`.qip` → intel-quartus-eda-tcl; `.do` → mentor-eda-tcl; `.sdc`/`.upf` → synopsys-eda-tcl; `.xdc` → xilinx-eda-tcl. bpf and microchip-libero-eda-tcl own no extension, so pick them by setting or `# tcl-dialect:` comment. Shebang `#!/usr/bin/expect` also triggers the expect dialect, and a SpecTcl pack can route further extensions with a `file_extension` row.

## Operational context

The dialect controls which commands are available in completions and hover, which diagnostic rules apply, and which event metadata is loaded. iRules dialects enable iRules-specific commands (HTTP::, IP::, etc.) and event handlers.

## Failure modes

- Wrong dialect produces false-positive diagnostics.
- Dialect not persisted across restarts.

## Screenshots

- `25-dialect-selection` — dialect picker showing available dialects

![dialect picker showing available dialects](../screenshots/25-dialect-selection.png)

## Discoverability

- [KCS feature index](README.md)
- [Command registry event model](../../../docs/design/contracts/command-registry-event-model.md)
