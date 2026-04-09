# KCS: feature — Rename

> **Audience:** User
> **Type:** Functionality

## Summary

Rename a proc or a variable consistently across the file.

## Surface

lsp, mcp, all-editors

## Question

What does the rename feature do, and how do I use it?

## How to use

- **In the editor**: put your cursor on the proc or variable, press
  `F2`, type the new name, and press **Enter**. The editor updates the
  definition and every reference in the current file in one step.
- **From a script or MCP tool**: call the `rename` tool with the source
  file, a cursor position, and the new name. The tool returns the full
  set of text edits for the editor or script to apply.

## Options

- `tclLsp.features.rename` — turn the rename feature on or off. Default:
  on.

## How it finds references

Rename uses the same shared proc-reference matching as **Find
References**, so the definition and every call site are always updated
together. For the full contract, see
[LSP feature providers](../../design/contracts/lsp-feature-providers.md).

## Failure modes

- The rename updates some but not all of the references. This almost
  always means the symbol is visible under more than one scope; run
  **Find References** first to confirm what would be touched.
- The rename is applied to a different symbol than the one you clicked.
  This can happen if the cursor is on a namespace-qualified name where
  the qualifier is ambiguous.

## Screenshots

![rename dialog inline](../screenshots/18-rename.png)

## Related

- [KCS feature index](README.md)
- [Glossary](../../GLOSSARY.md)
- [LSP feature providers](../../design/contracts/lsp-feature-providers.md)
