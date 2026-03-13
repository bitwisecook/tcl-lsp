# KCS: feature — Signature Help

## Summary

Parameter hints for commands and procs as you type arguments.

## Surface

lsp, all-editors

## How to use

- **Editor**: Shown automatically when typing arguments after a command or proc name.
- **Settings**: Toggle with `tclLsp.features.signatureHelp`.

## Operational context

The provider looks up the command or proc under the cursor and shows the expected arguments, highlighting the current parameter position.

## File-path anchors

- `lsp/features/signature_help.py`

## Failure modes

- Wrong parameter highlighted for commands with complex argument patterns.

## Test anchors

- `tests/test_signature_help.py`

## Screenshots

- `19-signature-help` — parameter hints popup

![parameter hints popup](../screenshots/19-signature-help.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../kcs-lsp-feature-providers.md)
