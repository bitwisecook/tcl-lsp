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
| **Tcl: Restart Language Server** | Stop and restart the language server. Use when settings that are read at startup have changed (server path, native binary path, log level). |
| **Tcl: Select Dialect** | Open a picker to switch between the Tcl 8.4–9.1, F5 iRules, F5 iApps, F5 tmsh, F5 BIG-IP, BPF, Expect, SpecTcl, SslicTcl, and EDA-tool dialects. |
| **Tcl: Export Settings to Config File** | Write the current LSP settings to an XDG-compatible configuration file so they persist outside VS Code. |
| **Tcl: Toggle Optimiser Suggestions** | Flip `tclLsp.optimiser.enabled` on or off. When enabled, hint-level O-code diagnostics appear in the editor. |
| **Tcl: Toggle AI Features** | Flip `tclLsp.ai.enabled` on or off. When disabled, the `@irule`, `@tcl`, and `@tk` chat participants are removed. |

## Options

- `tclLsp.notifications.environmentKind` — show a one-time notice the first time a tool environment such as Vivado or Quartus is in use, explaining that it is a Tcl release plus library packages. Default `true`. Editors with no settings screen can set `environment_kind = false` under `[notifications]` in `config.ini`. See [Tool Environments](kcs-feature-tool-environments.md).
- `tclLsp.notifications.highlightingHealth` — warn when semantic highlighting cannot reach a Tcl file. Default `true`.

## Example

After updating the `tclLsp.rustServerPath` setting, the server keeps running with the old native binary until you run **Tcl: Restart Language Server** from the Command Palette. The output channel logs the native server path on restart.

## Related

- [KCS feature index](README.md)
- [Dialect Selection](kcs-feature-dialect-selection.md) — the picker in detail
- [Tool Environments](kcs-feature-tool-environments.md) — what the Vivado, Quartus, and other EDA entries are, and the notice about them
- [LSP features are missing](../kcs-issue-lsp-features-are-missing.md) — troubleshooting when the server does not start
- [XDG Configuration](../../design/contracts/xdg-config.md) — the file format Export Settings to Config File writes
