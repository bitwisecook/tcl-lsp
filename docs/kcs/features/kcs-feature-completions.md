# KCS: feature — Completions

## Summary

Context-aware completions for commands, subcommands, variables, switches, and procs.

## Surface

lsp, mcp, all-editors

## How to use

- **Editor**: Triggered automatically as you type or with Ctrl+Space.
- **MCP**: `complete` tool — pass source and cursor position.
- **Settings**: Toggle with `tclLsp.features.completion`.

## Operational context

The completion provider offers context-sensitive suggestions based on the cursor position: command names, subcommands after a known command, variable names after `$`, proc arguments, switch flags, and package names after `package require`.

## File-path anchors

- `lsp/features/completion.py`

## Failure modes

- Missing completions after registry or parser changes.
- Wrong context detection (e.g. offering commands where variables are expected).

## Test anchors

- `tests/test_completion.py`

## Screenshots

- `03-completions` — completion list triggered on partial command

![completion list triggered on partial command](../screenshots/03-completions.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../kcs-lsp-feature-providers.md)
