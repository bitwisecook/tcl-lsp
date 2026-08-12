# KCS: feature — Signature Help

> **Audience:** User
> **Type:** Functionality

## Summary

Parameter hints for commands and procs as you type arguments.

## Applies to

all-editors, analyser

## How to use

- **Editor**: Shown automatically when typing arguments after a command or proc name.
- **Settings**: Toggle with `tclLsp.features.signatureHelp`.

## Operational context

The provider looks up the command or proc under the cursor and shows the expected arguments, highlighting the current parameter position.

## Failure modes

- Wrong parameter highlighted for commands with complex argument patterns.

## Screenshots

- `19-signature-help` — parameter hints popup

![parameter hints popup](../screenshots/19-signature-help.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
