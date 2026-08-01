# KCS: feature — Workspace Symbols

> **Audience:** User
> **Type:** Functionality

## Summary

Search symbols across every indexed file in the workspace — including files
the editor has never opened.

## Applies to

all-editors, analyser

## How to use

- **Editor**: Ctrl+T and type a symbol name.
- **Settings**: Toggle with `tclLsp.features.workspaceSymbols`.

## Operational context

Answered from the workspace index (`WorkspaceIndex::symbols_matching`): every
proc, class, method, `classmethod`, constructor, and registry symbol-definer
definition — `tcltest` test cases, constraints, custom match modes, and iRules
`when EVENT` handlers — whose simple or qualified name contains the query,
case-insensitively (an empty query matches everything). The definer-backed
kinds are recorded from any command whose registry `CommandSpec` declares
`defines_symbol`, so the set grows by spec data rather than provider edits
(#790).

Because the answer comes from the index and not from the open-document map, a
symbol in a file the folder scan indexed but the editor never opened is
searchable (#1156). The index is refreshed on each document's diagnostics
publish, which the debounce puts about 50 ms behind an edit, so a name typed a
moment ago appears once that publish lands; an open-but-not-yet-published
buffer still contributes the symbols of its last publish.

The answer is capped at `MAX_WORKSPACE_SYMBOL_RESULTS` (1000). VS Code
re-issues the request on every keystroke in the Ctrl+T box, so the cap is
applied while scanning the index — it bounds the work, not merely the payload
— and the scan is document-major, so a truncated answer is a prefix of the
workspace rather than a prefix of one symbol table.

## File-path anchors

- `rust/tcl-lsp-core/src/workspace_symbols.rs` — the wire contract: symbol
  kinds, the query match, the result cap
- `rust/tcl-lsp-core/src/workspace_index.rs` — `WorkspaceIndex::symbols_matching`,
  the scan itself
- `rust/tcl-lsp-server/src/lib.rs` — the `workspace/symbol` handler, which
  resolves each hit's byte span against its document's source

## Failure modes

- Results up to one diagnostics publish behind the buffer for a document being
  edited.
- A hit is dropped when its document can be neither read from the open buffers
  nor from disk (a file deleted since it was indexed).

## Test anchors

- `rust/tcl-lsp-core/src/workspace_symbols.rs` — unit tests for kinds,
  containers, the query match, the unopened-document case, and the cap
- `rust/tcl-lsp-core/tests/lsp_edit_workspace.rs` — `workspace_symbols_*`
- `rust/tcl-lsp-server/src/lib.rs` — `workspace_symbol_finds_a_symbol_in_an_unopened_indexed_file`,
  `workspace_symbol_caps_its_answer`
- `rust/tcl-lsp-server/tests/e2e/structure.rs`,
  `rust/tcl-lsp-server/tests/e2e/document_symbols.rs`

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
