# KCS: feature — Document Links

> **Audience:** User
> **Type:** Functionality

## Summary

Clickable links for URLs and file paths in comments and strings.

## Applies to

all-editors, analyser

## How to use

- **Editor**: Ctrl+Click on a URL or file path to open it.
- **Settings**: Toggle with `tclLsp.features.documentLinks`.

## Operational context

The provider scans comments and string literals for URLs and file paths, making them clickable in the editor.

## Failure modes

- Links not detected for unusual URL schemes.

## Example

In this Tcl file:

```tcl
# See https://www.tcl-lang.org/man/tcl/TclCmd/string.htm for reference.
source lib/helpers.tcl
```

The URL in the comment becomes an underlined link that opens in
the browser on Ctrl+Click, and `lib/helpers.tcl` becomes a link
that opens the file in a new editor tab.

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
