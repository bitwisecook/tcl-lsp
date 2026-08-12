# KCS: feature — Text Transforms

> **Audience:** User
> **Type:** Functionality

## Summary

Escape, unescape, and base64 encode/decode selected text.

## Applies to

VS Code

## Availability

| Context | How |
|---------|-----|
| VS Code | `Tcl: Escape Selection`, `Tcl: Unescape Selection`, `Tcl: Base64 Encode Selection`, `Tcl: Base64 Decode Selection` |
| VS Code (file explorer) | `Tcl: Copy File as Base64`, `Tcl: Copy File as Gzip+Base64` |

## How to use

- **VS Code**: Select text, then run the transform from the command palette or the right-click Tcl submenu. Requires a text selection for in-editor transforms.

## Operational context

These commands help work with Tcl-escaped strings and base64-encoded payloads commonly used in iRules data-groups and BIG-IP configurations.

## Failure modes

- Transform produces invalid encoding for edge-case inputs.

## Test anchors

- `editors/vscode/src/test/selectionTransforms.test.ts`

## Example

Selecting `Hello, "World"!` in the editor and running **Tcl:
Escape Selection** replaces the selection with the Tcl-escaped
form:

```
Hello, \"World\"!
```

Selecting the literal `hello` and running **Tcl: Base64 Encode
Selection** replaces it with `aGVsbG8=`. Running **Tcl: Base64
Decode Selection** on that string brings `hello` back.

## Discoverability

- [KCS feature index](README.md)
- [VS Code extension contracts](../../../docs/design/contracts/vscode-extension.md)
