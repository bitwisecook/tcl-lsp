# KCS: feature — Dialect Selection

## Summary

Switch between Tcl versions and iRules/iApps/BIG-IP/EDA dialects to get dialect-specific analysis.

## Surface

vscode-command, mcp, all-editors

## Availability

| Context | How |
|---------|-----|
| VS Code | `Tcl: Select Dialect` command palette, or automatic from file extension |
| Any LSP editor | `tclLsp.dialect` workspace setting |
| MCP | `set_dialect` tool |
| CLI | `--dialect` flag on `tcl_ai.py` |

## How to use

- **VS Code**: Run `Tcl: Select Dialect` from the command palette and pick from: tcl8.4, tcl8.5, tcl8.6, tcl9.0, f5-irules, f5-iapps, f5-bigip, eda-tools.
- **Other editors**: Set `tclLsp.dialect` in workspace settings.
- **MCP**: Call `set_dialect` with the dialect name.
- **Automatic**: `.irul`/`.irule` files default to f5-irules; `.iapp` to f5-iapps; `bigip.conf` to f5-bigip.

## Operational context

The dialect controls which commands are available in completions and hover, which diagnostic rules apply, and which event metadata is loaded. iRules dialects enable iRules-specific commands (HTTP::, IP::, etc.) and event handlers.

## File-path anchors

- `core/commands/registry/runtime.py`
- `editors/vscode/src/extension.ts`

## Failure modes

- Wrong dialect produces false-positive diagnostics.
- Dialect not persisted across restarts.

## Test anchors

- `editors/vscode/src/test/dialectDetection.test.ts`

## Screenshots

- `25-dialect-selection` — dialect picker showing available dialects

![dialect picker showing available dialects](../screenshots/25-dialect-selection.png)

## Discoverability

- [KCS feature index](README.md)
- [Command registry event model](../kcs-command-registry-event-model.md)
