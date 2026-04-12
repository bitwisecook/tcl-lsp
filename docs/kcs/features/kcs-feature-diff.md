# KCS: feature — iRule Diff

> **Audience:** User
> **Type:** Functionality

## Summary

Semantic comparison of two iRule versions: explains behavioural changes, event deltas, and security or performance implications.

## Applies to

VS Code Copilot Chat, Claude skill

## Question

How do I understand the semantic differences between two versions of an iRule?

## How to use

### VS Code Copilot Chat

Type `@irule /diff` in the Chat panel and provide two file references (e.g., `#file:old.irul #file:new.irul`). The participant runs LSP analysis on both versions, compares the results, and explains what changed.

### Claude Code

Use the `/irule-diff` skill with two file paths:

```
/irule-diff old_irule.irul new_irule.irul
```

## Example

Comparing an iRule before and after an event was added:

> **Events added:** `HTTP_RESPONSE` (per-request)
>
> **Behavioural change:** The new version inspects response headers and
> strips the `Server` header before forwarding. This is a security
> improvement — it prevents server-software fingerprinting.
>
> **No breaking changes detected.** All existing `HTTP_REQUEST` logic
> is unchanged.

The diff is semantic, not textual — it focuses on what the traffic flow does differently, not which lines moved. Security and performance implications are called out explicitly.

## Related

- [KCS feature index](README.md)
- [Chat Slash Commands](kcs-feature-chat-slash-commands.md) — the `/diff` command
- [iRule Review](kcs-feature-irule-review.md) — security review for a single iRule
- [Diagnostics](kcs-feature-diagnostics.md) — the analysis engine behind both versions
