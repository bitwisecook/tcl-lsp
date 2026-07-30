# KCS: feature — Hover

> **Audience:** User
> **Type:** Functionality

## Summary

Command documentation, proc signatures, variable info, and taint status on hover.

## Applies to

all-editors, MCP, analyser

## How to use

- **Editor**: Hover over any symbol to see documentation.
- **MCP**: `hover` tool — pass source, line, and character position.
- **Settings**: Toggle with `tclLsp.features.hover`.

## Operational context

The hover provider resolves the symbol under the cursor and returns documentation from the command registry, proc signatures from analysis, variable types, and taint tracking status for iRules.

### Commands defined in another file

When nothing in the current document explains the word under the cursor and
that word is a command being called, hover looks further afield — the same two
steps go-to-definition takes:

1. **Across the workspace.** A `proc` or class declared in another open
   document, including one reached through `source`.
2. **Through the library index.** A command an installed library auto-loads
   (a `tclIndex` or `pkgIndex.tcl` on the configured library paths), even
   though no file in the workspace declares it.

The popup is rendered from the *defining* file, so it reads exactly as it does
when you hover the declaration itself.

Hover only looks further afield for a **command being called**. An ordinary
argument word that happens to share a name with a proc in another file shows
nothing, so a `puts widget` never pops up an unrelated `widget` procedure.

## File-path anchors

- `rust/tcl-lsp-core/src/hover.rs` — the provider and its renderers, including
  `qualified_symbol_hover` for a symbol defined in another document
- `rust/tcl-lsp-server/src/lib.rs` — `cross_document_hover`, the workspace and
  library-index fallback

## Failure modes

- Missing hover after command registry updates.
- Incorrect position mapping in multi-line constructs.
- A command reached only at run time (built by `eval`, or dispatched through a
  variable) has no declaration to point at, so hover shows nothing.

## Test anchors

- `rust/tcl-lsp-server/tests/e2e/hover.rs`
- `rust/tcl-lsp-core/src/hover.rs` — renderer unit tests

## Screenshots

- `02-hover-proc` — hover showing proc signature and documentation

![hover showing proc signature and documentation](../screenshots/02-hover-proc.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
