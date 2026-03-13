# KCS: feature — Text Transforms

## Summary

Escape, unescape, and base64 encode/decode selected text.

## Surface

vscode-command

## Availability

| Context | How |
|---------|-----|
| VS Code | `Tcl: Escape Selection`, `Tcl: Unescape Selection`, `Tcl: Base64 Encode Selection`, `Tcl: Base64 Decode Selection` |
| VS Code (file explorer) | `Tcl: Copy File as Base64`, `Tcl: Copy File as Gzip+Base64` |

## How to use

- **VS Code**: Select text, then run the transform from the command palette or the right-click Tcl submenu. Requires a text selection for in-editor transforms.

## Operational context

These commands help work with Tcl-escaped strings and base64-encoded payloads commonly used in iRules data-groups and BIG-IP configurations.

## File-path anchors

- `editors/vscode/src/extension.ts`

## Failure modes

- Transform produces invalid encoding for edge-case inputs.

## Test anchors

- `editors/vscode/src/test/selectionTransforms.test.ts`

## Discoverability

- [KCS feature index](README.md)
- [VS Code extension contracts](../kcs-vscode-extension-contracts.md)
