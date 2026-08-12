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

A word in a **variable-name argument position** is painted as a variable
whether it is a bareword or brace-quoted: `set n 1` and `set {$n} 1` are both
declarations, and `[set n]` / `[set {$n}]` both references. `{$n}` really does
name a variable — tclsh reports `info exists {$n}` as 1 while `info exists n`
is 0 — and quoting is the only way that variable can ever be written, so
painting the word as a plain string hid the declaration and made the `$n`
inside look like a substitution it is not. Which argument is a name comes from
the registry's `VarWrite` / `VarRead` roles, so a brace-quoted word anywhere
else (`puts {$n}`) stays a string.

## Performance and caching

- **Delta encoding**: The server advertises `textDocument/semanticTokens/full/delta`. After the first full response, editors request deltas — only the changed portion of the token array is sent.  If the delta would be larger than a full response, the server falls back to full automatically.
- **Per-chunk token cache**: Tokens are cached per top-level chunk (command).  After an edit, only dirty chunks are recomputed; unchanged chunks reuse cached absolute-position tokens.
- **Fast source path**: On `didChange`, `update_source_quick()` updates source text and chunks on the event loop before yielding, so queued semantic-token requests can be served immediately with the new source — even before analysis completes.  A `workspace/semanticTokens/refresh` notification is sent after analysis finishes so editors re-request tokens with analysis enrichment (e.g. regex variable positions).
- **`DocumentBuffer`**: All position conversions (offset-to-line/col, chunk line ranges) go through the shared `DocumentBuffer`, which caches the line-starts index and provides O(log n) lookups via `bisect`.
- **Rust server — coarse/enriched tiering**: the native server races the
  fully analysis-enriched result (retagged regex sources, resolved
  object-method dispatch) against a 40 ms budget timer. A cold or very
  large document serves the cheap coarse tier (segmenter + registry only)
  immediately when the timer wins, then a `workspace/semanticTokens/refresh`
  request is sent once the enriched result is ready and actually differs
  from what was served — the same delta-triggering mechanism as the
  Python fast-source path above, applied to the whole-document response
  rather than the per-chunk cache. See
  [`docs/design/rust/lsp-performance.md`](../../design/rust/lsp-performance.md)
  §7 and [`docs/design/rust/incremental-analysis.md`](../../design/rust/incremental-analysis.md)
  (Slice 6).

## File-path anchors

  race), `SemanticTokensRefreshCtx`, `db_semantic_tokens`
- `rust/tcl-lsp-db/src/lib.rs` — `semantic_tokens` salsa query

## Failure modes

- Token types misclassified after regex or format parser changes.
- Tokens not applied for new embedded DSL patterns.
- Chunk cache returns stale tokens after an offset-shifting edit (cache is invalidated when chunk hashes change).
- Delta encoding produces incorrect edits if token arrays are not sorted by position before encoding.

## Screenshots

- `09-semantic-highlighting` — rich syntax highlighting for embedded DSLs

![rich syntax highlighting for embedded DSLs](../screenshots/09-semantic-highlighting.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
