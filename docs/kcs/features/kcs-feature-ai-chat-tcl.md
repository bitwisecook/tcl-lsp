# KCS: feature — @tcl Chat Participant

## Summary

VS Code Copilot Chat participant for creating, explaining, fixing, validating, and optimising general Tcl code.

## Surface

vscode-chat

## Availability

| Context | How |
|---------|-----|
| VS Code Copilot Chat | Type `@tcl` then a slash command or question |

## How to use

Type `@tcl` in the Copilot Chat panel followed by a slash command:

| Command | Description |
|---------|-------------|
| `/create` | Create Tcl code from a description |
| `/explain` | Explain Tcl code |
| `/fix` | Fix issues found by the LSP |
| `/validate` | Run LSP diagnostics |
| `/optimise` | Apply LSP optimiser suggestions |
| `/help` | Show available features and commands |

Or ask a free-form Tcl question without a slash command.

## Operational context

Uses the same analysis engine as `@irule` but with general Tcl system prompts and dialect settings. The agentic loop validates and iterates until diagnostics are clean.

## File-path anchors

- `editors/vscode/src/chat/tclParticipant.ts`
- `editors/vscode/src/chat/commands/`

## Failure modes

- AI features disabled (`tclLsp.ai.enabled` is false).
- Copilot extension not installed.

## Test anchors

- `editors/vscode/src/test/chatUtilities.test.ts`

## Discoverability

- [KCS feature index](README.md)
- [VS Code extension contracts](../kcs-vscode-extension-contracts.md)
