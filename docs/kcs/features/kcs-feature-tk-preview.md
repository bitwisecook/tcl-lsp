# KCS: feature — Tk Preview

> **Audience:** User
> **Type:** Functionality

## Summary

Live preview pane for Tk GUI applications that updates as you edit.

## Applies to

VS Code, Copilot Chat, MCP

## Availability

| Context | How |
|---------|-----|
| VS Code command | `Tcl: Open Tk Preview` |
| VS Code chat | `@tk /preview` |
| MCP | `tk_layout` tool (widget tree extraction) |

## How to use

- **VS Code**: Open a file containing `package require Tk` and run `Tcl: Open Tk Preview`. The preview updates live as you edit.
- **VS Code chat**: `@tk /preview` opens the preview for the current file.
- **MCP**: `tk_layout` extracts the widget tree structure for analysis.

## Operational context

The Tk preview extracts the widget hierarchy from source code and renders it in a webview panel. It does not execute the Tcl code — it statically analyses the widget creation calls.

## Failure modes

- Preview blank for unsupported widget patterns.
- Layout incorrect when grid/pack options are complex.

## Example

With this file open in the editor:

```tcl
package require Tk
frame .main
label .main.title -text "Welcome"
entry .main.name
button .main.ok -text "OK"
pack .main.title .main.name .main.ok -side top
pack .main
```

Running **Tcl: Open Tk Preview** opens a side panel rendering a
frame with a "Welcome" label, an empty entry box, and an "OK"
button stacked vertically. Edits to the file refresh the panel
live — for example, changing `-text "Welcome"` to `-text "Hello"`
updates the rendered label without reloading.

## Discoverability

- [KCS feature index](README.md)
- [VS Code extension contracts](../../../docs/design/contracts/vscode-extension.md)
