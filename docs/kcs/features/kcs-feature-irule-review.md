# KCS: feature — iRule Review

> **Audience:** User
> **Type:** Functionality

## Summary

Security-focused analysis of iRules: filters the full diagnostic set to show only security warnings, taint findings, and thread-safety concerns.

## Applies to

Copilot Chat, tcl-lsp CLI, MCP, Claude skill

## Question

How do I run a security-focused review of my iRule?

## How to use

### VS Code Copilot Chat

Type `@irule /review` in the Chat panel. The review runs the full LSP analysis, filters to security-relevant codes, and presents them with AI-generated explanations and fix suggestions.

### tcl-lsp CLI

```
tcl diag my_irule.tcl
tcl diag my_irule.tcl --json
```

`tcl diag` reports the whole diagnostic set. The security, taint, and iRule
codes (the `S`, `T`, and `IRULE` families) are the ones the review focuses on.

### MCP

```json
{"tool": "review", "arguments": {"source": "when HTTP_REQUEST { ... }"}}
```

### Claude Code

The `/irule-review` skill runs the review and presents findings with remediation advice.

## Example

Reviewing an iRule that passes `[HTTP::uri]` to `eval`:

```
$ tcl diag unsafe_irule.irul
unsafe_irule.irul:2:14: warning IRULE3102 Use 'HTTP::uri -normalized' for canonicalized request data; non-normalized values may allow URL evasion patterns.
unsafe_irule.irul:3:10: warning T100     Tainted variable $uri flows into eval; possible code injection
unsafe_irule.irul:3:10: warning W101     eval with substituted arguments risks code injection. Prefer direct invocation or {*}$cmdList to preserve argument boundaries.
diagnostics=3 across 1 input(s)
```

The JSON form returns structured file, line, column, severity, code, and message fields for each finding.

## Related

- [KCS feature index](README.md)
- [Diagnostics](kcs-feature-diagnostics.md) — the full analysis the review filters from
- [Chat Slash Commands](kcs-feature-chat-slash-commands.md) — the `/review` command in Copilot Chat
- [Claude Code Skills](kcs-feature-claude-code-skills.md) — the `/irule-review` skill
