# KCS: feature — Semantic Tokens

## Summary

Rich syntax highlighting for regex, format strings, binary specs, and clock formats.

## Surface

lsp, all-editors

## How to use

- **Editor**: Applied automatically on top of the TextMate grammar. Provides more accurate highlighting for embedded DSLs within Tcl strings.
- **Settings**: Toggle with `tclLsp.features.semanticTokens`.

## Operational context

Semantic tokens add highlighting for constructs the TextMate grammar cannot handle: regular expression syntax inside `regexp`/`regsub`, `format`/`scan` specifiers, `binary format`/`scan` field descriptors, and `clock format`/`scan` directives.

## File-path anchors

- `lsp/features/semantic_tokens.py`

## Failure modes

- Token types misclassified after regex or format parser changes.
- Tokens not applied for new embedded DSL patterns.

## Test anchors

- `tests/test_semantic_tokens.py`

## Screenshots

- `09-semantic-highlighting` — rich syntax highlighting for embedded DSLs

![rich syntax highlighting for embedded DSLs](../screenshots/09-semantic-highlighting.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../kcs-lsp-feature-providers.md)
