# KCS: feature — Workspace Symbols

> **Audience:** User
> **Type:** Functionality

## Summary

Search symbols across all open files in the workspace.

## Applies to

all-editors, analyser

## How to use

- **Editor**: Ctrl+T and type a symbol name.
- **Settings**: Toggle with `tclLsp.features.workspaceSymbols`.

## Operational context

Searches the workspace index for procs, namespaces, variables, and `tcltest` definitions (test cases, constraints, custom match modes) matching the query. Relies on the workspace scanner for cross-file indexing. These are recorded from any command whose registry `CommandSpec` declares `defines_symbol`, so the set grows by spec data rather than provider edits (#790).

## File-path anchors

- `server/features/workspace_symbols.py`

## Failure modes

- Stale results if the workspace index is not refreshed.

## Test anchors

- `tests/test_workspace_symbols.py`

## Example

In a workspace containing `lib/http.tcl` with:

```tcl
proc http_get {url} { ... }
proc http_post {url body} { ... }
```

Pressing Ctrl+T and typing `http_` lists `http_get` and
`http_post` — each entry shows the containing file and line
number, and selecting one jumps straight to its definition.

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
- [Workspace indexing contracts](../../../docs/design/contracts/workspace-indexing.md)
