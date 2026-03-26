# KCS: feature — Semantic Tokens

## Summary

Rich syntax highlighting for regex, format strings, binary specs, and clock formats with incremental delta delivery and per-chunk caching.

## Surface

lsp, all-editors

## How to use

- **Editor**: Applied automatically on top of the TextMate grammar. Provides more accurate highlighting for embedded DSLs within Tcl strings.
- **Settings**: Toggle with `tclLsp.features.semanticTokens`.

## Operational context

Semantic tokens add highlighting for constructs the TextMate grammar cannot handle: regular expression syntax inside `regexp`/`regsub`, `format`/`scan` specifiers, `binary format`/`scan` field descriptors, and `clock format`/`scan` directives.

## Performance and caching

- **Delta encoding**: The server advertises `textDocument/semanticTokens/full/delta`. After the first full response, editors request deltas — only the changed portion of the token array is sent.  If the delta would be larger than a full response, the server falls back to full automatically.
- **Range support**: The server advertises `textDocument/semanticTokens/range`. Editors can request tokens for only the visible viewport, reducing payload size for large files.
- **Per-chunk token cache**: Tokens are cached per top-level chunk (command).  After an edit, only dirty chunks are recomputed; unchanged chunks reuse cached absolute-position tokens.  When all chunks are cached, delta requests skip the entire collection pipeline and assemble directly from cache.
- **Fast source path**: On `didChange`, `update_source_quick()` updates source text and chunks on the event loop before yielding, so queued semantic-token requests can be served immediately with the new source — even before analysis completes.  A `workspace/semanticTokens/refresh` notification is sent after analysis finishes so editors re-request tokens with analysis enrichment (e.g. regex variable positions).
- **`DocumentBuffer`**: All position conversions (offset-to-line/col, chunk line ranges) go through the shared `DocumentBuffer`, which caches the line-starts index and provides O(log n) lookups via `bisect`.
- **Thread safety**: Semantic token result caching uses a bounded, thread-safe store (`_SemanticTokenState`) with automatic eviction. Per-document chunk caches are protected by `DocumentState._lock`.

## Editor integration

- **VS Code**: `vscode-languageclient` auto-negotiates delta when the server advertises it. No extra configuration needed.
- **Neovim**: The built-in LSP client (`vim.lsp.semantic_tokens`) supports delta natively.
- **Other editors**: Any LSP client that advertises `requests.full.delta` in its capabilities will receive delta responses.

## File-path anchors

- `lsp/features/semantic_tokens.py`
- `core/common/document_buffer.py`
- `lsp/workspace/document_state.py` — chunk cache storage and `update_source_quick()`
- `lsp/server.py` — `on_semantic_tokens_full`, `on_semantic_tokens_delta`, `on_semantic_tokens_range`

## Failure modes

- Token types misclassified after regex or format parser changes.
- Tokens not applied for new embedded DSL patterns.
- Chunk cache returns stale tokens after an offset-shifting edit (cache is invalidated when chunk hashes change).
- Delta encoding produces incorrect edits if token arrays are not sorted by position before encoding.

## Test anchors

- `tests/test_semantic_tokens.py`
- `tests/test_semantic_tokens_delta.py` — delta encoding, chunk boundary, multi-cursor edits, batch role lookup

## Screenshots

- `09-semantic-highlighting` — rich syntax highlighting for embedded DSLs

![rich syntax highlighting for embedded DSLs](../screenshots/09-semantic-highlighting.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../kcs-lsp-feature-providers.md)
