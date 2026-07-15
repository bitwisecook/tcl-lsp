# KCS: feature — Completions

> **Audience:** User
> **Type:** Functionality

## Summary

Context-aware completions for commands, subcommands, variables, switches, and procs.

## Applies to

all-editors, MCP, analyser

## How to use

- **Editor**: Triggered automatically as you type or with Ctrl+Space.
- **MCP**: `complete` tool — pass source and cursor position.
- **Settings**: Toggle with `tclLsp.features.completion`.

## Operational context

The completion provider offers context-sensitive suggestions based on the cursor position: command names, subcommands after a known command, variable names after `$`, proc arguments, switch flags, and package names after `package require`.

When what you have typed matches nothing by prefix and is at least two
characters long, the provider falls back to fuzzy matching, so a small typo
still finds the intended name: `lsaerch` offers `lsearch`, `string lenght`
offers `length`, `lsort -ncoase` offers `-nocase`, and `$bnaana` offers
`$banana`. Choosing a fuzzy suggestion replaces the typo. Prefix matches are
never mixed with fuzzy ones — a fragment that matches something today keeps
exactly today's list.

## File-path anchors

- `rust/tcl-lsp-core/src/completion.rs`

## Failure modes

- Missing completions after registry or parser changes.
- Wrong context detection (e.g. offering commands where variables are expected).

## Test anchors

- `rust/tcl-lsp-core/src/completion.rs` (unit tests)
- `rust/tcl-lsp-server/tests/e2e/completion.rs`

## Screenshots

- `03-completions` — completion list triggered on partial command

![completion list triggered on partial command](../screenshots/03-completions.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
