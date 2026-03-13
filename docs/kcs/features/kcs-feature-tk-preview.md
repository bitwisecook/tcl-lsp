# KCS: feature — Tk Preview

## Summary

Live preview pane for Tk GUI applications that updates as you edit.

## Surface

vscode-command, vscode-chat, mcp

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

## File-path anchors

- `editors/vscode/src/tkPreviewPanel.ts`
- `editors/vscode/src/tkPreviewPanelHtml.ts`
- `editors/vscode/src/tkLivePreview.ts`
- `core/tk/extract.py`

## Failure modes

- Preview blank for unsupported widget patterns.
- Layout incorrect when grid/pack options are complex.

## Test anchors

- `tests/test_tk_extract.py`

## Discoverability

- [KCS feature index](README.md)
- [VS Code extension contracts](../kcs-vscode-extension-contracts.md)
