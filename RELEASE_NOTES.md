# v1.6.0

## New Features

- **Ten new LSP capabilities**, each behind its own feature toggle:
  - `textDocument/documentHighlight` — Read/Write-aware highlights driven by
    shared variable-scoping analysis
  - `textDocument/implementation` — jump from interfaces/base methods to
    concrete implementations
  - `textDocument/typeDefinition` and `textDocument/declaration`
  - `textDocument/codeLens` + `codeLens/resolve`
  - `textDocument/willSaveWaitUntil` — format-on-save handler
  - `textDocument/linkedEditingRange` — synchronised renaming of matching
    identifiers
  - `workspace/willRenameFiles` + `workspace/didRenameFiles`
  - `textDocument/diagnostic` and `workspace/diagnostic` pull-model handlers
    (opt-in; the existing push pipeline remains the default)
- **Workspace scan progress** via `$/progress` notifications, so editors can
  show a real progress indicator while the workspace index is being built.

## Improvements

- Shared variable-scoping detection underpins document highlights, with
  coordinated updates to the VS Code extension and bundled test fixtures.
- New VS Code extension tests covering all ten LSP features.
- KCS documentation overhaul for diagnosing failed VS Code LSP startup,
  including a reusable user-issue troubleshooting template.

## Bug Fixes

- **Fix `{*}` argument-expansion arity checks (#129).** Calls like
  `proc rgbToLab {r g b} { ... }; rgbToLab {*}$rgb` no longer raise false
  E002/E003 diagnostics. Expansion markers from the segmenter are now
  threaded through every arity-check call site and each expanded word
  contributes a `(min, max)` range resolved via the value lattice
  (literal lists, `[list ...]`, and constant-valued variables are expanded
  exactly; unknown expansions contribute `0..∞`). `TclLexer.expand_syntax`
  is dialect-gated, so Tcl 8.4 and F5 iRules correctly parse `{*}$x` as a
  braced literal rather than an expansion prefix.
- **Guard specialised IR lowerings against `{*}` expansion.** The
  specialised lowerings for `set`, `incr`, `expr`, `return`, `proc`,
  `when`, `namespace eval`, `if`, `switch`, `for`, `while`, `foreach`,
  and `foreach_in_collection` now fall back to a generic `IRCall` or
  `IRBarrier` whenever any argument word is `{*}`-expanded, so wrong-shape
  IR can no longer bypass the arity checks.
- **Fix E100/E102 false positives on quoted `"}"` and `"]"` literals
  (#130).** Quoted close-brace and close-bracket characters are no longer
  mistaken for stray punctuation through the full pipeline — lexer,
  analyser, formatter, minifier, optimiser, code generator, and the LSP
  semantic-token stray-bracket recovery helper.
- **Key the quoted-context lookup by token identity, not start offset.**
  The zero-width synthetic `SEP` the iRules lexer injects at a `}{` word
  boundary shares the start offset of the following `STR`, so an
  offset-keyed lookup could return the wrong flag. Switched to `id(tok)`
  lookup, which is safe because `argv` and `all_tokens_buf` are populated
  from the same token objects.
- **Fix pygls registration of `willSaveWaitUntil`** so the format-on-save
  handler is actually advertised in server capabilities.
- **Fix a CI `test-ext` flake** by dropping premature `analysis=None`
  early returns in the extension test path.

## Notes for Editor Integrations

- All ten new LSP handlers are gated by individual feature toggles, so
  existing editor integrations continue to work unchanged until the
  relevant toggle is enabled.
- Pull diagnostics are opt-in — the push pipeline remains the default.
