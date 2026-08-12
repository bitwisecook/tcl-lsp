# KCS: feature — Go to Definition

> **Audience:** User
> **Type:** Functionality

## Summary

Jump to proc or variable definitions within and across files.

## Applies to

all-editors, MCP, analyser

## How to use

- **Editor**: Ctrl+Click or F12 on a symbol.
- **MCP**: `goto_definition` tool — pass source, line, and character position.
- **Settings**: Toggle with `tclLsp.features.definition`.

## Operational context

Resolves proc calls, variable references, namespace-qualified names, and BIG-IP cross-object references to their definition locations. Proc-reference matching is shared with Find References, so the two always agree.

## Failure modes

- Definition not found after proc lookup or namespace resolution changes.
- A cross-file namespace variable stops resolving when the declaring file is
  no longer in the workspace index (it was closed *and* is outside every
  workspace folder).
- A namespace whose target is computed (`namespace eval $ns { … }`) names no
  fixed namespace, so neither the block nor any reference to it resolves.
- A callee that binds a *literal* caller-side name (`upvar 1 options options`)
  names it nowhere at the call site, so there is no word to jump to.
- A callee whose `upvar` level is not `1` (`upvar 0`, `upvar #0`, `upvar 2`)
  aliases some other frame, so its call site defines nothing where you are
  reading and no location is reported.

## Screenshots

- `15-definition` — peek definition inline

![peek definition inline](../screenshots/15-definition.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
