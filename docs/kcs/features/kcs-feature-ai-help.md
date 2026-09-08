# KCS: feature — AI Help

> **Audience:** User
> **Type:** Functionality

## Summary

Feature catalogue and full-text search across every tcl-lsp feature, served by the KCS help database.

## Applies to

Copilot Chat, tcl-lsp CLI, MCP, Claude skill

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
$ tcl help --limit 2 "taint"
2 matches for 'taint':
- iRule Review [VS Code AI Chat]
  Security-focused analysis of iRules: filters the full diagnostic set to show only security warnings, taint findings, and thread-safety concerns.
  file: kcs-feature-irule-review.md
- Diagnostics [LSP + AI Features]
  Errors, warnings, security, taint tracking, and style checks shown as you type.
  file: kcs-feature-diagnostics.md
```

Run `tcl help` with no query to list every feature grouped by category.

## Related

- [KCS feature index](README.md)
- [Chat Slash Commands](kcs-feature-chat-slash-commands.md) — the `/help` command
- [MCP Server](kcs-feature-mcp-server.md)
- [Claude Code Skills](kcs-feature-claude-code-skills.md)
