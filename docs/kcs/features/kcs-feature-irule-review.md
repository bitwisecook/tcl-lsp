# KCS: feature — iRule Review

> **Audience:** User
> **Type:** Functionality

## Summary

Security-focused analysis of iRules: filters the full diagnostic set to show only security warnings, taint findings, and thread-safety concerns.

## Applies to

VS Code Copilot Chat, tcl-lsp CLI, MCP, Claude skill

## Question

How do I run a security-focused review of my iRule?

## How to use

### VS Code Copilot Chat

Type `@irule /review` in the Chat panel. The review runs the full LSP analysis, filters to security-relevant codes, and presents them with AI-generated explanations and fix suggestions.

### tcl-lsp CLI

```
tcl review my_irule.tcl
tcl review my_irule.tcl --json
```

### MCP

```json
{"tool": "review", "arguments": {"source": "when HTTP_REQUEST { ... }"}}
```

### Claude Code

The `/irule-review` skill runs the review and presents findings with remediation advice.

## Example

Reviewing an iRule that passes `[HTTP::uri]` to `eval`:

```
$ tcl review unsafe_irule.tcl
=== Security Review ===

  T100 (line 5): Tainted data flows into eval — code injection risk.
       Source: [HTTP::uri] (taint source)
       Sink:  eval (dangerous code-execution sink)

  1 security finding, 0 thread-safety findings.
```

The JSON form returns structured code, message, range, severity, and sink details for each finding.

## Related

- [KCS feature index](README.md)
- [Diagnostics](kcs-feature-diagnostics.md) — the full analysis the review filters from
- [Chat Slash Commands](kcs-feature-chat-slash-commands.md) — the `/review` command in Copilot Chat
- [Claude Code Skills](kcs-feature-claude-code-skills.md) — the `/irule-review` skill
