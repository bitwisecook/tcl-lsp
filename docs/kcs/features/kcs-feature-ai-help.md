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
$ tcl help --limit 1 "fakecmp"
1 match for 'fakecmp':
- FakeCMP Tools [MCP Tools]
  Deterministic TMM hash lookup and multi-TMM test distribution planner for iRule testing without hardware.
  file: kcs-feature-fakecmp-tools.md
```

Each hit names the feature, the category it is grouped under, its summary, and
the KCS page to read.  Run `tcl help` with no query to list every feature
grouped by category, `--limit N` to cap the matches, and `--dialect NAME` to
filter to one dialect.

## Related

- [KCS feature index](README.md)
- [Chat Slash Commands](kcs-feature-chat-slash-commands.md) — the `/help` command
- [MCP Server](kcs-feature-mcp-server.md)
- [Claude Code Skills](kcs-feature-claude-code-skills.md)
