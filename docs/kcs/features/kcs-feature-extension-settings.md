# KCS: feature — Extension Settings and Server Control

> **Audience:** User
> **Type:** Functionality

## Summary

VS Code commands for restarting the language server, switching dialects, exporting configuration, and toggling AI or optimiser features.

## Applies to

VS Code

## Question

How do I restart the server, switch dialect, or toggle the AI and optimiser features in VS Code?

## How to use

Open the Command Palette (`Ctrl+Shift+P` or `Cmd+Shift+P`) and type `Tcl:` to see the full list. Key commands:

| Command | What it does |
|---------|-------------|
| **Tcl: Restart Language Server** | Stop and restart the language server. Use when settings that are read at startup have changed (Python path, server path, log level). |
| **Tcl: Select Dialect** | Open a picker to switch between Tcl 8.4, 8.5, 8.6, 9.0, F5 iRules, F5 iApps, and EDA tool dialects. |
| **Tcl: Export Configuration** | Write the current LSP settings to an XDG-compatible configuration file so they persist outside VS Code. |
| **Tcl: Toggle Optimiser Suggestions** | Flip `tclLsp.optimiser.enabled` on or off. When enabled, hint-level O-code diagnostics appear in the editor. |
| **Tcl: Toggle AI Features** | Flip `tclLsp.ai.enabled` on or off. When disabled, the `@irule`, `@tcl`, and `@tk` chat participants are removed. |

## Example

After updating the `tclLsp.pythonPath` setting, the server keeps running with the old interpreter until you run **Tcl: Restart Language Server** from the Command Palette. The output channel logs the new interpreter path on restart.

## Related

- [KCS feature index](README.md)
- [Dialect Selection](kcs-feature-dialect-selection.md) — the picker in detail
- [LSP features are missing](../kcs-issue-lsp-features-are-missing.md) — troubleshooting when the server does not start
- [XDG Configuration](../../design/contracts/xdg-config.md) — the file format used by Export Configuration
