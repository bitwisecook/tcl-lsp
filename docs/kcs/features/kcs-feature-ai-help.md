# KCS: feature — AI Help

> **Audience:** User
> **Type:** Functionality

## Summary

Feature catalogue and full-text search across every tcl-lsp feature, served by the KCS help database.

## Applies to

VS Code Copilot Chat, tcl-lsp CLI, MCP, Claude skill

## Question

How do I find out what tcl-lsp features exist and how to use them?

## How to use

### VS Code Copilot Chat

Type `@irule /help`, `@tcl /help`, or `@tk /help` in the Chat panel. Ask a question or leave it blank to see the full catalogue.

### tcl-lsp CLI

```
tcl help
tcl help "optimise"
```

### MCP

```json
{"tool": "help", "arguments": {"topic": "formatting"}}
```

### Claude Code

Use the `/ai-help` skill.

## Example

```
$ tcl help "taint"

=== LSP + AI Features ===

  Diagnostics
    Errors, warnings, security, taint tracking, and style checks shown as you type.

  iRule Review
    Security-focused analysis: security warnings, taint findings, and thread-safety concerns.
```

The help tool queries the KCS feature database with full-text search, groups results by category, and returns summaries with pointers to the relevant KCS feature pages.

## Related

- [KCS feature index](README.md)
- [Chat Slash Commands](kcs-feature-chat-slash-commands.md) — the `/help` command
- [MCP Server](kcs-feature-mcp-server.md)
- [Claude Code Skills](kcs-feature-claude-code-skills.md)
