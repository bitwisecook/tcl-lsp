# KCS: feature — Semantic Tokens

> **Audience:** User
> **Type:** Functionality

## Summary

Rich syntax highlighting for regex, format strings, binary specs, and clock formats with incremental delta delivery and per-chunk caching.

## Applies to

all-editors, analyser

## How to use

- **Editor**: Applied automatically on top of the TextMate grammar. Provides more accurate highlighting for embedded DSLs within Tcl strings.
- **Settings**: Toggle with `tclLsp.features.semanticTokens`.

## Operational context

Semantic tokens add highlighting for constructs the TextMate grammar cannot handle: regular expression syntax inside `regexp`/`regsub`, `format`/`scan` specifiers, `binary format`/`scan` field descriptors, and `clock format`/`scan` directives.

The provider also recurses into script bodies that are not a whole command argument. An `apply` lambda `{argList body}` has its parameter list highlighted as parameters and its body highlighted as code — commands, variables, and nested substitutions inside the lambda body are tokenised the same way a `proc` body is, instead of appearing as one opaque braced string.

## Performance and caching

- **Delta encoding**: The server advertises `textDocument/semanticTokens/full/delta`. After the first full response, editors request deltas — only the changed portion of the token array is sent.  If the delta would be larger than a full response, the server falls back to full automatically.
- **Per-chunk token cache**: Tokens are cached per top-level chunk (command).  After an edit, only dirty chunks are recomputed; unchanged chunks reuse cached absolute-position tokens.
- **Fast source path**: On `didChange`, `update_source_quick()` updates source text and chunks on the event loop before yielding, so queued semantic-token requests can be served immediately with the new source — even before analysis completes.  A `workspace/semanticTokens/refresh` notification is sent after analysis finishes so editors re-request tokens with analysis enrichment (e.g. regex variable positions).
- **`DocumentBuffer`**: All position conversions (offset-to-line/col, chunk line ranges) go through the shared `DocumentBuffer`, which caches the line-starts index and provides O(log n) lookups via `bisect`.

## File-path anchors

- `server/features/_semantic_tokens/`
- `shared/document_buffer.py`
- `server/workspace/document_state.py` — chunk cache storage and `update_source_quick()`
- `server/server.py` — `on_semantic_tokens_full`, `on_semantic_tokens_delta`

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
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
