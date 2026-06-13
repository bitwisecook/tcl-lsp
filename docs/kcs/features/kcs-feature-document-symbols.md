# KCS: feature — Document Symbols

> **Audience:** User
> **Type:** Functionality

## Summary

Outline of procs, namespaces, event handlers, and variables in the current file.

## Applies to

all-editors, MCP, analyser

## How to use

- **Editor**: Ctrl+Shift+O or the Outline panel.
- **MCP**: `symbols` tool — pass source code.
- **Settings**: Toggle with `tclLsp.features.documentSymbols`.

## Operational context

Produces a hierarchical symbol tree with procs nested inside namespaces, variables inside procs, and event handlers (iRules `when` blocks) at the top level.

A BIG-IP `.conf` file (any canonical basename — `bigip.conf`, `bigip_base.conf`, …) gets a different outline shape: a `module → kind → object` tree built from the config stanza tree rather than the Tcl scope walk. Nameless global singletons (`auth password-policy`, `net self-allow`, …) fall back to their kind label so no outline entry is ever empty. Both the Python and the native Rust servers serve this outline.

## File-path anchors

- `server/features/document_symbols.py`
- `server/features/_bigip_symbols.py` — BIG-IP `module → kind → object` outline (Python)
- `rust/tcl-lsp-core/src/bigip.rs` — BIG-IP outline + basename detection (native server)

## Failure modes

- Symbols missing or mis-nested after parser changes.
- VS Code drops the entire outline when any `DocumentSymbol.name` is empty — BIG-IP nameless singletons must fall back to a kind label (#534).

## Test anchors

- `tests/test_document_symbols.py`
- `tests/lsp_e2e/test_bigip_e2e.py` — BIG-IP outline + diagnostic suppression, both backends

## Screenshots

- `17-document-symbols` — symbol picker showing proc outline

![symbol picker showing proc outline](../screenshots/17-document-symbols.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
