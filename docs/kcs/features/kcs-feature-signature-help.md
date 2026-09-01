# KCS: feature — Signature Help

> **Audience:** User
> **Type:** Functionality

## Summary

Parameter hints for commands and procs as you type arguments.

## Applies to

all-editors, analyser

## How to use

- **Editor**: Shown automatically when typing arguments after a command or proc name. In a `proc` declaration it describes `proc name args body` while the declaration header is active; commands typed inside the braced body get their own signatures.
- **Master setting**: Toggle with `tclLsp.features.signatureHelp`.
- **Per-command setting**: Add built-in command names to `tclLsp.signatureHelp.disabledCommands`. For example, `["set", "incr"]` silences those built-ins while retaining help for `format` and user-defined procs.
- **Config file**: The equivalent layered INI setting is `[signatureHelp] disabled_commands = set, incr`. Names are resolved through Tcl's shared qualified-name rules, so equivalent namespace-separator spellings select the same registry command.

## Operational context

The provider looks up the command or proc under the text caret and shows the expected arguments, highlighting the current parameter position. Registry-declared Tcl script bodies, switch/case actions, lambdas, definition bodies, and command substitutions establish nested command contexts. Ordinary braced data remains an argument of its containing command.

## Failure modes

- Wrong parameter highlighted for commands with complex argument patterns.
- A signature that stays open on comments or blank text inside a proc body indicates that the body was mistaken for the outer `proc` argument. Current releases close it by returning no active signature in those positions.

## Screenshots

- `19-signature-help` — parameter hints popup

![parameter hints popup](../screenshots/19-signature-help.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
