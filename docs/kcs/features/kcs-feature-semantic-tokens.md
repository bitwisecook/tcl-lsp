# KCS: feature — Semantic Tokens

> **Audience:** User
> **Type:** Functionality

## Summary

Rich syntax highlighting for regex, format strings, binary specs, and clock formats, delivered incrementally as deltas.

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
is 0 — and quoting is the only way to write it. Which argument is a name comes
from the registry's `VarWrite` / `VarRead` roles, so a brace-quoted word
anywhere else (`puts {$n}`) stays a string.

## Performance and caching

- **Delta encoding**: the server advertises
  `textDocument/semanticTokens/full/delta`. After the first full response,
  editors request deltas — only the changed part of the token array is sent.
- **Coarse and enriched tiers**: the server races the analysis-enriched result
  (retagged regex sources, resolved object-method dispatch) against a 40 ms
  budget. A cold or very large document is served the cheap coarse tier
  (segmenter plus registry only) when the timer wins. Once the enriched result
  is ready and actually differs, a `workspace/semanticTokens/refresh` asks the
  editor to re-request. Bursts of those refreshes are coalesced into one per
  50 ms window. See
  [`docs/design/rust/lsp-performance.md`](../../design/rust/lsp-performance.md)
  and [`docs/design/rust/incremental-analysis.md`](../../design/rust/incremental-analysis.md).

## Failure modes

- Token types misclassified after regex or format parser changes.
- Tokens not applied for new embedded DSL patterns.
- Delta encoding produces incorrect edits if token arrays are not sorted by position before encoding.

## Screenshots

- `09-semantic-highlighting` — rich syntax highlighting for embedded DSLs

![rich syntax highlighting for embedded DSLs](../../screenshots/09-semantic-highlighting.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
