# KCS: feature — Optimiser

> **Audience:** User
> **Type:** Functionality

## Summary

Finds optimisation opportunities in Tcl/iRules code and produces rewritten source.

## Applies to

all-editors, Copilot Chat, MCP, Claude skill, optimisation

## Availability

| Context | How |
|---------|-----|
| VS Code command | `Tcl: Apply All Optimisations` (Ctrl+Alt+O), `Tcl: Show Optimisation Suggestions` |
| VS Code chat | `@irule /optimise`, `@tcl /optimise` |
| Any LSP editor | Code actions (lightbulb) on O-code diagnostics |
| MCP | `optimize` tool |
| Claude Code | `/irule-optimise`, `/tcl-optimise` |
| CLI | `tcl opt <file>` |

## How to use

- **Editor**: Optimiser suggestions appear as hint-level diagnostics (O-codes). Use the lightbulb or `Tcl: Apply All Optimisations` to apply them. Toggle with `Tcl: Toggle Optimiser Suggestions` or `tclLsp.optimiser.enabled`.
- **MCP**: `optimize` tool returns suggestions and rewritten source.
- **Claude Code**: `/irule-optimise` or `/tcl-optimise` applies suggestions with explanations.

## Operational context

The optimiser analyses compiled IR to find patterns that can be rewritten for better performance: string map vs regexp, lindex vs lrange, braced expressions, static string concatenation, and more. Each suggestion has an O-code and includes the rewritten source.

### O105 — Constant var-ref propagation and string interpolation

O105 propagates SCCP-resolved constant values into `$var` references in command arguments and string interpolations. When a variable is assigned a constant value (`set x 42`) and subsequently used (`puts $x`), O105 replaces `$x` with the literal `42`. This works in both bare-word positions and inside double-quoted strings.

**Safety constraints:**
- String interpolation propagation is restricted to constants defined in the **same basic block** with no intervening `IRCall` or `IRBarrier` (which could mutate the variable via `upvar` or exception side-effects).
- Values containing unsafe characters (metacharacters that could change interpretation in the target context) are not propagated.
- O105 also covers global value numbering and common-subexpression
  elimination (redundant computation elimination).

### O126 — Unused variable removal

O126 removes `set` statements for variables that are never read anywhere in the function. Unlike O109 (dead stores — overwritten before read), O126 targets entirely unused variables. Skipped at top-level to avoid changing observable script results. See [Unused Variable Detection](kcs-feature-unused-variables.md).

## Failure modes

- Optimisation produces semantically different code.
- O-code suppression not respected.

## Screenshots

- `22-optimiser` — optimiser suggestions in the editor

![optimiser suggestions in the editor](../screenshots/22-optimiser.png)

## Discoverability

- [KCS feature index](README.md)
- [Code actions](kcs-feature-code-actions.md)
