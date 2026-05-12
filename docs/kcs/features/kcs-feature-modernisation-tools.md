# KCS: feature — Modernisation Tools

> **Audience:** User
> **Type:** Functionality

## Summary

Detect legacy iRules patterns eligible for modernisation, and convert nginx, Apache, or HAProxy configurations to iRules.

## Applies to

VS Code Copilot Chat, tcl-lsp CLI, Claude skill

## Question

How do I modernise an old iRule, or convert a reverse-proxy configuration into an iRule?

## How to use

### Find legacy patterns (detection only)

Scans an iRule for legacy patterns and reports modern replacements.  The verb
only *reports* — it does not rewrite source.  Run `tcl opt` to actually apply
the transforms.

| Legacy pattern | Modern replacement |
|---|---|
| Unbraced expressions (`W100`) | Braced `expr {…}` |
| String concatenation for lists (`W104`) | `lappend` |
| Deprecated `matchclass` (`IRULE2001`) | `class match` |
| Ungated `log` in hot events (`IRULE5001`) | Guard with `[log level local0.]` |
| Missing `--` terminators (`W304`) | Add `--` before user-controlled arguments |

### VS Code Copilot Chat

`@irule /convert` — runs the detection and explains each suggestion.

### tcl-lsp CLI

```
tcl find-legacy my_irule.irul
```

### Claude Code

The `/irule-convert` skill wraps the CLI with AI-generated explanations.

### Migrate (reverse-proxy conversion)

`@irule /migrate` in Copilot Chat, or `/irule-migrate` in Claude Code.

Reads an nginx `location` block, Apache `RewriteRule`, or HAProxy `acl`/`use_backend` stanza and generates an equivalent iRule with:

- `location` → `switch -glob [HTTP::uri]`
- `proxy_pass` → `pool`
- `RewriteRule` → `HTTP::uri`
- `acl` → `if`/`class match`

## Example

```
$ tcl find-legacy old_irule.irul
=== Modernisation Suggestions ===

  W100 (line 3): Unbraced expression — use expr {$x + 1} instead of expr $x + 1.
  IRULE2001 (line 7): Deprecated matchclass — use "class match" instead.

  2 suggestions found.
```

## Related

- [KCS feature index](README.md)
- [Chat Slash Commands](kcs-feature-chat-slash-commands.md) — the `/convert` and `/migrate` commands
- [Diagnostics](kcs-feature-diagnostics.md) — the analyser codes the converter detects
- [XC Translation](kcs-feature-xc-translation.md) — translates iRules to F5 Distributed Cloud (different target)
