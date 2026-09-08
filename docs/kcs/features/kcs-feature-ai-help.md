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
- AI Help [Claude Code Skills]
  Feature catalogue and full-text search across every tcl-lsp feature, served by the KCS help database.
  file: kcs-feature-ai-help.md
- Semantic Graphs [Claude Code Skills]
  Structured call graph, symbol graph, and data-flow graph extraction from Tcl and iRules source code.
  file: kcs-feature-semantic-graphs.md
```

Run `tcl help` with no query to list every feature grouped by category.

## Related

- [KCS feature index](README.md)
- [Chat Slash Commands](kcs-feature-chat-slash-commands.md) — the `/help` command
- [MCP Server](kcs-feature-mcp-server.md)
- [Claude Code Skills](kcs-feature-claude-code-skills.md)
